use crate::prelude::*;
use std::collections::HashMap;
use std::hash::Hash;
use std::vec::IntoIter;

// TODO: usize
// TODO: Use DecoderArray or InfallableDecoderArray
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
        f: impl Send + FnOnce(DynArrayBranch<'_>) -> DecodeResult<IntoIter<T>>,
    ) -> DecodeResult<Self> {
        let (indexes, values) = parallel(|| <u64 as Decodable>::DecoderArray::new(*indexes, options), || f(*values), options);
        let indexes = indexes?;
        let values = values?;

        Ok(Self {
            indexes,
            values,
            cache: HashMap::new(),
        })
    }
}

pub(crate) struct Dictionary<S> {
    sub_compressors: S,
}

impl<S> Dictionary<S> {
    pub fn new(sub_compressors: S) -> Self {
        Self { sub_compressors }
    }
}

// TODO: (Performance) Drop to u32 or below when possible. This showed small but non-negligible improvement.
// That improvement would apply to other types as well (eg: enum variants, rle)
// See also 2a3a69eb-eba1-4c95-9399-f1b9daf48733
fn get_lookup_table<T: PartialEq + Copy + std::fmt::Debug + Hash + Eq>(data: &[T]) -> Result<(Vec<u64>, Vec<T>), ()> {
    // It will always be more efficient to just defer to another encoding.
    if data.len() < 2 {
        return Err(());
    }

    profile!(T, "get_lookup_table");

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
        //indices.push(index.try_into().map_err(|_| ())?);
        indices.push(index as u64);
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

    Ok((indices, values))
}

impl<T: PartialEq + Copy + std::fmt::Debug + Hash + Eq, S: CompressorSet<T>> Compressor<T> for Dictionary<S> {
    // TODO: fast_size_for
    fn compress<O: EncodeOptions>(&self, data: &[T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        // Prevent panic on indexing first item.
        profile!("compress");

        let (indices, values) = get_lookup_table(data)?;

        stream.encode_with_id(|stream| compress(&values[..], stream, &self.sub_compressors));
        stream.encode_with_id(|stream| indices.flush(stream));

        Ok(ArrayTypeId::Dictionary)
    }

    fn fast_size_for<O: EncodeOptions>(&self, data: &[T], options: &O) -> Result<usize, ()> {
        profile!("fast_size_for");

        let (indices, values) = get_lookup_table(data)?;

        let from_values = fast_size_for(&values[..], &self.sub_compressors, options);
        let from_indices = Vec::<u64>::fast_size_for_all(&indices[..], options);

        let from_ids = 2;

        Ok(from_ids + from_indices + from_values)
    }
}
