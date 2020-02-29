#[cfg(feature = "read")]
use crate::internal::encodings::varint::decode_prefix_varint;
use crate::prelude::*;
use std::vec::IntoIter;


#[cfg(feature = "read")]
impl ReaderArray for IntoIter<usize> {
    type Read = usize;
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
        todo!();
    }
    fn read_next(&mut self) -> ReadResult<Self::Read> {
        self.next().ok_or_else(|| ReadError::InvalidFormat(InvalidFormat::ShortArray))
    }
}

// TODO: Come back to usize
// TODO: Error check that the result fits in the platform size
#[cfg(feature = "read")]
pub fn read_usize(bytes: &[u8], offset: &mut usize) -> ReadResult<usize> {
    Ok(decode_prefix_varint(bytes, offset)? as usize)
}

/*
impl Readable for usize {
    type ReaderArray = IntoIter<usize>;
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
        Ok(u64::read(sticks)? as Self)
    }
}
*/
