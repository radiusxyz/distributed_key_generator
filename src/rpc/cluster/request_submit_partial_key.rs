use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcError, RpcParameter},
    },
    signature::Address,
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{PartialKey as SkdePartialKey, PartialKeyProof};
use tracing::info;

use super::{PartialKeyPayload, SubmitPartialKey};
use crate::{
    error::KeyGenerationError,
    rpc::prelude::*,
    task::partial_key_manager::PartialKeyManager,
    utils::{get_current_timestamp, AddressExt},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestSubmitPartialKey {
    pub session_id: SessionId,
}

// 각 committee 멤버가 자신의 미리 생성된 partial key를 리더에게 제출하는 RPC
impl RpcParameter<AppState> for RequestSubmitPartialKey {
    type Response = Option<()>;

    fn method() -> &'static str {
        "request_submit_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params();
        let my_address = context.config().address().clone();

        info!(
            "[{}] Requesting to submit partial key for session {}",
            context.config().address().to_short(),
            self.session_id.as_u64()
        );

        let manager = PartialKeyManager::global();

        match manager
            .get_fresh_partial_key_for_session(self.session_id, &my_address)
            .await
        {
            Ok(Some((partial_key, proof))) => {
                submit_partial_key_to_leader(
                    my_address,
                    self.session_id,
                    partial_key.into_inner(),
                    proof,
                    context.clone(),
                )
                .await?;

                info!(
                    "[{}] Submitted precomputed partial key to leader for session {}",
                    context.config().address().to_short(),
                    self.session_id.as_u64()
                );
            }
            Ok(None) => {
                info!(
                    "[{}] No precomputed key available, generating new one for session {}",
                    context.config().address().to_short(),
                    self.session_id.as_u64()
                );

                let (secret_value, partial_key) =
                    skde::key_generation::generate_partial_key(skde_params).unwrap();
                let proof =
                    skde::key_generation::prove_partial_key_validity(skde_params, &secret_value)
                        .unwrap();

                submit_partial_key_to_leader(
                    my_address,
                    self.session_id,
                    partial_key,
                    proof,
                    context.clone(),
                )
                .await?;
            }
            Err(e) => {
                return Err(RpcError::from(KeyGenerationError::InternalError(format!(
                    "PartialKeyManager error: {}",
                    e
                ))));
            }
        }

        Ok(Some(()))
    }
}

async fn submit_partial_key_to_leader(
    sender: Address,
    session_id: SessionId,
    partial_key: SkdePartialKey,
    proof: PartialKeyProof,
    context: AppState,
) -> Result<(), RpcError> {
    let leader_rpc_url = if let Some(url) = context.config().leader_cluster_rpc_url() {
        url.clone()
    } else {
        return Err(RpcError::from(KeyGenerationError::InternalError(
            "Leader RPC URL not found".to_string(),
        )));
    };

    // PartialKeyPayload 생성
    let payload = PartialKeyPayload {
        sender: sender.clone(),
        partial_key,
        proof,
        submit_timestamp: get_current_timestamp(),
        session_id,
    };

    // 서명 생성
    let signature = crate::rpc::common::create_signature(&payload);

    let parameter = SubmitPartialKey { signature, payload };

    // 리더에게 제출
    let rpc_client = RpcClient::new()?;

    // 명시적으로 타입을 지정하여 never type fallback 방지
    let _: () = rpc_client
        .request::<SubmitPartialKey, ()>(
            &leader_rpc_url,
            SubmitPartialKey::method(),
            parameter,
            Id::Null,
        )
        .await?;

    Ok(())
}
