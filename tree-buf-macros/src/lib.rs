extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro2::{TokenStream, Ident};
use syn::{
    parse_macro_input, DeriveInput, Data, Fields, Type,
};

type NamedFields<'a> = Vec<(&'a Ident, &'a Type)>;

#[proc_macro_derive(Write)]
pub fn write_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let output = impl_write_macro(&ast);
    proc_macro::TokenStream::from(output)
}

fn impl_write_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let writer_name = Ident::new(format!("{}Writer", name).as_str(), ast.ident.span());
    let named_fields = get_named_fields(&ast.data);
    let writable = impl_writable(name, &writer_name);
    let writer_struct = impl_writer_struct(&writer_name, &named_fields);
    let writer = impl_writer(&name, &writer_name, &named_fields);
    let gen = quote! {
        #writer_struct
        #writer
        #writable
    };
    gen.into()
}

#[proc_macro_derive(Read)]
pub fn read_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let output = impl_read_macro(&ast);
    proc_macro::TokenStream::from(output)
    
}

fn impl_read_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let reader_name = Ident::new(format!("{}Reader", name).as_str(), ast.ident.span());
    let named_fields = get_named_fields(&ast.data);
    let readable = impl_readable(name, &reader_name);
    let reader_struct = impl_reader_struct(&reader_name, &named_fields);
    let reader = impl_reader(&name, &reader_name, &named_fields);
    let gen = quote! {
        #reader_struct
        #reader
        #readable
    };
    gen.into()
}



fn impl_writable(name: &Ident, writer_name: &Ident) -> TokenStream {
    quote! {
        impl tree_buf::internal::Writable for #name {
            type Writer = #writer_name;
        }
    }
}

fn impl_readable(name: &Ident, reader_name: &Ident) -> TokenStream {
    quote! {
        impl tree_buf::internal::Readable for #name {
            type Reader = #reader_name;
        }
    }
}

fn get_named_fields(data: &Data) -> NamedFields {
    // TODO: Lift restrictions
    let data_struct = match data {
        Data::Struct(data_struct) => data_struct,
        _ => panic!("The struct must be a data struct, not an enum or union")
    };

    // TODO: Lift restriction
    let fields_named = match &data_struct.fields {
        Fields::Named(fields_named) => fields_named,
        _=> panic!("The struct must have named fields"),
    };

    fields_named.named.iter().map(|field|
        (field.ident.as_ref().unwrap(), &field.ty)
    ).collect()
}


fn impl_writer(name: &Ident, writer_name: &Ident, fields: &NamedFields) -> TokenStream {
    let init: Vec<_> =
        fields.iter().map(|(ident, _)| {
            quote! {
                #ident: tree_buf::internal::Writer::new(),
            }
        }).collect();

    let new = quote! {
        fn new() -> Self {
            Self {
                #(#init)*
            }
        }
    };

    let writers: Vec<_> =
        fields.iter().map(|(ident, _)| {
            quote! {
                self.#ident.write(&value.#ident);
            }
        }).collect();

    // TODO: Writing the struct probably isn't necessary, just flushing the struct.
    let write = quote! {
        fn write(&mut self, value: &Self::Write) {
            #(#writers)*
        }
    };

    let flushes: Vec<_> = 
        fields.iter().map(|(ident, _)| {
            let ident_str = format!("{}", ident);
            quote! {
                tree_buf::internal::Str::write_one(#ident_str, bytes);
                self.#ident.flush(branch, bytes, lens);
            }
        }).collect();

    let num_fields = flushes.len();
    let flush = quote! {
        fn flush<ParentBranch: tree_buf::internal::StaticBranch>(self, branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
            // Do flush an Object branch as a marker and error check.
            tree_buf::internal::Object { num_fields: #num_fields }
                .flush(branch, bytes);

            #(#flushes)*
        }
    };
    
    quote! {
        impl tree_buf::internal::Writer for #writer_name {
            type Write = #name;
            #new
            #write
            #flush
        }
    }
}

fn impl_reader(name: &Ident, reader_name: &Ident, fields: &NamedFields) -> TokenStream {
    let inits: Vec<_> =
        fields.iter().map(|(ident, _)| {
            let ident_str = format!("{}", ident);
            quote! {
                #ident: tree_buf::internal::Reader::new(
                    children.remove(#ident_str).unwrap_or_else(|| todo!("schema mismatch error handling")),
                    branch,
                ),
            }
        }).collect();
    let new = quote! {
        fn new<ParentBranch: tree_buf::internal::StaticBranch>(sticks: tree_buf::internal::DynBranch, branch: ParentBranch) -> Self {
            let mut children = match sticks {
                tree_buf::internal::DynBranch::Object { children } => children,
                _ => todo!("schema mismatch error handling")
            };
            Self {
                #(#inits)*
            }
        }
    };

    let readers: Vec<_> = 
        fields.iter().map(|(ident, _)| {
            quote! {
                #ident: self.#ident.read(),
            }
        }).collect();

    let read = quote! {
        fn read(&mut self) -> Self::Read {
            Self::Read {
                #(#readers)*
            }
        }
    };

    quote! {
        impl tree_buf::internal::Reader for #reader_name {
            type Read = #name;
            #new
            #read
        }
    }
}

fn impl_writer_struct(writer_name: &Ident, fields: &NamedFields) -> TokenStream {
    let fields: Vec<_> =
        fields.iter().map(|(ident, ty)| {
            quote! {
                #ident: <#ty as tree_buf::internal::Writable>::Writer,
            }
        }).collect();

    quote! {
        pub struct #writer_name {
            #(#fields)*
        }
    }
}

fn impl_reader_struct(reader_name: &Ident, fields: &NamedFields) -> TokenStream {
    let fields: Vec<_> =
        fields.iter().map(|(ident, ty)| {
            quote! {
                #ident: <#ty as tree_buf::internal::Readable>::Reader,
            }
        }).collect();

    quote! {
        pub struct #reader_name {
            #(#fields)*
        }
    }
}