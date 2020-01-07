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
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        Wrapper::read_batch(bytes)
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> Self {
        unsafe { std::mem::transmute(bool::read_one(bytes, offset)) }
    }
    fn write_one(value: Self, bytes: &mut Vec<u8>) {
        unsafe { bool::write_one(std::mem::transmute(value), bytes) }
    }
}

impl Primitive for Nullable {
    fn id() -> PrimitiveId {
        PrimitiveId::Nullable
    }
    fn from_dyn_branch(_branch: DynBranch) -> OneOrMany<Self> {
        unreachable!();
    }
}


#[derive(Debug)]
pub struct NullableWriter<V> {
    opt: PrimitiveBuffer<Nullable>,
    value: V,
}

pub struct NullableReader<V> {
    opt: PrimitiveBuffer<Nullable>,
    value: V,
}

impl<V: Writer> Writer for NullableWriter<V> {
    type Write = Option<V::Write>;
    fn new() -> Self {
        Self {
            opt: PrimitiveBuffer::new(),
            value: V::new(),
        }
    }
    fn write(&mut self, value: &Self::Write) {
        self.opt.write(&Nullable(value.is_some()));
        if let Some(value) = value {
            self.value.write(value);
        }
    }
    fn flush<B: StaticBranch>(self, branch: B, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
        self.opt.flush(branch, bytes, lens);
        self.value.flush(branch, bytes, lens);
    }
}

impl<V: Reader> Reader for NullableReader<V> {
    type Read = Option<V::Read>;
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch, branch: ParentBranch) -> Self {
        match sticks {
            DynBranch::Nullable { opt, values } => {
                let values = *values;
                Self {
                    opt: PrimitiveBuffer::read_from(opt),
                    value: Reader::new(values, branch),
                }
            },
            _ => todo!("schema mismatch"),
        }
    }
    fn read(&mut self) -> Self::Read {
        if self.opt.read().0 {
            Some(self.value.read())
        } else {
            None
        }
    }
}

impl<T: Writable> Writable for Option<T> {
    type Writer = NullableWriter<T::Writer>;
}

impl<T: Readable> Readable for Option<T> {
    type Reader = NullableReader<T::Reader>;
}
