use crate::prelude::*;
use std::cell::RefCell;
use std::thread_local;
use std::vec::IntoIter;

// FIXME: This won't fly when encodes are mult-threaded.
// Should use options here, but the wanted closure API isn't
// easy for some reason. Also, this API is fragile.
thread_local! {
    static IN_RLE_ENCODE: RefCell<bool> = RefCell::new(false);
}

pub fn get_in_rle() -> bool {
    IN_RLE_ENCODE.with(|v| *v.borrow())
}
pub fn set_in_rle(value: bool) {
    IN_RLE_ENCODE.with(|v| *v.borrow_mut() = value);
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
            Some(run) => match run {
                0 => {
                    self.current_run = None;
                    self.current_value.take()
                }
                _ => {
                    self.current_run = Some(run - 1);
                    self.current_value.clone()
                }
            },
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

impl<T: PartialEq + Copy + std::fmt::Debug, S: CompressorSet<T>> Compressor<T> for RLE<S> {
    fn compress<O: EncodeOptions>(&self, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        // Nesting creates performance problems
        if get_in_rle() {
            return Err(());
        }

        // It will always be more efficient to just defer to another encoding. Also, this prevents a panic.
        if data.len() < 2 {
            return Err(());
        }

        // Prevent panic on indexing first item.
        profile!(&[T], "RLE::compress");

        let mut runs = Vec::new();
        let mut current_run = 0u64;
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
            return Err(());
        }

        stream.encode_with_id(|stream| compress(&values[..], stream, &self.sub_compressors));

        set_in_rle(true);
        stream.encode_with_id(|stream| runs.flush(stream));
        set_in_rle(false);

        Ok(ArrayTypeId::RLE)
    }
}
