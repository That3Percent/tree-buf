use crate::internal::encodings::varint::*;
use crate::prelude::*;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

#[derive(Debug)]
pub enum ArrayFloat<'a> {
    F64(&'a [u8]),
    F32(&'a [u8]),
}

#[derive(Debug)]
pub enum DynArrayBranch<'a> {
    Object { children: HashMap<&'a str, DynArrayBranch<'a>> },
    Tuple { children: Vec<DynArrayBranch<'a>> },
    Array0,
    Array { len: Box<DynArrayBranch<'a>>, values: Box<DynArrayBranch<'a>> },
    Integer(ArrayInteger<'a>),
    Nullable { opt: &'a [u8], values: Box<DynArrayBranch<'a>> },
    Boolean(&'a [u8]),
    Float(ArrayFloat<'a>),
    Void,
    String(&'a [u8]),
    // TODO:
    // In any array context, we can have a 'dynamic' value, which resolves to an array of DynRootBranch (like a nested file)
    // This generally should not be used, but the existance of it is an escape hatch bringing the capability to use truly unstructured
    // data when necessary. // TODO: The hard-line appraoch would be to enforce the use of enum instead.
    // Dynamic(&'a [u8]),
}

pub fn read_next_array<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynArrayBranch<'a>> {
    let id = ArrayTypeId::read_next(bytes, offset)?;

    use ArrayTypeId::*;

    fn read_ints<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize, encoding: ArrayIntegerEncoding) -> ReadResult<DynArrayBranch<'a>> {
        let bytes = read_bytes_from_len(bytes, offset, lens)?;
        Ok(DynArrayBranch::Integer(ArrayInteger { bytes, encoding }))
    }

    fn read_bytes_from_len<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<&'a [u8]> {
        let len = decode_suffix_varint(bytes, lens)?;
        read_bytes(len as usize, bytes, offset)
    }

    // See also e25db64d-8424-46b9-bdc1-cdb618807513
    fn read_tuple<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynArrayBranch<'a>> {
        let mut children = Vec::with_capacity(num_fields);
        for _ in 0..num_fields {
            let child = read_next_array(bytes, offset, lens)?;
            children.push(child);
        }
        Ok(DynArrayBranch::Tuple { children })
    }

    // See also 47a1482f-5ce3-4b78-b356-30c66dc60cda
    fn read_obj<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynArrayBranch<'a>> {
        let mut children = HashMap::with_capacity(num_fields);
        for _ in 0..num_fields {
            let name = crate::internal::read_str(bytes, offset)?;
            let child = read_next_array(bytes, offset, lens)?;
            children.insert(name, child);
        }
        Ok(DynArrayBranch::Object { children })
    }

    let branch = match id {
        Nullable => {
            let opt = read_bytes_from_len(bytes, offset, lens)?;
            let values = read_next_array(bytes, offset, lens)?;
            let values = Box::new(values);
            DynArrayBranch::Nullable { opt, values }
        }
        Void => DynArrayBranch::Void,
        Tuple2 => read_tuple(2, bytes, offset, lens)?,
        Tuple3 => read_tuple(3, bytes, offset, lens)?,
        Tuple4 => read_tuple(4, bytes, offset, lens)?,
        Tuple5 => read_tuple(5, bytes, offset, lens)?,
        Tuple6 => read_tuple(6, bytes, offset, lens)?,
        Tuple7 => read_tuple(7, bytes, offset, lens)?,
        Tuple8 => read_tuple(8, bytes, offset, lens)?,
        TupleN => read_tuple(decode_prefix_varint(bytes, offset)? as usize + 9, bytes, offset, lens)?,
        ArrayVar => {
            let values = read_next_array(bytes, offset, lens)?;
            match values {
                DynArrayBranch::Void => DynArrayBranch::Array0,
                _ => {
                    // FIXME: Verify that this is Integer here. If not, the file is invalid.
                    // This may not be verified later if the schema is selectively matched.
                    let len = read_next_array(bytes, offset, lens)?;
                    let len = Box::new(len);
                    let values = Box::new(values);
                    DynArrayBranch::Array { len, values }
                }
            }
        }

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
        Boolean => {
            let bytes = read_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Boolean(bytes)
        },
        IntSimple16 => read_ints(bytes, offset, lens, ArrayIntegerEncoding::Simple16)?,
        IntPrefixVar => read_ints(bytes, offset, lens, ArrayIntegerEncoding::PrefixVarInt)?,
        F32 => {
            let bytes = read_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::F32(bytes))
        },
        F64 => {
            let bytes = read_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::F64(bytes))
        }
        Utf8 => {
            let bytes = read_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::String(bytes)
        }
    };

    Ok(branch)
}

impl<'a> Default for DynArrayBranch<'a> {
    fn default() -> Self {
        DynArrayBranch::Void
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ArrayTypeId {
    // Constructions
    Nullable,
    Void,
    Tuple2,
    Tuple3,
    Tuple4,
    Tuple5,
    Tuple6,
    Tuple7,
    Tuple8,
    TupleN,
    ArrayVar, // TODO: ArrayFixed
    Obj1,
    Obj2,
    Obj3,
    Obj4,
    Obj5,
    Obj6,
    Obj7,
    Obj8,
    ObjN,

    // Bool
    Boolean,

    // Int
    IntSimple16,
    IntPrefixVar,

    // Float,
    F32,
    F64,

    // Str
    Utf8,
}

impl ArrayTypeId {
    // See also 582c63bc-851d-40d5-8ccc-caa05e8f3dc6
    fn read_next(bytes: &[u8], offset: &mut usize) -> ReadResult<ArrayTypeId> {
        let next = bytes.get(*offset).ok_or_else(|| ReadError::InvalidFormat(InvalidFormat::EndOfFile))?;
        *offset += 1;
        (*next).try_into()
    }
}

#[derive(Debug)]
pub struct ArrayInteger<'a> {
    pub bytes: &'a [u8],
    //delta: bool,
    //zigzag: bool,
    pub encoding: ArrayIntegerEncoding,
}

#[derive(Debug)]
pub enum ArrayIntegerEncoding {
    PrefixVarInt,
    Simple16,
}

impl TryFrom<u8> for ArrayTypeId {
    type Error = ReadError;
    fn try_from(value: u8) -> ReadResult<Self> {
        use ArrayTypeId::*;
        let ok = match value {
            0 => Nullable,
            1 => Void,
            2 => Tuple2,
            3 => Tuple3,
            4 => Tuple4,
            5 => Tuple5,
            6 => Tuple6,
            7 => Tuple7,
            8 => Tuple8,
            9 => TupleN,
            10 => ArrayVar,
            11 => Obj1,
            12 => Obj2,
            13 => Obj3,
            14 => Obj4,
            15 => Obj5,
            16 => Obj6,
            17 => Obj7,
            18 => Obj8,
            19 => ObjN,
            20 => Boolean,
            21 => IntSimple16,
            22 => IntPrefixVar,
            23 => F32,
            24 => F64,
            25 => Utf8,
            _ => return Err(ReadError::InvalidFormat(InvalidFormat::UnrecognizedTypeId)),
        };
        debug_assert_eq!(value, ok.into());
        Ok(ok)
    }
}

impl From<ArrayTypeId> for u8 {
    fn from(value: ArrayTypeId) -> Self {
        use ArrayTypeId::*;
        match value {
            Nullable => 0,
            Void => 1,
            Tuple2 => 2,
            Tuple3 => 3,
            Tuple4 => 4,
            Tuple5 => 5,
            Tuple6 => 6,
            Tuple7 => 7,
            Tuple8 => 8,
            TupleN => 9,
            ArrayVar => 10,
            Obj1 => 11,
            Obj2 => 12,
            Obj3 => 13,
            Obj4 => 14,
            Obj5 => 15,
            Obj6 => 16,
            Obj7 => 17,
            Obj8 => 18,
            ObjN => 19,
            Boolean => 20,
            IntSimple16 => 21,
            IntPrefixVar => 22,
            F32 => 23,
            F64 => 24,
            Utf8 => 25,
        }
    }
}
