#[cfg(feature = "write")]
use crate::internal::encodings::varint::encode_prefix_varint;
use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "write")]
impl<'a, T: Writable<'a>> Writable<'a> for Vec<T> {
    type WriterArray = VecArrayWriter<'a, T::WriterArray>;
    fn write_root<'b: 'a>(&'b self, stream: &mut impl WriterStream) -> RootTypeId {
        match self.len() {
            0 => RootTypeId::Array0,
            1 => {
                stream.write_with_id(|stream| (&self[0]).write_root(stream));
                RootTypeId::Array1
            }
            _ => {
                // TODO: Seems kind of redundant to have both the array len,
                // and the bytes len. Though, it's not for obvious reasons.
                // Maybe sometimes we can infer from context. Eg: bool always
                // requires the same number of bits per item
                encode_prefix_varint(self.len() as u64, stream.bytes());

                // TODO: When there are types that are already
                // primitive (eg: Vec<f64>) it doesn't make sense
                // to buffer at this level. Specialization may
                // be useful here.
                //
                // TODO: See below, and just call buffer on the vec
                // and flush it!
                let mut writer = T::WriterArray::default();
                for item in self {
                    writer.buffer(item);
                }

                stream.write_with_id(|stream| writer.flush(stream));

                RootTypeId::ArrayN
            }
        }
    }
}

#[cfg(feature = "read")]
impl<T: Readable> Readable for Vec<T> {
    type ReaderArray = Option<VecArrayReader<T::ReaderArray>>;
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynRootBranch::Array0 => Ok(Vec::new()),
            DynRootBranch::Array1(inner) => {
                let inner = T::read(*inner)?;
                Ok(vec![inner])
            }
            DynRootBranch::Array { len, values } => {
                let mut v = Vec::with_capacity(len);
                // TODO: Some of what the code is actually doing here is silly.
                // Actual ReaderArray's may be IntoIter, which moved out of a Vec
                // that we wanted in the first place. Specialization here would be nice.
                let mut reader = T::ReaderArray::new(values)?;
                for _ in 0..len {
                    v.push(reader.read_next()?);
                }
                Ok(v)
                // We could try to verify the file here by trying
                // to read one more value, but that doesn't work well
                // for block based readers (eg: bool) because they round up
                // the number of values read to pad the block
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "write")]
#[derive(Debug, Default)]
pub struct VecArrayWriter<'a, T> {
    // TODO: usize
    len: <u64 as Writable<'a>>::WriterArray,
    // Using Option here enables recursion when necessary.
    values: Option<T>,
}

#[cfg(feature = "read")]
pub struct VecArrayReader<T> {
    // TODO: usize
    len: IntoIter<u64>,
    values: T,
}

#[cfg(feature = "write")]
impl<'a, T: WriterArray<'a>> WriterArray<'a> for VecArrayWriter<'a, T> {
    type Write = Vec<T::Write>;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        // TODO: Consider whether buffer should actually just
        // do something non-flat, (like literally push the Vec<T> into another Vec<T>)
        // and the flattening could happen later at flush time. This may reduce memory cost.
        // Careful though.
        // I feel though that somehow this outer buffer type
        // could fix the specialization problem above for single-vec
        // values.
        self.len.buffer(&(value.len() as u64));
        let values = self.values.get_or_insert_with(Default::default);
        for item in value {
            values.buffer(item);
        }
    }
    fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId {
        let Self { len, values } = self;
        if let Some(values) = values {
            let type_id = stream.write_with_id(|stream| values.flush(stream));
            debug_assert_ne!(type_id, ArrayTypeId::Void); // If this is Void, it's ambigous

            // TODO: Maybe combine the permutations of valid int compressors here with ArrayVar to save a byte
            // here every time. Eg: ArrayVarSimple16 ArrayVarIntPrefixVar
            stream.write_with_id(|stream| len.flush(stream));
        } else {
            stream.write_with_id(|_| ArrayTypeId::Void);
        }

        ArrayTypeId::ArrayVar
    }
}

#[cfg(feature = "read")]
impl<T: ReaderArray> ReaderArray for Option<VecArrayReader<T>> {
    type Read = Vec<T::Read>;
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynArrayBranch::Array0 => Ok(None),
            DynArrayBranch::Array { len, values } => {
                let values = T::new(*values)?;
                let len = <<u64 as Readable>::ReaderArray as ReaderArray>::new(*len)?;
                Ok(Some(VecArrayReader { len, values }))
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next(&mut self) -> ReadResult<Self::Read> {
        if let Some(inner) = self {
            let len = inner.len.read_next()?;
            let mut result = Vec::with_capacity(len as usize);
            for _ in 0..len {
                result.push(inner.values.read_next()?);
            }
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }
}
