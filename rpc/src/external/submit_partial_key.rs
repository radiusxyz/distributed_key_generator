use crate::{cluster::broadcast_partial_key_ack, primitives::*};
use dkg_primitives::{
    AppState,
    KeyGenerationError,
    PartialKeyPayload,
    KeyGeneratorList,
    SubmitterList,
    PartialKeySubmission,
    Event,
    AsyncTask,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKey<Signature, Address> {
    pub signature: Signature,
    pub payload: PartialKeyPayload<Address>,
}

impl<C: AppState> RpcParameter<C> for SubmitPartialKey<C::Signature, C::Address> {
    
    type Response = ();

    fn method() -> &'static str {
        "submit_partial_key"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let sender_address = ctx.verify_signature(
            &self.signature, 
            &self.payload, 
            Some(&self.payload.sender)
        )?;
        let session_id = self.payload.session_id;
        info!(
            "Received partial key - session_id: {:?}, sender: {:?}, timestamp: {}",
            session_id,
            sender_address,
            self.payload.submit_timestamp
        );

        // Check if key generator is registered in the cluster
        let key_generator_list = KeyGeneratorList::get()?;
        if !key_generator_list.contains(&sender_address) {
            return Err(RpcError::from(KeyGenerationError::NotRegisteredGenerator(sender_address.into())));
        } 

        let partial_key_submission = PartialKeySubmission::new(self.signature, self.payload);
        partial_key_submission.put(session_id, sender_address.clone())?;

        let mut is_threshold_met = false;
        let mut partial_key_list = Vec::new();

        SubmitterList::<C::Address>::apply(session_id, |list| {
            list.insert(sender_address.clone());
            if list.len() >= ctx.threshold() as usize {
                info!("Threshold met for session {:?}", session_id);
                is_threshold_met = true;
                partial_key_list = match list.get_partial_keys::<C>(session_id) {
                    Ok(list) => list,
                    Err(e) => {
                        error!("Error getting partial key list at {:?}: {:?}", session_id, e);
                        is_threshold_met = false;
                        Vec::new()
                    }
                };
            }
        })?;
        if is_threshold_met {
            ctx.async_task().emit_event(Event::ThresholdMet(partial_key_list)).await.map_err(|e| {
                error!("Error emitting event: {:?}", e);
                RpcError::from(e)
            })?;
        }

        let _ = broadcast_partial_key_ack(partial_key_submission, &ctx);

        Ok(())
    }
}
