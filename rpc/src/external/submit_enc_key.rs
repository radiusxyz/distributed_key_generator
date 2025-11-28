use dkg_primitives::{
    ActiveCommitteeList, Config, EncKeyCommitment, SessionEvent, SignedCommitment, SubmitterList,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitEncKey<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature: Clone, Address: Clone> SubmitEncKey<Signature, Address> {
    pub fn sender(&self) -> Option<Address> {
        self.0.commitment.sender.clone()
    }

    pub fn inner(&self) -> SignedCommitment<Signature, Address> {
        self.0.clone()
    }

    pub fn payload(&self) -> Payload {
        self.0.commitment.payload.clone()
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
        info!(
            "method::{} for session: {:?}",
            <Self as RpcParameter<C>>::method(),
            session_id
        );
        if !ctx.is_leader(session_id) {
            debug!("Not a leader for session: {:?}. Skipping...", session_id);
            return Ok(());
        }
        let submitter =
            ctx.verify_signature(&self.0.signature, &self.0.commitment, self.sender())?;

        // Sanity check: if the sender is not a key generator, skip
        let committees = ActiveCommitteeList::<C::Address>::get()?;
        if !committees.contains(&submitter) {
            return Ok(());
        }
        // Store commitment for `session` and `sender`
        let commitment = EncKeyCommitment::<C::Signature, C::Address>::new(self.inner());
        commitment.put(&session_id, &submitter)?;

        if SubmitterList::<C::Address>::get(session_id).is_err() {
            info!("Initializing submitter list for session {:?}", session_id);
            SubmitterList::<C::Address>::new().put(session_id)?;
        }
        SubmitterList::<C::Address>::apply(session_id, |submitter_list| {
            submitter_list.insert(submitter.clone());
        })?;
        ctx.async_task()
            .emit_event(
                SessionEvent::SubmitEncKey {
                    submitter,
                    session_id,
                }
                .into(),
            )
            .await
            .map_err(|e| RpcError::from(e))?;

        let _ = multicast_enc_key_ack(ctx, session_id, commitment);

        Ok(())
    }
}
