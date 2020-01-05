use crate::prelude::*;
use std::marker::PhantomData;

// The Default derive enables DefaultOnMissing to have an empty array
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Array(usize);

unsafe impl Wrapper for Array {
    type Inner = usize;
}

impl Primitive for Array {
    fn id() -> PrimitiveId {
        PrimitiveId::Array
    }
}

struct StaticArrayBranch<T>(PhantomData<*const T>);

impl<'a, T: StaticBranch> StaticArrayBranch<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'a, T: StaticBranch> StaticBranch for StaticArrayBranch<T> {
    #[inline(always)]
    fn children_in_array_context() -> bool {
        true
    }
    #[inline(always)]
    fn self_in_array_context() -> bool {
        T::children_in_array_context()
    }
}

impl BatchData for Array {
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        Wrapper::write_batch(items, bytes)
    }
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        Wrapper::read_batch(bytes)
    }
}


#[derive(Debug)]
pub struct ArrayWriter<T> {
    len: PrimitiveBuffer<Array>,
    values: T,
}

pub struct ArrayReader<T> {
    len: PrimitiveBuffer<Array>,
    values: T,
}

impl<T: Writer> Writer for ArrayWriter<T> {
    type Write = Vec<T::Write>;
    fn new() -> Self {
        Self {
            len: PrimitiveBuffer::new(),
            values: T::new(),
        }
    }
    fn write(&mut self, value: &Self::Write) {
        self.len.write(&Array(value.len()));
        for item in value.iter() {
            self.values.write(item);
        }
    }
    fn flush<ParentBranch: StaticBranch>(self, branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
        self.len.flush(branch, bytes, lens);
        self.values.flush(StaticArrayBranch::<ParentBranch>::new(), bytes, lens);
    }
}

impl<T: Reader> Reader for ArrayReader<T> {
    type Read = Vec<T::Read>;
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, branch: ParentBranch) -> Self {
        match sticks {
            DynBranch::Array {len, values} => {
                todo!()
            },
            _ => todo!(), // Schema mismatch
        }
        /*
        Self {
            len: Reader::new(sticks, branch),
            values: Reader::new(sticks, StaticArrayBranch::<ParentBranch>::new())
        }
        */
    }
    fn read(&mut self) -> Self::Read {
        let len = self.len.read().0;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(self.values.read());
        }
        result
    }
}


impl<T: Writable> Writable for Vec<T> {
    type Writer = ArrayWriter<T::Writer>;
}

impl<T: Readable> Readable for Vec<T> {
    type Reader = ArrayReader<T::Reader>;
}