use crate::internal::encodings::varint::*;
use crate::prelude::*;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::ops::Deref;

/// This wrapper is just to make the Debug impl not display every byte
pub struct Bytes<'a>(&'a [u8]);

impl fmt::Debug for Bytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Bytes").field(&self.0.len()).finish()
    }
}

impl<'a> From<&'a [u8]> for Bytes<'a> {
    #[inline]
    fn from(value: &'a [u8]) -> Self {
        Bytes(value)
    }
}

impl Deref for Bytes<'_> {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub enum ArrayFloat<'a> {
    F64(Bytes<'a>),
    F32(Bytes<'a>),
    DoubleGorilla(Bytes<'a>),
    Zfp32(Bytes<'a>),
    Zfp64(Bytes<'a>),
}

#[derive(Debug)]
pub enum ArrayBool<'a> {
    Packed(Bytes<'a>),
    RLE(bool, Box<DynArrayBranch<'a>>),
}

#[derive(Debug)]
pub struct ArrayEnumVariant<'a> {
    pub ident: Ident<'a>,
    pub data: DynArrayBranch<'a>,
}

#[derive(Debug)]
pub enum DynArrayBranch<'a> {
    Object {
        fields: HashMap<Ident<'a>, DynArrayBranch<'a>>,
    },
    Tuple {
        fields: Vec<DynArrayBranch<'a>>,
    },
    Array0,
    Array {
        len: Box<DynArrayBranch<'a>>,
        values: Box<DynArrayBranch<'a>>,
    },
    ArrayFixed {
        len: usize,
        values: Box<DynArrayBranch<'a>>,
    },
    Map0,
    Map {
        len: Box<DynArrayBranch<'a>>,
        keys: Box<DynArrayBranch<'a>>,
        values: Box<DynArrayBranch<'a>>,
    },
    Integer(ArrayInteger<'a>),
    Nullable {
        opt: Box<DynArrayBranch<'a>>,
        values: Box<DynArrayBranch<'a>>,
    },
    Boolean(ArrayBool<'a>),
    Float(ArrayFloat<'a>),
    Void,
    String(Bytes<'a>),
    Enum {
        discriminants: Box<DynArrayBranch<'a>>,
        variants: Vec<ArrayEnumVariant<'a>>,
    },
    RLE {
        runs: Box<DynArrayBranch<'a>>,
        values: Box<DynArrayBranch<'a>>,
    },
    Dictionary {
        indices: Box<DynArrayBranch<'a>>,
        values: Box<DynArrayBranch<'a>>,
    },
    // TODO:
    // In any array context, we can have a 'dynamic' value, which resolves to an array of DynRootBranch (like a nested file)
    // This generally should not be used, but the existance of it is an escape hatch bringing the capability to use truly unstructured
    // data when necessary. // TODO: The hard-line appraoch would be to enforce the use of enum instead.
    // Dynamic(Bytes<'a>)
}

pub fn decode_next_array<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> DecodeResult<DynArrayBranch<'a>> {
    let id = ArrayTypeId::decode_next(bytes, offset)?;

    use ArrayTypeId::*;

    fn decode_ints<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize, encoding: ArrayIntegerEncoding) -> DecodeResult<DynArrayBranch<'a>> {
        let bytes = decode_bytes_from_len(bytes, offset, lens)?.into();
        Ok(DynArrayBranch::Integer(ArrayInteger { bytes, encoding }))
    }

    fn decode_bytes_from_len<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> DecodeResult<Bytes<'a>> {
        let len = decode_suffix_varint(bytes, lens)?;
        Ok(decode_bytes(len as usize, bytes, offset)?.into())
    }

    // See also e25db64d-8424-46b9-bdc1-cdb618807513
    fn decode_tuple<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> DecodeResult<DynArrayBranch<'a>> {
        let mut fields = Vec::with_capacity(num_fields);
        for _ in 0..num_fields {
            let child = decode_next_array(bytes, offset, lens)?;
            fields.push(child);
        }
        Ok(DynArrayBranch::Tuple { fields })
    }

    // See also 47a1482f-5ce3-4b78-b356-30c66dc60cda
    fn decode_obj<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> DecodeResult<DynArrayBranch<'a>> {
        let mut fields = HashMap::with_capacity(num_fields);
        for _ in 0..num_fields {
            let name = crate::internal::decode_ident(bytes, offset)?;
            let child = decode_next_array(bytes, offset, lens)?;
            fields.insert(name, child);
        }
        Ok(DynArrayBranch::Object { fields })
    }

    let branch = match id {
        Nullable => {
            let opt = decode_next_array(bytes, offset, lens)?.into();
            let values = decode_next_array(bytes, offset, lens)?.into();
            DynArrayBranch::Nullable { opt, values }
        }
        Void => DynArrayBranch::Void,
        Tuple2 => decode_tuple(2, bytes, offset, lens)?,
        Tuple3 => decode_tuple(3, bytes, offset, lens)?,
        Tuple4 => decode_tuple(4, bytes, offset, lens)?,
        Tuple5 => decode_tuple(5, bytes, offset, lens)?,
        Tuple6 => decode_tuple(6, bytes, offset, lens)?,
        Tuple7 => decode_tuple(7, bytes, offset, lens)?,
        Tuple8 => decode_tuple(8, bytes, offset, lens)?,
        TupleN => decode_tuple(decode_prefix_varint(bytes, offset)? as usize + 9, bytes, offset, lens)?,
        ArrayVar => {
            let len = decode_next_array(bytes, offset, lens)?;
            match len {
                DynArrayBranch::Void => DynArrayBranch::Array0,
                _ => {
                    // FIXME: Verify that len is Integer here. If not, the file is invalid.
                    // This may not be verified later if the schema is selectively matched.
                    let len = Box::new(len);
                    let values = decode_next_array(bytes, offset, lens)?;
                    let values = Box::new(values);
                    DynArrayBranch::Array { len, values }
                }
            }
        }
        ArrayFixed => {
            let len = decode_usize(bytes, offset)?;
            let values = decode_next_array(bytes, offset, lens)?;
            let values = Box::new(values);
            DynArrayBranch::ArrayFixed { len, values }
        }
        Map => {
            let len = decode_next_array(bytes, offset, lens)?;
            match len {
                DynArrayBranch::Void => DynArrayBranch::Map0,
                _ => {
                    // FIXME: Verify that len is Integer here. If not, the file is invalid.
                    // This may not be verified later if the schema is selectively matched.
                    let len = Box::new(len);
                    let keys = decode_next_array(bytes, offset, lens)?;
                    let keys = Box::new(keys);
                    let values = decode_next_array(bytes, offset, lens)?;
                    let values = Box::new(values);
                    DynArrayBranch::Map { len, keys, values }
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
        PackedBool => {
            let bytes = decode_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Boolean(ArrayBool::Packed(bytes))
        }
        RLEBoolTrue | RLEBoolFalse => {
            let first = matches!(id, ArrayTypeId::RLEBoolTrue);
            let runs = decode_next_array(bytes, offset, lens)?;
            DynArrayBranch::Boolean(ArrayBool::RLE(first, runs.into()))
        }
        IntSimple16 => decode_ints(bytes, offset, lens, ArrayIntegerEncoding::Simple16)?,
        IntPrefixVar => decode_ints(bytes, offset, lens, ArrayIntegerEncoding::PrefixVarInt)?,
        U8 => decode_ints(bytes, offset, lens, ArrayIntegerEncoding::U8)?,
        F32 => {
            let bytes = decode_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::F32(bytes))
        }
        F64 => {
            let bytes = decode_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::F64(bytes))
        }
        Zfp32 => {
            let bytes = decode_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::Zfp32(bytes))
        }
        Zfp64 => {
            let bytes = decode_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::Zfp64(bytes))
        }
        Utf8 => {
            let bytes = decode_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::String(bytes)
        }
        DoubleGorilla => {
            let bytes = decode_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::DoubleGorilla(bytes))
        }
        Enum => {
            let count = decode_prefix_varint(bytes, offset)? as usize;
            let mut variants = Vec::with_capacity(count);

            // TODO: Elide discriminants when there are 0 or 1 variants
            let discriminants = decode_next_array(bytes, offset, lens)?.into();

            if count != 0 {
                for _ in 0..count {
                    variants.push(ArrayEnumVariant {
                        ident: decode_ident(bytes, offset)?,
                        data: decode_next_array(bytes, offset, lens)?,
                    });
                }
            }

            DynArrayBranch::Enum { discriminants, variants }
        }
        RLE => {
            let values = decode_next_array(bytes, offset, lens)?.into();
            let runs = decode_next_array(bytes, offset, lens)?.into();
            DynArrayBranch::RLE { runs, values }
        }
        Dictionary => {
            let values = decode_next_array(bytes, offset, lens)?.into();
            let indices = decode_next_array(bytes, offset, lens)?.into();
            DynArrayBranch::Dictionary { values, indices }
        }
    };

    Ok(branch)
}

impl<'a> Default for DynArrayBranch<'a> {
    fn default() -> Self {
        DynArrayBranch::Void
    }
}

impl_type_id!(ArrayTypeId, [
    Nullable: 1,
    ArrayVar: 2,
    PackedBool: 3,
    IntSimple16: 4,
    IntPrefixVar: 5,
    F32: 6,
    F64: 7,
    Utf8: 8,
    DoubleGorilla: 9,
    Map: 10,
    Enum: 11,
    ArrayFixed: 12,
    U8: 13,
    RLE: 14,
    Zfp32: 15,
    Zfp64: 16,
    Dictionary: 17,
    RLEBoolTrue: 18,
    RLEBoolFalse: 19,
]);

#[derive(Debug)]
pub struct ArrayInteger<'a> {
    pub bytes: Bytes<'a>,
    //delta: bool,
    //zigzag: bool,
    pub encoding: ArrayIntegerEncoding,
}

#[derive(Debug)]
pub enum ArrayIntegerEncoding {
    PrefixVarInt,
    Simple16,
    U8,
}
