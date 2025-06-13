use super::RuntimeEvent;
use crate::ConsensusError;
use radius_sdk::{
    json_rpc::{client::RpcClientError, server::RpcServerError},
    kvstore::KvStoreError,
    signature::{SignatureError, Signature, Address},
};
use thiserror::Error;
use tokio::{sync::mpsc::error::SendError, task::JoinError};
use serde_json::Error as SerdeError;
use skde::delay_encryption::{DecryptionError, EncryptionError};

/// Error types of the runtime
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// A database error has occurred
    #[error(transparent)]
    Database(#[from] KvStoreError),
    /// A RPC server error has occurred
    #[error(transparent)]
    RpcServerError(#[from] RpcServerError),
    /// A RPC client error has occurred
    #[error(transparent)]
    RpcClientError(#[from] RpcClientError),
    /// Key service errors
    #[error(transparent)]
    KeyServiceError(#[from] KeyServiceError),
    /// Task join error
    #[error(transparent)]
    TaskJoinError(#[from] JoinError),
    /// Event emission error
    #[error(transparent)]
    EventError(#[from] SendError<RuntimeEvent<Signature, Address>>),
    /// Conversion error
    #[error("Conversion error: {0}")]
    ConvertError(String),
    /// Consensus error
    #[error(transparent)]
    Consensus(#[from] ConsensusError),
    /// General (de)serialization error
    #[error(transparent)]
    SerializeError(#[from] SerdeError),
    /// Any error wrapped type
    #[error(transparent)]
    AnyError(#[from] Box<dyn std::error::Error>),
    /// Auth service error
    #[error(transparent)]
    AuthServiceError(#[from] AuthServiceError),
    /// Maybe overflow or underflow
    #[error("Arithmetic error(e.g. overflow or underflow)")]
    Arithmetic,
    /// Key not found for db
    #[error("Key not found for db")]
    NotFound,
    /// Leader not found
    #[error("Leader not found")]
    LeaderNotFound,
}

// Ensure Error type can be safely sent between threads
unsafe impl Send for RuntimeError {}
unsafe impl Sync for RuntimeError {}

/// Error type for key generation process
#[derive(Debug, Error)]
pub enum KeyServiceError {
    /// Key generator not registered
    #[error("Key generator not registered: {0}")]
    NotRegistered(String),
    /// Invalid partial key
    #[error("Invalid partial key: {0}")]
    InvalidPartialKey(String),
    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
    /// Invalid signature
    #[error(transparent)]
    InvalidSignature(#[from] SignatureError),
    /// Decryption error has occurred
    #[error(transparent)]
    DecryptionError(#[from] DecryptionError),
    /// Encryption error has occurred
    #[error(transparent)]
    EncryptionError(#[from] EncryptionError),
    /// Message mismatch
    #[error("Message mismatch")]
    MessageMismatch,
    /// Error on (de)serialization
    #[error(transparent)]
    SerdeError(#[from] SerdeError),
}

unsafe impl Send for KeyServiceError {}
unsafe impl Sync for KeyServiceError {}

#[derive(Debug, Error)]
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
    