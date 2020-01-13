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
        primitive::{Primitive, PrimitiveId}
    };

    pub(crate) type ReadResult<T> = Result<T, ReadError>;
}

pub use prelude::*;
pub use internal::error::ReadError;
// TODO: Create another Readable/Writable trait that would be public, without the associated type. Then impl Readable for the internal type.
// That would turn Readable into a tag that one could use as a constraint, without exposing any internal details
pub use internal::{Readable, Writable, NonArrayBranch};
use internal::encodings::varint::{encode_suffix_varint};

pub fn write<'a, 'b: 'a, T: Writable<'a>>(value: &'b T) -> Vec<u8> {
    let mut writer = T::Writer::new();
    writer.write(value);
    let mut lens = Vec::new();
    let mut bytes = Vec::new();
    // TODO: The pre-amble could come back as optional, as long as it has it's own PrimitiveId
    writer.flush(NonArrayBranch, &mut bytes, &mut lens);

    for len in lens.iter().rev() {
        encode_suffix_varint(*len as u64, &mut bytes);
    }

    bytes
}

pub fn read<T: Readable>(bytes: &[u8]) -> ReadResult<T> {
    let sticks = read_root(bytes)?;
    let mut reader = T::Reader::new(sticks, NonArrayBranch)?;
    reader.read()
}

// TODO: Figure out recursion, at least enough to handle this: https://docs.rs/serde_json/1.0.44/serde_json/value/enum.Value.html
// TODO: Nullable should be able to handle recursion as well, even if Option doesn't. (Option<Box<T>> could though)

// TODO: When deriving, use the assert implements check that eg: Clone does, to give good compiler errors
//       If this is not possible because it's an internal API, use static_assert


// TODO: Evaluate TurboPFor https://github.com/powturbo/TurboPFor
// or consider the best parts of it. The core differentiator here
// is the ability to use this.