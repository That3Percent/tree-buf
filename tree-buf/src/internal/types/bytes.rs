use crate::prelude::*;

pub fn read_bytes<'a>(bytes: &'a [u8], len: usize, offset: &'_ mut usize) -> ReadResult<&'a [u8]> {
    let start = *offset;
    let end = start + len;
    if start > bytes.len() || end > bytes.len() {
        return Err(ReadError::InvalidFormat);
    }
    *offset = end;
    Ok(&bytes[start..end])
}