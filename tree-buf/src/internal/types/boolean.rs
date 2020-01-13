use crate::prelude::*;
use crate::internal::encodings::packed_bool::*;

impl Primitive for bool {
    fn id() -> PrimitiveId {
        PrimitiveId::Boolean
    }
    fn from_dyn_branch(branch: DynBranch) -> ReadResult<OneOrMany<Self>> {
        match branch {
            DynBranch::Boolean(v) => Ok(v),
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

impl_primitive_reader_writer!(bool);

impl BatchData for bool {
    fn read_batch(bytes: &[u8]) -> ReadResult<Vec<Self>> {
        Ok(decode_packed_bool(bytes))
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        encode_packed_bool(items, bytes);
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> ReadResult<Self> {
        // TODO: Performance
        Ok(Self::read_batch(read_bytes(bytes, 1, offset)?)?[0])
    }
}