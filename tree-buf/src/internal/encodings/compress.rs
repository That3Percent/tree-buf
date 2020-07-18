use crate::prelude::*;

#[cfg(feature = "encode")]
pub(crate) fn compress<T: PartialEq, O: EncodeOptions>(data: &[T], stream: &mut EncoderStream<'_, O>, compressors: &impl CompressorSet<T>) -> ArrayTypeId {
    profile_fn!(master_compress);

    // If there aren't multiple compressors, no need to be dynamic
    if compressors.len() == 1 {
        return compressors.compress(0, data, stream).unwrap();
    }

    profile_section!(samples);
    let restore_bytes = stream.bytes.len();
    let restore_lens = stream.lens.len();
    let sample_size = data.len().min(256);
    let sample = &data[..sample_size];

    // Rank compressors by how well they do on a sample of the data
    // TODO: Use second-stack, or considering how few items there are fixed tuples with sort or iter.
    let mut by_size = Vec::new();
    for i in 0..compressors.len() {
        // FIXME: A lot of these implementations are wrong, because they do not account for the lens or type id
        // If a compressor returns Err, that's because it determines as an early out that another compressor is always going to be better.
        if let Ok(size) = compressors.fast_size_for(i, sample, stream.options) {
            by_size.push((i, size));
        }
    }

    drop(samples);

    profile_section!(actual_compress);

    by_size.sort_unstable_by_key(|&(_, size)| size);

    // Return the first compressor that succeeds
    for ranked in by_size.iter() {
        if let Ok(ok) = compressors.compress(ranked.0, data, stream) {
            return ok;
        }
        // If the compressor failed, clear out whatever it wrote to try again.
        stream.bytes.truncate(restore_bytes);
        stream.lens.truncate(restore_lens);
    }

    // This must be called with at least one infallable compressor.
    panic!("Missing infallable compressor for type");
}

#[cfg(feature = "encode")]
pub(crate) fn fast_size_for<T: PartialEq, O: EncodeOptions>(data: &[T], compressors: &impl CompressorSet<T>, options: &O) -> usize {
    profile_fn!(master fast_size_for);

    let mut min = usize::MAX;
    for i in 0..compressors.len() {
        // If a compressor returns Err, that's because it determines as an early out that another compressor is always going to be better.
        if let Ok(size) = compressors.fast_size_for(i, data, options) {
            min = size.min(min);
        }
    }
    debug_assert!(min != usize::MAX);
    // TODO: When compiled in debug, verify the size of some of these.
    min
}

#[cfg(feature = "encode")]
pub(crate) trait Compressor<T> {
    /// Report how big the data will be without actually doing the work of compressing.
    /// Only return Err in 2 cases:
    ///   * If the compressor would fail to compress the data
    ///   * If it is known that another compressor surely can compress better
    fn fast_size_for<O: EncodeOptions>(&self, data: &[T], options: &O) -> Result<usize, ()>;
    fn compress<O: EncodeOptions>(&self, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()>;
}

pub(crate) trait CompressorSet<T> {
    fn len(&self) -> usize;
    // TODO: Replace with fast_smallest_size(&self, data: &[T], options: &O) -> usize;
    // Then make parallel
    fn fast_size_for<O: EncodeOptions>(&self, compressor: usize, data: &[T], options: &O) -> Result<usize, ()>;
    fn compress<O: EncodeOptions>(&self, compressor: usize, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()>;
}
