use dkg_primitives::{
    AsyncTask, Config, EncKey, KeyGenerator, Payload, SessionEvent, SessionId, SignedCommitment,
};
use radius_sdk::json_rpc::server::RpcError;
use serde::{Deserialize, Serialize};

use crate::{FinalizedEncKeyPayload, *};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Handler for syncing the finalized encryption keys.
/// Create decryption key if solver, otherwise derive encryption key from the finalized encryption keys
pub struct SyncFinalizedEncKeys<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature, Address: Clone> SyncFinalizedEncKeys<Signature, Address> {
    fn get_session_id(&self) -> SessionId {
        self.0.session_id()
    }

    fn payload(&self) -> Payload {
        self.0.commitment.payload.clone()
    }
}

impl<C: Config> RpcParameter<C> for SyncFinalizedEncKeys<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_finalized_enc_keys"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        let session_id = self.get_session_id();
        info!(
            "method::{:?} at session {:?}",
            <Self as RpcParameter<C>>::method(),
            session_id
        );
        let FinalizedEncKeyPayload {
            randomness,
            enc_keys,
        } = self
            .payload()
            .decode::<FinalizedEncKeyPayload<C::Signature, C::Address>>()
            .map_err(|e| RpcError::from(e))?;
        let mut enc_keys = enc_keys
            .iter()
            .map(|key| Ok(key.inner().commitment.payload.inner()))
            .collect::<Result<Vec<Vec<u8>>, RpcError>>()?;
        enc_keys.sort();
        let enc_key = ctx
            .key_generator()
            .read()
            .await
            .gen_enc_key(&randomness, Some(enc_keys))?;
        EncKey::new(enc_key.clone()).put(session_id)?;
        if ctx.is_solver() {
            ctx.async_task()
                .emit_event(
                    SessionEvent::SolveKey {
                        enc_key,
                        randomness,
                        session_id,
                    }
                    .into(),
                )
                .await
                .map_err(|e| RpcError::from(e))?;
        }
        Ok(())
    }
}
