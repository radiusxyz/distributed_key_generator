use crate::{cluster::broadcast_partial_key_ack, primitives::*};
use dkg_primitives::{
    AppState,
    KeyGenerationError,
    PartialKeyPayload,
    KeyGeneratorList,
    PartialKeyAddressList,
    PartialKeySubmission,
};
use serde::{Deserialize, Serialize};
use tracing::info;

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

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let sender_address = context.verify_signature(
            &self.signature, 
            &self.payload, 
            Some(&self.payload.sender)
        )?;
        let session_id = self.payload.session_id;
        info!(
            "{} Received partial key - session_id: {:?}, sender: {:?}, timestamp: {}",
            context.log_prefix(),
            session_id,
            sender_address,
            self.payload.submit_timestamp
        );

        // Check if key generator is registered in the cluster
        let key_generator_list = KeyGeneratorList::get()?;
        if !key_generator_list.is_key_generator_in_cluster(&sender_address) {
            return Err(RpcError::from(KeyGenerationError::NotRegisteredGenerator(
                sender_address.into(),
            )));
        }

        PartialKeyAddressList::apply(session_id, |list| {
            list.insert(sender_address.clone());
        })?;

        let partial_key_submission = PartialKeySubmission::new(self.signature, self.payload);
        partial_key_submission.put(session_id, sender_address)?;

        let _ = broadcast_partial_key_ack(partial_key_submission, &context);

        Ok(())
    }
}
