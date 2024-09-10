use std::{collections::BTreeMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    client::key_generator::KeyGeneratorClient,
    error::{self, Error},
    types::{Address, Config, KeyGenerator},
};

pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: Config,
    key_generator_clients: Mutex<BTreeMap<Address, KeyGeneratorClient>>,
    skde_params: skde::SkdeParams,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl AppState {
    pub fn new(
        config: Config,
        key_generator_clients: BTreeMap<Address, KeyGeneratorClient>,
        skde_params: skde::SkdeParams,
    ) -> Self {
        let inner = AppStateInner {
            config,
            key_generator_clients: Mutex::new(key_generator_clients),
            skde_params,
        };

        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub async fn add_key_generator_client(
        &self,
        key_generator: KeyGenerator,
    ) -> Result<(), error::Error> {
        let key_generator_client: KeyGeneratorClient =
            KeyGeneratorClient::new(key_generator.ip_address()).map_err(error::Error::RpcError)?;

        let mut key_generator_clients = self.inner.key_generator_clients.lock().await;
        key_generator_clients.insert(key_generator.address().to_owned(), key_generator_client);

        Ok(())
    }

    pub async fn key_generator_clients(
        &self,
    ) -> Result<BTreeMap<Address, KeyGeneratorClient>, Error> {
        let key_generator_clients = self.inner.key_generator_clients.lock().await;

        Ok(key_generator_clients.clone())
    }

    pub fn skde_params(&self) -> &skde::SkdeParams {
        &self.inner.skde_params
    }
}
