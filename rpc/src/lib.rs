pub use dkg_primitives::{Config, SessionId, KeyGeneratorList, Commitment, Payload, SignedCommitment, AsyncTask, EncKeyCommitment};
pub use tracing::info;
pub use radius_sdk::json_rpc::server::{RpcParameter, RpcError};

pub mod cluster;
pub use cluster::*;
pub mod external;
pub use external::*;
pub mod payload;
pub use payload::*;

pub type RpcResult<T> = Result<T, RpcError>;

pub use helper::*;
pub mod helper {
    use super::*;
    
    /// Helper function to submit encryption key 
    pub async fn submit_enc_key<C: Config>(
        session_id: SessionId,
        enc_key: Vec<u8>,
        ctx: &C,
    ) -> RpcResult<()> {
        let leader = ctx.current_leader(false).map_err(|e| RpcError::from(e))?;
        let commitment = Commitment::new(enc_key.into(), Some(ctx.address()), session_id);
        let signature = ctx.sign(&commitment)?;
        ctx.async_task().multicast(vec![leader.1], <SubmitEncKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SubmitEncKey(SignedCommitment { commitment, signature }));
        Ok(())
    }

    /// Helper function to multicast encryption key acknowledgment
    pub fn multicast_enc_key_ack<C: Config>(
        ctx: &C,
        session_id: SessionId,
        commitment: EncKeyCommitment<C::Signature, C::Address>,
    ) -> RpcResult<()> {
        let current_round = ctx.current_round().map_err(|e| C::Error::from(e))?;
        let key_generators =
            KeyGeneratorList::<C::Address>::get(current_round)
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
    pub fn multicast_dec_key_ack<C: Config>(
        ctx: &C,
        payload: Payload,
        session_id: SessionId,
    ) -> RpcResult<()> {
        let current_round = ctx.current_round().map_err(|e| C::Error::from(e))?;
        let key_generators = KeyGeneratorList::<C::Address>::get(current_round)?.all_rpc_urls(true);
        info!("Broadcast decryption key - session_id: {:?}, all_dkg_list: {:?}", session_id, key_generators);
        let commitment = Commitment::new(payload, Some(ctx.address()), session_id);
        let signature = ctx.sign(&commitment)?;
        ctx.async_task().multicast(key_generators, <SyncDecKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SyncDecKey(SignedCommitment { commitment, signature }));
        Ok(())
    } 
}