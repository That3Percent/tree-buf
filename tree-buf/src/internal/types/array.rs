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
    fn from_dyn_branch(_branch: DynBranch) -> ReadResult<OneOrMany<Self>> {
        unreachable!()
    }
}

impl_primitive_reader_writer!(Array);

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
    fn read_batch(bytes: &[u8]) -> ReadResult<Vec<Self>> {
        Wrapper::read_batch(bytes)
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> ReadResult<Self> {
        unsafe { std::mem::transmute(usize::read_one(bytes, offset)) }
    }
    fn write_one(value: Self, bytes: &mut Vec<u8>) {
        unsafe { usize::write_one(std::mem::transmute(value), bytes) }
    }
}

// TODO: Have Void for recursion support
#[derive(Debug, Default)]
pub struct ArrayWriter<'a, T> {
    len: <Array as Writable<'a>>::Writer,
    values: T,
}

pub struct ArrayReader<T> {
    len: <Array as Readable>::Reader,
    values: T,
}

impl<'a, T: Writer<'a>> Writer<'a> for ArrayWriter<'a, T> {
    type Write = Vec<T::Write>;
    fn write<'b : 'a>(&mut self, value: &'b Self::Write) {
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
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, _branch: ParentBranch) -> ReadResult<Self> {
        match sticks {
            DynBranch::Array {len, values} => {
                let values = *values;
                Ok(Self {
                    len: PrimitiveReader::read_from(len)?,
                    values: Reader::new(values, ArrayBranch)?,
                })
            },
            _ => Err(ReadError::SchemaMismatch), // Schema mismatch
        }
    }
    fn read(&mut self) -> ReadResult<Self::Read> {
        let len = self.len.read()?.0;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(self.values.read()?);
        }
        Ok(result)
    }
}


impl<'a, T: Writable<'a>> Writable<'a> for Vec<T> {
    type Writer = ArrayWriter<'a, T::Writer>;
}

impl<T: Readable> Readable for Vec<T> {
    type Reader = ArrayReader<T::Reader>;
}