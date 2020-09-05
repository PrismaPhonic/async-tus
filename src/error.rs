use reqwest::header::InvalidHeaderValue;
use tokio::sync::mpsc::error::SendError;
use crate::Chunk;

#[derive(Debug)]
pub enum Error {
    ReqwestError(reqwest::Error),
    IoError(std::io::Error),
    InvalidHeaderValue(InvalidHeaderValue),

    // Remove this. For now - to make things work.
    StringError(String),

    ChannelError(SendError<(Chunk, u32)>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // TODO: Make this not shitty.
        write!(f, "got an error I guess")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        // TODO: Make this not shitty.
        None
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ReqwestError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<InvalidHeaderValue> for Error {
    fn from(err: InvalidHeaderValue) -> Self {
        Error::InvalidHeaderValue(err)
    }
}
