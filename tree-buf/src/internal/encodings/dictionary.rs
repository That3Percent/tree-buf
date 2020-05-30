use crate::prelude::*;
use encodings::varint::encode_prefix_varint;
use std::collections::HashMap;
use std::convert::TryInto as _;
use std::hash::Hash;
use std::vec::IntoIter;

// TODO: usize
// TODO: Use ReaderArray or InfallableReaderArray
pub struct DictionaryIterator<T> {
    // FIXME: There may be a subtle issue here with compounded removing of default values.
    // It is not the same to remove a default elem from the original collection as
    // to remove a 0 from the end of indexes. If both happen, this could be a problem.
    // See also 522d2f4f-c5f7-478c-8d94-e7457ae45b29
    indexes: IntoIter<u64>,
    values: IntoIter<T>,
    cache: HashMap<u64, T>,
}

impl<T: Clone + Default> Iterator for DictionaryIterator<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self.indexes.next() {
            None => None,
            Some(index) => {
                while self.cache.len() <= index as usize {
                    // Have to use unwrap_or_default() here, because we used the compress fn
                    let next = self.values.next().unwrap_or_default();
                    self.cache.insert(self.cache.len() as u64, next);
                }
                Some(self.cache.get(&index).unwrap().clone())
            }
        }
    }
}

impl<T: Send + Clone> DictionaryIterator<T> {
    pub fn new(
        indexes: Box<DynArrayBranch<'_>>,
        values: Box<DynArrayBranch<'_>>,
        options: &impl DecodeOptions,
        f: impl Send + FnOnce(DynArrayBranch<'_>) -> ReadResult<IntoIter<T>>,
    ) -> ReadResult<Self> {
        let (indexes, values) = parallel(|| <u64 as Readable>::ReaderArray::new(*indexes, options), || f(*values), options);
        let indexes = indexes?;
        let values = values?;

        Ok(Self {
            indexes,
            values,
            cache: HashMap::new(),
        })
    }
}

pub(crate) struct Dictionary<T> {
    // TODO: (Performance) Do not require the allocation of this Vec
    sub_compressors: Vec<Box<dyn Compressor<T>>>,
}

impl<T> Dictionary<T> {
    pub fn new(sub_compressors: Vec<Box<dyn Compressor<T>>>) -> Self {
        Self { sub_compressors }
    }
}

impl<T: PartialEq + Copy + Default + std::fmt::Debug + Hash + Eq> Compressor<T> for Dictionary<T> {
    fn compress(&self, data: &[T], bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> Result<ArrayTypeId, ()> {
        // Prevent panic on indexing first item.
        profile!("compress");
        // It will always be more efficient to just defer to another encoding.
        if data.len() < 2 {
            return Err(());
        }

        // TODO: This calls for a specialized data structure
        let mut indices = Vec::<u64>::new();
        let mut values = Vec::new();
        let mut lookup = HashMap::new();

        for value in data.iter() {
            let index = if let Some(i) = lookup.get(value) {
                *i
            } else {
                let i = lookup.len();
                lookup.insert(value, i);
                values.push(*value);
                i
            };
            indices.push(index.try_into().map_err(|_| ())?);
        }

        debug_assert!(lookup.len() == values.len());
        debug_assert!(indices.len() == data.len());

        // If no values are removed, it is determined
        // that this cannot possibly be better,
        // so don't go through the compression step
        // for nothing.
        if indices.len() == values.len() {
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
        // a stream and re-use the existing code.
        // See also 57b5623b-5222-4087-bc4b-0cd196adff07
        //
        // TODO: We would probably prefer Simple16 here, but because of a subtle
        // issue we have to use PrefixVar instead. The problem is that compress removes
        // default values from the end of the array, but with Simple16 we don't know
        // for sure if a 0 at the end is a default value or just "padding" from the
        // encoder. This comes at a significant loss to compression in many cases,
        // since it is very likely that most values are small. This would be a problem
        // if we tried to use the compress fn here too.
        // See also 490cf083-7fba-49ea-a14a-41c4ba91a656
        bytes.push(ArrayTypeId::IntPrefixVar.into());
        let len = bytes.len();
        for item in indices {
            encode_prefix_varint(item, bytes);
        }
        lens.push(bytes.len() - len);

        Ok(ArrayTypeId::Dictionary)
    }
}
