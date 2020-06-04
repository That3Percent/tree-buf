use crate::prelude::*;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Ignore;

#[cfg(feature = "encode")]
impl Encodable for Ignore {
    type EncoderArray = Ignore;
    fn encode_root<O: EncodeOptions>(&self, _stream: &mut EncoderStream<'_, O>) -> RootTypeId {
        RootTypeId::Void
    }
}

#[cfg(feature = "decode")]
impl Decodable for Ignore {
    type DecoderArray = Ignore;
    fn decode(_sticks: DynRootBranch<'_>, _options: &impl DecodeOptions) -> DecodeResult<Self> {
        Ok(Self)
    }
}

#[cfg(feature = "encode")]
impl EncoderArray<Ignore> for Ignore {
    fn buffer<'a, 'b: 'a>(&'a mut self, _value: &'b Ignore) {}
    fn flush<O: EncodeOptions>(self, _stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
        ArrayTypeId::Void
    }
}

#[cfg(feature = "decode")]
impl InfallibleDecoderArray for Ignore {
    type Decode = Ignore;
    fn new_infallible(_sticks: DynArrayBranch<'_>, _options: &impl DecodeOptions) -> DecodeResult<Self> {
        Ok(Ignore)
    }
    fn decode_next_infallible(&mut self) -> Self::Decode {
        Ignore
    }
}
