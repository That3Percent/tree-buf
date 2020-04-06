use crate::prelude::*;

#[cfg(feature = "write")]
pub(crate) fn compress<'a: 'b, 'b, T>(data: &'a [T], bytes: &mut Vec<u8>, compressors: &'b [Box<dyn Compressor<'a, Data = T>>]) -> ArrayTypeId {
    // TODO: If there aren't multiple compressors, no need to be dynamic
    // debug_assert!(compressors.len() > 1);
    if compressors.len() == 1 {
        return compressors[0].compress(data, bytes).unwrap();
    }

    let restore_point = bytes.len();
    let sample_size = data.len().min(256);
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
