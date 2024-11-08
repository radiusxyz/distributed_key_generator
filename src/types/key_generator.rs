use std::collections::{hash_set::Iter, HashSet};

use radius_sdk::{kvstore::Model, signature::Address};

use crate::types::prelude::*;

#[derive(Clone, Hash, Eq, PartialEq, Debug, Deserialize, Serialize)]

pub struct KeyGenerator {
    address: Address,
    ip_address: String,
}

impl KeyGenerator {
    pub fn new(address: Address, ip_address: String) -> Self {
        Self {
            address,
            ip_address,
        }
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn ip_address(&self) -> &str {
        &self.ip_address
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
        self.0.remove(&key_generator);
    }

    pub fn iter(&self) -> Iter<'_, KeyGenerator> {
        self.0.iter()
    }

    pub fn contains(&self, key_generator: &KeyGenerator) -> bool {
        self.0.contains(&key_generator)
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
                    Some(key_generator.ip_address().to_owned())
                }
            })
            .collect()
    }

    pub fn get_all_key_generator_rpc_url_list(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|key_generator| key_generator.ip_address().to_owned())
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
