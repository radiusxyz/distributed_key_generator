
use dkg_node_primitives::{Config, DkgAppState, DkgExecutor, Role};
use radius_sdk::{signature::{PrivateKeySigner, ChainType, Address}, kvstore::KvStoreBuilder};
use dkg_primitives::{Error, KeyGeneratorList, SessionId};
use std::{fs, path::PathBuf};

mod task;
pub use task::*;

#[cfg(feature = "experimental")]
mod builder;

fn create_app_state(config: &Config) -> DkgAppState {
    let signer = create_signer(&config.private_key_path, config.chain_type);
    let executor = DkgExecutor;
    DkgAppState::new(
        config.maybe_leader_rpc_url.clone(),
        signer,
        executor,
        config.role.clone(),
    )
}

fn create_signer(path: &PathBuf, chain_type: ChainType) -> PrivateKeySigner {
    match fs::read_to_string(path) {
        Ok(key_string) => {
            let clean_key = key_string.trim().replace("\n", "").replace("\r", "");
            match PrivateKeySigner::from_str(chain_type, &clean_key) {
                Ok(signer) => signer,
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

pub async fn build_node(config: Config) -> Result<(), Error> {
    // Run common service(e.g Open database)
    // Initialize the database
    init_db(&config)?;
    let mut app_state = create_app_state(&config);
    // TODO: Refactor me!
    match config.role {
        Role::Authority => authority::run_node(&mut app_state, config).await?,
        Role::Committee => committee::run_node(&mut app_state, config).await?,
        Role::Leader => leader::run_node(&mut app_state, config).await?,
        Role::Solver => solver::run_node(&mut app_state, config).await?,
        _ => panic!("Invalid role"),
    }

    // This will spawn all required tasks and return the handler
    // Ok(service_builder.build())

    Ok(())
}
