use crate::branch::*;

pub trait Writable: Sized {
    type Writer: Writer<Write = Self>;
}

pub trait Readable: Sized {
    type Reader: Reader<Read = Self>;
}

pub trait Writer {
    type Write: Writable;

    fn write(&mut self, value: &Self::Write);
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>);
    fn new() -> Self;
}

pub trait Reader {
    type Read: Readable;
    // TODO: It would be nice to be able to keep reference to the original byte array, especially for reading strings.
    // I think that may require GAT though the way things are setup so come back to this later.
    fn new(sticks: &Vec<Stick>, branch: &BranchId<'_>) -> Self;
    fn read(&mut self) -> Self::Read;
}
