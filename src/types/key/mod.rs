mod model;

use std::collections::btree_map::{BTreeMap, Iter};

pub use model::*;
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey;

use super::Address;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PartialKeyList(BTreeMap<Address, PartialKey>);

impl PartialKeyList {
    pub fn insert(&mut self, address: Address, partial_key: PartialKey) {
        self.0.insert(address, partial_key);
    }

    pub fn remove(&mut self, address: Address) {
        self.0.remove(&address);
    }

    pub fn iter(&self) -> Iter<'_, Address, PartialKey> {
        self.0.iter()
    }

    pub fn to_vec(&self) -> Vec<PartialKey> {
        self.0.iter().map(|(_key, value)| value.clone()).collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, address: &Address) -> Option<&PartialKey> {
        self.0.get(address)
    }

    pub fn get_with_index(&self, index: usize) -> Option<(&Address, &PartialKey)> {
        self.0.iter().nth(index)
    }

    pub fn get_address_list(&self) -> Vec<Address> {
        self.0.keys().cloned().collect()
    }
}
