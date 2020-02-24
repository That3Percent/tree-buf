use std::fmt::{Debug, Display, Formatter};

// These are mainly useful for debugging when writing the format
// and probably isn't useful to the public API.
#[cfg(feature = "read")]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InvalidFormat {
    UnrecognizedTypeId,
    // TODO: Consider removing this. Without it, non-default values on the tail
    // end of the array need not be serialized. It also may require less branching
    // so that the code run faster if a caller to read() can always expect a value.
    ShortArray,
    Utf8Error(std::str::Utf8Error),
    EndOfFile,
    DecompressionError,
}

#[cfg(feature = "read")]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ReadError {
    SchemaMismatch,
    InvalidFormat(InvalidFormat),
}

#[cfg(feature = "read")]
impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ReadError::SchemaMismatch => f.write_str("The expected schema did not match that in the document."),
            ReadError::InvalidFormat(invalid_format) => {
                f.write_str("Invalid Format: ")?;
                match invalid_format {
                    InvalidFormat::UnrecognizedTypeId => f.write_str(format!("The type id is not recognized.").as_str()),
                    InvalidFormat::ShortArray => f.write_str("The array did not contain enough elements"),
                    InvalidFormat::EndOfFile => f.write_str("Attempted to read beyond the end of the file"),
                    InvalidFormat::Utf8Error(inner) => std::fmt::Display::fmt(inner, f),
                    InvalidFormat::DecompressionError => f.write_str("A decompression failed"),
                }
            }
        }
    }
}

#[cfg(feature = "read")]
impl std::error::Error for ReadError {}

#[cfg(feature = "read")]
impl From<std::str::Utf8Error> for ReadError {
    fn from(value: std::str::Utf8Error) -> Self {
        ReadError::InvalidFormat(InvalidFormat::Utf8Error(value))
    }
}
