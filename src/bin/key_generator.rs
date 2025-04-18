use std::str::FromStr;

use clap::{Parser, Subcommand};
use distributed_key_generation::{
    error::{self, Error},
    rpc::{
        cluster::{self, GetKeyGeneratorList, GetKeyGeneratorRpcUrlListResponse},
        external, internal,
    },
    state::AppState,
    task::single_key_generator::run_single_key_generator,
    types::*,
};
use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcParameter, RpcServer},
    },
    kvstore::KvStoreBuilder,
};
pub use serde::{Deserialize, Serialize};
use skde::{delay_encryption::SkdeParams, BigUint};
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

            // TODO: remove this values
            const MOD_N: &str = "26737688233630987849749538623559587294088037102809480632570023773459222152686633609232230584184543857897813615355225270819491245893096628373370101798393754657209853664433779631579690734503677773804892912774381357280025811519740953667880409246987453978226997595139808445552217486225687511164958368488319372068289768937729234964502681229612929764203977349037219047813560623373035187038018937232123821089208711930458219009895581132844064176371047461419609098259825422421077554570457718558971463292559934623518074946858187287041522976374186587813034651849410990884606427758413847140243755163116582922090226726575253150079";
            const GENERATOR: &str = "4";
            const TIME_PARAM_T: u32 = 2;
            const MAX_KEY_GENERATOR_NUMBER: u32 = 2;

            let n = BigUint::from_str(MOD_N).expect("Invalid MOD_N");
            let g = BigUint::from_str(GENERATOR).expect("Invalid GENERATOR");
            let max_key_generator_number = BigUint::from(MAX_KEY_GENERATOR_NUMBER);
            let t = 2_u32.pow(TIME_PARAM_T);
            let mut h = g.clone();
            (0..t).for_each(|_| {
                h = (&h * &h) % n.clone();
            });

            let skde_params = SkdeParams {
                t,
                n: n.to_str_radix(10),
                g: g.to_str_radix(10),
                h: h.to_str_radix(10),
                max_sequencer_number: max_key_generator_number.to_str_radix(10),
            };

            // Initialize the database
            KvStoreBuilder::default()
                .set_default_lock_timeout(5000)
                .set_txn_lock_timeout(5000)
                .build(config.database_path())
                .map_err(error::Error::Database)?
                .init();

            KeyGeneratorList::initialize().map_err(error::Error::Database)?;
            SessionId::initialize().map_err(error::Error::Database)?;

            tracing::info!(
                "Successfully initialized the database at {:?}.",
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

            // Log node role
            tracing::info!("Node started with role: {}", config.role());

            // Based on the role, start appropriate services
            if config.is_leader() {
                tracing::info!("Starting leader node operations...");
                run_single_key_generator(app_state.clone());
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
    let internal_rpc_url = app_state.config().internal_rpc_url().to_string();

    // Initialize the internal RPC server.
    let internal_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<internal::AddKeyGenerator>()?
        .init(app_state.config().internal_rpc_url().to_string())
        .await
        .map_err(error::Error::RpcServerError)?;

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
    let cluster_rpc_url = anywhere(&app_state.config().cluster_port()?);

    let key_generator_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<cluster::GetKeyGeneratorList>()?
        .register_rpc_method::<cluster::SyncKeyGenerator>()?
        .register_rpc_method::<cluster::SyncAggregatedKey>()?
        .register_rpc_method::<cluster::SyncPartialKey>()?
        .register_rpc_method::<cluster::SubmitPartialKey>()?
        .register_rpc_method::<cluster::SubmitPartialKeyAck>()?
        .register_rpc_method::<cluster::RunGeneratePartialKey>()?
        .init(cluster_rpc_url.clone())
        .await
        .map_err(error::Error::RpcServerError)?;

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
    let external_rpc_url = anywhere(&app_state.config().external_port()?);

    // Initialize the external RPC server.
    let external_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<external::GetEncryptionKey>()?
        .register_rpc_method::<external::GetDecryptionKey>()?
        .register_rpc_method::<external::GetLatestEncryptionKey>()?
        .register_rpc_method::<external::GetLatestKeyId>()?
        .register_rpc_method::<external::GetSkdeParams>()?
        .init(external_rpc_url.clone())
        .await
        .map_err(error::Error::RpcServerError)?;

    tracing::info!(
        "Successfully started the external RPC server: {}",
        external_rpc_url
    );

    let server_handle = tokio::spawn(async move {
        external_rpc_server.stopped().await;
    });

    Ok(server_handle)
}

pub fn anywhere(port: &str) -> String {
    format!("0.0.0.0:{}", port)
}
