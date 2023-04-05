use crate::internal::encodings::varint::*;
use crate::prelude::*;
use brotli::enc::BrotliEncoderParams;
use brotli::{BrotliCompress, BrotliDecompress};
use std::borrow::Borrow;
use std::convert::TryInto;
use std::io::Cursor;
use std::ops::Deref;
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
        profile_method!(flush);

        let compressors = (Utf8Compressor, RLE::new((Utf8Compressor,)), Dictionary::new((Utf8Compressor,)), BrotliCompressor);
        compress(&self[..], stream, &compressors)
    }
}

#[cfg(feature = "decode")]
impl Decodable for String {
    // TODO: Use lifetimes to make this decode lazy rather than IntoIter
    type DecoderArray = IntoIter<String>;
    fn decode(sticks: DynRootBranch<'_>, _options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile_method!(decode);
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
        profile_method!(new_infallible);

        // TODO: Consider when compressing a bloom filter to decide whether to use a dictionary

        match sticks {
            DynArrayBranch::BrotliUtf8 { utf8, lens } => {
                profile_section!(brotli_utf8);

                let (long_str, lens) = parallel(
                    || {
                        let mut out = Vec::new();
                        let mut cursor = Cursor::new(utf8.deref());
                        BrotliDecompress(&mut cursor, &mut out).map_err(|_| DecodeError::InvalidFormat)?;
                        let s = String::from_utf8(out).map_err(|_| DecodeError::InvalidFormat)?;
                        Result::<_, DecodeError>::Ok(s)
                    },
                    // TODO: Why does not this not use <usize> as Decodable?
                    // See also cc81c324-ae01-4473-b8c2-e486f8032860
                    || <u64 as Decodable>::DecoderArray::new(*lens, options),
                    options,
                );
                let lens = lens?;

                // TODO: Support Null for lens to indicate there is exactly 1 item.
                let long_str = long_str?;
                let long_str = long_str.as_str();

                // See also c2c4fad7-c231-4fb2-8cf1-50ca1bce7fc6
                // The last length is implied so we don't have to write it.
                // Therefore an array with 1 item actually has 2 strings
                // TODO: NOPE! Nixed the above idea because the length of lens
                // was not implied, causing us to sometimes take the wrong strings.
                // Can go back and fix this.
                let mut all = Vec::with_capacity(lens.len());
                let mut start: usize = 0;
                for len in lens {
                    let len = len.try_into().map_err(|_| DecodeError::InvalidFormat)?;
                    let end = start.checked_add(len).ok_or(DecodeError::InvalidFormat)?;
                    let s = long_str.get(start..end).ok_or(DecodeError::InvalidFormat)?;
                    all.push(s.to_owned());
                    start = end;
                }
                //all.push(long_str[start..].to_owned());

                Ok(all.into_iter())
            }
            DynArrayBranch::String(bytes) => {
                profile_section!(str_utf8);

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
pub(crate) struct BrotliCompressor;

// TODO: The Borrow<String> here is interesting. Can we get rid of other lifetimes?
#[cfg(feature = "encode")]
impl<T: Borrow<String>> Compressor<T> for BrotliCompressor {
    fn fast_size_for<O: EncodeOptions>(&self, data: &[T], _options: &O) -> Result<usize, ()> {
        // TODO: Very unscientific. Basically what we're saying here is that if the other compressors
        // used more than 10 bytes per item and the minimum length is 100 bytes then use Brotli.
        // This estimate is totally wrong and it may be nice to have a fast_size_for for Brotli if
        // it's not crazy difficult.
        // See also 9003b01b-83e8-4acc-9f38-d584a37e20c6
        Ok(data.len().max(10) * 10)
    }
    fn compress<O: EncodeOptions>(&self, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        profile_method!(compress);

        // See also c2c4fad7-c231-4fb2-8cf1-50ca1bce7fc6
        if data.is_empty() {
            // It's not currently possible to hit this.
            // See also 9003b01b-83e8-4acc-9f38-d584a37e20c6
            todo!("Support null lens");
        }

        // TODO: (Performance) It would be good to have a different buffer type that merged
        // all of the strings and lens separately before this method, which takes &[T] but
        // could take (&str, &[usize]) instead. Right now we're buffering twice in some cases.
        let (buffer, lens) = {
            profile_section!(buffer);
            let mut buffer = String::new();
            let mut lens = Vec::new();

            for s in data {
                let s = s.borrow();
                buffer.push_str(s);
                lens.push(s.len() as u64);
            }

            // See also c2c4fad7-c231-4fb2-8cf1-50ca1bce7fc6
            //lens.pop();

            (buffer, lens)
        };

        stream.encode_with_len(|stream| {
            // TODO: check all the params, including the utf-8 hint
            let params = BrotliEncoderParams::default();
            //params.mode = BrotliEncoderMode::BROTLI_FORCE_UTF8_PRIOR;
            let mut r = Cursor::new(buffer.as_bytes());

            let mut copy = Vec::<u8>::new();
            let mut w = Cursor::new(&mut copy);
            // TODO: (Performance) We would like to use the stream directly,
            // but the borrow checker is unhappy. So we are copying after :(
            // TODO: deref_mut doesn't work here because it would use the slice.
            //let mut w = Cursor::new(stream.bytes);

            // TODO: Understand error conditions
            // TODO: The return here is the "pointer sized integer type".
            // Is this something we need to account for?
            BrotliCompress(&mut r, &mut w, &params).expect("Failed to brotli");

            stream.bytes.extend_from_slice(&copy);
        });

        stream.encode_with_id(|stream| lens.flush(stream));

        Ok(ArrayTypeId::BrotliUtf8)
    }
}

#[cfg(feature = "encode")]
pub(crate) struct Utf8Compressor;

// TODO: The Borrow<String> here is interesting. Can we get rid of other lifetimes?
#[cfg(feature = "encode")]
impl<T: Borrow<String>> Compressor<T> for Utf8Compressor {
    fn fast_size_for<O: EncodeOptions>(&self, data: &[T], _options: &O) -> Result<usize, ()> {
        profile_method!(fast_size_for);
        let mut total = 0;
        for s in data {
            total += size_for_varint(s.borrow().len() as u64);
            total += s.borrow().as_bytes().len();
        }
        Ok(total + size_for_varint(total as u64))
    }
    fn compress<O: EncodeOptions>(&self, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        profile_method!(compress);

        stream.encode_with_len(|stream| {
            for value in data.iter() {
                encode_prefix_varint(value.borrow().len() as u64, stream.bytes);
                stream.bytes.extend_from_slice(value.borrow().as_bytes());
            }
        });

        Ok(ArrayTypeId::Utf8)
    }
}
