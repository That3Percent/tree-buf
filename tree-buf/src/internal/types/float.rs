use crate::prelude::*;
use std::convert::TryInto;

impl BatchData for f64 {
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        read_all(bytes, Self::read_one)
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        // TODO: Compression
        for item in items {
            Self::write_one(*item, bytes);
        }
    }
    fn write_one(item: Self, bytes: &mut Vec<u8>) {
        let b = item.to_le_bytes();
        bytes.extend_from_slice(&b);
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> Self {
        let bytes = read_bytes(bytes, 8, offset);
        f64::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl Primitive for f64 {
    fn id() -> PrimitiveId { PrimitiveId::Float }
    fn from_dyn_branch(branch: DynBranch) -> OneOrMany<Self> {
        match branch {
            DynBranch::Float(v) => v,
            _ => todo!("schema mismatch"),
        }
    }
}