use crate::internal::encodings::packed_bool::*;
use crate::internal::encodings::rle_bool::*;
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

        let compressors: Vec<Box<dyn Compressor<bool>>> = vec![Box::new(PackedBoolCompressor), Box::new(RLEBoolCompressor)];

        stream.write_with_len(|stream| compress(&self, stream.bytes, stream.lens, &compressors))
    }
}

struct PackedBoolCompressor;
impl Compressor<bool> for PackedBoolCompressor {
    fn fast_size_for(&self, data: &[bool]) -> Option<usize> {
        Some((data.len() + 7) / 8)
    }
    fn compress(&self, data: &[bool], bytes: &mut Vec<u8>, _lens: &mut Vec<usize>) -> Result<ArrayTypeId, ()> {
        encode_packed_bool(data, bytes);
        Ok(ArrayTypeId::PackedBool)
    }
}

struct RLEBoolCompressor;
impl Compressor<bool> for RLEBoolCompressor {
    fn compress(&self, data: &[bool], bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> Result<ArrayTypeId, ()> {
        encode_rle_bool(data, bytes, lens)
    }
}

#[cfg(feature = "read")]
impl InfallibleReaderArray for IntoIter<bool> {
    type Read = bool;

    fn new_infallible(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("ReaderArray::new");

        match sticks {
            DynArrayBranch::Boolean(encoding) => {
                let v = match encoding {
                    ArrayBool::Packed(bytes) => decode_packed_bool(&bytes).into_iter(),
                    ArrayBool::RLE(first, runs) => {
                        let runs = <u64 as Readable>::ReaderArray::new(*runs, options)?;
                        decode_rle_bool(runs, first)
                    }
                };
                Ok(v)
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next_infallible(&mut self) -> Self::Read {
        self.next().unwrap_or_default()
    }
}
