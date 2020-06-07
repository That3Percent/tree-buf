use crate::internal::encodings::packed_bool::*;
use crate::internal::encodings::rle_bool::*;
use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "encode")]
impl Encodable for bool {
    type EncoderArray = Vec<bool>;
    #[inline]
    fn encode_root<O: EncodeOptions>(&self, _stream: &mut EncoderStream<'_, O>) -> RootTypeId {
        if *self {
            RootTypeId::True
        } else {
            RootTypeId::False
        }
    }
}

#[cfg(feature = "decode")]
impl Decodable for bool {
    type DecoderArray = IntoIter<bool>;
    fn decode(sticks: DynRootBranch<'_>, _options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("Decodable::decode");
        match sticks {
            DynRootBranch::Boolean(v) => Ok(v),
            _ => Err(DecodeError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "encode")]
impl EncoderArray<bool> for Vec<bool> {
    fn buffer_one<'a, 'b: 'a>(&'a mut self, value: &'b bool) {
        self.push(*value);
    }
    fn buffer_many<'a, 'b: 'a>(&'a mut self, values: &'b [bool]) {
        self.extend_from_slice(values);
    }
    fn encode_all<O: EncodeOptions>(values: &[bool], stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        profile!("encode_all");

        let compressors = (PackedBoolCompressor, RLEBoolCompressor);

        compress(values, stream, &compressors)
    }
    fn flush<O: EncodeOptions>(self, stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        Self::encode_all(&self[..], stream)
    }
}

struct PackedBoolCompressor;
impl Compressor<bool> for PackedBoolCompressor {
    fn fast_size_for(&self, data: &[bool]) -> Option<usize> {
        Some((data.len() + 7) / 8)
    }
    fn compress<O: EncodeOptions>(&self, data: &[bool], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        stream.encode_with_len(|stream| encode_packed_bool(data, stream.bytes));
        Ok(ArrayTypeId::PackedBool)
    }
}

struct RLEBoolCompressor;
impl Compressor<bool> for RLEBoolCompressor {
    fn compress<O: EncodeOptions>(&self, data: &[bool], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        if get_in_rle() {
            return Err(());
        }
        set_in_rle(true);
        let result = encode_rle_bool(data, stream);
        set_in_rle(false);
        result
    }
}

#[cfg(feature = "decode")]
impl InfallibleDecoderArray for IntoIter<bool> {
    type Decode = bool;

    fn new_infallible(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("DecoderArray::new");

        match sticks {
            DynArrayBranch::Boolean(encoding) => {
                let v = match encoding {
                    ArrayBool::Packed(bytes) => decode_packed_bool(&bytes).into_iter(),
                    ArrayBool::RLE(first, runs) => {
                        let runs = <u64 as Decodable>::DecoderArray::new(*runs, options)?;
                        decode_rle_bool(runs, first)
                    }
                };
                Ok(v)
            }
            _ => Err(DecodeError::SchemaMismatch),
        }
    }
    fn decode_next_infallible(&mut self) -> Self::Decode {
        self.next().unwrap_or_default()
    }
}
