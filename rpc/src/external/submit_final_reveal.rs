use crate::{primitives::*, SyncDecryptionKey, SyncPartialKey};
use dkg_primitives::{AppState, SessionId, KeyGeneratorList, AsyncTask};
use serde::{Deserialize, Serialize};
use tracing::info;

// TODO: Add handler to submit partial keys and decryption key from leader to a verifier
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitFinalReveal<Signature, Address> {
    pub signature: Signature,
    pub payload: FinalRevealPayload<Signature, Address>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalRevealPayload<Signature, Address> {
    pub session_id: SessionId,
    pub partial_keys: Vec<SyncPartialKey<Signature, Address>>,
    pub sync_decryption_key: SyncDecryptionKey<Signature, Address>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevealResponse {
    pub success: bool,
}

impl<C: AppState> RpcParameter<C> for SubmitFinalReveal<C::Signature, C::Address> {
    type Response = RevealResponse;

    fn method() -> &'static str {
        "submit_final_reveal"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let prefix = ctx.log_prefix();
        info!(
            "{} Received final reveal - session_id: {:?}, partial_keys: {}",
            prefix,
            self.payload.session_id,
            self.payload.partial_keys.len()
        );
        Ok(RevealResponse { success: true })
    }
}

// TODO: Verifier - Broadcast final reveal information from leader
pub fn broadcast_final_reveal<C: AppState>(
    session_id: SessionId,
    partial_keys: Vec<SyncPartialKey<C::Signature, C::Address>>,
    sync_decryption_key: SyncDecryptionKey<C::Signature, C::Address>,
    ctx: &C,
) -> Result<(), C::Error> {
    let committee_urls = KeyGeneratorList::<C::Address>::get()?.all_rpc_urls(false);
    let payload = FinalRevealPayload { session_id, partial_keys, sync_decryption_key };
    let signature = ctx.sign(&payload)?;
    ctx.async_task().multicast(committee_urls, <SubmitFinalReveal::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SubmitFinalReveal { signature, payload });
    Ok(())
}
