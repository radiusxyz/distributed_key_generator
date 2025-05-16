use crate::KeyGenerationError;
use std::hash::Hash;
use radius_sdk::signature::{Address, PrivateKeySigner, Signature, SignatureError};
use skde::delay_encryption::SkdeParams;
use futures_util::future::BoxFuture;
use tokio::task::JoinHandle;
use serde::{Serialize, de::DeserializeOwned};

pub trait AppState: Clone + Send + Sync {

    /// The error type of this app
    type Error: std::error::Error + Send + Sync + 'static + From<SignatureError>;
    /// The address type that this app accepts
    type Address: Serialize + DeserializeOwned + PartialEq + Eq + Hash + Send + Sync + 'static;
    /// The signature type that this app accepts
    type Signature: Serialize + DeserializeOwned + Send + Sync + 'static;
    /// Verifier for the signature
    type Verify: Verify<Self::Signature>;

    /// Check if the node is a leader
    fn is_leader(&self) -> bool;
    /// Get the leader's rpc url
    fn leader_rpc_url(&self) -> String;
    /// Get the node's signer
    fn signer(&self) -> PrivateKeySigner;
    /// Get the node's address which is used for creating payload
    fn address(&self) -> Address;
    /// Get pre-set skde parameter
    fn skde_params(&self) -> SkdeParams;
    /// Get the session cycle
    fn session_cycle(&self) -> u64;
    /// Helper function to get log prefix
    fn log_prefix(&self) -> String;
    /// Helper function to get signature
    fn create_signature<T: Serialize>(&self, message: &T) -> Result<Signature, Self::Error> {
        self.signer()
            .sign_message(message)
            .map_err(|e| Self::Error::from(e))
    }
    /// Helper function to verify signature. Verification will be handled by `Self::VerifySignature` type
    fn verify_signature<T: Serialize>(&self, signature: &Self::Signature, message: &T) -> Result<Address, Self::Error> {
        Self::Verify::verify_signature(signature, message)
            .map_err(|e| Self::Error::from(e))
    }
    /// Helper function to verify decryption key
    fn verify_decryption_key(
        &self,
        skde_params: &SkdeParams,
        encryption_key: &str,
        decryption_key: &str,
        prefix: &str,
    ) -> Result<(), KeyGenerationError> {
        Self::Verify::verify_decryption_key(skde_params, encryption_key, decryption_key, prefix)
    }
}

pub trait Verify<Signature> {

    fn verify_signature<T: Serialize>(signature: &Signature, message: &T) -> Result<Address, SignatureError>;

    fn verify_decryption_key(
        skde_params: &SkdeParams,
        encryption_key: &str,
        decryption_key: &str,
        prefix: &str,
    ) -> Result<(), KeyGenerationError>;
}

pub trait TaskSpawner: Send + Sync + Unpin {
    fn spawn_task(&self, fut: BoxFuture<'static, ()>) -> JoinHandle<()>;

    fn spawn_blocking(&self, fut: BoxFuture<'static, ()>) -> JoinHandle<()>; 
}

/// Using unwrap() inside the task block is caught by tracing::error!().
/// However, if the task involves a loop that must not break when panics,
/// the trait helps to convert `Result<T, E>` to `Option<T>` while printing
/// the error message to the console.
pub trait TraceExt {
    type Output: Send + 'static;

    fn ok_or_trace(self) -> Option<Self::Output>;
}

impl<T, E> TraceExt for Result<T, E>
where
    T: Send + 'static,
    E: std::error::Error + Send + 'static,
{
    type Output = T;

    #[track_caller]
    fn ok_or_trace(self) -> Option<Self::Output> {
        match self {
            Ok(value) => Some(value),
            Err(error) => {
                let location = std::panic::Location::caller();
                tracing::error!("{} at {}", error, location);
                None
            }
        }
    }
}
