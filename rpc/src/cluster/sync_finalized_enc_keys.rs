use crate::{*, FinalizedEncKeyPayload };
use dkg_primitives::{AsyncTask, Config, EncKey, RuntimeEvent, KeyService, Payload, SessionId, SignedCommitment};
use radius_sdk::json_rpc::server::RpcError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Handler for syncing the finalized encryption keys.
/// Create decryption key if solver, otherwise derive encryption key from the finalized encryption keys
pub struct SyncFinalizedEncKeys<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature, Address: Clone> SyncFinalizedEncKeys<Signature, Address> {
    fn get_session_id(&self) -> SessionId { self.0.session_id() }

    fn payload(&self) -> Payload { self.0.commitment.payload.clone() }
}

impl<C: Config> RpcParameter<C> for SyncFinalizedEncKeys<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_finalized_enc_keys"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        let session_id = self.get_session_id();
        info!("{:?} at session {:?}", <Self as RpcParameter<C>>::method(), session_id);
        let mut enc_keys = self.payload()
            .decode::<FinalizedEncKeyPayload<C::Signature, C::Address>>()
            .map_err(|e| RpcError::from(e))?
            .inner()
            .iter()
            .map(|key| {
                Ok(key.inner().commitment.payload.inner())
            })
            .collect::<Result<Vec<Vec<u8>>, RpcError>>()?;
        enc_keys.sort();
        let enc_key = ctx.key_service().gen_enc_key(ctx.randomness(session_id), Some(enc_keys))?;
        EncKey::new(enc_key.clone()).put(session_id)?;
        ctx.async_task().emit_event(RuntimeEvent::SolveKey { enc_key, session_id }).await.map_err(|e| RpcError::from(e))?;
        Ok(())
    }
}
