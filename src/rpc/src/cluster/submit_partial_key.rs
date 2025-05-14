use tracing::info;
use dkg_utils::{
    log::{log_prefix_with_session_id, AddressExt},
    signature::verify_signature,
};
use dkg_types::{error::KeyGenerationError, KeyGeneratorList};

use crate::{
    primitives::*,
    cluster::broadcast_partial_key_ack,
    common::PartialKeyPayload,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKey {
    pub signature: Signature,
    pub payload: PartialKeyPayload,
}

impl RpcParameter<AppState> for SubmitPartialKey {
    type Response = ();

    fn method() -> &'static str {
        "submit_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let sender_address = verify_signature(&self.signature, &self.payload)?;

        if sender_address != self.payload.sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }

        let prefix = log_prefix_with_session_id(context.config(), &self.payload.session_id);

        info!(
            "{} Received partial key - session_id: {:?}, sender: {}, timestamp: {}",
            prefix,
            self.payload.session_id,
            self.payload.sender.to_short(),
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

        let partial_key_submission = PartialKeySubmission::from_submit_partial_key(&self);
        partial_key_submission.put(self.payload.session_id, &self.payload.sender)?;

        let _ = broadcast_partial_key_ack(sender_address, partial_key_submission, &context);

        Ok(())
    }
}
