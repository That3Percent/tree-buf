extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
use inflector::cases::camelcase::to_camel_case;

use proc_macro2::{Ident, TokenStream};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

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
    let name = &ast.ident;
    let span = name.span();
    let fields = get_named_fields(&ast.data);
    let vis = &ast.vis;

    let array_writer_name = format_ident!("{}TreeBufArrayWriter", name);

    let writers = fields.iter().map(|NamedField {ident, ty, canon_str}| {
        quote! {
            tree_buf::internal::write_str(#canon_str, bytes);
            let type_index = bytes.len();
            bytes.push(0);
            let type_id = <#ty as tree_buf::internal::Writable>::write_root(&value.#ident, bytes, lens);
            bytes[type_index] = type_id.into();
        }
    });

    let array_fields = fields.iter().map(|NamedField {ident, ty, ..}| {
        quote! {
            #ident: <#ty as tree_buf::internal::Writable<'a>>::WriterArray,
        }
    });

    let buffers = fields.iter().map(|NamedField {ident, ..}| {
        quote! {
            self.#ident.buffer(&value.#ident);
        }
    });

    let flushers = fields.iter().map(|NamedField {ident, canon_str, ..}| {
        quote! {
            tree_buf::internal::write_str(#canon_str, bytes);
            let type_index = bytes.len();
            bytes.push(0);
            let type_id = self.#ident.flush(bytes, lens);
            bytes[type_index] = type_id.into();
        }
    });

    let num_fields = fields.len();

    // See also: fadaec14-35ad-4dc1-b6dc-6106ab811669
    let (prefix, suffix) = match num_fields {
        0..=8 => (quote! {}, Ident::new(format!("Obj{}", num_fields).as_str(), span.clone())),
        _ => (
            quote! {
                tree_buf::internal::encodings::varint::encode_prefix_varint(#num_fields as u64 - 9, bytes);
            },
            Ident::new("ObjN", span.clone()),
        ),
    };

    // TODO: pub/private needs to match outer type.
    let tokens = quote! {
        #[derive(Default)]
        #vis struct #array_writer_name<'a> {
            #(#array_fields)*
        }

        impl<'a> tree_buf::internal::WriterArray<'a> for #array_writer_name<'a> {
            type Write=#name;
            fn buffer<'b : 'a>(&mut self, value: &'b Self::Write) {
                #(#buffers)*
            }
            fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> tree_buf::internal::ArrayTypeId {
                #prefix
                #(#flushers)*
                tree_buf::internal::ArrayTypeId::#suffix
            }
        }

        impl<'a> tree_buf::internal::Writable<'a> for #name {
            type WriterArray=#array_writer_name<'a>;
            fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> tree_buf::internal::RootTypeId {
                #prefix
                #(#writers)*
                tree_buf::internal::RootTypeId::#suffix
            }
        }
    };
    tokens.into()
}

#[proc_macro_derive(Read)]
pub fn read_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let output = impl_read_macro(&ast);
    proc_macro::TokenStream::from(output)
}

fn impl_read_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let array_reader_name = Ident::new(format!("{}TreeBufArrayReader", name).as_str(), ast.ident.span());
    let fields = get_named_fields(&ast.data);
    let vis = &ast.vis;

    let reads = fields.iter().map(|NamedField {ident, ty, canon_str}| {
        quote! {
            #ident: <#ty as tree_buf::internal::Readable>::read(
                children.remove(#canon_str).unwrap_or_default()
            )?,
        }
    });

    let news = fields.iter().map(|NamedField {ident, canon_str, ..}| {
        quote! {
            #ident: tree_buf::internal::ReaderArray::new(children.remove(#canon_str).unwrap_or_default())?,
        }
    });

    let array_fields = fields.iter().map(|NamedField {ident, ty, ..}| {
        quote! {
            #ident: <#ty as tree_buf::internal::Readable>::ReaderArray,
        }
    });

    let read_nexts = fields.iter().map(|NamedField {ident, ..}| {
        quote! {
            #ident: self.#ident.read_next()?,
        }
    });

    let tokens = quote! {
        impl tree_buf::internal::Readable for #name {
            type ReaderArray = #array_reader_name;
            fn read(sticks: tree_buf::internal::DynRootBranch<'_>) -> Result<Self, tree_buf::ReadError> {
                let mut children = match sticks {
                    tree_buf::internal::DynRootBranch::Object { children } => children,
                    _ => return Err(tree_buf::ReadError::SchemaMismatch),
                };

                Ok(Self {
                    #(#reads)*
                })
            }
        }
        #vis struct #array_reader_name {
            #(#array_fields)*
        }

        impl tree_buf::internal::ReaderArray for #array_reader_name {
            type Read=#name;
            fn new(sticks: tree_buf::internal::DynArrayBranch<'_>) -> Result<Self, tree_buf::ReadError> {
                let mut children = match sticks {
                    tree_buf::internal::DynArrayBranch::Object { children } => children,
                    _ => return Err(tree_buf::ReadError::SchemaMismatch),
                };

                Ok(Self {
                    #(#news)*
                })
            }
            fn read_next(&mut self) -> Result<Self::Read, tree_buf::ReadError> {
                Ok(#name {
                    #(#read_nexts)*
                })
            }
        }
    };

    tokens.into()
}

fn get_named_fields(data: &Data) -> NamedFields {
    // TODO: Lift restrictions
    let data_struct = match data {
        Data::Struct(data_struct) => data_struct,
        _ => panic!("The struct must be a data struct, not an enum or union"),
    };

    // TODO: Lift restriction
    let fields_named = match &data_struct.fields {
        Fields::Named(fields_named) => fields_named,
        _ => panic!("The struct must have named fields"),
    };

    fields_named.named.iter().map(|field| {
        let ident = field.ident.as_ref().unwrap();
        NamedField {
            ident: field.ident.as_ref().unwrap(),
            ty: &field.ty,
            canon_str: canonical_ident(&ident),
        }
    }).collect()
}

// TODO: Document this
// TODO: Ensure that leading separators are preserved?
fn canonical_ident(ident: &Ident) -> String {
    let ident_str = format!("{}", ident);
    to_camel_case(&ident_str)
}
