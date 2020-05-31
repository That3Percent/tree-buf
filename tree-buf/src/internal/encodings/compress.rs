use crate::prelude::*;

#[cfg(feature = "write")]
pub(crate) fn compress<T: PartialEq + Default>(data: &[T], bytes: &mut Vec<u8>, lens: &mut Vec<usize>, compressors: &impl CompressorSet<T>) -> ArrayTypeId {
    profile!(T, "compress");

    // Remove trailing default values.
    // All the readers always generate defaults when values "run out".
    // These cause problems with nested encodings like Dictionary and RLE
    // TODO: Bring back in "root" compression schemes?
    /*
    let default = T::default(); // TODO: (Performance) Minor benefit here by not allocating String and having an "IsDefault" trait.
    let trailing_defaults = data.iter().rev().take_while(|i| *i == &default);
    let data = &data[0..data.len() - trailing_defaults.count()];
    */

    // TODO: If there aren't multiple compressors, no need to be dynamic
    // debug_assert!(compressors.len() > 1);
    if compressors.len() == 1 {
        return compressors.compress(0, data, bytes, lens).unwrap();
    }

    let restore_bytes = bytes.len();
    // TODO: Yuck!. This is ugly and error prone to restore these
    // and update the byte count with the assumed compressor for lens
    let restore_lens = lens.len();
    let sample_size = data.len().min(256);
    let sample = &data[..sample_size];

    // Rank compressors by how well they do on a sample of the data
    // TODO: Use second-stack
    // TODO (Performance): This is silly how sometimes the sample size is the
    // entire value, but we end up encoding twice. If the most likely best
    // is at the end, then we can just keep it in the case where it wins
    let mut by_size = Vec::new();
    for i in 0..compressors.len() {
        if let Some(size) = compressors.fast_size_for(i, sample) {
            by_size.push((i, size));
        } else {
            if compressors.compress(i, sample, bytes, lens).is_ok() {
                let mut size = bytes.len() - restore_bytes;
                for len in &lens[restore_lens..lens.len()] {
                    size += crate::internal::encodings::varint::size_for_varint(*len as u64);
                }
                by_size.push((i, size));
            }
            bytes.truncate(restore_bytes);
            lens.truncate(restore_lens);
        }
    }

    // Sorting stable allows us to have a preference for one encoder over another.
    by_size.sort_by_key(|&(_, size)| size);

    // Return the first compressor that succeeds
    for ranked in by_size.iter() {
        if let Ok(ok) = compressors.compress(ranked.0, data, bytes, lens) {
            return ok;
        }
        // If the compressor failed, clear out whatever it wrote to try again.
        bytes.truncate(restore_bytes);
        lens.truncate(restore_lens);
    }

    // This must be called with at least one infallable compressor.
    panic!("Missing infallable compressor for type");
}

#[cfg(feature = "write")]
pub(crate) trait Compressor<T> {
    /// If it's possible to figure out how big the data will be without
    /// compressing it, implement that here.
    fn fast_size_for(&self, _data: &[T]) -> Option<usize> {
        None
    }
    fn compress(&self, data: &[T], bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> Result<ArrayTypeId, ()>;
}


// TODO: Remove the ?Sized and the impl for [Box<dyn Compressor<T>>]
pub (crate) trait CompressorSet<T> {
    fn len(&self) -> usize;
    fn fast_size_for(&self, compressor: usize, data: &[T]) -> Option<usize>;
    fn compress(&self, compressor: usize, data: &[T], bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> Result<ArrayTypeId, ()>;
}

impl<T> CompressorSet<T> for Vec<Box<dyn Compressor<T>>> {
    fn len(&self) -> usize {
        <[Box<dyn Compressor<T>>]>::len(self)
    }
    fn fast_size_for(&self, compressor: usize, data: &[T]) -> Option<usize> {
        self[compressor].fast_size_for(data)
    }
    fn compress(&self, compressor: usize, data: &[T], bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> Result<ArrayTypeId, ()> {
        self[compressor].compress(data, bytes, lens)
    }
}