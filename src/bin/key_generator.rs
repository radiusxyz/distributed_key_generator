use std::collections::HashMap;

use clap::{Parser, Subcommand};
use key_management_system::{
    client::key_generator::KeyGeneratorClient,
    error::{self, Error},
    rpc::{cluster, external, internal},
    state::AppState,
    types::{
        Address, Config, ConfigOption, ConfigPath, KeyGeneratorAddressListModel, KeyGeneratorModel,
    },
};
use radius_sequencer_sdk::{json_rpc::RpcServer, kvstore::KvStore as Database};
pub use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

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

    /// Starts the node
    Start {
        #[clap(flatten)]
        config_option: Box<ConfigOption>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().init();

    let mut cli = Cli::init();

    match cli.command {
        Commands::Init { ref config_path } => ConfigPath::init(config_path)?,
        Commands::Start {
            ref mut config_option,
        } => {
            // Load the configuration from the path
            let config = Config::load(config_option)?;

            tracing::info!(
                "Successfully loaded the configuration file at {:?}.",
                config.path(),
            );

            // Initialize the database
            Database::new(config.database_path())
                .map_err(error::Error::Database)?
                .init();
            tracing::info!(
                "Successfully initialized the database at {:?}.",
                config.database_path(),
            );

            let mut key_generator_address_list =
                KeyGeneratorAddressListModel::get_or_default().map_err(error::Error::Database)?;

            if config.seed_cluster_rpc_url().is_some() {
                // Initialize the cluster RPC server
                let seed_key_generator_client =
                    KeyGeneratorClient::new(config.seed_cluster_rpc_url().clone().unwrap())
                        .map_err(error::Error::RpcError)?;

                let key_generator_list = seed_key_generator_client.get_key_generator_list().await?;

                tracing::info!("Sync key generators {:?}.", key_generator_list);

                key_generator_list.iter().for_each(|key_generator| {
                    let _ = KeyGeneratorModel::put(key_generator);

                    key_generator_address_list.insert(key_generator.address().clone());
                });

                KeyGeneratorAddressListModel::put(&key_generator_address_list)
                    .map_err(error::Error::Database)?;
            }

            let key_generator_clients = key_generator_address_list
                .iter()
                .map(
                    |key_generator_address| -> Result<(Address, KeyGeneratorClient), Error> {
                        let key_generator = KeyGeneratorModel::get(key_generator_address)
                            .map_err(error::Error::Database)?;

                        tracing::info!(
                            "Create key generator client - address: {:?} / ip_address: {:?}.",
                            key_generator.address(),
                            key_generator.ip_address(),
                        );

                        let key_generator_client: KeyGeneratorClient =
                            KeyGeneratorClient::new(key_generator.ip_address())
                                .map_err(error::Error::RpcError)?;
                        Ok((key_generator.address().clone(), key_generator_client))
                    },
                )
                .collect::<Result<HashMap<Address, KeyGeneratorClient>, Error>>()?;

            // Initialize an application-wide state instance
            let app_state = AppState::new(config, key_generator_clients);

            // Initialize the internal RPC server
            initialize_internal_rpc_server(&app_state).await?;

            // Initialize the cluster RPC server
            initialize_cluster_rpc_server(&app_state).await?;

            // Initialize the external RPC server.
            let server_handle = initialize_external_rpc_server(&app_state).await?;

            // Run the single key generator task
            // let node_address = app_state.config().address();
            // run_single_key_generator(Arc::new(app_state), node_address);

            server_handle.await.unwrap();
        }
    }

    Ok(())
}

async fn initialize_internal_rpc_server(app_state: &AppState) -> Result<(), Error> {
    let internal_rpc_url = app_state.config().internal_rpc_url().to_string();

    // Initialize the internal RPC server.
    let internal_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method(
            internal::AddKeyGenerator::METHOD_NAME,
            internal::AddKeyGenerator::handler,
        )?
        .register_rpc_method(
            internal::debug::RunSingleKeyGenerator::METHOD_NAME,
            internal::debug::RunSingleKeyGenerator::handler,
        )?
        .init(app_state.config().internal_rpc_url().to_string())
        .await
        .map_err(error::Error::RpcError)?;

    tracing::info!(
        "Successfully started the internal RPC server: {}",
        internal_rpc_url
    );

    tokio::spawn(async move {
        internal_rpc_server.stopped().await;
    });

    Ok(())
}

async fn initialize_cluster_rpc_server(app_state: &AppState) -> Result<(), Error> {
    let cluster_rpc_url = app_state.config().cluster_rpc_url().to_string();

    let key_generator_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method(
            cluster::GetKeyGeneratorList::METHOD_NAME,
            cluster::GetKeyGeneratorList::handler,
        )?
        .register_rpc_method(
            cluster::SyncKeyGenerator::METHOD_NAME,
            cluster::SyncKeyGenerator::handler,
        )?
        .register_rpc_method(
            cluster::SyncPartialKey::METHOD_NAME,
            cluster::SyncPartialKey::handler,
        )?
        .init(cluster_rpc_url.clone())
        .await
        .map_err(error::Error::RpcError)?;

    tracing::info!(
        "Successfully started the cluster RPC server: {}",
        cluster_rpc_url
    );

    tokio::spawn(async move {
        key_generator_rpc_server.stopped().await;
    });

    Ok(())
}

async fn initialize_external_rpc_server(app_state: &AppState) -> Result<JoinHandle<()>, Error> {
    let external_rpc_url = app_state.config().external_rpc_url().to_string();

    // Initialize the external RPC server.
    let external_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method(
            external::GetEncryptionKey::METHOD_NAME,
            external::GetEncryptionKey::handler,
        )?
        .register_rpc_method(
            external::GetDecryptionKey::METHOD_NAME,
            external::GetDecryptionKey::handler,
        )?
        .init(external_rpc_url.clone())
        .await
        .map_err(error::Error::RpcError)?;

    tracing::info!(
        "Successfully started the external RPC server: {}",
        external_rpc_url
    );

    let server_handle = tokio::spawn(async move {
        external_rpc_server.stopped().await;
    });

    Ok(server_handle)
}
