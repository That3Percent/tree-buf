use crate::prelude::*;

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
    fn from_dyn_branch(_branch: DynBranch) -> OneOrMany<Self> {
        unreachable!()
    }
}

#[derive(Copy, Clone)]
struct ArrayBranch;

impl StaticBranch for ArrayBranch {
    #[inline(always)]
    fn in_array_context() -> bool {
        true
    }
}

impl BatchData for Array {
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        Wrapper::write_batch(items, bytes)
    }
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        Wrapper::read_batch(bytes)
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> Self {
        unsafe { std::mem::transmute(usize::read_one(bytes, offset)) }
    }
    fn write_one(value: Self, bytes: &mut Vec<u8>) {
        unsafe { usize::write_one(std::mem::transmute(value), bytes) }
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
        
        self.values.flush(ArrayBranch, bytes, lens);
    }
}

impl<T: Reader> Reader for ArrayReader<T> {
    type Read = Vec<T::Read>;
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, _branch: ParentBranch) -> Self {
        match sticks {
            DynBranch::Array {len, values} => {
                let values = *values;
                Self {
                    len: PrimitiveBuffer::read_from(len),
                    values: Reader::new(values, ArrayBranch),
                }
            },
            _ => todo!("schema mismatch"), // Schema mismatch
        }
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