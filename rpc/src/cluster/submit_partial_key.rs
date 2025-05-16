use crate::{cluster::broadcast_partial_key_ack, primitives::*};
use radius_sdk::signature::Signature;
use dkg_primitives::{
    AppState,
    KeyGenerationError,
    PartialKeyPayload,
    KeyGeneratorList,
    PartialKeyAddressList,
    PartialKeySubmission,
};
use dkg_utils::{signature::verify_signature, short_addr};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKey {
    pub signature: Signature,
    pub payload: PartialKeyPayload,
}

impl<C: AppState> RpcParameter<C> for SubmitPartialKey 
where
    C: 'static
{
    type Response = ();

    fn method() -> &'static str {
        "submit_partial_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let sender_address = verify_signature(&self.signature, &self.payload)?;

        if sender_address != self.payload.sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }

        info!(
            "{} Received partial key - session_id: {:?}, sender: {}, timestamp: {}",
            context.log_prefix(),
            self.payload.session_id,
            short_addr(&self.payload.sender),
            self.payload.submit_timestamp
        );

        // Check if key generator is registered in the cluster
        let key_generator_list = KeyGeneratorList::get()?;
        if !key_generator_list.is_key_generator_in_cluster(&self.payload.sender) {
            return Err(RpcError::from(KeyGenerationError::NotRegisteredGenerator(
                self.payload.sender.as_hex_string(),
            )));
        }

        PartialKeyAddressList::apply(self.payload.session_id, |list| {
            list.insert(sender_address.clone());
        })?;

        let partial_key_submission = PartialKeySubmission::new(self.signature, self.payload);
        partial_key_submission.put(self.payload.session_id, &self.payload.sender)?;

        let _ = broadcast_partial_key_ack(partial_key_submission, &context);

        Ok(())
    }
}
