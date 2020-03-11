use crate::internal::encodings::packed_bool::*;
use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "write")]
impl<'a> Writable<'a> for bool {
    type WriterArray = Vec<bool>;
    #[inline]
    fn write_root<'b: 'a>(&'b self, _stream: &mut impl WriterStream) -> RootTypeId {
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
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynRootBranch::Boolean(v) => Ok(v),
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "write")]
impl<'a> WriterArray<'a> for Vec<bool> {
    type Write = bool;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.push(*value);
    }
    fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId {
        stream.write_with_len(|stream| encode_packed_bool(&self, stream.bytes()));
        ArrayTypeId::Boolean
    }
}

#[cfg(feature = "read")]
impl ReaderArray for IntoIter<bool> {
    type Read = bool;
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynArrayBranch::Boolean(bytes) => {
                let v = decode_packed_bool(bytes);
                Ok(v.into_iter())
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next(&mut self) -> ReadResult<Self::Read> {
        self.next().ok_or(ReadError::InvalidFormat(InvalidFormat::ShortArray))
    }
}
