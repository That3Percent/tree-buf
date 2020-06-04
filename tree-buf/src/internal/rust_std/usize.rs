#[cfg(feature = "decode")]
use crate::internal::encodings::varint::{decode_prefix_varint, encode_prefix_varint};
use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "decode")]
impl InfallibleDecoderArray for IntoIter<usize> {
    type Decode = usize;
    fn new_infallible(_sticks: DynArrayBranch<'_>, _options: &impl DecodeOptions) -> DecodeResult<Self> {
        todo!("usize DecoderArray new");
    }
    fn decode_next_infallible(&mut self) -> Self::Decode {
        self.next().unwrap_or_default()
    }
}

// TODO: Come back to usize
// TODO: Error check that the result fits in the platform size
#[cfg(feature = "decode")]
pub fn decode_usize(bytes: &[u8], offset: &mut usize) -> DecodeResult<usize> {
    Ok(decode_prefix_varint(bytes, offset)? as usize)
}

#[cfg(feature = "encode")]
pub fn encode_usize<O: EncodeOptions>(value: usize, stream: &mut EncoderStream<'_, O>) {
    encode_prefix_varint(value as u64, stream.bytes);
}

/*
impl Decodable for usize {
    type DecoderArray = IntoIter<usize>;
    fn decode(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        Ok(u64::decode(sticks, options)? as Self)
    }
}
*/
