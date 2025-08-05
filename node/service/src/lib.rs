use dkg_node_primitives::{BasicDkgService, DefaultTaskExecutor, Role, NodeConfig, DefaultDbManager, DbManager};
use dkg_node_operator::BAppService;
use futures::future::join_all;
use radius_sdk::{signature::{PrivateKeySigner, ChainType, Signature, Address}, kvstore::KvStoreBuilder};
use dkg_primitives::{Config, KeyGenerator, OperatorService, OperatorTrustedSetupFor, RuntimeError, RuntimeResult, SessionEvent, SessionId, Sha3Hasher, TrustedSetupFor};
use std::{fs, path::PathBuf};
use tokio::{signal, sync::mpsc::{channel, Sender}, task::JoinHandle};
use tracing::{info, error};

mod task;
pub use task::*;

#[cfg(feature = "experimental")]
mod builder;

#[cfg(feature = "skde")]
use dkg_node_skde_key_generator::Skde;

async fn create_key_generator<C: Config>(ctx: &C, config: &NodeConfig) -> Result<C::KeyGenerator, C::Error> 
where
    TrustedSetupFor<C>: From<OperatorTrustedSetupFor<C>> + Into<OperatorTrustedSetupFor<C>>,
{
    // If the role is authority, setup the trusted setup 
    if config.role.is_authority() {
        let path = config.trusted_setup_path().join("trusted_setup.json"); 
        match fs::read_to_string(&path) {
            Ok(data) => {
                match serde_json::from_str::<TrustedSetupFor<C>>(&data) {
                    Ok(trusted_setup) => {
                        if let Err(e) = ctx.operator_service().update_trusted_setup(trusted_setup.clone()).await {
                            panic!("Failed to update trusted setup: {}", e);
                        }
                        Ok(C::KeyGenerator::setup(trusted_setup))
                    },
                    Err(e) => { panic!("Failed to parse trusted setup file: {}", e) }
                }
            }
            Err(e) => { panic!("Trusted setup not set for authority node: {}", e) }
        }
    } else {
        loop {
            match ctx.operator_service().get_active_trusted_setup::<TrustedSetupFor<C>>().await {
                Ok(trusted_setup) => {
                    return Ok(C::KeyGenerator::setup(trusted_setup))
                }
                Err(e) => { 
                    error!("Failed to get trusted setup: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

fn create_dkg_service<KG, OS, DB>(config: &NodeConfig, signer: PrivateKeySigner, tx: Sender<SessionEvent<Signature, Address>>, operator_service: OS, db_manager: DB) -> RuntimeResult<BasicDkgService<KG, OS, DB>> 
where
    KG: KeyGenerator + Clone,
    OS: OperatorService<Address> + Clone, 
    DB: DbManager<Address> + Clone + Send + Sync + 'static,
{
    let task_executor = DefaultTaskExecutor::new(tx)?;
    info!("Creating app state for: {:?}", config.role);
    BasicDkgService::<KG, OS, DB>::new(
        signer,
        task_executor,
        config.role.clone(),
        operator_service,
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
    SessionId::new().put()?;
    tracing::info!("Successfully initialized the database at {:?}.", config.db_path);
    Ok(())
}

fn cleanup_db(config: &NodeConfig) {
    if config.is_dev {
        info!("🧹 Cleaning up database at {:?} (dev mode)", config.db_path);
        if let Err(e) = fs::remove_dir_all(&config.db_path) {
            error!("Failed to clean up database at {:?}: {}", config.db_path, e);
        } else {
            info!("✅ Database cleaned up successfully");
        }
    }
}

async fn handle_shutdown(config: NodeConfig, handles: Vec<JoinHandle<()>>) -> RuntimeResult<()> {
    tokio::select! {
        _ = join_all(handles) => {
            error!("❌ Node tasks completed unexpectedly - there may be an issue");
            cleanup_db(&config);
            Ok(())
        },
        _ = signal::ctrl_c() => {
            info!("Ctrl+C received, shutting down...");
            cleanup_db(&config);
            Ok(())
        },
        _ = async {
            if let Ok(mut sigterm) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
                sigterm.recv().await;
            }
        } => {
            info!("Terminate signal received, shutting down...");
            cleanup_db(&config);
            Ok(())
        }
    }
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
    let (mut bapp_service, blockchain_event_rx) = BAppService::new(&config.blockchain_http_rpc_url, &private_key, &config.trusted_address);
    bapp_service.subscribe_events(&config.blockchain_ws_rpc_url).await;
    let mut dkg_service = create_dkg_service::<Skde<Sha3Hasher>, BAppService, DefaultDbManager>(&config, signer, tx.clone(), bapp_service, db_manager)?;
    // TODO: Refactor me! - Key generator should be created in the operator service
    dkg_service.key_generator = Some(create_key_generator(&dkg_service, &config).await?);
    if config.role.is_authority() {
        return Ok(());
    } else {
        init_db(&config)?;
        let mut service_handles = vec![];
        let operator_handle = operator::start_blockchain_operator_worker(&dkg_service, blockchain_event_rx).await?;
        service_handles.push(operator_handle);
        let handles = match config.role {
            Role::Committee => {
                if !dkg_service.operator_service.is_operator(dkg_service.address()).await? {
                    panic!("Node is not registered as a operator");
                }
                committee::run_node(&mut dkg_service, &config, tx, rx).await?
            },
            Role::Solver => {
                if !dkg_service.operator_service.is_solver(dkg_service.address()).await? {
                    panic!("Node is not registered as a solver");
                }
                solver::run_node(&mut dkg_service, &config, rx).await?
            },
            Role::Verifier => unimplemented!("Verifier is not implemented yet"),
            _ => panic!("Invalid role"),
        };
        service_handles.extend(handles);
        let _ = handle_shutdown(config.clone(), service_handles).await?;
    
        Ok(())
    }
}
