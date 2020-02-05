use crate::prelude::*;

pub trait Writable<'a>: Sized {
    type WriterArray: WriterArray<'a, Write = Self>;
    // At the root level, there is no need to buffer/flush, just write. By not buffering, we may
    // significantly decrease total memory usage when there are multiple arrays at the root level,
    // by not requiring that both be fully buffered simultaneously.
    #[must_use]
    fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> RootTypeId;
}

pub trait Readable: Sized {
    type ReaderArray: ReaderArray<Read = Self>;
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self>;
}

pub trait WriterArray<'a>: Default {
    type Write: Writable<'a>;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write);
    fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId;
}

pub trait ReaderArray: Sized {
    type Read;
    // TODO: It would be nice to be able to keep reference to the original byte array, especially for reading strings.
    // I think that may require GAT though the way things are setup so come back to this later.
    // TODO: This needs to be split up into 2 steps to support schema matching before deserialization.
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self>;
    fn read_next(&mut self) -> ReadResult<Self::Read>;
}
