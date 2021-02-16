extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

mod decode;
mod encode;
mod utils;

use {
    decode::impl_decode_macro,
    encode::impl_encode_macro,
    syn::{parse_macro_input, DeriveInput},
};

#[proc_macro_derive(Encode)]
pub fn encode_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let output = impl_encode_macro(&ast);
    proc_macro::TokenStream::from(output)
}

#[proc_macro_derive(Decode)]
pub fn decode_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let output = impl_decode_macro(&ast);
    proc_macro::TokenStream::from(output)
}
