use crate::branch::*;

pub trait Writable : Sized {
    type Writer: Writer<Write=Self>;
}

pub trait Readable {

}
// FIXME: This is just to compile
impl<T: Writable> Readable for T { }

pub trait Writer {
    type Write: Writable;

    fn write(&mut self, value: &Self::Write);
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>);
    fn new() -> Self;
}