use crate::prelude::*;
use crate::internal::encodings::packed_bool::*;

impl Primitive for bool {
    fn id() -> PrimitiveId {
        PrimitiveId::Boolean
    }
    fn from_dyn_branch(branch: DynBranch) -> OneOrMany<Self> {
        match branch {
            DynBranch::Boolean(v) => v,
            _ => todo!("schema mismatch"),
        }
    }
}

impl BatchData for bool {
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        decode_packed_bool(bytes)
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        encode_packed_bool(items, bytes);
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> Self {
        // TODO: Performance
        Self::read_batch(read_bytes(bytes, 1, offset))[0]
    }
}