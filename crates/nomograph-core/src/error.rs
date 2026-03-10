use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("index error: {0}")]
    Index(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type IndexError = CoreError;
