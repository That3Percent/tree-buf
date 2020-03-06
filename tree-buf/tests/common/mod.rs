#![allow(dead_code)] // TODO: This shouldn't be needed. Consider filing a bug because not all of the common module is used by all tests

use std::fmt::Debug;
use tree_buf::prelude::*;
use tree_buf::{Readable, Writable};

/// Asserts that the serialized value deserializes to the same value.
/// Asserts a specific size. If we get a number above this size, that's a fail.
/// If we add compression and achieve lower, we can ratchet the number down.
/// This ensures the use of the format is improving.
pub fn round_trip<'a, 'b: 'a, T: Readable + Writable<'a> + Debug + PartialEq>(value: &'b T, size: usize) {
    serialize_eq(value, value, size);
}

pub fn serialize_eq<'a, I: Writable<'a>, O: Readable + Debug + PartialEq>(i: &'a I, o: &'a O, size: usize) {
    let bytes = write(i);
    let result = read(&bytes);
    match result {
        Ok(parsed) => assert_eq!(o, &parsed),
        Err(e) => assert!(false, "{}", e),
    }
    assert_eq!(bytes.len(), size);
}

pub fn round_trip_default<T: for<'a> Default + Readable + for<'a> Writable<'a> + Debug + PartialEq>(size: usize) {
    let data = T::default();
    round_trip(&data, size);
}
