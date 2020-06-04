#![allow(dead_code)] // TODO: This shouldn't be needed. Consider filing a bug because not all of the common module is used by all tests

use std::fmt::Debug;
use tree_buf::prelude::*;
use tree_buf::{Decodable, Encodable};

/// Asserts that the serialized value deserializes to the same value.
/// Asserts a specific size. If we get a number above this size, that's a fail.
/// If we add compression and achieve lower, we can ratchet the number down.
/// This ensures the use of the format is improving.
/// Works on both arrays and root values to hit both code paths.
pub fn round_trip<T: Encodable + Decodable + Clone + std::fmt::Debug + PartialEq + 'static>(value: &T, root_size: impl Into<Option<i32>>, array_size: impl Into<Option<i32>>)
// Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
where
    tree_buf::DecodeError: From<<<T as Decodable>::DecoderArray as tree_buf::internal::DecoderArray>::Error>,
{
    serialize_eq(value, value, root_size);
    let v = vec![value.clone(), value.clone()];
    serialize_eq(&v, &v, array_size);
}

pub fn serialize_eq<I: Encodable, O: Decodable + Debug + PartialEq>(i: &I, o: &O, size: impl Into<Option<i32>>)
// Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
where
    tree_buf::DecodeError: From<<<O as Decodable>::DecoderArray as tree_buf::internal::DecoderArray>::Error>,
{
    let bytes = encode(i);
    let result = decode(&bytes);
    //dbg!(tree_buf::internal::decode_root(&bytes));
    match result {
        Ok(parsed) => assert_eq!(o, &parsed),
        Err(e) => assert!(false, "{}", e),
    }
    if let Some(size) = size.into() {
        assert_eq!(bytes.len() as i32, size, "Size Before: {}\nSize After: {}", size, bytes.len());
    }
}

pub fn round_trip_default<T: Default + Decodable + Encodable + Debug + PartialEq + Clone + 'static>(root_size: i32, array_size: i32)
// Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
where
    tree_buf::DecodeError: From<<<T as Decodable>::DecoderArray as tree_buf::internal::DecoderArray>::Error>,
{
    let data = T::default();
    round_trip(&data, root_size, array_size);
}
