use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Identity error: {0}")]
    Identity(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Signal error: {0}")]
    Signal(String),

    #[error("Dashboard error: {0}")]
    Dashboard(String),
}

pub type NodeResult<T> = Result<T, NodeError>;
