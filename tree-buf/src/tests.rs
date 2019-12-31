use crate::prelude::*;
use std::fmt::Debug;
use tree_buf_macros::{Read, Write};
use crate as tree_buf; // This warns about being unused, but it's used in the macro.

#[derive(Read, Write, PartialEq, Debug, Clone)]
struct Bits {
    int: u32,
    obj_array: Vec<Bobs>,
    extra: Option<Bobs>,
}


#[derive(Read, Write, PartialEq, Debug, Clone)]
struct Bobs {
    one: Vec<u32>,
}

fn make_item() -> Bits {
    Bits {
        int: 5,
        extra: Some(Bobs { one: vec![99] }),
        obj_array: vec![
            Bobs { one: vec![3, 2, 1, 0] },
            Bobs { one: vec![] },
            Bobs {
                one: vec![20, 20, 20, 20, 20, 20, 20],
            },
        ],
    }
}

fn round_trip<T: Readable + Writable + Debug + PartialEq>(value: &T) {
    let bytes = crate::write(value);
    let result = crate::read(&bytes);
    match result {
        Ok(parsed) => assert_eq!(value, &parsed),
        _ => assert!(false),
    }
}

#[test]
fn round_trip_item() {
    let item = make_item();
    round_trip(&item);
}

#[test]
fn round_trip_vec() {
    let item = make_item();
    let item = vec![item; 5];
    round_trip(&item);
}
