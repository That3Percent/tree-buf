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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
    pub fn round_trip<T: Copy + PartialEq + Debug>(data: &[T], encoder: impl Fn(T, &mut Vec<u8>), decoder: impl Fn(&[u8], &mut usize)->T) {
        let mut bytes = Vec::new();
        for value in data.iter() {
            encoder(*value, &mut bytes);
        }

        let result = read_all(&bytes, decoder);
        
        assert_eq!(&result, &data);
    }
}