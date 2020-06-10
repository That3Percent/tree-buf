///! This is a namespace to make specific names available to macros,
///! and traits necessary for encode/decode that must be public to
///! be used to be found somehow, but hiding it behind a namespace so that
///! the implementation details cannot be relied upon.
#[cfg(feature = "decode")]
pub mod branch;
#[macro_use]
pub mod encodings;
pub mod buffer;
pub mod chunk;
pub mod encoder_decoder;
pub mod error;
pub mod markers;
pub mod options;
pub mod parallel;
pub mod rust_std;
pub mod types;

pub use {branch::*, buffer::*, encoder_decoder::*, encodings::*, options::*, parallel::*, rust_std::*, types::*};

pub(crate) use markers::*;

pub use crate::profile;
