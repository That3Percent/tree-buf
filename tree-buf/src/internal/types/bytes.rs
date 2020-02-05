use crate::prelude::*;

pub fn read_bytes<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize) -> ReadResult<&'a [u8]> {
    let start = *offset;
    // TODO: Check for overflow
    let end = start + len;
    if end > bytes.len() {
        return Err(ReadError::InvalidFormat(InvalidFormat::EndOfFile));
    }
    *offset = end;
    Ok(&bytes[start..end])
}
