use crate::internal::encodings::packed_bool::*;
use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "write")]
impl Writable for bool {
    type WriterArray = Vec<bool>;
    #[inline]
    fn write_root<O: EncodeOptions>(&self, _stream: &mut WriterStream<'_, O>) -> RootTypeId {
        if *self {
            RootTypeId::True
        } else {
            RootTypeId::False
        }
    }
}

#[cfg(feature = "read")]
impl Readable for bool {
    type ReaderArray = IntoIter<bool>;
    fn read(sticks: DynRootBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("Readable::read");
        match sticks {
            DynRootBranch::Boolean(v) => Ok(v),
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "write")]
impl WriterArray<bool> for Vec<bool> {
    fn buffer<'a, 'b: 'a>(&'a mut self, value: &'b bool) {
        self.push(*value);
    }
    fn flush<O: EncodeOptions>(self, stream: &mut WriterStream<'_, O>) -> ArrayTypeId {
        profile!("flush");
        stream.write_with_len(|stream| encode_packed_bool(&self, stream.bytes));
        ArrayTypeId::Boolean
    }
}

#[cfg(feature = "read")]
impl InfallibleReaderArray for IntoIter<bool> {
    type Read = bool;

    fn new_infallible(sticks: DynArrayBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("ReaderArray::new");

        match sticks {
            DynArrayBranch::Boolean(bytes) => {
                let v = decode_packed_bool(&bytes);
                Ok(v.into_iter())
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next_infallible(&mut self) -> Self::Read {
        self.next().unwrap_or_default()
    }
}
