use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcError, RpcParameter},
    },
    signature::Address,
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{
    generate_partial_key, prove_partial_key_validity, PartialKey, PartialKeyProof,
};

use crate::rpc::{cluster::SyncPartialKey, prelude::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunGeneratePartialKey {
    pub session_id: SessionId,
}

// TODO: Getting partial key from partial key storage
// i.e. precomputed partial key list is required for each committee member
impl RpcParameter<AppState> for RunGeneratePartialKey {
    type Response = ();

    fn method() -> &'static str {
        "run_generate_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params();

        let (secret_value, partial_key) = generate_partial_key(skde_params).unwrap();

        let partial_key_proof = prove_partial_key_validity(skde_params, &secret_value).unwrap();

        sync_partial_key(
            context.config().signer().address().clone(),
            self.session_id,
            partial_key,
            partial_key_proof,
        );

        Ok(())
    }
}

pub fn sync_partial_key(
    address: Address,
    session_id: SessionId,
    partial_key: PartialKey,
    partial_key_proof: PartialKeyProof,
) {
    let all_key_generator_rpc_url_list = KeyGeneratorList::get()
        .unwrap()
        .get_all_key_generator_rpc_url_list();

    tokio::spawn(async move {
        let parameter = SyncPartialKey {
            address,
            session_id,
            skde_partial_key: partial_key,
            partial_key_proof,
        };

        let rpc_client = RpcClient::new().unwrap();
        rpc_client
            .multicast(
                all_key_generator_rpc_url_list,
                SyncPartialKey::method(),
                &parameter,
                Id::Null,
            )
            .await
            .unwrap();
    });
}
