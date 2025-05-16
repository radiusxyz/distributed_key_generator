use dkg_node_primitives::Config;
use dkg_node_service::{
    authority_setup::run_setup_skde_params, single_key_generator::run_single_key_generator,
};
use dkg_primitives::Error;

use crate::{config::ConfigOption, Cli, Commands, ConfigPath};

pub fn run() -> Result<(), Error> {
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
        } => run_node_inner(),
    }

    Ok(())
}

fn run_node_inner() -> Result<(), Error> {
    // Load the configuration from the path
    let config = load_config(config_option);
    let prefix = log_prefix(&config);

    info!(
        "{} Successfully loaded the configuration file at {:?}.",
        prefix,
        config.path(),
    );

    // let skde_params = fetch_skde_params_with_retry(&config).await;

    // if config.is_authority() {
    //     let app_state = AppState::new(config.clone(), skde_params);
    //     let prefix = log_prefix(app_state.config());

    //     info!("{} Serving get_authorized_skde_params", prefix);
    //     let handle = initialize_authority_rpc_server(&app_state).await?;
    //     handle.await.unwrap();

    //     return Ok(());
    // }

    // // Initialize the database
    // KvStoreBuilder::default()
    //     .set_default_lock_timeout(5000)
    //     .set_txn_lock_timeout(5000)
    //     .build(config.database_path())
    //     .map_err(error::Error::Database)?
    //     .init();

    // KeyGeneratorList::initialize().map_err(error::Error::Database)?;
    // SessionId::initialize().map_err(error::Error::Database)?;

    // info!(
    //     "{} Successfully initialized the database at {:?}.",
    //     prefix,
    //     config.database_path(),
    // );

    // // If not a leader, get the key generator list from leader
    // if let Some(leader_rpc_url) = config.leader_cluster_rpc_url() {
    //     // Non-leader node
    //     let rpc_client = RpcClient::new()?;

    //     let response: GetKeyGeneratorRpcUrlListResponse = rpc_client
    //         .request(
    //             leader_rpc_url,
    //             GetKeyGeneratorList::method(),
    //             &GetKeyGeneratorList,
    //             Id::Null,
    //         )
    //         .await?;

    //     let key_generator_list: KeyGeneratorList = response.key_generator_rpc_url_list.into();

    //     key_generator_list.put()?;
    // }

    // // Initialize an application-wide state instance
    // let app_state = AppState::new(config.clone(), skde_params);
    // let prefix = log_prefix(app_state.config());

    // // Based on the role, start appropriate services
    // if config.is_leader() {
    //     info!("{} Starting leader node operations...", prefix);
    //     run_single_key_generator(app_state.clone());

    //     info!("{} Initializing solve RPC server on leader...", prefix);
    //     initialize_solve_rpc_server(&app_state).await?;
    // } else if config.is_solver() {
    //     info!("{} Initializing solve RPC server on solver...", prefix);
    //     initialize_solve_rpc_server(&app_state).await?;
    // }
    // // Initialize the internal RPC server
    // initialize_internal_rpc_server(&app_state).await?;

    // // Initialize the cluster RPC server
    // initialize_cluster_rpc_server(&app_state).await?;

    // // Initialize the external RPC server
    // let server_handle = initialize_external_rpc_server(&app_state).await?;

    // server_handle.await.unwrap();
    Ok(())
}

fn load_config(config_option: &mut ConfigOption) -> Config {
    let config_path = match config_option.path.as_mut() {
        Some(config_path) => config_path.clone(),
        None => {
            let config_path: PathBuf = ConfigPath::default().as_ref().into();
            config_option.path = Some(config_path.clone());
            config_path
        }
    };

    // Read config file
    let config_file_path = config_path.join(CONFIG_FILE_NAME);

    // Try to read config file, if it doesn't exist or can't be read, use default values
    let config_file: ConfigOption = if config_file_path.exists() {
        match fs::read_to_string(&config_file_path) {
            Ok(config_string) => match toml::from_str(&config_string) {
                Ok(parsed) => parsed,
                Err(e) => {
                    warn!("Failed to parse config file: {}, using default values", e);
                    ConfigOption::default()
                }
            },
            Err(e) => {
                warn!("Failed to read config file: {}, using default values", e);
                ConfigOption::default()
            }
        }
    } else {
        warn!(
            "Config file not found at {:?}, using default values",
            config_file_path
        );
        ConfigOption::default()
    };

    // Merge configs from CLI input
    let merged_config_option = config_file.merge(config_option);
    info!("chain_type: {:?}", merged_config_option);

    let chain_type = merged_config_option.chain_type.unwrap().try_into().unwrap();

    // Read signing key
    let signing_key_path = config_path.join(SIGNING_KEY);

    let signer = if signing_key_path.exists() {
        match fs::read_to_string(&signing_key_path) {
            Ok(key_string) => {
                let clean_key = key_string.trim().replace("\n", "").replace("\r", "");
                match PrivateKeySigner::from_str(chain_type, &clean_key) {
                    Ok(signer) => signer,
                    Err(err) => {
                        warn!("Invalid signing key in file: {}, using default key", err);
                        warn!("Key string was: '{}'", clean_key);
                        let default_key =
                            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
                        PrivateKeySigner::from_str(chain_type, default_key).unwrap()
                    }
                }
            }
            Err(err) => {
                warn!(
                    "Failed to read signing key file: {}, using default key",
                    err
                );
                let default_key =
                    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
                PrivateKeySigner::from_str(chain_type, default_key).unwrap()
            }
        }
    } else {
        warn!(
            "Signing key file not found at {:?}, using default key",
            signing_key_path
        );
        // Create directory if it doesn't exist
        if let Some(parent) = signing_key_path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        // Write default key to file
        let default_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let _ = fs::write(&signing_key_path, default_key);
        PrivateKeySigner::from_str(chain_type, default_key).unwrap()
    };

    // Parse role if provided
    let role = if let Some(role_str) = &merged_config_option.role {
        match role_str.parse::<Role>() {
            Ok(role) => role,
            Err(e) => {
                warn!("Invalid role: {}, ignoring role setting", e);
                Role::Committee
            }
        }
    } else {
        Role::Committee
    };

    Config::new(
        config_path,
        merged_config_option.external_rpc_url.unwrap(),
        merged_config_option.internal_rpc_url.unwrap(),
        merged_config_option.cluster_rpc_url.unwrap(),
        merged_config_option.solver_rpc_url.clone(),
        merged_config_option.leader_cluster_rpc_url.clone(),
        merged_config_option.leader_solver_rpc_url.clone(),
        merged_config_option.solver_solver_rpc_url.clone(),
        merged_config_option.authority_rpc_url.unwrap(),
        role,
        signer,
        Address::from_str(
            chain_type,
            &merged_config_option.radius_foundation_address.unwrap(),
        )
        .unwrap(),
        chain_type,
        merged_config_option.session_cycle.unwrap(),
    )
}
