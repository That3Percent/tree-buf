use crate::prelude::*;

pub use crate::internal::options::{
    DisableParallel,
    EnableParallel,
    LosslessFloat,
    LossyFloatTolerance,
    EncodeOptions, DecodeOptions
};

pub use crate::{encode_options, decode_options};


#[cfg(feature = "encode")]
pub fn encode_with_options<T: Encodable>(value: &T, options: &impl EncodeOptions) -> Vec<u8> {
    profile_fn!(encode_with_options);
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