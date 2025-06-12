use crate::*;
use dkg_primitives::{
    Config, SessionId, SignedCommitment, SubmitterList, EncKeyCommitment
};
use radius_sdk::kvstore::KvStoreError;
use serde::{Deserialize, Serialize};

/// Get commitments for all encryption keys at a given session id
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetFinalizedEncKeys {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response<Signature, Address> {
    pub commitments: Vec<SignedCommitment<Signature, Address>>,
}

impl<C: Config> RpcParameter<C> for GetFinalizedEncKeys {
    type Response = Response<C::Signature, C::Address>;

    fn method() -> &'static str {
        "get_finalized_partial_keys"
    }

    async fn handler(self, _context: C) -> RpcResult<Self::Response> {
        let session_id = self.session_id;
        let commitments = SubmitterList::<C::Address>::get(session_id)?
            .into_iter()
            .try_fold(Vec::new(), |mut acc, address| -> Result<Vec<EncKeyCommitment<C::Signature, C::Address>>, KvStoreError> {
                EncKeyCommitment::<C::Signature, C::Address>::get(&session_id, &address)
                    .map(|commitment| {
                        acc.push(commitment);
                        acc
                    })
            })?
            .into_iter()
            .map(|commitment| commitment.inner())
            .collect::<Vec<SignedCommitment<C::Signature, C::Address>>>();
        Ok(Response { commitments })
    }
}
