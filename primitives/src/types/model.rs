use std::{
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    ops::Add,
};

use radius_sdk::kvstore::Model;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{AddressT, Payload, RuntimeError, SignedCommitment};
#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: &SessionId, address: &Address))]
/// Kvstore for signed commitment to a encryption keys mapped by session id and address
pub struct EncKeyCommitment<Signature, Address>(SignedCommitment<Signature, Address>);

impl<Signature: Clone, Address: Clone> EncKeyCommitment<Signature, Address> {
    pub fn new(commitment: SignedCommitment<Signature, Address>) -> Self {
        Self(commitment)
    }

    pub fn inner(&self) -> SignedCommitment<Signature, Address> {
        self.0.clone()
    }

    pub fn payload(&self) -> Payload {
        self.0.commitment.payload.clone()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
/// List of encryption key SubmitterList mapped by session id
pub struct SubmitterList<Address>(pub Vec<Address>);

impl<Address: AddressT> SubmitterList<Address> {
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
pub struct Operator<Address> {
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

impl<Address> From<(Address, String, String)> for Operator<Address> {
    fn from((address, cluster_rpc_url, external_rpc_url): (Address, String, String)) -> Self {
        Self {
            address,
            cluster_rpc_url,
            external_rpc_url,
        }
    }
}

impl<Address: Debug> Display for Operator<Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "address: {:?}, cluster_rpc_url: {:?}, external_rpc_url: {:?}",
            self.address, self.cluster_rpc_url, self.external_rpc_url
        )
    }
}

impl<Address: PartialEq> PartialEq for Operator<Address> {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

impl<Address: Eq> Eq for Operator<Address> {}

impl<Address: Hash> Hash for Operator<Address> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.hash(state);
    }
}

impl<Address: Clone> Operator<Address> {
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
pub struct ActiveOperatorList<Address>(Vec<Operator<Address>>);

impl<Address: AddressT> ActiveOperatorList<Address> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Operator<Address>> {
        self.0.get(index)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn insert(&mut self, operator: Operator<Address>) {
        self.0.push(operator);
    }

    pub fn remove(&mut self, operator: &Operator<Address>) {
        self.0.retain(|o| o != operator);
    }

    pub fn contains(&self, address: &Address) -> bool {
        self.0.iter().any(|o| o.address == *address)
    }

    pub fn all_addresses(&self) -> Vec<Address> {
        self.0.iter().map(|o| o.address()).collect()
    }

    /// Returns all RPC URLs of the key generators.
    /// If `is_sync` is true, it returns the RPC URLs of the key generators in the cluster.
    /// Otherwise, it returns the external RPC URLs of all key generators.
    pub fn all_rpc_urls(&self, is_sync: bool, exclude_list: Vec<Address>) -> Vec<String> {
        self.0
            .iter()
            .filter(|o| !exclude_list.contains(&o.address()))
            .map(|operator| {
                if is_sync {
                    operator.cluster_rpc_url().to_owned()
                } else {
                    operator.external_rpc_url().to_owned()
                }
            })
            .collect()
    }
}

impl<Address: AddressT> From<Vec<Operator<Address>>> for ActiveOperatorList<Address> {
    fn from(value: Vec<Operator<Address>>) -> Self {
        Self(value)
    }
}

impl<Address> Iterator for ActiveOperatorList<Address> {
    type Item = Operator<Address>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key())]
pub struct NextOperatorList<Address>(Vec<Operator<Address>>);

impl<Address: AddressT> NextOperatorList<Address> {
    pub fn inner(&self) -> Vec<Operator<Address>> {
        self.0.clone()
    }
}

impl<Address: AddressT> From<Vec<Operator<Address>>> for NextOperatorList<Address> {
    fn from(value: Vec<Operator<Address>>) -> Self {
        Self(value)
    }
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize, Model,
)]
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

impl Add for SessionId {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl SessionId {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn is_initial(&self) -> bool {
        self.0 == 0
    }

    pub fn prev(self) -> Option<Self> {
        self.0.checked_sub(1).map(Self)
    }

    pub fn with_value(&mut self, value: u64) {
        self.0 = value;
    }

    pub fn next(&self, amount: u64) -> Option<Self> {
        self.0.checked_add(amount).map(Self)
    }

    pub fn next_mut(&mut self, amount: u64) -> Result<Self, RuntimeError> {
        self.0 = self.next(amount).ok_or(RuntimeError::Arithmetic)?.into();
        Ok(*self)
    }

    pub fn prev_mut(&mut self) -> Result<Self, RuntimeError> {
        self.0 = self.prev().ok_or(RuntimeError::Arithmetic)?.into();
        Ok(*self)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key())]
pub struct OperatorTask(Vec<u8>);

impl OperatorTask {
    pub fn new(task: Vec<u8>) -> Self {
        Self(task)
    }

    pub fn inner(&self) -> Vec<u8> {
        self.0.clone()
    }
}

/// Record for request timestamp of decryption key in unix
#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct DecKeyRequestRecord {
    at: u128,
    timeout: u128,
}

impl DecKeyRequestRecord {
    pub fn new(at: u128, period: u128) -> Self {
        Self {
            at,
            timeout: at + period,
        }
    }

    pub fn is_good(&self, now: u128) -> bool {
        now < self.timeout
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct Randomness(Vec<u8>);

impl Randomness {
    pub fn new(randomness: Vec<u8>) -> Self {
        Self(randomness)
    }
}

impl Into<Vec<u8>> for Randomness {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key())]
pub struct TrustedSetup(Vec<u8>);

impl TrustedSetup {
    pub fn new(trusted_setup: Vec<u8>) -> Self {
        Self(trusted_setup)
    }
}
