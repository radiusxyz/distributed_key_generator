use std::time::{SystemTime, UNIX_EPOCH};

use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcError, RpcParameter},
    },
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{PartialKey as SkdePartialKey, PartialKeyProof};
use tracing::info;

use crate::rpc::{common::generate_dummy_signature, prelude::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AckPartialKey {
    pub signature: Signature,
    pub message: AckPartialKeyMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AckPartialKeyMessage {
    pub session_id: SessionId,
    pub recipient: Address,
    pub key_id: KeyId,
    pub partial_key: SkdePartialKey,
    pub proof: PartialKeyProof,
    pub index: usize,
    pub original_timestamp: u64,
    pub ack_timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AckPartialKeyResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for AckPartialKey {
    type Response = AckPartialKeyResponse;

    fn method() -> &'static str {
        "ack_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        // let sender_address = verify_signature(&self.signature, &self.message)?;

        info!(
            "Received partial key ACK - session_id: {}, key_id: {:?}, recipient: {}, index: {}, timestamp: {}",
            self.message.session_id,
            self.message.key_id,
            self.message.recipient.as_hex_string(),
            self.message.index,
            self.message.ack_timestamp
        );

        // 리더 검증 (리더만 ACK 가능)
        let my_address = context.config().signer().address();
        if &self.message.recipient != my_address {
            // 내게 온 ACK가 아니면 무시
            return Ok(AckPartialKeyResponse { success: true });
        }

        // TODO: 부분 키 인덱스 저장 및 추가 처리
        // (실제 구현에서는 인덱스 정보를 저장할 구조체가 필요)

        Ok(AckPartialKeyResponse { success: true })
    }
}

// 리더가 부분 키 승인을 전체 네트워크에 브로드캐스트
pub fn broadcast_partial_key_ack(
    session_id: SessionId,
    recipient: Address,
    key_id: KeyId,
    partial_key: SkdePartialKey,
    proof: PartialKeyProof,
    original_timestamp: u64,
    index: usize,
    context: &AppState,
) -> Result<(), Error> {
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    let ack_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let message = AckPartialKeyMessage {
        session_id,
        recipient,
        key_id,
        partial_key,
        proof,
        index,
        original_timestamp,
        ack_timestamp,
    };

    // TODO: Add to make actual signature
    let signature = generate_dummy_signature(&serialize_to_bincode(&message).unwrap());

    let parameter = AckPartialKey { signature, message };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    all_key_generator_rpc_url_list,
                    AckPartialKey::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
