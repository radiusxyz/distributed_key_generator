use crate::{primitives::*, SubmitDecKeyResponse, SubmitDecKey};
use dkg_primitives::{AppState, DecKey, Error, SubmitterList, SessionId, SubmitDecKeyPayload, SyncFinalizedPartialKeysPayload, SecureBlock, AsyncTask, get_partial_keys};
use dkg_utils::key::{get_dec_key, get_enc_key, verify_key_pair};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey;
use tracing::{error, info, warn};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeys<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload<Signature, Address>,
}

impl<Signature, Address> SyncFinalizedPartialKeys<Signature, Address> {
    pub fn new(signature: Signature, payload: SyncFinalizedPartialKeysPayload<Signature, Address>) -> Self {
        Self { signature, payload }
    }

    fn session_id(&self) -> SessionId {
        self.payload.session_id
    }
}

impl<C: AppState> RpcParameter<C> for SyncFinalizedPartialKeys<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_finalized_partial_keys"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let session_id = self.session_id();
        let partial_keys = get_partial_keys::<C>(&ctx, session_id, &self.payload.partial_keys())?;
        if ctx.is_solver() {
            SubmitterList::<C::Address>::initialize(session_id)?;
            let _ = ctx.verify_signature(&self.signature, &self.payload, Some(&self.payload.sender))?;
            let cloned_ctx = ctx.clone();
            cloned_ctx.async_task().spawn_task(Box::pin(
                async move {
                    if let Err(err) =
                        derive_decryption_key::<C>(ctx, session_id, &partial_keys)
                            .await
                    {
                        error!(
                            "Solve failed for session {:?}: {:?}",
                            session_id, err
                        );
                    } else {
                        info!(
                            "Solve completed successfully for session {:?}",
                            session_id
                        );
                    }
                }
            ));
        } else {
            let _ = get_enc_key(&ctx, session_id, &partial_keys)?;
        }
        Ok(())
    }
}

// TODO: Refactor 
// ```
// let dec_key = ctx.derive_dec_key()?;
// ctx.request()?;
// Ok(())
// ```
async fn derive_decryption_key<C: AppState>(
    ctx: C,
    session_id: SessionId,
    partial_keys: &[PartialKey],
) -> Result<(), RpcError> {
    let enc_key = get_enc_key(&ctx, session_id, &partial_keys)?;
    let decryption_key: String = get_dec_key(&ctx, session_id, &enc_key.inner())?.into();
    verify_key_pair(&ctx.skde_params(), &enc_key.key(), &decryption_key)?;

    DecKey::new(decryption_key.clone()).put(session_id)?;

    let payload =
        SubmitDecKeyPayload::new(ctx.address(), decryption_key.clone(), session_id);
    let timestamp = payload.timestamp;
    let signature = ctx.sign(&payload)?;
    let leader_rpc_url = ctx.leader_rpc_url().ok_or(Error::InvalidParams("Leader RPC URL is not set".to_string()))?;
    // TODO: Handle Error
    let response: SubmitDecKeyResponse = ctx
        .async_task()
        .request(
            leader_rpc_url,
            <SubmitDecKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(),
            SubmitDecKey { signature, payload },
        )
        .await?;
    if response.success {
        info!("Successfully submitted decryption key - session_id: {:?}, timestamp: {}", session_id, timestamp);
    } else {
        warn!("Submission acknowledged but not successful - session_id: {:?}, timestamp: {}", session_id, timestamp);
    }

    Ok(())
}
