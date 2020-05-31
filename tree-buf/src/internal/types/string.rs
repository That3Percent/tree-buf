use crate::internal::encodings::varint::*;
use crate::prelude::*;
use rle::RLE;
use std::vec::IntoIter;

// TODO: Consider compressed unicode (SCSU?) for String in general,
// but in particular for schema strings. As schema strings need only
// be compared and usually not displayed we can do bit-for-bit comparisons
// (Make sure that's true for SCSU, which may allow multiple encodings!)

#[cfg(feature = "write")]
pub fn write_str<O: EncodeOptions>(value: &str, stream: &mut WriterStream<'_, O>) {
    write_usize(value.len(), stream);
    stream.bytes.extend_from_slice(value.as_bytes());
}

#[cfg(feature = "read")]
fn read_str_len<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize) -> ReadResult<&'a str> {
    let utf8 = read_bytes(len, bytes, offset)?;
    Ok(std::str::from_utf8(utf8)?)
}

#[cfg(feature = "read")]
pub fn read_str<'a>(bytes: &'a [u8], offset: &'_ mut usize) -> ReadResult<&'a str> {
    let len = decode_prefix_varint(bytes, offset)? as usize;
    read_str_len(len, bytes, offset)
}

#[cfg(feature = "write")]
impl Writable for String {
    type WriterArray = Vec<&'static str>;
    fn write_root<O: EncodeOptions>(&self, stream: &mut WriterStream<'_, O>) -> RootTypeId {
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

#[cfg(feature = "write")]
impl WriterArray<String> for Vec<&'static str> {
    fn buffer<'a, 'b: 'a>(&'a mut self, value: &'b String) {
        // TODO: Working around lifetime issues for lack of GAT
        // A quick check makes this appear to be sound, since the signature
        // requires that the value outlive self.
        //
        // The big safety problem is that whe then give these references
        // away when flushing. We happen to know that nothing saves the references,
        // but when things like threading come into play it's hard to know.
        self.push(unsafe { std::mem::transmute(value.as_str()) });
    }
    fn flush<O: EncodeOptions>(self, stream: &mut WriterStream<'_, O>) -> ArrayTypeId {
        profile!("WriterArray::flush");

        let rle_inner: Vec<Box<dyn Compressor<&'static str>>> = vec![Box::new(Utf8Compressor)];
        let rle = RLE::new(rle_inner);

        let compressors: Vec<Box<dyn Compressor<&'static str>>> = vec![
            Box::new(Utf8Compressor),
            Box::new(rle),
            Box::new(Dictionary::new(vec![Box::new(Utf8Compressor)])),
        ];

        // TODO: This write_with_len puts an unnecessary len
        // See also 40ea8819-da26-4af3-8dc0-1a4602560f30
        stream.write_with_len(|stream| compress(&self, stream.bytes, stream.lens, &compressors))
    }
}

#[cfg(feature = "read")]
impl Readable for String {
    // TODO: Use lifetimes to make this read lazy rather than IntoIter
    type ReaderArray = IntoIter<String>;
    fn read(sticks: DynRootBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("Readable::read");
        match sticks {
            DynRootBranch::String(s) => Ok(s.to_owned()),
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "read")]
impl InfallibleReaderArray for IntoIter<String> {
    type Read = String;

    fn new_infallible(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("ReaderArray::new");

        match sticks {
            DynArrayBranch::String(bytes) => {
                #[cfg(feature = "profile")]
                let _g = flame::start_guard("String");

                let strs = read_all(&bytes, |b, o| read_str(b, o).and_then(|v| Ok(v.to_owned())))?;
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
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next_infallible(&mut self) -> Self::Read {
        self.next().unwrap_or_default()
    }
}

#[cfg(feature = "write")]
impl<'a> Compressor<&'a str> for Utf8Compressor {
    fn fast_size_for(&self, data: &[&'a str]) -> Option<usize> {
        profile!("Compressor::fast_size_for");
        let mut total = 0;
        for s in data {
            total += size_for_varint(s.len() as u64);
            total += s.as_bytes().len();
        }
        Some(total)
    }
    fn compress(&self, data: &[&'a str], bytes: &mut Vec<u8>, _lens: &mut Vec<usize>) -> Result<ArrayTypeId, ()> {
        profile!("Compressor::compress");

        for value in data.iter() {
            encode_prefix_varint(value.len() as u64, bytes);
            bytes.extend_from_slice(value.as_bytes());
        }

        Ok(ArrayTypeId::Utf8)
    }
}
