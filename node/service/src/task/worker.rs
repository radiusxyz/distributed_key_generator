use super::{AppState, Error, RpcParameter};
use dkg_rpc::{RequestSubmitPartialKey, SyncFinalizedPartialKeys};
use dkg_primitives::{SessionId, PartialKeyAddressList, KeyGeneratorList, SyncFinalizedPartialKeysPayload, PartialKeySubmission};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, warn};

pub async fn run_dkg_worker<C: AppState>(context: &C, worker: DkgWorker) -> Result<(), Error> {
    PartialKeyAddressList::<C::Address>::initialize(0u64.into()).map_err(|e| Error::from(e))?;
    loop {
        sleep(Duration::from_millis(worker.session_cycle)).await;
        worker.run(context).await?;
    }
}

pub struct DkgWorker {
    solver_rpc_url: String,
    session_cycle: u64,
    threshold: u16,
}

impl DkgWorker {

    pub fn new(solver_rpc_url: String, session_cycle: u64, threshold: u16) -> Self {
        Self { solver_rpc_url, session_cycle, threshold }
    }

    pub async fn run<C: AppState>(&self, context: &C) -> Result<(), Error> {
        let mut session_id = SessionId::get_mut().map_err(|e| Error::from(e))?;
        let key_generator_rpc_url_list = KeyGeneratorList::<C::Address>::get()
            .map_err(|e| Error::from(e))?
            .all_rpc_urls();

        if key_generator_rpc_url_list.is_empty() {
            warn!("No single key generator has been registered! Skipping...");
            return Ok(());
        }

        let partial_key_address_list = PartialKeyAddressList::<C::Address>::get_or(
            *session_id,
            || PartialKeyAddressList::new(),
        )
        .map_err(|e| Error::from(e))?;

        if partial_key_address_list.is_empty() {
            request_submit_partial_key(context, key_generator_rpc_url_list, *session_id);
            return Ok(());
        } else {
            // TODO: needs wait to collect partial keys, instead of loop
            let list = loop {
                if let Ok(list) = PartialKeyAddressList::<C::Address>::get(*session_id) {
                    let current_count = list.len();
                    if current_count >= (self.threshold as usize) {
                        break list;
                    }
                } 
                sleep(Duration::from_millis(100)).await;
            };
            let partial_key_submissions = list.get_partial_key_list::<C>(*session_id).map_err(|e| Error::from(e))?;
            if let Err(err) =
                broadcast_finalized_partial_keys::<C>(&context, partial_key_submissions, self.solver_rpc_url.clone(), *session_id).await
            {
                error!("Error during partial key broadcasting: {:?}", err);
                return Ok(());
            }
        }

        session_id.next_mut().map_err(|e| Error::from(e))?;
        PartialKeyAddressList::<C::Address>::initialize(session_id.clone()).map_err(|e| Error::from(e))?;
        session_id.update().map_err(|e| Error::from(e))?;
        Ok(())
    }
}

pub fn request_submit_partial_key<C: AppState>(
    context: &C,
    key_generator_rpc_url_list: Vec<String>,
    session_id: SessionId,
) {
    let parameter = RequestSubmitPartialKey { session_id };
    context.multicast(key_generator_rpc_url_list, <RequestSubmitPartialKey as RpcParameter<C>>::method().to_string(), parameter);
}

pub async fn broadcast_finalized_partial_keys<C: AppState>(
    ctx: &C,
    partial_keys: Vec<PartialKeySubmission<C::Signature, C::Address>>,
    solver_url: String,
    session_id: SessionId,
) -> Result<(), C::Error> {
    let payload = SyncFinalizedPartialKeysPayload::<C::Signature, C::Address>::new(
        ctx.address().clone(),
        partial_keys,
        session_id,
    );
    let signature = ctx.sign(&payload)?;
    let message = SyncFinalizedPartialKeys { signature, payload };
    let mut peers = KeyGeneratorList::<C::Address>::get()?.all_rpc_urls();
    peers.push(solver_url);
    ctx.multicast(peers, <SyncFinalizedPartialKeys<C::Signature, C::Address> as RpcParameter<C>>::method().to_string(), message);
    Ok(())
}

// pub async fn wait_for_decryption_key<C: AppState>(
//     ctx: &C,
//     session_id: SessionId,
//     timeout_secs: u64,
// ) -> Result<DecryptionKey, C::Error> {
//     let poll_interval = Duration::from_secs(1);
//     let mut waited = 0;
//     loop {
//         match DecryptionKey::get(session_id) {
//             Ok(key) => {
//                 info!("{} Received decryption key on session {:?}", ctx.log_prefix(), session_id);
//                 return Ok(key);
//             }
//             Err(_) => {
//                 if waited >= timeout_secs {
//                     error!("{} Timeout waiting for decryption key on session {:?}", ctx.log_prefix(), session_id);
//                     return Err(C::Error::from(RpcClientError::Response(format!(
//                         "Solver did not submit decryption key for session {:?} in time",
//                         session_id
//                     ))));
//                 }

//                 debug!(
//                     "{} Still waiting for decryption key on session {:?} (waited: {}s)",
//                     ctx.log_prefix(), session_id, waited
//                 );

//                 sleep(poll_interval).await;
//                 waited += 1;
//             }
//         }
//     }
// }
