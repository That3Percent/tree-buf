use crate::prelude::*;
use crate::encodings::varint::*;
use std::convert::{TryInto};

macro_rules! integer_impl {
    ($T:ty) => {
        // FIXME: This is just for convenience right now, schema matching and custom encodings are needed instead.
        impl BatchData for $T {
            fn read_batch(bytes: &[u8]) -> ReadResult<Vec<Self>> {
                read_all(bytes, |b, o| {
                    let v = decode_prefix_varint(b, o)?;
                    Ok(v.try_into().unwrap_or_else(|_| todo!())) // TODO: Error handling (which won't be needed when schema match occurs)
                })
            }
            fn write_batch(items: &[Self], bytes: &mut Vec<u8>) {
                for item in items {
                    let v = (*item).into();
                    encode_prefix_varint(v, bytes);
                }
            }
            fn read_one(bytes: &[u8], offset: &mut usize) -> ReadResult<Self> {
                let v = decode_prefix_varint(bytes, offset)?;
                Ok(v.try_into().unwrap_or_else(|_| todo!())) // TODO: Error handling (which won't be needed when schema match occurs)s
            }
        }

        impl Primitive for $T {
            fn id() -> PrimitiveId { PrimitiveId::Integer }
            fn from_dyn_branch(branch: DynBranch) -> ReadResult<OneOrMany<Self>> {
                let branch = match branch {
                    DynBranch::Integer(v) => {
                        match v {
                            OneOrMany::One(v) => OneOrMany::One(v.try_into().map_err(|_| ReadError::SchemaMismatch )?),
                            OneOrMany::Many(b) => OneOrMany::Many(b),
                        }
                    },
                    _ => Err(ReadError::SchemaMismatch)?,
                };
                Ok(branch)
            }
        }
    };
}


integer_impl!(u8);
integer_impl!(u16);
integer_impl!(u32);
integer_impl!(u64);


