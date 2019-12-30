use crate::context::*;
use crate::branch::*;
use crate::missing::*;
use crate::error::*;

pub trait Writable : Sized {
    type Writer: Writer<Write=Self>;
}

pub trait Reader {
    fn read(context: &mut Context, branch: &Branch, missing: &impl Missing) -> Result<Self, Error> where Self : Sized;
}

pub trait Writer {
    type Write: Writable;

    fn write(&mut self, value: &Self::Write);
    fn flush(&self, branch: &BranchId<'_>, bytes: &mut Vec<u8>);
    fn new() -> Self;
}