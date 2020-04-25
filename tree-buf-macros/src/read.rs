use {
    crate::utils::{NamedField, canonical_ident, get_named_fields},
    proc_macro2::TokenStream,
    quote::ToTokens,
    syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, FieldsUnnamed},
};

pub fn impl_read_macro(ast: &DeriveInput) -> TokenStream {
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
            // Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
            #ident: match self.#ident.read_next() { Ok(v) => v, Err(e) => { return Err(e.into()); } },
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
        Ok(#name {
            #(#read_nexts)*
        })
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
            // TODO: See if sometimes we can use Infallible here.
            type Error=::tree_buf::ReadError;
            fn new(sticks: ::tree_buf::internal::DynArrayBranch<'_>, options: &impl ::tree_buf::options::DecodeOptions) -> Result<Self, ::tree_buf::ReadError> {
                // TODO: Re-enable profile here
                // See also dcebaa54-d21e-4e79-abfe-4a89cc829180
                //::tree_buf::internal::profile!("ReaderArray::new");
                #new
            }
            fn read_next(&mut self) -> ::std::result::Result<Self::Read, Self::Error> {
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
                                    return Err(::tree_buf::ReadError::InvalidFormat);
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
                                    // Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
                                    return Ok(#ident::#variant_ident(match r.read_next() { Ok(v) => v, Err(e) => return Err(e.into()) }));
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
        let discriminant = ::tree_buf::internal::InfallibleReaderArray::read_next_infallible(&mut self.tree_buf_discriminant);
        #(#read_nexts)*

        // See also: fb0a3c86-23be-4d4a-9dbf-9c83ae6e2f0f
        todo!("Make this unreachable by verifying range");
    };

    fill_read_skeleton(ast, read, array_fields.iter(), new, read_next)
}
