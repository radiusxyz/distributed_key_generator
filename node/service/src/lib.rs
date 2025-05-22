use dkg_node_primitives::{Config, DkgAppState, DkgExecutor, Role};
use futures::future::join_all;
use radius_sdk::{signature::{PrivateKeySigner, ChainType, Signature, Address}, kvstore::KvStoreBuilder};
use dkg_primitives::{Error, KeyGeneratorList, SessionId, ConfigError, Event};
use std::{fs, path::PathBuf};
use tokio::sync::mpsc::{channel, Sender};

mod task;
pub use task::*;

#[cfg(feature = "experimental")]
mod builder;

fn create_app_state(config: &Config, tx: Sender<Event<Signature, Address>>) -> Result<DkgAppState, Error> {
    let signer = create_signer(&config.private_key_path, config.chain_type);
    let executor = DkgExecutor::new(tx)?;
    tracing::info!("Creating app state for: {:?}", config.role);
    DkgAppState::new(
        config.maybe_leader_rpc_url.clone(),
        signer,
        executor,
        config.role.clone(),
        config.threshold,
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
    let mut app_state = create_app_state(&config, tx)?;
    if !config.validate() {
        return Err(Error::Config(ConfigError::InvalidConfig));
    }
    let handles = match config.role {
        Role::Authority => authority::run_node(&mut app_state, config).await?,
        Role::Committee => committee::run_node(&mut app_state, config).await?,
        Role::Leader => leader::run_node(&mut app_state, config, rx).await?,
        Role::Solver => solver::run_node(&mut app_state, config).await?,
        _ => panic!("Invalid role"),
    };

    join_all(handles).await;

    Ok(())
}
