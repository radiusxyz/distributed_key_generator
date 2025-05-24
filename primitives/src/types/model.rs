use std::{fmt::{Debug, Display}, hash::{Hash, Hasher}};
use radius_sdk::kvstore::{KvStoreError, Model};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use crate::{SignedCommitment, Parameter, AddressT, Error};

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: &SessionId, address: &Address))]
/// Kvstore for commitment to a encryption keys mapped by session id and address
pub struct EncKeyCommitment<Signature, Address>(SignedCommitment<Signature, Address>);

impl<Signature: Clone, Address: Clone> EncKeyCommitment<Signature, Address> {
    pub fn new(commitment: SignedCommitment<Signature, Address>) -> Self {
        Self(commitment)
    }

    pub fn inner(&self) -> SignedCommitment<Signature, Address> {
        self.0.clone()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
/// List of encryption key SubmitterList mapped by session id
pub struct SubmitterList<Address>(pub Vec<Address>);

impl<Address: Parameter + AddressT> SubmitterList<Address> {

    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn insert(&mut self, address: Address) {
        self.0.push(address);
    }

    pub fn remove(&mut self, address: Address) {
        self.0.retain(|a| a != &address);
    }

    pub fn to_vec(&self) -> Vec<Address> {
        self.0.iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn initialize(session_id: SessionId) -> Result<(), KvStoreError> {
        Self(Vec::new()).put(session_id)
    }
}

impl<Address> IntoIterator for SubmitterList<Address> {
    type Item = Address;
    type IntoIter = std::vec::IntoIter<Address>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
/// Decryption key store for a given session
pub struct DecKey(Vec<u8>);

impl DecKey {
    pub fn new(key: Vec<u8>) -> Self {
        Self(key)
    }
    
    pub fn inner(&self) -> Vec<u8> {
        self.0.clone()
    }
}

impl From<DecKey> for Vec<u8> {
    fn from(value: DecKey) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
/// Encryption key store for a given session
pub struct EncKey(Vec<u8>);

impl EncKey {
    pub fn new(enc_key: Vec<u8>) -> Self {
        Self(enc_key)
    }

    pub fn inner(&self) -> Vec<u8> {
        self.0.clone()
    }
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize, Model)]
#[kvstore(key())]
pub struct SessionId(u64);

impl From<u64> for SessionId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl Into<u64> for SessionId {
    fn into(self) -> u64 {
        self.0
    }
}

impl SessionId {
    pub fn initialize() -> Result<(), KvStoreError> {
        Self(0).put()
    }

    pub fn is_initial(&self) -> bool {
        self.0 == 0
    }

    pub fn prev(self) -> Option<Self> {
        self.0.checked_sub(1).map(Self)
    }

    pub fn next(&self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }

    pub fn next_mut(&mut self) -> Result<(), Error> {
        self.0 = self.next().ok_or(Error::Arithmetic)?.into();
        Ok(())
    }

    pub fn prev_mut(&mut self) -> Result<(), Error> {
        self.0 = self.prev().ok_or(Error::Arithmetic)?.into();
        Ok(())
    }
}

