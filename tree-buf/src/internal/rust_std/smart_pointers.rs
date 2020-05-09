use crate::prelude::*;
use std::ops::Deref;

// TODO: impl Writable for () {
#[cfg(feature = "write")]
#[derive(Default)]
pub struct BoxWriterArray<T> {
    inner: T,
}

#[cfg(feature = "write")]
impl<T: Writable> Writable for Box<T> {
    type WriterArray = BoxWriterArray<T::WriterArray>;
    fn write_root<O: EncodeOptions>(&self, stream: &mut WriterStream<'_, O>) -> RootTypeId {
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
impl<T: Writable> WriterArray<Box<T>> for BoxWriterArray<T::WriterArray> {
    fn buffer<'a, 'b: 'a>(&'a mut self, value: &'b Box<T>) {
        self.inner.buffer(&value)
    }
    fn flush<O: EncodeOptions>(self, stream: &mut WriterStream<'_, O>) -> ArrayTypeId {
        self.inner.flush(stream)
    }
}

#[cfg(feature = "read")]
impl<T: ReaderArray> ReaderArray for BoxReaderArray<T> {
    type Read = Box<T::Read>;
    type Error = T::Error;
    fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("ReaderArray::new");
        Ok(BoxReaderArray { inner: T::new(sticks, options)? })
    }
    fn read_next(&mut self) -> Result<Self::Read, Self::Error> {
        Ok(Box::new(self.inner.read_next()?))
    }
}
