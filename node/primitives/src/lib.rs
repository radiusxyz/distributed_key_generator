pub use crate::Config;
use std::sync::Arc;
pub use dkg_primitives::{AppState, DecKey, Verify, AsyncTask, Error, TraceExt, KeyGenerationError, SessionId, Parameter, Event, SecureBlock, TrustedSetupFor};
use radius_sdk::{signature::{PrivateKeySigner, Address, Signature, SignatureError}, json_rpc::client::{RpcClient, Id}};
use ethers::{types::Signature as EthersSignature, utils::hash_message};
use serde::{Serialize, de::DeserializeOwned};
use futures_util::future::Future;
use tokio::{task::JoinHandle, sync::mpsc::Sender};
use async_trait::async_trait;

mod secure_block;
pub use secure_block::*;

pub mod config;
pub use config::*;

#[cfg(feature = "experimental")]
mod randomness;
#[cfg(feature = "experimental")]
mod key;


#[derive(Clone)]
pub struct DkgAppState<SB> {
    leader_rpc_url: Option<String>,
    signer: PrivateKeySigner,
    task_spawner: DkgExecutor,
    role: Role,
    threshold: u16,
    secure_block: SB,
}

impl<SB: SecureBlock> DkgAppState<SB> {
    pub fn new(
        leader_rpc_url: Option<String>,
        signer: PrivateKeySigner,
        task_spawner: DkgExecutor,
        role: Role,
        threshold: u16,
    ) -> Result<Self, Error> {
        let secure_block = SB::setup();
        Ok(Self { leader_rpc_url, signer, task_spawner, role, threshold, secure_block })
    }

    pub fn task_spawner(&self) -> &DkgExecutor {
        &self.task_spawner
    }
}

impl<SB: SecureBlock + Parameter> AppState for DkgAppState<SB> {
    type Address = Address;
    type Signature = Signature;
    type Verify = DkgVerify;
    type SecureBlock = SB;
    type AsyncTask = DkgExecutor;
    type Error = Error;

    fn threshold(&self) -> u16 { self.threshold }
    fn is_leader(&self) -> bool { self.role == Role::Leader }
    fn is_solver(&self) -> bool { self.role == Role::Solver }
    fn leader_rpc_url(&self) -> Option<String> { self.leader_rpc_url.clone() }
    fn signer(&self) -> PrivateKeySigner { self.signer.clone() }
    fn address(&self) -> Address { self.signer.address().clone() }
    fn log_prefix(&self) -> String { format!("{}", self.role) }
    fn randomness(&self, session_id: SessionId) -> Vec<u8> {
        match session_id.prev() {
            Some(prev) => match DecKey::get(prev) {
                Ok(key) => key.into(),
                Err(_) => b"default-randomness".to_vec(),
            },
            None => {
                // Underflow means `initial session`
                return b"initial-randomness".to_vec();
            }
        }
    }
    fn sign<T: Serialize>(&self, message: &T) -> Result<Self::Signature, Self::Error> { self.signer().sign_message(message).map_err(Self::Error::from) }
    fn secure_block(&self) -> &Self::SecureBlock { &self.secure_block }
    fn async_task(&self) -> &Self::AsyncTask { &self.task_spawner }
}

pub struct DkgVerify;

impl Verify<Signature, Address> for DkgVerify {
    fn verify_signature<T: Serialize>(signature: &Signature, message: &T) -> Result<Address, SignatureError> {
        let message_bytes = bincode::serialize(message).map_err(SignatureError::SerializeMessage)?;
        let message_hash = hash_message(message_bytes);
        let sig_bytes = signature.as_bytes();
        if sig_bytes.len() != 65 {
            return Err(SignatureError::UnsupportedChainType(
                "Invalid signature length".to_string(),
            ));
        }
        let mut sig_fixed = sig_bytes.to_vec();
        if sig_fixed[64] < 27 {
            sig_fixed[64] += 27;
        }
        let ethers_signature = EthersSignature::try_from(sig_fixed.as_slice())
            .map_err(|_| SignatureError::UnsupportedChainType("Signature parse failed".to_string()))?;

        let recovered_pubkey = ethers_signature.recover(message_hash).map_err(|_| {
            SignatureError::UnsupportedChainType("Signature recover failed".to_string())
        })?;

        Ok(Address::from(recovered_pubkey.as_bytes().to_vec()))
    }
}

#[derive(Clone)]
pub struct DkgExecutor {
    rpc_client: Arc<RpcClient>,
    sender: Sender<Event<Signature, Address>>,
}

impl DkgExecutor {
    pub fn new(sender: Sender<Event<Signature, Address>>) -> Result<Self, Error> {
        let rpc_client = RpcClient::new().map_err(Error::from)?;
        Ok(Self { rpc_client: Arc::new(rpc_client), sender })
    }
}

unsafe impl Send for DkgExecutor {}
unsafe impl Sync for DkgExecutor {}

#[async_trait]
impl AsyncTask<Signature, Address, Error> for DkgExecutor {
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

    async fn emit_event(&self, event: Event<Signature, Address>) -> Result<(), Error> {
        self.sender.send(event).await.map_err(|e| Error::from(e))
    }

    async fn request<P, R>(&self, url: String, method: String, parameter: P) -> Result<R, Error> 
    where
        P: Serialize + Send + Sync + 'static,
        R: DeserializeOwned + Send + Sync + 'static,
    {
        let rpc_client = self.rpc_client.clone();
        let res = rpc_client.request::<P, R>(url, method, parameter, Id::Null).await.map_err(Error::from)?;
        return Ok(res);  
    } 
    fn multicast<P>(&self, urls: Vec<String>, method: String, parameter: P) 
    where
        P: Serialize + Send + Sync + 'static
    {
        let rpc_client = self.rpc_client.clone();
        self.spawn_task(Box::pin(
            async move {
                let _ = rpc_client.multicast::<P>(urls, method, &parameter, Id::Null).await.map_err(Error::from);
            }
        ));
    }
}