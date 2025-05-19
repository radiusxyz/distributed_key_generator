use crate::{
    primitives::*,
    cluster::request_submit_partial_key::submit_partial_key_to_leader
};
use dkg_primitives::{
    AppState,
    SessionId, 
    SyncDecryptionKeyPayload,
    KeyGeneratorList, 
    AggregatedKey, 
    DecryptionKey,
};
use serde::{Deserialize, Serialize};
use skde::key_generation::generate_partial_key;
use tracing::debug;

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
        let prefix = context.log_prefix();
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
            &prefix,
        )?;

        decryption_key.put(session_id)?;

        let (_, partial_key) = generate_partial_key(&skde_params).unwrap();
        let next_session_id = session_id.next().unwrap(); //TODO: Remove unwrap
        submit_partial_key_to_leader(next_session_id, partial_key, &context.clone()).await?;

        debug!(target: "dkg-rpc", "{} Completed submitting partial key", prefix);

        Ok(())
    }
}

// Broadcast decryption key acknowledgment from leader to the network
pub fn broadcast_decryption_key_ack<C: AppState>(
    session_id: SessionId,
    decryption_key: String,
    solve_timestamp: u128,
    context: &C,
) -> Result<(), C::Error> {
    let prefix = context.log_prefix();
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::<C::Address>::get()?.get_all_key_generator_rpc_url_list();

    debug!(
        target: "dkg-rpc",
        "{} Broadcast decryption key - session_id: {:?}, all_dkg_list: {:?}",
        prefix, session_id, all_key_generator_rpc_url_list
    );

    let payload = SyncDecryptionKeyPayload::new(
        context.address(),
        decryption_key,
        session_id,
        solve_timestamp,
    );

    let signature = context.sign(&payload)?;

    let parameter = SyncDecryptionKey { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    all_key_generator_rpc_url_list,
                    <SyncDecryptionKey::<C::Signature, C::Address> as RpcParameter<C>>::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
