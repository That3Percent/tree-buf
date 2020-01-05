use crate::prelude::*;

pub mod array;
pub mod integer;
pub mod object;
pub mod nullable;
pub mod boolean;
pub mod string;

pub use {array::*, integer::*, object::*, nullable::*, boolean::*, string::*};

use std::mem::transmute;

pub unsafe trait Wrapper : Sized {
    type Inner;

    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) where Self::Inner : BatchData {
        unsafe { Self::Inner::write_batch(transmute(items), bytes) }
    }
    fn read_batch(bytes: &[u8]) -> Vec<Self> where Self::Inner : BatchData {
        unsafe { transmute(Self::Inner::read_batch(bytes)) }
    }
}
