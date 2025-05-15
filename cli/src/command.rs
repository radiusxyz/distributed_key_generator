
use crate::{Cli, Commands};

pub fn run() -> Result<(), Error> {
    let mut cli = Cli::init();

    match cli.command {
        Commands::Init { ref config_path } => ConfigPath::init(config_path)?,
        // Only for authority node: generate and write SKDE params
        Commands::SetupSkdeParams { ref config_path } => {
            ConfigPath::init(config_path)?; // ensure dir exists
            run_setup_skde_params(config_path);
        },
        Commands::Start {
            ref mut config_option,
        } => {
            run_node_inner()
        }
    }

    Ok(())
}

fn run_node_inner() -> Result<(), Error> {
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
