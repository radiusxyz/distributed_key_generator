async fn initialize_internal_rpc_server(app_state: &AppState) -> Result<(), Error> {
    let prefix = log_prefix(app_state.config());
    let internal_rpc_url = app_state.config().internal_rpc_url().to_string();

    // Initialize the internal RPC server.
    let internal_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<internal::AddKeyGenerator>()?
        .init(app_state.config().internal_rpc_url().to_string())
        .await
        .map_err(error::Error::RpcServerError)?;

    tracing::info!(
        "{} Successfully started the internal RPC server: {}",
        prefix, internal_rpc_url
    );

    tokio::spawn(async move {
        internal_rpc_server.stopped().await;
    });

    Ok(())
}

async fn initialize_cluster_rpc_server(app_state: &AppState) -> Result<(), Error> {
    let prefix = log_prefix(app_state.config());
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
    let prefix = log_prefix(app_state.config());
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
    let prefix = log_prefix(app_state.config());
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
    let prefix = log_prefix(app_state.config());
    let solver_rpc_url = app_state.config().solver_rpc_url().clone().unwrap();

    let rpc_server_builder = RpcServer::new(app_state.clone());

    let rpc_server = if app_state.config().is_leader() {
        rpc_server_builder
            .register_rpc_method::<common::GetSkdeParams>()?
            .register_rpc_method::<solver::SubmitDecryptionKey>()?
    } else {
        rpc_server_builder.register_rpc_method::<solver::SolverSyncFinalizedPartialKeys>()?
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
