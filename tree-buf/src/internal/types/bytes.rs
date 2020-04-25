use crate::prelude::*;

#[cfg(feature = "read")]
pub fn read_bytes<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize) -> ReadResult<&'a [u8]> {
    let start = *offset;
    let end = start.checked_add(len).ok_or(ReadError::InvalidFormat)?;
    if end > bytes.len() {
        return Err(ReadError::InvalidFormat);
    }
    *offset = end;
    Ok(&bytes[start..end])
}
