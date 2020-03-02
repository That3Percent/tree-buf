use crate::internal::encodings::varint::decode_prefix_varint;
use crate::prelude::*;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

// TODO: Most of these comments are likely out of date.
// TODO: The idea for int is to always encode up to 64 bit values,
// but for any data store the min value and offset first, then use
// that to select an optimal encoding. When deserializing, the min and
// offset can be used to find if the data type required by the schema
// matches.
// Consider something like this - https://lemire.me/blog/2012/09/12/fast-integer-compression-decoding-billions-of-integers-per-second/

// TODO: GUID
// TODO: Bytes = [u8]
// TODO: Date
// TODO: Enum - Something like this... needs to simmer.
//              The enum primitive id contains 1 number which is the discriminant count.
//              The enum discriminant as int is contained in the enum branch
//              Each sub-branch contains the discriminant name (string)
//              Each branch may have a sub-branch for data belonging to the variant for that discriminant in each entry.
//              In many cases, this data will be Void, which may be wasteful to have a branch for.
//              ..
//              Because enum is so flexible, it's possible to wrap some dynamic data into it. Eg: EnumValue<T>.
//              This would create some number of sub-branches 'dynamically'.

// TODO: Try each compression on a sample of the data (first 1024 or so?) in turn to decide which to use.
// 8-Array & different fixed/variable sizes - 0,1,2,128,custom(follows). Fixed 0 necessarily has Void child
// ? Integer - Different for array context or not? Min/Max? Different encoding options? (uncompressed option) signed, unsigned, 8,16,32,64
// ?-Enum - String,Int, or other discriminant, whether or not there is data for sub-branches, and whether
// 4-Float (32/64, compresssed/not) Consider:
//      dfcm - https://userweb.cs.txstate.edu/~mb92/papers/dcc06.pdf
//      https://www.cs.unc.edu/~isenburg/lcpfpv/
//      https://akumuli.org/akumuli/2017/02/05/compression_part2/
//      Consider an 'allow-lossy' flag (per field) or input trait
// 1-128 bits
// 2-Blob - compressed, uncompressed

#[derive(Debug)]
pub enum DynRootBranch<'a> {
    Object { children: HashMap<&'a str, DynRootBranch<'a>> },
    Tuple { children: Vec<DynRootBranch<'a>> },
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
            let name = crate::internal::read_str(bytes, offset)?;
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
        Obj1 => read_obj(1, bytes, offset, lens)?,
        Obj2 => read_obj(2, bytes, offset, lens)?,
        Obj3 => read_obj(3, bytes, offset, lens)?,
        Obj4 => read_obj(4, bytes, offset, lens)?,
        Obj5 => read_obj(5, bytes, offset, lens)?,
        Obj6 => read_obj(6, bytes, offset, lens)?,
        Obj7 => read_obj(7, bytes, offset, lens)?,
        Obj8 => read_obj(8, bytes, offset, lens)?,
        ObjN => read_obj(decode_prefix_varint(bytes, offset)? as usize + 9, bytes, offset, lens)?,

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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum RootTypeId {
    // Constructions
    // No nullable at root. Some is always just the inner value, and None is just elided.
    Void, // Void at root may be necessary. Eg: Tuple(?, Option<?>, ?) at root.
    Tuple2,
    Tuple3,
    Tuple4,
    Tuple5,
    Tuple6,
    Tuple7,
    Tuple8,
    TupleN,
    Array0,
    Array1,
    ArrayN, // Array0 and Array1 don't count as entering an array context. // TODO: Special common sized arrays - eg: 16 for 4x4 matrix.
    Obj1,
    Obj2,
    Obj3,
    Obj4,
    Obj5,
    Obj6,
    Obj7,
    Obj8,
    ObjN, // Obj0 is just void.

    // Bool
    True,
    False,

    // Int
    IntU64,
    IntU56,
    IntU48,
    IntU40,
    IntU32,
    IntU24,
    IntU16,
    IntU8,
    IntS64,
    IntS56,
    IntS48,
    IntS40,
    IntS32,
    IntS24,
    IntS16,
    IntS8,

    // Int Or Float
    Zero,
    One,
    NegOne,

    // Float,
    F32,
    F64,
    NaN, // Works for either f64 or f32.

    // Str
    Str0,
    Str1,
    Str2,
    Str3,
    Str, // Str0 = Empty string, Str1-Str3 get unit abbreviations, like ft or ft²
}

impl RootTypeId {
    // See also 582c63bc-851d-40d5-8ccc-caa05e8f3dc6
    fn read_next(bytes: &[u8], offset: &mut usize) -> ReadResult<Self> {
        let next = bytes.get(*offset).ok_or_else(|| ReadError::InvalidFormat(InvalidFormat::EndOfFile))?;
        *offset += 1;
        (*next).try_into()
    }
}

impl TryFrom<u8> for RootTypeId {
    type Error = ReadError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use RootTypeId::*;
        let ok = match value {
            0 => Void,
            1 => Tuple2,
            2 => Tuple3,
            3 => Tuple4,
            4 => Tuple5,
            5 => Tuple6,
            6 => Tuple7,
            7 => Tuple8,
            8 => TupleN,
            9 => Array0,
            10 => Array1,
            11 => ArrayN,
            12 => Obj1,
            13 => Obj2,
            14 => Obj3,
            15 => Obj4,
            16 => Obj5,
            17 => Obj6,
            18 => Obj7,
            19 => Obj8,
            20 => ObjN,
            21 => True,
            22 => False,
            23 => IntU64,
            24 => IntU56,
            25 => IntU48,
            26 => IntU40,
            27 => IntU32,
            28 => IntU24,
            29 => IntU16,
            30 => IntU8,
            31 => IntS64,
            32 => IntS56,
            33 => IntS48,
            34 => IntS40,
            35 => IntS32,
            36 => IntS24,
            37 => IntS16,
            38 => IntS8,
            39 => Zero,
            40 => One,
            41 => NegOne,
            42 => F32,
            43 => F64,
            44 => NaN,
            45 => Str0,
            46 => Str1,
            47 => Str2,
            48 => Str3,
            49 => Str,
            _ => return Err(ReadError::InvalidFormat(InvalidFormat::UnrecognizedTypeId)),
        };
        debug_assert_eq!(value, ok.into());
        Ok(ok)
    }
}

impl From<RootTypeId> for u8 {
    fn from(value: RootTypeId) -> Self {
        use RootTypeId::*;
        match value {
            Void => 0,
            Tuple2 => 1,
            Tuple3 => 2,
            Tuple4 => 3,
            Tuple5 => 4,
            Tuple6 => 5,
            Tuple7 => 6,
            Tuple8 => 7,
            TupleN => 8,
            Array0 => 9,
            Array1 => 10,
            ArrayN => 11,
            Obj1 => 12,
            Obj2 => 13,
            Obj3 => 14,
            Obj4 => 15,
            Obj5 => 16,
            Obj6 => 17,
            Obj7 => 18,
            Obj8 => 19,
            ObjN => 20,
            True => 21,
            False => 22,
            IntU64 => 23,
            IntU56 => 24,
            IntU48 => 25,
            IntU40 => 26,
            IntU32 => 27,
            IntU24 => 28,
            IntU16 => 29,
            IntU8 => 30,
            IntS64 => 31,
            IntS56 => 32,
            IntS48 => 33,
            IntS40 => 34,
            IntS32 => 35,
            IntS24 => 36,
            IntS16 => 37,
            IntS8 => 38,
            Zero => 39,
            One => 40,
            NegOne => 41,
            F32 => 42,
            F64 => 43,
            NaN => 44,
            Str0 => 45,
            Str1 => 46,
            Str2 => 47,
            Str3 => 48,
            Str => 49,
        }
    }
}

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
