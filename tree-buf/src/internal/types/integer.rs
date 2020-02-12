use crate::internal::encodings::varint::*;
use crate::internal::encodings::compress;
use crate::prelude::*;
use std::vec::IntoIter;
use std::convert::TryInto;
use simple_16::compress as compress_simple_16;

#[cfg(feature = "write")]
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

// ($count:expr, $trid:expr, $taid:expr, $($ts:ident, $ti:tt,)+)

macro_rules! impl_uint {
    ($t:ty) => {
        #[cfg(feature = "write")]
        impl<'a> Writable<'a> for $t {
            type WriterArray = Vec<$t>;
            fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, _lens: &mut Vec<usize>) -> RootTypeId {
                write_uint(*value as u64, bytes)
            }
        }

        #[cfg(feature = "read")]
        impl Readable for $t {
            type ReaderArray = IntoIter<$t>;
            fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
                match sticks {
                    DynRootBranch::Integer(root_int) => {
                        match root_int {
                            RootInteger::U(v) => v.try_into().map_err(|_| ReadError::SchemaMismatch),
                            _ => Err(ReadError::SchemaMismatch),
                        }
                    }
                    _ => Err(ReadError::SchemaMismatch),
                }
            }
        }
    };
}


impl_uint!(u64);

struct PrefixVarIntCompressor<T> {
    _marker: std::marker::PhantomData<*const T>
}

impl<T: Into<u64> + Copy> PrefixVarIntCompressor<T> {
    pub fn new() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
}

impl<T: Into<u64> + Copy> Compressor<'_> for PrefixVarIntCompressor<T> {
    type Data = T;
    fn fast_size_for(&self, data: &[Self::Data]) -> Option<usize> {
        let mut size = 0;
        for item in data {
            size += size_for_varint((*item).into());
        }
        Some(size)
    }
    fn compress(&self, data: &[Self::Data], bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()> {
        for item in data {
            encode_prefix_varint((*item).into(), bytes);
        }
        Ok(ArrayTypeId::IntPrefixVar)
    }
}

struct Simple16Compressor<T> {
    _marker: std::marker::PhantomData<*const T>
}

impl<T: TryInto<u64> + Copy> Simple16Compressor<T> {
    pub fn new() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
}

impl<T: TryInto<u32> + Copy> Compressor<'_> for Simple16Compressor<T> {
    type Data = T;
    
    fn compress(&self, data: &[Self::Data], bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()> {
        // TODO: Use second-stack.
        // TODO: This just copies to another Vec in the case where T is u64
        let mut v = Vec::new();
        for item in data {
            let item = *item;
            let item = item.try_into().map_err(|_| ())?;
            v.push(item);
        }

        compress_simple_16(&v, bytes).map_err(|_| ())?;

        Ok(ArrayTypeId::IntSimple16)
    }
}


#[cfg(feature = "write")]
impl<'a> WriterArray<'a> for Vec<u64> {
    type Write = u64;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.push(*value);
    }
    fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId {
        let start = bytes.len();
        // TODO: Remove allocations
        let compressors: Vec<Box<dyn Compressor<Data=u64>>> = vec![
            Box::new(PrefixVarIntCompressor::new()),
            Box::new(Simple16Compressor::new()),
        ];
        let type_id = compress(&self, bytes, &compressors);
        lens.push(bytes.len() - start);
        type_id
    }
}

#[cfg(feature = "read")]
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
                    ArrayIntegerEncoding::Simple16 => {
                        let mut v = Vec::new();
                        simple_16::decompress(bytes, &mut v).map_err(|_| ReadError::InvalidFormat(InvalidFormat::DecompressionError))?;
                        let v: Vec<_> = v.into_iter().map(Into::<u64>::into).collect();
                        Ok(v.into_iter())
                    }
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


// TODO: Bitpacking https://crates.io/crates/bitpacking
// TODO: Mayda https://crates.io/crates/mayda