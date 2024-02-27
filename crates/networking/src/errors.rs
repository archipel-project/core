use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum DeserializationError {
    NotEnoughBytes,
    InvalidPacketContent,
}

impl Error for DeserializationError {

}

impl Display for DeserializationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            DeserializationError::NotEnoughBytes => "Not enough bytes",
            DeserializationError::InvalidPacketContent => "Invalid packet content",
        })
    }
}
