use crate::encodings::varint::*;
use crate::prelude::*;
use std::vec::IntoIter;

fn write_uint(value: u64, bytes: &mut Vec<u8>) -> RootTypeId {
    let le = value.to_le_bytes();
    match value {
        0 => RootTypeId::Zero,
        1 => RootTypeId::One,
        2..=255 => {
            bytes.push(le[0]);
            RootTypeId::IntU8
        }
        256..=65535 => {
            bytes.extend_from_slice(&le[..2]);
            RootTypeId::IntU16
        }
        65536..=16777215 => {
            bytes.extend_from_slice(&le[..3]);
            RootTypeId::IntU24
        }
        16777216..=4294967295 => {
            bytes.extend_from_slice(&le[..4]);
            RootTypeId::IntU32
        }
        4294967296..=1099511627775 => {
            bytes.extend_from_slice(&le[..5]);
            RootTypeId::IntU40
        }
        1099511627776..=281474976710655 => {
            bytes.extend_from_slice(&le[..6]);
            RootTypeId::IntU48
        }
        281474976710656..=72057594037927936 => {
            bytes.extend_from_slice(&le[..7]);
            RootTypeId::IntU56
        }
        _ => {
            bytes.extend_from_slice(&le);
            RootTypeId::IntU64
        }
    }
}

impl<'a> Writable<'a> for u64 {
    type WriterArray = Vec<u64>;
    fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, _lens: &mut Vec<usize>) -> RootTypeId {
        write_uint(*value, bytes)
    }
}

impl Readable for u64 {
    type ReaderArray = IntoIter<u64>;
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynRootBranch::Integer(root_int) => {
                match root_int {
                    RootInteger::U(v) => Ok(v),
                    // TODO: Try to convert if allowed
                    _ => Err(ReadError::SchemaMismatch),
                }
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

impl<'a> WriterArray<'a> for Vec<u64> {
    type Write = u64;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.push(*value);
    }
    fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId {
        let start = bytes.len();
        for item in self {
            encode_prefix_varint(item, bytes);
        }
        lens.push(bytes.len() - start);
        ArrayTypeId::IntPrefixVar
    }
}

impl ReaderArray for IntoIter<u64> {
    type Read = u64;
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
        match sticks {
            // TODO: Support eg: delta/zigzag
            DynArrayBranch::Integer(array_int) => {
                let ArrayInteger { bytes, encoding } = array_int;
                match encoding {
                    ArrayIntegerEncoding::PrefixVarInt => {
                        let v = read_all(bytes, decode_prefix_varint)?;
                        Ok(v.into_iter())
                    }
                    _ => todo!(),
                }
            }
            // TODO: Simple16 is infallable.
            _ => Err(ReadError::SchemaMismatch),
        }
    }
    fn read_next(&mut self) -> ReadResult<Self::Read> {
        self.next().ok_or_else(|| ReadError::InvalidFormat(InvalidFormat::ShortArray))
    }
}
