use std::{collections::HashMap, sync::Arc};

use skde::{
    delay_encryption::SecretKey, key_aggregation::AggregatedKey, key_generation::PartialKey,
};
use tokio::sync::Mutex;

use crate::{
    cli::Config,
    client::key_generator::KeyGeneratorClient,
    error::{self, Error},
    types::{Address, KeyGenerator},
};

pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: Config,

    key_generator_clients: Mutex<HashMap<Address, KeyGeneratorClient>>,

    partial_keys: Mutex<HashMap<u64, HashMap<Address, PartialKey>>>,
    aggregated_keys: Mutex<HashMap<u64, AggregatedKey>>,
    decryption_keys: Mutex<HashMap<u64, SecretKey>>,
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
        key_generator_clients: HashMap<Address, KeyGeneratorClient>,
    ) -> Self {
        let inner = AppStateInner {
            config,
            key_generator_clients: Mutex::new(key_generator_clients),
            partial_keys: Mutex::new(HashMap::new()),
            aggregated_keys: Mutex::new(HashMap::new()),
            decryption_keys: Mutex::new(HashMap::new()),
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
        key_generator.address();

        Ok(())
    }

    pub async fn key_generator_clients(
        &self,
    ) -> Result<HashMap<Address, KeyGeneratorClient>, Error> {
        let key_generator_clients = self.inner.key_generator_clients.lock().await;

        Ok(key_generator_clients.clone())
    }

    pub async fn add_partial_key(
        &self,
        key_id: u64,
        address: Address,
        partial_key: PartialKey,
    ) -> Result<(), Error> {
        let mut partial_keys_lock = self.inner.partial_keys.lock().await;

        match partial_keys_lock.get_mut(&key_id) {
            Some(partial_keys) => {
                partial_keys.insert(address, partial_key);
                return Ok(());
            }
            None => {
                let mut partial_keys = HashMap::new();
                partial_keys.insert(address, partial_key);

                partial_keys_lock.insert(key_id, partial_keys);
            }
        }

        Ok(())
    }

    pub async fn get_partial_key_list(&self, key_id: u64) -> Result<Vec<PartialKey>, Error> {
        let partial_keys: tokio::sync::MutexGuard<'_, HashMap<u64, HashMap<Address, PartialKey>>> =
            self.inner.partial_keys.lock().await;

        let partial_keys = partial_keys.get(&key_id).ok_or(error::Error::NotFound)?;

        Ok(partial_keys.values().cloned().collect())
    }

    pub async fn add_aggregated_key(
        &self,
        key_id: u64,
        aggregated_key: AggregatedKey,
    ) -> Result<(), Error> {
        let mut aggregated_keys = self.inner.aggregated_keys.lock().await;

        aggregated_keys.insert(key_id, aggregated_key);

        Ok(())
    }

    pub async fn get_encryption_key(&self, key_id: u64) -> Result<AggregatedKey, Error> {
        let aggregated_keys = self.inner.aggregated_keys.lock().await;

        aggregated_keys
            .get(&key_id)
            .cloned()
            .ok_or(error::Error::NotFound)
    }

    pub async fn add_decryption_key(
        &self,
        key_id: u64,
        decryption_key: SecretKey,
    ) -> Result<(), Error> {
        let mut decryption_keys = self.inner.decryption_keys.lock().await;

        decryption_keys.insert(key_id, decryption_key);

        Ok(())
    }

    pub async fn get_decryption_key(&self, key_id: u64) -> Result<SecretKey, Error> {
        let decryption_keys = self.inner.decryption_keys.lock().await;

        decryption_keys
            .get(&key_id)
            .cloned()
            .ok_or(error::Error::NotFound)
    }
}
