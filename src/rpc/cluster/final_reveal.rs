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
pub struct PartialKeyEntry {
    pub index: usize,
    pub address: Address,
    pub partial_key: SkdePartialKey,
    pub proof: PartialKeyProof,
    pub timestamp: u64,
    pub signature: Signature,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SolverSubmission {
    pub timestamp: u64,
    pub signature: Signature,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecryptionAck {
    pub ack_timestamp: u64,
    pub signature: Signature,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalReveal {
    pub signature: Signature,
    pub message: FinalRevealMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalRevealMessage {
    pub session_id: SessionId,
    pub key_id: KeyId,
    pub decrypted_payload: Option<String>, // 실제 복호화된 페이로드 (선택사항)
    pub decryption_key: String,
    pub partial_keys: Vec<PartialKeyEntry>,
    pub solver_submission: SolverSubmission,
    pub decryption_ack: DecryptionAck,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalRevealResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for FinalReveal {
    type Response = FinalRevealResponse;

    fn method() -> &'static str {
        "final_reveal"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        info!(
            "Received final reveal - session_id: {}, key_id: {:?}, partial_keys: {}",
            self.message.session_id,
            self.message.key_id,
            self.message.partial_keys.len()
        );

        // 최종 공개 정보를 저장하는 로직
        // 실제 구현에서는 검증자가 후속 확인을 위해 이 데이터를 저장

        // TODO: 검증자 로직 추가 (지금은 단순 로깅만)
        // 실제 검증자는 이 데이터를 사용하여 리더 결정의 공정성을 확인할 수 있음

        Ok(FinalRevealResponse { success: true })
    }
}

// 리더가 최종 공개 정보를 브로드캐스트
pub fn broadcast_final_reveal(
    session_id: SessionId,
    key_id: KeyId,
    decryption_key: String,
    partial_keys: Vec<PartialKeyEntry>,
    solver_submission: SolverSubmission,
    decryption_ack: DecryptionAck,
    decrypted_payload: Option<String>,
    context: &AppState,
) -> Result<(), Error> {
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    let message = FinalRevealMessage {
        session_id,
        key_id,
        decryption_key,
        partial_keys,
        solver_submission,
        decryption_ack,
        decrypted_payload,
    };

    // TODO: Add to make actual signature
    let signature = generate_dummy_signature(&serialize_to_bincode(&message).unwrap());

    let parameter = FinalReveal { signature, message };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    all_key_generator_rpc_url_list,
                    FinalReveal::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
