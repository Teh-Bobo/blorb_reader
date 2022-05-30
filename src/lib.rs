use crate::blorb_chunk_types::BlorbChunkType;
use std::fmt::{Display, Formatter};

mod blorb_chunk_types;
pub mod blorb_reader;
pub mod ulx_reader;

pub(crate) fn read_be_u32(input: &[u8]) -> u32 {
    u32::from_be_bytes(input[0..4].try_into().unwrap())
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum FileReadError {
    UnexpectedStartingIdentifier(BlorbChunkType),
    /// An invalid length was supplied. Actual and Expected.
    InvalidLength(usize, usize),
    UnknownIdentifier(usize),
    InvalidConversion,
}

impl Display for FileReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FileReadError::UnexpectedStartingIdentifier(expected) => {
                write!(f, "Unexpected starting identifier. Expected {:?}", expected)
            }
            FileReadError::InvalidLength(actual, expected) => {
                write!(
                    f,
                    "An invalid length was supplied. Actual length: {}, expected length: {}",
                    actual, expected
                )
            }
            FileReadError::UnknownIdentifier(id) => {
                write!(f, "An unknown identifier was supplied: {}", id)
            }
            FileReadError::InvalidConversion => {
                write!(f, "An invalid conversion was attempted")
            }
        }
    }
}

impl std::error::Error for FileReadError {}
