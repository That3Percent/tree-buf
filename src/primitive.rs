use crate::branch::*;
use crate::reader_writer::*;


pub trait Primitive : Default {
    fn id() -> PrimitiveId;
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>);
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
    Array = 2, // TODO: Support fixed length in primitive id
    Opt = 3,
    U32 = 4,
    Bool = 5,
    Usize = 6,
    Str = 7,
    // TODO: [u8]
}

impl PrimitiveId {
    pub(crate) fn from_u32(v: u32) -> Self {
        use PrimitiveId::*;
        // TODO: Add some kind of check that all values are still correct.
        match v {
            1 => Struct,
            2 => Array,
            3 => Opt,
            4 => U32,
            5 => Bool,
            6 => Usize,
            7 => Str,
            _ => todo!("error handling. {}", v)
        }
    }
}

impl Primitive for Struct {
    fn id() -> PrimitiveId { PrimitiveId::Struct }
    fn write_batch(_items: &[Self], _bytes: &mut Vec<u8>) { }
    fn read_batch(_bytes: &[u8], count: usize) -> Vec<Self> {
        vec![Self; count]
    }
}
impl Primitive for Array {
    fn id() -> PrimitiveId { PrimitiveId::Array }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        for item in items {
            item.0.write(bytes);
        }
    }
    fn read_batch(bytes: &[u8], count: usize) -> Vec<Self> {
        todo!();
    }
}
impl Primitive for Opt {
    fn id() -> PrimitiveId { PrimitiveId::Opt }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        for item in items {
            item.0.write(bytes);
        }
    }
    fn read_batch(bytes: &[u8], count: usize) -> Vec<Self> {
        todo!();
    }
}
impl Primitive for u32 {
    fn id() -> PrimitiveId { PrimitiveId::U32 }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        for item in items {
            item.write(bytes);
        }
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
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
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
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
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

// TODO: Most uses of this are temporary until compression is used.
pub(crate) trait EzBytes {
    type Out : std::borrow::Borrow<[u8]>;
    fn to_bytes(self) -> Self::Out;
    fn from_bytes(bytes: Self::Out) -> Self;
    fn write(self, bytes: &mut Vec<u8>) where Self : Sized {
        let o = self.to_bytes();
        bytes.extend_from_slice(std::borrow::Borrow::borrow(&o));
    }
    fn read_bytes<'a>(bytes: &'a [u8], offset: &mut usize) -> Self where Self::Out : std::convert::TryFrom<&'a [u8]>, Self : Sized {
        let start = *offset;
        let end = *offset + std::mem::size_of::<Self::Out>();
        *offset = end;
        let bytes = &bytes[start..end];
        
        let bytes = std::convert::TryFrom::try_from(bytes).unwrap_or_else(|_| todo!("Error handling"));
        Self::from_bytes(bytes)
    }
}

impl EzBytes for u32 {
    type Out = [u8; 4];
    fn to_bytes(self) -> Self::Out {
        self.to_le_bytes()
    }
    fn from_bytes(bytes: Self::Out) -> Self {
        u32::from_le_bytes(bytes)
    }
}

impl EzBytes for u64 {
    type Out = [u8; 8];
    fn to_bytes(self) -> Self::Out {
        self.to_le_bytes()
    }
    fn from_bytes(bytes: Self::Out) -> Self {
        u64::from_le_bytes(bytes)
    }
}

impl EzBytes for usize {
    type Out = [u8; 8];
    fn to_bytes(self) -> Self::Out {
        (self as u64).to_bytes()
    }
    fn from_bytes(bytes: Self::Out) -> Self {
        u64::from_bytes(bytes) as Self
    }
}

impl EzBytes for bool {
    type Out = [u8; 1];
    fn to_bytes(self) -> Self::Out {
        (self as u8).to_bytes()
    }
    fn from_bytes(bytes: Self::Out) -> Self {
        u8::from_bytes(bytes) != 0
    }
}

impl EzBytes for u8 {
    type Out = [u8; 1];
    fn from_bytes(bytes: Self::Out) -> Self {
        Self::from_le_bytes(bytes)
    }
    fn to_bytes(self) -> Self::Out {
        self.to_le_bytes()
    }
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
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>) {
        // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712}
        // TODO: Can use varint if we read the file backward and write lengths at the end.
        // That would require some sort of reverse prefix varint... suffix varint if you will.
        let start = bytes.len();
        0usize.write(bytes);

        // Write the branch
        branch.flush(bytes);

        // Write the primitive id
        // TODO: Include data for the primitive - like int ranges
        (T::id() as u32).write(bytes);
        T::write_batch(&self.values, bytes);
        
        // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712}
        let end = bytes.len() as u64;
        let end = end.to_le_bytes();
        for i in 0..end.len() {
            bytes[start+i] = end[i];
        }
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
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>) {
        let own_id = bytes.len();
        self.len.flush(branch, bytes);

        let values = BranchId { name: "", parent: own_id };
        self.values.flush(&values, bytes);
    }
}

impl<T: Primitive + Copy> Writable for T {
    type Writer=PrimitiveBuffer<T>;
}

#[derive(Debug)]
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
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>) {
        let own_id = bytes.len();
        self.opt.flush(branch, bytes);

        let value = BranchId { name: "value", parent: own_id };
        self.value.flush(&value, bytes);
    }
}

impl<T: Writable> Writable for Option<T> {
    type Writer=OptionWriter<T::Writer>;
}


impl<T: Writable> Writable for Vec<T> {
    type Writer=VecWriter<T::Writer>;
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