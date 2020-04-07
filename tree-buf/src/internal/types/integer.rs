use crate::internal::encodings::compress;
use crate::internal::encodings::varint::*;
use crate::prelude::*;
use num_traits::{AsPrimitive, Bounded};
use simple_16::compress as compress_simple_16;
use std::any::TypeId;
use std::convert::{TryFrom, TryInto};
use std::mem::transmute;
use std::vec::IntoIter;

#[derive(Copy, Clone)]
struct U0;

impl Bounded for U0 {
    fn min_value() -> Self {
        U0
    }
    fn max_value() -> Self {
        U0
    }
}

fn write_u0<T>(_data: &[T], _max: T, _stream: &impl WriterStream) -> ArrayTypeId {
    unreachable!();
}

macro_rules! impl_lowerable {
    ($Ty:ty, $fn:ident, $Lty:ty, $lfn:ident, ($($lower:ty),*), ($($compressions:ty),+)) => {
        impl TryFrom<$Ty> for U0 {
            type Error=();
            fn try_from(_value: $Ty) -> Result<U0, Self::Error> {
                Err(())
            }
        }
        impl TryFrom<U0> for $Ty {
            type Error=();
            fn try_from(_value: U0) -> Result<$Ty, Self::Error> {
                Err(())
            }
        }
        impl AsPrimitive<U0> for $Ty {
            fn as_(self) -> U0 {
                unreachable!()
            }
        }

        #[cfg(feature = "write")]
        impl<'a> Writable<'a> for $Ty {
            type WriterArray = Vec<$Ty>;
            fn write_root<'b: 'a>(&'b self, stream: &mut impl WriterStream) -> RootTypeId {
                write_root_uint(*self as u64, stream.bytes())
            }
        }

        #[cfg(feature = "write")]
        impl<'a> WriterArray<'a> for Vec<$Ty> {
            type Write = $Ty;
            fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
                self.push(*value);
            }
            fn flush(self, stream: &mut impl WriterStream) -> ArrayTypeId {
                let max = self.iter().max();
                if let Some(max) = max {
                    $fn(&self, *max, stream)
                } else {
                    ArrayTypeId::Void
                }
            }
        }

        #[cfg(feature = "read")]
        impl Readable for $Ty {
            type ReaderArray = IntoIter<$Ty>;
            fn read(sticks: DynRootBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
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

        #[cfg(feature = "read")]
        impl ReaderArray for IntoIter<$Ty> {
            type Read = $Ty;
            fn new(sticks: DynArrayBranch<'_>, _options: &impl DecodeOptions) -> ReadResult<Self> {
                match sticks {
                    // TODO: Support eg: delta/zigzag
                    DynArrayBranch::Integer(array_int) => {
                        let ArrayInteger { bytes, encoding } = array_int;
                        match encoding {
                            ArrayIntegerEncoding::PrefixVarInt => {
                                let v: Vec<$Ty> = read_all(
                                        &bytes,
                                        |bytes, offset| {
                                            let r: $Ty = decode_prefix_varint(bytes, offset)?.try_into().map_err(|_| ReadError::SchemaMismatch)?;
                                            Ok(r)
                                        }
                                )?;
                                Ok(v.into_iter())
                            }
                            ArrayIntegerEncoding::Simple16 => {
                                let mut v = Vec::new();
                                simple_16::decompress(&bytes, &mut v).map_err(|_| ReadError::InvalidFormat(InvalidFormat::DecompressionError))?;
                                let result: Result<Vec<_>, _> = v.into_iter().map(TryInto::<$Ty>::try_into).collect();
                                let v = result.map_err(|_| ReadError::SchemaMismatch)?;
                                Ok(v.into_iter())
                            },
                            ArrayIntegerEncoding::U8 => {
                                let v: Vec<$Ty> = bytes.iter().map(|&b| b.into()).collect();
                                Ok(v.into_iter())
                            }
                        }
                    },
                    // FIXME: This fixes a particular test.
                    // It is unclear if this is canon.
                    // See also: 84d15459-35e4-4f04-896f-0f4ea9ce52a9
                    // TODO: Also apply this to other types
                    DynArrayBranch::Void => {
                        Ok(Vec::new().into_iter())
                    }
                    _ => Err(ReadError::SchemaMismatch),
                }
            }
            fn read_next(&mut self) -> Self::Read {
                self.next().unwrap_or_default()
            }
        }

        #[cfg(feature = "write")]
        fn $fn<T: Copy + std::fmt::Debug + AsPrimitive<$Ty> + AsPrimitive<U0> + AsPrimitive<u8> + AsPrimitive<$Lty> $(+ AsPrimitive<$lower>)*>
            (data: &[T], max: T, stream: &mut impl WriterStream) -> ArrayTypeId {
            let lower_max: Result<$Ty, _> = <$Lty as Bounded>::max_value().try_into();

            if let Ok(lower_max) = lower_max {
                if lower_max >= max.as_() {
                    return $lfn(data, max, stream)
                }
            }

            fn write_inner(data: &[$Ty], stream: &mut impl WriterStream) -> ArrayTypeId {
                // TODO: (Performance) Remove allocations
                let compressors: Vec<Box<dyn Compressor<$Ty>>> = vec![
                    $(Box::new(<$compressions>::new())),+
                ];
                stream.write_with_len(|stream|
                    compress(data, stream.bytes(), &compressors[..])
                )
            }

            // Convert data to as<T>, using a transmute if that's already correct
            if TypeId::of::<$Ty>() == TypeId::of::<T>() {
                // Safety - this is a unit conversion.
                let data = unsafe { transmute(data) };
                write_inner(data, stream)
            } else {
                // TODO: Use second-stack
                let mut v = Vec::new();
                for item in data.iter() {
                    v.push(item.as_());
                }
                write_inner(&v, stream)
            }
        }
    };
}

// TODO: This does all kinds of silly things. Eg: Perhaps we have u32 and simple16 is best.
// This may downcast to u16 then back up to u32. I'm afraid the final result is just going to
// be a bunch of hairy special code for each type with no generality.
//
// Broadly we only want to downcast if it allows for some other kind of compressor to be used.

// Type, array writer, next lower, next lower writer, non-inferred lowers
impl_lowerable!(u64, write_u64, u32, write_u32, (u16), (PrefixVarIntCompressor::<u64>));
impl_lowerable!(u32, write_u32, u16, write_u16, (), (Simple16Compressor::<u32>, PrefixVarIntCompressor::<u32>)); // TODO: Consider replacing PrefixVarInt at this level with Fixed.
impl_lowerable!(u16, write_u16, u8, write_u8, (), (Simple16Compressor::<u16>, PrefixVarIntCompressor::<u16>));
impl_lowerable!(u8, write_u8, U0, write_u0, (), (Simple16Compressor::<u8>, BytesCompressor));

#[cfg(feature = "write")]
fn write_root_uint(value: u64, bytes: &mut Vec<u8>) -> RootTypeId {
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

struct PrefixVarIntCompressor<T> {
    _marker: Unowned<T>,
}

impl<T: Into<u64> + Copy> PrefixVarIntCompressor<T> {
    pub fn new() -> Self {
        Self { _marker: Unowned::new() }
    }
}

impl<T: Into<u64> + Copy> Compressor<T> for PrefixVarIntCompressor<T> {
    fn fast_size_for(&self, data: &[T]) -> Option<usize> {
        let mut size = 0;
        for item in data {
            size += size_for_varint((*item).into());
        }
        Some(size)
    }
    fn compress(&self, data: &[T], bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()> {
        for item in data {
            encode_prefix_varint((*item).into(), bytes);
        }
        Ok(ArrayTypeId::IntPrefixVar)
    }
}

struct Simple16Compressor<T> {
    _marker: Unowned<T>,
}

impl<T: Into<u32> + Copy> Simple16Compressor<T> {
    pub fn new() -> Self {
        Self { _marker: Unowned::new() }
    }
}

impl<T: Into<u32> + Copy> Compressor<T> for Simple16Compressor<T> {
    fn compress(&self, data: &[T], bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()> {
        // TODO: (Performance) Use second-stack.
        // TODO: (Performance) This just copies to another Vec in the case where T is u32
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

struct BytesCompressor;
impl BytesCompressor {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor<u8> for BytesCompressor {
    fn compress(&self, data: &[u8], bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()> {
        bytes.extend_from_slice(data);
        Ok(ArrayTypeId::U8)
    }
    fn fast_size_for(&self, data: &[u8]) -> Option<usize> {
        Some(data.len())
    }
}

// TODO: Bitpacking https://crates.io/crates/bitpacking
// TODO: Mayda https://crates.io/crates/mayda
// TODO: https://lemire.me/blog/2012/09/12/fast-integer-compression-decoding-billions-of-integers-per-second/
