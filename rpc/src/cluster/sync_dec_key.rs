use crate::{*, DecKeyPayload};
use dkg_primitives::{Config, DecKey, EncKey, RuntimeEvent, SignedCommitment, KeyService};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Handler for syncing the newly generated decryption key and store it in the local kvstore
pub struct SyncDecKey<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature, Address> SyncDecKey<Signature, Address> {
    fn dec_key(&self) -> Result<DecKeyPayload, RpcError> {
        self.0.commitment.payload.decode::<DecKeyPayload>().map_err(|e| RpcError::from(e))
    } 
}

impl<C: Config> RpcParameter<C> for SyncDecKey<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_dec_key"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        if let Some(sender) = self.0.sender() {
            info!("Received decryption key from {:?}", sender);
            let _ = ctx.verify_signature(&self.0.signature, &self.0.commitment, Some(sender))?;
            let session_id = self.0.session_id();
            let payload = self.dec_key()?;
            let enc_key = EncKey::get(session_id)?;
            let dec_key = DecKey::new(payload.dec_key);
            ctx.key_service().verify_dec_key(&enc_key.inner(), &dec_key.inner()).map_err(|e| RpcError::from(e))?;
            dec_key.put(session_id)?;
            ctx.async_task().emit_event(RuntimeEvent::EndSession(session_id)).await.map_err(|e| RpcError::from(e))?;
        }

        Ok(())
    }
}
