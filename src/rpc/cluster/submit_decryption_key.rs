use std::time::{SystemTime, UNIX_EPOCH};

use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::rpc::{
    common::{generate_dummy_signature, verify_signature},
    prelude::*,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecryptionKey {
    pub signature: Signature,
    pub message: SubmitDecryptionKeyMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecryptionKeyMessage {
    pub session_id: SessionId,
    pub key_id: KeyId,
    pub decryption_key: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecryptionKeyResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for SubmitDecryptionKey {
    type Response = SubmitDecryptionKeyResponse;

    fn method() -> &'static str {
        "submit_decryption_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        // TODO: Add to make actual signature
        let sender_address = verify_signature(&self.signature, &self.message)?;

        info!(
            "Received decryption key - session_id: {}, key_id: {:?}, sender: {}, timestamp: {}",
            self.message.session_id,
            self.message.key_id,
            sender_address.as_hex_string(),
            self.message.timestamp
        );

        // 검증 로직 - 이 예제에서는 Solver가 클러스터 내에 등록된 노드인지만 확인
        // if !KeyGeneratorList::get()?.is_key_generator_in_cluster(&sender_address) {
        //     return Err(RpcError::InvalidParams(format!(
        //         "Address {} is not a registered key generator",
        //         sender_address.as_hex_string()
        //     )));
        // }

        // 복호화 키 저장
        let decryption_key = DecryptionKey::new(self.message.decryption_key.clone());
        decryption_key.put(self.message.key_id)?;

        // TODO: 이 지점에서 ack_decryption_key를 통해 승인 메시지 브로드캐스트 로직 추가
        // 여기서는 직접 구현하지 않고, 별도 함수로 분리 예정

        Ok(SubmitDecryptionKeyResponse { success: true })
    }
}
