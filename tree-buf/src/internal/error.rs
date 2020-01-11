use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, PartialEq)]
pub enum ReadError {
    SchemaMismatch,
    InvalidFormat,
}

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        use ReadError::*;
        match self {
            SchemaMismatch => f.write_str("The expected schema did not match that in the file"),
            InvalidFormat => f.write_str("The data is not a valid Tree-Buf"),
        }
    }
}

impl std::error::Error for ReadError {}


impl From<std::str::Utf8Error> for ReadError {
    fn from(_: std::str::Utf8Error) -> Self {
        ReadError::InvalidFormat
    }
}