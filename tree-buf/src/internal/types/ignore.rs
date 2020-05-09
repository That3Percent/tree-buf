use crate::prelude::*;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Ignore;

#[cfg(feature = "write")]
impl Writable for Ignore {
    type WriterArray = Ignore;
    fn write_root<O: EncodeOptions>(&self, _stream: &mut WriterStream<'_, O>) -> RootTypeId {
        RootTypeId::Void
    }
}

#[cfg(feature = "read")]
impl Readable for Ignore {
    type ReaderArray = Ignore;
    fn read(_sticks: DynRootBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
        Ok(Self)
    }
}

#[cfg(feature = "write")]
impl WriterArray<Ignore> for Ignore {
    fn buffer<'a, 'b: 'a>(&'a mut self, _value: &'b Ignore) {}
    fn flush<O: EncodeOptions>(self, _stream: &mut WriterStream<'_, O>) -> ArrayTypeId {
        ArrayTypeId::Void
    }
}

#[cfg(feature = "read")]
impl InfallibleReaderArray for Ignore {
    type Read = Ignore;
    fn new_infallible(_sticks: DynArrayBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
        Ok(Ignore)
    }
    fn read_next_infallible(&mut self) -> Self::Read {
        Ignore
    }
}
