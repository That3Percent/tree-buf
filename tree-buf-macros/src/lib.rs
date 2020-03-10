extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
use inflector::cases::camelcase::to_camel_case;

use proc_macro2::{Ident, TokenStream, Span};
use syn::{parse_macro_input, Data, DataStruct, DataEnum, DeriveInput, Fields, Type, Visibility};

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
    let vis = &ast.vis;
    let array_writer_name = format_ident!("{}TreeBufArrayWriter", name);

    match &ast.data {
        Data::Struct(data_struct) => {
            impl_struct_write(name, &span, vis, &array_writer_name, data_struct)
        },
        Data::Enum(data_enum) => {
            impl_enum_write(name, &span, vis, &array_writer_name, data_enum)
        },
        Data::Union(_) => panic!("Unions are not supported by tree-buf"),
    }
}

fn impl_struct_write(name: &Ident, span: &Span, vis: &Visibility, array_writer_name: &Ident, data_struct: &DataStruct) -> TokenStream {
    let fields = get_named_fields(data_struct);


    let writers = fields.iter().map(|NamedField { ident, ty, canon_str }| {
        quote! {
            ::tree_buf::internal::write_str(#canon_str, bytes);
            let type_index = bytes.len();
            bytes.push(0);
            let type_id = <#ty as ::tree_buf::internal::Writable>::write_root(&value.#ident, bytes, lens, options);
            bytes[type_index] = type_id.into();
        }
    });

    let array_fields = fields.iter().map(|NamedField { ident, ty, .. }| {
        quote! {
            #ident: <#ty as ::tree_buf::internal::Writable<'a>>::WriterArray,
        }
    });

    let buffers = fields.iter().map(|NamedField { ident, .. }| {
        quote! {
            self.#ident.buffer(&value.#ident);
        }
    });

    let flushers = fields.iter().map(|NamedField { ident, canon_str, .. }| {
        quote! {
            ::tree_buf::internal::write_str(#canon_str, bytes);
            let type_index = bytes.len();
            bytes.push(0);
            let type_id = self.#ident.flush(bytes, lens, options);
            bytes[type_index] = type_id.into();
        }
    });

    let num_fields = fields.len();

    // See also: fadaec14-35ad-4dc1-b6dc-6106ab811669
    let (prefix, suffix) = match num_fields {
        0..=8 => (quote! {}, Ident::new(format!("Obj{}", num_fields).as_str(), span.clone())),
        _ => (
            quote! {
                ::tree_buf::internal::encodings::varint::encode_prefix_varint(#num_fields as u64 - 9, bytes);
            },
            Ident::new("ObjN", span.clone()),
        ),
    };

    quote! {
        #[derive(Default)]
        #vis struct #array_writer_name<'a> {
            #(#array_fields)*
        }

        impl<'a> ::tree_buf::internal::WriterArray<'a> for #array_writer_name<'a> {
            type Write=#name;
            fn buffer<'b : 'a>(&mut self, value: &'b Self::Write) {
                #(#buffers)*
            }
            fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>, options: &impl ::tree_buf::options::EncodeOptions) -> ::tree_buf::internal::ArrayTypeId {
                #prefix
                #(#flushers)*
                ::tree_buf::internal::ArrayTypeId::#suffix
            }
        }

        impl<'a> ::tree_buf::internal::Writable<'a> for #name {
            type WriterArray=#array_writer_name<'a>;
            fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>, options: &impl ::tree_buf::options::EncodeOptions) -> tree_buf::internal::RootTypeId {
                #prefix
                #(#writers)*
                ::tree_buf::internal::RootTypeId::#suffix
            }
        }
    }
}

fn impl_enum_write(name: &Ident, span: &Span, vis: &Visibility, array_writer_name: &Ident, data_enum: &DataEnum) -> TokenStream {
    quote! {
        #[derive(Default)]
        #vis struct #array_writer_name<'a> {
            // TODO: Add list for discriminants and values.
            _todo_remove: ::std::marker::PhantomData<&'a ()>,
        }

        impl<'a> ::tree_buf::internal::Writable<'a> for #name {
            type WriterArray=#array_writer_name<'a>;
            fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>, options: &impl ::tree_buf::options::EncodeOptions) -> tree_buf::internal::RootTypeId {
                todo!()
            }
        }

        impl<'a> ::tree_buf::internal::WriterArray<'a> for #array_writer_name<'a> {
            type Write=#name;
            fn buffer<'b : 'a>(&mut self, value: &'b Self::Write) {
                todo!()
            }
            fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>, options: &impl ::tree_buf::options::EncodeOptions) -> ::tree_buf::internal::ArrayTypeId {
                todo!()
            }
        }
    }
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
    let vis = &ast.vis;

    match &ast.data {
        Data::Struct(data_struct) => {
            impl_struct_read(name, vis, &array_reader_name, data_struct)
        },
        Data::Enum(data_enum) => {
            impl_enum_read(name, vis, &array_reader_name, data_enum)
        },
        Data::Union(_) => panic!("Unions are not supported by tree-buf"),
    }
}

fn impl_struct_read(name: &Ident, vis: &Visibility, array_reader_name: &Ident, data_struct: &DataStruct) -> TokenStream {
    let fields = get_named_fields(data_struct);

    let reads = fields.iter().map(|NamedField { ident, ty, canon_str }| {
        quote! {
            #ident: <#ty as ::tree_buf::internal::Readable>::read(
                children.remove(#canon_str).unwrap_or_default()
            )?,
        }
    });

    let news = fields.iter().map(|NamedField { ident, canon_str, .. }| {
        quote! {
            #ident: ::tree_buf::internal::ReaderArray::new(children.remove(#canon_str).unwrap_or_default())?,
        }
    });

    let array_fields = fields.iter().map(|NamedField { ident, ty, .. }| {
        quote! {
            #ident: <#ty as ::tree_buf::internal::Readable>::ReaderArray,
        }
    });

    let read_nexts = fields.iter().map(|NamedField { ident, .. }| {
        quote! {
            #ident: self.#ident.read_next()?,
        }
    });

    quote! {
        impl ::tree_buf::internal::Readable for #name {
            type ReaderArray = #array_reader_name;
            fn read(sticks: ::tree_buf::internal::DynRootBranch<'_>) -> Result<Self, ::tree_buf::ReadError> {
                let mut children = match sticks {
                    ::tree_buf::internal::DynRootBranch::Object { children } => children,
                    _ => return Err(::tree_buf::ReadError::SchemaMismatch),
                };

                Ok(Self {
                    #(#reads)*
                })
            }
        }
        #vis struct #array_reader_name {
            #(#array_fields)*
        }

        impl ::tree_buf::internal::ReaderArray for #array_reader_name {
            type Read=#name;
            fn new(sticks: ::tree_buf::internal::DynArrayBranch<'_>) -> Result<Self, ::tree_buf::ReadError> {
                let mut children = match sticks {
                    ::tree_buf::internal::DynArrayBranch::Object { children } => children,
                    _ => return Err(::tree_buf::ReadError::SchemaMismatch),
                };

                Ok(Self {
                    #(#news)*
                })
            }
            fn read_next(&mut self) -> Result<Self::Read, ::tree_buf::ReadError> {
                Ok(#name {
                    #(#read_nexts)*
                })
            }
        }
    }
}

fn impl_enum_read(name: &Ident, vis: &Visibility, array_reader_name: &Ident, data_enum: &DataEnum) -> TokenStream {
    quote! {
        impl ::tree_buf::internal::Readable for #name {
            type ReaderArray = #array_reader_name;
            fn read(sticks: ::tree_buf::internal::DynRootBranch<'_>) -> Result<Self, ::tree_buf::ReadError> {
                todo!()
            }
        }
        #vis struct #array_reader_name {
        }

        impl ::tree_buf::internal::ReaderArray for #array_reader_name {
            type Read=#name;
            fn new(sticks: ::tree_buf::internal::DynArrayBranch<'_>) -> Result<Self, ::tree_buf::ReadError> {
                todo!()
            }
            fn read_next(&mut self) -> Result<Self::Read, ::tree_buf::ReadError> {
                todo!()
            }
        }
    }
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
