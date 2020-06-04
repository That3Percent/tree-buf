use std::fmt::Debug;
use tree_buf::prelude::*;
use tree_buf::DecodeError;
use tree_buf::{Decodable, Encodable};

fn expect_schema_mismatch<TIn: Encodable + Default, TOut: Debug + Decodable>() {
    let data = TIn::default();
    let bytes = encode(&data);
    let result = decode::<TOut>(&bytes);
    match result.unwrap_err() {
        DecodeError::SchemaMismatch => (),
        _ => assert!(false),
    }
}

#[test]
fn mismatched_root() {
    expect_schema_mismatch::<u64, String>();
}

#[test]
fn missnamed_obj_field() {
    #[derive(Default, Encode)]
    pub struct X {
        x: u64,
    }
    #[derive(Decode, Debug)]
    pub struct Y {
        y: u64,
    }

    expect_schema_mismatch::<X, Y>();
}

#[test]
fn wrong_tuple_length() {
    expect_schema_mismatch::<(u64, u64), (u64, u64, u64)>();
}
