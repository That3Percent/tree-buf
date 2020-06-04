use crate::prelude::*;

#[cfg(feature = "decode")]
pub fn decode_bytes<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize) -> DecodeResult<&'a [u8]> {
    let start = *offset;
    let end = start.checked_add(len).ok_or(DecodeError::InvalidFormat)?;
    if end > bytes.len() {
        return Err(DecodeError::InvalidFormat);
    }
    *offset = end;
    Ok(&bytes[start..end])
}
