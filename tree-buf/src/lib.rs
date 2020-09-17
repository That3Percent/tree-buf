#[doc(hidden)]
pub mod internal;

pub mod experimental;

pub mod prelude {
    // Likely the minimum API that should go here. It's easier to add later than to remove.

    #[cfg(feature = "macros")]
    pub use tree_buf_macros::{Decode, Encode};

    #[cfg(feature = "decode")]
    pub use crate::decode;

    #[cfg(feature = "encode")]
    pub use crate::encode;

    // This section makes everything interesting available to the rest of the crate
    // without bothering to manage imports.
    pub(crate) use crate::internal::encodings::varint::size_for_varint;
    pub(crate) use crate::{internal::error::*, internal::*};

    #[cfg(feature = "decode")]
    pub(crate) type DecodeResult<T> = Result<T, DecodeError>;

    pub(crate) use firestorm::{profile_fn, profile_method, profile_section};
}

#[cfg(feature = "decode")]
pub use internal::error::DecodeError;

#[cfg(feature = "decode")]
pub use internal::Decodable;

#[cfg(feature = "encode")]
pub use internal::Encodable;

pub use crate::prelude::*;

pub use internal::Ignore;

// TODO: Take Borrow or AsRef
pub fn encode<T: Encodable>(value: &T) -> Vec<u8> {
    let options = EncodeOptionsDefault;
    crate::experimental::options::encode_with_options(value, &options)
}

#[cfg(feature = "decode")]
pub fn decode<T: Decodable>(bytes: &[u8]) -> DecodeResult<T> {
    let options = DecodeOptionsDefault;
    crate::experimental::options::decode_with_options(bytes, &options)
}

// TODO: Figure out recursion, at least enough to handle this: https://docs.rs/serde_json/1.0.44/serde_json/value/enum.Value.html
// TODO: Nullable should be able to handle recursion as well, even if Option doesn't. (Option<Box<T>> could though)

// See also: c94adae3-9778-4a42-a454-650a97a87483
// TODO: (Performance) When recursion is not involved, there is a maximum to the amount of schema info needed to encode
//       In order to do a 1 pass encode on the data yet keep all the schema at the beginning of the file one could reserve
//       the maximum amount of buffer necessary for the schema, then encode the data to the primary buffer, encode the schema
//       to the beginning of the primary buffer and move it to be flush with the data. Technically, the schema will be 2-pass
//       but this may be much less than the data.
//
//       If we add a special sort of Recursion(depth) RootTypeId and ArrayTypeId then the schema may have a max size even
//       with recursion. This may have the added benefit of requiring less redundancy in the schema when recursion is involved.
//       One tricky part is that each array per recursion depth requires its own length. This could be dealt with by having the
//       Recursion Branch be it's own data set with it's own schema information? Not ideal.
//
//       The crux of the matter comes down to whether we want to have each depth and path for recursion be it's own branch.
//       If the branch is shared, then this may come at a loss for clustering and schemas. Data is needed to make a decision.
//       What is recursion typically used for? There's the generic JavaScript "Value" type case. Trees. What else? How do these cluster?
//       In the generic JavaScript value case, much of the clustering info seems lost anyway.

// TODO: Evaluate TurboPFor https://github.com/powturbo/TurboPFor
// or consider the best parts of it. The core differentiator here
// is the ability to use this.

// TODO: Automatic type extraction for json:
// http://stevehanov.ca/blog/?id=104

// TODO: Add decimal type
// This seems a reasonable starting point: https://github.com/paupino/rust-decimal
//
// TODO: Look at Apache Arrow Flight, Parquet, Avro
// TODO: Look at bincode
// TODO: Look at HDF5
// TODO: Look at zebra +
// TODO: Read this https://en.wikipedia.org/wiki/Column-oriented_DBMS#Column-oriented_systems
