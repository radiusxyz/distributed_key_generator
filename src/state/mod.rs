use std::{collections::HashMap, sync::Arc};

use skde::key_generation::PartialKey;
use tokio::sync::Mutex;

use crate::{
    cli::Config,
    client::key_generator::KeyGeneratorClient,
    types::{Address, KeyGenerator, SigningKey},
};

pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: Config,
    key_generator_clients: Mutex<HashMap<Address, KeyGeneratorClient>>,
    partial_keys: Mutex<HashMap<Address, PartialKey>>,
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
        };

        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub fn signing_key(&self) -> &SigningKey {
        self.inner.config.signing_key()
    }

    pub async fn add_key_generator_client(&self, key_generator: KeyGenerator) {
        // let mut key_generator_clients = self.inner.key_generator_clients.lock().await;
        // key_generator_clients.insert(address, key_generator_client);
        key_generator.address();
    }

    pub fn key_generator_clients(&self) -> &Mutex<HashMap<Address, KeyGeneratorClient>> {
        &self.inner.key_generator_clients
    }

    pub async fn add_partial_key(&self, address: Address, partial_key: PartialKey) {
        let mut partial_keys = self.inner.partial_keys.lock().await;
        partial_keys.insert(address, partial_key);
    }

    pub async fn get_encryption_key(&self) -> String {
        "hi".to_string()
    }
}
