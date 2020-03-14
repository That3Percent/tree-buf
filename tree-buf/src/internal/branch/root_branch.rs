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

// TODO: Try each compression on a sample of the data (first 1024 or so?) in turn to decide which to use.
// 8-Array & different fixed/variable sizes - 0,1,2,128,custom(follows). Fixed 0 necessarily has Void child
// 1-128 bits

#[derive(Debug)]
pub enum DynRootBranch<'a> {
    Object { children: HashMap<Ident<'a>, DynRootBranch<'a>> },
    Tuple { children: Vec<DynRootBranch<'a>> },
    Enum { discriminant: Ident<'a>, value: Box<DynRootBranch<'a>> },
    Array0,                         // Separate from Array because it infers Void
    Array1(Box<DynRootBranch<'a>>), // Separate from Array because it does not need to enter an array context
    Array { len: usize, values: DynArrayBranch<'a> },
    Integer(RootInteger),
    Boolean(bool),
    Float(RootFloat),
    Void,
    String(&'a str),
}

pub fn read_next_root<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynRootBranch<'a>> {
    let id = RootTypeId::read_next(bytes, offset)?;

    // See also e25db64d-8424-46b9-bdc1-cdb618807513
    fn read_tuple<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynRootBranch<'a>> {
        let mut children = Vec::with_capacity(num_fields);
        for _ in 0..num_fields {
            let child = read_next_root(bytes, offset, lens)?;
            children.push(child);
        }
        Ok(DynRootBranch::Tuple { children })
    }

    fn read_array<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynRootBranch<'a>> {
        let values = read_next_array(bytes, offset, lens)?;
        Ok(DynRootBranch::Array { len, values })
    }

    // See also 47a1482f-5ce3-4b78-b356-30c66dc60cda
    fn read_obj<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynRootBranch<'a>> {
        let mut children = HashMap::with_capacity(num_fields);
        for _ in 0..num_fields {
            let name = crate::internal::read_ident(bytes, offset)?;
            let child = read_next_root(bytes, offset, lens)?;
            children.insert(name, child);
        }
        Ok(DynRootBranch::Object { children })
    }

    fn read_str<'a>(len: usize, bytes: &'a [u8], offset: &'_ mut usize) -> ReadResult<DynRootBranch<'a>> {
        let bytes = read_bytes(len, bytes, offset)?;
        let s = std::str::from_utf8(bytes)?;
        Ok(DynRootBranch::String(s))
    }

    use RootTypeId::*;
    let branch = match id {
        Void => DynRootBranch::Void,

        Tuple2 => read_tuple(2, bytes, offset, lens)?,
        Tuple3 => read_tuple(3, bytes, offset, lens)?,
        Tuple4 => read_tuple(4, bytes, offset, lens)?,
        Tuple5 => read_tuple(5, bytes, offset, lens)?,
        Tuple6 => read_tuple(6, bytes, offset, lens)?,
        Tuple7 => read_tuple(7, bytes, offset, lens)?,
        Tuple8 => read_tuple(8, bytes, offset, lens)?,
        TupleN => read_tuple(decode_prefix_varint(bytes, offset)? as usize + 9, bytes, offset, lens)?,

        Array0 => DynRootBranch::Array0,
        Array1 => DynRootBranch::Array1(Box::new(read_next_root(bytes, offset, lens)?)),
        //Array2 => read_array(2, bytes, offset, lens)?,
        //Array3 => read_array(3, bytes, offset, lens)?,
        //Array4 => read_array(4, bytes, offset, lens)?,
        // TODO: usize - 2
        ArrayN => read_array(read_usize(bytes, offset)?, bytes, offset, lens)?,

        // See also: fadaec14-35ad-4dc1-b6dc-6106ab811669
        Obj0 => read_obj(0, bytes, offset, lens)?,
        Obj1 => read_obj(1, bytes, offset, lens)?,
        Obj2 => read_obj(2, bytes, offset, lens)?,
        Obj3 => read_obj(3, bytes, offset, lens)?,
        Obj4 => read_obj(4, bytes, offset, lens)?,
        Obj5 => read_obj(5, bytes, offset, lens)?,
        Obj6 => read_obj(6, bytes, offset, lens)?,
        Obj7 => read_obj(7, bytes, offset, lens)?,
        Obj8 => read_obj(8, bytes, offset, lens)?,
        ObjN => read_obj(decode_prefix_varint(bytes, offset)? as usize + 9, bytes, offset, lens)?,

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
            let discriminant = read_ident(bytes, offset)?;
            let value = read_next_root(bytes, offset, lens)?.into();
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
        F32 => DynRootBranch::Float(RootFloat::F32(f32::from_le_bytes(read_bytes(4, bytes, offset)?.try_into().unwrap()))),
        F64 => DynRootBranch::Float(RootFloat::F64(f64::from_le_bytes(read_bytes(8, bytes, offset)?.try_into().unwrap()))),
        NaN => DynRootBranch::Float(RootFloat::NaN), // Works for either f64 or f32.

        Str0 => read_str(0, bytes, offset)?,
        Str1 => read_str(1, bytes, offset)?,
        Str2 => read_str(2, bytes, offset)?,
        Str3 => read_str(3, bytes, offset)?,
        Str => read_str(decode_prefix_varint(bytes, offset)? as usize, bytes, offset)?,
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
]);

impl RootInteger {
    #[inline(always)]
    pub fn new(bytes: &[u8], offset: &mut usize, len: usize, signed: bool) -> ReadResult<Self> {
        let bytes = read_bytes(len, bytes, offset)?;
        let ok = match (len, signed) {
            (1, true) => Self::S((bytes[0] as i8).into()),
            (1, false) => Self::U(bytes[0].into()),
            (2, true) => Self::S(i16::from_le_bytes(bytes.try_into().unwrap()).into()),
            (2, false) => Self::U(u16::from_le_bytes(bytes.try_into().unwrap()).into()),
            (4, true) => Self::S(i32::from_le_bytes(bytes.try_into().unwrap()).into()),
            (4, false) => Self::U(u32::from_le_bytes(bytes.try_into().unwrap()).into()),
            (8, true) => Self::S(i64::from_le_bytes(bytes.try_into().unwrap()).into()),
            (8, false) => Self::U(u64::from_le_bytes(bytes.try_into().unwrap()).into()),
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
