pub use crate::NodeConfig;
use std::sync::Arc;
pub use dkg_primitives::{
    Config, DecKey, KeyGeneratorList, VerifyService, SelectLeader, AsyncTask, RuntimeResult, RuntimeError,
    TraceExt, KeyServiceError, SessionId, Round, Parameter, RuntimeEvent, TrustedSetupFor, 
    AuthService, AuthServiceError, KeyService, DbManager, AddressT
};
use radius_sdk::{signature::{PrivateKeySigner, Address, Signature, SignatureError}, json_rpc::client::{RpcClient, Id}};
use ethers::{types::Signature as EthersSignature, utils::hash_message};
use serde::{Serialize, de::DeserializeOwned};
use futures_util::future::Future;
use tokio::{task::JoinHandle, sync::mpsc::Sender};
use async_trait::async_trait;

mod auth;
pub use auth::*;

mod key_service;
pub use key_service::*;

pub mod config;
pub use config::*;


#[derive(Clone)]
/// Instance of DKG service
pub struct BasicDkgService<KS, AS, DB> {
    signer: PrivateKeySigner,
    task_executor: DefaultTaskExecutor,
    role: Role,
    threshold: u16,
    pub key_service: Option<KS>,
    pub auth_service: AS,
    pub db_manager: DB,
}

impl<KS, AS, DB> BasicDkgService<KS, AS, DB> {
    pub fn new(
        signer: PrivateKeySigner,
        task_executor: DefaultTaskExecutor,
        role: Role,
        threshold: u16,
        auth_service: AS,
        db_manager: DB,
    ) -> RuntimeResult<Self> {        
        Ok(Self { signer, task_executor, role, threshold, key_service: None, auth_service, db_manager })
    }

    pub fn task_executor(&self) -> &DefaultTaskExecutor {
        &self.task_executor
    }
}

mod consts {
    // Day in 2s session
    pub const DAY: u64 = 1800;
    pub const WEEK: u64 = 7 * DAY;
}

impl<KS, AS, DB> Config for BasicDkgService<KS, AS, DB> 
where
    KS: KeyService + Parameter,
    AS: AuthService<Address> + Clone,
    DB: DbManager<Address, Error = RuntimeError> + Clone + Send + Sync + 'static,
{
    type Address = Address;
    type Signature = Signature;
    type SelectLeader = DefaultSelectLeader;
    type VerifyService = DefaultVerifier;
    type KeyService = KS;
    type AuthService = AS;
    type AsyncTask = DefaultTaskExecutor;
    type DbManager = DB;
    type Error = RuntimeError;

    const ROUND_DURATION: u64 = consts::WEEK;

    fn threshold(&self) -> u16 { self.threshold }
    fn is_leader(&self) -> bool { 
        match self.current_leader(false) {
            Ok((leader, _)) => leader == self.address(),
            Err(_) => false,
        }    
    }
    fn is_solver(&self) -> bool { self.role == Role::Solver }
    fn signer(&self) -> &PrivateKeySigner { &self.signer }
    fn address(&self) -> Address { self.signer.address().clone() }
    fn randomness(&self, session_id: SessionId) -> Vec<u8> {
        match session_id.prev() {
            Some(prev) => match self.db_manager().get_dec_key(prev) {
                Ok(key) => key.into(),
                Err(_) => b"default-randomness".to_vec(),
            },
            None => {
                // Underflow means `initial session`
                return b"initial-randomness".to_vec();
            }
        }
    }
    fn should_end_round(&self, current_session: u64) -> bool { 
        if current_session == 0 {
            tracing::info!("First round");
            return false;
        }
        current_session % Self::ROUND_DURATION == 0 
    }
    fn current_leader(&self, is_sync: bool) -> Result<(Self::Address, String), Self::Error> {
        let current_session = self.db_manager().current_session()?;
        let current_round = self.db_manager().current_round()?;
        let key_generator_list = KeyGeneratorList::<Self::Address>::get(current_round).map_err(Self::Error::from)?;
        let index = if current_session.is_initial() { 0 } else { Self::SelectLeader::select_leader(current_session.into(), key_generator_list.len()).ok_or(RuntimeError::LeaderNotFound)? };
        let key_generator = key_generator_list.get_by_index(index).ok_or(RuntimeError::LeaderNotFound)?;
        if is_sync {
            Ok((key_generator.address(), key_generator.cluster_rpc_url().to_string()))
        } else {
            Ok((key_generator.address(), key_generator.external_rpc_url().to_string()))
        }
    }
    fn sign<T: Serialize>(&self, message: &T) -> Result<Self::Signature, Self::Error> { self.signer().sign_message(message).map_err(|e| KeyServiceError::InvalidSignature(e).into()) }
    fn auth_service(&self) -> &Self::AuthService { &self.auth_service }
    fn key_service(&self) -> &Self::KeyService { self.key_service.as_ref().expect("App not initialized with key service") }
    fn async_task(&self) -> &Self::AsyncTask { &self.task_executor }
    fn db_manager(&self) -> &Self::DbManager { &self.db_manager }
}

pub struct DefaultVerifier;

impl VerifyService<Signature, Address> for DefaultVerifier {
    fn verify_signature<T: Serialize>(signature: &Signature, message: &T) -> Result<Address, SignatureError> {
        let message_bytes = bincode::serialize(message).map_err(SignatureError::SerializeMessage)?;
        let message_hash = hash_message(message_bytes);
        let sig_bytes = signature.as_bytes();
        if sig_bytes.len() != 65 { return Err(SignatureError::InvalidLength(sig_bytes.len()).into()); }
        let mut sig_fixed = sig_bytes.to_vec();
        if sig_fixed[64] < 27 { sig_fixed[64] += 27; }
        let ethers_signature = EthersSignature::try_from(sig_fixed.as_slice())
            .map_err(|_| SignatureError::UnsupportedChainType("Expected Ethereum signature".to_string()))?;
        let recovered_pubkey = ethers_signature.recover(message_hash).map_err(|_| SignatureError::RecoverError)?;

        Ok(Address::from(recovered_pubkey.as_bytes().to_vec()))
    }
}

#[derive(Clone)]
pub struct DefaultDbManager;
impl<Address: AddressT> DbManager<Address> for DefaultDbManager {
    type Error = RuntimeError;
}

#[derive(Clone)]
pub struct DefaultTaskExecutor {
    rpc_client: Arc<RpcClient>,
    sender: Sender<RuntimeEvent<Signature, Address>>,
}

impl DefaultTaskExecutor {
    pub fn new(sender: Sender<RuntimeEvent<Signature, Address>>) -> RuntimeResult<Self> {
        let rpc_client = RpcClient::new().map_err(RuntimeError::from)?;
        Ok(Self { rpc_client: Arc::new(rpc_client), sender })
    }
}

unsafe impl Send for DefaultTaskExecutor {}
unsafe impl Sync for DefaultTaskExecutor {}

#[async_trait]
impl AsyncTask<Signature, Address, RuntimeError> for DefaultTaskExecutor {
    fn spawn_task<Fut>(&self, fut: Fut) -> JoinHandle<()>
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(Box::pin(fut))
    }

    fn spawn_blocking<Fut>(&self, fut: Fut) -> JoinHandle<()>
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        tokio::task::spawn_blocking(move || tokio::runtime::Handle::current().block_on(Box::pin(fut)))
    }

    async fn emit_event(&self, event: RuntimeEvent<Signature, Address>) -> RuntimeResult<()> {
        self.sender.send(event).await.map_err(|e| RuntimeError::from(e))
    }

    async fn request<P, R>(&self, url: String, method: String, parameter: P) -> RuntimeResult<R> 
    where
        P: Serialize + Send + Sync + 'static,
        R: DeserializeOwned + Send + Sync + 'static,
    {
        let rpc_client = self.rpc_client.clone();
        let res = rpc_client.request::<P, R>(url, method, parameter, Id::Null).await.map_err(RuntimeError::from)?;
        return Ok(res);  
    } 
    fn multicast<P>(&self, urls: Vec<String>, method: String, parameter: P) 
    where
        P: Serialize + Send + Sync + 'static
    {
        let rpc_client = self.rpc_client.clone();
        self.spawn_task(Box::pin(
            async move {
                let _ = rpc_client.multicast::<P>(urls, method, &parameter, Id::Null).await.map_err(RuntimeError::from);
            }
        ));
    }
}

/// Simple round robin leader selection
pub struct DefaultSelectLeader;
impl SelectLeader for DefaultSelectLeader {
    fn select_leader(current_session: u64, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        let index = current_session % len as u64;
        Some(index as usize)
    }
}
