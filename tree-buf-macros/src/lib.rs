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
        impl tree_buf::Writable for #name {
            type Writer = #writer_name;
        }
    }
}

fn impl_readable(name: &Ident, reader_name: &Ident) -> TokenStream {
    quote! {
        impl tree_buf::Readable for #name {
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
                #ident: tree_buf::Writer::new(),
            }
        }).collect();

    let new = quote! {
        fn new() -> Self {
            Self {
                _struct: tree_buf::Writer::new(),
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
            self._struct.write(&tree_buf::Struct);
            #(#writers)*
        }
    };

    let flushes: Vec<_> = 
        fields.iter().map(|(ident, _)| {
            let ident_str = format!("{}", ident);
            quote! {
                let #ident = tree_buf::BranchId { name: #ident_str, parent: _own_id };
                self.#ident.flush(&#ident, bytes);
            }
        }).collect();

    let flush = quote! {
        fn flush(&self, branch: &tree_buf::BranchId<'_>, bytes: &mut Vec<u8>) {
            let _own_id = bytes.len();
            self._struct.flush(branch, bytes);

            #(#flushes)*
        }
    };
    
    quote! {
        impl tree_buf::Writer for #writer_name {
            type Write = #name;
            #new
            #write
            #flush
        }
    }
}

fn impl_reader(name: &Ident, reader_name: &Ident, fields: &NamedFields) -> TokenStream {
    // TODO: I think finding the stick again here to get the own_id is unnecessary. We could
    // take a stick as argument and pass it to the primitive reader, which would then match the type.
    
    let inits: Vec<_> =
        fields.iter().map(|(ident, _)| {
            let ident_str = format!("{}", ident);
            quote! {
                #ident: tree_buf::Reader::new(
                    sticks, &tree_buf::BranchId { name: #ident_str, parent: own_id }
                ),
            }
        }).collect();
    let new = quote! {
        fn new(sticks: &Vec<tree_buf::Stick>, branch: &tree_buf::BranchId) -> Self {
            let own_id = branch.find_stick(sticks).unwrap().start; // TODO: Error handling
            let _struct = tree_buf::Reader::new(sticks, branch);

            Self {
                _struct,
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
            self._struct.read();
            Self::Read {
                #(#readers)*
            }
        }
    };

    quote! {
        impl tree_buf::Reader for #reader_name {
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
                #ident: <#ty as tree_buf::Writable>::Writer,
            }
        }).collect();

    quote! {
        pub struct #writer_name {
            _struct: <tree_buf::Struct as tree_buf::Writable>::Writer,
            #(#fields)*
        }
    }
}

fn impl_reader_struct(reader_name: &Ident, fields: &NamedFields) -> TokenStream {
    let fields: Vec<_> =
        fields.iter().map(|(ident, ty)| {
            quote! {
                #ident: <#ty as tree_buf::Readable>::Reader,
            }
        }).collect();

    quote! {
        pub struct #reader_name {
            _struct: <tree_buf::Struct as tree_buf::Readable>::Reader,
            #(#fields)*
        }
    }
}