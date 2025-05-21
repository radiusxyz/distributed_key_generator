use crate::{primitives::*, submit_partial_key};
use dkg_primitives::{AppState, SessionId, SyncDecryptionKeyPayload, KeyGeneratorList, AggregatedKey, DecryptionKey};
use serde::{Deserialize, Serialize};
use skde::key_generation::generate_partial_key;
use tracing::{info, debug};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncDecryptionKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncDecryptionKeyPayload<Address>,
}

// TODO (Post-PoC): Decouple session start trigger from decryption key sync to improve robustness.
impl<C: AppState> RpcParameter<C> for SyncDecryptionKey<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_decryption_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        info!("Received decryption key from {:?}", self.payload.sender);
        let _ = context.verify_signature(
            &self.signature, 
            &self.payload, 
            Some(&self.payload.sender)
        )?;

        let session_id = self.payload.session_id;
        let skde_params = context.skde_params();
        let encryption_key = AggregatedKey::get(session_id)?.enc_key();
        let decryption_key = DecryptionKey::new(self.payload.decryption_key.clone());

        context.verify_decryption_key(
            &skde_params,
            encryption_key,
            decryption_key.clone().into(),
        )?;

        decryption_key.put(session_id)?;

        let (_, partial_key) = generate_partial_key(&skde_params).unwrap();
        let next_session_id = session_id.next().unwrap(); //TODO: Remove unwrap
        submit_partial_key(next_session_id, partial_key, &context.clone()).await?;

        info!("Completed submitting partial key");

        Ok(())
    }
}

// Broadcast decryption key acknowledgment from leader to the network
pub fn broadcast_decryption_key_ack<C: AppState>(
    session_id: SessionId,
    decryption_key: String,
    solve_timestamp: u128,
    ctx: &C,
) -> Result<(), C::Error> {
    let committee_urls = KeyGeneratorList::<C::Address>::get()?.all_rpc_urls(true);

    info!("Broadcast decryption key - session_id: {:?}, all_dkg_list: {:?}", session_id, committee_urls);

    let payload = SyncDecryptionKeyPayload::new(ctx.address(), decryption_key, session_id, solve_timestamp);
    let signature = ctx.sign(&payload)?;

    ctx.multicast(committee_urls, <SyncDecryptionKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SyncDecryptionKey { signature, payload });

    Ok(())
}
