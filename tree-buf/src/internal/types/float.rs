// Promising Compressors:
// Gorilla - https://crates.io/crates/tsz   http://www.vldb.org/pvldb/vol8/p1816-teller.pdf
// FPC
// Akamuli - https://akumuli.org/akumuli/2017/02/05/compression_part2/
// ? http://blog.omega-prime.co.uk/2016/01/25/compression-of-floating-point-timeseries/
// https://www.cs.unc.edu/~isenburg/lcpfpv/
// dfcm - https://userweb.cs.txstate.edu/~mb92/papers/dcc06.pdf

// TODO: Lowerings
// Interesting reading: https://internals.rust-lang.org/t/tryfrom-for-f64/9793/35
// A useful starting point is that all possible down-cast through up-cast round trips
// must preserve bit-for-bit the original value. That's not quite enough though, since this
// is true for some values due to saturating rounding that one wouldn't want to downcast.
// https://floating-point-gui.de/formats/fp/
// f64 -> u64
// f64 -> f32
// f32 -> u32

// TODO: More compressors

macro_rules! impl_float {
    ($T:ident, $id:ident) => {
        //use crate::encodings::zfp;
        use crate::prelude::*;
        use num_traits::AsPrimitive as _;
        use std::convert::TryInto;
        use std::mem::size_of;
        use std::vec::IntoIter;

        // TODO: Check for lowering - f64 -> f63
        #[cfg(feature = "encode")]
        fn encode_item(item: $T, bytes: &mut Vec<u8>) {
            let b = item.to_le_bytes();
            bytes.extend_from_slice(&b);
        }

        #[cfg(feature = "decode")]
        pub(super) fn decode_item(bytes: &[u8], offset: &mut usize) -> DecodeResult<$T> {
            let bytes = decode_bytes(size_of::<$T>(), bytes, offset)?;
            // This unwrap is ok, because we just read exactly size_of::<T> bytes on the line above.
            Ok(<$T>::from_le_bytes(bytes.try_into().unwrap()))
        }

        #[cfg(feature = "encode")]
        impl Encodable for $T {
            type EncoderArray = Vec<$T>;
            fn encode_root<O: EncodeOptions>(&self, stream: &mut EncoderStream<'_, O>) -> RootTypeId {
                let value = *self;

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
                    encode_item(value, stream.bytes);
                    RootTypeId::$id
                }
            }
        }

        #[cfg(feature = "decode")]
        impl Decodable for $T {
            type DecoderArray = IntoIter<$T>;
            fn decode(sticks: DynRootBranch<'_>, _options: &impl DecodeOptions) -> DecodeResult<Self> {
                profile_method!(decode);
                match sticks {
                    DynRootBranch::Integer(root_integer) => {
                        // FIXME: Fast and lose to get refactoring done. Double check here.
                        // Also, float can express some (but not all) integers larger than MAX_SAFE_INT
                        match root_integer {
                            RootInteger::U(u) => {
                                if u < (2 << std::$T::MANTISSA_DIGITS) {
                                    Ok(u as $T)
                                } else {
                                    Err(DecodeError::SchemaMismatch)
                                }
                            }
                            RootInteger::S(s) => {
                                if s < (2 << std::$T::MANTISSA_DIGITS) && s > (-2 << (std::$T::MANTISSA_DIGITS - 1)) {
                                    // FIXME: Made up number
                                    Ok(s as $T)
                                } else {
                                    Err(DecodeError::SchemaMismatch)
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
                    _ => Err(DecodeError::SchemaMismatch),
                }
            }
        }

        #[cfg(feature = "decode")]
        impl InfallibleDecoderArray for IntoIter<$T> {
            type Decode = $T;
            fn new_infallible(sticks: DynArrayBranch<'_>, _options: &impl DecodeOptions) -> DecodeResult<Self> {
                profile_method!(new_infallibe);

                match sticks {
                    DynArrayBranch::Float(float) => {
                        match float {
                            ArrayFloat::F64(bytes) => {
                                profile_section!(array_f64);

                                // FIXME: Should do schema mismatch for f32 -> f64
                                let values = decode_all(&bytes, |bytes, offset| Ok(super::_f64::decode_item(bytes, offset)?.as_()))?;
                                Ok(values.into_iter())
                            }
                            ArrayFloat::F32(bytes) => {
                                profile_section!(array_f32);

                                let values = decode_all(&bytes, |bytes, offset| Ok(super::_f32::decode_item(bytes, offset)?.as_()))?;
                                Ok(values.into_iter())
                            }
                            ArrayFloat::DoubleGorilla(bytes) => gorilla::decompress::<$T>(&bytes).map(|f| f.into_iter()),
                            /*
                            ArrayFloat::Zfp32(bytes) => {
                                // FIXME: This is likely a bug switching between 32 and 64 might just get garbage data out
                                let values = zfp::decompress::<f32>(&bytes)?;
                                // TODO: (Performance) unnecessary copy in some cases
                                let values: Vec<_> = values.iter().map(|v| v.as_()).collect();
                                Ok(values.into_iter())
                            }
                            ArrayFloat::Zfp64(bytes) => {
                                let values = zfp::decompress::<f64>(&bytes)?;
                                // TODO: (Performance) unnecessary copy in some cases
                                let values: Vec<_> = values.iter().map(|v| v.as_()).collect();
                                Ok(values.into_iter())
                            }
                            */
                            ArrayFloat::Zfp32(_bytes) => unimplemented!("zfp32"),
                            ArrayFloat::Zfp64(_bytes) => unimplemented!("zfp64"),
                        }
                    }
                    // TODO: There are some conversions that are infallable.
                    // Eg: Simple16.
                    _ => Err(DecodeError::SchemaMismatch),
                }
            }
            fn decode_next_infallible(&mut self) -> Self::Decode {
                self.next().unwrap_or_default()
            }
        }

        #[cfg(feature = "encode")]
        impl EncoderArray<$T> for Vec<$T> {
            fn buffer_one<'a, 'b: 'a>(&'a mut self, value: &'b $T) {
                self.push(*value);
            }
            fn buffer_many<'a, 'b: 'a>(&'a mut self, values: &'b [$T]) {
                profile_method!(buffer_many);
                self.extend_from_slice(values);
            }
            fn encode_all<O: EncodeOptions>(values: &[$T], stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
                profile_method!(encode_all);

                // See also 558c24b8-dc75-4f08-8ea2-0f839af4da2e
                let compressors = (
                    Fixed, //Zfp,
                    Gorilla,
                );

                compress(values, stream, &compressors)
            }
            fn flush<O: EncodeOptions>(self, stream: &mut EncoderStream<'_, O>) -> ArrayTypeId {
                Self::encode_all(&self[..], stream)
            }
        }

        impl PrimitiveEncoderArray<$T> for Vec<$T> {
            fn fast_size_for_all<O: EncodeOptions>(values: &[$T], options: &O) -> usize {
                // See also 558c24b8-dc75-4f08-8ea2-0f839af4da2e
                let compressors = (
                    Fixed, //Zfp,
                    Gorilla,
                );
                fast_size_for(values, &compressors, options)
            }
        }

        struct Fixed;
        impl Compressor<$T> for Fixed {
            fn fast_size_for<O: EncodeOptions>(&self, data: &[$T], _options: &O) -> Result<usize, ()> {
                let arr_size = size_of::<$T>() * data.len();
                Ok(arr_size + size_for_varint(arr_size as u64))
            }
            fn compress<O: EncodeOptions>(&self, data: &[$T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
                profile_method!(compress);
                stream.encode_with_len(|stream| {
                    for item in data {
                        encode_item(*item, &mut stream.bytes);
                    }
                });
                Ok(ArrayTypeId::$id)
            }
        }

        struct Gorilla;
        impl Compressor<$T> for Gorilla {
            fn fast_size_for<O: EncodeOptions>(&self, data: &[$T], options: &O) -> Result<usize, ()> {
                profile_method!(fast_size_for);

                if let Some(tolerance) = options.lossy_float_tolerance() {
                    // TODO: This is a hack (albeit a surprisingly effective one) to get lossy compression
                    // before a real lossy compressor (Eg: fzip) is used.
                    let multiplier = (2.0 as $T).powi(tolerance * -1);
                    let data = data.iter().map(|f| ((f * multiplier).floor() / multiplier) as f64);
                    gorilla::size_for(data)
                } else {
                    let data = data.iter().map(|f| *f as f64);
                    gorilla::size_for(data)
                }
            }

            fn compress<O: EncodeOptions>(&self, data: &[$T], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
                profile_method!(compress);

                stream.encode_with_len(|stream| {
                    if let Some(tolerance) = stream.options.lossy_float_tolerance() {
                        // TODO: This is a hack (albeit a surprisingly effective one) to get lossy compression
                        // before a real lossy compressor (Eg: fzip) is used.
                        let multiplier = (2.0 as $T).powi(tolerance * -1);
                        let data = data.iter().map(|f| ((f * multiplier).floor() / multiplier) as f64);
                        gorilla::compress(data, stream.bytes)
                    } else {
                        let data = data.iter().map(|f| *f as f64);
                        gorilla::compress(data, stream.bytes)
                    }
                })
            }
        }
    };
}

/*
struct Zfp64 {
    tolerance: Option<i32>,
}
impl Compressor<f64> for Zfp64 {
    fn compress<O: EncodeOptions>(&self, data: &[f64], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        profile_method!(compress);
        stream.encode_with_len(|stream| zfp::compress(data, &mut stream.bytes, self.tolerance));
    }
}

struct Zfp32 {
    tolerance: Option<i32>,
}
impl Compressor<f32> for Zfp32 {
    fn compress<O: EncodeOptions>(&self, data: &[f32], stream: &mut EncoderStream<'_, O>) -> Result<ArrayTypeId, ()> {
        profile_method!(compress);
        stream.encode_with_len(|stream| zfp::compress(data, bytes, self.tolerance));
    }
}
*/

mod _f64 {
    impl_float!(f64, F64);
}
mod _f32 {
    impl_float!(f32, F32);
}
