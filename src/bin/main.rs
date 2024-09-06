use std::{collections::HashMap, sync::Arc};

use key_management_system::{
    cli::{Cli, Commands, Config, ConfigPath},
    client::key_generator::KeyGeneratorClient,
    error::{self, Error},
    models::{KeyGeneratorAddressListModel, KeyGeneratorModel},
    rpc::{cluster, internal},
    state::AppState,
    task::single_key_generator::run_single_key_generator,
    types::Address,
};
use radius_sequencer_sdk::{json_rpc::RpcServer, kvstore::KvStore as Database};
use tokio::task::JoinHandle;

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

            let key_generator_address_list =
                KeyGeneratorAddressListModel::get_or_default().map_err(error::Error::Database)?;

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

            run_single_key_generator(Arc::new(app_state), Address::new("123".to_string()));

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
