use tree_buf::prelude::*;
use tree_buf::ReadError;
use tree_buf::{Readable, Writable};
use std::fmt::Debug;


fn expect_schema_mismatch<TIn: Writable + Default, TOut: Debug + Readable>() {
    let data = TIn::default();
    let bytes = write(&data);
    let result = read::<TOut>(&bytes);
    match result.unwrap_err() {
        ReadError::SchemaMismatch => (),
        _ => assert!(false),
    }
}

#[test]
fn mismatched_root() {
    expect_schema_mismatch::<u32, f64>();
}

#[test]
fn missnamed_obj_field() {
    #[derive(Default, Write)]
    pub struct X { x: u32 }
    #[derive(Read, Debug)]
    pub struct Y { y: u32 }

    expect_schema_mismatch::<X, Y>();
}

#[test]
fn wrong_tuple_length() {
    expect_schema_mismatch::<(u32, u32), (u32, u32, u32)>();
}