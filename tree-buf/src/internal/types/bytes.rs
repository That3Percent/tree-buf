pub fn read_bytes<'a>(bytes: &'a [u8], len: usize, offset: &'_ mut usize) -> &'a [u8] {
    let start = *offset;
    let end = start + len;
    *offset = end;
    &bytes[start..end]
}