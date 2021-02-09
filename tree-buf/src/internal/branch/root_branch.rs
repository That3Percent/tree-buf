use crate::internal::encodings::varint::decode_prefix_varint;
use crate::prelude::*;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

// TODO: Before there was an idea that Void should be lowered removed from eg: Object fields and the like. But,
// it's less clear when considering this is data and not messages that doing so is useful.

// TODO: The idea for int is to always encode up to 64 bit values,
// but for any data store the min value and offset first, then use
// that to select an optimal encoding. When deserializing, the min and
// offset can be used to find if the data type required by the schema
// matches.

// TODO: GUID. Consider having some sort of "semantic" flag to denote other kinds of values (like f64/u64 -> timestamp/date, 128 bit [u8] -> GUID)
// TODO: Other kinds of self-description may also be interesting, since this is for data self-description is higher value
// TODO: Bytes/Blog = [u8] compressed (eg: gzip), uncompressed

#[derive(Debug)]
pub enum DynRootBranch<'a> {
    Object {
        fields: HashMap<Ident<'a>, DynRootBranch<'a>>,
    },
    Tuple {
        fields: Vec<DynRootBranch<'a>>,
    },
    Enum {
        discriminant: Ident<'a>,
        value: Box<DynRootBranch<'a>>,
    },
    Array0,                         // Separate from Array because it infers Void
    Array1(Box<DynRootBranch<'a>>), // Separate from Array because it does not need to enter an array context
    Array {
        len: usize,
        values: DynArrayBranch<'a>,
    },
    Integer(RootInteger),
    Boolean(bool),
    Float(RootFloat),
    Void,
    String(&'a str),
    Map0,
    Map1 {
        key: Box<DynRootBranch<'a>>,
        value: Box<DynRootBranch<'a>>,
    },
    Map {
        len: usize,
        keys: DynArrayBranch<'a>,
        values: DynArrayBranch<'a>,
    },
}

pub fn decode_next_root<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> DecodeResult<DynRootBranch<'a>> {
    let id = RootTypeId::decode_next(bytes, offset)?;

    // See also e25db64d-8424-46b9-bdc1-cdb618807513
    fn decode_tuple<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> DecodeResult<DynRootBranch<'a>> {
        let mut fields = Vec::with_capacity(num_fields);
        for _ in 0..num_fields {
            let child = decode_next_root(bytes, offset, lens)?;
            fields.push(child);
        }
        Ok(DynRootBranch::Tuple { fields })
    }

    fn decode_array<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> DecodeResult<DynRootBranch<'a>> {
        let values = decode_next_array(bytes, offset, lens)?;
        Ok(DynRootBranch::Array { len, values })
    }

    // See also 47a1482f-5ce3-4b78-b356-30c66dc60cda
    fn decode_obj<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> DecodeResult<DynRootBranch<'a>> {
        let mut fields = HashMap::with_capacity(num_fields);
        for _ in 0..num_fields {
            let name = crate::internal::decode_ident(bytes, offset)?;
            let child = decode_next_root(bytes, offset, lens)?;
            fields.insert(name, child);
        }
        Ok(DynRootBranch::Object { fields })
    }

    fn decode_str<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize) -> DecodeResult<DynRootBranch<'a>> {
        let bytes = decode_bytes(len, bytes, offset)?;
        let s = std::str::from_utf8(bytes)?;
        Ok(DynRootBranch::String(s))
    }

    use RootTypeId::*;
    let branch = match id {
        Void => DynRootBranch::Void,

        Tuple2 => decode_tuple(2, bytes, offset, lens)?,
        Tuple3 => decode_tuple(3, bytes, offset, lens)?,
        Tuple4 => decode_tuple(4, bytes, offset, lens)?,
        Tuple5 => decode_tuple(5, bytes, offset, lens)?,
        Tuple6 => decode_tuple(6, bytes, offset, lens)?,
        Tuple7 => decode_tuple(7, bytes, offset, lens)?,
        Tuple8 => decode_tuple(8, bytes, offset, lens)?,
        TupleN => decode_tuple(decode_prefix_varint(bytes, offset)? as usize + 9, bytes, offset, lens)?,

        Array0 => DynRootBranch::Array0,
        Array1 => DynRootBranch::Array1(Box::new(decode_next_root(bytes, offset, lens)?)),
        //Array2 => decode_array(2, bytes, offset, lens)?,
        //Array3 => decode_array(3, bytes, offset, lens)?,
        //Array4 => decode_array(4, bytes, offset, lens)?,
        // TODO: usize - 2
        ArrayN => decode_array(decode_usize(bytes, offset)?, bytes, offset, lens)?,
        Map => {
            let len = decode_usize(bytes, offset)?;
            match len {
                0 => DynRootBranch::Map0,
                1 => {
                    let key = decode_next_root(bytes, offset, lens)?.into();
                    let value = decode_next_root(bytes, offset, lens)?.into();
                    DynRootBranch::Map1 { key, value }
                }
                _ => {
                    let keys = decode_next_array(bytes, offset, lens)?;
                    let values = decode_next_array(bytes, offset, lens)?;
                    DynRootBranch::Map { len, keys, values }
                }
            }
        }

        // See also: fadaec14-35ad-4dc1-b6dc-6106ab811669
        Obj0 => decode_obj(0, bytes, offset, lens)?,
        Obj1 => decode_obj(1, bytes, offset, lens)?,
        Obj2 => decode_obj(2, bytes, offset, lens)?,
        Obj3 => decode_obj(3, bytes, offset, lens)?,
        Obj4 => decode_obj(4, bytes, offset, lens)?,
        Obj5 => decode_obj(5, bytes, offset, lens)?,
        Obj6 => decode_obj(6, bytes, offset, lens)?,
        Obj7 => decode_obj(7, bytes, offset, lens)?,
        Obj8 => decode_obj(8, bytes, offset, lens)?,
        ObjN => decode_obj(decode_prefix_varint(bytes, offset)? as usize + 9, bytes, offset, lens)?,

        Enum => {
            // TODO: Consider having the enum be:
            //    Root: static_data: RootBranch, instance_data: RootBranch
            //    Array: static_data: RootBranch, instance_data: ArrayBranch, discriminant: ArrayBranch(int)
            // The interesting idea here is that it can be more powerful in that it supports complex value de-duplication
            // Eg: like a primary key. One could represent more information (eg: Like a politician having a name and party
            // when having a table for primaries and delegates). That might be more interesting as some kind of pointer type.
            // It also would support c-style enums, which could be good for eg: rendering.
            //
            // The downside is it's not always clear what the intent is, and how to merge static and instance data for particular
            // languages. Eg: GeoJson could have { "type" : "..." } as static_data in an object here and merge that with another
            // object in instance_data. So many questions though, like does static_data need to be the same type for each discriminant?
            // If so, it would be an ArrayBranch with a fixed size.
            let discriminant = decode_ident(bytes, offset)?;
            let value = decode_next_root(bytes, offset, lens)?.into();
            DynRootBranch::Enum { discriminant, value }
        }

        True => DynRootBranch::Boolean(true),
        False => DynRootBranch::Boolean(false),

        // Int
        IntU64 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 8, false)?),
        IntU56 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 7, false)?),
        IntU48 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 6, false)?),
        IntU40 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 5, false)?),
        IntU32 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 4, false)?),
        IntU24 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 3, false)?),
        IntU16 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 2, false)?),
        IntU8 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 1, false)?),
        IntS64 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 8, true)?),
        IntS56 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 7, true)?),
        IntS48 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 6, true)?),
        IntS40 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 5, true)?),
        IntS32 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 4, true)?),
        IntS24 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 3, true)?),
        IntS16 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 2, true)?),
        IntS8 => DynRootBranch::Integer(RootInteger::new(bytes, offset, 1, true)?),

        // Int Or Float
        Zero => DynRootBranch::Integer(RootInteger::U(0)),
        One => DynRootBranch::Integer(RootInteger::U(1)),
        NegOne => DynRootBranch::Integer(RootInteger::S(-1)),

        // Float,
        F32 => DynRootBranch::Float(RootFloat::F32(f32::from_le_bytes(decode_bytes(4, bytes, offset)?.try_into().unwrap()))),
        F64 => DynRootBranch::Float(RootFloat::F64(f64::from_le_bytes(decode_bytes(8, bytes, offset)?.try_into().unwrap()))),
        NaN => DynRootBranch::Float(RootFloat::NaN), // Works for either f64 or f32.

        Str0 => decode_str(0, bytes, offset)?,
        Str1 => decode_str(1, bytes, offset)?,
        Str2 => decode_str(2, bytes, offset)?,
        Str3 => decode_str(3, bytes, offset)?,
        Str => decode_str(decode_prefix_varint(bytes, offset)? as usize, bytes, offset)?,
    };
    Ok(branch)
}

impl<'a> Default for DynRootBranch<'a> {
    fn default() -> Self {
        DynRootBranch::Void
    }
}

#[derive(Debug)]
pub enum RootInteger {
    S(i64),
    U(u64),
}

impl_type_id!(RootTypeId, [
    // No nullable at root. Some is always just the inner value, and None is just elided.
    Array0: 1,
    Array1: 2,
    ArrayN: 3,
    True: 4,
    False: 5,
    IntU64: 6,
    IntU56: 7,
    IntU48: 8,
    IntU40: 9,
    IntU32: 10,
    IntU24: 11,
    IntU16: 12,
    IntU8: 13,
    IntS64: 14,
    IntS56: 15,
    IntS48: 16,
    IntS40: 17,
    IntS32: 18,
    IntS24: 19,
    IntS16: 20,
    IntS8: 21,
    Zero: 22,
    One: 23,
    NegOne: 24,
    F32: 25,
    F64: 26,
    NaN: 27,
    Str0: 28,
    Str1: 29,
    Str2: 30,
    Str3: 31,
    Str: 32,
    Enum: 33,
    Map: 34,
]);

impl RootInteger {
    #[inline(always)]
    pub fn new(bytes: &[u8], offset: &mut usize, len: usize, signed: bool) -> DecodeResult<Self> {
        let bytes = decode_bytes(len, bytes, offset)?;
        let ok = match (len, signed) {
            (1, true) => Self::S((bytes[0] as i8).into()),
            (1, false) => Self::U(bytes[0].into()),
            (2, true) => Self::S(i16::from_le_bytes(bytes.try_into().unwrap()).into()),
            (2, false) => Self::U(u16::from_le_bytes(bytes.try_into().unwrap()).into()),
            (3, false) => Self::U({
                let b = [bytes[0], bytes[1], bytes[2], 0];
                u32::from_le_bytes(b).into()
            }),
            (3, true) => Self::S({
                let b = [bytes[0], bytes[1], bytes[2], 0];
                i32::from_le_bytes(b).into()
            }),
            (4, true) => Self::S(i32::from_le_bytes(bytes.try_into().unwrap()).into()),
            (4, false) => Self::U(u32::from_le_bytes(bytes.try_into().unwrap()).into()),
            (5, false) => Self::U({
                let b = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], 0, 0, 0];
                u64::from_le_bytes(b)
            }),
            (5, true) => Self::S({
                let b = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], 0, 0, 0];
                i64::from_le_bytes(b)
            }),
            (6, false) => Self::U({
                let b = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], 0, 0];
                u64::from_le_bytes(b)
            }),
            (6, true) => Self::S({
                let b = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], 0, 0];
                i64::from_le_bytes(b)
            }),
            (7, false) => Self::U({
                let b = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], 0];
                u64::from_le_bytes(b)
            }),
            (7, true) => Self::S({
                let b = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], 0];
                i64::from_le_bytes(b)
            }),
            (8, true) => Self::S(i64::from_le_bytes(bytes.try_into().unwrap())),
            (8, false) => Self::U(u64::from_le_bytes(bytes.try_into().unwrap())),
            _ => unreachable!(),
        };
        Ok(ok)
    }
}

#[derive(Debug)]
pub enum RootFloat {
    F64(f64),
    F32(f32),
    NaN,
}
