use crate::{KeyGenerationError, Event, SessionId};
use std::{hash::Hash, fmt::Debug, time::Duration};
use radius_sdk::{
    signature::{PrivateKeySigner, SignatureError}, 
    kvstore::KvStoreError,
    json_rpc::client::RpcClientError,
    json_rpc::server::RpcServerError,
};
use skde::delay_encryption::SkdeParams;
use futures::future::{select, Either};
use futures_util::{pin_mut, future::Future};
use futures_timer::Delay;
use tokio::task::JoinHandle;
use serde::{Serialize, de::DeserializeOwned};
use async_trait::async_trait;

#[async_trait]
pub trait AppState: Clone + Send + Sync + 'static {
    /// The address type that this app accepts
    type Address: Parameter + AddressT;
    /// The signature type that this app accepts
    type Signature: Parameter + Debug;
    /// Verifier for the signature
    type Verify: Verify<Self::Signature, Self::Address>;
    /// Type that spawns tasks
    type AsyncTask: AsyncTask<Self::Signature, Self::Address, Self::Error>;
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

    /// Get the threshold for the key generator
    fn threshold(&self) -> u16;
    /// Check if the node is a leader
    fn is_leader(&self) -> bool;
    /// Check if the node is a solver
    fn is_solver(&self) -> bool;
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
    ) -> Result<(), KeyGenerationError> {
        Self::Verify::verify_decryption_key(skde_params, encryption_key, decryption_key)
    }
    /// Helper function to get task spawner. This should not be used outside of the task module.
    fn async_task(&self) -> &Self::AsyncTask;
}

pub trait Verify<Signature, Address> {

    fn verify_signature<T: Serialize>(signature: &Signature, message: &T) -> Result<Address, SignatureError>;

    fn verify_decryption_key(
        skde_params: &SkdeParams,
        encryption_key: String,
        decryption_key: String,
    ) -> Result<(), KeyGenerationError>;
}

pub trait SecureBlock {
    type EncKey: Parameter;
    type DecKey: Parameter;
    type Error: std::error::Error + Send + Sync + 'static;

    fn get_enc_key(&self, session_id: SessionId) -> Result<Self::EncKey, Self::Error>;

    fn get_dec_key(&self, session_id: SessionId) -> Result<Self::DecKey, Self::Error>;

    fn verify_key_pair(&self) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait AsyncTask<Signature, Address, Error>: Send + Sync + Unpin + 'static 
where
    Error: std::error::Error + Send + Sync + 'static,
{
    fn spawn_task<Fut>(&self, fut: Fut) -> JoinHandle<()>
    where
        Fut: Future<Output = ()> + Send + 'static;

    fn spawn_blocking<Fut>(&self, fut: Fut) -> JoinHandle<()>
    where
        Fut: Future<Output = ()> + Send + 'static;

    /// Helper function to spawn a task with a timeout
    async fn spawn_with_timeout<Fut, R>(&self, fut: Fut, timeout: Duration) -> Option<R>
    where
        Fut: Future<Output = Result<R, Error>> + Send + 'static,
    {
        let delay = Delay::new(timeout);
        pin_mut!(fut);
        match select(fut, delay).await {
            Either::Left((Ok(res), _)) => Some(res),
            Either::Left((Err(e), _)) => {
                tracing::error!("{:?}", e);
                None
            }
            Either::Right(_) => None,
        }
    }

    /// Helper function to emit an event
    async fn emit_event(&self, event: Event<Signature, Address>) -> Result<(), Error>;

    // TODO: REFACTOR ME! - RPC Worker should be a separate thread
    /// API for RPC request which waits for the response
    async fn request<P, R>(&self, url: String, method: String, parameter: P) -> Result<R, Error>
    where
        P: Serialize + Send + Sync + 'static,
        R: DeserializeOwned + Send + Sync + 'static;

    // TODO: REFACTOR ME! - RPC Worker should be a separate thread
    /// API for RPC multicast which does not wait for the response
    fn multicast<P>(&self, urls: Vec<String>, method: String, parameter: P)
    where
        P: Serialize + Send + Sync + 'static;
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

/// A trait for types that can be used as a hasher
pub trait Hasher {
    type Output;
    const LENGTH: usize;

    /// Hash function which size would be dependent on the given input size
    fn hash(input: &[u8], size: Option<usize>) -> Self::Output;
}