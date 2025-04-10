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
use tracing::info;

use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AckDecryptionKey {
    pub signature: Signature,
    pub message: AckDecryptionKeyMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AckDecryptionKeyMessage {
    pub session_id: SessionId,
    pub key_id: KeyId,
    pub decryption_key: String,
    pub original_timestamp: u64,
    pub ack_timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AckDecryptionKeyResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for AckDecryptionKey {
    type Response = AckDecryptionKeyResponse;

    fn method() -> &'static str {
        "ack_decryption_key"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        // let sender_address = verify_signature(&self.signature, &self.message, &_context)?;

        info!(
            "Received decryption key ACK - session_id: {}, key_id: {:?}, timestamps: {}/{}",
            self.message.session_id,
            self.message.key_id,
            self.message.original_timestamp,
            self.message.ack_timestamp
        );

        // 복호화 키 유효성 확인하는 로직은 생략 (실제 구현에서는 필요)
        // 여기서는 승인 정보를 받았음을 기록만 함

        // TODO: 검증자 로그 정보 저장 로직 추가
        // (실제 구현에서 검증자가 리더 행동을 검증하기 위해 필요)

        Ok(AckDecryptionKeyResponse { success: true })
    }
}

// 리더가 복호화 키 승인을 네트워크에 브로드캐스트
pub fn broadcast_decryption_key_ack(
    session_id: SessionId,
    key_id: KeyId,
    decryption_key: String,
    original_timestamp: u64,
    context: &AppState,
) -> Result<(), Error> {
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    let ack_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let message = AckDecryptionKeyMessage {
        session_id,
        key_id,
        decryption_key,
        original_timestamp,
        ack_timestamp,
    };

    let signature = context
        .config()
        .signer()
        .sign_message(&serialize_to_bincode(&message).unwrap())
        .unwrap();

    let parameter = AckDecryptionKey { signature, message };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    all_key_generator_rpc_url_list,
                    AckDecryptionKey::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
