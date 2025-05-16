use crate::primitives::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use dkg_primitives::{
    AppState,
    KeyGeneratorList,
    KeyGenerationError,
    PartialKeyAddressList,
    PartialKeySubmission,
    SyncPartialKeyPayload,
    Error,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncPartialKeyPayload<Signature, Address>,
}

impl<C: AppState> RpcParameter<C> for SyncPartialKey<C::Signature, C::Address> 
where
    C::Signature: Send + 'static,
{
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let prefix = context.log_prefix();

        // If partial_key_sender is me, ignore
        let partial_key_sender = &self.payload.partial_key_submission.payload.sender;
        if partial_key_sender == &context.address() {
            return Ok(());
        }

        let sender_address = context.verify_signature(&self.signature, &self.payload)?;
        if sender_address != self.payload.sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }

        info!(
            "{} Received partial key ACK - sender:{:?}, session_id: {:?
            }, timestamp: {}",
            prefix,
            &self.payload.partial_key_submission.payload.sender,
            self.payload.session_id,
            self.payload.ack_timestamp
        );

        PartialKeyAddressList::initialize(self.payload.session_id)?;
        PartialKeyAddressList::apply(self.payload.session_id, |list| {
            list.insert(self.payload.partial_key_submission.payload.sender.clone());
        })?;

        let partial_key_submission = self.payload.partial_key_submission.clone();
        partial_key_submission.put(
            self.payload.session_id,
            &self.payload.partial_key_submission.payload.sender,
        )?;

        Ok(())
    }
}

// Broadcast partial key acknowledgment from leader to the entire network
pub fn broadcast_partial_key_ack<C: AppState>(
    partial_key_submission: PartialKeySubmission,
    context: &C,
) -> Result<(), Error> 
where
    Error: From<C::Error>,
{
    let prefix = context.log_prefix();
    let key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_other_key_generator_rpc_url_list(&context.address());
    // let key_generator_rpc_url_list = KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    info!(
        "{} Broadcasting partial key acknowledgment - sender: {:?}, session_id: {:?}, timestamp: {}",
        prefix,
        partial_key_submission.payload.sender,
        partial_key_submission.payload.session_id,
        partial_key_submission.payload.submit_timestamp
    );

    let payload = SyncPartialKeyPayload::new(
        context.address(),
        partial_key_submission,
        partial_key_submission.payload.session_id,
    );

    let signature = context.create_signature(&payload)?;

    let parameter = SyncPartialKey { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    key_generator_rpc_url_list,
                    SyncPartialKey::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
