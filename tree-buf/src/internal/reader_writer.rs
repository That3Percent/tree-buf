use crate::prelude::*;

#[cfg(feature = "write")]
pub trait Writable<'a>: Sized {
    type WriterArray: WriterArray<'a, Write = Self>;
    // At the root level, there is no need to buffer/flush, just write. By not buffering, we may
    // significantly decrease total memory usage when there are multiple arrays at the root level,
    // by not requiring that both be fully buffered simultaneously.
    #[must_use]
    fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> RootTypeId;
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
    fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId;
}

#[cfg(feature = "read")]
pub trait ReaderArray: Sized {
    type Read;
    // TODO: It would be nice to be able to keep reference to the original byte array, especially for reading strings.
    // I think that may require GAT though the way things are setup so come back to this later.
    // TODO: This needs to be split up into 2 steps to support schema matching before deserialization.
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self>;
    fn read_next(&mut self) -> ReadResult<Self::Read>;
}
