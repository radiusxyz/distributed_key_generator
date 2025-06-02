use crate::MockRegistry;
use dkg_rpc::AppState;
use radius_sdk::json_rpc::server::{RpcParameter, RpcError};
use serde::{Deserialize, Serialize};
use dkg_node_primitives::Role;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Unregister<Address> {
    address: Address,
    role: Role,
}

impl<C: AppState> RpcParameter<C> for Unregister<C::Address> {
    type Response = (); 

    fn method() -> &'static str {
        "unregister"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let mut registry = MockRegistry::<C::Address>::get_mut(ctx.current_round()? + 2)?;
        if registry.contains(&self.role, &self.address) {
            registry.unregister(self.role, self.address);
            registry.update()?;
        }
        Ok(())
    }
}