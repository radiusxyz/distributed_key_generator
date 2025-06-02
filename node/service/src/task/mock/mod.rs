use std::{fmt::Debug, marker::PhantomData};
use dkg_node_primitives::Role;
use dkg_primitives::{AddressT, Parameter, AuthError, AuthService};
use radius_sdk::{kvstore::Model, validation::symbiotic::types::map::HashMap};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use async_trait::async_trait;

mod rpc;

#[derive(Debug, Clone)]
pub struct MockAuthService<Address>(PhantomData<Address>);

impl<Address> MockAuthService<Address> {
    pub fn new() -> Self { Self(Default::default()) }
}

#[async_trait]
impl<Address: AddressT + Parameter> AuthService<Address> for MockAuthService<Address> {
    type Error = AuthError;

    async fn update_trusted_setup(&self, _bytes: Vec<u8>, _signature: Vec<u8>) -> Result<(), Self::Error> {
        Ok(())
    }
    async fn get_trusted_setup(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(vec![])
    }

    async fn get_authority_info(&self) -> Result<(Address, String, String), Self::Error> {
        unimplemented!("Not implemented");
    }

    async fn get_solver_info(&self) -> Result<(Address, String, String), Self::Error> {
        unimplemented!("Not implemented");
    }

    async fn is_active(&self, current_round: u64, address: Address) -> Result<bool, Self::Error> {
        Ok(self.current_auth_registry(current_round).await?.contains(&address))
    }

    async fn current_auth_registry(&self, current_round: u64) -> Result<Vec<Address>, Self::Error> {
        let active_set = MockRegistry::<Address>::get(current_round).map_err(|_| AuthError::GetStateError)?;
        Ok(active_set.get_all_addresses())
    }

    async fn next_auth_registry(&self, next_round: u64) -> Result<Vec<Address>, Self::Error> {
        let active_set = MockRegistry::<Address>::get(next_round).map_err(|_| AuthError::GetStateError)?;
        Ok(active_set.get_all_addresses())
    }

    async fn is_ready(&self, current_round: u64, threshold: u16) -> Result<bool, Self::Error> {
        Ok(MockRegistry::<Address>::get(current_round).map_err(|_| AuthError::GetStateError).map(|active_set| active_set.is_ready(threshold)).unwrap_or(false))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[kvstore(key(round: u64))]
/// Active set of the given round which act as blockchain
pub struct MockRegistry<Address>(HashMap<Role, Vec<Address>>);

impl<Address: Parameter + AddressT> MockRegistry<Address> {

    fn new() -> Self { Self(Default::default()) }

    /// Initialize the registry for the first two rounds
    fn _initialize(committees: Vec<Address>,  solvers: Vec<Address>) {
        let mut registry = Self::new();
        for committee in committees {
            registry.register(Role::Committee, committee);
        }
        for solver in solvers {
            registry.register(Role::Solver, solver);
        }
        registry.put(0).unwrap();
        registry.put(1).unwrap();
    }

    fn get_all_addresses(&self) -> Vec<Address> {
        self.0.keys().into_iter().filter_map(|role| { self.0.get(role) }).flatten().cloned().collect()
    }

    fn is_ready(&self, threshold: u16) -> bool {
        let mut is_ready = true;
        for role in Role::iter_roles() {
            match role {
                Role::Committee => is_ready &= self.0.get(&role).map_or(false, |addresses| addresses.len() >= threshold as usize),
                _ => is_ready &= self.0.get(&role).map_or(false, |addresses| addresses.len() >= 1),
            }
        }
        is_ready
    }

    fn contains(&self, role: &Role, address: &Address) -> bool {
        if let Some(addresses) = self.0.get(role) {
            addresses.iter().any(|a| a == address)
        } else {
            false
        }
    }

    fn register(&mut self, role: Role, address: Address) {
        self.0.entry(role).or_insert(vec![]).push(address);
    }

    fn unregister(&mut self, role: Role, address: Address) {
        if let Some(addresses) = self.0.get_mut(&role) {
            addresses.retain(|a| a != &address);
        }
    }
}