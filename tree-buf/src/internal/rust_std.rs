use crate::prelude::*;
#[cfg(feature = "read")]
use crate::internal::encodings::varint::decode_prefix_varint;
use std::vec::IntoIter;

// TODO: impl Writable for () {
#[cfg(feature = "write")]
#[derive(Default)]
pub struct BoxWriterArray<T> {
    inner: T,
}

#[cfg(feature = "write")]
impl<'a, T: Writable<'a>> Writable<'a> for Box<T> {
    type WriterArray = BoxWriterArray<T::WriterArray>;
    fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> RootTypeId {
        T::write_root(&value, bytes, lens)
    }
}

#[cfg(feature = "read")]
pub struct BoxReaderArray<T> {
    inner: T,
}

#[cfg(feature = "read")]
impl<T: Readable> Readable for Box<T> {
    type ReaderArray = BoxReaderArray<T::ReaderArray>;
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
        Ok(Box::new(T::read(sticks)?))
    }
}

#[cfg(feature = "write")]
impl<'a, T: WriterArray<'a>> WriterArray<'a> for BoxWriterArray<T> {
    type Write = Box<T::Write>;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.inner.buffer(&value)
    }
    fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId {
        self.inner.flush(bytes, lens)
    }
}

#[cfg(feature = "read")]
impl<T: ReaderArray> ReaderArray for BoxReaderArray<T> {
    type Read = Box<T::Read>;
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
        Ok(BoxReaderArray { inner: T::new(sticks)? })
    }
    fn read_next(&mut self) -> ReadResult<Self::Read> {
        Ok(Box::new(self.inner.read_next()?))
    }
}



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

