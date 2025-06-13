use crate::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{Config, EncKeyCommitment, SessionId, SignedCommitment, SubmitterList};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Handler for syncing the newly generated encryption key and store it in the local kvstore
pub struct SyncEncKey<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature: Clone, Address: Clone> SyncEncKey<Signature, Address> {
    fn session_id(&self) -> SessionId { self.0.session_id() }
    fn sender(&self) -> Option<Address> { self.0.sender() }
}

impl<C: Config> RpcParameter<C> for SyncEncKey<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_enc_key"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> { 
        if let Some(sender) = self.sender() {
            info!("Received encryption key");
            let session_id = self.session_id();
            if sender == ctx.address() { return Ok(()); }
            let _ = ctx.verify_signature(&self.0.signature, &self.0.commitment, Some(sender.clone()))?;
            SubmitterList::<C::Address>::initialize(session_id)?;
            SubmitterList::<C::Address>::apply(session_id, |list| { list.insert(sender.clone());})?;
            let enc_key_commitment = self.0.commitment.payload.decode::<EncKeyCommitment<C::Signature, C::Address>>()?;
            enc_key_commitment.put(&session_id, &sender)?;
        } 
        Ok(())
    }
}
