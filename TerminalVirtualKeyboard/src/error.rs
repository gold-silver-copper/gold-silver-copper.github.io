use thiserror::Error;
use std::num::ParseIntError;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("{0}")]
    Err(String),
    #[error("Parse error: {0}")]
    ParseIntError(#[from] ParseIntError),
}
