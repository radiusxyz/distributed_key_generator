use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{PartialKey as SkdePartialKey, PartialKeyProof};
use tracing::info;

use crate::{
    rpc::{common::verify_signature, prelude::*},
    types::{KeyId, SessionId},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKey {
    pub signature: Signature,
    pub message: SubmitPartialKeyMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKeyMessage {
    pub session_id: crate::types::SessionId,
    pub key_id: KeyId,
    pub partial_key: SkdePartialKey,
    pub proof: PartialKeyProof,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKeyResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for SubmitPartialKey {
    type Response = SubmitPartialKeyResponse;

    fn method() -> &'static str {
        "submit_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        // TODO: Add to verify actual signature
        let sender_address = verify_signature(&self.signature, &self.message)?;

        info!(
            "Received partial key - session_id: {}, key_id: {:?}, sender: {}, timestamp: {}",
            self.message.session_id,
            self.message.key_id,
            sender_address.as_hex_string(),
            self.message.timestamp
        );

        // 클러스터 내 키 생성기 확인
        // if !KeyGeneratorList::get()?.is_key_generator_in_cluster(&sender_address) {
        //     return Err(RpcError {
        //         code: -32603,
        //         message: format!(
        //             "Address {} is not a registered key generator",
        //             sender_address.as_hex_string()
        //         )
        //         .into(),
        //         data: None,
        //     });
        // }

        // 부분 키 유효성 검증
        let is_valid = skde::key_generation::verify_partial_key_validity(
            context.skde_params(),
            self.message.partial_key.clone(),
            self.message.proof,
        );

        // if !is_valid {
        //     return Err(RpcError {
        //         code: -32603,
        //         message: "Invalid partial key".into(),
        //         data: None,
        //     });
        // }

        // 키 ID에 대한 부분 키 주소 목록 초기화
        PartialKeyAddressList::initialize(self.message.key_id)?;

        // 부분 키 주소 목록에 발신자 주소 추가
        PartialKeyAddressList::apply(self.message.key_id, |list| {
            list.insert(sender_address.clone());
        })?;

        // 부분 키 저장
        let partial_key = PartialKey::new(self.message.partial_key.clone());
        partial_key.put(self.message.key_id, &sender_address)?;

        // TODO: ACK 메시지 생성 및 브로드캐스트 (차후 ack_partial_key 메서드에서 구현)

        Ok(SubmitPartialKeyResponse { success: true })
    }
}
