use crate::internal::encodings::varint::*;
use crate::prelude::*;
use rle::RLE;
use std::borrow::Borrow;
use std::vec::IntoIter;

// TODO: Consider compressed unicode (SCSU?) for String in general,
// but in particular for schema strings. As schema strings need only
// be compared and usually not displayed we can do bit-for-bit comparisons
// (Make sure that's true for SCSU, which may allow multiple encodings!)

#[cfg(feature = "encode")]
pub fn encode_str<O: EncodeOptions>(value: &str, stream: &mut EncoderStream<'_, O>) {
    encode_usize(value.len(), stream);
    stream.bytes.extend_from_slice(value.as_bytes());
}

#[cfg(feature = "decode")]
fn decode_str_len<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize) -> DecodeResult<&'a str> {
    let utf8 = decode_bytes(len, bytes, offset)?;
    Ok(std::str::from_utf8(utf8)?)
}

#[cfg(feature = "decode")]
pub fn decode_str<'a>(bytes: &'a [u8], offset: &'_ mut usize) -> DecodeResult<&'a str> {
    let len = decode_prefix_varint(bytes, offset)? as usize;
    decode_str_len(len, bytes, offset)
}

#[cfg(feature = "encode")]
impl Encodable for String {
    type EncoderArray = Vec<&'static String>;
    fn encode_root<O: EncodeOptions>(&self, stream: &mut EncoderStream<'_, O>) -> RootTypeId {
        let value = self.as_str();
        match value.len() {
            0 => RootTypeId::Str0,
            1 => {
                stream.bytes.push(value.as_bytes()[0]);
                RootTypeId::Str1
            }
            2 => {
                stream.bytes.extend_from_slice(value.as_bytes());
                RootTypeId::Str2
            }
            3 => {
                stream.bytes.extend_from_slice(value.as_bytes());
                RootTypeId::Str3
            }
            _ => {
                let b = value.as_bytes();
                encode_prefix_varint(b.len() as u64, stream.bytes);
                stream.bytes.extend_from_slice(b);
                RootTypeId::Str
            }
        }
    }
}

#[cfg(feature = "encode")]
impl EncoderArray<String> for Vec<&'static String> {
    fn buffer_one<'a, 'b: 'a>(&'a mut self, value: &'b String) {
        // TODO: Working around lifetime issues for lack of GAT
        // A quick check makes this appear to be sound, since the signature
        // requires that the value outlive self.
        //
        // The big safety problem is that whe then give these references
        // away when flushing. We happen to know that nothing saves the references,
        // but when things like threading come into play it's hard to know.
        //
        // TODO: Use extend_lifetime crate
        self.push(unsafe { std::mem::transmute(value) });
    }

    fn flush<O: EncodeOptions>(self, stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        profile!("flush");

        let compressors = (Utf8Compressor, RLE::new((Utf8Compressor,)), Dictionary::new((Utf8Compressor,)));

        compress(&self[..], stream, &compressors)
    }
}

#[cfg(feature = "decode")]
impl Decodable for String {
    // TODO: Use lifetimes to make this decode lazy rather than IntoIter
    type DecoderArray = IntoIter<String>;
    fn decode(sticks: DynRootBranch<'_>, _options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("String Decodable::decode");
        match sticks {
            DynRootBranch::String(s) => Ok(s.to_owned()),
            _ => Err(DecodeError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "decode")]
impl InfallibleDecoderArray for IntoIter<String> {
    type Decode = String;

    fn new_infallible(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("String new_infallible");

        match sticks {
            DynArrayBranch::String(bytes) => {
                let _g = firestorm::start_guard("String");

                let strs = decode_all(&bytes, |b, o| decode_str(b, o).and_then(|v| Ok(v.to_owned())))?;
                Ok(strs.into_iter())
            }
            DynArrayBranch::RLE { runs, values } => {
                let rle = RleIterator::new(runs, values, options, |values| Self::new_infallible(values, options))?;
                let all = rle.collect::<Vec<_>>();
                Ok(all.into_iter())
            }
            DynArrayBranch::Dictionary { indices, values } => {
                let dict = DictionaryIterator::new(indices, values, options, |values| Self::new_infallible(values, options))?;
                let all = dict.collect::<Vec<_>>();
                Ok(all.into_iter())
            }
            _ => Err(DecodeError::SchemaMismatch),
        }
    }
    fn decode_next_infallible(&mut self) -> Self::Decode {
        self.next().unwrap_or_default()
    }
}

#[cfg(feature = "encode")]
pub(crate) struct Utf8Compressor;

// TODO: The Borrow<String> here is interesting. Can we get rid of other lifetimes?
#[cfg(feature = "encode")]
impl<T: Borrow<String>> Compressor<T> for Utf8Compressor {
    fn fast_size_for<O: EncodeOptions>(&self, data: &[T], _options: &O) -> Result<usize, ()> {
        profile!("Utf8 fast_size_for");
        let mut total = 0;
        for s in data {
            total += size_for_varint(s.borrow().len() as u64);
            total += s.borrow().as_bytes().len();
        }
        Ok(total + size_for_varint(total as u64))
    }
    fn compress<O: EncodeOptions>(&self, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        profile!("Utf8 compress");

        stream.encode_with_len(|stream| {
            for value in data.iter() {
                encode_prefix_varint(value.borrow().len() as u64, stream.bytes);
                stream.bytes.extend_from_slice(value.borrow().as_bytes());
            }
        });

        Ok(ArrayTypeId::Utf8)
    }
}
