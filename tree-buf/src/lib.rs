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

    #[derive(Default, Debug)]
    pub(crate) struct Unowned<T: ?Sized> {
        _marker: std::marker::PhantomData<*const T>,
    }
    impl<T> Unowned<T> {
        pub fn new() -> Self {
            Self {
                _marker: std::marker::PhantomData,
            }
        }
    }
    unsafe impl<T> Send for Unowned<T> {}
}

#[cfg(feature = "read")]
pub use internal::error::ReadError;

#[cfg(feature = "read")]
pub use internal::Readable;

#[cfg(feature = "write")]
pub use internal::Writable;

#[cfg(feature = "write")]
pub use internal::options;

pub use crate::prelude::*;

pub use internal::Ignore;

pub fn write<'a, 'b: 'a, T: Writable<'a>>(value: &'b T) -> Vec<u8> {
    let options = EncodeOptionsDefault;
    write_with_options(value, &options)
}

#[cfg(feature = "write")]
pub fn write_with_options<'a, 'b: 'a, T: Writable<'a>>(value: &'b T, options: &impl EncodeOptions) -> Vec<u8> {
    use internal::encodings::varint::encode_suffix_varint;

    let mut lens = Vec::new();
    let mut bytes = Vec::new();
    let mut stream = VecWriterStream::new(&mut bytes, &mut lens, options);
    stream.write_with_id(|stream| T::write_root(value, stream));

    for len in lens.iter().rev() {
        encode_suffix_varint(*len as u64, &mut bytes);
    }

    bytes
}

#[cfg(feature = "read")]
pub fn read<T: Readable>(bytes: &[u8]) -> ReadResult<T> {
    let options = DecodeOptionsDefault;
    read_with_options(bytes, &options)
}

#[cfg(feature = "read")]
pub fn read_with_options<T: Readable>(bytes: &[u8], options: &impl DecodeOptions) -> ReadResult<T> {
    let sticks = read_root(bytes)?;
    T::read(sticks, options)
}

// TODO: Figure out recursion, at least enough to handle this: https://docs.rs/serde_json/1.0.44/serde_json/value/enum.Value.html
// TODO: Nullable should be able to handle recursion as well, even if Option doesn't. (Option<Box<T>> could though)

// See also: c94adae3-9778-4a42-a454-650a97a87483
// TODO: (Performance) When recursion is not involved, there is a maximum to the amount of schema info needed to write
//       In order to do a 1 pass write on the data yet keep all the schema at the beginning of the file one could reserve
//       the maximum amount of buffer necessary for the schema, then write the data to the primary buffer, write the schema
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
