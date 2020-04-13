use crate::prelude::*;
use std::vec::IntoIter;

#[cfg(feature = "write")]
impl<'a, T: Writable<'a>> Writable<'a> for Vec<T> {
    type WriterArray = VecArrayWriter<'a, T::WriterArray>;
    fn write_root<'b: 'a>(&'b self, stream: &mut impl WriterStream) -> RootTypeId {
        profile!("write_root");
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
                write_usize(self.len(), stream);

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
    fn read(sticks: DynRootBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("Readable::read");
        match sticks {
            DynRootBranch::Array0 => Ok(Vec::new()),
            DynRootBranch::Array1(inner) => {
                let inner = T::read(*inner, options)?;
                Ok(vec![inner])
            }
            DynRootBranch::Array { len, values } => {
                let mut v = Vec::with_capacity(len);
                // TODO: Some of what the code is actually doing here is silly.
                // Actual ReaderArray's may be IntoIter, which moved out of a Vec
                // that we wanted in the first place. Specialization here would be nice.
                let mut reader = T::ReaderArray::new(values, options)?;
                for _ in 0..len {
                    v.push(reader.read_next());
                }
                Ok(v)
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

// TODO: usize
enum FixedOrVariableLength {
    Fixed(usize),
    Variable(IntoIter<u64>),
}

impl FixedOrVariableLength {
    fn next(&mut self) -> usize {
        match self {
            Self::Fixed(v) => *v,
            Self::Variable(i) => i.read_next() as usize,
        }
    }
}

#[cfg(feature = "read")]
pub struct VecArrayReader<T> {
    len: FixedOrVariableLength,
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
        profile!("flush");
        let Self { len, values } = self;
        if let Some(values) = values {
            if len.iter().all(|l| *l == len[0]) {
                write_usize(len[0] as usize, stream);
                stream.write_with_id(|stream| values.flush(stream));
                return ArrayTypeId::ArrayFixed;
            }
            // TODO: Consider an all-0 type // See also: 84d15459-35e4-4f04-896f-0f4ea9ce52a9
            stream.write_with_id(|stream| len.flush(stream));
            stream.write_with_id(|stream| values.flush(stream));
        } else {
            stream.write_with_id(|_| ArrayTypeId::Void);
        }

        ArrayTypeId::ArrayVar
    }
}

#[cfg(feature = "read")]
impl<T: ReaderArray> ReaderArray for Option<VecArrayReader<T>> {
    type Read = Vec<T::Read>;
    
    fn new(sticks: DynArrayBranch<'_>, options: &impl DecodeOptions) -> ReadResult<Self> {
        profile!("ReaderArray::new");

        match sticks {
            DynArrayBranch::Array0 => Ok(None),
            DynArrayBranch::Array { len, values } => {
                let (values, len) = parallel(|| T::new(*values, options), || <<u64 as Readable>::ReaderArray as ReaderArray>::new(*len, options), options);
                let values = values?;
                let len = FixedOrVariableLength::Variable(len?);
                Ok(Some(VecArrayReader { len, values }))
            }
            DynArrayBranch::ArrayFixed { len, values } => Ok(if len == 0 {
                None
            } else {
                let len = FixedOrVariableLength::Fixed(len);
                let values = T::new(*values, options)?;
                Some(VecArrayReader { len, values })
            }),
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next(&mut self) -> Self::Read {
        if let Some(inner) = self {
            let len = inner.len.next();
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                result.push(inner.values.read_next());
            }
            result
        } else {
            Vec::new()
        }
    }
}
