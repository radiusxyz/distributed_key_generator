use clap::{Parser, Subcommand};
use distributed_key_generation::{
    error::{self, Error},
    rpc::{
        authority::GetAuthorizedSkdeParams,
        cluster::{self, GetKeyGeneratorList, GetKeyGeneratorRpcUrlListResponse},
        common, external, internal, solver,
    },
    skde_params::fetch_skde_params_with_retry,
    state::AppState,
    task::{
        authority_setup::run_setup_skde_params, single_key_generator::run_single_key_generator,
    },
    types::*,
    utils::log::log_prefix_role_and_address,
};
use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcParameter, RpcServer},
    },
    kvstore::KvStoreBuilder,
};
pub use serde::{Deserialize, Serialize};
// use skde::{delay_encryption::setup, BigUint};
use tokio::task::JoinHandle;
use tracing::info;

#[derive(Debug, Deserialize, Parser, Serialize)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn init() -> Self {
        Cli::parse()
    }
}

#[derive(Subcommand, Debug, Deserialize, Serialize)]
pub enum Commands {
    /// Initializes a node
    Init {
        #[clap(flatten)]
        config_path: Box<ConfigPath>,
    },
    SetupSkdeParams {
        #[clap(flatten)]
        config_path: Box<ConfigPath>,
    },

    /// Starts the node
    Start {
        #[clap(flatten)]
        config_option: Box<ConfigOption>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().with_target(false).init();

    let mut cli = Cli::init();

    match cli.command {
        Commands::Init { ref config_path } => ConfigPath::init(config_path)?,
        // Only for authority node: generate and write SKDE params
        Commands::SetupSkdeParams { ref config_path } => {
            ConfigPath::init(config_path)?; // ensure dir exists
            run_setup_skde_params(config_path);
        }
        Commands::Start {
            ref mut config_option,
        } => {
            // Load the configuration from the path
            let config = Config::load(config_option)?;
            let prefix = log_prefix_role_and_address(&config);

            info!(
                "{} Successfully loaded the configuration file at {:?}.",
                prefix,
                config.path(),
            );

            let skde_params = fetch_skde_params_with_retry(&config).await;

            if config.is_authority() {
                let app_state = AppState::new(config.clone(), skde_params);
                let prefix = log_prefix_role_and_address(app_state.config());

                info!("{} Serving get_authorized_skde_params", prefix);
                let handle = initialize_authority_rpc_server(&app_state).await?;
                handle.await.unwrap();

                return Ok(());
            }

            // Initialize the database
            KvStoreBuilder::default()
                .set_default_lock_timeout(5000)
                .set_txn_lock_timeout(5000)
                .build(config.database_path())
                .map_err(error::Error::Database)?
                .init();

            KeyGeneratorList::initialize().map_err(error::Error::Database)?;
            SessionId::initialize().map_err(error::Error::Database)?;

            info!(
                "{} Successfully initialized the database at {:?}.",
                prefix,
                config.database_path(),
            );

            // If not a leader, get the key generator list from leader
            if let Some(leader_rpc_url) = config.leader_cluster_rpc_url() {
                // Non-leader node
                let rpc_client = RpcClient::new()?;

                let response: GetKeyGeneratorRpcUrlListResponse = rpc_client
                    .request(
                        leader_rpc_url,
                        GetKeyGeneratorList::method(),
                        &GetKeyGeneratorList,
                        Id::Null,
                    )
                    .await?;

                let key_generator_list: KeyGeneratorList =
                    response.key_generator_rpc_url_list.into();

                key_generator_list.put()?;
            }

            // Initialize an application-wide state instance
            let app_state = AppState::new(config.clone(), skde_params);
            let prefix = log_prefix_role_and_address(app_state.config());

            // Based on the role, start appropriate services
            if config.is_leader() {
                info!("{} Starting leader node operations...", prefix);
                run_single_key_generator(app_state.clone());

                info!("{} Initializing solve RPC server on leader...", prefix);
                initialize_solve_rpc_server(&app_state).await?;
            } else if config.is_solver() {
                info!("{} Initializing solve RPC server on solver...", prefix);
                initialize_solve_rpc_server(&app_state).await?;
            }
            // Initialize the internal RPC server
            initialize_internal_rpc_server(&app_state).await?;

            // Initialize the cluster RPC server
            initialize_cluster_rpc_server(&app_state).await?;

            // Initialize the external RPC server
            let server_handle = initialize_external_rpc_server(&app_state).await?;

            server_handle.await.unwrap();
        }
    }

    Ok(())
}

async fn initialize_internal_rpc_server(app_state: &AppState) -> Result<(), Error> {
    let prefix = log_prefix_role_and_address(app_state.config());
    let internal_rpc_url = app_state.config().internal_rpc_url().to_string();

    // Initialize the internal RPC server.
    let internal_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<internal::AddKeyGenerator>()?
        .init(app_state.config().internal_rpc_url().to_string())
        .await
        .map_err(error::Error::RpcServerError)?;

    info!(
        "{} Successfully started the internal RPC server: {}",
        prefix, internal_rpc_url
    );

    tokio::spawn(async move {
        internal_rpc_server.stopped().await;
    });

    Ok(())
}

async fn initialize_cluster_rpc_server(app_state: &AppState) -> Result<(), Error> {
    let prefix = log_prefix_role_and_address(app_state.config());
    let cluster_rpc_url = anywhere(&app_state.config().cluster_port()?);

    let key_generator_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<cluster::GetKeyGeneratorList>()?
        .register_rpc_method::<cluster::SyncKeyGenerator>()?
        .register_rpc_method::<cluster::SyncPartialKey>()?
        .register_rpc_method::<cluster::ClusterSyncFinalizedPartialKeys>()?
        .register_rpc_method::<cluster::SyncDecryptionKey>()?
        .register_rpc_method::<cluster::SubmitPartialKey>()?
        .register_rpc_method::<cluster::RequestSubmitPartialKey>()?
        .register_rpc_method::<common::GetSkdeParams>()?
        .init(cluster_rpc_url.clone())
        .await
        .map_err(error::Error::RpcServerError)?;

    info!(
        "{} Successfully started the cluster RPC server: {}",
        prefix, cluster_rpc_url
    );

    tokio::spawn(async move {
        key_generator_rpc_server.stopped().await;
    });

    Ok(())
}

async fn initialize_external_rpc_server(app_state: &AppState) -> Result<JoinHandle<()>, Error> {
    let prefix = log_prefix_role_and_address(app_state.config());
    let external_rpc_url = anywhere(&app_state.config().external_port()?);

    // Initialize the external RPC server.
    let external_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<external::GetEncryptionKey>()?
        .register_rpc_method::<external::GetDecryptionKey>()?
        .register_rpc_method::<external::GetLatestEncryptionKey>()?
        .register_rpc_method::<external::GetLatestSessionId>()?
        .register_rpc_method::<common::GetSkdeParams>()?
        .init(external_rpc_url.clone())
        .await
        .map_err(error::Error::RpcServerError)?;

    info!(
        "{} Successfully started the external RPC server: {}",
        prefix, external_rpc_url
    );

    let server_handle = tokio::spawn(async move {
        external_rpc_server.stopped().await;
    });

    Ok(server_handle)
}

pub fn anywhere(port: &str) -> String {
    format!("0.0.0.0:{}", port)
}

async fn initialize_authority_rpc_server(app_state: &AppState) -> Result<JoinHandle<()>, Error> {
    let prefix = log_prefix_role_and_address(app_state.config());
    let authority_rpc_url = anywhere(&app_state.config().authority_port()?);

    let rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<GetAuthorizedSkdeParams>()?
        .init(authority_rpc_url.clone())
        .await
        .map_err(Error::RpcServerError)?;

    info!(
        "{} Successfully started the authority RPC server: {}",
        prefix, authority_rpc_url
    );

    let handle = tokio::spawn(async move {
        rpc_server.stopped().await;
    });

    Ok(handle)
}

async fn initialize_solve_rpc_server(app_state: &AppState) -> Result<JoinHandle<()>, Error> {
    let prefix = log_prefix_role_and_address(app_state.config());
    let solver_rpc_url = app_state.config().solver_rpc_url().clone().unwrap();

    let rpc_server_builder = RpcServer::new(app_state.clone());

    let rpc_server = if app_state.config().is_leader() {
        rpc_server_builder
            .register_rpc_method::<common::GetSkdeParams>()?
            .register_rpc_method::<solver::SubmitDecryptionKey>()?
    } else {
        rpc_server_builder
            .register_rpc_method::<solver::SolverSyncFinalizedPartialKeys>()?
    };

    let rpc_server = rpc_server
        .init(solver_rpc_url.clone())
        .await
        .map_err(Error::RpcServerError)?;

    info!("{} Started solve RPC server at {}", prefix, solver_rpc_url);

    let handle = tokio::spawn(async move {
        rpc_server.stopped().await;
    });

    Ok(handle)
}

