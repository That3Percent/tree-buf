// https://docs.rs/ndarray-zfp-rs/0.1.0/ndarray_zfp_rs/trait.Zfp.htmls

// TODO: This would be the place to specify in-place padded/aligned encoding when desired.
// I'm not sure that's as useful, since it moves out of where tree-buf competes into different
// territory (eg: FlatBuffers). Though Flatbuffers by way of example doesn't allow for in-place encoding,
// 

macro_rules! options {
    ($(($name:ident, $T:ty, $fallback:expr, $over:ident)),*) => {
        pub trait EncodeOptions {
            $(
                #[inline(always)]
                fn $name(&self) -> $T { $fallback }
            )*
        }

        pub struct DefaultEncodeOptions;
        impl EncodeOptions for DefaultEncodeOptions { }

        pub trait EncodeOptionsOverride {
            $(
                #[inline(always)]
                fn $over(&self) -> Option<$T> { None }
            )*
        }

        impl<T0: EncodeOptions, T1: EncodeOptionsOverride> EncodeOptions for EncodeOptionsHierarchy<T0, T1> {
            $(
                #[inline(always)]
                fn $name(&self) -> $T {
                    self.overrides.$over().unwrap_or_else(|| self.fallback.$name())
                }
            )*
        }
    };
}

options!((lossy_float_tolerance, Option<f64>, None, override_lossy_float_tolerance));

struct EncodeOptionsHierarchy<T0, T1> {
    fallback: T0,
    overrides: T1,
}

pub struct LosslessFloat;
impl EncodeOptionsOverride for LosslessFloat {
    fn override_lossy_float_tolerance(&self) -> Option<Option<f64>> {
        Some(None)
    }
}

pub struct LossyFloatTolerance(pub f64);
impl EncodeOptionsOverride for LossyFloatTolerance {
    fn override_lossy_float_tolerance(&self) -> Option<Option<f64>> {
        Some(Some(self.0))
    }
}

pub fn override_encode_options(options: impl EncodeOptions, overrides: impl EncodeOptionsOverride) -> impl EncodeOptions {
    EncodeOptionsHierarchy { fallback: options, overrides }
}
