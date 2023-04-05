// TODO: Remove this allow when scratch is used
#![allow(dead_code)]
use crate::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct Scratch {
    buffers: Rc<RefCell<BufferPool>>,
}

impl Scratch {
    pub(crate) fn take_buffer<T: Copy>(&self) -> Buffer<T> {
        self.buffers.borrow_mut().take()
    }
    pub(crate) fn put_buffer<T>(&self, buffer: Buffer<T>) {
        self.buffers.borrow_mut().put(buffer)
    }
}

/// A re-usable object which may increase performance when encoding over and over again
/// in a loop. Avoids allocations
pub fn scratch<T: Encodable>() -> Scratch {
    Scratch { buffers: Default::default() }
}

pub fn encode_into_with_scratch<T: Encodable>(_value: &T, _scratch: &mut Scratch, _into: &mut [u8]) {
    todo!()
}
