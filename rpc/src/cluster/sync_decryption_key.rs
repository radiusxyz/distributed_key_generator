use crate::{primitives::*, submit_partial_key};
use dkg_primitives::{AppState, SessionId, SyncDecKeyPayload, KeyGeneratorList, EncKey, DecKey, AsyncTask};
use serde::{Deserialize, Serialize};
use skde::key_generation::generate_partial_key;
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncDecKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncDecKeyPayload<Address>,
}

// TODO (Post-PoC): Decouple session start trigger from decryption key sync to improve robustness.
impl<C: AppState> RpcParameter<C> for SyncDecKey<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_decryption_key"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        info!("Received decryption key from {:?}", self.payload.sender);
        let _ = ctx.verify_signature(
            &self.signature, 
            &self.payload, 
            Some(&self.payload.sender)
        )?;

        let session_id = self.payload.session_id;
        let skde_params = ctx.skde_params();
        let enc_key = EncKey::get(session_id)?.key();
        let dec_key = DecKey::new(self.payload.decryption_key.clone());

        ctx.verify_decryption_key(
            &skde_params,
            enc_key,
            dec_key.clone().into(),
        )?;

        dec_key.put(session_id)?;

        let (_, partial_key) = generate_partial_key(&skde_params).unwrap();
        let next_session_id = session_id.next().unwrap(); //TODO: Remove unwrap
        submit_partial_key(next_session_id, partial_key, &ctx).await?;

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

    let payload = SyncDecKeyPayload::new(ctx.address(), decryption_key, session_id, solve_timestamp);
    let signature = ctx.sign(&payload)?;

    ctx.async_task().multicast(committee_urls, <SyncDecKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SyncDecKey { signature, payload });

    Ok(())
}
