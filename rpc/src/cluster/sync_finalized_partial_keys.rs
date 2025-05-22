use crate::{primitives::*, DecryptionKeyResponse, SubmitDecryptionKey};
use dkg_primitives::{AppState, DecryptionKey, Error, SubmitterList, SessionId, SubmitDecryptionKeyPayload, SyncFinalizedPartialKeysPayload, AsyncTask};
use dkg_utils::key::{calculate_decryption_key, perform_randomized_aggregation, verify_encryption_decryption_key_pair};
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
        if ctx.is_solver() {
            SubmitterList::<C::Address>::initialize(session_id)?;
            let _ = ctx.verify_signature(&self.signature, &self.payload, Some(&self.payload.sender))?;
            let partial_keys = get_partial_keys::<C>(&ctx, &self.payload)?;
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
            let partial_keys = get_partial_keys::<C>(&ctx, &self.payload)?;
            perform_randomized_aggregation(&ctx, session_id, &partial_keys);
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
    let aggregated_key = perform_randomized_aggregation(&ctx, session_id, &partial_keys);
    let decryption_key: String = calculate_decryption_key(&ctx, session_id, &aggregated_key)
        .unwrap()
        .into();
    let encryption_key = aggregated_key.u;
    verify_encryption_decryption_key_pair(&ctx.skde_params(), &encryption_key, &decryption_key)?;

    DecryptionKey::new(decryption_key.clone()).put(session_id)?;

    let payload =
        SubmitDecryptionKeyPayload::new(ctx.address(), decryption_key.clone(), session_id);
    let timestamp = payload.timestamp;
    let signature = ctx.sign(&payload)?;
    let leader_rpc_url = ctx.leader_rpc_url().ok_or(Error::InvalidParams("Leader RPC URL is not set".to_string()))?;
    // TODO: Handle Error
    let response: DecryptionKeyResponse = ctx
        .async_task()
        .request(
            leader_rpc_url,
            <SubmitDecryptionKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(),
            SubmitDecryptionKey { signature, payload },
        )
        .await?;
    if response.success {
        info!("Successfully submitted decryption key - session_id: {:?}, timestamp: {}", session_id, timestamp);
    } else {
        warn!("Submission acknowledged but not successful - session_id: {:?}, timestamp: {}", session_id, timestamp);
    }

    Ok(())
}

pub fn get_partial_keys<C: AppState>(
    ctx: &C,
    payload: &SyncFinalizedPartialKeysPayload<C::Signature, C::Address>,
) -> Result<Vec<PartialKey>, RpcError> {
    let SyncFinalizedPartialKeysPayload { session_id, ack_timestamp, .. } = payload;

    info!(
        "Received finalized partial keys - num: {:?}, session_id: {:?}, timestamp: {}",
        payload.len(),
        session_id,
        ack_timestamp
    );

    let partial_keys = payload
        .partial_keys()
        .iter()
        .try_fold(Vec::new(), |mut acc, key| -> Result<Vec<PartialKey>, C::Error> {
            let signer = ctx.verify_signature(&key.signature, &key.payload, Some(&key.sender()))?;
            SubmitterList::<C::Address>::initialize(*session_id)?;
            SubmitterList::apply(*session_id, |list| {
                list.insert(signer.clone());
            })?;
            key.put(*session_id, signer)?;
            acc.push(key.partial_key());
            Ok(acc)
        })?;

    Ok(partial_keys)
}
