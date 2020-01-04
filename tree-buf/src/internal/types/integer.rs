use crate::prelude::*;
use crate::encodings::varint::*;
use std::convert::{TryFrom, TryInto};

// FIXME: This is just for convenience right now, schema matching and custom encodings are needed instead.
impl<T: IntFromU64> BatchData for T {
    fn read_batch(bytes: &[u8]) -> Vec<Self> {
        read_all(bytes, |b, o| {
            let v = decode_prefix_varint(b, o);
            v.try_into().unwrap_or_else(|_| todo!()) // TODO: Error handling (which won't be needed when schema match occurs)
        })
    }
    fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
        for item in items {
            let v = (*item).into();
            encode_prefix_varint(v, bytes);
        }
    }
}

pub trait IntFromU64 : Into<u64> + TryFrom<u64> + Copy + Default {}
impl IntFromU64 for u8 {}
impl IntFromU64 for u16 {}
impl IntFromU64 for u32 {}
impl IntFromU64 for u64 {}


impl<T: IntFromU64> Primitive for T {
    fn id() -> PrimitiveId { PrimitiveId::Integer }
}