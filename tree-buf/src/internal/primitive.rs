use crate::prelude::*;
use crate::internal::encodings::varint::{encode_prefix_varint, decode_prefix_varint, encode_suffix_varint};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::mem::transmute;

pub trait Primitive: Default + BatchData {
    fn id() -> PrimitiveId;
}
// TODO: The interaction between Default and Missing here may be dubious.
// What it will ultimately infer is that the struct exists, but that all it's
// fields should also come up missing. Where this gets really sketchy though
// is that there may be no mechanism to ensure that none of it's fields actually
// do come up missing in the event of a name collision. I think what we actually
// want is to try falling back to the owning struct default implementation instead,
// but that would require Default on too much. Having the branch type be a part
// of the lookup somehow, or have missing be able to cancel the branch to something bogus may help.
//
// Ammendment to previous. This comment is somewhat out of date, now that Missing isn't really implemented,
// and that the schema match has been moved to one place.
#[derive(Copy, Clone, Default, Debug)]
pub struct Struct;
// The Default derive enables DefaultOnMissing to have an empty array
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Array(usize);
// The Default derive enabled DefaultOnMissing to have None
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Opt(bool);

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum PrimitiveId {
    Struct = 1,
    Array = 2, // TODO: Support fixed length in primitive id
    Opt = 3,
    // TODO: The idea for int is to always encode up to 64 bit values,
    // but for any data store the min value and offset first, then use
    // that to select an optimal encoding. When deserializing, the min and
    // offset can be used to find if the data type required by the schema
    // matches.
    // Consider something like this - https://lemire.me/blog/2012/09/12/fast-integer-compression-decoding-billions-of-integers-per-second/
    Int = 4,
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
            4 => Int,
            5 => Bool,
            6 => Usize,
            7 => Str,
            _ => todo!("error handling. {}", v),
        }
    }
}

pub trait IntFromU64 : Into<u64> + TryFrom<u64> + Copy + Default {}
impl IntFromU64 for u8 {}
impl IntFromU64 for u16 {}
impl IntFromU64 for u32 {}
impl IntFromU64 for u64 {}

unsafe trait Wrapper : Sized {
    type Inner;

    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) where Self::Inner : BatchData {
        unsafe { Self::Inner::write_batch(transmute(items), bytes) }
    }
    fn read_batch(bytes: &[u8]) -> Vec<Self> where Self::Inner : BatchData {
        unsafe { transmute(Self::Inner::read_batch(bytes)) }
    }
}

unsafe impl Wrapper for Array {
    type Inner = usize;
}
unsafe impl Wrapper for Opt {
    type Inner = bool;
}

impl<T: IntFromU64> Primitive for T {
    fn id() -> PrimitiveId { PrimitiveId::Int }
}
// FIXME: This is just for convenience right now, schema matching and custom encodings are needed instead.
impl<T: IntFromU64> BatchData for T {
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        read_all(bytes, |b, o| {
            let v = decode_prefix_varint(b, o);
            v.try_into().unwrap_or_else(|_| todo!()) // TODO: Error handling (which won't be needed when schema match occurs)
        })
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        for item in items {
            let v = (*item).into();
            encode_prefix_varint(v, bytes);
        }
    }
}


pub trait BatchData: Sized {
    fn read_batch(bytes: &[u8]) -> Vec<Self>;
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>);
}


impl Primitive for Struct {
    fn id() -> PrimitiveId {
        PrimitiveId::Struct
    }
}

// TODO: Performance - remove the need to allocate vec here.
impl BatchData for Struct {
    fn write_batch(_items: &[Self], _bytes: &mut Vec<u8>) { }
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        debug_assert_eq!(bytes.len(), 0);
        Vec::new()
    }
}

impl Primitive for Array {
    fn id() -> PrimitiveId {
        PrimitiveId::Array
    }
}

impl BatchData for Opt {
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        Wrapper::write_batch(items, bytes)
    }
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        Wrapper::read_batch(bytes)
    }
}

impl BatchData for Array {
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        Wrapper::write_batch(items, bytes)
    }
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        Wrapper::read_batch(bytes)
    }
}


impl Primitive for Opt {
    fn id() -> PrimitiveId {
        PrimitiveId::Opt
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

impl Primitive for bool {
    fn id() -> PrimitiveId {
        PrimitiveId::Bool
    }
}

impl BatchData for bool {
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        // TODO: This actually may get the wrong length, taking more bools then necessary.
        // This doesn't currently present a problem though.
        let capacity = bytes.len() * 8;
        let mut result = Vec::with_capacity(capacity);
        for byte in bytes {
            result.extend_from_slice(&[
                (byte & 1 << 0) != 0,
                (byte & 1 << 1) != 0,
                (byte & 1 << 2) != 0,
                (byte & 1 << 3) != 0,
                (byte & 1 << 4) != 0,
                (byte & 1 << 5) != 0,
                (byte & 1 << 6) != 0,
                (byte & 1 << 7) != 0,
            ]);
        }
        debug_assert!(result.len() == capacity);
        result
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        let mut offset = 0;
        while offset + 8 < items.len() {
            let b = 
                (items[offset + 0] as u8) << 0 |
                (items[offset + 1] as u8) << 1 |
                (items[offset + 2] as u8) << 2 |
                (items[offset + 3] as u8) << 3 |
                (items[offset + 4] as u8) << 4 |
                (items[offset + 5] as u8) << 5 |
                (items[offset + 6] as u8) << 6 |
                (items[offset + 7] as u8) << 7;
            bytes.push(b);
            offset += 8;
        }

        if offset < items.len() {
            let mut b = 0;
            for i in 0..items.len() - offset {
                b |= (items[offset + i] as u8) << i;
            }
            bytes.push(b);
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
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>) {
        let start = bytes.len();

        // Write the branch
        branch.flush(bytes);

        // Write the primitive id
        // TODO: Include data for the primitive - like int ranges
        bytes.push(T::id() as u8);

        T::write_batch(&self.values, bytes);

        // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712}
        let size = (bytes.len() - start) as u64;
        encode_suffix_varint(size, bytes);
    }
}

impl<T: Primitive + Copy> Reader for PrimitiveBuffer<T> {
    type Read = T;
    fn new(sticks: &Vec<Stick<'_>>, branch: &BranchId) -> Self {
        let stick = branch.find_stick(&sticks).unwrap(); // TODO: Error handling
        if stick.primitive != T::id() {
            todo!("error handling. {:?} {:?} {:?}", T::id(), branch, stick);
        }

        let values = T::read_batch(stick.bytes);
        Self { values, read_offset: 0 }
    }
    fn read(&mut self) -> Self::Read {
        let value = self.values[self.read_offset];
        self.read_offset += 1;
        value
    }
}

#[derive(Debug)]
pub struct VecWriter<T> {
    len: PrimitiveBuffer<Array>,
    values: T,
}

pub struct VecReader<T> {
    len: PrimitiveBuffer<Array>,
    values: T,
}

impl<T: Writer> Writer for VecWriter<T> {
    type Write = Vec<T::Write>;
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

impl<T: Reader> Reader for VecReader<T> {
    type Read = Vec<T::Read>;
    fn new(sticks: &Vec<Stick>, branch: &BranchId) -> Self {
        let own_id = branch.find_stick(sticks).unwrap().start; // TODO: Error handling
        let len = Reader::new(sticks, branch);

        let values = BranchId { name: "", parent: own_id };
        let values = Reader::new(sticks, &values);

        Self { len, values }
    }
    fn read(&mut self) -> Self::Read {
        let len = self.len.read().0;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(self.values.read());
        }
        result
    }
}

impl<T: Primitive + Copy> Writable for T {
    type Writer = PrimitiveBuffer<T>;
}

impl<T: Primitive + Copy> Readable for T {
    type Reader = PrimitiveBuffer<T>;
}

#[derive(Debug)]
pub struct OptionWriter<V> {
    opt: PrimitiveBuffer<Opt>,
    value: V,
}

pub struct OptionReader<V> {
    opt: PrimitiveBuffer<Opt>,
    value: V,
}

impl<V: Writer> Writer for OptionWriter<V> {
    type Write = Option<V::Write>;
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

        let value = BranchId { name: "", parent: own_id };
        self.value.flush(&value, bytes);
    }
}

impl<V: Reader> Reader for OptionReader<V> {
    type Read = Option<V::Read>;
    fn new(sticks: &Vec<Stick<'_>>, branch: &BranchId) -> Self {
        let own_id = branch.find_stick(sticks).unwrap().start; // TODO: Error handling
        let opt = Reader::new(sticks, branch);

        let value = BranchId { name: "", parent: own_id };
        let value = Reader::new(sticks, &value);
        Self { opt, value }
    }
    fn read(&mut self) -> Self::Read {
        if self.opt.read().0 {
            Some(self.value.read())
        } else {
            None
        }
    }
}

impl<T: Writable> Writable for Option<T> {
    type Writer = OptionWriter<T::Writer>;
}

impl<T: Readable> Readable for Option<T> {
    type Reader = OptionReader<T::Reader>;
}

impl<T: Writable> Writable for Vec<T> {
    type Writer = VecWriter<T::Writer>;
}

impl<T: Readable> Readable for Vec<T> {
    type Reader = VecReader<T::Reader>;
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