use crate::MockRegistry;
use dkg_rpc::AppState;
use radius_sdk::json_rpc::server::{RpcParameter, RpcError};
use serde::{Deserialize, Serialize};
use dkg_node_primitives::Role;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Register<Address> {
    address: Address,
    role: Role,
}

impl<C: AppState> RpcParameter<C> for Register<C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "register"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let current_round = ctx.current_round()?;
        let mut registry = MockRegistry::<C::Address>::new();
        if !registry.contains(&self.role, &self.address) {
            registry.register(self.role, self.address);
            registry.put(current_round + 2)?
        }
        Ok(())
    }
}