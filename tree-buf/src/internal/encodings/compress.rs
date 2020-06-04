use crate::prelude::*;

#[cfg(feature = "encode")]
pub(crate) fn compress<T: PartialEq + Default, O: EncodeOptions>(data: &[T], stream: &mut EncoderStream<'_, O>, compressors: &impl CompressorSet<T>) -> ArrayTypeId {
    profile!(T, "compress");

    // Remove trailing default values.
    // All the decoders always generate defaults when values "run out".
    // These cause problems with nested encodings like Dictionary and RLE
    // TODO: Bring back in "root" compression schemes?
    /*
    let default = T::default(); // TODO: (Performance) Minor benefit here by not allocating String and having an "IsDefault" trait.
    let trailing_defaults = data.iter().rev().take_while(|i| *i == &default);
    let data = &data[0..data.len() - trailing_defaults.count()];
    */

    // If there aren't multiple compressors, no need to be dynamic
    if compressors.len() == 1 {
        return compressors.compress(0, data, stream).unwrap();
    }

    let restore_bytes = stream.bytes.len();
    // TODO: Yuck!. This is ugly and error prone to restore these
    // and update the byte count with the assumed compressor for lens
    let restore_lens = stream.lens.len();
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
            if compressors.compress(i, sample, stream).is_ok() {
                let mut size = stream.bytes.len() - restore_bytes;
                for len in &stream.lens[restore_lens..stream.lens.len()] {
                    size += crate::internal::encodings::varint::size_for_varint(*len as u64);
                }
                by_size.push((i, size));
            }
            stream.bytes.truncate(restore_bytes);
            stream.lens.truncate(restore_lens);
        }
    }

    // Sorting stable allows us to have a preference for one encoder over another.
    by_size.sort_by_key(|&(_, size)| size);

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
pub(crate) trait Compressor<T> {
    /// If it's possible to figure out how big the data will be without
    /// compressing it, implement that here.
    fn fast_size_for(&self, _data: &[T]) -> Option<usize> {
        None
    }
    fn compress<O: EncodeOptions>(&self, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()>;
}


pub (crate) trait CompressorSet<T> {
    fn len(&self) -> usize;
    fn fast_size_for(&self, compressor: usize, data: &[T]) -> Option<usize>;
    fn compress<O: EncodeOptions>(&self, compressor: usize, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()>;
}


