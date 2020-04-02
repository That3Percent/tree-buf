use crate::internal::encodings::varint::*;
use crate::prelude::*;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::ops::Deref;

/// This wrapper is just to make the Debug impl not write every byte
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
        opt: Bytes<'a>,
        values: Box<DynArrayBranch<'a>>,
    },
    Boolean(Bytes<'a>),
    Float(ArrayFloat<'a>),
    Void,
    String(Bytes<'a>),
    Enum {
        discriminants: Box<DynArrayBranch<'a>>,
        variants: Vec<ArrayEnumVariant<'a>>,
    },
    // TODO:
    // In any array context, we can have a 'dynamic' value, which resolves to an array of DynRootBranch (like a nested file)
    // This generally should not be used, but the existance of it is an escape hatch bringing the capability to use truly unstructured
    // data when necessary. // TODO: The hard-line appraoch would be to enforce the use of enum instead.
    // Dynamic(Bytes<'a>),
}

pub fn read_next_array<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynArrayBranch<'a>> {
    let id = ArrayTypeId::read_next(bytes, offset)?;

    use ArrayTypeId::*;

    fn read_ints<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize, encoding: ArrayIntegerEncoding) -> ReadResult<DynArrayBranch<'a>> {
        let bytes = read_bytes_from_len(bytes, offset, lens)?.into();
        Ok(DynArrayBranch::Integer(ArrayInteger { bytes, encoding }))
    }

    fn read_bytes_from_len<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<Bytes<'a>> {
        let len = decode_suffix_varint(bytes, lens)?;
        Ok(read_bytes(len as usize, bytes, offset)?.into())
    }

    // See also e25db64d-8424-46b9-bdc1-cdb618807513
    fn read_tuple<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynArrayBranch<'a>> {
        let mut fields = Vec::with_capacity(num_fields);
        for _ in 0..num_fields {
            let child = read_next_array(bytes, offset, lens)?;
            fields.push(child);
        }
        Ok(DynArrayBranch::Tuple { fields })
    }

    // See also 47a1482f-5ce3-4b78-b356-30c66dc60cda
    fn read_obj<'a>(num_fields: usize, bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize) -> ReadResult<DynArrayBranch<'a>> {
        let mut fields = HashMap::with_capacity(num_fields);
        for _ in 0..num_fields {
            let name = crate::internal::read_ident(bytes, offset)?;
            let child = read_next_array(bytes, offset, lens)?;
            fields.insert(name, child);
        }
        Ok(DynArrayBranch::Object { fields })
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
            let len = read_next_array(bytes, offset, lens)?;
            match len {
                DynArrayBranch::Void => DynArrayBranch::Array0,
                _ => {
                    // FIXME: Verify that len is Integer here. If not, the file is invalid.
                    // This may not be verified later if the schema is selectively matched.
                    let len = Box::new(len);
                    let values = read_next_array(bytes, offset, lens)?;
                    let values = Box::new(values);
                    DynArrayBranch::Array { len, values }
                }
            }
        }
        ArrayFixed => {
            let len = read_usize(bytes, offset)?;
            let values = read_next_array(bytes, offset, lens)?;
            let values = Box::new(values);
            DynArrayBranch::ArrayFixed { len, values }
        }
        Map => {
            let len = read_next_array(bytes, offset, lens)?;
            match len {
                DynArrayBranch::Void => DynArrayBranch::Map0,
                _ => {
                    // FIXME: Verify that len is Integer here. If not, the file is invalid.
                    // This may not be verified later if the schema is selectively matched.
                    let len = Box::new(len);
                    let keys = read_next_array(bytes, offset, lens)?;
                    let keys = Box::new(keys);
                    let values = read_next_array(bytes, offset, lens)?;
                    let values = Box::new(values);
                    DynArrayBranch::Map { len, keys, values }
                }
            }
        }

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
        Boolean => {
            let bytes = read_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Boolean(bytes)
        }
        IntSimple16 => read_ints(bytes, offset, lens, ArrayIntegerEncoding::Simple16)?,
        IntPrefixVar => read_ints(bytes, offset, lens, ArrayIntegerEncoding::PrefixVarInt)?,
        F32 => {
            let bytes = read_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::F32(bytes))
        }
        F64 => {
            let bytes = read_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::F64(bytes))
        }
        Utf8 => {
            let bytes = read_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::String(bytes)
        }
        DoubleGorilla => {
            let bytes = read_bytes_from_len(bytes, offset, lens)?;
            DynArrayBranch::Float(ArrayFloat::DoubleGorilla(bytes))
        }
        Enum => {
            let count = decode_prefix_varint(bytes, offset)? as usize;
            let mut variants = Vec::with_capacity(count);

            // TODO: Elide discriminants when there are 0 or 1 variants
            let discriminants = read_next_array(bytes, offset, lens)?.into();

            if count != 0 {
                for _ in 0..count {
                    variants.push(ArrayEnumVariant {
                        ident: read_ident(bytes, offset)?,
                        data: read_next_array(bytes, offset, lens)?,
                    });
                }
            }

            DynArrayBranch::Enum { discriminants, variants }
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
    Boolean: 3,
    IntSimple16: 4,
    IntPrefixVar: 5,
    F32: 6,
    F64: 7,
    Utf8: 8,
    DoubleGorilla: 9,
    Map: 10,
    Enum: 11,
    ArrayFixed: 12,
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
}
