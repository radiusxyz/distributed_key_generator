pub use dkg_primitives::{AppState, SessionId, KeyGeneratorList, Commitment, Payload, SignedCommitment, AsyncTask, EncKeyCommitment};
pub use tracing::info;
pub use radius_sdk::json_rpc::server::{RpcParameter, RpcError};

pub mod cluster;
pub use cluster::*;
pub mod external;
pub use external::*;
pub mod payload;
pub use payload::*;

pub use helper::*;
pub mod helper {
    use super::*;
    
    /// Helper function to submit encryption key 
    pub async fn submit_enc_key<C: AppState>(
        session_id: SessionId,
        enc_key: Vec<u8>,
        ctx: &C,
    ) -> Result<(), RpcError> {
        if let Some(leader_rpc_url) = ctx.leader_rpc_url() {
            let commitment = Commitment::new(enc_key.into(), Some(ctx.address()), session_id);
            let signature = ctx.sign(&commitment)?;
            ctx.async_task().multicast(vec![leader_rpc_url], <SubmitEncKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SubmitEncKey(SignedCommitment { commitment, signature }));
            return Ok(());
        }
        Ok(())
    }

    /// Helper function to multicast encryption key acknowledgment
    pub fn multicast_enc_key_ack<C: AppState>(
        ctx: &C,
        session_id: SessionId,
        commitment: EncKeyCommitment<C::Signature, C::Address>,
    ) -> Result<(), C::Error> {
        let key_generators =
            KeyGeneratorList::<C::Address>::get()
                .map_err(|e| C::Error::from(e))?
                .into_iter()
                .filter(|kg| kg.address() != ctx.address())
                .map(|kg| kg.cluster_rpc_url().to_owned())
                .collect();
        let payload = serde_json::to_vec(&commitment)?;
        let commitment = Commitment::new(payload.into(), Some(ctx.address()), session_id);
        let signature = ctx.sign(&commitment)?;
        info!("Broadcasting enc key ack to {:?}", key_generators);
        ctx.async_task().multicast(key_generators, <SyncEncKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SyncEncKey(SignedCommitment { commitment, signature }));
        Ok(())
    }

    /// Helper function to multicast decryption key acknowledgment
    pub fn multicast_dec_key_ack<C: AppState>(
        ctx: &C,
        payload: Payload,
        session_id: SessionId,
    ) -> Result<(), C::Error> {
        let key_generators = KeyGeneratorList::<C::Address>::get()?.all_rpc_urls(true);
        info!("Broadcast decryption key - session_id: {:?}, all_dkg_list: {:?}", session_id, key_generators);
        let commitment = Commitment::new(payload, Some(ctx.address()), session_id);
        let signature = ctx.sign(&commitment)?;
        ctx.async_task().multicast(key_generators, <SyncDecKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SyncDecKey(SignedCommitment { commitment, signature }));
        Ok(())
    } 
}