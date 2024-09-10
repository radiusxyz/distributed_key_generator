mod model;
use std::collections::btree_set::{BTreeSet, Iter};

pub use model::*;

use crate::types::prelude::*;

pub type KeyGeneratorList = Vec<KeyGenerator>;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct KeyGeneratorAddressList(BTreeSet<Address>);

impl KeyGeneratorAddressList {
    pub fn insert(&mut self, key_generator_address: Address) {
        self.0.insert(key_generator_address);
    }

    pub fn remove(&mut self, key_generator_address: &Address) {
        self.0.remove(key_generator_address);
    }

    pub fn iter(&self) -> Iter<'_, Address> {
        self.0.iter()
    }

    pub fn contains(&self, key_generator_address: &Address) -> bool {
        self.0.contains(key_generator_address)
    }
}
