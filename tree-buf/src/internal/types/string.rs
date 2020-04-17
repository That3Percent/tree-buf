use crate::internal::encodings::varint::*;
use crate::prelude::*;
use std::vec::IntoIter;

// TODO: Consider compressed unicode (SCSU?) for String in general,
// but in particular for schema strings. As schema strings need only
// be compared and usually not displayed we can do bit-for-bit comparisons
// (Make sure that's true for SCSU, which may allow multiple encodings!)

#[cfg(feature = "write")]
pub fn write_str(value: &str, stream: &mut impl WriterStream) {
    write_usize(value.len(), stream);
    stream.bytes().extend_from_slice(value.as_bytes());
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
impl<'a> Writable<'a> for String {
    type WriterArray = Vec<&'a str>;
    fn write_root<'b: 'a>(&'b self, stream: &mut impl WriterStream) -> RootTypeId {
        let value = self.as_str();
        match value.len() {
            0 => RootTypeId::Str0,
            1 => {
                stream.bytes().push(value.as_bytes()[0]);
                RootTypeId::Str1
            }
            2 => {
                stream.bytes().extend_from_slice(value.as_bytes());
                RootTypeId::Str2
            }
            3 => {
                stream.bytes().extend_from_slice(value.as_bytes());
                RootTypeId::Str3
            }
            _ => {
                let b = value.as_bytes();
                encode_prefix_varint(b.len() as u64, stream.bytes());
                stream.bytes().extend_from_slice(b);
                RootTypeId::Str
            }
        }
    }
}

#[cfg(feature = "write")]
impl<'a> WriterArray<'a> for Vec<&'a str> {
    type Write = String;

    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.push(value.as_str());
    }
    fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId {
        profile!("WriterArray::flush");
        stream.write_with_len(|stream| {
            for s in self.iter() {
                write_str(s, stream)
            }
        });

        ArrayTypeId::Utf8
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
    
    fn new_infallible(sticks: DynArrayBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("ReaderArray::new");

        match sticks {
            DynArrayBranch::String(bytes) => {
                #[cfg(feature="profile")]
                let _g = flame::start_guard("String");

                let strs = read_all(&bytes, |b, o| read_str(b, o).and_then(|v| Ok(v.to_owned())))?;
                Ok(strs.into_iter())
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
    fn compress(&self, _data: &[&'a str], _bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()> {
        profile!("Compressor::compress");
        todo!("utf8 compress");
    }
}