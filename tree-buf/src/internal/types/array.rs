use crate::encodings::varint::encode_prefix_varint;
use crate::prelude::*;
use std::vec::IntoIter;

impl<'a, T: Writable<'a>> Writable<'a> for Vec<T> {
    type WriterArray = VecArrayWriter<'a, T::WriterArray>;
    fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> RootTypeId {
        match value.len() {
            0 => RootTypeId::Array0,
            1 => {
                let type_index = bytes.len();
                bytes.push(0);
                let type_id = T::write_root(&value[0], bytes, lens);
                bytes[type_index] = type_id.into();
                RootTypeId::Array1
            }
            _ => {
                // TODO: Seems kind of redundant to have both the array len,
                // and the bytes len. Though, it's not for obvious reasons.
                // Maybe sometimes we can infer from context. Eg: bool
                encode_prefix_varint(value.len() as u64, bytes);

                let type_index = bytes.len();
                bytes.push(0);

                // TODO: When there are types that are already
                // primitive (eg: Vec<f64>) it doesn't make sense
                // to buffer at this level. Specialization may
                // be useful here.
                //
                // TODO: See below, and just call buffer on the vec
                // and flush it!
                let mut writer = T::WriterArray::default();
                for item in value {
                    writer.buffer(item);
                }
                let type_id = writer.flush(bytes, lens);
                bytes[type_index] = type_id.into();
                RootTypeId::ArrayN
            }
        }
    }
}

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
                // that we wanted in the first place. An overload here would be nice.
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

#[derive(Debug, Default)]
pub struct VecArrayWriter<'a, T> {
    // TODO: usize
    len: <u64 as Writable<'a>>::WriterArray,
    // Using Option here enables recursion when necessary.
    values: Option<T>,
}

pub struct VecArrayReader<T> {
    len: IntoIter<usize>,
    values: T,
}

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
    fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId {
        let Self { len, values } = self;
        if let Some(values) = values {
            let type_index = bytes.len();
            bytes.push(0);
            let type_id = values.flush(bytes, lens);
            debug_assert_ne!(type_id, ArrayTypeId::Void); // If this is Void, it's ambigous
            bytes[type_index] = type_id.into();
            // Array knows it's own type id, so drop it.
            let len_type_id = len.flush(bytes, lens);
            debug_assert_eq!(len_type_id, ArrayTypeId::IntPrefixVar);
        } else {
            bytes.push(ArrayTypeId::Void.into())
        }

        ArrayTypeId::ArrayVar
    }
}

impl<T: ReaderArray> ReaderArray for Option<VecArrayReader<T>> {
    type Read = Vec<T::Read>;
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynArrayBranch::Array0 => Ok(None),
            DynArrayBranch::Array { len, values } => {
                let values = T::new(*values)?;
                let len = read_all(len, read_usize)?.into_iter();
                Ok(Some(VecArrayReader { len, values }))
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next(&mut self) -> ReadResult<Self::Read> {
        if let Some(inner) = self {
            let len = inner.len.read_next()?;
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                result.push(inner.values.read_next()?);
            }
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }
}
