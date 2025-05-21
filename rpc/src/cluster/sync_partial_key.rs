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

impl<C: AppState> RpcParameter<C> for SyncPartialKey<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {

        // If partial_key_sender is node itself, ignore
        if self.payload.partial_key_sender() == &context.address() {
            return Ok(());
        }

        let _ = context.verify_signature(&self.signature, &self.payload, Some(&self.payload.sender()))?;

        info!("{} {}", <Self as RpcParameter<C>>::method(), self.payload);

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
    ctx: &C,
) -> Result<(), C::Error> 
where
    C: AppState,
{
    info!("Broadcasting partial key acknowledgment from leader to the entire network");
    let key_generator_rpc_url_list =
        KeyGeneratorList::<C::Address>::get()
            .map_err(|e| C::Error::from(e))?
            .get_other_key_generator_rpc_url_list(&ctx.address());
    let payload = SyncPartialKeyPayload::new(
        ctx.address(),
        partial_key_submission,
    );
    let signature = ctx.sign(&payload)?;
    ctx.multicast(key_generator_rpc_url_list, <SyncPartialKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SyncPartialKey { signature, payload });

    Ok(())
}
