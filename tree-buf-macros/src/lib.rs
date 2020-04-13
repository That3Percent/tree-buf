extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use {
    inflector::cases::camelcase::to_camel_case,
    proc_macro2::{Ident, TokenStream},
    quote::ToTokens,
    syn::{parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Fields, FieldsUnnamed, Type},
};

struct NamedField<'a> {
    ident: &'a Ident,
    ty: &'a Type,
    canon_str: String,
}
type NamedFields<'a> = Vec<NamedField<'a>>;

#[proc_macro_derive(Write)]
pub fn write_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let output = impl_write_macro(&ast);
    proc_macro::TokenStream::from(output)
}

fn impl_write_macro(ast: &DeriveInput) -> TokenStream {
    match &ast.data {
        Data::Struct(data_struct) => impl_struct_write(ast, data_struct),
        Data::Enum(data_enum) => impl_enum_write(ast, data_enum),
        Data::Union(_) => panic!("Unions are not supported by tree-buf"),
    }
}

fn impl_struct_write(ast: &DeriveInput, data_struct: &DataStruct) -> TokenStream {
    let fields = get_named_fields(data_struct);

    let writers = fields.iter().map(|NamedField { ident, canon_str, .. }| {
        quote! {
            ::tree_buf::internal::write_ident(#canon_str, stream);
            stream.write_with_id(|stream| self.#ident.write_root(stream));
        }
    });

    let array_fields = fields.iter().map(|NamedField { ident, ty, .. }| {
        quote! {
            #ident: <#ty as ::tree_buf::internal::Writable<'a>>::WriterArray
        }
    });

    let buffers = fields.iter().map(|NamedField { ident, .. }| {
        quote! {
            self.#ident.buffer(&value.#ident);
        }
    });

    let flushers = fields.iter().map(|NamedField { ident, canon_str, .. }| {
        quote! {
            ::tree_buf::internal::write_ident(#canon_str, stream);
            let o = self.#ident;
            stream.write_with_id(|stream| o.flush(stream));
        }
    });

    let num_fields = fields.len();

    /*
    quote! {
        ::tree_buf::internal::write_fields(#num_fields, stream, |stream| move {
            #(#writers)*
        })
    };
    */
    // See also: fadaec14-35ad-4dc1-b6dc-6106ab811669
    let (prefix, suffix) = match num_fields {
        0..=8 => (quote! {}, Ident::new(format!("Obj{}", num_fields).as_str(), ast.ident.span().clone())),
        _ => (
            quote! {
                ::tree_buf::internal::encodings::varint::encode_prefix_varint(#num_fields as u64 - 9, stream.bytes());
            },
            Ident::new("ObjN", ast.ident.span().clone()),
        ),
    };

    let flush = quote! {
        #prefix
        #(#flushers)*
        ::tree_buf::internal::ArrayTypeId::#suffix
    };

    let buffer = quote! {
        #(#buffers)*
    };

    let write_root = quote! {
        #prefix
        #(#writers)*
        ::tree_buf::internal::RootTypeId::#suffix
    };

    fill_write_skeleton(ast, array_fields, buffer, flush, write_root)
}

fn fill_write_skeleton<A: ToTokens>(
    ast: &DeriveInput,
    array_fields: impl Iterator<Item = A>,
    buffer: impl ToTokens,
    flush: impl ToTokens,
    write_root: impl ToTokens,
) -> TokenStream {
    let name = &ast.ident;
    let vis = &ast.vis;
    let array_writer_name = format_ident!("{}TreeBufWriterArray", name);

    quote! {
        #[derive(Default)]
        #[allow(non_snake_case)]
        #vis struct #array_writer_name<'a> {
            #(#array_fields,)*
        }

        impl<'a> ::tree_buf::internal::WriterArray<'a> for #array_writer_name<'a> {
            type Write=#name;
            fn buffer<'b : 'a>(&mut self, value: &'b Self::Write) {
                #buffer
            }
            fn flush(mut self, stream: &mut impl ::tree_buf::internal::WriterStream) -> ::tree_buf::internal::ArrayTypeId {
                // TODO: Re-enable profile here
                // See also dcebaa54-d21e-4e79-abfe-4a89cc829180
                //::tree_buf::internal::profile!("WriterArray::flush");
                #flush
            }
        }

        impl<'a> ::tree_buf::internal::Writable<'a> for #name {
            type WriterArray=#array_writer_name<'a>;
            fn write_root<'b: 'a>(&'b self, stream: &mut impl ::tree_buf::internal::WriterStream) -> tree_buf::internal::RootTypeId {
                // TODO: Re-enable profile here
                // See also dcebaa54-d21e-4e79-abfe-4a89cc829180
                //::tree_buf::internal::profile!("WriterArray::write_root");
                #write_root
            }
        }
    }
}

fn impl_enum_write(ast: &DeriveInput, data_enum: &DataEnum) -> TokenStream {
    // What is needed:
    // An outer struct containing writers for each variant
    // For each variant, possibly it's own writer struct if it's a tuple or struct sort of DataEnum
    // A discriminant

    let mut array_fields = Vec::new();
    array_fields.push(quote! {
        tree_buf_discriminant: <u64 as ::tree_buf::Writable<'a>>::WriterArray,
        tree_buf_next_discriminant: u64
    });

    let mut array_matches = Vec::new();
    let mut root_matches = Vec::new();
    let mut flushes = Vec::new();
    let ident = &ast.ident;

    for variant in data_enum.variants.iter() {
        let variant_ident = &variant.ident;
        let discriminant = canonical_ident(variant_ident);

        match &variant.fields {
            Fields::Unit => todo!("Unit enums not yet supported by tree-buf write"),
            Fields::Named(_named_fields) => todo!("Enums with named fields not yet supported by tree-buf"),
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                let unnamed: Vec<_> = unnamed.iter().collect();
                match unnamed.len() {
                    // TODO: Check if this is really unreachable. It might be `Variant {}`.
                    // In this case consider writing a struct with no fields
                    0 => unreachable!(),
                    1 => {
                        let ty = &unnamed[0].ty;
                        root_matches.push(quote! {
                            #ident::#variant_ident(_0) => {
                                ::tree_buf::internal::write_ident(#discriminant, stream);
                                stream.write_with_id(|stream| _0.write_root(stream));
                            }
                        });
                        array_fields.push(quote! {
                            #variant_ident: Option<(u64, <#ty as ::tree_buf::Writable<'a>>::WriterArray)>
                        });
                        array_matches.push(quote! {
                            #ident::#variant_ident(_0) => {
                                if self.#variant_ident.is_none() {
                                    self.#variant_ident = Some((self.tree_buf_next_discriminant, Default::default()));
                                    self.tree_buf_next_discriminant += 1;
                                }
                                let t = self.#variant_ident.as_mut().unwrap();
                                self.tree_buf_discriminant.buffer(&t.0);
                                t.1.buffer(_0);
                            }
                        });
                        flushes.push(quote! {
                            let mut matches = false;
                            if let Some((d, _)) = &self.#variant_ident {
                                if *d == current_discriminant {
                                    matches = true;
                                }
                            }
                            if matches {
                                let mut buffer = self.#variant_ident.take().unwrap().1;
                                ::tree_buf::internal::write_ident(#discriminant, stream);
                                stream.write_with_id(|stream| buffer.flush(stream));
                                continue;
                            }
                        });
                    }
                    _ => todo!("Enums with multiple unnamed fields not yet supported by tree-buf Write"),
                }
            }
        }
    }

    let write_root = quote! {
       match self {
           #(#root_matches,)*
       }
       ::tree_buf::internal::RootTypeId::Enum
    };
    let buffer = quote! {
       match value {
           #(#array_matches,)*
       }
    };
    let flush = quote! {
        let variant_count = self.tree_buf_next_discriminant;
        ::tree_buf::internal::encodings::varint::encode_prefix_varint(variant_count, stream.bytes());
        let _0 = self.tree_buf_discriminant;
        stream.write_with_id(|stream| _0.flush(stream));

        for current_discriminant in 0..variant_count {
            #(#flushes)*
        }

        ::tree_buf::internal::ArrayTypeId::Enum
    };

    fill_write_skeleton(ast, array_fields.iter(), buffer, flush, write_root)
}

#[proc_macro_derive(Read)]
pub fn read_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let output = impl_read_macro(&ast);
    proc_macro::TokenStream::from(output)
}

fn impl_read_macro(ast: &DeriveInput) -> TokenStream {
    match &ast.data {
        Data::Struct(data_struct) => impl_struct_read(ast, data_struct),
        Data::Enum(data_enum) => impl_enum_read(ast, data_enum),
        Data::Union(_) => panic!("Unions are not supported by tree-buf"),
    }
}

fn impl_struct_read(ast: &DeriveInput, data_struct: &DataStruct) -> TokenStream {
    let fields = get_named_fields(data_struct);
    let name = &ast.ident;

    let inits = fields.iter().map(|NamedField { ident, canon_str, .. }| {
        quote! {
            let #ident = fields.remove(#canon_str).unwrap_or_default();
        }
    }).collect::<Vec<_>>();
    let unwraps = fields.iter().map(|NamedField { ident, .. }| {
        quote! {
            #ident: #ident?,
        }
    }).collect::<Vec<_>>();

    let mut parallel_lhs = quote! {};
    let mut reads_parallel_rhs = quote! {};
    let mut news_parallel_rhs = quote! {};
    let mut is_first = true;

    for NamedField { ident, ty, .. } in fields.iter() {
        if is_first {
            is_first = false;
            parallel_lhs = quote! { #ident };
            reads_parallel_rhs = quote! { 
                <#ty as ::tree_buf::internal::Readable>::read(
                    #ident,
                    options,
                )
            };
            news_parallel_rhs = quote! {
                ::tree_buf::internal::ReaderArray::new(#ident, options)
            };
        } else {
            parallel_lhs = quote! { (#ident, #parallel_lhs) };
            reads_parallel_rhs = quote! {
                ::tree_buf::internal::parallel(
                    || <#ty as ::tree_buf::internal::Readable>::read(
                        #ident,
                        options,
                    ),
                    || #reads_parallel_rhs,
                    options
                )
            };
            news_parallel_rhs = quote! {
                ::tree_buf::internal::parallel(
                    || ::tree_buf::internal::ReaderArray::new(#ident, options),
                    || #news_parallel_rhs,
                    options
                )
            }
        }
    }

    let array_fields = fields.iter().map(|NamedField { ident, ty, .. }| {
        quote! {
            #ident: <#ty as ::tree_buf::internal::Readable>::ReaderArray
        }
    });

    let read_nexts = fields.iter().map(|NamedField { ident, .. }| {
        quote! {
            #ident: self.#ident.read_next(),
        }
    });

    let read = quote! {
        let mut fields = match sticks {
            ::tree_buf::internal::DynRootBranch::Object { fields } => fields,
            _ => return Err(::tree_buf::ReadError::SchemaMismatch),
        };

        #(#inits)*

        let #parallel_lhs = #reads_parallel_rhs;

        Ok(Self {
            #(#unwraps)*
        })
    };
    let new = quote! {
        let mut fields = match sticks {
            ::tree_buf::internal::DynArrayBranch::Object { fields } => fields,
            _ => return Err(::tree_buf::ReadError::SchemaMismatch),
        };

        #(#inits)*

        let #parallel_lhs = #news_parallel_rhs;

        Ok(Self {
            #(#unwraps)*
        })
    };

    let read_next = quote! {
        #name {
            #(#read_nexts)*
        }
    };

    fill_read_skeleton(ast, read, array_fields, new, read_next)
}

fn fill_read_skeleton<A: ToTokens>(ast: &DeriveInput, read: impl ToTokens, array_fields: impl Iterator<Item = A>, new: impl ToTokens, read_next: impl ToTokens) -> TokenStream {
    let name = &ast.ident;
    let vis = &ast.vis;
    let array_reader_name = format_ident!("{}TreeBufReaderArray", name);

    quote! {
        #[allow(non_snake_case)]
        impl ::tree_buf::internal::Readable for #name {
            type ReaderArray = #array_reader_name;
            fn read(sticks: ::tree_buf::internal::DynRootBranch<'_>, options: &impl ::tree_buf::options::DecodeOptions) -> Result<Self, ::tree_buf::ReadError> {
                // TODO: Re-enable profile here
                // See also dcebaa54-d21e-4e79-abfe-4a89cc829180
                //::tree_buf::internal::profile!("Readable::read");
                #read
            }
        }

        #[allow(non_snake_case)]
        #vis struct #array_reader_name {
            #(#array_fields,)*
        }

        #[allow(non_snake_case)]
        impl ::tree_buf::internal::ReaderArray for #array_reader_name {
            type Read=#name;
            fn new(sticks: ::tree_buf::internal::DynArrayBranch<'_>, options: &impl ::tree_buf::options::DecodeOptions) -> Result<Self, ::tree_buf::ReadError> {
                // TODO: Re-enable profile here
                // See also dcebaa54-d21e-4e79-abfe-4a89cc829180
                //::tree_buf::internal::profile!("ReaderArray::new");
                #new
            }
            fn read_next(&mut self) -> Self::Read {
                #read_next
            }
        }
    }
}

fn impl_enum_read(ast: &DeriveInput, data_enum: &DataEnum) -> TokenStream {
    let ident = &ast.ident;
    let mut array_fields = Vec::new();
    array_fields.push(quote! {
        tree_buf_discriminant: <u64 as ::tree_buf::Readable>::ReaderArray
    });

    let mut new_matches = Vec::new();
    let mut new_inits = Vec::new();
    let mut read_nexts = Vec::new();
    let mut new_unpacks = Vec::new();
    let mut new_parallel_lhs = quote! { tree_buf_discriminant };
    let mut new_parallel_rhs = quote! { ::tree_buf::internal::ReaderArray::new(tree_buf_discriminant, options) };

    let mut root_matches = Vec::new();

    for variant in data_enum.variants.iter() {
        let variant_ident = &variant.ident;
        let discriminant = canonical_ident(variant_ident);

        match &variant.fields {
            Fields::Unit => todo!("Unit enums not yet supported by tree-buf read"),
            Fields::Named(_named_fields) => todo!("Enums with named fields not yet supported by tree-buf read"),
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                match unnamed.len() {
                    // TODO: Check if this is really unreachable. It might be `Variant {}`
                    0 => unreachable!(),
                    1 => {
                        root_matches.push(quote! {
                            #discriminant => {
                                Self::#variant_ident(::tree_buf::internal::Readable::read(*value, options)?)
                            }
                        });
                        let ty = &unnamed[0].ty;
                        array_fields.push(quote! {
                            #variant_ident: Option<(u64, <#ty as ::tree_buf::internal::Readable>::ReaderArray)>
                        });
                        new_unpacks.push(quote! { #variant_ident: #variant_ident.transpose()?, });
                        new_parallel_lhs = quote! { (#variant_ident, #new_parallel_lhs) };
                        new_parallel_rhs = quote! {
                            ::tree_buf::internal::parallel(
                                || #variant_ident.map(|(i, d)| { ::tree_buf::internal::ReaderArray::new(d, options).map(|v| (i, v)) }),
                                || #new_parallel_rhs,
                                options
                            )
                        };
                        new_matches.push(quote! {
                            #discriminant => {
                                if #variant_ident.is_some() {
                                    return Err(::tree_buf::ReadError::InvalidFormat(::tree_buf::internal::error::InvalidFormat::DuplicateEnumDiscriminant));
                                }
                                #variant_ident = Some(
                                    (index as u64, data)
                                );
                            }
                        });
                        new_inits.push(quote! {
                            let mut #variant_ident = None;
                        });
                        read_nexts.push(quote! {
                            if let Some((d, r)) = &mut self.#variant_ident {
                                if *d == discriminant {
                                    return #ident::#variant_ident(r.read_next());
                                }
                            }
                        })
                    }
                    _ => todo!("Enums with multiple unnamed fields not yet supported by tree-buf Read"),
                }
            }
        }
    }

    let read = quote! {
        // If this is an enum,
        if let ::tree_buf::internal::DynRootBranch::Enum { discriminant, value } = sticks {
            Ok(
                // See if it's a variant we are aware of, and that the value
                // matches the expected data.
                match discriminant {
                    #(#root_matches),*
                    _ => { return Err(::tree_buf::ReadError::SchemaMismatch); },
                }
            )
        } else {
            Err(::tree_buf::ReadError::SchemaMismatch)
        }
    };

    let new = quote! {
        match sticks {
            ::tree_buf::internal::DynArrayBranch::Enum {discriminants, variants} => {
                let tree_buf_discriminant = *discriminants;
                #(#new_inits)*;
                

                for (index, variant) in variants.into_iter().enumerate() {
                    let ::tree_buf::internal::ArrayEnumVariant { ident, data } = variant;
                    match ident {
                        #(#new_matches),*
                        _ => { return Err(::tree_buf::ReadError::SchemaMismatch); }
                    }
                }

                let #new_parallel_lhs = #new_parallel_rhs;

                let result = Self {
                    tree_buf_discriminant: tree_buf_discriminant?,
                    #(#new_unpacks)*
                };

                // FIXME: Need to verify that the range of tree_buf_discriminant does
                // not go beyond the number of variants listed (this would indicate a corrupt file)
                // See also: fb0a3c86-23be-4d4a-9dbf-9c83ae6e2f0f
                Ok(result)
            }
            _ => {
                Err(::tree_buf::ReadError::SchemaMismatch)
            }
        }
    };

    let read_next = quote! {
        let discriminant = self.tree_buf_discriminant.read_next();
        #(#read_nexts)*

        // See also: fb0a3c86-23be-4d4a-9dbf-9c83ae6e2f0f
        todo!("Make this unreachable by verifying range");
    };

    fill_read_skeleton(ast, read, array_fields.iter(), new, read_next)
}

fn get_named_fields(data_struct: &DataStruct) -> NamedFields {
    // TODO: Lift restriction
    let fields_named = match &data_struct.fields {
        Fields::Named(fields_named) => fields_named,
        _ => panic!("The struct must have named fields"),
    };

    fields_named
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().unwrap();
            NamedField {
                ident: field.ident.as_ref().unwrap(),
                ty: &field.ty,
                canon_str: canonical_ident(&ident),
            }
        })
        .collect()
}

// TODO: Semantically this is a sequence of case-folded canonically encoded utf-8 words (though, this is not quite implemented as such here)
// This is prime for some kind of dictionary compression. Most applications won't ever need to ship the dictionary since it only
// would happen in the proc-macro, except when introspection is required. (Reader for example just compares byte arrays)
// or compression, and that can just happen in the proc-macro.
// TODO: Ensure that leading separators are preserved?
// TODO: Unfortunately, the current method is quite inadequate. Consider a language with no case. Consider a letter 'q' having
// neither uppercase nor lowercase. qq vs q_q is different. But, in this encoding they are the same.
fn canonical_ident(ident: &Ident) -> String {
    let ident_str = format!("{}", ident);
    to_camel_case(&ident_str)
}
