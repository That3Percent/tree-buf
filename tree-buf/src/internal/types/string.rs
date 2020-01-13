use crate::prelude::*;
use crate::encodings::varint::{encode_prefix_varint, decode_prefix_varint};
use std::vec::IntoIter;

pub struct Str;

// TODO: Move this to BatchData
impl Str {
    pub fn write_one(value: &str, bytes: &mut Vec<u8>) {
        encode_prefix_varint(value.len() as u64, bytes);
        bytes.extend_from_slice(value.as_bytes());
    }
    pub fn read_one<'a>(bytes: &'a [u8], offset: &'_ mut usize) -> ReadResult<&'a str> {
        let len = decode_prefix_varint(bytes, offset)? as usize;
        let utf8 = read_bytes(bytes, len, offset)?;
        Ok(std::str::from_utf8(utf8)?)
    }
}

impl BatchData for String {
    fn read_batch(_bytes: &[u8]) -> ReadResult<Vec<Self>> {
        unreachable!();
    }
    fn write_batch(_items: &[Self], _bytes: &mut Vec<u8>) {
        unreachable!();
    }
    fn write_one(_item: Self, _bytes: &mut Vec<u8>) {
        unreachable!();
    }
    fn read_one(bytes: &[u8], offset: &mut usize) -> ReadResult<Self> {
        Ok(Str::read_one(bytes, offset)?.to_owned())
    }
}


#[derive(Default)]
pub struct StringWriter<'a> {
    values: Vec<&'a str>
}

impl<'a> Writable<'a> for String {
    type Writer = StringWriter<'a>;
} 

impl<'a> Writer<'a> for StringWriter<'a> {
    type Write=String;

    fn write<'b : 'a>(&mut self, value: &'b Self::Write) {
        self.values.push(value.as_str());
    }
    fn flush<ParentBranch: StaticBranch>(self, _branch: ParentBranch, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) {
        // See also {2d1e8f90-c77d-488c-a41f-ce0fe3368712}
        PrimitiveId::String.write(bytes);

        if ParentBranch::in_array_context() {
            let start = bytes.len();
            for s in self.values {
                Str::write_one(s, bytes)
            }
            let len = bytes.len() - start;
            lens.push(len);
        } else {
            let Self { mut values, .. } = self;
            // TODO: This may be 0 for Object
            assert_eq!(values.len(), 1);
            let value = values.pop().unwrap();
            Str::write_one(value, bytes);
        }
    }
    fn new() -> Self {
        Default::default()
    }
}


pub struct StringReader {
    values: IntoIter<String>
}

impl Readable for String {
    type Reader=StringReader;
}

impl Reader for StringReader {
    type Read=String;
    // TODO: It would be nice to be able to keep reference to the original byte array, especially for reading strings.
    // I think that may require GAT though the way things are setup so come back to this later.
    fn new<ParentBranch: StaticBranch>(sticks: DynBranch<'_>, _branch: ParentBranch) -> ReadResult<Self> {
        match sticks {
            DynBranch::String(items) => {
                let values = match items {
                    OneOrMany::One(one) => vec![one],
                    OneOrMany::Many(bytes) => {
                        let mut offset = 0;
                        let mut result = Vec::new();
                        while offset < bytes.len() {
                            result.push(Str::read_one(bytes, &mut offset)?.to_owned())
                        };
                        result
                    }
                };
                Ok(Self {
                    values: values.into_iter(),
                })
            },
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read(&mut self) -> ReadResult<Self::Read> {
        self.values.next().ok_or(ReadError::InvalidFormat)
    }
}

