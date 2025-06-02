use dkg_node_primitives::{Config, DkgAppState, DkgExecutor, DkgAuth, Role, Skde};
use futures::future::join_all;
use radius_sdk::{signature::{PrivateKeySigner, ChainType, Signature, Address}, kvstore::KvStoreBuilder};
use dkg_primitives::{AppState, TrustedSetupFor, ConfigError, Error, Event, KeyGeneratorList, SecureBlock, SessionId, Sha3Hasher, AuthService};
use std::{fs, path::PathBuf};
use tokio::sync::mpsc::{channel, Sender};
use tracing::{info, error};

mod task;
pub use task::*;

#[cfg(feature = "experimental")]
mod builder;

async fn create_secure_block<C: AppState>(ctx: &C, config: &Config) -> Result<C::SecureBlock, C::Error> {
    // If the role is authority, setup the trusted setup 
    if config.role == Role::Authority {
        let path = config.trusted_setup_path().join("trusted_setup.json");  
        match fs::read_to_string(&path) {
            Ok(data) => {
                match serde_json::from_str::<TrustedSetupFor<C>>(&data) {
                    Ok(trusted_setup) => {
                        let signature = ctx.sign(&trusted_setup)?;
                        let trusted_setup_bytes = serde_json::to_vec(&trusted_setup)?;
                        let signature_bytes = serde_json::to_vec(&signature)?;
                        match ctx.auth_service().update_trusted_setup(trusted_setup_bytes, signature_bytes).await {
                            Ok(_) => {
                                info!("Successfully updated trusted setup");
                            }
                            Err(e) => { panic!("Failed to update trusted setup: {}", e) }
                        }
                        Ok(C::SecureBlock::setup(trusted_setup))
                    },
                    Err(e) => { panic!("Failed to parse trusted setup file: {}", e) }
                }
            }
            Err(e) => { panic!("Trusted setup not set for authority node: {}", e) }
        }
    } else {
        loop {
            match ctx.auth_service().get_trusted_setup().await {
                Ok(trusted_setup) => {
                    let trusted_setup = serde_json::from_slice::<TrustedSetupFor<C>>(&trusted_setup).map_err(|e| C::Error::from(e))?;
                    return Ok(C::SecureBlock::setup(trusted_setup))
                }
                Err(e) => { 
                    error!("Failed to get trusted setup: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

fn create_app_state<SB, AS>(config: &Config, tx: Sender<Event<Signature, Address>>, auth_service: AS) -> Result<DkgAppState<SB, AS>, Error> 
where
    SB: SecureBlock,
    AS: AuthService<Address>, 
{
    let signer = create_signer(&config.private_key_path, config.chain_type);
    let executor = DkgExecutor::new(tx)?;
    info!("Creating app state for: {:?}", config.role);
    DkgAppState::<SB, AS>::new(
        signer,
        executor,
        config.role.clone(),
        config.threshold,
        auth_service,
    )
    .map_err(Error::from)
}

fn create_signer(path: &PathBuf, chain_type: ChainType) -> PrivateKeySigner {
    match fs::read_to_string(path) {
        Ok(key_string) => {
            let clean_key = key_string.trim().replace("\n", "").replace("\r", "");
            match PrivateKeySigner::from_str(chain_type, &clean_key) {
                Ok(signer) => {
                    tracing::info!("Created signer for: {:?}", path);
                    signer
                },
                Err(err) => {
                    panic!("Invalid signing key in file: {}", err);
                }
            }
        }
        Err(err) => { panic!("Failed to read signing key file: {}", err); }
    }
}

fn init_db(config: &Config) -> Result<(), Error> {
    KvStoreBuilder::default()
        .set_default_lock_timeout(5000)
        .set_txn_lock_timeout(5000)
        .build(config.db_path.clone())
        .map_err(Error::Database)?
        .init();
    KeyGeneratorList::<Address>::default().put()?;
    SessionId::initialize().map_err(Error::Database)?;
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
pub async fn run_node(config: Config) -> Result<(), Error> {

    init_db(&config)?;
    
    let (tx, rx) = channel(10);
    let auth_service = DkgAuth::new(&config.auth_service_endpoint, &config.trusted_address);
    let mut app_state = create_app_state::<Skde<Sha3Hasher>, DkgAuth>(&config, tx, auth_service)?;
    app_state.secure_block = Some(create_secure_block(&app_state, &config).await?);

    if !config.validate() {
        return Err(Error::Config(ConfigError::InvalidConfig));
    }

    let handles = match config.role {
        Role::Authority => authority::run_node(&mut app_state, config).await?,
        Role::Committee => committee::run_node(&mut app_state, config, rx).await?,
        Role::Solver => solver::run_node(&mut app_state, config).await?,
        _ => panic!("Invalid role"),
    };

    join_all(handles).await;

    Ok(())
}
