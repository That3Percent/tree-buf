use crate::play::*;
use crate::branch::*;
use crate::reader_writer::*;
use crate::missing::*;
use crate::context::*;
use crate::error::*;


pub trait Primitive : Default {
    fn id() -> PrimitiveId;
    /// The return value (usize, usize) is to indicate the start and length
    /// of the data written to bytes. The implementation must append only,
    /// and may return a range that is a sub-range of the appended data.
    /// This API is to allow for padding and alignment in the future.
    /// TODO: Maybe a better idea is just to have layout requirements
    /// as an associated function, and this can be setup on the outside
    /// to handle it.
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) -> (usize, usize);
    fn read_batch(bytes: &[u8], count: usize) -> Vec<Self>;
}
// TODO: The interaction between Default and Missing here may be dubious.
// What it will ultimately infer is that the struct exists, but that all it's
// fields should also come up missing. Where this gets really sketchy though
// is that there may be no mechanism to ensure that none of it's fields actually
// do come up missing in the event of a name collision. I think what we actually
// want is to try falling back to the owning struct default implementation instead,
// but that would require Default on too much. Having the branch type be a part
// of the lookup somehow, or have missing be able to cancel the branch to something bogus may help.
#[derive(Copy, Clone, Default, Debug)]
pub struct Struct;
// The Default derive enables DefaultOnMissing to have an empty array
#[derive(Copy, Clone, Default, Debug)]
pub struct Array(usize);
// The Default derive enabled DefaultOnMissing to have None
#[derive(Copy, Clone, Default, Debug)]
pub struct Opt(bool);

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum PrimitiveId {
    Struct = 1,
    Array = 2,
    Opt = 3,
    U32 = 4,
    Bool = 5,
    Usize = 6,
    Str = 7,
    // TODO: [u8]
}



impl Primitive for Struct {
    fn id() -> PrimitiveId { PrimitiveId::Struct }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) -> (usize, usize) {
        (0, 0)
    }
    fn read_batch(bytes: &[u8], count: usize) -> Vec<Self> {
        vec![Self; count]
    }
}
impl Primitive for Array {
    fn id() -> PrimitiveId { PrimitiveId::Array }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) -> (usize, usize) {
        todo!();
    }
    fn read_batch(bytes: &[u8], count: usize) -> Vec<Self> {
        todo!();
    }
}
impl Primitive for Opt {
    fn id() -> PrimitiveId { PrimitiveId::Opt }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) -> (usize, usize) {
        todo!();
    }
    fn read_batch(bytes: &[u8], count: usize) -> Vec<Self> {
        todo!();
    }
}
impl Primitive for u32 {
    fn id() -> PrimitiveId { PrimitiveId::U32 }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) -> (usize, usize) {
        todo!();
    }
    fn read_batch(bytes: &[u8], count: usize) -> Vec<Self> {
        todo!();
    }
}

/// usize gets it's own primitive which uses varint because we don't know the platform and maximum value here.
/// This enables support for arbitrarily large indices, with runtime errors for values unsupported by the platform
impl Primitive for usize {
    fn id() -> PrimitiveId {
        PrimitiveId::Usize
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) -> (usize, usize) {
        todo!();
    }
    fn read_batch(bytes: &[u8], count: usize) -> Vec<Self> {
        todo!();
    }
}

impl Primitive for bool {
    fn id() -> PrimitiveId {
        PrimitiveId::Bool
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) -> (usize, usize) {
        todo!();
    }
    fn read_batch(bytes: &[u8], count: usize) -> Vec<Self> {
        todo!();
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
}

#[derive(Debug)]
pub struct VecWriter<T> {
    len: PrimitiveBuffer<Array>,
    values: T,
}

impl<T: Writer> Writer for VecWriter<T> {
    type Write=Vec<T::Write>;
    fn new() -> Self {
        Self {
            len: PrimitiveBuffer::new(),
            values: T::new(),
        }
    }
    fn write(&mut self, value: &Self::Write) {
        self.len.write(&Array(value.len()));
        for item in value.iter() {
            self.values.write(item);
        }
    }
}

impl<T: Primitive + Copy> Writable for T {
    type Writer=PrimitiveBuffer<T>;
}

impl<T: Primitive + Copy> Reader for T {
    fn read(context: &mut Context<'_>, branch: &Branch<'_>, missing: &impl Missing) -> Result<Self, Error> {
        let reader = context.get_reader::<T>(branch);
        match reader {
            Some(reader) => Ok(reader.read()),
            None => missing.missing(&branch)
        }
    }
}

pub struct OptionWriter<V> {
    opt: PrimitiveBuffer<Opt>,
    value: V,
}

impl<V: Writer> Writer for OptionWriter<V> {
    type Write=Option<V::Write>;
    fn new() -> Self {
        Self {
            opt: PrimitiveBuffer::new(),
            value: V::new(),
        }
    }
    fn write(&mut self, value: &Self::Write) {
        self.opt.write(&Opt(value.is_some()));
        if let Some(value) = value {
            self.value.write(value);
        }
    }
}

impl<T: Writable> Writable for Option<T> {
    type Writer=OptionWriter<T::Writer>;
}

impl<T: Reader> Reader for Option<T> {
    fn read(context: &mut Context<'_>, branch: &Branch<'_>, missing: &impl Missing) -> Result<Self, Error> {
        Ok(match Opt::read(context, branch, missing)?.0 {
            true => Some(T::read(context, &branch.child(""), missing)?),
            false => None,
        })
    }
}

impl<T: Writable> Writable for Vec<T> {
    type Writer=VecWriter<T::Writer>;
}

impl<T: Reader> Reader for Vec<T> {
    fn read(context: &mut Context<'_>, branch: &Branch<'_>, missing: &impl Missing) -> Result<Self, Error> {
        let length = Array::read(context, branch, missing)?.0;
        let items = branch.child("");
        let mut result = Vec::with_capacity(length);
        for _ in 0..length {
            result.push(T::read(context, &items, missing)?);
        }
        Ok(result)
    }
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