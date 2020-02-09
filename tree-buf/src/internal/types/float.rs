use crate::prelude::*;
use std::convert::TryInto;
use std::mem::size_of;
use std::vec::IntoIter;

#[cfg(feature = "write")]
fn write_64(item: f64, bytes: &mut Vec<u8>) {
    let b = item.to_le_bytes();
    bytes.extend_from_slice(&b);
}

#[cfg(feature = "read")]
fn read_64(bytes: &[u8], offset: &mut usize) -> ReadResult<f64> {
    let bytes = read_bytes(size_of::<f64>(), bytes, offset)?;
    // This unwrap is ok, because we just read exactly size_of::<f64> bytes on the line above.
    Ok(f64::from_le_bytes(bytes.try_into().unwrap()))
}

// Interesting reading: https://internals.rust-lang.org/t/tryfrom-for-f64/9793/35
// A useful starting point is that all possible down-cast through up-cast round trips
// must preserve bit-for-bit the original value. That's not quite enough though, since this
// is true for some values due to saturating rounding that one wouldn't want to downcast.
#[cfg(feature = "write")]
impl<'a> Writable<'a> for f64 {
    type WriterArray = Vec<f64>;
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
            // TODO: We can also try downcasting to several int types and
            // f32 if they are exactly representable.
            write_64(value, bytes);
            RootTypeId::F64
        }
    }
}

#[cfg(feature = "read")]
impl Readable for f64 {
    type ReaderArray = IntoIter<f64>;
    fn read(sticks: DynRootBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynRootBranch::Integer(root_integer) => {
                // FIXME: Fast and lose to get refactoring done. Double check here.
                // Also, f64 can express some integers larger than 2^53
                match root_integer {
                    RootInteger::U(u) => {
                        if u < (2 << 52) {
                            Ok(u as f64)
                        } else {
                            Err(ReadError::SchemaMismatch)
                        }
                    }
                    RootInteger::S(s) => {
                        if s < (2 << 52) && s > (-2 << 51) {
                            // FIXME: Made up number
                            Ok(s as f64)
                        } else {
                            Err(ReadError::SchemaMismatch)
                        }
                    }
                }
            }
            DynRootBranch::Float(root_float) => {
                match root_float {
                    RootFloat::F64(v) => Ok(v),
                    RootFloat::NaN => Ok(std::f64::NAN),
                    // This should be safe to cast without loss of information.
                    // Double-check that the meaning of various NaN values
                    // is preserved though (signaling, non-signaling, etc)
                    // https://stackoverflow.com/a/59795029/11837266
                    RootFloat::F32(v) => Ok(v as f64),
                }
            }
            _ => Err(ReadError::SchemaMismatch),
        }
    }
}

#[cfg(feature = "read")]
impl ReaderArray for IntoIter<f64> {
    type Read = f64;
    fn new(sticks: DynArrayBranch<'_>) -> ReadResult<Self> {
        match sticks {
            DynArrayBranch::Float(float) => {
                match float {
                    ArrayFloat::F64(bytes) => {
                        let values = read_all(bytes, read_64)?;
                        Ok(values.into_iter())
                    }
                    // TODO:
                    ArrayFloat::F32(_) => todo!(),
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
impl<'a> WriterArray<'a> for Vec<f64> {
    type Write = f64;
    fn buffer<'b: 'a>(&mut self, value: &'b Self::Write) {
        self.push(*value);
    }
    fn flush(self, bytes: &mut Vec<u8>, lens: &mut Vec<usize>) -> ArrayTypeId {
        let start = bytes.len();
        for item in self {
            write_64(item, bytes);
        }
        lens.push(bytes.len() - start);
        ArrayTypeId::F64
    }
}
