use crate::prelude::*;
use crate::internal::encodings::varint::decode_suffix_varint;
use std::collections::HashMap;


#[derive(Debug)]
pub enum OneOrMany<'a, T> {
    One(T),
    Many(&'a [u8]),
}

impl<'a, T: BatchData + std::fmt::Debug> OneOrMany<'a, T> {
    pub fn new(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize, is_array_context: bool) -> ReadResult<Self> {
        if is_array_context {
            let len = decode_suffix_varint(bytes, lens)? as usize;
            let bytes = read_bytes(bytes, len, offset)?;
            Ok(OneOrMany::Many(bytes))
        } else {
            let value = T::read_one(bytes, offset)?;
            Ok(OneOrMany::One(value))
        }
    }
}

#[derive(Debug)]
pub enum DynBranch<'a> {
    Object { children: HashMap<&'a str, DynBranch<'a>> },
    Tuple { children: Vec<DynBranch<'a>> },
    Array { len: OneOrMany<'a, Array>, values: Box<DynBranch<'a>> },
    Integer(OneOrMany<'a, u64>),
    Nullable {opt: OneOrMany<'a, Nullable>, values: Box<DynBranch<'a>> },
    Boolean(OneOrMany<'a, bool>),
    Float(OneOrMany<'a, f64>),
    Void,
    String(OneOrMany<'a, String>),
}

impl<'a> Default for DynBranch<'a> {
    fn default() -> Self {
        DynBranch::Void
    }
}

fn read_next<'a>(bytes: &'a [u8], offset: &'_ mut usize, lens: &'_ mut usize, is_array_context: bool) -> ReadResult<DynBranch<'a>> {
    let primitive = PrimitiveId::read(bytes, offset)?;

    // TODO: The PrimitiveId isn't really pulling it's weight, considering how each type is getting special cased here anyway
    let branch = match primitive {
        PrimitiveId::Object { num_fields } => {
            let mut children = HashMap::with_capacity(num_fields);
            for _ in 0..num_fields {
                let name = Str::read_one(bytes, offset)?;
                let child = read_next(bytes, offset, lens, is_array_context)?;
                children.insert(name, child);
            }
            DynBranch::Object {children }
        },
        PrimitiveId::Tuple { num_fields } => {
            let mut children = Vec::with_capacity(num_fields);
            for _ in 0..num_fields {
                let child = read_next(bytes, offset, lens, is_array_context)?;
                children.push(child)
            }
            DynBranch::Tuple { children }
        }
        PrimitiveId::Array => {
            let len = OneOrMany::new(bytes, offset, lens, is_array_context)?;
            let values = read_next(bytes, offset, lens, true)?;
            let values = Box::new(values);
            DynBranch::Array {len, values}
        },
        PrimitiveId::Nullable => {
            let opt = OneOrMany::new(bytes, offset, lens, is_array_context)?;
            let values = read_next(bytes, offset, lens, is_array_context)?;
            let values = Box::new(values);
            DynBranch::Nullable {opt, values}
        }
        PrimitiveId::Integer => DynBranch::Integer(OneOrMany::new(bytes, offset, lens, is_array_context)?),
        PrimitiveId::Boolean => DynBranch::Boolean(OneOrMany::new(bytes, offset, lens, is_array_context)?),
        PrimitiveId::Float => DynBranch::Float(OneOrMany::new(bytes, offset, lens, is_array_context)?),
        PrimitiveId::Void => DynBranch::Void,
        PrimitiveId::String => DynBranch::String(OneOrMany::new(bytes, offset, lens, is_array_context)?),
    };
    Ok(branch)
}

pub fn read_root(bytes: &[u8]) -> ReadResult<DynBranch<'_>> {
    if bytes.len() == 0 {
        return Ok(DynBranch::Void);
    }
    let mut lens = bytes.len() - 1;
    let mut offset = 0;
    read_next(bytes, &mut offset, &mut lens, false)
} 