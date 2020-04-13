use crate::prelude::*;
use std::ops::Deref;

// TODO: impl Writable for () {
#[cfg(feature = "write")]
#[derive(Default)]
pub struct BoxWriterArray<T> {
    inner: T,
}

#[cfg(feature = "write")]
impl<'a, T: Writable<'a>> Writable<'a> for Box<T> {
    type WriterArray = BoxWriterArray<T::WriterArray>;
    fn write_root<'b: 'a>(&'b self, stream: &mut impl WriterStream) -> RootTypeId {
        profile!("write_root");
        self.deref().write_root(stream)
    }
}

#[cfg(feature = "read")]
pub struct BoxReaderArray<T> {
    inner: T,
}

#[cfg(feature = "read")]
impl<T: Readable> Readable for Box<T> {
    type ReaderArray = BoxReaderArray<T::ReaderArray>;
    fn read(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("Readable::read");
        Ok(Box::new(T::read(sticks, options)?))
    }
}

#[cfg(feature = "write")]
impl<'a, T: WriterArray<'a>> WriterArray<'a> for BoxWriterArray<T> {
    type Write = Box<T::Write>;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.inner.buffer(&value)
    }
    fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId {
        self.inner.flush(stream)
    }
}

#[cfg(feature = "read")]
impl<T: ReaderArray> ReaderArray for BoxReaderArray<T> {
    type Read = Box<T::Read>;
    fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("ReaderArray::new");
        Ok(BoxReaderArray { inner: T::new(sticks, options)? })
    }
    fn read_next(&mut self) -> Self::Read {
        Box::new(self.inner.read_next())
    }
}
