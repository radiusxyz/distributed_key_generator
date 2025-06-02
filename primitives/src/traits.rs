use crate::{AuthError, Event, KeyGenerationError, SessionId};
use std::{hash::Hash, fmt::Debug, time::Duration};
use futures::future::{select, Either};
use futures_util::{pin_mut, future::Future};
use futures_timer::Delay;
use tokio::task::JoinHandle;
use serde::{Serialize, de::DeserializeOwned};
use async_trait::async_trait;
use radius_sdk::{
    signature::{PrivateKeySigner, SignatureError}, 
    kvstore::KvStoreError,
    json_rpc::client::RpcClientError,
    json_rpc::server::RpcServerError,
};

#[async_trait]
pub trait AppState: Clone + Send + Sync + 'static {
    /// The address type of this app
    type Address: Parameter + AddressT;
    /// The signature type of this app
    type Signature: Parameter + Debug;
    /// Verifier of this app 
    type Verify: Verify<Self::Signature, Self::Address>;
    /// Type that selects the leader
    type SelectLeader: SelectLeader;
    /// Type that generates key
    type SecureBlock: SecureBlock;
    /// Auth service of this app which interacts with the registry(e.g blockchain)
    type AuthService: AuthService<Self::Address>;
    /// Type that spawns tasks
    type AsyncTask: AsyncTask<Self::Signature, Self::Address, Self::Error>;
    /// The error type of this app
    type Error: std::error::Error 
        + From<SignatureError>
        + From<KvStoreError>
        + From<KeyGenerationError>
        + From<RpcServerError>
        + From<RpcClientError>
        + From<serde_json::Error>
        + From<AuthError>
        + Send 
        + Sync 
        + 'static;

    /// Get the threshold for the key generator
    fn threshold(&self) -> u16;
    /// Check if the node is a leader
    fn is_leader(&self) -> bool;
    /// Check if the node is a solver
    fn is_solver(&self) -> bool;
    /// Get the node's signer
    fn signer(&self) -> PrivateKeySigner;
    /// Get the node's address which is used for creating payload
    fn address(&self) -> Self::Address;
    /// Helper function to get log prefix
    fn log_prefix(&self) -> String;
    /// Helper function to get signature
    fn sign<T: Serialize>(&self, message: &T) -> Result<Self::Signature, Self::Error>;
    /// Get the randomness for a given session id
    fn randomness(&self, session_id: SessionId) -> Vec<u8>;
    /// Get the current session id
    fn current_session(&self) -> Result<SessionId, Self::Error>;
    /// Get the current round
    fn current_round(&self) -> Result<u64, Self::Error>;
    /// Get the current leader which will return (address, rpc_url)
    fn current_leader(&self, is_sync: bool) -> Result<(Self::Address, String), Self::Error>;
    /// Get the next leader which will return (address, rpc_url)
    fn next_leader(&self, is_sync: bool) -> Result<(Self::Address, String), Self::Error>;
    /// Helper function to verify signature. Verification will be handled by `Self::VerifySignature` type
    fn verify_signature<T: Serialize>(&self, signature: &Self::Signature, message: &T, maybe_signer: Option<Self::Address>) -> Result<Self::Address, Self::Error> {
        let signer = Self::Verify::verify_signature(signature, message)
            .map_err(|e| Self::Error::from(e))?;
        if let Some(address) = maybe_signer {
            if signer != address {
                return Err(Self::Error::from(KeyGenerationError::InvalidSignature));
            }
        }
        Ok(signer)
    }
    /// Get the instance of the auth service
    fn auth_service(&self) -> &Self::AuthService;
    /// Get the instance of the key generator
    fn secure_block(&self) -> &Self::SecureBlock;
    /// Get the instance of the task spawner
    fn async_task(&self) -> &Self::AsyncTask;
}

/// A trait for selecting the leader
pub trait SelectLeader {
    /// Get the current leader which will return the index of the leader
    fn current_leader(current_session: u64, len: usize) -> Option<usize>;
    /// Select the leader which will return the index of the leader
    fn next_leader(current_session: u64, len: usize) -> Option<usize>;
}

pub trait Verify<Signature, Address> {

    fn verify_signature<T: Serialize>(signature: &Signature, message: &T) -> Result<Address, SignatureError>;
}

pub trait SecureBlock {
    
    type TrustedSetUp: Parameter + Debug;
    type Metadata: Parameter;
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create new instance of the secure block with the trusted setup
    fn setup(param: Self::TrustedSetUp) -> Self;

    /// Get the trusted setup for this app
    fn get_trusted_setup(&self) -> Self::TrustedSetUp;

    /// Generate encryption key for a given session
    fn gen_enc_key(&self, randomness: Vec<u8>, maybe_enc_keys: Option<Vec<Vec<u8>>>) -> Result<Vec<u8>, Self::Error>;

    /// Generate decryption key from encryption key
    fn gen_dec_key(&self, enc_key: &Vec<u8>) -> Result<(Vec<u8>, u128), Self::Error>;

    /// Verify the given decryption key for a given session 
    fn verify_dec_key(&self, enc_key: &Vec<u8>, dec_key: &Vec<u8>) -> Result<(), Self::Error>;
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
            Either::Right(_) => {
                tracing::error!("Task timed out");
                None
            },
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


/// Interface for providing auth service
#[async_trait]
pub trait AuthService<Address>: Send + Sync + 'static {

    /// The error type of the auth service
    type Error: std::error::Error + Send + Sync + 'static;

    /// Update the trusted setup
    async fn update_trusted_setup(&self, bytes: Vec<u8>, signature: Vec<u8>) -> Result<(), Self::Error>;
    /// Get the trusted setup
    async fn get_trusted_setup(&self) -> Result<Vec<u8>, Self::Error>;
    /// Get the authority info which will return (address, cluster_rpc_url, external_rpc_url)
    async fn get_authority_info(&self) -> Result<(Address, String, String), Self::Error>;
    /// Get the solver info which will return (address, cluster_rpc_url, external_rpc_url)
    async fn get_solver_info(&self) -> Result<(Address, String, String), Self::Error>;
    /// Check if the given address is active at the given round
    async fn is_active(&self, current_round: u64, address: Address) -> Result<bool, Self::Error>;
    /// Get the current auth registry
    async fn current_auth_registry(&self, current_round: u64) -> Result<Vec<Address>, Self::Error>;
    /// Get the next auth registry
    async fn next_auth_registry(&self, next_round: u64) -> Result<Vec<Address>, Self::Error>;
    /// Check if the given round is ready
    async fn is_ready(&self, current_round: u64, threshold: u16) -> Result<bool, Self::Error>;
}

/// Interface for managing the session
pub trait SessionManager {
    
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

/// A trait that can be converted to and from a given type `T`
pub trait IsType<T>: From<T> + Into<T> {}
impl<T: From<T> + Into<T>> IsType<T> for T {}

pub trait Get<T> {
    fn get() -> T;
}