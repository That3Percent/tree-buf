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
