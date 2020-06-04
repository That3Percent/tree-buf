use std::fmt::{Debug, Display, Formatter};

#[cfg(feature = "decode")]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DecodeError {
    SchemaMismatch,
    // Had these broken up at one point into different kinds of errors,
    // like reading past the end of the file or having an invalid type id.
    // In practice, a corrupt file will just trigger one of the problem at random
    // so it's not useful information. Removing the variants makes it so that at
    // least for now we can avoid boxing.
    InvalidFormat,
}

use coercible_errors::coercible_errors;
coercible_errors!(DecodeError);

#[cfg(feature = "decode")]
impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            DecodeError::SchemaMismatch => f.write_str("The expected schema did not match that in the document."),
            DecodeError::InvalidFormat => f.write_str("The format was not a valid Tree-Buf"),
        }
    }
}

#[cfg(feature = "decode")]
impl std::error::Error for DecodeError {}

#[cfg(feature = "decode")]
impl From<std::str::Utf8Error> for DecodeError {
    fn from(_value: std::str::Utf8Error) -> Self {
        DecodeError::InvalidFormat
    }
}
