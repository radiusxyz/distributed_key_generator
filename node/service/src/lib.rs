use std::{fs, path::PathBuf};

use dkg_node_primitives::{
    Context, ContextInner, DbManager, DefaultDbManager, DefaultTaskExecutor, NodeConfig, Role,
};
use dkg_primitives::{
    Config, KeyGenerator, RuntimeError, RuntimeResult, SessionEvent, SessionId, Sha3Hasher,
    SolverEvent,
};
use futures::future::join_all;
use radius_sdk::{
    kvstore::KvStoreBuilder,
    signature::{Address, ChainType, PrivateKeySigner, Signature},
    validation_service::{
        create_pub_sub_with_signer,
        validation_dkg::{DkgValidation, DkgValidationService},
        RestakingValidation,
    },
};
use tokio::{
    signal,
    sync::mpsc::{channel, Sender},
    task::JoinHandle,
};
use tracing::{error, info};

mod task;
pub use task::*;

#[cfg(feature = "experimental")]
mod builder;

#[cfg(feature = "skde")]
use dkg_node_skde_key_generator::Skde;

async fn init_trusted_setup<C: Config>(ctx: &C, config: &NodeConfig) -> Result<Vec<u8>, C::Error> {
    // If the role is authority, setup the trusted setup
    if config.role.is_authority() {
        let path = config.trusted_setup_path().join("trusted_setup.json");
        match fs::read_to_string(&path) {
            Ok(data) => {
                let bytes = data.into_bytes();
                ctx.validation_service()
                    .update_trusted_setup(bytes.clone())
                    .await
                    .expect("Failed to update trusted setup");
                Ok(bytes)
            }
            Err(e) => {
                panic!("Trusted setup not set for authority node: {}", e)
            }
        }
    } else {
        loop {
            match ctx.validation_service().get_active_trusted_setup().await {
                Ok(bytes) => return Ok(bytes),
                Err(e) => {
                    error!("Failed to get trusted setup: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

async fn create_context<KG, VS, DB>(
    config: &NodeConfig,
    signer: PrivateKeySigner,
    session_event_tx: Sender<SessionEvent<Signature, Address>>,
    solver_event_tx: Sender<SolverEvent<Address>>,
    key_generator: KG,
    validation_service: VS,
    db_manager: DB,
) -> RuntimeResult<Context<KG, VS, DB>>
where
    KG: KeyGenerator + Clone,
    VS: DkgValidation,
    DB: DbManager<Address> + Clone + Send + Sync + 'static,
{
    let task_executor = DefaultTaskExecutor::new(session_event_tx, solver_event_tx)?;
    info!("Creating app state for: {:?}", config.role);
    let session_duration = validation_service
        .get_session_duration()
        .await
        .expect("Failed to get session duration");
    Ok(ContextInner::<KG, VS, DB>::new(
        signer,
        task_executor,
        config.role.clone(),
        session_duration,
        key_generator,
        validation_service,
        db_manager,
    )
    .into())
}

fn create_signer(path: &PathBuf, chain_type: ChainType) -> (PrivateKeySigner, String) {
    match fs::read_to_string(path) {
        Ok(key_string) => {
            let clean_key = key_string.trim().replace("\n", "").replace("\r", "");
            match PrivateKeySigner::from_str(chain_type, &clean_key) {
                Ok(signer) => {
                    tracing::info!("Created signer for: {:?}", path);
                    (signer, clean_key)
                }
                Err(err) => {
                    panic!("Invalid signing key in file: {}", err);
                }
            }
        }
        Err(err) => {
            panic!("Failed to read signing key file: {}", err);
        }
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
    tracing::info!(
        "Successfully initialized the database at {:?}.",
        config.db_path
    );
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
    let (session_event_tx, session_event_rx) = channel(100);
    let (solver_event_tx, solver_event_rx) = channel(100);
    let db_manager = DefaultDbManager;
    let (signer, private_key) = create_signer(&config.private_key_path, config.chain_type);
    let (_pub, _sub) = create_pub_sub_with_signer(
        &config.blockchain_http_rpc_url,
        &config.blockchain_ws_rpc_url,
        &config.trusted_address,
        &private_key,
    )
    .await;
    let dkg_validation_service = DkgValidationService::new(_pub, _sub);
    println!("dkg validation service created");
    let context = create_context::<_, _, _>(
        &config,
        signer,
        session_event_tx.clone(),
        solver_event_tx.clone(),
        Skde::<Sha3Hasher>::new(),
        dkg_validation_service.clone(),
        db_manager,
    )
    .await?;
    let raw_trusted_setup = init_trusted_setup(&context, &config).await?;
    if config.role.is_authority() {
        return Ok(());
    }
    context.set_key_generator(raw_trusted_setup).await;
    let operator = validation::ValidationService::new(context.clone());
    let mut service_handles = vec![];
    let event_handle = tokio::spawn(async move {
        if let Err(e) = dkg_validation_service
            .subscribe_events(|e| operator.handle_callback_events(e))
            .await
        {
            error!("Error subscribing to DKG validation service events: {}", e);
        }
    });
    service_handles.push(event_handle);
    init_db(&config)?;
    let handles = match config.role {
        Role::Committee => {
            if !context
                .validation_service()
                .is_operator(context.address())
                .await?
            {
                panic!("Node is not registered as a operator");
            }
            committee::run_node(
                context,
                &config,
                session_event_tx,
                session_event_rx,
                solver_event_rx,
            )
            .await?
        }
        Role::Solver => {
            if !context
                .validation_service()
                .is_solver(context.address())
                .await?
            {
                panic!("Node is not registered as a solver");
            }
            solver::run_node(context, &config, session_event_rx).await?
        }
        Role::Verifier => unimplemented!("Verifier is not implemented yet"),
        _ => panic!("Invalid role"),
    };
    service_handles.extend(handles);
    let _ = handle_shutdown(config.clone(), service_handles).await?;
    Ok(())
}
