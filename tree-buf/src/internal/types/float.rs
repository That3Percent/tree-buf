use crate::prelude::*;
use std::convert::TryInto;
use std::mem::size_of;

impl BatchData for f64 {
    fn read_batch(bytes: &[u8]) -> ReadResult<Vec<Self>> {
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
    fn read_one(bytes: &[u8], offset: &mut usize) -> ReadResult<Self> {
        let bytes = read_bytes(bytes, size_of::<f64>(), offset)?;
        // This unwrap is ok, because we just read exactly size_of::<f64> bytes on the line above.
        Ok(f64::from_le_bytes(bytes.try_into().unwrap()))
    }
}

impl Primitive for f64 {
    fn id() -> PrimitiveId { PrimitiveId::Float }
    fn from_dyn_branch(branch: DynBranch) -> ReadResult<OneOrMany<Self>> {
        match branch {
            DynBranch::Float(v) => Ok(v),
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

impl_primitive_reader_writer!(f64);