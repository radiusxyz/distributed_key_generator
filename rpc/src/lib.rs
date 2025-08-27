pub use std::sync::Arc;

pub use dkg_primitives::{
    to_signed_commitment, ActiveOperatorList, AsyncTask, Commitment, Config, DbManager,
    EncKeyCommitment, Payload, SessionId, SignedCommitment,
};
pub use radius_sdk::json_rpc::server::{RpcError, RpcParameter};
pub use tracing::info;

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
    pub fn submit_enc_key<C: Config>(
        ctx: C,
        session_id: SessionId,
        enc_key: Vec<u8>,
    ) -> RpcResult<()> {
        let leader = ctx
            .current_leader(session_id, false)
            .map_err(|e| RpcError::from(e))?;
        info!("Submitting enc key to leader: {:?}", leader.1);
        let commitment = Commitment::new(enc_key.into(), Some(ctx.address()), session_id);
        let signature = ctx.sign(&commitment)?;
        ctx.async_task().multicast(
            vec![leader.1],
            <SubmitEncKey<C::Signature, C::Address> as RpcParameter<C>>::method().into(),
            SubmitEncKey(SignedCommitment {
                commitment,
                signature,
            }),
        );
        Ok(())
    }

    /// Helper function to multicast encryption key acknowledgment
    pub fn multicast_enc_key_ack<C: Config>(
        ctx: C,
        session_id: SessionId,
        commitment: EncKeyCommitment<C::Signature, C::Address>,
    ) -> RpcResult<()> {
        let operators = ctx
            .db_manager()
            .get_active_operator_list()
            .map_err(|e| RpcError::from(e))?
            .into_iter()
            .filter(|kg| kg.address() != ctx.address()) // Exclude self
            .map(|kg| kg.cluster_rpc_url().to_owned())
            .collect::<Vec<_>>();
        let commitment = to_signed_commitment(ctx.clone(), session_id, commitment)?;
        if !operators.is_empty() {
            info!(
                "Broadcasting enc key ack to {:?} at session: {:?}",
                operators, session_id
            );
            ctx.async_task().multicast(
                operators,
                <SyncEncKey<C::Signature, C::Address> as RpcParameter<C>>::method().into(),
                SyncEncKey(commitment),
            );
        }
        Ok(())
    }

    /// Helper function to multicast decryption key acknowledgment
    pub fn multicast_dec_key_ack<C: Config>(
        ctx: &C,
        payload: Payload,
        session_id: SessionId,
        exclude_list: Vec<C::Address>,
    ) -> RpcResult<()> {
        let operators = ctx
            .db_manager()
            .get_active_operator_list()
            .map_err(|e| RpcError::from(e))?
            .into_iter()
            .filter(|kg| !exclude_list.contains(&kg.address()))
            .map(|kg| kg.cluster_rpc_url().to_owned())
            .collect::<Vec<_>>();
        if !operators.is_empty() {
            info!(
                "Broadcasting dec key to {:?} at session: {:?}",
                operators, session_id
            );
            let commitment = Commitment::new(payload, Some(ctx.address()), session_id);
            let signature = ctx.sign(&commitment)?;
            let randomness = ctx.randomness(session_id);
            ctx.async_task().multicast(
                operators,
                <SyncDecKey<C::Signature, C::Address> as RpcParameter<C>>::method().into(),
                SyncDecKey {
                    signed_commitment: SignedCommitment {
                        commitment,
                        signature,
                    },
                    randomness,
                },
            );
        }
        Ok(())
    }
}
