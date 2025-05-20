pub use crate::Config;
pub use dkg_primitives::{
    AppState, Verify, TaskSpawner, Error, TraceExt, KeyGenerationError, SessionId,
    DecryptionKey,
};
use radius_sdk::signature::PrivateKeySigner;
pub use radius_sdk::signature::{Address, Signature, SignatureError};
use ethers::{types::Signature as EthersSignature, utils::hash_message};
use serde::{Serialize, Deserialize};
use skde::delay_encryption::{decrypt, encrypt, SkdeParams};
use futures_util::future::BoxFuture;
use tokio::task::JoinHandle;

pub mod config;
pub use config::*;

#[cfg(feature = "experimental")]
mod randomness;
#[cfg(feature = "experimental")]
mod key;


#[derive(Clone)]
pub struct DkgAppState {
    leader_rpc_url: Option<String>,
    signer: PrivateKeySigner,
    skde_params: Option<SkdeParams>,
    task_spawner: DkgExecutor,
    role: Role,
}

impl DkgAppState {
    pub fn new(
        leader_rpc_url: Option<String>,
        signer: PrivateKeySigner,
        task_spawner: DkgExecutor,
        role: Role,
    ) -> Self {
        Self { leader_rpc_url, signer, skde_params: None, task_spawner, role }
    }

    pub fn with_skde_params(&mut self, skde_params: SkdeParams) {
        self.skde_params = Some(skde_params);
    }

    pub fn task_spawner(&self) -> &DkgExecutor {
        &self.task_spawner
    }
}

impl AppState for DkgAppState {
    type Address = Address;
    type SessionId = SessionId;
    type Signature = Signature;
    type Verify = DkgVerify;
    type TaskSpawner = DkgExecutor;
    type Error = Error;

    fn is_leader(&self) -> bool {
        self.role == Role::Leader
    }

    fn is_solver(&self) -> bool {
        self.role == Role::Solver
    }

    fn leader_rpc_url(&self) -> Option<String> {
        self.leader_rpc_url.clone()
    }

    fn signer(&self) -> PrivateKeySigner {
        self.signer.clone()
    }

    fn address(&self) -> Address {
        self.signer.address().clone()
    }

    fn skde_params(&self) -> SkdeParams {
        // This should never be None
        self.skde_params.clone().expect("SKDE params not initialized")
    }

    fn log_prefix(&self) -> String {
        format!("[{}][{:?}]", self.role, self.address())
    }

    fn sign<T: Serialize>(&self, message: &T) -> Result<Self::Signature, Self::Error> {
        self.signer().sign_message(message).map_err(Self::Error::from)
    }

    fn task_spawner(&self) -> &Self::TaskSpawner {
        &self.task_spawner
    }
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

    fn verify_decryption_key(
        skde_params: &skde::delay_encryption::SkdeParams,
        encryption_key: String,
        decryption_key: String,
        prefix: &str,
    ) -> Result<(), dkg_primitives::KeyGenerationError> {
        let sample_message = "sample_message";
        let ciphertext = encrypt(skde_params, sample_message, &encryption_key, true)
            .ok_or_trace()
            .ok_or_else(|| KeyGenerationError::InternalError("Encryption failed".into()))?;
        let decrypted_message = match decrypt(skde_params, &ciphertext, &decryption_key) {
            Ok(message) => message,
            Err(err) => {
                tracing::error!("{} Decryption failed: {}", prefix, err);
                return Err(KeyGenerationError::InternalError(
                    format!("Decryption failed: {}", err).into(),
                ));
            }
        };

        if decrypted_message.as_str() != sample_message {
            return Err(KeyGenerationError::InternalError(
                "Decryption failed: message mismatch".into(),
            ));
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct DkgExecutor;

unsafe impl Send for DkgExecutor {}
unsafe impl Sync for DkgExecutor {}

impl TaskSpawner for DkgExecutor {
    fn spawn_task(&self, fut: BoxFuture<'static, ()>) -> JoinHandle<()> {
        tokio::spawn(fut)
    }

    fn spawn_blocking(&self, fut: BoxFuture<'static, ()>) -> JoinHandle<()> {
        tokio::task::spawn_blocking(move || tokio::runtime::Handle::current().block_on(fut))
    }
}

/// Node roles in the DKG network
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum Role {
    /// Leader node responsible for collecting partial keys and coordinating
    Leader,
    /// Committee node that generates partial keys
    Committee,
    /// Solver node that computes decryption keys
    Solver,
    /// Verifier node that monitors the network for Byzantine behavior
    Verifier,
    /// Authority node that conducts the secure skde parameter setup
    Authority,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Leader => write!(f, "leader"),
            Role::Committee => write!(f, "committee"),
            Role::Solver => write!(f, "solver"),
            Role::Verifier => write!(f, "verifier"),
            Role::Authority => write!(f, "authority"),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "leader" => Ok(Role::Leader),
            "committee" => Ok(Role::Committee),
            "solver" => Ok(Role::Solver),
            "verifier" => Ok(Role::Verifier),
            "authority" => Ok(Role::Authority),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}
