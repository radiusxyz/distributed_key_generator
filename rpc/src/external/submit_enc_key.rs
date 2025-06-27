use crate::*;
use dkg_primitives::{Config, AsyncTask, EncKeyCommitment, RuntimeEvent, KeyServiceError, KeyGeneratorList, SignedCommitment, SubmitterList, DbManager};
use radius_sdk::kvstore::KvStoreError;
use serde::{Deserialize, Serialize};
use tracing::{info, error};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitEncKey<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature: Clone, Address: Clone> SubmitEncKey<Signature, Address> {
    pub fn sender(&self) -> Option<Address> {
        self.0.commitment.sender.clone()
    }

    pub fn inner(&self) -> SignedCommitment<Signature, Address> {
        self.0.clone()
    }
}

impl<C: Config> RpcParameter<C> for SubmitEncKey<C::Signature, C::Address> {
    
    type Response = ();

    fn method() -> &'static str {
        "submit_enc_key"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        // Leader of the session will handle the enc key submission
        let session_id = self.0.session_id();
        info!("method::{} for session: {:?}", <Self as RpcParameter<C>>::method(), session_id);
        if !ctx.is_leader(session_id) { 
            info!("Not a leader for session: {:?}. Skipping...", session_id);
            return Ok(()); 
        }
        let current_round = ctx.db_manager().current_round().map_err(|e| RpcError::from(e))?;
        let sender = ctx.verify_signature(&self.0.signature, &self.0.commitment, self.sender())?;
        let key_generators = KeyGeneratorList::<C::Address>::get(current_round)?;
        if !key_generators.contains(&sender) {
            return Err(RpcError::from(KeyServiceError::NotRegistered(sender.into())));
        } 
        // Store commitment for `session` and `sender`
        let commitment = EncKeyCommitment::new(self.inner());
        commitment.put(&session_id, &sender)?;

        let mut is_threshold_met = false;
        let mut commitments = Vec::new();

        if SubmitterList::<C::Address>::get(session_id).is_err() {
            info!("Initializing submitter list for session {:?}", session_id);
            SubmitterList::<C::Address>::new().put(session_id)?;
        }
        SubmitterList::<C::Address>::apply(session_id, |submitter_list| {
            submitter_list.insert(sender.clone());
            if submitter_list.len() >= ctx.threshold() as usize {
                info!("Threshold({}/{}) met for session {:?}", submitter_list.len(), ctx.threshold(), session_id);
                is_threshold_met = true;
                commitments = match submitter_list.clone().into_iter().map(|addr| {
                    EncKeyCommitment::<C::Signature, C::Address>::get(&session_id, &addr)
                }).collect::<Result<Vec<EncKeyCommitment<C::Signature, C::Address>>, KvStoreError>>().map_err(|e| RpcError::from(e)) {
                    Ok(commitments) => commitments,
                    Err(e) => {
                        error!("Error getting encryption key list at {:?}: {:?}", session_id, e);
                        is_threshold_met = false;
                        Vec::new()
                    }
                }
            }
        })?;
        if is_threshold_met {
            ctx.async_task().emit_event(RuntimeEvent::FinalizeKey { commitments, start_session_id: session_id }).await.map_err(|e| RpcError::from(e))?;
        }

        let _ = multicast_enc_key_ack(&ctx, session_id, commitment);

        Ok(())
    }
}
