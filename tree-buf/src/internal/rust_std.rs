use crate::prelude::*;

#[derive(Default)]
pub struct BoxWriter<T> {
    writer: T,
}

pub struct BoxReader<T> {
    reader: T,
}

impl<'a, T: Writer<'a>> Writer<'a> for BoxWriter<T> {
    type Write=Box<T::Write>;
    fn write<'b : 'a>(&mut self, value: &'b Self::Write) {
        self.writer.write(value);
    }
    fn flush<ParentBranch: StaticBranch>(self, branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
        let Self { writer } = self;
        writer.flush(branch, bytes, lens);
    }
}

impl<T: Reader> Reader for BoxReader<T> {
    type Read=Box<T::Read>;
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, branch: ParentBranch) -> ReadResult<Self> {
        Ok(BoxReader { reader: T::new(sticks, branch)? })
    }
    fn read(&mut self) -> ReadResult<Self::Read> {
        Ok(Box::new(self.reader.read()?))
    }
}

impl<'a, T: Writable<'a>> Writable<'a> for Box<T> {
    type Writer=BoxWriter<T::Writer>;
}

impl<'a, T: Readable> Readable for Box<T> {
    type Reader=BoxReader<T::Reader>;
}