use crate::KeyGenerationError;
use std::{hash::Hash, fmt::Debug};
use radius_sdk::{
    signature::{PrivateKeySigner, SignatureError}, 
    kvstore::KvStoreError,
    json_rpc::client::RpcClientError,
    json_rpc::server::RpcServerError,
};
use skde::delay_encryption::SkdeParams;
use futures_util::future::BoxFuture;
use tokio::task::JoinHandle;
use serde::{Serialize, de::DeserializeOwned};

pub trait AppState: Clone + Send + Sync + 'static {
    /// The address type that this app accepts
    type Address: Parameter + AddressT;
    /// The signature type that this app accepts
    type Signature: Parameter + Debug;
    /// Verifier for the signature
    type Verify: Verify<Self::Signature, Self::Address>;
    /// Type that spawns tasks
    type TaskSpawner: TaskSpawner;
    /// The error type of this app
    type Error: std::error::Error 
        + From<SignatureError>
        + From<KvStoreError>
        + From<KeyGenerationError>
        + From<RpcServerError>
        + From<RpcClientError>
        + Send 
        + Sync 
        + 'static;

    /// Check if the node is a leader
    fn is_leader(&self) -> bool;
    /// Get the leader's rpc url
    fn leader_rpc_url(&self) -> Option<String>;
    /// Get the node's signer
    fn signer(&self) -> PrivateKeySigner;
    /// Get the node's address which is used for creating payload
    fn address(&self) -> Self::Address;
    /// Get pre-set skde parameter
    fn skde_params(&self) -> SkdeParams;
    /// Helper function to get log prefix
    fn log_prefix(&self) -> String;
    /// Helper function to get signature
    fn sign<T: Serialize>(&self, message: &T) -> Result<Self::Signature, Self::Error>;
    /// Helper function to verify signature. Verification will be handled by `Self::VerifySignature` type
    fn verify_signature<T: Serialize>(&self, signature: &Self::Signature, message: &T, maybe_signer: Option<&Self::Address>) -> Result<Self::Address, Self::Error> {
        let signer = Self::Verify::verify_signature(signature, message)
            .map_err(|e| Self::Error::from(e))?;
        if let Some(address) = maybe_signer {
            if signer != *address {
                return Err(Self::Error::from(KeyGenerationError::InvalidSignature));
            }
        }
        Ok(signer)
    }
    /// Helper function to verify decryption key
    fn verify_decryption_key(
        &self,
        skde_params: &SkdeParams,
        encryption_key: String,
        decryption_key: String,
        prefix: &str,
    ) -> Result<(), KeyGenerationError> {
        Self::Verify::verify_decryption_key(skde_params, encryption_key, decryption_key, prefix)
    }

    /// Helper function to get task spawner. This should not be used outside of the task module.
    fn task_spawner(&self) -> &Self::TaskSpawner;

    /// Helper function to spawn a task
    fn spawn_task(&self, fut: BoxFuture<'static, ()>) -> JoinHandle<()> {
        self.task_spawner().spawn_task(fut)
    }

    /// Helper function to spawn a blocking task
    fn spawn_blocking(&self, fut: BoxFuture<'static, ()>) -> JoinHandle<()> {
        self.task_spawner().spawn_blocking(fut)
    }
}

pub trait Verify<Signature, Address> {

    fn verify_signature<T: Serialize>(signature: &Signature, message: &T) -> Result<Address, SignatureError>;

    fn verify_decryption_key(
        skde_params: &SkdeParams,
        encryption_key: String,
        decryption_key: String,
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

/// A trait for types that can be used in RPC parameters
pub trait Parameter: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}

impl<T> Parameter for T where T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}

pub trait AddressT: Hash + Eq + PartialEq + Clone + Debug + Into<String> + From<String> {}

impl<T> AddressT for T where T: Hash + Eq + PartialEq + Clone + Debug + Into<String> + From<String> {}


