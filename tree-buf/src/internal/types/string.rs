use crate::prelude::*;
use crate::encodings::varint::{encode_prefix_varint, decode_prefix_varint};

pub struct Str;

// TODO: Move this to BatchData
impl Str {
    pub fn write_one(value: &str, bytes: &mut Vec<u8>) {
        encode_prefix_varint(value.len() as u64, bytes);
        bytes.extend_from_slice(value.as_bytes());
    }
    pub fn read_one<'a>(bytes: &'a [u8], offset: &'_ mut usize) -> ReadResult<&'a str> {
        let len = decode_prefix_varint(bytes, offset)? as usize;
        let utf8 = read_bytes(bytes, len, offset)?;
        Ok(std::str::from_utf8(utf8)?)
    }
}