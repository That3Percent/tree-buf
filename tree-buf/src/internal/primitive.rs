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
    Object { num_fields: usize },
    Array, // TODO: Support fixed length in primitive id
    Nullable,
    // TODO: The idea for int is to always encode up to 64 bit values,
    // but for any data store the min value and offset first, then use
    // that to select an optimal encoding. When deserializing, the min and
    // offset can be used to find if the data type required by the schema
    // matches.
    // Consider something like this - https://lemire.me/blog/2012/09/12/fast-integer-compression-decoding-billions-of-integers-per-second/
    Integer,
    Boolean,

    // TODO: String,
    // TODO: Bytes = [u8]
    // TODO: Date
    // TODO: Void
    // TODO: Enum - Something like this... needs to simmer.
    //              The enum primitive id contains 1 number which is the discriminant count.
    //              The enum discriminant as int is contained in the enum branch
    //              Each sub-branch contains the discriminant name (string)
    //              Each branch may have a sub-branch for data belonging to the variant for that discriminant in each entry.
    //              In many cases, this data will be Void, which may be wasteful to have a branch for.
    //              ..
    //              Because enum is so flexible, it's possible to wrap some dynamic data into it. Eg: EnumValue<T>.
    //              This would create some number of sub-branches 'dynamically'.
    
}

impl PrimitiveId {
    pub(crate) fn write(self: &Self, bytes: &mut Vec<u8>) {
        use PrimitiveId::*;
        dbg!(self);
        dbg!(bytes.len());
        match self {
            Object { num_fields } => {
                bytes.push(1);
                encode_prefix_varint(*num_fields as u64, bytes);
            },
            _ => {
                let discriminant = match self {
                    Object {..} => unreachable!(),
                    Array => 2,
                    Nullable => 3,
                    Integer => 4,
                    Boolean => 5,
                };
                bytes.push(discriminant);
            }
        }
    }
    pub(crate) fn read(bytes: &[u8], offset: &mut usize) -> Self {
        use PrimitiveId::*;
        let discriminant = bytes[*offset];
        *offset += 1;
        match discriminant {
            1 => Object { num_fields: decode_prefix_varint(bytes, offset) as usize },
            2 => Array,
            3 => Nullable,
            4 => Integer,
            5 => Boolean,
            _ => todo!("error handling. {}", discriminant),
        }
    }
}


pub trait BatchData: Sized {
    fn read_batch(bytes: &[u8]) -> Vec<Self>;
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>);
    fn write_one(item: Self, bytes: &mut Vec<u8>) {
        // TODO: Overload these
        Self::write_batch(&[item], bytes)
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> Self {
        todo!()
    }
}



impl Primitive for usize {
    // TODO: I wrote this earlier, but now I'm not sure it makes sense.
    // usize gets it's own primitive which uses varint because we don't know the platform and maximum value here.
    // This enables support for arbitrarily large indices, with runtime errors for values unsupported by the platform
    fn id() -> PrimitiveId {
        PrimitiveId::Integer
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
    fn flush<ParentBranch: StaticBranch>(self, branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
        // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712}
        T::id().write(bytes);

        if ParentBranch::self_in_array_context() {
            let start = bytes.len();
            T::write_batch(&self.values, bytes);
            let len = bytes.len() - start;
            lens.push(len);
        } else {
            let Self { mut values, .. } = self;
            // TODO: This may be 0 for Object
            assert_eq!(values.len(), 1);
            let value = values.pop().unwrap();
            T::write_one(value, bytes);

        }
    }
}

impl<T: Primitive + Copy> Reader for PrimitiveBuffer<T> {
    type Read = T;
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch, branch: ParentBranch) -> Self {
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