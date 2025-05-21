use std::{
    hash::{Hash, Hasher},
    fmt::{Debug, Display},
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

impl<Address: Debug> Display for KeyGenerator<Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "address: {:?}, cluster_rpc_url: {:?}, external_rpc_url: {:?}", self.address, self.cluster_rpc_url, self.external_rpc_url)
    }
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

    pub fn contains(&self, address: &Address) -> bool {
        self.0.iter().any(|kg| kg.address == *address)
    }

    /// Returns all RPC URLs of the key generators.
    /// If `is_sync` is true, it returns the RPC URLs of the key generators in the cluster.
    /// Otherwise, it returns the external RPC URLs of all key generators.
    pub fn all_rpc_urls(&self, is_sync: bool) -> Vec<String> {
        self.0
            .iter()
            .map(|key_generator| {
                if is_sync {
                    key_generator.cluster_rpc_url().to_owned()
                } else {
                    key_generator.external_rpc_url().to_owned()
                }
            })
            .collect()
    }
}

impl<Address> Iterator for KeyGeneratorList<Address> {
    type Item = KeyGenerator<Address>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}
