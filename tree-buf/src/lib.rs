pub mod branch;
pub mod error;
pub mod missing;
pub mod prelude;
pub mod primitive;
pub mod reader_writer;

// TODO: Move export of some types into an internals module which would re-export named needed by the macro,
// and make all the mods private with just re-exported read/write, and macros

#[cfg(test)]
mod tests;

use crate::prelude::*;

// TREEBUF
const PREAMBLE: [u8; 7] = [84, 82, 69, 69, 66, 85, 70];

pub fn write<T: Writable>(value: &T) -> Vec<u8> {
    let mut writer = T::Writer::new();
    writer.write(value);
    let mut bytes = Vec::new();
    // The pre-amble is actually necessary for correctness. Without it,
    // the first branch would get written to the beginning and share
    // the same parent id as the null branch.
    bytes.extend_from_slice(&PREAMBLE);

    writer.flush(&BranchId { name: "", parent: 0 }, &mut bytes);

    bytes
}

pub fn read<T: Readable>(bytes: &[u8]) -> Result<T, Error> {
    let mut offset = 0;
    assert_eq!(&PREAMBLE, &bytes[offset..offset + PREAMBLE.len()], "Not valid file"); // TODO: Error handling
    offset += PREAMBLE.len();
    let mut sticks = Vec::new();
    while offset < bytes.len() {
        sticks.push(Stick::read(bytes, &mut offset));
    }

    let branch = BranchId { name: "", parent: 0 };
    let mut reader = T::Reader::new(&sticks, &branch);

    Ok(reader.read())
}

// TODO: When deriving, use the assert implements check that eg: Clone does, to give good compiler errors
//       If this is not possible because it's an internal API, use static_assert
