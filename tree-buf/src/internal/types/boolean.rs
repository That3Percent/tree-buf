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
        profile_method!(decode);
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
        profile_method!(buffer_many);
        self.extend_from_slice(values);
    }
    fn encode_all<O: EncodeOptions>(values: &[bool], stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        profile_method!(encode_all);

        // See also 42d5f4b4-823f-4ab4-8448-6e1a341ff28b
        let compressors = (PackedBoolCompressor, RLEBoolCompressor);
        compress(values, stream, &compressors)
    }
    fn flush<O: EncodeOptions>(self, stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        Self::encode_all(&self[..], stream)
    }
}

impl PrimitiveEncoderArray<bool> for Vec<bool> {
    fn fast_size_for_all<O: EncodeOptions>(values: &[bool], options: &O) -> usize {
        // See also 42d5f4b4-823f-4ab4-8448-6e1a341ff28b
        let compressors = (PackedBoolCompressor, RLEBoolCompressor);
        fast_size_for(values, &compressors, options)
    }
}

struct PackedBoolCompressor;
impl Compressor<bool> for PackedBoolCompressor {
    fn fast_size_for<O: EncodeOptions>(&self, data: &[bool], _options: &O) -> Result<usize, ()> {
        let buffer_len = (data.len() + 7) / 8;
        let len_len = size_for_varint(buffer_len as u64);
        Ok(buffer_len + len_len)
    }
    fn compress<O: EncodeOptions>(&self, data: &[bool], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        profile_method!(compress);
        stream.encode_with_len(|stream| encode_packed_bool(data, stream.bytes));
        Ok(ArrayTypeId::PackedBool)
    }
}

struct RLEBoolCompressor;

impl Compressor<bool> for RLEBoolCompressor {
    // TODO: fast_size_for
    fn compress<O: EncodeOptions>(&self, data: &[bool], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        within_rle(|| encode_rle_bool(data, stream))
    }
    fn fast_size_for<O: EncodeOptions>(&self, data: &[bool], options: &O) -> Result<usize, ()> {
        within_rle(|| size_of_rle_bool(data, options))
    }
}

#[cfg(feature = "decode")]
impl InfallibleDecoderArray for IntoIter<bool> {
    type Decode = bool;

    fn new_infallible(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile_method!(new_infallible);

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
