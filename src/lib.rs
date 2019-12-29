pub mod branch;
pub mod primitive;
pub mod missing;
pub mod error;
pub mod reader_writer;
pub mod context;
mod play;

use crate::reader_writer::*;
use crate::error::*;
use crate::context::*;
use crate:: branch::*;

pub fn write<T: Writer>(value: &T) -> Vec<u8> {
    let mut context = Context::new();
    let root = Branch::root();
    value.write(&mut context, &root);
    // TODO: Get bytes from context.
    todo!();
}

pub fn read<T: Reader>(from: &[u8]) -> Result<T, Error> {
    todo!();
}

pub fn test_play() {
    play::test();
}


// TODO: When deriving, use the assert implements check that eg: Clone does, to give good compiler errors
//       If this is not possible because it's an internal API, use static_assert


