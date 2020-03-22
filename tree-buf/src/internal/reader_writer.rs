use crate::internal::encodings::varint::{size_for_varint, write_varint_into};
use crate::prelude::*;

#[cfg(feature = "write")]
pub trait WriterStream {
    type Options: EncodeOptions;
    fn write_with_id<T: TypeId>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T;
    fn write_with_len<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T;
    fn bytes(&mut self) -> &mut Vec<u8>;
    fn options(&self) -> &Self::Options;
    // TODO: Not yet used
    fn restore_if_void<T: TypeId>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let restore = self.bytes().len();
        let id = f(self);
        if id == T::void() {
            self.bytes().drain(restore..);
        }
        id
    }
    // TODO: Not yet used
    fn reserve_and_write_with_varint(&mut self, max: u64, f: impl FnOnce(&mut Self) -> u64) {
        let reserved = size_for_varint(max);
        let start = self.bytes().len();
        for _ in 0..reserved {
            self.bytes().push(0);
        }
        let end = self.bytes().len();
        let v = f(self);
        debug_assert!(v <= max);
        write_varint_into(v, &mut self.bytes()[start..end]);
    }
}

pub struct VecWriterStream<'a, O> {
    bytes: &'a mut Vec<u8>,
    lens: &'a mut Vec<usize>,
    options: &'a O,
}

impl<'a, O: EncodeOptions> VecWriterStream<'a, O> {
    pub fn new(bytes: &'a mut Vec<u8>, lens: &'a mut Vec<usize>, options: &'a O) -> Self {
        Self { bytes, lens, options }
    }
}

impl<'a, O: EncodeOptions> WriterStream for VecWriterStream<'a, O> {
    type Options = O;
    fn write_with_id<T: TypeId>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let type_index = self.bytes.len();
        self.bytes.push(0);
        let id = f(self);
        debug_assert!(id != T::void() || (self.bytes.len() == type_index + 1), "Expecting Void to write no bytes to stream");
        self.bytes[type_index] = id.into();
        id
    }
    fn write_with_len<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let start = self.bytes.len();
        let result = f(self);
        self.lens.push(self.bytes.len() - start);
        result
    }
    // This is sort of a temporary patch while figuring out what the API of the WriterStream should be
    #[inline]
    fn bytes(&mut self) -> &mut Vec<u8> {
        self.bytes
    }
    #[inline]
    fn options(&self) -> &Self::Options {
        self.options
    }
}

#[cfg(feature = "write")]
pub trait Writable<'a>: Sized {
    // TODO: What happens if we get rid of the Write = Self trait bound?
    type WriterArray: WriterArray<'a, Write = Self>;
    // At the root level, there is no need to buffer/flush, just write. By not buffering, we may
    // significantly decrease total memory usage when there are multiple arrays at the root level,
    // by not requiring that both be fully buffered simultaneously.
    #[must_use]
    fn write_root<'b: 'a>(&'b self, stream: &mut impl WriterStream) -> RootTypeId;
}

#[cfg(feature = "read")]
pub trait Readable: Sized {
    type ReaderArray: ReaderArray<Read = Self>;
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self>;
}

// TODO: Introduce a separate "Scratch" type to make eg: WriterArray re-usable.
// The scratch type would be passed to write, so it needs to be for Writable (root)
// Since not all root types have array children, some of these structs will be empty.
// To some degree it is possible to know about re-use for fields of the same type, reducing
// allocations further.

#[cfg(feature = "write")]
pub trait WriterArray<'a>: Default {
    type Write: Writable<'a>;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write);
    fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId;
}

#[cfg(feature = "read")]
pub trait ReaderArray: Sized {
    type Read;
    // TODO: It would be nice to be able to keep reference to the original byte array, especially for reading strings.
    // I think that may require GAT though the way things are setup so come back to this later.
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self>;
    fn read_next(&mut self) -> Self::Read;
}
