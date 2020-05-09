use {
    crate::utils::{canonical_ident, get_named_fields, NamedField},
    proc_macro2::{Ident, Span, TokenStream},
    quote::ToTokens,
    syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, FieldsUnnamed},
};

pub fn impl_write_macro(ast: &DeriveInput) -> TokenStream {
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
            #ident: <#ty as ::tree_buf::internal::Writable>::WriterArray
        }
    });

    let buffers = fields.iter().map(|NamedField { ident, .. }| {
        quote! {
            self.#ident.buffer(&value.#ident);
        }
    });

    let flushers = fields.iter().map(|NamedField { ident, canon_str, ty }| {
        quote! {
            ::tree_buf::internal::write_ident(#canon_str, stream);
            let o = self.#ident;
            stream.write_with_id(|stream| ::tree_buf::internal::WriterArray::<#ty>::flush(o, stream));
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
        0..=8 => (quote! {}, Ident::new(format!("Obj{}", num_fields).as_str(), Span::call_site())),
        _ => (
            quote! {
                ::tree_buf::internal::encodings::varint::encode_prefix_varint(#num_fields as u64 - 9, stream.bytes);
            },
            Ident::new("ObjN", Span::call_site()),
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
        #vis struct #array_writer_name {
            #(#array_fields,)*
        }

        impl ::tree_buf::internal::WriterArray<#name> for #array_writer_name {
            fn buffer<'a, 'b : 'a>(&'a mut self, value: &'b #name) {
                #buffer
            }
            fn flush<O: ::tree_buf::options::EncodeOptions>(mut self, stream: &mut ::tree_buf::internal::WriterStream<'_, O>) -> ::tree_buf::internal::ArrayTypeId {
                // TODO: Re-enable profile here
                // See also dcebaa54-d21e-4e79-abfe-4a89cc829180
                //::tree_buf::internal::profile!("WriterArray::flush");
                #flush
            }
        }

        impl ::tree_buf::internal::Writable for #name {
            type WriterArray=#array_writer_name;
            fn write_root<O: ::tree_buf::options::EncodeOptions>(&self, stream: &mut ::tree_buf::internal::WriterStream<'_, O>) -> tree_buf::internal::RootTypeId {
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
        tree_buf_discriminant: <u64 as ::tree_buf::Writable>::WriterArray,
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
                            #variant_ident: Option<(u64, <#ty as ::tree_buf::Writable>::WriterArray)>
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
                                stream.write_with_id(|stream| ::tree_buf::internal::WriterArray::<#ty>:: flush(buffer, stream));
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
        ::tree_buf::internal::encodings::varint::encode_prefix_varint(variant_count, stream.bytes);
        let _0 = self.tree_buf_discriminant;
        stream.write_with_id(|stream| _0.flush(stream));

        for current_discriminant in 0..variant_count {
            #(#flushes)*
        }

        ::tree_buf::internal::ArrayTypeId::Enum
    };

    fill_write_skeleton(ast, array_fields.iter(), buffer, flush, write_root)
}
