extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

mod utils;
mod read;
mod write;

use {
    read::impl_read_macro,
    write::impl_write_macro,
    syn::{parse_macro_input, DeriveInput},
};



#[proc_macro_derive(Write)]
pub fn write_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let output = impl_write_macro(&ast);
    proc_macro::TokenStream::from(output)
}

#[proc_macro_derive(Read)]
pub fn read_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let output = impl_read_macro(&ast);
    proc_macro::TokenStream::from(output)
}