///! This is a namespace to make specific names available to macros,
///! and traits necessary for reading/writing that must be public to
///! be used to be found somehow, but hiding it behind a namespace so that
///! the implementation details cannot be relied upon.

pub mod branch;
pub mod primitive;
pub mod reader_writer;
pub mod error;
pub mod missing;
pub(crate) mod encodings;
pub mod types;

pub use {
    reader_writer::{Readable, Reader, Writable, Writer},
    primitive::*,
    branch::*,
    encodings::*,
    types::*,
};