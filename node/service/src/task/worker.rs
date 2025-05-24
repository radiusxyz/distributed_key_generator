use super::{AppState, Error, RpcParameter};
use dkg_rpc::{RequestSubmitEncKey, SyncFinalizedEncKeys, FinalizedEncKeyPayload};
use dkg_primitives::{
    AsyncTask, Commitment, Event, SessionId, SignedCommitment, SubmitterList, 
    EncKeyCommitment, KeyGeneratorList
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use tokio::sync::mpsc::Receiver;

pub async fn run_dkg_worker<C: AppState>(context: &C, worker: &mut DkgWorker<C>) -> Result<(), Error> {
    SubmitterList::<C::Address>::initialize(0u64.into()).map_err(|e| Error::from(e))?;
    info!("Init DKG worker");
    loop {
        // TODO: loop { future::select!(worker.run(context), timer) }
        worker.run(context).await?;
        // TODO: Timer
        sleep(Duration::from_millis(worker.session_cycle)).await;
    }
}

pub struct DkgWorker<C: AppState> {
    solver_rpc_url: String,
    session_cycle: u64,
    rx: Receiver<Event<C::Signature, C::Address>>,
}

impl<C: AppState> DkgWorker<C> {

    pub fn new(solver_rpc_url: String, session_cycle: u64, rx: Receiver<Event<C::Signature, C::Address>>) -> Self {
        Self { solver_rpc_url, session_cycle, rx }
    }

    pub async fn run(&mut self, context: &C) -> Result<(), Error> {
        let mut session_id = SessionId::get_mut().map_err(|e| Error::from(e))?;
        let submitter_list = SubmitterList::<C::Address>::get(*session_id).map_err(|e| Error::from(e))?;
        info!("Partial key address list at {:?}: {:?}", *session_id, submitter_list);
        let has_submit = !submitter_list.is_empty();
        let is_sync = if has_submit { true } else { false };
        let mut committee_urls = KeyGeneratorList::<C::Address>::get()
            .map_err(|e| Error::from(e))?
            .all_rpc_urls(is_sync);
        info!("Committee URLs: {:?}", committee_urls);
        if committee_urls.is_empty() {
            warn!("No single key generator has been registered! Skipping...");
            return Ok(());
        }
        if !has_submit {
            info!("Rrequesting partial keys from committee");
            // 0.5s timeout
            request_submit_partial_key(context, committee_urls, *session_id);
            return Ok(());
        } else {
            if let Some(event) = self.rx.recv().await {
                match event {
                    Event::ThresholdMet(list) => {
                        if let Err(err) =
                            broadcast_finalized_partial_keys::<C>(&context, &mut committee_urls, list, self.solver_rpc_url.clone(), *session_id).await
                        {
                            error!("Error during partial key broadcasting: {:?}", err);
                            return Ok(());
                        }
                    }
                }
            }
        }

        session_id.next_mut().map_err(|e| Error::from(e))?;
        SubmitterList::<C::Address>::initialize(session_id.clone()).map_err(|e| Error::from(e))?;
        session_id.update().map_err(|e| Error::from(e))?;
        Ok(())
    }
}

pub fn request_submit_partial_key<C: AppState>(
    context: &C,
    committee_urls: Vec<String>,
    session_id: SessionId,
) {
    let parameter = RequestSubmitEncKey { session_id };
    context.async_task().multicast(committee_urls, <RequestSubmitEncKey as RpcParameter<C>>::method().to_string(), parameter);
}

pub async fn broadcast_finalized_partial_keys<C: AppState>(
    ctx: &C,
    committee_urls: &mut Vec<String>,
    commitments: Vec<EncKeyCommitment<C::Signature, C::Address>>,
    solver_url: String,
    session_id: SessionId,
) -> Result<(), C::Error> {
    let payload = FinalizedEncKeyPayload::<C::Signature, C::Address>::new(commitments);
    let bytes = serde_json::to_vec(&payload).map_err(|e| C::Error::from(e))?;
    let commitment = Commitment::new(bytes.into(), Some(ctx.address()), session_id);
    let signature = ctx.sign(&commitment)?;
    committee_urls.push(solver_url);
    info!("Broadcasting finalized partial keys to {:?}", committee_urls);
    ctx.async_task().multicast(committee_urls.clone(), <SyncFinalizedEncKeys<C::Signature, C::Address> as RpcParameter<C>>::method().to_string(), SignedCommitment { signature, commitment });
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
