use crate::prelude::*;
use crate::internal::encodings::{
    varint::{encode_prefix_varint, decode_prefix_varint},
};
use std::convert::{TryInto};
use std::fmt::Debug;
use std::vec::IntoIter;

pub trait Primitive: Default + BatchData {
    fn id() -> PrimitiveId;
    fn from_dyn_branch(branch: DynBranch) -> ReadResult<OneOrMany<Self>>;
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
    Float,
    Tuple { num_fields: usize },
    Void,
    String,

    // TODO: Tuple,
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
        match self {
            Object { num_fields } => {
                bytes.push(1);
                encode_prefix_varint(*num_fields as u64, bytes);
            },
            Tuple { num_fields } => {
                bytes.push(7);
                encode_prefix_varint(*num_fields as u64, bytes);
            }
            _ => {
                let discriminant = match self {
                    Object {..} => unreachable!(),
                    Tuple {..} => unreachable!(),
                    Array => 2,
                    Nullable => 3,
                    Integer => 4,
                    Boolean => 5,
                    Float => 6,
                    Void => 8,
                    String => 9,
                };
                bytes.push(discriminant);
            }
        }
    }
    pub(crate) fn read(bytes: &[u8], offset: &mut usize) -> ReadResult<Self> {
        use PrimitiveId::*;
        let discriminant = bytes[*offset];
        *offset += 1;
        Ok(match discriminant {
            1 => Object { num_fields: decode_prefix_varint(bytes, offset)? as usize },
            2 => Array,
            3 => Nullable,
            4 => Integer,
            5 => Boolean,
            6 => Float,
            7 => Tuple { num_fields: decode_prefix_varint(bytes, offset)? as usize },
            8 => Void,
            9 => String,
            _ => Err(ReadError::InvalidFormat)?,
        })
    }
}


pub trait BatchData: Sized {
    fn read_batch(bytes: &[u8]) -> ReadResult<Vec<Self>>;
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>);
    fn write_one(item: Self, bytes: &mut Vec<u8>) {
        // TODO: Overload these
        Self::write_batch(&[item], bytes)
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> ReadResult<Self>;
}


impl Primitive for usize {
    // TODO: I wrote this earlier, but now I'm not sure it makes sense.
    // usize gets it's own primitive which uses varint because we don't know the platform and maximum value here.
    // This enables support for arbitrarily large indices, with runtime errors for values unsupported by the platform
    fn id() -> PrimitiveId {
        PrimitiveId::Integer
    }
    fn from_dyn_branch(branch: DynBranch) -> ReadResult<OneOrMany<Self>> {
        match branch {
            DynBranch::Integer(r) => {
                Ok(match r {
                    OneOrMany::One(i) => OneOrMany::One(i as usize),
                    OneOrMany::Many(b) => OneOrMany::Many(b),
                })
            },
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

impl BatchData for usize {
    fn read_batch(bytes: &[u8]) -> ReadResult<Vec<Self>> {
        read_all(bytes, |b, o| {
            let v = decode_prefix_varint(b, o)?;
            Ok(v.try_into().unwrap_or_else(|_| todo!())) // TODO: Error handling (which won't be needed when schema match occurs)
        })
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        for item in items {
            let v = (*item) as u64;
            encode_prefix_varint(v, bytes);
        }
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> ReadResult<Self> {
        Ok(decode_prefix_varint(bytes, offset)? as usize)
    }
}


// TODO: String + &str will need their own special Writer implementation that blits bits immediately to a byte buffer

#[derive(Debug, Default)]
pub struct PrimitiveBuffer<T> {
    pub(crate) values: Vec<T>,
}

pub struct PrimitiveReader<T> {
    pub(crate) values: IntoIter<T>,
}

impl<T: BatchData> PrimitiveReader<T> {
    pub fn read_from(items: OneOrMany<T>) -> ReadResult<Self> {
        let values = match items {
            OneOrMany::One(one) => vec![one],
            OneOrMany::Many(bytes) => T::read_batch(bytes)?
        };
        Ok(Self {
            values: values.into_iter(),
        })
    }
}


/// This is a substitute for a blanket impl, since the blanket impl makes us run afoul of orphan rules.
/// The core problem is that if we want to impl<'a, T: Primitive> Writable for PrimitiveBuffer<T>, then we can't also impl Writer for Box<Writer>
#[macro_export]
macro_rules! impl_primitive_reader_writer {
    ($T:ty) => {
        impl Reader for PrimitiveReader<$T> {
            type Read = $T;
            fn new<ParentBranch: StaticBranch>(sticks: DynBranch, _branch: ParentBranch) -> ReadResult<Self> {
                let values = <$T>::from_dyn_branch(sticks)?;
                Self::read_from(values)
            }
            fn read(&mut self) -> ReadResult<Self::Read> {
                self.values.next().ok_or(ReadError::InvalidFormat)
            }
        }
        
        impl<'a> Writer<'a> for PrimitiveBuffer<$T> {
            type Write = $T;
            fn write<'b : 'a>(&mut self, value: &'a Self::Write) {
                self.values.push(*value);
            }
            fn flush<ParentBranch: StaticBranch>(self, _branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
                // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712}
                <$T>::id().write(bytes);

                if ParentBranch::in_array_context() {
                    let start = bytes.len();
                    BatchData::write_batch(&self.values, bytes);
                    let len = bytes.len() - start;
                    lens.push(len);
                } else {
                    let Self { mut values, .. } = self;
                    // TODO: This may be 0 for Object
                    assert_eq!(values.len(), 1);
                    let value = values.pop().unwrap();
                    <$T>::write_one(value, bytes);
                }
            }
        }

        impl<'a> Writable<'a> for $T {
            type Writer = PrimitiveBuffer<$T>;
        }
        
        impl Readable for $T {
            type Reader = PrimitiveReader<$T>;
        }
    };
}

impl_primitive_reader_writer!(usize);


impl<T: Primitive> PrimitiveBuffer<T> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
        }
    }
}