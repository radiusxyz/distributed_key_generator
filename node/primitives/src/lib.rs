pub use crate::NodeConfig;
use std::sync::Arc;
use dkg_primitives::DkgEvent;
pub use dkg_primitives::{
    Config, DecKey, ActiveOperatorList, VerifyService, SelectLeader, AsyncTask, RuntimeResult, RuntimeError,
    TraceExt, KeyGeneratorError, SessionId, Parameter, SessionEvent, TrustedSetupFor, SolverEvent,
    OperatorService, OperatorServiceError, KeyGenerator, DbManager, AddressT
};
use radius_sdk::{signature::{PrivateKeySigner, Address, Signature, SignatureError}, json_rpc::client::{RpcClient, Id}};
use ethers::{types::Signature as EthersSignature, utils::hash_message};
use serde::{Serialize, de::DeserializeOwned};
use futures_util::future::Future;
use tokio::{task::JoinHandle, sync::mpsc::Sender};
use async_trait::async_trait;

pub mod config;
pub use config::*;


#[derive(Clone)]
/// Instance of DKG service
pub struct BasicDkgService<KG, OS, DB> {
    signer: PrivateKeySigner,
    task_executor: DefaultTaskExecutor,
    role: Role,
    session_duration: u64,
    pub key_generator: Option<KG>,
    pub operator_service: OS,
    pub db_manager: DB,
}

impl<KG, OS, DB> BasicDkgService<KG, OS, DB> {
    pub fn new(
        signer: PrivateKeySigner,
        task_executor: DefaultTaskExecutor,
        role: Role,
        session_duration: u64,
        operator_service: OS,
        db_manager: DB,
    ) -> RuntimeResult<Self> {        
        Ok(Self { signer, task_executor, role, key_generator: None, operator_service, db_manager, session_duration })
    }

    pub fn task_executor(&self) -> &DefaultTaskExecutor {
        &self.task_executor
    }
}

impl<KG, OS, DB> Config for BasicDkgService<KG, OS, DB> 
where
    KG: KeyGenerator + Parameter,
    OS: OperatorService<Address> + Clone,
    DB: DbManager<Address, Error = RuntimeError> + Clone + Send + Sync + 'static,
    KG::TrustedSetUp: From<OS::TrustedSetup> + Into<OS::TrustedSetup>,
{
    type Address = Address;
    type Signature = Signature;
    type SelectLeader = DefaultSelectLeader;
    type VerifyService = DefaultVerifier;
    type KeyGenerator = KG;
    type OperatorService = OS;
    type AsyncTask = DefaultTaskExecutor;
    type DbManager = DB;
    type Error = RuntimeError;

    fn is_leader(&self, session_id: SessionId) -> bool { 
        match self.current_leader(session_id, false) {
            Ok((leader, _)) => leader == self.address(),
            Err(_) => false,
        }    
    }
    fn is_solver(&self) -> bool { self.role == Role::Solver }
    fn signer(&self) -> &PrivateKeySigner { &self.signer }
    fn address(&self) -> Address { self.signer.address().clone() }
    fn session_duration(&self) -> u64 { self.session_duration }
    fn randomness(&self, session_id: SessionId) -> Vec<u8> {
        match session_id.prev() {
            Some(prev) => match self.db_manager().get_dec_key(prev) {
                Ok(key) => key.into(),
                Err(_) => b"default-randomness".to_vec(),
            },
            None => {
                // Underflow or initial session
                return b"initial-randomness".to_vec();
            }
        }
    }
    fn should_force_generating(&self) -> Result<bool, Self::Error> { 
        let operator_list = ActiveOperatorList::<Self::Address>::get()?;
        Ok(operator_list.len() == 1)
    }
    fn current_leader(&self, session_id: SessionId, is_sync: bool) -> Result<(Self::Address, String), Self::Error> {
        let operator_list = ActiveOperatorList::<Self::Address>::get()?;
        let index = if session_id.is_initial() { 0 } else { Self::SelectLeader::select_leader(session_id.into(), operator_list.len()).ok_or(RuntimeError::LeaderNotFound)? };
        let operator = operator_list.get_by_index(index).ok_or(RuntimeError::LeaderNotFound)?;
        if is_sync {
            Ok((operator.address(), operator.cluster_rpc_url().to_string()))
        } else {
            Ok((operator.address(), operator.external_rpc_url().to_string()))
        }
    }
    fn sign<T: Serialize>(&self, message: &T) -> Result<Self::Signature, Self::Error> { self.signer().sign_message(message).map_err(|e| KeyGeneratorError::InvalidSignature(e).into()) }
    fn operator_service(&self) -> &Self::OperatorService { &self.operator_service }
    fn key_generator(&self) -> &Self::KeyGenerator { self.key_generator.as_ref().expect("App not initialized with key generator") }
    fn key_generator_mut(&mut self) -> &mut Self::KeyGenerator { self.key_generator.as_mut().expect("App not initialized with key generator") }
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
    session_event_tx: Sender<SessionEvent<Signature, Address>>,
    solver_event_tx: Sender<SolverEvent>,
}

impl DefaultTaskExecutor {
    pub fn new(session_event_tx: Sender<SessionEvent<Signature, Address>>, solver_event_tx: Sender<SolverEvent>) -> RuntimeResult<Self> {
        let rpc_client = RpcClient::new().map_err(RuntimeError::from)?;
        Ok(Self { rpc_client: Arc::new(rpc_client), session_event_tx, solver_event_tx })
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

    async fn emit_event(&self, event: DkgEvent<Signature, Address>) -> RuntimeResult<()> {
        match event {
            DkgEvent::SessionEvent(event) => self.session_event_tx.send(event).await.map_err(|e| RuntimeError::AnyError(Box::new(e))),
            DkgEvent::SolverEvent(event) => self.solver_event_tx.send(event).await.map_err(|e| RuntimeError::AnyError(Box::new(e))),
        }
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
