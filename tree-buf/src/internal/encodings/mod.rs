pub mod varint;
pub mod packed_bool;

/// Reads all items from some byte aligned encoding
pub fn read_all<T>(bytes: &[u8], f: impl Fn(&[u8], &mut usize)->T) -> Vec<T> {
    let mut offset = 0;
    let mut result = Vec::new();
    while offset < bytes.len() {
        let read = f(bytes, &mut offset);
        result.push(read);
    }
    debug_assert_eq!(offset, bytes.len());

    result
}