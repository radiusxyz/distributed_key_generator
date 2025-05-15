use std::{
    collections::{hash_set::Iter, HashSet},
    hash::{Hash, Hasher},
};
use radius_sdk::{kvstore::{Model, KvStoreError}, signature::Address};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyGenerator {
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

impl PartialEq for KeyGenerator {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

impl Eq for KeyGenerator {}

impl Hash for KeyGenerator {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.hash(state);
    }
}

impl KeyGenerator {
    pub fn new(address: Address, cluster_rpc_url: String, external_rpc_url: String) -> Self {
        Self {
            address,
            cluster_rpc_url,
            external_rpc_url,
        }
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn cluster_rpc_url(&self) -> &str {
        &self.cluster_rpc_url
    }

    pub fn external_rpc_url(&self) -> &str {
        &self.external_rpc_url
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key())]
pub struct KeyGeneratorList(HashSet<KeyGenerator>);

impl KeyGeneratorList {
    pub fn default() -> Self {
        Self(HashSet::new())
    }

    pub fn insert(&mut self, key_generator: KeyGenerator) {
        self.0.insert(key_generator);
    }

    pub fn remove(&mut self, key_generator: &KeyGenerator) {
        self.0.remove(key_generator);
    }

    pub fn iter(&self) -> Iter<'_, KeyGenerator> {
        self.0.iter()
    }

    pub fn contains(&self, key_generator: &KeyGenerator) -> bool {
        self.0.contains(key_generator)
    }

    pub fn is_key_generator_in_cluster(&self, address: &Address) -> bool {
        for key_generator in self.0.iter() {
            if key_generator.address() == address {
                return true;
            }
        }

        false
    }

    pub fn get_other_key_generator_rpc_url_list(&self, my_address: &Address) -> Vec<String> {
        self.0
            .iter()
            .filter_map(|key_generator| {
                if key_generator.address() == my_address {
                    None
                } else {
                    Some(key_generator.cluster_rpc_url().to_owned())
                }
            })
            .collect()
    }

    pub fn get_all_key_generator_rpc_url_list(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|key_generator| key_generator.cluster_rpc_url().to_owned())
            .collect()
    }

    pub fn initialize() -> Result<(), KvStoreError> {
        if Self::get().is_err() {
            let key_generator_list = Self::default();

            key_generator_list.put()?
        }

        Ok(())
    }
}
