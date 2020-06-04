use crate::prelude::*;

#[cfg(feature = "encode")]
impl<T: Encodable> Encodable for Option<T> {
    type EncoderArray = NullableEncoder<T::EncoderArray>;
    fn encode_root<O: EncodeOptions>(&self, stream: &mut EncoderStream<'_, O>) -> RootTypeId {
        if let Some(value) = self {
            T::encode_root(value, stream)
        } else {
            RootTypeId::Void
        }
    }
}

#[cfg(feature = "decode")]
impl<T: Decodable> Decodable for Option<T> {
    type DecoderArray = Option<NullableDecoder<T::DecoderArray>>;
    fn decode(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("Decodable::decode");
        match sticks {
            DynRootBranch::Void => Ok(None),
            _ => Ok(Some(T::decode(sticks, options)?)),
        }
    }
}

#[cfg(feature = "encode")]
#[derive(Default)]
pub struct NullableEncoder<V> {
    opt: <bool as Encodable>::EncoderArray,
    value: Option<V>,
}

#[cfg(feature = "encode")]
impl<T: Encodable> EncoderArray<Option<T>> for NullableEncoder<T::EncoderArray> {
    fn buffer<'a, 'b: 'a>(&'a mut self, value: &'b Option<T>) {
        self.opt.buffer(&value.is_some());
        if let Some(value) = value {
            self.value.get_or_insert_with(T::EncoderArray::default).buffer(value);
        }
    }
    fn flush<O: EncodeOptions>(self, stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        let Self { opt, value } = self;
        if let Some(value) = value {
            stream.encode_with_id(|stream| opt.flush(stream));
            stream.encode_with_id(|stream| value.flush(stream));
            ArrayTypeId::Nullable
        } else {
            ArrayTypeId::Void
        }
    }
}

#[cfg(feature = "decode")]
pub struct NullableDecoder<T> {
    opts: <bool as Decodable>::DecoderArray,
    values: T,
}

#[cfg(feature = "decode")]
impl<T: DecoderArray> DecoderArray for Option<NullableDecoder<T>> {
    type Decode = Option<T::Decode>;
    type Error = T::Error;
    fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> DecodeResult<Self> {
        profile!("DecoderArray::new");

        match sticks {
            DynArrayBranch::Nullable { opt, values } => {
                let (opts, values) = parallel(|| <bool as Decodable>::DecoderArray::new(*opt, options), || T::new(*values, options), options);
                let opts = opts?;
                let values = values?;
                Ok(Some(NullableDecoder { opts, values }))
            }
            DynArrayBranch::Void => Ok(None),
            _ => Err(DecodeError::SchemaMismatch),
        }
    }
    fn decode_next(&mut self) -> Result<Self::Decode, Self::Error> {
        Ok(if let Some(inner) = self {
            if inner.opts.decode_next_infallible() {
                Some(inner.values.decode_next()?)
            } else {
                None
            }
        } else {
            None
        })
    }
}
