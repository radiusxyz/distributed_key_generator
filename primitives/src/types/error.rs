use super::Event;
use std::io::Error as IoError;
use radius_sdk::{
    json_rpc::{
        client::RpcClientError,
        server::{RpcError, RpcServerError},
    },
    kvstore::KvStoreError,
    signature::{SignatureError, Signature, Address},
};
use toml::de::Error as TomlError;
use thiserror::Error;

/// Main error type used throughout the application
#[derive(Debug, Error)]
pub enum Error {
    /// External library errors
    Config(ConfigError),
    /// General database error
    Database(KvStoreError),
    /// General RPC server error
    RpcServerError(RpcServerError),
    /// General RPC client error
    RpcClientError(RpcClientError),
    /// Key generation errors
    KeyGeneration(KeyGenerationError),
    /// File system errors
    LoadConfigOption(IoError),
    ParseTomlString(TomlError),
    /// Signature error
    Signature(SignatureError),
    RemoveConfigDirectory,
    CreateConfigDirectory,
    CreateConfigFile,
    CreatePrivateKeyFile,
    // Data processing errors
    HexDecodeError,
    /// malformed, missing fields
    InvalidParams(String),
    /// follower or leader tried to call authority logic
    UnauthorizedParamAccess,
    /// Maybe overflow or underflow
    Arithmetic,
    /// Key not found for db
    NotFound,
    /// Task join error
    TaskJoinError(tokio::task::JoinError),
    /// Event emission error
    #[error(transparent)]
    EventError(#[from] tokio::sync::mpsc::error::SendError<Event<Signature, Address>>),
    /// Conversion error
    ConvertError(String),
}

/// Error type for key generation process
#[derive(Debug)]
pub enum KeyGenerationError {
    NotRegisteredGenerator(String),
    InvalidPartialKey(String),
    InternalError(String),
    InvalidSignature,
}

// Implement Display trait for KeyGenerationError
impl std::fmt::Display for KeyGenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyGenerationError::NotRegisteredGenerator(msg) => {
                write!(f, "Not registered key generator: {}", msg)
            }
            KeyGenerationError::InvalidPartialKey(msg) => write!(f, "Invalid key format: {}", msg),
            KeyGenerationError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            KeyGenerationError::InvalidSignature => write!(f, "Invalid signature"),
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

impl From<SignatureError> for Error {
    fn from(value: SignatureError) -> Self {
        Self::Signature(value)
    }
}

impl From<KeyGenerationError> for Error {
    fn from(value: KeyGenerationError) -> Self {
        Self::KeyGeneration(value)
    }
}

// Implement From trait for external error types
impl From<KvStoreError> for Error {
    fn from(err: KvStoreError) -> Self {
        Self::Database(err)
    }
}

impl From<ConfigError> for Error {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<radius_sdk::json_rpc::server::RpcServerError> for Error {
    fn from(value: radius_sdk::json_rpc::server::RpcServerError) -> Self {
        Self::RpcServerError(value)
    }
}

impl From<radius_sdk::json_rpc::client::RpcClientError> for Error {
    fn from(value: radius_sdk::json_rpc::client::RpcClientError) -> Self {
        Self::RpcClientError(value)
    }
}

impl From<KeyGenerationError> for RpcError {
    fn from(error: KeyGenerationError) -> Self {
        match error {
            KeyGenerationError::NotRegisteredGenerator(msg) => RpcError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Not registered key generator: {}", msg),
            )),
            KeyGenerationError::InvalidPartialKey(msg) => RpcError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid key format: {}", msg),
            )),
            KeyGenerationError::InternalError(msg) => RpcError::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Internal error: {}", msg),
            )),
            KeyGenerationError::InvalidSignature => RpcError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid signature",
            )),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Load(std::io::Error),
    Parse(toml::de::Error),
    RemoveConfigDirectory(std::io::Error),
    CreateConfigDirectory(std::io::Error),
    CreateConfigFile(std::io::Error),
    CreatePrivateKeyFile(std::io::Error),
    NotFound(String),
    InvalidExternalPort,
    InvalidClusterPort,
    AlreadyExists,
    InvalidConfig,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ConfigError {}
