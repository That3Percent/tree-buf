use crate::prelude::*;
use std::ops::Deref;

// TODO: impl Encodable for () {
#[cfg(feature = "encode")]
#[derive(Default)]
pub struct BoxEncoderArray<T> {
    inner: T,
}

#[cfg(feature = "encode")]
impl<T: Encodable> Encodable for Box<T> {
    type EncoderArray = BoxEncoderArray<T::EncoderArray>;
    fn encode_root<O: EncodeOptions>(&self, stream: &mut EncoderStream<'_, O>) -> RootTypeId {
        self.deref().encode_root(stream)
    }
}

#[cfg(feature = "decode")]
pub struct BoxDecoderArray<T> {
    inner: T,
}

#[cfg(feature = "decode")]
impl<T: Decodable> Decodable for Box<T> {
    type DecoderArray = BoxDecoderArray<T::DecoderArray>;
    fn decode(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        Ok(Box::new(T::decode(sticks, options)?))
    }
}

#[cfg(feature = "encode")]
impl<T: Encodable> EncoderArray<Box<T>> for BoxEncoderArray<T::EncoderArray> {
    fn buffer_one<'a, 'b: 'a>(&'a mut self, value: &'b Box<T>) {
        self.inner.buffer_one(value)
    }
    fn flush<O: EncodeOptions>(self, stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        self.inner.flush(stream)
    }
}

#[cfg(feature = "decode")]
impl<T: DecoderArray> DecoderArray for BoxDecoderArray<T> {
    type Decode = Box<T::Decode>;
    type Error = T::Error;
    fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        Ok(BoxDecoderArray { inner: T::new(sticks, options)? })
    }
    fn decode_next(&mut self) -> Result<Self::Decode, Self::Error> {
        Ok(Box::new(self.inner.decode_next()?))
    }
}
