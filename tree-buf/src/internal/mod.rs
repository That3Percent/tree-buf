///! This is a namespace to make specific names available to macros,
///! and traits necessary for reading/writing that must be public to
///! be used to be found somehow, but hiding it behind a namespace so that
///! the implementation details cannot be relied upon.
#[cfg(feature = "read")]
pub mod branch;
#[macro_use]
pub mod encodings;
pub mod error;
pub mod options;
pub mod parallel;
pub mod reader_writer;
pub mod rust_std;
pub mod types;
pub mod markers;

pub use {
    branch::*,
    encodings::*,
    options::*,
    parallel::*,
    reader_writer::*,
    rust_std::*,
    types::*,
};

pub(crate) use markers::*;

pub use crate::profile;