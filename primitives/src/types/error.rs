use super::Event;
use crate::ConsensusError;
use radius_sdk::{
    json_rpc::{
        client::RpcClientError,
        server::{RpcError, RpcServerError},
    },
    kvstore::KvStoreError,
    signature::{SignatureError, Signature, Address},
};
use thiserror::Error;
use tokio::{sync::mpsc::error::SendError, task::JoinError};

/// All error types of the service
#[derive(Debug, Error)]
pub enum Error {
    /// A database error has occurred
    Database(KvStoreError),
    /// A RPC server error has occurred
    RpcServerError(RpcServerError),
    /// A RPC client error has occurred
    RpcClientError(RpcClientError),
    /// Key service errors
    KeyServiceError(KeyServiceError),
    /// Task join error
    TaskJoinError(JoinError),
    /// Event emission error
    #[error(transparent)]
    EventError(#[from] SendError<Event<Signature, Address>>),
    /// Conversion error
    ConvertError(String),
    /// Consensus error
    #[error(transparent)]
    Consensus(#[from] ConsensusError),
    /// General (de)serialization error
    #[error(transparent)]
    SerializeError(#[from] serde_json::Error),
    /// Any error wrapped type
    #[error(transparent)]
    AnyError(#[from] Box<dyn std::error::Error>),
    /// Auth service error
    #[error(transparent)]
    AuthServiceError(#[from] AuthServiceError),
    /// Maybe overflow or underflow
    Arithmetic,
    /// Key not found for db
    NotFound,
    /// Leader not found
    LeaderNotFound,
}

/// Error type for key generation process
#[derive(Debug)]
pub enum KeyServiceError {
    /// Key generator not registered
    NotRegistered(String),
    /// Invalid partial key
    InvalidPartialKey(String),
    /// Internal error
    InternalError(String),
    /// Invalid signature
    InvalidSignature(SignatureError),
}

impl From<KeyServiceError> for Error {
    fn from(value: KeyServiceError) -> Self { Self::KeyServiceError(value) }
}

// Implement Display trait for KeyGenerationError
impl std::fmt::Display for KeyServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyServiceError::NotRegistered(msg) => {
                write!(f, "Not registered key generator: {}", msg)
            }
            KeyServiceError::InvalidPartialKey(msg) => write!(f, "Invalid key format: {}", msg),
            KeyServiceError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            KeyServiceError::InvalidSignature(e) => write!(f, "Invalid signature: {}", e),
        }
    }
}

// Ensure Error type can be safely sent between threads
unsafe impl Send for Error {}
unsafe impl Sync for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Implement From trait for external error types
impl From<KvStoreError> for Error {
    fn from(err: KvStoreError) -> Self {
        Self::Database(err)
    }
}

impl From<RpcServerError> for Error {
    fn from(value: RpcServerError) -> Self { Self::RpcServerError(value) }
}

impl From<RpcClientError> for Error {
    fn from(value: RpcClientError) -> Self { Self::RpcClientError(value) }
}

impl From<KeyServiceError> for RpcError {
    fn from(error: KeyServiceError) -> Self {
        match error {
            KeyServiceError::NotRegistered(msg) => RpcError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Not registered key generator: {}", msg),
            )),
            KeyServiceError::InvalidPartialKey(msg) => RpcError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid key format: {}", msg),
            )),
            KeyServiceError::InternalError(msg) => RpcError::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Internal error: {}", msg),
            )),
            KeyServiceError::InvalidSignature(e) => RpcError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid signature: {}", e),
            )),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AuthServiceError {
    #[error("Error on getting state from blockchain!")]
    GetStateError,
    #[error("Invalid role!")]
    InvalidRole,
    #[error("Error on registering!")]
    RegisterError,
    #[error("Error on unregistering!")]
    UnregisterError,
    #[error("Already registered!")]
    AlreadyRegistered,
    #[error("Any error: {0}")]
    AnyError(String),
}
