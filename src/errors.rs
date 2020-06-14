use nom::error::ErrorKind;
use std::str::Utf8Error;
use thiserror::Error;

/// An error that occurred while parsing. This is the general error type for
/// this library.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Parse encountered some other error.
    /// This is probably the most common error.
    #[error("Error parings input: {} at {at}", .kind.description())]
    Other {
        /// Remain input when error occurred
        at: ErrorBytes,
        /// Type of error
        kind: ErrorKind,
    },
    /// Parser couldn't finish due to incomplete input
    #[error("Incomplete input")]
    Incomplete,
    #[error("Error parsing string to utf8")]
    Utf8Error { bytes: Vec<u8>, source: Utf8Error },
}

/// The remaining input from the parser.  Useful for debugging to see where the
/// parser failed.  This is used in [`ParseError`](struct.ParseError.html).
/// It'll be `Valid` if the remaining input was a valid string and `Invalid` if
/// it wasn't
#[derive(Debug, Error)]
pub enum ErrorBytes {
    /// Input was a valid string
    #[error("`{0}`")]
    Valid(String),
    /// Input was not a valid string
    #[error("`{0:?}`")]
    Invalid(Vec<u8>),
}

impl From<nom::Err<(&[u8], nom::error::ErrorKind)>> for ParseError {
    fn from(e: nom::Err<(&[u8], nom::error::ErrorKind)>) -> Self {
        match e.to_owned() {
            nom::Err::Error((bytes, kind))
            | nom::Err::Failure((bytes, kind)) => {
                match std::str::from_utf8(&bytes) {
                    Ok(s) => ParseError::Other {
                        at: ErrorBytes::Valid(s.to_owned()),
                        kind,
                    },
                    Err(_) => ParseError::Other {
                        at: ErrorBytes::Invalid(bytes),
                        kind,
                    },
                }
            }
            nom::Err::Incomplete(_) => ParseError::Incomplete,
        }
    }
}
