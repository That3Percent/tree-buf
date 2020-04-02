use std::fmt::Debug;
use tree_buf::prelude::*;
mod common;
use common::*;

// TODO: This works on structs, add for tuples
// TODO: Consider whether this should really be automatic, or
// part of a more general default on missing or func on missing kind of thing
#[test]
fn add_option_defaults_to_none() {
    #[derive(Write)]
    pub struct Before {
        x: u64,
    }
    #[derive(Debug, Read, PartialEq)]
    pub struct After {
        x: u64,
        y: Option<u64>,
    }

    serialize_eq(&Before { x: 1 }, &After { x: 1, y: None }, 4);
}

#[test]
fn canonical_idents_compatible() {
    #![allow(non_snake_case)]

    #[derive(Read, Write, PartialEq, Debug)]
    pub struct JavaScript {
        myName: u64,
    }

    #[derive(Read, Write, PartialEq, Debug)]
    pub struct Rust {
        my_name: u64,
    }

    let js = &JavaScript { myName: 1 };
    let rust = &Rust { my_name: 1 };

    serialize_eq(js, rust, 9);
    serialize_eq(rust, js, 9);
}

#[test]
fn fixed_array_to_vec() {
    let fixed = [0u8, 1, 2, 3, 4];
    let vec: Vec<_> = fixed.iter().copied().collect();
    serialize_eq(&fixed, &vec, 8);

    let fixed = [fixed, fixed];
    let vec: Vec<Vec<_>> = fixed.iter().map(|f| f.iter().copied().collect()).collect();
    serialize_eq(&fixed, &vec, 14);
}
