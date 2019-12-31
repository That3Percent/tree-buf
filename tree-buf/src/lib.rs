pub mod internal;

pub mod prelude {
    // Likely the minimum API that should go here. It's easier to add later than to remove.
    pub use {
        crate:: {
            read, write
        },
        tree_buf_macros:: {
            Read, Write
        }
    };

    // This section makes everything interesting available to the rest of the crate
    // without bothering to manage imports.
    pub(crate) use crate::{internal::*, error::*,
        primitive::{EzBytes, Primitive, PrimitiveId}
    };
}

pub use prelude::*;
pub use internal::error::Error;
// TODO: Create another Readable/Writable trait that would be public, without the associated type. Then impl Readable for the internal type.
// That would turn Readable into a tag that one could use as a constraint, without exposing any internal details
pub use internal::{Readable, Writable};

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
