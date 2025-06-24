use dkg_node_primitives::{BasicDkgService, DefaultTaskExecutor, DefaultAuthService, Role, Skde, NodeConfig, DefaultDbManager, DbManager};
use futures::future::join_all;
use radius_sdk::{signature::{PrivateKeySigner, ChainType, Signature, Address}, kvstore::KvStoreBuilder};
use dkg_primitives::{AuthService, AuthTrustedSetupFor, Config, KeyService, Round, RuntimeError, RuntimeEvent, RuntimeResult, SessionId, Sha3Hasher, TrustedSetupFor};
use std::{fs, path::PathBuf};
use tokio::sync::mpsc::{channel, Sender};
use tracing::{info, error};

mod task;
pub use task::*;

#[cfg(feature = "experimental")]
mod builder;

async fn create_key_service<C: Config>(ctx: &C, config: &NodeConfig) -> Result<C::KeyService, C::Error> 
where
    AuthTrustedSetupFor<C>: From<TrustedSetupFor<C>> + Into<TrustedSetupFor<C>>,
{
    // If the role is authority, setup the trusted setup 
    if config.role.is_authority() {
        let path = config.trusted_setup_path().join("trusted_setup.json");  
        match fs::read_to_string(&path) {
            Ok(data) => {
                match serde_json::from_str::<TrustedSetupFor<C>>(&data) {
                    Ok(trusted_setup) => {
                        let signature = ctx.sign(&trusted_setup)?;
                        let signature_bytes = serde_json::to_vec(&signature)?;
                        if let Err(e) = ctx.auth_service().update_trusted_setup(trusted_setup.clone(), signature_bytes).await {
                            panic!("Failed to update trusted setup: {}", e);
                        }
                        Ok(C::KeyService::setup(trusted_setup))
                    },
                    Err(e) => { panic!("Failed to parse trusted setup file: {}", e) }
                }
            }
            Err(e) => { panic!("Trusted setup not set for authority node: {}", e) }
        }
    } else {
        loop {
            match ctx.auth_service().get_trusted_setup::<TrustedSetupFor<C>>().await {
                Ok(trusted_setup) => {
                    return Ok(C::KeyService::setup(trusted_setup))
                }
                Err(e) => { 
                    error!("Failed to get trusted setup: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

fn create_dkg_service<KS, AS, DB>(config: &NodeConfig, signer: PrivateKeySigner, tx: Sender<RuntimeEvent<Signature, Address>>, auth_service: AS, db_manager: DB) -> RuntimeResult<BasicDkgService<KS, AS, DB>> 
where
    KS: KeyService + Clone,
    AS: AuthService<Address> + Clone, 
    DB: DbManager<Address> + Clone + Send + Sync + 'static,
{
    let task_executor = DefaultTaskExecutor::new(tx)?;
    info!("Creating app state for: {:?}", config.role);
    BasicDkgService::<KS, AS, DB>::new(
        signer,
        task_executor,
        config.role.clone(),
        config.threshold,
        auth_service,
        db_manager,
    )
    .map_err(RuntimeError::from)
}

fn create_signer(path: &PathBuf, chain_type: ChainType) -> (PrivateKeySigner, String) {
    match fs::read_to_string(path) {
        Ok(key_string) => {
            let clean_key = key_string.trim().replace("\n", "").replace("\r", "");
            match PrivateKeySigner::from_str(chain_type, &clean_key) {
                Ok(signer) => {
                    tracing::info!("Created signer for: {:?}", path);
                    (signer, clean_key)
                },
                Err(err) => {
                    panic!("Invalid signing key in file: {}", err);
                }
            }
        }
        Err(err) => { panic!("Failed to read signing key file: {}", err); }
    }
}

fn init_db(config: &NodeConfig) -> RuntimeResult<()> {
    KvStoreBuilder::default()
        .set_default_lock_timeout(5000)
        .set_txn_lock_timeout(5000)
        .build(config.db_path.clone())
        .map_err(RuntimeError::Database)?
        .init();
    // Initialize neccessary kv stores
    let session_id = SessionId::new();
    session_id.put()?;
    Round::new().put()?;
    tracing::info!("Successfully initialized the database at {:?}.", config.db_path);
    Ok(())
}

// TODO: Refactor me! - Service Builder pattern
// ```
// let service_builder = ServiceBuilder::new();
// service_builder.add_task(task1);
// service_builder.add_task(task2);
// let service = service_builder.build();
// service.start();
//```
pub async fn run_node(config: NodeConfig) -> RuntimeResult<()> {
    
    info!("{}", config.log());
    let (tx, rx) = channel(10);
    let db_manager = DefaultDbManager;
    let (signer, private_key) = create_signer(&config.private_key_path, config.chain_type);
    let auth_service = DefaultAuthService::new(&config.auth_service_endpoint, &private_key, &config.trusted_address);
    let mut dkg_service = create_dkg_service::<Skde<Sha3Hasher>, DefaultAuthService, DefaultDbManager>(&config, signer, tx, auth_service, db_manager)?;
    dkg_service.key_service = Some(create_key_service(&dkg_service, &config).await?);
    if config.role.is_authority() {
        return Ok(());
    } else {
        init_db(&config)?;

        let handles = match config.role {
            Role::Committee => committee::run_node(&mut dkg_service, config, rx).await?,
            Role::Solver => solver::run_node(&mut dkg_service, config, rx).await?,
            Role::Verifier => unimplemented!("Verifier is not implemented yet"),
            _ => panic!("Invalid role"),
        };
    
        join_all(handles).await;
    
        Ok(())
    }
}
