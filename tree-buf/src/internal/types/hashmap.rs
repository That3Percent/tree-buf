use crate::prelude::*;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::vec::IntoIter;

#[cfg(feature = "encode")]
impl<K: Encodable, V: Encodable, S: Default + BuildHasher> Encodable for HashMap<K, V, S> {
    type EncoderArray = HashMapArrayEncoder<K::EncoderArray, V::EncoderArray, S>;
    fn encode_root<O: EncodeOptions>(&self, stream: &mut EncoderStream<'_, O>) -> RootTypeId {
        profile!("encode_root");

        encode_usize(self.len(), stream);
        match self.len() {
            0 => {}
            1 => {
                for key in self.keys() {
                    stream.encode_with_id(|stream| key.encode_root(stream));
                }
                for value in self.values() {
                    stream.encode_with_id(|stream| value.encode_root(stream));
                }
            }
            _ => {
                let mut keys_encoder = K::EncoderArray::default();
                for key in self.keys() {
                    keys_encoder.buffer(key);
                }
                stream.encode_with_id(|stream| keys_encoder.flush(stream));

                let mut values_encoder = V::EncoderArray::default();
                for value in self.values() {
                    values_encoder.buffer(value);
                }
                stream.encode_with_id(|stream| values_encoder.flush(stream));
            }
        }

        RootTypeId::Map
    }
}

#[cfg(feature = "decode")]
impl<K: Decodable + Hash + Eq + Send, V: Decodable + Send, S: Default + BuildHasher> Decodable for HashMap<K, V, S>
where
    // Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
    DecodeError: From<<<K as Decodable>::DecoderArray as DecoderArray>::Error>,
    // Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
    DecodeError: From<<<V as Decodable>::DecoderArray as DecoderArray>::Error>,
{
    type DecoderArray = Option<HashMapArrayDecoder<K::DecoderArray, V::DecoderArray, S>>;
    fn decode(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("Decodable::decode");

        let mut v = Default::default(); // TODO: (Performance) Capacity
        match sticks {
            DynRootBranch::Map0 => Ok(v),
            DynRootBranch::Map1 { key, value } => {
                let (key, value) = parallel(move || K::decode(*key, options), move || V::decode(*value, options), options);
                v.insert(key?, value?);
                Ok(v)
            }
            DynRootBranch::Map { len, keys, values } => {
                let (keys, values) = parallel(|| K::DecoderArray::new(keys, options), || V::DecoderArray::new(values, options), options);
                let mut keys = keys?;
                let mut values = values?;
                for _ in 0..len {
                    if v.insert(keys.decode_next()?, values.decode_next()?).is_some() {
                        return Err(DecodeError::InvalidFormat);
                    }
                }
                Ok(v)
            }
            _ => Err(DecodeError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "encode")]
#[derive(Debug, Default)]
pub struct HashMapArrayEncoder<K, V, S> {
    len: <u64 as Encodable>::EncoderArray,
    items: Option<(K, V)>,
    _marker: Unowned<S>,
}

#[cfg(feature = "decode")]
pub struct HashMapArrayDecoder<K, V, S> {
    len: IntoIter<u64>,
    keys: K,
    values: V,
    _marker: Unowned<S>,
}

#[cfg(feature = "encode")]
impl<K: Encodable, V: Encodable, S: Default + BuildHasher> EncoderArray<HashMap<K, V, S>> for HashMapArrayEncoder<K::EncoderArray, V::EncoderArray, S> {
    fn buffer<'a, 'b: 'a>(&'a mut self, value: &'b HashMap<K, V, S>) {
        profile!("EncoderArray::buffer");
        self.len.buffer(&(value.len() as u64));
        let (keys, values) = self.items.get_or_insert_with(Default::default);
        for (key, value) in value.iter() {
            keys.buffer(key);
            values.buffer(value);
        }
    }
    fn flush<O: EncodeOptions>(self, stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        profile!("EncoderArray::flush");
        let Self { len, items, _marker } = self;
        if let Some((keys, values)) = items {
            stream.encode_with_id(|stream| len.flush(stream));
            stream.encode_with_id(|stream| keys.flush(stream));
            stream.encode_with_id(|stream| values.flush(stream));
        } else {
            stream.encode_with_id(|_| ArrayTypeId::Void);
        }
        ArrayTypeId::Map
    }
}

#[cfg(feature = "decode")]
impl<K: DecoderArray, V: DecoderArray, S: Default + BuildHasher> DecoderArray for Option<HashMapArrayDecoder<K, V, S>>
where
    K::Decode: Hash + Eq,
    // Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
    DecodeError: From<K::Error>,
    // Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
    DecodeError: From<V::Error>,
{
    type Decode = HashMap<K::Decode, V::Decode, S>;
    type Error = DecodeError;

    fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("DecoderArray::new");

        match sticks {
            DynArrayBranch::Map0 => Ok(None),
            DynArrayBranch::Map { len, keys, values } => {
                let (keys, (values, len)) = parallel(
                    || K::new(*keys, options),
                    || {
                        parallel(
                            || V::new(*values, options),
                            || <<u64 as Decodable>::DecoderArray as DecoderArray>::new(*len, options),
                            options,
                        )
                    },
                    options,
                );
                let keys = keys?;
                let values = values?;
                let len = len?;
                Ok(Some(HashMapArrayDecoder {
                    len,
                    keys,
                    values,
                    _marker: Unowned::new(),
                }))
            }
            _ => Err(DecodeError::SchemaMismatch),
        }
    }
    fn decode_next(&mut self) -> Result<Self::Decode, Self::Error> {
        if let Some(inner) = self {
            let len = inner.len.decode_next_infallible();
            let mut result = <Self::Decode as Default>::default(); // TODO: (Performance) capacity
            for _ in 0..len {
                let key = inner.keys.decode_next()?;
                let value = inner.values.decode_next()?;
                // TODO: decode_next was made infallable for performance reasons,
                // but duplicate keys would seem a reason to fail. Ideally this could
                // have a Result<T, !> and perform well in the future.
                if result.insert(key, value).is_some() {
                    return Err(DecodeError::InvalidFormat);
                };
            }
            Ok(result)
        } else {
            Ok(Default::default())
        }
    }
}
