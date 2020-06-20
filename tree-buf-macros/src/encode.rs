use {
    crate::utils::{canonical_ident, get_named_fields, NamedField},
    proc_macro2::{Ident, Span, TokenStream},
    quote::ToTokens,
    syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, FieldsUnnamed},
};

pub fn impl_encode_macro(ast: &DeriveInput) -> TokenStream {
    match &ast.data {
        Data::Struct(data_struct) => impl_struct_encode(ast, data_struct),
        Data::Enum(data_enum) => impl_enum_encode(ast, data_enum),
        Data::Union(_) => panic!("Unions are not supported by tree-buf"),
    }
}

fn impl_struct_encode(ast: &DeriveInput, data_struct: &DataStruct) -> TokenStream {
    let fields = get_named_fields(data_struct);

    let encoders = fields.iter().map(|NamedField { ident, canon_str, .. }| {
        quote! {
            ::tree_buf::internal::encode_ident(#canon_str, stream);
            stream.encode_with_id(|stream| self.#ident.encode_root(stream));
        }
    });

    let array_fields = fields.iter().map(|NamedField { ident, ty, .. }| {
        quote! {
            #ident: <#ty as ::tree_buf::internal::Encodable>::EncoderArray
        }
    });

    let buffers = fields.iter().map(|NamedField { ident, .. }| {
        quote! {
            self.#ident.buffer_one(&value.#ident);
        }
    });

    let flushers = fields.iter().map(|NamedField { ident, canon_str, ty }| {
        quote! {
            ::tree_buf::internal::encode_ident(#canon_str, stream);
            let o = self.#ident;
            stream.encode_with_id(|stream| ::tree_buf::internal::EncoderArray::<#ty>::flush(o, stream));
        }
    });

    let num_fields = fields.len();

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

    let encode_root = quote! {
        #prefix
        #(#encoders)*
        ::tree_buf::internal::RootTypeId::#suffix
    };

    fill_encode_skeleton(ast, array_fields, buffer, flush, encode_root)
}

fn fill_encode_skeleton<A: ToTokens>(
    ast: &DeriveInput,
    array_fields: impl Iterator<Item = A>,
    buffer: impl ToTokens,
    flush: impl ToTokens,
    encode_root: impl ToTokens,
) -> TokenStream {
    let name = &ast.ident;
    let vis = &ast.vis;
    let array_encoder_name = format_ident!("{}TreeBufEncoderArray", name);

    quote! {
        #[derive(Default)]
        #[allow(non_snake_case)]
        #vis struct #array_encoder_name {
            #(#array_fields,)*
        }

        impl ::tree_buf::internal::EncoderArray<#name> for #array_encoder_name {
            fn buffer_one<'a, 'b : 'a>(&'a mut self, value: &'b #name) {
                #buffer
            }
            fn flush<O: ::tree_buf::options::EncodeOptions>(mut self, stream: &mut ::tree_buf::internal::EncoderStream<'_, O>) -> ::tree_buf::internal::ArrayTypeId {
                let _profile_guard = ::tree_buf::internal::firestorm::start_guard(::tree_buf::internal::firestorm::FmtStr::Str3(::std::any::type_name::<Self>(), "::", "flush"));
                #flush
            }
        }

        impl ::tree_buf::internal::Encodable for #name {
            type EncoderArray=#array_encoder_name;
            fn encode_root<O: ::tree_buf::options::EncodeOptions>(&self, stream: &mut ::tree_buf::internal::EncoderStream<'_, O>) -> tree_buf::internal::RootTypeId {
                let _profile_guard = ::tree_buf::internal::firestorm::start_guard(::tree_buf::internal::firestorm::FmtStr::Str3(::std::any::type_name::<Self>(), "::", "flush"));
                #encode_root
            }
        }
    }
}

fn impl_enum_encode(ast: &DeriveInput, data_enum: &DataEnum) -> TokenStream {
    // What is needed:
    // An outer struct containing encoders for each variant
    // For each variant, possibly it's own encoder struct if it's a tuple or struct sort of DataEnum
    // A discriminant

    let mut array_fields = Vec::new();
    array_fields.push(quote! {
        // TODO: (Performance) have the size scale to the number of variants
        tree_buf_discriminant: <u64 as ::tree_buf::Encodable>::EncoderArray,
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
            Fields::Unit => {
                root_matches.push(quote! {
                    #ident::#variant_ident => {
                        ::tree_buf::internal::encode_ident(#discriminant, stream);
                        stream.encode_with_id(|stream| ::tree_buf::internal::RootTypeId::Void);
                    }
                });
                array_fields.push(quote! {
                    #variant_ident: Option<u64>
                });
                array_matches.push(quote! {
                    #ident::#variant_ident => {
                        let t = if let Some(t) = self.#variant_ident {
                            t
                        } else {
                            let current = self.tree_buf_next_discriminant;
                            self.#variant_ident = Some(current);
                            self.tree_buf_next_discriminant += 1;
                            current
                        };
                        self.tree_buf_discriminant.buffer_one(&t);
                    }
                });
                flushes.push(quote! {
                    let mut matches = false;
                    if let Some(d) = &self.#variant_ident {
                        if *d == current_discriminant {
                            matches = true;
                        }
                    }
                    if matches {
                        ::tree_buf::internal::encode_ident(#discriminant, stream);
                        stream.encode_with_id(|stream| ::tree_buf::internal::ArrayTypeId::Void);
                        continue;
                    }
                });
            }
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
                                ::tree_buf::internal::encode_ident(#discriminant, stream);
                                stream.encode_with_id(|stream| _0.encode_root(stream));
                            }
                        });
                        array_fields.push(quote! {
                            #variant_ident: Option<(u64, <#ty as ::tree_buf::Encodable>::EncoderArray)>
                        });
                        array_matches.push(quote! {
                            #ident::#variant_ident(_0) => {
                                if self.#variant_ident.is_none() {
                                    self.#variant_ident = Some((self.tree_buf_next_discriminant, Default::default()));
                                    self.tree_buf_next_discriminant += 1;
                                }
                                let t = self.#variant_ident.as_mut().unwrap();
                                self.tree_buf_discriminant.buffer_one(&t.0);
                                t.1.buffer_one(_0);
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
                                ::tree_buf::internal::encode_ident(#discriminant, stream);
                                stream.encode_with_id(|stream| ::tree_buf::internal::EncoderArray::<#ty>:: flush(buffer, stream));
                                continue;
                            }
                        });
                    }
                    _ => todo!("Enums with multiple unnamed fields not yet supported by tree-buf Encode"),
                }
            }
        }
    }

    let encode_root = quote! {
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
        stream.encode_with_id(|stream| _0.flush(stream));

        for current_discriminant in 0..variant_count {
            #(#flushes)*
        }

        ::tree_buf::internal::ArrayTypeId::Enum
    };

    fill_encode_skeleton(ast, array_fields.iter(), buffer, flush, encode_root)
}
