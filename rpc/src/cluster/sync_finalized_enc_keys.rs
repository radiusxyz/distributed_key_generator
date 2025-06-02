use crate::{*, SubmitDecKeyResponse, SubmitDecKey, DecKeyPayload, FinalizedEncKeyPayload};
use dkg_primitives::{AppState, AsyncTask, Commitment, DecKey, EncKey, Payload, SecureBlock, SessionId, SignedCommitment, SubmitterList};
use radius_sdk::json_rpc::server::RpcError;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Handler for syncing the finalized encryption keys.
/// Create decryption key if solver, otherwise derive encryption key from the finalized encryption keys
pub struct SyncFinalizedEncKeys<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature, Address: Clone> SyncFinalizedEncKeys<Signature, Address> {
    fn get_session_id(&self) -> SessionId { self.0.session_id() }

    fn payload(&self) -> Payload { self.0.commitment.payload.clone() }
}

impl<C: AppState> RpcParameter<C> for SyncFinalizedEncKeys<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_finalized_enc_keys"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        info!("Syncing finalized encryption keys");
        let session_id = self.get_session_id();
        SubmitterList::<C::Address>::initialize(session_id)?;
        let mut enc_keys = self.payload()
            .decode::<FinalizedEncKeyPayload<C::Signature, C::Address>>()
            .map_err(|e| RpcError::from(e))?
            .inner()
            .iter()
            .map(|key| {
                let signer = ctx.verify_signature(&key.inner().signature, &key.inner().commitment, key.inner().commitment.sender.clone())?;
                SubmitterList::<C::Address>::apply(session_id, |list| { list.insert(signer.clone()); })?;
                key.put(&session_id, &signer)?;
                Ok(key.inner().commitment.payload.inner())
            })
            .collect::<Result<Vec<Vec<u8>>, RpcError>>()?;
        enc_keys.sort();
        let enc_key = ctx.secure_block().gen_enc_key(ctx.randomness(session_id), Some(enc_keys))?;
        EncKey::new(enc_key.clone()).put(session_id)?;
        if ctx.is_solver() {
            let _ = ctx.verify_signature(&self.0.signature, &self.0.commitment, self.0.commitment.sender.clone())?;
            // Since we already check before start the node, it is guaranteed to be Some
            let leader = ctx.current_leader(false).map_err(|e| RpcError::from(e))?;
            let cloned_ctx = ctx.clone();
            cloned_ctx.async_task().spawn_task(
                async move {
                    match solve::<C>(&ctx, session_id, &enc_key) {
                        Ok(commitment) => {
                            info!("🔑 Solved!");
                            // TODO: Handle Error
                            let response: SubmitDecKeyResponse = match ctx
                                .async_task()
                                .request(
                                    leader.1,
                                    <SubmitDecKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(),
                                    SubmitDecKey(commitment),
                                )
                                .await {
                                    Ok(response) => response,
                                    Err(e) => {
                                        error!("Failed to submit dec key to leader: {:?}", e);
                                        return;
                                    }
                                };
                            if !response.0 {
                                // TODO: SHOULD HANDLE ERROR
                                warn!("Submission acknowledged but not successful - session_id: {:?}", session_id);
                            }
                        },
                        Err(e) => {
                            error!("Solve failed for session {:?}: {:?}", session_id, e);
                        }
                    }
                }
            );
        }
        Ok(())
    }
}

/// Solve based on the given encryption keys and create a signed commitment
fn solve<C: AppState>(
    ctx: &C,
    session_id: SessionId,
    enc_key: &Vec<u8>,
) -> Result<SignedCommitment<C::Signature, C::Address>, RpcError> {
    let (dec_key, solve_at) = ctx.secure_block().gen_dec_key(enc_key).map_err(|e| RpcError::from(e))?;
    ctx.secure_block().verify_dec_key(&enc_key, &dec_key).map_err(|e| RpcError::from(e))?;
    DecKey::new(dec_key.clone()).put(session_id).map_err(|e| RpcError::from(e))?;
    let payload = DecKeyPayload::new(dec_key, solve_at);
    let bytes = serde_json::to_vec(&payload).map_err(|e| RpcError::from(e))?;
    let commitment = Commitment::new(bytes.into(), Some(ctx.address()), session_id);
    let signature = ctx.sign(&commitment)?;
    Ok(SignedCommitment { signature, commitment })
}
