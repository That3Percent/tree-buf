
use tree_buf::prelude::*;
use std::fmt::Debug;

// TODO: This works on structs, add for tuples
// TODO: Consider whether this should really be automatic, or
// part of a more general default on missing or func on missing kind of thing
#[test]
fn add_option_defaults_to_none() {
    #[derive(Write)]
    pub struct Before {
        x: u32,
    }
    #[derive(Debug, Read, PartialEq)]
    pub struct After {
        x: u32,
        y: Option<u32>,
    }

    let before = Before { x: 1 };
    let bytes = write(&before);
    let result = read(&bytes);
    assert_eq!(Ok(After { x: 1, y: None }), result);
}