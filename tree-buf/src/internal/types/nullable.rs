#[cfg(feature = "read")]
use crate::internal::encodings::packed_bool::decode_packed_bool;
use crate::prelude::*;

#[cfg(feature = "write")]
impl<'a, T: Writable<'a>> Writable<'a> for Option<T> {
    type WriterArray = NullableWriter<'a, T::WriterArray>;
    fn write_root<'b: 'a>(&'b self, stream: &mut impl WriterStream) -> RootTypeId {
        if let Some(value) = self {
            T::write_root(value, stream)
        } else {
            RootTypeId::Void
        }
    }
}

#[cfg(feature = "read")]
impl<T: Readable> Readable for Option<T> {
    type ReaderArray = Option<NullableReader<T::ReaderArray>>;
    fn read(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("Readable::read");
        match sticks {
            DynRootBranch::Void => Ok(None),
            _ => Ok(Some(T::read(sticks, options)?)),
        }
    }
}

#[cfg(feature = "write")]
#[derive(Default)]
pub struct NullableWriter<'a, V> {
    opt: <bool as Writable<'a>>::WriterArray,
    value: Option<V>,
}

#[cfg(feature = "write")]
impl<'a, T: WriterArray<'a>> WriterArray<'a> for NullableWriter<'a, T> {
    type Write = Option<T::Write>;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.opt.buffer(&value.is_some());
        if let Some(value) = value {
            self.value.get_or_insert_with(T::default).buffer(value);
        }
    }
    fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId {
        if let Some(value) = self.value {
            let opts_id = self.opt.flush(stream);
            debug_assert_eq!(opts_id, ArrayTypeId::Boolean);
            stream.write_with_id(|stream| value.flush(stream));
            ArrayTypeId::Nullable
        } else {
            ArrayTypeId::Void
        }
    }
}

#[cfg(feature = "read")]
pub struct NullableReader<T> {
    opts: <bool as Readable>::ReaderArray,
    values: T,
}

#[cfg(feature = "read")]
impl<T: ReaderArray> ReaderArray for Option<NullableReader<T>> {
    type Read = Option<T::Read>;
    type Error = T::Error;
    fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("ReaderArray::new");

        match sticks {
            DynArrayBranch::Nullable { opt, values } => {
                let (opts, values) = parallel(|| decode_packed_bool(&opt).into_iter(), || T::new(*values, options), options);
                let values = values?;
                Ok(Some(NullableReader { opts, values }))
            }
            DynArrayBranch::Void => Ok(None),
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next(&mut self) -> Result<Self::Read, Self::Error> {
        Ok(if let Some(inner) = self {
            if inner.opts.read_next_infallible() {
                Some(inner.values.read_next()?)
            } else {
                None
            }
        } else {
            None
        })
    }
}
