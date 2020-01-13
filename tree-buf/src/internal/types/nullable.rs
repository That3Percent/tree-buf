use crate::prelude::*;

// The Default derive enabled DefaultOnMissing to have None
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Nullable(bool);

unsafe impl Wrapper for Nullable {
    type Inner = bool;
}

impl BatchData for Nullable {
    // TODO: This is boilerplate, want blanket implementation to cover this and Array
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        Wrapper::write_batch(items, bytes)
    }
    fn read_batch(bytes: &[u8]) -> ReadResult<Vec<Self>> {
        Wrapper::read_batch(bytes)
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> ReadResult<Self> {
        Ok(unsafe { std::mem::transmute(bool::read_one(bytes, offset)?) })
    }
    fn write_one(value: Self, bytes: &mut Vec<u8>) {
        unsafe { bool::write_one(std::mem::transmute(value), bytes) }
    }
}

impl Primitive for Nullable {
    fn id() -> PrimitiveId {
        PrimitiveId::Nullable
    }
    fn from_dyn_branch(_branch: DynBranch) -> ReadResult<OneOrMany<Self>> {
        unreachable!();
    }
}


//#[derive(Debug)]
pub struct NullableWriter<'a, V> {
    opt: <Nullable as Writable<'a>>::Writer,
    value: Option<V>,
}

pub struct NullableReader<V> {
    opt: <Nullable as Readable>::Reader,
    value: V,
}

impl<'a, V: Writer<'a>> Writer<'a> for NullableWriter<'a, V> {
    type Write = Option<V::Write>;
    fn new() -> Self {
        Self {
            opt: Writer::new(),
            value: None,
        }
    }
    fn write<'b : 'a>(&mut self, value: &'b Self::Write) {
        self.opt.write(&Nullable(value.is_some()));
        if let Some(value) = value {
            self.value
                .get_or_insert_with(|| V::new())
                .write(value);
        }
    }
    fn flush<B: StaticBranch>(self, branch: B, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
        if let Some(value) = self.value {
            self.opt.flush(branch, bytes, lens);
            value.flush(branch, bytes, lens);
        } else {
            // TODO: Since Option can react to no branch, we can
            // save some bytes here by returning a value which (possibly) rolls back the outer value.
            // Eg: in struct this would simply remove the field name string that was written,
            // tuple is a bit more nuanced as it must write the void if there is a non-void that follows.
            PrimitiveId::Void.write(bytes);
        }
        
    }
}

impl<V: Reader> Reader for Option<NullableReader<V>> {
    type Read = Option<V::Read>;
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch, branch: ParentBranch) -> ReadResult<Self> {
        match sticks {
            DynBranch::Nullable { opt, values } => {
                let values = *values;
                Ok(Some(NullableReader {
                    opt: PrimitiveReader::read_from(opt)?,
                    value: Reader::new(values, branch)?,
                }))
            },
            DynBranch::Void => Ok(None),
            _ => Err(ReadError::SchemaMismatch)?,
        }
    }
    fn read(&mut self) -> ReadResult<Self::Read> {
        let value = if let Some(inner) = self {
            if inner.opt.read()?.0 {
                Some(inner.value.read()?)
            } else {
                None
            }
        } else {
            None
        };
        Ok(value)
    }
}

impl<'a, T: Writable<'a>> Writable<'a> for Option<T> {
    type Writer = NullableWriter<'a, T::Writer>;
}

impl<T: Readable> Readable for Option<T> {
    type Reader = Option<NullableReader<T::Reader>>;
}
