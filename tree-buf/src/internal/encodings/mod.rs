pub mod delta;
pub mod packed_bool;
pub mod varint;
use crate::encodings::varint::size_for_varint;
use crate::prelude::*;

fn compress<'a, 'b: 'a, T>(data: &'a [T], bytes: &mut Vec<u8>, compressors: &'b [&'b dyn Compressor<'a, Data = T>]) -> usize {
    // Skip the whole bit about approximating the cost if there's just one.
    if compressors.len() == 1 {
        (&compressors[0]).compress(data, bytes).unwrap();
        return 0;
    }
    let restore_point = bytes.len();
    let sample_size = bytes.len().min(512);
    let sample = &data[..sample_size];

    // Rank compressors by how well they do on a sample of the data
    // TODO: Use second_stack
    let mut by_size = Vec::new();
    for i in 0..compressors.len() {
        let compressor = compressors[i];
        if let Some(size) = compressor.fast_size_for(sample) {
            by_size.push((i, size));
        } else {
            if compressor.compress(sample, bytes).is_ok() {
                let size = bytes.len() - restore_point;
                by_size.push((i, size));
            }
            bytes.truncate(restore_point);
        }
    }

    // Sorting stable allows us to have a preference for one encoder over another.
    by_size.sort_by_key(|&(_, size)| size);

    // Return the first compressor that succeeds
    for ranked in by_size.iter() {
        let compressor = compressors[ranked.0];
        if compressor.compress(data, bytes).is_ok() {
            return ranked.0;
        }
        // If the compressor failed, clear out whatever it wrote to try again.
        bytes.truncate(restore_point);
    }

    // This must be called with at least one infallable compressor.
    panic!("Missing infallable compressor for type");
}

pub(crate) trait Compressor<'a> {
    type Data;
    /// If it's possible to figure out how big the data will be without
    /// compressing it, implement that here.
    fn fast_size_for(&self, _data: &[Self::Data]) -> Option<usize> {
        None
    }
    fn compress(&self, data: &[Self::Data], bytes: &mut Vec<u8>) -> Result<(), ()>;
}

pub(crate) struct Utf8Compressor;
impl<'a> Compressor<'a> for Utf8Compressor {
    type Data = &'a str;
    fn fast_size_for(&self, data: &[Self::Data]) -> Option<usize> {
        let mut total = 0;
        for s in data {
            total += size_for_varint(s.len() as u64);
            total += s.as_bytes().len();
        }
        Some(total)
    }
    fn compress(&self, data: &[Self::Data], bytes: &mut Vec<u8>) -> Result<(), ()> {
        todo!();
    }
}

/// Reads all items from some byte aligned encoding
pub fn read_all<T>(bytes: &[u8], f: impl Fn(&[u8], &mut usize) -> ReadResult<T>) -> ReadResult<Vec<T>> {
    let mut offset = 0;
    let mut result = Vec::new();
    while offset < bytes.len() {
        let read = f(bytes, &mut offset)?;
        result.push(read);
    }
    debug_assert_eq!(offset, bytes.len());

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
    pub fn round_trip<T: Copy + PartialEq + Debug>(data: &[T], encoder: impl Fn(T, &mut Vec<u8>), decoder: impl Fn(&[u8], &mut usize) -> ReadResult<T>) -> ReadResult<()> {
        let mut bytes = Vec::new();
        for value in data.iter() {
            encoder(*value, &mut bytes);
        }

        let result = read_all(&bytes, decoder)?;

        assert_eq!(&result, &data);
        Ok(())
    }
}
