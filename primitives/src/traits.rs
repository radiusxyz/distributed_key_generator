use std::{fmt::Debug, hash::Hash, sync::Arc, time::Duration};

use async_trait::async_trait;
use dkg_utils::timestamp;
use futures::future::{select, Either};
use futures_timer::Delay;
use futures_util::{future::Future, pin_mut};
use radius_sdk::{
    json_rpc::{client::RpcClientError, server::RpcServerError},
    kvstore::KvStoreError,
    signature::{PrivateKeySigner, SignatureError},
    validation_service::{validation_dkg::DkgValidation, DkgValidationServiceError},
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Error as SerdeJsonError;
use tokio::{sync::RwLock, task::JoinHandle};

use crate::{
    ActiveOperatorList, DecKey, DecKeyRequestRecord, DkgEvent, EncKey, KeyGeneratorError,
    NextOperatorList, Operator, RuntimeError, SessionId,
};

/// Config trait for the node
#[async_trait]
pub trait Config: Clone + Send + Sync + 'static {
    /// The address type of the runtime
    type Address: AddressT;
    /// The signature type of the runtime
    type Signature: Parameter + Debug;
    /// Type that selects the leader
    type SelectLeader: SelectLeader;
    /// Type that serves related to verification
    type VerifyService: VerifyService<Self::Signature, Self::Address>;
    /// Type that serves related key generation(e.g Set trusted setup, generate encryption and decryption key)
    type KeyGenerator: KeyGenerator + Parameter;
    /// Type that validates the distributed key generation process
    type ValidationService: DkgValidation<Address = Self::Address, Key = u64, Proof = Vec<u8>>;
    /// Type that spawns tasks
    type AsyncTask: AsyncTask<Self::Signature, Self::Address, Self::Error>;
    /// Type that serves related to backend services
    type DbManager: DbManager<Self::Address>;
    /// Type of hasher this runtime uses
    type Hasher: Hasher;
    /// The error type of the runtime
    type Error: std::error::Error
        + IsType<RuntimeError>
        + From<KvStoreError>
        + From<KeyGeneratorError>
        + From<RpcServerError>
        + From<RpcClientError>
        + From<SerdeJsonError>
        + From<DkgValidationServiceError>
        + Send
        + Sync
        + 'static;

    /// Check if the node is a leader
    fn is_leader(&self, session_id: SessionId) -> bool;
    /// Check if the node is a solver
    fn is_solver(&self) -> bool;
    /// Get the node's signer
    fn signer(&self) -> &PrivateKeySigner;
    /// Get the node's address which is used for creating payload
    fn address(&self) -> Self::Address;
    /// Get the session duration
    fn session_duration(&self) -> u64;
    /// Helper function to get signature
    fn sign<T: Serialize>(&self, message: &T) -> Result<Self::Signature, Self::Error>;
    /// Get the randomness for a given session id
    fn randomness(&self, session_id: SessionId) -> Vec<u8>;
    /// Check if the node should force generate the encryption key
    fn should_force_generating(&self) -> Result<bool, Self::Error>;
    /// Get the current leader on the current session which will return (address, rpc_url)
    fn current_leader(
        &self,
        session_id: SessionId,
        is_sync: bool,
    ) -> Result<(Self::Address, String), Self::Error>;
    /// Helper function to verify signature. Verification will be handled by `Self::VerifySignature` type
    fn verify_signature<T: Serialize>(
        &self,
        signature: &Self::Signature,
        message: &T,
        maybe_signer: Option<Self::Address>,
    ) -> Result<Self::Address, Self::Error> {
        let signer = Self::VerifyService::verify_signature(signature, message)
            .map_err(|e| KeyGeneratorError::InvalidSignature(e))?;
        if let Some(address) = maybe_signer {
            if signer != address {
                return Err(
                    KeyGeneratorError::InvalidSignature(SignatureError::Unauthorized).into(),
                );
            }
        }
        Ok(signer)
    }
    /// Hash the given input
    fn hash(&self, input: &[u8]) -> Vec<u8> {
        Self::Hasher::hash(input, None).into()
    }
    /// Get the instance of the validation service
    fn validation_service(&self) -> &Self::ValidationService;
    /// Get the instance of the key generator
    fn key_generator(&self) -> Arc<RwLock<Self::KeyGenerator>>;
    /// Get the instance of the task spawner
    fn async_task(&self) -> &Self::AsyncTask;
    /// Get the instance of the db manager
    fn db_manager(&self) -> &Self::DbManager;
}

/// Interface for selecting the leader
pub trait SelectLeader {
    /// Get the current leader which will return the index of the leader
    fn select_leader(current_session: u64, len: usize) -> Option<usize>;
}

/// Interface for db client services
pub trait DbManager<Address: AddressT> {
    type Error: IsType<RuntimeError>
        + From<KvStoreError>
        + std::error::Error
        + Send
        + Sync
        + 'static;

    /// Get the current session id
    fn current_session(&self) -> Result<SessionId, Self::Error> {
        let kv_store: SessionId = SessionId::get()?;
        Ok(kv_store)
    }

    /// Increase the session id by one
    fn increase_session(&self, amount: u64) -> Result<(), Self::Error> {
        let mut kv_store: SessionId = SessionId::get()?;
        kv_store.next_mut(amount)?.put()?;
        Ok(())
    }
    /// Update the operator list
    fn update_active_operator_list(
        &self,
        operators: &Vec<Operator<Address>>,
    ) -> Result<(), Self::Error> {
        let kv_store: ActiveOperatorList<Address> = operators.clone().into();
        let _ = kv_store.put()?;
        Ok(())
    }

    fn update_next_operator_list(
        &self,
        operators: &Vec<Operator<Address>>,
    ) -> Result<(), Self::Error> {
        let kv_store: NextOperatorList<Address> = operators.clone().into();
        let _ = kv_store.put()?;
        Ok(())
    }

    fn get_active_operator_list(&self) -> Result<Vec<Operator<Address>>, Self::Error> {
        let kv_store: ActiveOperatorList<Address> = ActiveOperatorList::get()?;
        Ok(kv_store.into_iter().collect())
    }

    fn get_enc_key(&self, session_id: SessionId) -> Result<EncKey, Self::Error> {
        let kv_store: EncKey = EncKey::get(session_id)?;
        Ok(kv_store)
    }

    fn get_dec_key(&self, session_id: SessionId) -> Result<DecKey, Self::Error> {
        let kv_store: DecKey = DecKey::get(session_id)?;
        Ok(kv_store)
    }

    fn update_dec_key_request_record(
        &self,
        session_id: SessionId,
        session_duration: u64,
    ) -> Result<(), Self::Error> {
        let record = DecKeyRequestRecord::new(timestamp(), session_duration as u128);
        let _ = record.put(session_id)?;
        Ok(())
    }

    fn get_dec_key_request_record(
        &self,
        session_id: SessionId,
    ) -> Result<DecKeyRequestRecord, Self::Error> {
        let kv_store: DecKeyRequestRecord = DecKeyRequestRecord::get(session_id)?;
        Ok(kv_store)
    }
}

/// Inferface for verifying related
pub trait VerifyService<Signature, Address> {
    fn verify_signature<T: Serialize>(
        signature: &Signature,
        message: &T,
    ) -> Result<Address, SignatureError>;
}

/// Interface for generating (encryption, decryption) keys
pub trait KeyGenerator {
    type Error: std::error::Error + Send + Sync + 'static + Into<RuntimeError>;

    fn setup(trusted_setup: Vec<u8>) -> Result<Self, Self::Error> where Self: Sized;

    /// Generate encryption key for a given session
    fn gen_enc_key(
        &self,
        randomness: &Vec<u8>,
        maybe_enc_keys: Option<Vec<Vec<u8>>>,
    ) -> Result<Vec<u8>, Self::Error>;

    /// Generate decryption key from encryption key
    fn gen_dec_key(&self, enc_key: &Vec<u8>) -> Result<(Vec<u8>, u128), Self::Error>;

    /// Verify the given decryption key for a given session
    fn verify_dec_key(&self, enc_key: &Vec<u8>, dec_key: &Vec<u8>) -> Result<(), Self::Error>;
}

/// Interface for spawning async tasks
#[async_trait]
pub trait AsyncTask<Signature, Address, Error>: Send + Sync + Unpin + 'static
where
    Error: std::error::Error + Send + Sync + 'static + Into<RuntimeError>,
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
            }
        }
    }

    /// Helper function to emit an event
    async fn emit_event(&self, event: DkgEvent<Signature, Address>) -> Result<(), Error>;

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

pub trait AddressT:
    Parameter + Hash + Eq + PartialEq + Clone + Debug + Into<String> + From<String> + From<Vec<u8>>
{
}

impl<T> AddressT for T where
    T: Parameter
        + Hash
        + Eq
        + PartialEq
        + Clone
        + Debug
        + Into<String>
        + From<String>
        + From<Vec<u8>>
{
}

/// A trait for types that can be used as a hasher
pub trait Hasher {
    type Output: Into<Vec<u8>>;
    const LENGTH: usize;

    /// Hash function which size would be dependent on the given input size
    fn hash(input: &[u8], size: Option<usize>) -> Self::Output;
}

/// A trait that can be converted to and from a given type `T`
pub trait IsType<T>: From<T> + Into<T> {}
impl<T: From<T> + Into<T>> IsType<T> for T {}

/// Infallible conversion from `A` to `B`
pub trait Convert<A, B> {
    fn convert(value: A) -> B;
}

pub trait Get<T> {
    fn get() -> T;
}
