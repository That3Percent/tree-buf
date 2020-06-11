use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "encode")]
impl<T: Encodable> Encodable for Vec<T> {
    type EncoderArray = VecArrayEncoder<T::EncoderArray>;
    fn encode_root<O: EncodeOptions>(&self, stream: &mut EncoderStream<'_, O>) -> RootTypeId {
        profile!("Array encode_root");
        match self.len() {
            0 => RootTypeId::Array0,
            1 => {
                stream.encode_with_id(|stream| (&self[0]).encode_root(stream));
                RootTypeId::Array1
            }
            _ => {
                // TODO: Seems kind of redundant to have both the array len,
                // and the bytes len. Though, it's not for obvious reasons.
                // Maybe sometimes we can infer from context. Eg: bool always
                // requires the same number of bits per item
                encode_usize(self.len(), stream);

                stream.encode_with_id(|stream| T::EncoderArray::encode_all(&self[..], stream));

                RootTypeId::ArrayN
            }
        }
    }
}

#[cfg(feature = "decode")]
impl<T: Decodable> Decodable for Vec<T>
// Overly verbose because of `?` requiring `From` See also ec4fa3ba-def5-44eb-9065-e80b59530af6
where
    DecodeError: From<<<T as Decodable>::DecoderArray as DecoderArray>::Error>,
{
    type DecoderArray = Option<VecArrayDecoder<T::DecoderArray>>;
    fn decode(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("Array Decodable::decode");
        match sticks {
            DynRootBranch::Array0 => Ok(Vec::new()),
            DynRootBranch::Array1(inner) => {
                let inner = T::decode(*inner, options)?;
                Ok(vec![inner])
            }
            DynRootBranch::Array { len, values } => {
                let mut v = Vec::with_capacity(len);
                // TODO: Some of what the code is actually doing here is silly.
                // Actual DecoderArray's may be IntoIter, which moved out of a Vec
                // that we wanted in the first place. Specialization here would be nice.
                let mut decoder = T::DecoderArray::new(values, options)?;
                for _ in 0..len {
                    v.push(decoder.decode_next()?);
                }
                Ok(v)
            }
            _ => Err(DecodeError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "encode")]
#[derive(Debug, Default)]
pub struct VecArrayEncoder<T> {
    // TODO: usize
    len: <u64 as Encodable>::EncoderArray,
    // Using Option here enables recursion when necessary.
    values: Option<T>,
}

// TODO: usize
enum FixedOrVariableLength {
    Fixed(usize),
    Variable(IntoIter<u64>),
}

impl FixedOrVariableLength {
    fn next(&mut self) -> usize {
        match self {
            Self::Fixed(v) => *v,
            Self::Variable(i) => i.decode_next_infallible() as usize,
        }
    }
}

#[cfg(feature = "decode")]
pub struct VecArrayDecoder<T> {
    len: FixedOrVariableLength,
    values: T,
}

#[cfg(feature = "encode")]
impl<T: Encodable> EncoderArray<Vec<T>> for VecArrayEncoder<T::EncoderArray> {
    fn buffer_one<'a, 'b: 'a>(&'a mut self, value: &'b Vec<T>) {
        self.len.buffer_one(&(value.len() as u64));
        let values = self.values.get_or_insert_with(Default::default);
        values.buffer_many(&value[..]);
    }
    fn flush<O: EncodeOptions>(self, stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        profile!("Array flush");
        let Self { len, values } = self;
        if let Some(values) = values {
            if len.iter().all(|l| *l == len[0]) {
                encode_usize(len[0] as usize, stream);
                stream.encode_with_id(|stream| values.flush(stream));
                return ArrayTypeId::ArrayFixed;
            }
            // TODO: Consider an all-0 type // See also: 84d15459-35e4-4f04-896f-0f4ea9ce52a9
            stream.encode_with_id(|stream| len.flush(stream));
            stream.encode_with_id(|stream| values.flush(stream));
        } else {
            stream.encode_with_id(|_| ArrayTypeId::Void);
        }

        ArrayTypeId::ArrayVar
    }
}

#[cfg(feature = "decode")]
impl<T: DecoderArray> DecoderArray for Option<VecArrayDecoder<T>> {
    type Decode = Vec<T::Decode>;
    type Error = T::Error;

    fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("Array DecoderArray::new");

        match sticks {
            DynArrayBranch::Array0 => Ok(None),
            DynArrayBranch::Array { len, values } => {
                let (values, len) = parallel(
                    || T::new(*values, options),
                    || <<u64 as Decodable>::DecoderArray as DecoderArray>::new(*len, options),
                    options,
                );
                let values = values?;
                let len = FixedOrVariableLength::Variable(len?);
                Ok(Some(VecArrayDecoder { len, values }))
            }
            DynArrayBranch::ArrayFixed { len, values } => Ok(if len == 0 {
                None
            } else {
                let len = FixedOrVariableLength::Fixed(len);
                let values = T::new(*values, options)?;
                Some(VecArrayDecoder { len, values })
            }),
            _ => Err(DecodeError::SchemaMismatch),
        }
    }
    fn decode_next(&mut self) -> Result<Self::Decode, Self::Error> {
        if let Some(inner) = self {
            let len = inner.len.next();
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                result.push(inner.values.decode_next()?);
            }
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }
}
