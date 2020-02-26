pub mod internal;

pub mod prelude {
    // Likely the minimum API that should go here. It's easier to add later than to remove.

    #[cfg(feature = "macros")]
    pub use tree_buf_macros::{Read, Write};

    #[cfg(feature = "read")]
    pub use crate::read;

    #[cfg(feature = "write")]
    pub use crate::write;

    // This section makes everything interesting available to the rest of the crate
    // without bothering to manage imports.
    pub(crate) use crate::{internal::error::*, internal::*};

    #[cfg(feature = "read")]
    pub(crate) type ReadResult<T> = Result<T, ReadError>;
}

#[cfg(feature = "read")]
pub use internal::error::ReadError;

// TODO: Create another Readable/Writable trait that would be public, without the associated type. Then impl Readable for the internal type.
// That would turn Readable into a tag that one could use as a constraint, without exposing any internal details

#[cfg(feature = "read")]
pub use internal::Readable;

#[cfg(feature = "write")]
pub use internal::Writable;

pub use crate::prelude::*;

#[cfg(feature = "write")]
pub fn write<'a, 'b: 'a, T: Writable<'a>>(value: &'b T) -> Vec<u8> {
    use internal::encodings::varint::encode_suffix_varint;

    let mut lens = Vec::new();
    let mut bytes = Vec::new();
    bytes.push(0);
    // TODO: The pre-amble could come back as optional as a DynRootBranch
    let type_id = T::write_root(value, &mut bytes, &mut lens);
    bytes[0] = type_id.into();

    for len in lens.iter().rev() {
        encode_suffix_varint(*len as u64, &mut bytes);
    }

    bytes
}

#[cfg(feature = "read")]
pub fn read<T: Readable>(bytes: &[u8]) -> ReadResult<T> {
    let sticks = read_root(bytes)?;
    T::read(sticks)
}

// TODO: Figure out recursion, at least enough to handle this: https://docs.rs/serde_json/1.0.44/serde_json/value/enum.Value.html
// TODO: Nullable should be able to handle recursion as well, even if Option doesn't. (Option<Box<T>> could though)

// See also: c94adae3-9778-4a42-a454-650a97a87483
// TODO: (Performance) When recursion is not involved, there is a maximum to the amount of schema info needed to write
//       In order to do a 1 pass write on the data yet keep all the schema at the beginning of the file one could reserve
//       the maximum amount of buffer necessary for the schema, then write the data to the primary buffer, write the schema
//       to the beginning of the primary buffer and move it to be flush with the data. Technically, the schema will be 2-pass
//       but this may be much less than the data.
//       If we add a special sort of Recursion(depth) RootTypeId and ArrayTypeId then the schema may have a max size even
//       with recursion. This may have the added benefit of requiring less redundancy in the schema when recursinon is involved.

// TODO: When deriving, use the assert implements check that eg: Clone does, to give good compiler errors
//       If this is not possible because it's an internal API, use static_assert

// TODO: Evaluate TurboPFor https://github.com/powturbo/TurboPFor
// or consider the best parts of it. The core differentiator here
// is the ability to use this.
