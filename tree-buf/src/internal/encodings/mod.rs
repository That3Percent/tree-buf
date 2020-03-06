pub mod delta;
pub mod packed_bool;
pub mod varint;
#[cfg(feature = "write")]
use crate::internal::encodings::varint::size_for_varint;
use crate::prelude::*;

#[cfg(feature = "write")]
pub(crate) fn compress<'a: 'b, 'b, T>(data: &'a [T], bytes: &mut Vec<u8>, compressors: &'b [Box<dyn Compressor<'a, Data = T>>]) -> ArrayTypeId {
    // TODO: If there aren't multiple compressors, no need to be dynamic
    // debug_assert!(compressors.len() > 1);
    if compressors.len() == 1 {
        return compressors[0].compress(data, bytes).unwrap();
    }

    let restore_point = bytes.len();
    let sample_size = data.len().min(512);
    let sample = &data[..sample_size];

    // Rank compressors by how well they do on a sample of the data
    // TODO: Use second-stack
    // TODO (Performance): This is silly how sometimes the sample size is the
    // entire value, but we end up encoding twice. If the most likely best
    // is at the end, then we can just keep it in the case where it wins
    let mut by_size = Vec::new();
    for i in 0..compressors.len() {
        let compressor = &compressors[i];
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
        let compressor = &compressors[ranked.0];
        if let Ok(ok) = compressor.compress(data, bytes) {
            return ok;
        }
        // If the compressor failed, clear out whatever it wrote to try again.
        bytes.truncate(restore_point);
    }

    // This must be called with at least one infallable compressor.
    panic!("Missing infallable compressor for type");
}

#[cfg(feature = "write")]
pub(crate) trait Compressor<'a> {
    type Data;
    /// If it's possible to figure out how big the data will be without
    /// compressing it, implement that here.
    fn fast_size_for(&self, _data: &[Self::Data]) -> Option<usize> {
        None
    }
    fn compress(&self, data: &[Self::Data], bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()>;
}

#[cfg(feature = "write")]
pub(crate) struct Utf8Compressor;

#[cfg(feature = "write")]
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
    fn compress(&self, _data: &[Self::Data], _bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()> {
        todo!();
    }
}

#[cfg(feature = "read")]
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
    #[cfg(all(feature = "read", feature = "write"))]
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
