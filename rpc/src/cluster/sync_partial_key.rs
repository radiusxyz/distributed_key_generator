use crate::primitives::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use dkg_primitives::{SessionId, AppState, KeyGeneratorList, SubmitterList, PartialKeySubmission, SyncPartialKeyPayload};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncPartialKeyPayload<Signature, Address>,
}

impl<Signature: Clone, Address: Clone> SyncPartialKey<Signature, Address> {
    fn session_id(&self) -> SessionId {
        self.payload.session_id
    }

    fn partial_key_sender(&self) -> Address {
        self.payload.partial_key_submission.sender().clone()
    }

    fn partial_key(&self) -> PartialKeySubmission<Signature, Address> {
        self.payload.partial_key_submission.clone()
    }
}

impl<C: AppState> RpcParameter<C> for SyncPartialKey<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> { 
        info!("Received partial key from {:?}", self.payload.sender());
        // If partial_key_sender is node itself, ignore
        let session_id = self.session_id();
        if self.payload.partial_key_sender() == &context.address() { return Ok(()); }
        let _ = context.verify_signature(&self.signature, &self.payload, Some(&self.payload.sender()))?;

        SubmitterList::<C::Address>::initialize(session_id)?;
        SubmitterList::apply(session_id, |list| {
            list.insert(self.partial_key_sender());
        })?;
        self.partial_key().put(session_id, self.partial_key_sender())?;

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
    let committee_urls =
        KeyGeneratorList::<C::Address>::get()
            .map_err(|e| C::Error::from(e))?
            .into_iter()
            .filter(|kg| kg.address() != ctx.address())
            .map(|kg| kg.cluster_rpc_url().to_owned())
            .collect();
    info!("Broadcasting partial key ack to {:?}", committee_urls);
    let payload = SyncPartialKeyPayload::new(ctx.address(), partial_key_submission);
    let signature = ctx.sign(&payload)?;
    ctx.multicast(committee_urls, <SyncPartialKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SyncPartialKey { signature, payload });

    Ok(())
}
