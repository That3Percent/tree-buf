use crate::prelude::*;

pub mod array;
pub mod integer;
pub mod object;
pub mod nullable;
pub mod boolean;
pub mod string;
pub mod bytes;
pub mod float;
pub mod tuple;

pub use {array::*, integer::*, object::*, nullable::*, boolean::*, string::*, bytes::*, float::*, tuple::*};

use std::mem::transmute;

pub unsafe trait Wrapper : Sized {
    type Inner: BatchData;

    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        unsafe { Self::Inner::write_batch(transmute(items), bytes) }
    }
    fn read_batch(bytes: &[u8]) -> ReadResult<Vec<Self>> {
        Ok(unsafe { transmute(Self::Inner::read_batch(bytes)?) })
    }
}
