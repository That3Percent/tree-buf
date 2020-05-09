use crate::prelude::*;
use simple_16;
use std::vec::IntoIter;

// TODO: usize
// TODO: Use ReaderArray or InfallableReaderArray
pub struct RleIterator<T> {
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
        f: impl Send + FnOnce(DynArrayBranch<'_>) -> ReadResult<IntoIter<T>>,
    ) -> ReadResult<Self> {
        let (runs, values) = parallel(|| <u64 as Readable>::ReaderArray::new(*runs, options), || f(*values), options);
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

pub(crate) struct RLE<T> {
    // TODO: (Performance) Do not require the allocation of this Vec
    sub_compressors: Vec<Box<dyn Compressor<T>>>,
}

impl<T> RLE<T> {
    pub fn new(sub_compressors: Vec<Box<dyn Compressor<T>>>) -> Self {
        Self { sub_compressors }
    }
}

impl<T: PartialEq + Copy + Default + std::fmt::Debug> Compressor<T> for RLE<T> {
    fn compress(&self, data: &[T], bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> Result<ArrayTypeId, ()> {
        // Prevent panic on indexing first item.
        profile!(&[T], "RLE::compress");
        // It will always be more efficient to just defer to another encoding. Also, this prevents a panic.
        if data.len() == 0 {
            return Err(());
        }
        let mut runs = Vec::new();
        let mut current_run = 0u32;
        let mut current_value = data[0];
        let mut values = vec![];
        // This limit is imposed by Simple16.
        // See also 57b5623b-5222-4087-bc4b-0cd196adff07
        const MAX_RUN: u32 = (1u32 << 28) - 1;
        for item in data[1..].iter() {
            if current_value == *item && current_run < MAX_RUN {
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

        // Can't use write_with_id and write_with_len directly, because that would cause problems
        // with object safety.
        // See also f4aba341-af61-490f-b113-380cb4c38a77
        //
        let type_index = bytes.len();
        bytes.push(0);
        let len = bytes.len();
        let id = compress(&values[..], bytes, lens, &self.sub_compressors[..]);
        lens.push(bytes.len() - len);
        bytes[type_index] = id.into();

        // TODO: FIXME: Because of the traits and such, can't compress to a
        // a stream and re-use the existing code. So, assume Simple16 for the runs for now.
        // See also 57b5623b-5222-4087-bc4b-0cd196adff07
        // TODO: Impl Error trait for ValueOutOfRange in Simple16
        bytes.push(ArrayTypeId::IntSimple16.into());
        let len = bytes.len();
        simple_16::compress(&runs, bytes).map_err(|_| panic!("All values should be in range")).unwrap();
        lens.push(bytes.len() - len);

        Ok(ArrayTypeId::RLE)
    }
}
