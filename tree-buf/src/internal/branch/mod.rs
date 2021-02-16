use crate::prelude::*;

#[macro_use]
macro_rules! impl_type_id {
    ($T:ident, [$($name:ident: $i:expr,)+]) => {
        impl_type_id_inner!($T, [
            Void: 0,
            Tuple2: 100,
            Tuple3: 101,
            Tuple4: 102,
            Tuple5: 103,
            Tuple6: 104,
            Tuple7: 105,
            Tuple8: 106,
            TupleN: 107,
            Obj0: 108,
            Obj1: 109,
            Obj2: 110,
            Obj3: 111,
            Obj4: 112,
            Obj5: 113,
            Obj6: 114,
            Obj7: 115,
            Obj8: 116,
            ObjN: 117,
            $($name: $i,)+
        ]);
    };
}

macro_rules! impl_type_id_inner {
    ($T:ident, [$($name:ident: $i:expr,)+]) => {
        #[derive(PartialEq, Eq, Debug, Copy, Clone)]
        pub enum $T {
            $($name),+
        }

        impl TypeId for $T {
            fn void() -> Self where Self: Sized {
                $T::Void
            }
        }

        impl TryFrom<u8> for $T {
            type Error = DecodeError;
            fn try_from(value: u8) -> DecodeResult<Self> {
                Ok(match value {
                    $($i => $T::$name,)+
                    _ => return Err(DecodeError::InvalidFormat),
                })
            }
        }

        impl From<$T> for u8 {
            fn from(value: $T) -> Self {
                match value {
                    $($T::$name => $i,)+
                }
            }
        }

        impl $T {
            fn decode_next(bytes: &[u8], offset: &mut usize) -> DecodeResult<Self> {
                let next = bytes.get(*offset).ok_or_else(|| DecodeError::InvalidFormat)?;
                *offset += 1;
                (*next).try_into()
            }
        }
    }
}

mod root_branch;
pub use root_branch::*;

mod array_branch;
pub use array_branch::*;

pub type Ident<'a> = &'a str;

#[cfg(feature = "decode")]
#[inline]
pub fn decode_ident<'a>(bytes: &'a [u8], offset: &mut usize) -> DecodeResult<Ident<'a>> {
    decode_str(bytes, offset)
}

#[cfg(feature = "encode")]
#[inline]
pub fn encode_ident<O: EncodeOptions>(value: &str, stream: &mut EncoderStream<'_, O>) {
    encode_str(value, stream)
}

pub trait TypeId: Copy + Into<u8> + PartialEq + std::fmt::Debug {
    fn void() -> Self
    where
        Self: Sized;
}

// TODO: Finish or get rid of this mod
//mod visitor;

// TODO: There are conceptually 4 pieces which are intermingled in this code.
// 1: The actual 'object model' that TreeBuf uses. Eg:
//     Root values, array values,
// 2: Allowable conversions and downcasts inherant to the object model
// 3: The binary representation of the object model
// 4: The conversion of the object model to Rust data types, like Vec, u8, etc.
//
// It would be good to split these, but not at the cost of performance.
// Doing so would go a long way in guiding a port to another language.
//
// A purist architecture would do each step separately...
// 1: Convert Rust types into Object Model
// 2: Encode Object model
// - and in reverse for deserialize
//
// Note that the object model may be just defined in terms of eg: Number, where Number is the sum type of F64, u64, and i64 with downcasts.

#[cfg(feature = "decode")]
pub fn decode_root(bytes: &[u8]) -> DecodeResult<DynRootBranch<'_>> {
    profile_fn!(decode_root);
    if bytes.is_empty() {
        return Ok(DynRootBranch::Void);
    }
    let mut lens = bytes.len() - 1;
    let mut offset = 0;
    decode_next_root(bytes, &mut offset, &mut lens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::{TryFrom, TryInto};
    fn convert_all<T: TryFrom<u8> + Into<u8>>() {
        for i in 0..=255_u8 {
            let v: Result<T, _> = i.try_into();
            if let Ok(v) = v {
                debug_assert_eq!(i, v.into());
            }
        }
    }

    #[cfg(feature = "decode")]
    #[test]
    fn all_root_type_ids() {
        convert_all::<RootTypeId>();
    }

    #[cfg(feature = "decode")]
    #[test]
    fn all_array_type_ids() {
        convert_all::<ArrayTypeId>();
    }
}
