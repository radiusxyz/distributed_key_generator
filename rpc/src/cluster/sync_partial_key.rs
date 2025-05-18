use crate::primitives::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use dkg_primitives::{
    AppState,
    KeyGeneratorList,
    PartialKeyAddressList,
    PartialKeySubmission,
    SyncPartialKeyPayload,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncPartialKeyPayload<Signature, Address>,
}

impl<C> RpcParameter<C> for SyncPartialKey<C::Signature, C::Address> 
where
    C: AppState
{
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let prefix = context.log_prefix();

        // If partial_key_sender is node itself, ignore
        if self.payload.partial_key_sender() == &context.address() {
            return Ok(());
        }

        let _ = context.verify_signature(&self.signature, &self.payload, &self.payload.sender())?;

        info!("{} {}", prefix, self.payload);

        PartialKeyAddressList::<C::Address>::initialize(self.payload.session_id)?;
        PartialKeyAddressList::apply(self.payload.session_id, |list| {
            list.insert(self.payload.partial_key_sender().clone());
        })?;

        let partial_key_submission = self.payload.partial_key_submission.clone();
        partial_key_submission.put(
            self.payload.session_id,
            self.payload.partial_key_sender().clone()
        )?;

        Ok(())
    }
}

// Broadcast partial key acknowledgment from leader to the entire network
pub fn broadcast_partial_key_ack<C>(
    partial_key_submission: PartialKeySubmission<C::Signature, C::Address>,
    context: &C,
) -> Result<(), C::Error> 
where
    C: AppState,
{
    let prefix = context.log_prefix();
    let key_generator_rpc_url_list =
        KeyGeneratorList::<C::Address>::get()
            .map_err(|e| C::Error::from(e))?
            .get_other_key_generator_rpc_url_list(&context.address());

    info!("{} {}", prefix, partial_key_submission);

    let payload = SyncPartialKeyPayload::new(
        context.address(),
        partial_key_submission,
    );

    let signature = context.sign(&payload)?;

    let parameter = SyncPartialKey { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    key_generator_rpc_url_list,
                    <SyncPartialKey::<C::Signature, C::Address> as RpcParameter<C>>::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
