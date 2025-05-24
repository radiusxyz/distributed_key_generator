use dkg_primitives::{AppState, SessionId, KeyGeneratorList, Commitment, Payload, SignedCommitment, AsyncTask};
use radius_sdk::json_rpc::server::RpcParameter;
use tracing::info;

pub mod cluster;
pub use cluster::*;
pub mod external;
pub use external::*;
pub mod payload;
pub use payload::*;

mod primitives {
    pub use radius_sdk::json_rpc::server::{RpcParameter, RpcError};
}

/// Multicast encryption key acknowledgment
pub fn multicast_enc_key_ack<C: AppState>(
    ctx: &C,
    session_id: SessionId,
    commitment: SignedCommitment<C::Signature, C::Address>,
) -> Result<(), C::Error> {
    let committee_urls =
        KeyGeneratorList::<C::Address>::get()
            .map_err(|e| C::Error::from(e))?
            .into_iter()
            .filter(|kg| kg.address() != ctx.address())
            .map(|kg| kg.cluster_rpc_url().to_owned())
            .collect();
    let payload = serde_json::to_vec(&commitment)?;
    let commitment = Commitment::new(payload.into(), Some(ctx.address()), session_id);
    let signature = ctx.sign(&commitment)?;
    info!("Broadcasting enc key ack to {:?}", committee_urls);
    ctx.async_task().multicast(committee_urls, <SyncEncKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SyncEncKey(SignedCommitment { commitment, signature }));
    Ok(())
}

/// Multicast decryption key acknowledgment
pub fn multicast_dec_key_ack<C: AppState>(
    ctx: &C,
    payload: Payload,
    session_id: SessionId,
) -> Result<(), C::Error> {
    let committee_urls = KeyGeneratorList::<C::Address>::get()?.all_rpc_urls(true);
    info!("Broadcast decryption key - session_id: {:?}, all_dkg_list: {:?}", session_id, committee_urls);
    let commitment = Commitment::new(payload, Some(ctx.address()), session_id);
    let signature = ctx.sign(&commitment)?;
    ctx.async_task().multicast(committee_urls, <SyncDecKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SyncDecKey(SignedCommitment { commitment, signature }));
    Ok(())
}