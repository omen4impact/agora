use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Identity error: {0}")]
    Identity(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Room error: {0}")]
    Room(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Libp2p error: {0}")]
    Libp2p(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Crypto error: {0}")]
    Crypto(String),
    
    #[error("NAT traversal error: {0}")]
    Nat(String),
}

pub type AgoraResult<T> = std::result::Result<T, Error>;
