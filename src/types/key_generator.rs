use std::collections::btree_set::{BTreeSet, Iter};

use super::Address;
use crate::types::prelude::*;

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
pub struct KeyGeneratorAddressList(BTreeSet<String>);

impl KeyGeneratorAddressList {
    pub fn insert(&mut self, cluster_id: impl AsRef<str>) {
        self.0.insert(cluster_id.as_ref().into());
    }

    pub fn remove(&mut self, cluster_id: impl AsRef<str>) {
        self.0.remove(cluster_id.as_ref());
    }

    pub fn iter(&self) -> Iter<'_, String> {
        self.0.iter()
    }
}
