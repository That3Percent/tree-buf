use crate::prelude::*;
use std::convert::TryInto;
use std::mem::size_of;
use std::vec::IntoIter;
use num_traits::AsPrimitive as _;

// TODO: Lowerings
// Interesting reading: https://internals.rust-lang.org/t/tryfrom-for-f64/9793/35
// A useful starting point is that all possible down-cast through up-cast round trips
// must preserve bit-for-bit the original value. That's not quite enough though, since this
// is true for some values due to saturating rounding that one wouldn't want to downcast.
// https://floating-point-gui.de/formats/fp/
// f64 -> u64
// f64 -> f32
// f32 -> u32

macro_rules! impl_float {
    ($T:ident, $write_item:ident, $read_item:ident, $id:ident) => {
        // TODO: Check for lowering - f64 -> f63
        #[cfg(feature = "write")]
        fn $write_item(item: $T, bytes: &mut Vec<u8>) {
            let b = item.to_le_bytes();
            bytes.extend_from_slice(&b);
        }

        #[cfg(feature = "read")]
        fn $read_item(bytes: &[u8], offset: &mut usize) -> ReadResult<$T> {
            let bytes = read_bytes(size_of::<$T>(), bytes, offset)?;
            // This unwrap is ok, because we just read exactly size_of::<T> bytes on the line above.
            Ok(<$T>::from_le_bytes(bytes.try_into().unwrap()))
        }

        
        #[cfg(feature = "write")]
        impl<'a> Writable<'a> for $T {
            type WriterArray = Vec<$T>;
            fn write_root<'b: 'a>(value: &'b Self, bytes: &mut Vec<u8>, _lens: &mut Vec<usize>) -> RootTypeId {
                let value = *value;

                // Check for positive sign so that -0.0 goes through
                // the unhappy path but round-trips bit-for-bit
                if value == 0.0 && value.is_sign_positive() {
                    RootTypeId::Zero
                } else if value == 1.0 {
                    RootTypeId::One
                } else if value == -1.0 {
                    RootTypeId::NegOne
                } else if value.is_nan() {
                    // FIXME: Check for canonical NaN,
                    // so that other NaN round trip bit-for-bit
                    RootTypeId::NaN
                } else {
                    $write_item(value, bytes);
                    RootTypeId::$id
                }
            }
        }

        
        #[cfg(feature = "read")]
        impl Readable for $T {
            type ReaderArray = IntoIter<$T>;
            fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
                match sticks {
                    DynRootBranch::Integer(root_integer) => {
                        // FIXME: Fast and lose to get refactoring done. Double check here.
                        // Also, float can express some (but not all) integers larger than MAX_SAFE_INT
                        match root_integer {
                            RootInteger::U(u) => {
                                if u < (2 << std::$T::MANTISSA_DIGITS) {
                                    Ok(u as $T)
                                } else {
                                    Err(ReadError::SchemaMismatch)
                                }
                            }
                            RootInteger::S(s) => {
                                if s < (2 << std::$T::MANTISSA_DIGITS) && s > (-2 << (std::$T::MANTISSA_DIGITS - 1)) {
                                    // FIXME: Made up number
                                    Ok(s as $T)
                                } else {
                                    Err(ReadError::SchemaMismatch)
                                }
                            }
                        }
                    }
                    DynRootBranch::Float(root_float) => {
                        match root_float {
                            // FIXME: Macro here - should be schema mismatch for f64 -> f32
                            RootFloat::F64(v) => Ok(v as $T),
                            RootFloat::NaN => Ok(std::$T::NAN),
                            // This should be safe to cast without loss of information.
                            // Double-check that the meaning of various NaN values
                            // is preserved though (signaling, non-signaling, etc)
                            // https://stackoverflow.com/a/59795029/11837266
                            RootFloat::F32(v) => Ok(v as $T),
                        }
                    }
                    _ => Err(ReadError::SchemaMismatch),
                }
            }
        }

        
        #[cfg(feature = "read")]
        impl ReaderArray for IntoIter<$T> {
            type Read = $T;
            fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
                match sticks {
                    DynArrayBranch::Float(float) => {
                        match float {
                            ArrayFloat::F64(bytes) => {
                                // FIXME: Should do schema mismatch for f32 -> f64
                                let values = read_all(bytes, |bytes, offset| Ok(read_64(bytes, offset)?.as_()))?;
                                Ok(values.into_iter())
                            }
                            ArrayFloat::F32(bytes) => {
                                let values = read_all(bytes, |bytes, offset| Ok(read_32(bytes, offset)?.as_()))?;
                                Ok(values.into_iter())
                            },
                        }
                    }
                    // TODO: There are some conversions that are infallable.
                    // Eg: Simple16.
                    _ => Err(ReadError::SchemaMismatch),
                }
            }
            fn read_next(&mut self) -> ReadResult<Self::Read> {
                self.next().ok_or_else(|| ReadError::InvalidFormat(InvalidFormat::ShortArray))
            }
        }

        #[cfg(feature = "write")]
        impl<'a> WriterArray<'a> for Vec<$T> {
            type Write = $T;
            fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
                self.push(*value);
            }
            fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId {
                let start = bytes.len();
                for item in self {
                    $write_item(item, bytes);
                }
                lens.push(bytes.len() - start);
                ArrayTypeId::$id
            }
        }
    };
}

impl_float!(f64, write_64, read_64, F64);
impl_float!(f32, write_32, read_32, F32);

