use nom::error::ErrorKind;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Error parings input: {} at {at}", .kind.description())]
    Other { at: ErrorBytes, kind: ErrorKind },
    #[error("Incomplete input")]
    Incomplete,
}

#[derive(Debug, Error)]
pub enum ErrorBytes {
    #[error("`{0}`")]
    Valid(String),
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
