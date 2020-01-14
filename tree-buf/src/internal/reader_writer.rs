use crate::prelude::*;

pub trait Writable<'a>: Sized {
    type Writer: Writer<'a, Write = Self>;
}

pub trait Readable: Sized {
    type Reader: Reader<Read = Self>;
}

pub trait Writer<'a> : Default {
    type Write: Writable<'a>;

    // TODO: There are some cases where it may be possible to write and flush sections independently.
    // This should be true for all paths up to the point where arrays become involved. In doing so, there may be
    // significant memory savings in not buffering multiple sections of the document simultaneously.
    fn write<'b : 'a>(&mut self, value: &'b Self::Write);
    fn flush<ParentBranch: StaticBranch>(self, branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>);
}

pub trait Reader : Sized {
    type Read: Readable;
    // TODO: It would be nice to be able to keep reference to the original byte array, especially for reading strings.
    // I think that may require GAT though the way things are setup so come back to this later.
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, branch: ParentBranch) -> ReadResult<Self>;
    fn read(&mut self) -> ReadResult<Self::Read>;
}
