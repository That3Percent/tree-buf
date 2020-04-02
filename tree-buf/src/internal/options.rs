// https://docs.rs/ndarray-zfp-rs/0.1.0/ndarray_zfp_rs/trait.Zfp.htmls

// TODO: This would be the place to specify in-place padded/aligned encoding when desired.
// I'm not sure that's as useful, since it moves out of where tree-buf competes into different
// territory (eg: FlatBuffers). Though Flatbuffers by way of example doesn't allow for in-place encoding,
//

macro_rules! options {
    ($Options:ident, $Default:ident, $Override:ident, $Hierarchy:ident, {$($name:ident: $T:ty = $fallback:expr),*}) => {
        pub trait $Options: Send + Sync {
            $(
                #[inline(always)]
                fn $name(&self) -> $T { $fallback }
            )*
        }

        pub struct $Default;
        impl $Options for $Default { }

        pub trait $Override: Send + Sync {
            $(
                #[inline(always)]
                fn $name(&self) -> Option<$T> { None }
            )*
        }

        impl<T0: $Options, T1: $Override> $Options for $Hierarchy<T0, T1> {
            $(
                #[inline(always)]
                fn $name(&self) -> $T {
                    self.overrides.$name().unwrap_or_else(|| self.fallback.$name())
                }
            )*
        }

        struct $Hierarchy<T0, T1> {
            fallback: T0,
            overrides: T1,
        }
    };
}

options!(EncodeOptions, EncodeOptionsDefault, EncodeOptionsOverride, EncodeOptionsHierarchy, {
    lossy_float_tolerance: Option<i32> = None
});

options!(DecodeOptions, DecodeOptionsDefault, DecodeOptionsOverride, DecodeOptionsHierarchy, {
    parallel: bool = true
});

pub struct EnableParallel;
impl DecodeOptionsOverride for EnableParallel {
    #[inline(always)]
    fn parallel(&self) -> Option<bool> {
        Some(true)
    }
}

pub struct DisableParallel;
impl DecodeOptionsOverride for DisableParallel {
    #[inline(always)]
    fn parallel(&self) -> Option<bool> {
        Some(false)
    }
}

pub struct LosslessFloat;
impl EncodeOptionsOverride for LosslessFloat {
    #[inline(always)]
    fn lossy_float_tolerance(&self) -> Option<Option<i32>> {
        Some(None)
    }
}

pub struct LossyFloatTolerance(pub i32);
impl EncodeOptionsOverride for LossyFloatTolerance {
    #[inline(always)]
    fn lossy_float_tolerance(&self) -> Option<Option<i32>> {
        Some(Some(self.0))
    }
}

// TODO: Move the remainder here into the macro
pub fn override_encode_options(options: impl EncodeOptions, overrides: impl EncodeOptionsOverride) -> impl EncodeOptions {
    EncodeOptionsHierarchy { fallback: options, overrides }
}

#[macro_export]
macro_rules! encode_options {
    ($($opts:expr),*) => {
        {
            let options = $crate::options::EncodeOptionsDefault;
            $(
                let options = $crate::options::override_encode_options(options, $opts);
            )*
            options
        }
    }
}

pub fn override_decode_options(options: impl DecodeOptions, overrides: impl DecodeOptionsOverride) -> impl DecodeOptions {
    DecodeOptionsHierarchy { fallback: options, overrides }
}

#[macro_export]
macro_rules! decode_options {
    ($($opts:expr),*) => {
        {
            let options = $crate::options::DecodeOptionsDefault;
            $(
                let options = $crate::options::override_decode_options(options, $opts);
            )*
            options
        }
    }
}
