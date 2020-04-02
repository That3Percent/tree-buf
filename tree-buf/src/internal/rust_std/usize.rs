#[cfg(feature = "read")]
use crate::internal::encodings::varint::{decode_prefix_varint, encode_prefix_varint};
use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "read")]
impl ReaderArray for IntoIter<usize> {
    type Read = usize;
    fn new(_sticks: DynArrayBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
        todo!("usize ReaderArray new");
    }
    fn read_next(&mut self) -> Self::Read {
        self.next().unwrap_or_default()
    }
}

// TODO: Come back to usize
// TODO: Error check that the result fits in the platform size
#[cfg(feature = "read")]
pub fn read_usize(bytes: &[u8], offset: &mut usize) -> ReadResult<usize> {
    Ok(decode_prefix_varint(bytes, offset)? as usize)
}

#[cfg(feature = "write")]
pub fn write_usize(value: usize, stream: &mut impl WriterStream) {
    encode_prefix_varint(value as u64, stream.bytes());
}

/*
impl Readable for usize {
    type ReaderArray = IntoIter<usize>;
    fn read(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        Ok(u64::read(sticks, options)? as Self)
    }
}
*/
