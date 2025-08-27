use std::sync::Arc;

use async_trait::async_trait;
pub use dkg_primitives::{
    ActiveOperatorList, AddressT, AsyncTask, Config, DbManager, DecKey, KeyGenerator,
    KeyGeneratorError, Parameter, RuntimeError, RuntimeResult, SelectLeader, SessionEvent,
    SessionId, Sha3Hasher, SolverEvent, TraceExt, VerifyService,
};
use dkg_primitives::{DkgEvent, Randomness};
use ethers::{types::Signature as EthersSignature, utils::hash_message};
use futures_util::future::Future;
use radius_sdk::{
    json_rpc::client::{Id, RpcClient},
    signature::{Address, PrivateKeySigner, Signature, SignatureError},
    validation_service::DkgValidation,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::{
    sync::{mpsc::Sender, RwLock},
    task::JoinHandle,
};

pub use crate::NodeConfig;

pub mod config;
pub use config::*;

#[derive(Clone)]
pub struct Context<KG, VS, DB> {
    inner: Arc<ContextInner<KG, VS, DB>>,
}

impl<KG: KeyGenerator, VS, DB> Context<KG, VS, DB> {
    pub async fn set_key_generator(&self, trusted_setup: Vec<u8>) {
        let shared = self.inner.key_generator.clone();
        let mut kg = shared.write().await;
        *kg = KG::setup(trusted_setup).expect("Failed to initialize key generator");
    }
}

/// Instance of DKG service
pub struct ContextInner<KG, VS, DB> {
    signer: PrivateKeySigner,
    task_executor: DefaultTaskExecutor,
    role: Role,
    session_duration: u64,
    pub key_generator: Arc<RwLock<KG>>,
    pub validation_service: VS,
    pub db_manager: DB,
}

impl<KG, VS, DB> ContextInner<KG, VS, DB> {
    pub fn new(
        signer: PrivateKeySigner,
        task_executor: DefaultTaskExecutor,
        role: Role,
        session_duration: u64,
        key_generator: KG,
        validation_service: VS,
        db_manager: DB,
    ) -> Self {
        Self {
            signer,
            task_executor,
            role,
            key_generator: Arc::new(RwLock::new(key_generator)),
            validation_service,
            db_manager,
            session_duration,
        }
    }

    pub fn task_executor(&self) -> &DefaultTaskExecutor {
        &self.task_executor
    }
}

impl<KG, VS, DB> From<ContextInner<KG, VS, DB>> for Context<KG, VS, DB> {
    fn from(value: ContextInner<KG, VS, DB>) -> Self {
        Self {
            inner: Arc::new(value),
        }
    }
}

impl<KG, VS, DB> Config for Context<KG, VS, DB>
where
    KG: KeyGenerator + Parameter,
    VS: DkgValidation<Address = Address, Key = u64, Proof = Vec<u8>> + Clone + Send + Sync + 'static,
    DB: DbManager<Address, Error = RuntimeError> + Clone + Send + Sync + 'static,
{
    type Address = Address;
    type Signature = Signature;
    type SelectLeader = DefaultSelectLeader;
    type VerifyService = DefaultVerifier;
    type KeyGenerator = KG;
    type ValidationService = VS;
    type AsyncTask = DefaultTaskExecutor;
    type DbManager = DB;
    type Hasher = Sha3Hasher;
    type Error = RuntimeError;

    fn is_leader(&self, session_id: SessionId) -> bool {
        match self.current_leader(session_id, false) {
            Ok((leader, _)) => leader == self.address(),
            Err(_) => false,
        }
    }
    fn is_solver(&self) -> bool {
        self.inner.role == Role::Solver
    }
    fn signer(&self) -> &PrivateKeySigner {
        &self.inner.signer
    }
    fn address(&self) -> Address {
        self.inner.signer.address().clone()
    }
    fn session_duration(&self) -> u64 {
        self.inner.session_duration
    }
    fn randomness(&self, session_id: SessionId) -> Vec<u8> {
        if let Ok(randomness) = Randomness::get(session_id) {
            randomness.into()
        } else {
            b"initial-randomness".to_vec()
        }
    }
    fn should_force_generating(&self) -> Result<bool, Self::Error> {
        let operator_list = ActiveOperatorList::<Self::Address>::get()?;
        Ok(operator_list.len() == 1)
    }
    fn current_leader(
        &self,
        session_id: SessionId,
        is_sync: bool,
    ) -> Result<(Self::Address, String), Self::Error> {
        let operator_list = ActiveOperatorList::<Self::Address>::get()?;
        let index = if session_id.is_initial() {
            0
        } else {
            Self::SelectLeader::select_leader(session_id.into(), operator_list.len())
                .ok_or(RuntimeError::LeaderNotFound)?
        };
        let operator = operator_list
            .get_by_index(index)
            .ok_or(RuntimeError::LeaderNotFound)?;
        if is_sync {
            Ok((operator.address(), operator.cluster_rpc_url().to_string()))
        } else {
            Ok((operator.address(), operator.external_rpc_url().to_string()))
        }
    }
    fn sign<T: Serialize>(&self, message: &T) -> Result<Self::Signature, Self::Error> {
        self.signer()
            .sign_message(message)
            .map_err(|e| KeyGeneratorError::InvalidSignature(e).into())
    }
    fn validation_service(&self) -> &Self::ValidationService {
        &self.inner.validation_service
    }
    fn key_generator(&self) -> Arc<RwLock<Self::KeyGenerator>> {
        self.inner.key_generator.clone()
    }
    fn async_task(&self) -> &Self::AsyncTask {
        &self.inner.task_executor
    }
    fn db_manager(&self) -> &Self::DbManager {
        &self.inner.db_manager
    }
}

pub struct DefaultVerifier;

impl VerifyService<Signature, Address> for DefaultVerifier {
    fn verify_signature<T: Serialize>(
        signature: &Signature,
        message: &T,
    ) -> Result<Address, SignatureError> {
        let message_bytes =
            bincode::serialize(message).map_err(SignatureError::SerializeMessage)?;
        let message_hash = hash_message(message_bytes);
        let sig_bytes = signature.as_bytes();
        if sig_bytes.len() != 65 {
            return Err(SignatureError::InvalidLength(sig_bytes.len()).into());
        }
        let mut sig_fixed = sig_bytes.to_vec();
        if sig_fixed[64] < 27 {
            sig_fixed[64] += 27;
        }
        let ethers_signature = EthersSignature::try_from(sig_fixed.as_slice()).map_err(|_| {
            SignatureError::UnsupportedChainType("Expected Ethereum signature".to_string())
        })?;
        let recovered_pubkey = ethers_signature
            .recover(message_hash)
            .map_err(|_| SignatureError::RecoverError)?;

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
    solver_event_tx: Sender<SolverEvent<Address>>,
}

impl DefaultTaskExecutor {
    pub fn new(
        session_event_tx: Sender<SessionEvent<Signature, Address>>,
        solver_event_tx: Sender<SolverEvent<Address>>,
    ) -> RuntimeResult<Self> {
        let rpc_client = RpcClient::new().map_err(RuntimeError::from)?;
        Ok(Self {
            rpc_client: Arc::new(rpc_client),
            session_event_tx,
            solver_event_tx,
        })
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
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(Box::pin(fut))
        })
    }

    async fn emit_event(&self, event: DkgEvent<Signature, Address>) -> RuntimeResult<()> {
        match event {
            DkgEvent::SessionEvent(event) => self
                .session_event_tx
                .send(event)
                .await
                .map_err(|e| RuntimeError::AnyError(Box::new(e))),
            DkgEvent::SolverEvent(event) => self
                .solver_event_tx
                .send(event)
                .await
                .map_err(|e| RuntimeError::AnyError(Box::new(e))),
        }
    }

    async fn request<P, R>(&self, url: String, method: String, parameter: P) -> RuntimeResult<R>
    where
        P: Serialize + Send + Sync + 'static,
        R: DeserializeOwned + Send + Sync + 'static,
    {
        let rpc_client = self.rpc_client.clone();
        let res = rpc_client
            .request::<P, R>(url, method, parameter, Id::Null)
            .await
            .map_err(RuntimeError::from)?;
        return Ok(res);
    }
    fn multicast<P>(&self, urls: Vec<String>, method: String, parameter: P)
    where
        P: Serialize + Send + Sync + 'static,
    {
        let rpc_client = self.rpc_client.clone();
        self.spawn_task(Box::pin(async move {
            let _ = rpc_client
                .multicast::<P>(urls, method, &parameter, Id::Null)
                .await
                .map_err(RuntimeError::from);
        }));
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
