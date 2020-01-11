use crate::prelude::*;

pub trait Writable: Sized {
    type Writer: Writer<Write = Self>;
}

pub trait Readable: Sized {
    type Reader: Reader<Read = Self>;
}

pub trait Writer {
    type Write: Writable;

    fn write(&mut self, value: &Self::Write);
    fn flush<ParentBranch: StaticBranch>(self, branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>);
    fn new() -> Self;
}

pub trait Reader : Sized {
    type Read: Readable;
    // TODO: It would be nice to be able to keep reference to the original byte array, especially for reading strings.
    // I think that may require GAT though the way things are setup so come back to this later.
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, branch: ParentBranch) -> ReadResult<Self>;
    fn read(&mut self) -> ReadResult<Self::Read>;
}
