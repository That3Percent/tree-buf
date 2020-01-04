use crate::prelude::*;
use crate::internal::encodings::packed_bool::*;

impl Primitive for bool {
    fn id() -> PrimitiveId {
        PrimitiveId::Boolean
    }
}

impl BatchData for bool {
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        decode_packed_bool(bytes)
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        encode_packed_bool(items, bytes);
    }
}