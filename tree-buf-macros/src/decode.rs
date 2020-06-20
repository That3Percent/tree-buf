use {
    crate::utils::{canonical_ident, get_named_fields, NamedField},
    proc_macro2::TokenStream,
    quote::ToTokens,
    syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, FieldsUnnamed},
};

pub fn impl_decode_macro(ast: &DeriveInput) -> TokenStream {
    match &ast.data {
        Data::Struct(data_struct) => impl_struct_decode(ast, data_struct),
        Data::Enum(data_enum) => impl_enum_decode(ast, data_enum),
        Data::Union(_) => panic!("Unions are not supported by tree-buf"),
    }
}

fn impl_struct_decode(ast: &DeriveInput, data_struct: &DataStruct) -> TokenStream {
    let fields = get_named_fields(data_struct);
    let name = &ast.ident;

    let inits = fields
        .iter()
        .map(|NamedField { ident, canon_str, .. }| {
            quote! {
                let #ident = fields.remove(#canon_str).unwrap_or_default();
            }
        })
        .collect::<Vec<_>>();
    let unwraps = fields
        .iter()
        .map(|NamedField { ident, .. }| {
            quote! {
                #ident: #ident?,
            }
        })
        .collect::<Vec<_>>();

    let mut parallel_lhs = quote! {};
    let mut decodes_parallel_rhs = quote! {};
    let mut news_parallel_rhs = quote! {};
    let mut is_first = true;

    for NamedField { ident, ty, .. } in fields.iter() {
        if is_first {
            is_first = false;
            parallel_lhs = quote! { #ident };
            decodes_parallel_rhs = quote! {
                <#ty as ::tree_buf::internal::Decodable>::decode(
                    #ident,
                    options,
                )
            };
            news_parallel_rhs = quote! {
                ::tree_buf::internal::DecoderArray::new(#ident, options)
            };
        } else {
            parallel_lhs = quote! { (#ident, #parallel_lhs) };
            decodes_parallel_rhs = quote! {
                ::tree_buf::internal::parallel(
                    || <#ty as ::tree_buf::internal::Decodable>::decode(
                        #ident,
                        options,
                    ),
                    || #decodes_parallel_rhs,
                    options
                )
            };
            news_parallel_rhs = quote! {
                ::tree_buf::internal::parallel(
                    || ::tree_buf::internal::DecoderArray::new(#ident, options),
                    || #news_parallel_rhs,
                    options
                )
            }
        }
    }

    let array_fields = fields.iter().map(|NamedField { ident, ty, .. }| {
        quote! {
            #ident: <#ty as ::tree_buf::internal::Decodable>::DecoderArray
        }
    });

    let decode_nexts = fields.iter().map(|NamedField { ident, .. }| {
        quote! {
            // Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
            #ident: match self.#ident.decode_next() { Ok(v) => v, Err(e) => { return Err(e.into()); } },
        }
    });

    let decode = quote! {
        let mut fields = match sticks {
            ::tree_buf::internal::DynRootBranch::Object { fields } => fields,
            _ => return Err(::tree_buf::DecodeError::SchemaMismatch),
        };

        #(#inits)*

        let #parallel_lhs = #decodes_parallel_rhs;

        Ok(Self {
            #(#unwraps)*
        })
    };
    let new = quote! {
        let mut fields = match sticks {
            ::tree_buf::internal::DynArrayBranch::Object { fields } => fields,
            _ => return Err(::tree_buf::DecodeError::SchemaMismatch),
        };

        #(#inits)*

        let #parallel_lhs = #news_parallel_rhs;

        Ok(Self {
            #(#unwraps)*
        })
    };

    let decode_next = quote! {
        Ok(#name {
            #(#decode_nexts)*
        })
    };

    fill_decode_skeleton(ast, decode, array_fields, new, decode_next)
}

fn fill_decode_skeleton<A: ToTokens>(
    ast: &DeriveInput,
    decode: impl ToTokens,
    array_fields: impl Iterator<Item = A>,
    new: impl ToTokens,
    decode_next: impl ToTokens,
) -> TokenStream {
    let name = &ast.ident;
    let vis = &ast.vis;
    let array_decoder_name = format_ident!("{}TreeBufDecoderArray", name);

    quote! {
        #[allow(non_snake_case)]
        impl ::tree_buf::internal::Decodable for #name {
            type DecoderArray = #array_decoder_name;
            fn decode(sticks: ::tree_buf::internal::DynRootBranch<'_>, options: &impl ::tree_buf::options::DecodeOptions) -> Result<Self, ::tree_buf::DecodeError> {
                let _profile_guard = ::tree_buf::internal::firestorm::start_guard(::tree_buf::internal::firestorm::FmtStr::Str3(::std::any::type_name::<Self>(), "::", "decode"));
                #decode
            }
        }

        #[allow(non_snake_case)]
        #vis struct #array_decoder_name {
            #(#array_fields,)*
        }

        #[allow(non_snake_case)]
        impl ::tree_buf::internal::DecoderArray for #array_decoder_name {
            type Decode=#name;
            // TODO: See if sometimes we can use Infallible here.
            type Error=::tree_buf::DecodeError;
            fn new(sticks: ::tree_buf::internal::DynArrayBranch<'_>, options: &impl ::tree_buf::options::DecodeOptions) -> Result<Self, ::tree_buf::DecodeError> {
                let _profile_guard = ::tree_buf::internal::firestorm::start_guard(::tree_buf::internal::firestorm::FmtStr::Str3(::std::any::type_name::<Self>(), "::", "decode"));
                #new
            }
            fn decode_next(&mut self) -> ::std::result::Result<Self::Decode, Self::Error> {
                #decode_next
            }
        }
    }
}

fn impl_enum_decode(ast: &DeriveInput, data_enum: &DataEnum) -> TokenStream {
    let ident = &ast.ident;
    let mut array_fields = Vec::new();
    array_fields.push(quote! {
        tree_buf_discriminant: <u64 as ::tree_buf::Decodable>::DecoderArray
    });

    let mut new_matches = Vec::new();
    let mut new_inits = Vec::new();
    let mut decode_nexts = Vec::new();
    let mut new_unpacks = Vec::new();
    let mut new_parallel_lhs = quote! { tree_buf_discriminant };
    let mut new_parallel_rhs = quote! { ::tree_buf::internal::DecoderArray::new(tree_buf_discriminant, options) };

    let mut root_matches = Vec::new();

    for variant in data_enum.variants.iter() {
        let variant_ident = &variant.ident;
        let discriminant = canonical_ident(variant_ident);

        match &variant.fields {
            Fields::Unit => {
                root_matches.push(quote! {
                    // TODO: Verify that the branch is the void type?
                    #discriminant => Self::#variant_ident,
                });
                array_fields.push(quote! {
                    #variant_ident: Option<u64>
                });
                new_unpacks.push(quote! { #variant_ident: #variant_ident, });
                new_matches.push(quote! {
                    #discriminant => {
                        if #variant_ident.is_some() {
                            return Err(::tree_buf::DecodeError::InvalidFormat);
                        }
                        #variant_ident = Some(index as u64);
                    }
                });
                new_inits.push(quote! {
                    let mut #variant_ident = None;
                });

                decode_nexts.push(quote! {
                    if let Some(d) = &mut self.#variant_ident {
                        if *d == discriminant {
                            return Ok(#ident::#variant_ident);
                        }
                    }
                });
            }
            Fields::Named(_named_fields) => todo!("Enums with named fields not yet supported by tree-buf decode"),
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                match unnamed.len() {
                    // TODO: Check if this is really unreachable. It might be `Variant {}`
                    0 => unreachable!(),
                    1 => {
                        root_matches.push(quote! {
                            #discriminant => {
                                Self::#variant_ident(::tree_buf::internal::Decodable::decode(*value, options)?)
                            },
                        });
                        let ty = &unnamed[0].ty;
                        array_fields.push(quote! {
                            #variant_ident: Option<(u64, <#ty as ::tree_buf::internal::Decodable>::DecoderArray)>
                        });
                        new_unpacks.push(quote! { #variant_ident: #variant_ident.transpose()?, });
                        new_parallel_lhs = quote! { (#variant_ident, #new_parallel_lhs) };
                        new_parallel_rhs = quote! {
                            ::tree_buf::internal::parallel(
                                || #variant_ident.map(|(i, d)| { ::tree_buf::internal::DecoderArray::new(d, options).map(|v| (i, v)) }),
                                || #new_parallel_rhs,
                                options
                            )
                        };
                        new_matches.push(quote! {
                            #discriminant => {
                                if #variant_ident.is_some() {
                                    return Err(::tree_buf::DecodeError::InvalidFormat);
                                }
                                #variant_ident = Some(
                                    (index as u64, data)
                                );
                            }
                        });
                        new_inits.push(quote! {
                            let mut #variant_ident = None;
                        });
                        decode_nexts.push(quote! {
                            if let Some((d, r)) = &mut self.#variant_ident {
                                if *d == discriminant {
                                    // Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
                                    return Ok(#ident::#variant_ident(match r.decode_next() { Ok(v) => v, Err(e) => return Err(e.into()) }));
                                }
                            }
                        })
                    }
                    _ => todo!("Enums with multiple unnamed fields not yet supported by tree-buf Decode"),
                }
            }
        }
    }

    let decode = quote! {
        // If this is an enum,
        if let ::tree_buf::internal::DynRootBranch::Enum { discriminant, value } = sticks {
            Ok(
                // See if it's a variant we are aware of, and that the value
                // matches the expected data.
                match discriminant {
                    #(#root_matches)*
                    _ => { return Err(::tree_buf::DecodeError::SchemaMismatch); },
                }
            )
        } else {
            Err(::tree_buf::DecodeError::SchemaMismatch)
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
                        _ => { return Err(::tree_buf::DecodeError::SchemaMismatch); }
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
                Err(::tree_buf::DecodeError::SchemaMismatch)
            }
        }
    };

    let decode_next = quote! {
        let discriminant = ::tree_buf::internal::InfallibleDecoderArray::decode_next_infallible(&mut self.tree_buf_discriminant);
        #(#decode_nexts)*

        // See also: fb0a3c86-23be-4d4a-9dbf-9c83ae6e2f0f
        todo!("Make this unreachable by verifying range");
    };

    fill_decode_skeleton(ast, decode, array_fields.iter(), new, decode_next)
}
