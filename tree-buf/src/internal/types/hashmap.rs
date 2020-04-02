use crate::prelude::*;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::vec::IntoIter;

#[cfg(feature = "write")]
impl<'a, K: Writable<'a>, V: Writable<'a>, S: Default + BuildHasher> Writable<'a> for HashMap<K, V, S> {
    type WriterArray = HashMapArrayWriter<'a, K::WriterArray, V::WriterArray, S>;
    fn write_root<'b: 'a>(&'b self, stream: &mut impl WriterStream) -> RootTypeId {
        write_usize(self.len(), stream);
        match self.len() {
            0 => {}
            1 => {
                for key in self.keys() {
                    stream.write_with_id(|stream| key.write_root(stream));
                }
                for value in self.values() {
                    stream.write_with_id(|stream| value.write_root(stream));
                }
            }
            _ => {
                let mut keys_writer = K::WriterArray::default();
                for key in self.keys() {
                    keys_writer.buffer(key);
                }
                stream.write_with_id(|stream| keys_writer.flush(stream));

                let mut values_writer = V::WriterArray::default();
                for value in self.values() {
                    values_writer.buffer(value);
                }
                stream.write_with_id(|stream| values_writer.flush(stream));
            }
        }

        RootTypeId::Map
    }
}

#[cfg(feature = "read")]
impl<K: Readable + Hash + Eq + Send, V: Readable + Send, S: Default + BuildHasher> Readable for HashMap<K, V, S> {
    type ReaderArray = Option<HashMapArrayReader<K::ReaderArray, V::ReaderArray, S>>;
    fn read(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        let mut v = Default::default(); // TODO: (Performance) Capacity
        match sticks {
            DynRootBranch::Map0 => Ok(v),
            DynRootBranch::Map1 { key, value } => {
                let (key, value) = parallel(move || K::read(*key, options), move || V::read(*value, options), options);
                v.insert(key?, value?);
                Ok(v)
            }
            DynRootBranch::Map { len, keys, values } => {
                let (keys, values) = parallel(|| K::ReaderArray::new(keys, options), || V::ReaderArray::new(values, options), options);
                let mut keys = keys?;
                let mut values = values?;
                for _ in 0..len {
                    // TODO: This should not be infallable.
                    v.insert(keys.read_next(), values.read_next());
                }
                Ok(v)
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "write")]
#[derive(Debug, Default)]
pub struct HashMapArrayWriter<'a, K, V, S> {
    len: <u64 as Writable<'a>>::WriterArray,
    items: Option<(K, V)>,
    _marker: Unowned<S>,
}

#[cfg(feature = "read")]
pub struct HashMapArrayReader<K, V, S> {
    len: IntoIter<u64>,
    keys: K,
    values: V,
    _marker: Unowned<S>,
}

#[cfg(feature = "write")]
impl<'a, K: WriterArray<'a>, V: WriterArray<'a>, S: Default + BuildHasher> WriterArray<'a> for HashMapArrayWriter<'a, K, V, S> {
    type Write = HashMap<K::Write, V::Write, S>;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.len.buffer(&(value.len() as u64));
        let (keys, values) = self.items.get_or_insert_with(Default::default);
        for (key, value) in value.iter() {
            keys.buffer(key);
            values.buffer(value);
        }
    }
    fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId {
        let Self { len, items, _marker } = self;
        if let Some((keys, values)) = items {
            stream.write_with_id(|stream| len.flush(stream));
            stream.write_with_id(|stream| keys.flush(stream));
            stream.write_with_id(|stream| values.flush(stream));
        } else {
            stream.write_with_id(|_| ArrayTypeId::Void);
        }
        ArrayTypeId::Map
    }
}

#[cfg(feature = "read")]
impl<K: ReaderArray, V: ReaderArray, S: Default + BuildHasher> ReaderArray for Option<HashMapArrayReader<K, V, S>>
where
    K::Read: Hash + Eq,
{
    type Read = HashMap<K::Read, V::Read, S>;
    fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        match sticks {
            DynArrayBranch::Map0 => Ok(None),
            DynArrayBranch::Map { len, keys, values } => {
                let (keys, (values, len)) = parallel(
                    || K::new(*keys, options),
                    || parallel(|| V::new(*values, options), || <<u64 as Readable>::ReaderArray as ReaderArray>::new(*len, options), options),
                    options,
                );
                let keys = keys?;
                let values = values?;
                let len = len?;
                Ok(Some(HashMapArrayReader {
                    len,
                    keys,
                    values,
                    _marker: Unowned::new(),
                }))
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next(&mut self) -> Self::Read {
        if let Some(inner) = self {
            let len = inner.len.read_next();
            let mut result = <Self::Read as Default>::default(); // TODO: (Performance) capacity
            for _ in 0..len {
                let key = inner.keys.read_next();
                let value = inner.values.read_next();
                // TODO: read_next was made infallable for performance reasons,
                // but duplicate keys would seem a reason to fail. Ideally this could
                // have a Result<T, !> and perform well in the future.
                result.insert(key, value);
            }
            result
        } else {
            Default::default()
        }
    }
}
