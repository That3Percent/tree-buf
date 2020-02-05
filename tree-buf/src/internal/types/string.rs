use crate::encodings::varint::{decode_prefix_varint, encode_prefix_varint};
use crate::prelude::*;
use std::vec::IntoIter;

// TODO: Consider compressed unicode (SCSU?) for String in general,
// but in particular for schema strings. As schema strings need only
// be compared and usually not displayed we can do bit-for-bit comparisons
// (Make sure that's true for SCSU, which may allow multiple encodings!)

// TODO: Move this to BatchData
pub fn write_str(value: &str, bytes: &mut Vec<u8>) {
    encode_prefix_varint(value.len() as u64, bytes);
    bytes.extend_from_slice(value.as_bytes());
}

fn read_str_len<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize) -> ReadResult<&'a str> {
    let utf8 = read_bytes(len, bytes, offset)?;
    Ok(std::str::from_utf8(utf8)?)
}
pub fn read_str<'a>(bytes: &'a [u8], offset: &'_ mut usize) -> ReadResult<&'a str> {
    let len = decode_prefix_varint(bytes, offset)? as usize;
    read_str_len(len, bytes, offset)
}

impl<'a> Writable<'a> for String {
    type WriterArray = Vec<&'a str>;
    fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, _lens: &mut Vec<usize>) -> RootTypeId {
        let value = value.as_str();
        match value.len() {
            0 => RootTypeId::Str0,
            1 => {
                bytes.push(value.as_bytes()[0]);
                RootTypeId::Str1
            }
            2 => {
                bytes.extend_from_slice(value.as_bytes());
                RootTypeId::Str2
            }
            3 => {
                bytes.extend_from_slice(value.as_bytes());
                RootTypeId::Str3
            }
            _ => {
                let b = value.as_bytes();
                encode_prefix_varint(b.len() as u64, bytes);
                bytes.extend_from_slice(b);
                RootTypeId::Str
            }
        }
    }
}

impl<'a> WriterArray<'a> for Vec<&'a str> {
    type Write = String;

    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.push(value.as_str());
    }
    fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId {
        let start = bytes.len();
        for s in self.iter() {
            write_str(s, bytes)
        }
        let len = bytes.len() - start;
        lens.push(len);

        ArrayTypeId::Utf8
    }
}

impl Readable for String {
    // TODO: Use lifetimes to make this read lazy rather than IntoIter
    type ReaderArray = IntoIter<String>;
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynRootBranch::String(s) => Ok(s.to_owned()),
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

impl ReaderArray for IntoIter<String> {
    type Read = String;
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynArrayBranch::String(bytes) => {
                let strs = read_all(bytes, |b, o| read_str(b, o).and_then(|v| Ok(v.to_owned())))?;
                Ok(strs.into_iter())
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next(&mut self) -> ReadResult<Self::Read> {
        self.next().ok_or_else(|| ReadError::InvalidFormat(InvalidFormat::ShortArray))
    }
}
