use std::{
    hash::{Hash, Hasher},
    fmt::Debug,
};
use crate::traits::AddressT;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use radius_sdk::kvstore::Model;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyGenerator<Address> {
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

impl<Address: PartialEq> PartialEq for KeyGenerator<Address> {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

impl<Address: Eq> Eq for KeyGenerator<Address> {}

impl<Address: Hash> Hash for KeyGenerator<Address> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.hash(state);
    }
}

impl<Address: Clone> KeyGenerator<Address> {
    pub fn new(address: Address, cluster_rpc_url: String, external_rpc_url: String) -> Self {
        Self {
            address,
            cluster_rpc_url,
            external_rpc_url,
        }
    }

    pub fn address(&self) -> Address {
        self.address.clone()
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
pub struct KeyGeneratorList<Address>(Vec<KeyGenerator<Address>>);

impl<Address: AddressT> KeyGeneratorList<Address> {
    pub fn default() -> Self {
        Self(Vec::new())
    }

    pub fn insert(&mut self, key_generator: KeyGenerator<Address>) {
        self.0.push(key_generator);
    }

    pub fn remove(&mut self, key_generator: &KeyGenerator<Address>) {
        self.0.retain(|kg| kg != key_generator);
    }

    pub fn contains(&self, key_generator: &KeyGenerator<Address>) -> bool {
        self.0.contains(key_generator)
    }

    pub fn is_key_generator_in_cluster(&self, address: &Address) -> bool {
        for key_generator in self.0.iter() {
            if key_generator.address() == *address {
                return true;
            }
        }

        false
    }

    pub fn get_other_key_generator_rpc_url_list(&self, my_address: &Address) -> Vec<String> {
        self.0
            .iter()
            .filter_map(|key_generator| {
                if key_generator.address() == *my_address {
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
}

impl<Address> Iterator for KeyGeneratorList<Address> {
    type Item = KeyGenerator<Address>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}
