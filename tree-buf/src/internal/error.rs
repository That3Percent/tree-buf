use std::fmt::{Debug, Display, Formatter};

#[cfg(feature = "read")]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ReadError {
    SchemaMismatch,
    // Had these broken up at one point into different kinds of errors,
    // like reading past the end of the file or having an invalid type id.
    // In practice, a corrupt file will just trigger one of the problem at random
    // so it's not useful information. Removing the variants makes it so that at
    // least for now we can avoid boxing.
    InvalidFormat,
}

use coercible_errors::coercible_errors;
coercible_errors!(ReadError);

#[cfg(feature = "read")]
impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ReadError::SchemaMismatch => f.write_str("The expected schema did not match that in the document."),
            ReadError::InvalidFormat => f.write_str("The format was not a valid Tree-Buf"),
        }
    }
}

#[cfg(feature = "read")]
impl std::error::Error for ReadError {}

#[cfg(feature = "read")]
impl From<std::str::Utf8Error> for ReadError {
    fn from(_value: std::str::Utf8Error) -> Self {
        ReadError::InvalidFormat
    }
}




