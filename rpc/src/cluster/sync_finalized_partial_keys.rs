use crate::{primitives::*, SubmitDecKeyResponse, SubmitDecKey, DecKeyPayload, FinalizedEncKeyPayload};
use dkg_primitives::{AppState, AsyncTask, Commitment, DecKey, EncKey, Payload, SecureBlock, SessionId, SignedCommitment, SubmitterList};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

#[derive(Clone, Debug, Deserialize, Serialize)]
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
        let session_id = self.get_session_id();
        SubmitterList::<C::Address>::initialize(session_id)?;
        let enc_keys = self.payload()
            .decode::<FinalizedEncKeyPayload<C::Signature, C::Address>>()
            .map_err(|e| RpcError::from(e))?
            .inner()
            .iter()
            .map(|key| {
                let signer = ctx.verify_signature(&key.inner().signature, &key.inner().commitment, key.inner().commitment.sender.clone())?;
                SubmitterList::<C::Address>::apply(session_id, |list| { list.insert(signer.clone()); })?;
                key.put(&session_id, &signer)?;
                key.inner().commitment.payload.decode::<EncKey>().map_err(|e| RpcError::from(e)).map(|enc_key| enc_key.inner())
            })
            .collect::<Result<Vec<Vec<u8>>, RpcError>>()?;
        if ctx.is_solver() {
            let _ = ctx.verify_signature(&self.0.signature, &self.payload(), self.0.commitment.sender.clone())?;
            let cloned_ctx = ctx.clone();
            cloned_ctx.async_task().spawn_task(Box::pin(
                async move {
                    match solve::<C>(&ctx, session_id, enc_keys) {
                        Ok(payload) => {
                            let bytes = serde_json::to_vec(&payload).unwrap();
                            let commitment = Commitment::new(bytes.into(), Some(ctx.address()), session_id);
                            let signature = ctx.sign(&commitment).unwrap();
                            let leader_rpc_url = ctx.leader_rpc_url().unwrap();
                            
                            // TODO: Handle Error
                            let response: SubmitDecKeyResponse = ctx
                                .async_task()
                                .request(
                                    leader_rpc_url,
                                    <SubmitDecKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(),
                                    SubmitDecKey(SignedCommitment { signature, commitment }),
                                )
                                .await.unwrap();
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
            ));
        } else {
            let key = ctx.secure_block().gen_enc_key(ctx.randomness(session_id), Some(enc_keys))?;
            EncKey::new(key).put(session_id)?;
        }
        Ok(())
    }
}

fn solve<C: AppState>(
    ctx: &C,
    session_id: SessionId,
    enc_keys: Vec<Vec<u8>>,
) -> Result<DecKeyPayload, RpcError> {
    let enc_key = ctx.secure_block().gen_enc_key(ctx.randomness(session_id), Some(enc_keys)).map_err(|e| RpcError::from(e))?;
    let (dec_key, solve_at) = ctx.secure_block().gen_dec_key(&enc_key).map_err(|e| RpcError::from(e))?;
    ctx.secure_block().verify_dec_key(&enc_key, &dec_key).map_err(|e| RpcError::from(e))?;
    DecKey::new(dec_key.clone()).put(session_id).map_err(|e| RpcError::from(e))?;
    Ok(DecKeyPayload::new(dec_key, solve_at))
}
