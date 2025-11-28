use dkg_primitives::{Config, DecKey, EncKey, KeyGenerator, SignedCommitment};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{DecKeyPayload, *};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Handler for syncing the newly generated decryption key and store it in the local kvstore
pub struct SyncDecKey<Signature, Address> {
    pub signed_commitment: SignedCommitment<Signature, Address>,
    pub randomness: Vec<u8>,
}

impl<Signature, Address> SyncDecKey<Signature, Address> {
    fn payload(&self) -> Result<DecKeyPayload, RpcError> {
        self.signed_commitment
            .commitment
            .payload
            .decode::<DecKeyPayload>()
            .map_err(|e| RpcError::from(e))
    }
}

impl<C: Config> RpcParameter<C> for SyncDecKey<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_dec_key"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        if let Some(sender) = self.signed_commitment.sender() {
            let session_id = self.signed_commitment.session_id();
            info!(
                "method::{:?} at session {:?}",
                <Self as RpcParameter<C>>::method(),
                session_id
            );
            let _ = ctx.verify_signature(
                &self.signed_commitment.signature,
                &self.signed_commitment.commitment,
                Some(sender),
            )?;
            let DecKeyPayload {
                enc_key, dec_key, ..
            } = self.payload()?;
            let enc_key = EncKey::new(enc_key);
            let dec_key = DecKey::new(dec_key);
            ctx.key_generator()
                .read()
                .await
                .verify_dec_key(&enc_key.inner(), &dec_key.inner())?;
            dec_key.put(session_id)?;
        }

        Ok(())
    }
}
