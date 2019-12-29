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

trait IdGen {
    fn next(&mut self) -> usize;
}

pub trait Writer {
    type Write: Writable;

    fn write(&mut self, value: &Self::Write);
    fn new() -> Self; // Don't need the branch or id_gen until flush
}