//! Configure aspects about encode-decode like whether to opt into a lossy encoding,
//! or whether to use parallelism. Options can currently be specified per encode or decode operation,
//! but the eventual goal is to be able to specify options hierarchically on fields, optionally
//! recursively. This API is very likely to change.

use crate::prelude::*;

pub use crate::internal::options::{DecodeOptions, DisableParallel, EnableParallel, EncodeOptions, LosslessFloat, LossyFloatTolerance};

pub use crate::{decode_options, encode_options};

#[cfg(feature = "encode")]
pub fn encode_with_options<T: Encodable>(value: &T, options: &impl EncodeOptions) -> Vec<u8> {
    profile_fn!(T, encode_with_options);
    use crate::internal::encodings::varint::encode_suffix_varint;

    let mut lens = Vec::new();
    let mut bytes = Vec::new();
    let mut stream = EncoderStream::new(&mut bytes, &mut lens, options);
    stream.encode_with_id(|stream| T::encode_root(value, stream));

    for len in lens.iter().rev() {
        encode_suffix_varint(*len as u64, &mut bytes);
    }

    bytes
}

#[cfg(feature = "decode")]
pub fn decode_with_options<T: Decodable>(bytes: &[u8], options: &impl DecodeOptions) -> DecodeResult<T> {
    profile_fn!(T, decode_with_options);
    let sticks = decode_root(bytes)?;
    T::decode(sticks, options)
}
