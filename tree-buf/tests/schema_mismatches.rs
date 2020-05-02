use std::fmt::Debug;
use tree_buf::prelude::*;
use tree_buf::ReadError;
use tree_buf::{Readable, Writable};

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
    expect_schema_mismatch::<u64, String>();
}

#[test]
fn missnamed_obj_field() {
    #[derive(Default, Write)]
    pub struct X {
        x: u64,
    }
    #[derive(Read, Debug)]
    pub struct Y {
        y: u64,
    }

    expect_schema_mismatch::<X, Y>();
}

#[test]
fn wrong_tuple_length() {
    expect_schema_mismatch::<(u64, u64), (u64, u64, u64)>();
}
