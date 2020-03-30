use crate::internal::encodings::varint::*;
use crate::prelude::*;
use std::vec::IntoIter;

// TODO: Consider compressed unicode (SCSU?) for String in general,
// but in particular for schema strings. As schema strings need only
// be compared and usually not displayed we can do bit-for-bit comparisons
// (Make sure that's true for SCSU, which may allow multiple encodings!)

#[cfg(feature = "write")]
pub fn write_str(value: &str, bytes: &mut Vec<u8>) {
    encode_prefix_varint(value.len() as u64, bytes);
    bytes.extend_from_slice(value.as_bytes());
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
        stream.write_with_len(|stream| {
            for s in self.iter() {
                write_str(s, stream.bytes())
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
        match sticks {
            DynRootBranch::String(s) => Ok(s.to_owned()),
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "read")]
impl ReaderArray for IntoIter<String> {
    type Read = String;
    fn new(sticks: DynArrayBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
        match sticks {
            DynArrayBranch::String(bytes) => {
                let strs = read_all(&bytes, |b, o| read_str(b, o).and_then(|v| Ok(v.to_owned())))?;
                Ok(strs.into_iter())
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next(&mut self) -> Self::Read {
        self.next().unwrap_or_default()
    }
}
