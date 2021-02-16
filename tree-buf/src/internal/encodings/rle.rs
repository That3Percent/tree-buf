use crate::prelude::*;
use std::cell::RefCell;
use std::thread_local;
use std::vec::IntoIter;

// FIXME: This won't fly when encodes are mult-threaded.
// Should use options here, but the wanted closure API isn't
// easy for some reason. Also, this API is fragile and won't withstand panic.
// That will have to be fixed when using arbitrary writers.
thread_local! {
    static IN_RLE_ENCODE: RefCell<bool> = RefCell::new(false);
}

pub(crate) fn within_rle<T>(f: impl FnOnce() -> Result<T, ()>) -> Result<T, ()> {
    if IN_RLE_ENCODE.with(|v| *v.borrow()) {
        Err(())
    } else {
        IN_RLE_ENCODE.with(|v| *v.borrow_mut() = true);
        let result = f();
        IN_RLE_ENCODE.with(|v| *v.borrow_mut() = false);
        result
    }
}

// TODO: Use DecoderArray or InfallableDecoderArray
pub struct RleIterator<T> {
    // See also 522d2f4f-c5f7-478c-8d94-e7457ae45b29
    runs: IntoIter<u64>,
    values: IntoIter<T>,
    current_run: Option<u64>,
    current_value: Option<T>,
}

impl<T: Clone> Iterator for RleIterator<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self.current_run {
            None => {
                self.current_run = Some(self.runs.next().unwrap_or_default());
                self.current_value = self.values.next();
                self.next()
            }
            Some(0) => {
                self.current_run = None;
                self.current_value.take()
            }
            Some(run) => {
                self.current_run = Some(run - 1);
                self.current_value.clone()
            }
        }
    }
}

impl<T: Send + Clone> RleIterator<T> {
    pub fn new(
        runs: Box<DynArrayBranch<'_>>,
        values: Box<DynArrayBranch<'_>>,
        options: &impl DecodeOptions,
        f: impl Send + FnOnce(DynArrayBranch<'_>) -> DecodeResult<IntoIter<T>>,
    ) -> DecodeResult<Self> {
        let (runs, values) = parallel(|| <u64 as Decodable>::DecoderArray::new(*runs, options), || f(*values), options);
        let runs = runs?;
        let values = values?;

        Ok(Self {
            current_run: None,
            current_value: None,
            runs,
            values,
        })
    }
}

pub(crate) struct RLE<S> {
    // TODO: (Performance) Do not require the allocation of this Vec
    sub_compressors: S,
}

impl<S> RLE<S> {
    pub fn new(sub_compressors: S) -> Self {
        Self { sub_compressors }
    }
}

// See also 2a3a69eb-eba1-4c95-9399-f1b9daf48733
fn get_runs<T: PartialEq + Copy>(data: &[T]) -> Result<(Vec<u64>, Vec<T>), ()> {
    // It will always be more efficient to just defer to another encoding. Also, this prevents a panic.
    if data.len() < 2 {
        return Err(());
    }

    // Prevent panic on indexing first item.
    profile_fn!(rle_get_runs);

    let mut runs = Vec::new();
    let mut current_run = 0_u64;
    let mut current_value = data[0];
    let mut values = vec![];
    for item in data[1..].iter() {
        if current_value == *item {
            current_run += 1;
        } else {
            runs.push(current_run);
            values.push(current_value);
            current_value = *item;
            current_run = 0;
        }
    }
    runs.push(current_run);
    values.push(current_value);
    debug_assert!(runs.len() == values.len());

    // If no values are removed, it is determined
    // that this cannot possibly be better,
    // so don't go through the compression step
    // for nothing.
    if values.len() == data.len() {
        Err(())
    } else {
        Ok((runs, values))
    }
}

impl<T: PartialEq + Copy + std::fmt::Debug, S: CompressorSet<T>> Compressor<T> for RLE<S> {
    fn compress<O: EncodeOptions>(&self, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        profile_method!(compress);

        within_rle(|| {
            let (runs, values) = get_runs(data)?;

            stream.encode_with_id(|stream| compress(&values[..], stream, &self.sub_compressors));
            stream.encode_with_id(|stream| runs.flush(stream));

            Ok(ArrayTypeId::RLE)
        })
    }

    fn fast_size_for<O: EncodeOptions>(&self, data: &[T], options: &O) -> Result<usize, ()> {
        profile_method!(fast_size_for);

        within_rle(|| {
            let (runs, values) = get_runs(data)?;

            let from_values = fast_size_for(&values[..], &self.sub_compressors, options);
            let from_runs = Vec::<u64>::fast_size_for_all(&runs[..], options);

            let from_ids = 2;

            Ok(from_ids + from_runs + from_values)
        })
    }
}
