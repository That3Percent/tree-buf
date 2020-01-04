use crate::prelude::*;
use crate::internal::encodings::{
    varint::{encode_prefix_varint, decode_prefix_varint, encode_suffix_varint},
};
use std::convert::{TryInto};
use std::fmt::Debug;

pub trait Primitive: Default + BatchData {
    fn id() -> PrimitiveId;
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum PrimitiveId {
    Object = 1,
    Array = 2, // TODO: Support fixed length in primitive id
    Nullable = 3,
    // TODO: The idea for int is to always encode up to 64 bit values,
    // but for any data store the min value and offset first, then use
    // that to select an optimal encoding. When deserializing, the min and
    // offset can be used to find if the data type required by the schema
    // matches.
    // Consider something like this - https://lemire.me/blog/2012/09/12/fast-integer-compression-decoding-billions-of-integers-per-second/
    Integer = 4,
    Boolean = 5,
    Usize = 6,
    String = 7,
    // TODO: Bytes = [u8]
    // TODO: Date
    
}

impl PrimitiveId {
    pub(crate) fn from_u32(v: u32) -> Self {
        use PrimitiveId::*;
        // TODO: Add some kind of check that all values are still correct.
        match v {
            1 => Object,
            2 => Array,
            3 => Nullable,
            4 => Integer,
            5 => Boolean,
            6 => Usize,
            7 => String,
            _ => todo!("error handling. {}", v),
        }
    }
}


pub trait BatchData: Sized {
    fn read_batch(bytes: &[u8]) -> Vec<Self>;
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>);
    fn write_one(item: Self, bytes: &mut Vec<u8>) {
        todo!()
    }
}


/// usize gets it's own primitive which uses varint because we don't know the platform and maximum value here.
/// This enables support for arbitrarily large indices, with runtime errors for values unsupported by the platform
impl Primitive for usize {
    fn id() -> PrimitiveId {
        PrimitiveId::Usize
    }
}

impl BatchData for usize {
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        read_all(bytes, |b, o| {
            let v = decode_prefix_varint(b, o);
            v.try_into().unwrap_or_else(|_| todo!()) // TODO: Error handling (which won't be needed when schema match occurs)
        })
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        for item in items {
            let v = (*item) as u64;
            encode_prefix_varint(v, bytes);
        }
    }
}


// TODO: String + &str will need their own special Writer implementation that blits bits immediately to a byte buffer

#[derive(Debug)]
pub struct PrimitiveBuffer<T> {
    values: Vec<T>,
    read_offset: usize,
}

impl<T: Primitive + Copy> Writer for PrimitiveBuffer<T> {
    type Write = T;
    fn new() -> Self {
        Self {
            values: Vec::new(),
            read_offset: 0,
        }
    }
    fn write(&mut self, value: &Self::Write) {
        self.values.push(*value);
    }
    fn flush<ParentBranch: StaticBranch>(&self, branch: ParentBranch, bytes: &mut Vec<u8>) {
        let start = bytes.len();

        // TODO: Move the number of children for a struct to the branch primitive id
        if let Some(name) = branch.name() {
            encode_prefix_varint(name.len() as u64, bytes);
            bytes.extend_from_slice(name.as_bytes());
        }

        // Write the primitive id
        // TODO: Include data for the primitive - like int ranges
        bytes.push(T::id() as u8);

        // TODO: If not in an array context, use T::write_one
        T::write_batch(&self.values, bytes);

        // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712} 
        // FIXME: This can't go here, it needs to go to the end of the file.
        if ParentBranch::self_in_array_context() {
            let size = (bytes.len() - start) as u64;
            encode_suffix_varint(size, bytes);
        }
    }
}

impl<T: Primitive + Copy> Reader for PrimitiveBuffer<T> {
    type Read = T;
    fn new<ParentBranch: StaticBranch>(sticks: &Vec<Stick<'_>>, branch: ParentBranch) -> Self {
        todo!()
        /*
        let stick = branch.find_stick(&sticks).unwrap(); // TODO: Error handling
        if stick.primitive != T::id() {
            todo!("error handling. {:?} {:?} {:?}", T::id(), branch, stick);
        }

        let values = T::read_batch(stick.bytes);
        Self { values, read_offset: 0 }
        */
    }
    fn read(&mut self) -> Self::Read {
        let value = self.values[self.read_offset];
        self.read_offset += 1;
        value
    }
}


impl<T: Primitive + Copy> Writable for T {
    type Writer = PrimitiveBuffer<T>;
}

impl<T: Primitive + Copy> Readable for T {
    type Reader = PrimitiveBuffer<T>;
}

// TODO: Split implementation for read/write
impl<T: Copy> PrimitiveBuffer<T> {
    pub fn read(&mut self) -> T {
        // TODO: Consider handling index out of bounds
        let value = self.values[self.read_offset];
        self.read_offset += 1;
        value
    }
}

impl<T: Primitive> PrimitiveBuffer<T> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            read_offset: 0,
        }
    }
}