use super::{Config, RpcParameter};
use dkg_rpc::{RequestSubmitEncKey, SyncFinalizedEncKeys, FinalizedEncKeyPayload};
use dkg_primitives::{
    AsyncTask, Commitment, Event, SessionId, SignedCommitment, 
    EncKeyCommitment, AuthService, KeyGenerator
};
use dkg_utils::timestamp;
use std::{marker::PhantomData, sync::Arc, time::{Duration, Instant}};
use futures_timer::Delay;
use tokio::sync::Mutex;
use tracing::{info, error};
use tokio::sync::mpsc::Receiver;

pub struct SessionResult<Signature>(PhantomData<Signature>);

pub async fn run_session_worker<C>(ctx: &C, worker: &mut SessionWorker<C>, session_duration: Duration) -> Result<(), C::Error> 
where
    C: Config
{
    let mut sessions = Sessions::new(session_duration);
    loop {
        let session_info = sessions.next_session().await;
        let _ = worker.on_session(ctx, session_info).await;
    }
}

/// Calculate the duration in milliseconds until the next session starts from now 
fn time_until_next_session(session_duration: Duration) -> Duration {
    let now = timestamp();
    let session_duration_millis = session_duration.as_millis();
    let next_session = (now + session_duration_millis) / session_duration_millis;
    let remaining_millis = (next_session * session_duration_millis) - now;
    Duration::from_millis(remaining_millis as u64) 
}

/// Information about the session
pub struct SessionInfo {
    /// Current session number
    pub session_id: SessionId, 
    /// Duration of the session in milliseconds
    pub duration: Duration, 
    /// Instant when the session ends
    pub ends_at: Instant,
}

impl SessionInfo {
    pub fn new(session_id: SessionId, duration: Duration) -> Self {
        Self { session_id, duration, ends_at: Instant::now() + time_until_next_session(duration) }
    }
}

/// A stream that returns every time there is a new session 
pub struct Sessions {
    last_session: SessionId,
    session_duration: Duration,
    until_next_session: Option<Delay>,
}

impl Sessions {
    pub fn new(session_duration: Duration) -> Self {
        let session_id = SessionId::get().expect("Failed to get session id");
        // Just in case 
        assert!(session_id.is_initial(), "Session id is not initial");
        Self {
            last_session: session_id,
            session_duration,
            until_next_session: None, 
        }
    }
    
    /// Simple function that returns the next session info if any
    /// First time this function is called, it will wait until the next session starts and update `until_next_session` with the `Delay` at that time
    /// After that, 
    /// For example, if session length is 2 seconds and current time is 09:00:00, it will wait until 09:00:02 and `until_next_session` will be updated which will wait until 09:00:04
    /// Reason for this is to make sure all nodes start at the same time.
    /// Then, when `next_session` is called, all nodes will start session at the same time 
    pub async fn next_session(&mut self) -> SessionInfo {
        loop {
            // Wait for the next session
            self.until_next_session
                .take()
                .unwrap_or_else(|| {
                    // Delay for the first
                    let wait_dur = time_until_next_session(self.session_duration);
                    Delay::new(wait_dur)
                })
                .await;
            let wait_dur = time_until_next_session(self.session_duration);
            // Delay from `now` to the next session
            self.until_next_session = Some(Delay::new(wait_dur));
            if self.last_session.is_initial() {
                // We don't update the last session for the initial session
                break SessionInfo::new(self.last_session, self.session_duration);
            } else {
                let current_session = match SessionId::get() {
                    Ok(session_id) => session_id,
                    Err(_) => { tracing::error!("Error getting session id"); continue; }
                };
                // Current session should be greater than the last session
                if current_session > self.last_session {
                    self.last_session = current_session;
                    break SessionInfo::new(current_session, self.session_duration);
                }
            }   
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionWorkerState {
    /// The session is not started yet
    Init, 
    /// The session has started
    Start(SessionId),
    /// The session has ended
    End(SessionId),
}

#[derive(Clone)]
pub struct SessionWorker<C: Config> {
    solver_rpc_url: String,
    rx: Arc<Mutex<Receiver<Event<C::Signature, C::Address>>>>,
    state: SessionWorkerState,
    /// Key generators for the current session
    key_generators: Vec<KeyGenerator<C::Address>>,
}

impl<C: Config> SessionWorker<C> {

    pub fn new(ctx: &C, solver_rpc_url: String, rx: Receiver<Event<C::Signature, C::Address>>, key_generators: Vec<KeyGenerator<C::Address>>) -> Self {
        // on_before_session()
        let session_id = SessionId::get().expect("Not initialized"); 
        if !session_id.is_initial() { panic!("Session id is not initial"); }
        init(ctx, key_generators.clone(), session_id); 
        Self { solver_rpc_url, rx: Arc::new(Mutex::new(rx)), state: SessionWorkerState::Init, key_generators }
    }

    pub async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Option<SessionResult<C::Signature>> {
        let session_id = session_info.session_id;
        let mut key_generators = self.key_generators.clone();
        // Ends at after this delay 
        let mut timeout = Delay::new(session_info.ends_at.duration_since(Instant::now()));
        loop {
            tokio::select! {
                event = async {
                    let mut rx = self.rx.lock().await;
                    rx.recv().await
                } => {
                    if let Some(event) = event {
                        match event {
                            Event::FinalizeKey { commitments, current_session_id } => {
                                match self.state {
                                    SessionWorkerState::Init => {
                                        self.state = SessionWorkerState::Start(session_id);
                                    },
                                    SessionWorkerState::End(last_session_id) => {
                                        // It should be greater 
                                        if last_session_id > current_session_id {
                                            continue;
                                        }
                                    },
                                    _ => continue,
                                }
                                if let Err(err) =
                                    broadcast_finalized_enc_keys::<C>(&ctx, &mut key_generators, commitments, self.solver_rpc_url.clone(), session_id).await
                                {
                                    error!("Error during encryption key broadcasting: {:?}", err);
                                    return None;
                                }
                                continue;
                            },
                            Event::EndSession(mut end_session_id) => {
                                match self.state {
                                    SessionWorkerState::Start(current_session_id) => {
                                        // Should be the same session id
                                        if current_session_id != end_session_id {
                                            continue;
                                        }
                                        self.state = SessionWorkerState::End(end_session_id);
                                    }, 
                                    _ => continue,
                                }
                                // Update the session id 
                                if let Err(e) = end_session_id.next_mut() {
                                    error!("Error during session id increment: {:?}", e);
                                    return None;
                                }
                                if ctx.should_end_round((end_session_id + 1u64.into()).into()) {
                                    match ctx.current_round() {
                                        Ok(round) => {
                                            if let Ok(next_key_generators) = ctx.auth_service().get_key_generators(round.into()).await {
                                                self.key_generators = next_key_generators;
                                            }
                                        }
                                        Err(e) => {
                                            error!("Error getting current round: {:?}", e);
                                            return None;
                                        }
                                    }
                                }
                                return None;
                            }
                        }
                    }
                },
                _ = &mut timeout => {
                    info!("Timeout for session {:?}", session_info.session_id);
                    return None;
                }
            }
        }
    }
}

/// Request submit encryption key for the initial session
pub fn init<C: Config>(
    ctx: &C,
    key_generators: Vec<KeyGenerator<C::Address>>,
    session_id: SessionId,
) {
    if !ctx.is_leader() { return; }
    let urls = key_generators.iter().map(|kg| kg.cluster_rpc_url().to_string()).collect::<Vec<_>>();
    ctx.async_task().multicast(urls, <RequestSubmitEncKey as RpcParameter<C>>::method().to_string(), RequestSubmitEncKey { session_id });
}

/// Broadcast finalized encryption keys to the key generators including the solver
pub async fn broadcast_finalized_enc_keys<C: Config>(
    ctx: &C,
    key_generators: &mut Vec<KeyGenerator<C::Address>>,
    commitments: Vec<EncKeyCommitment<C::Signature, C::Address>>,
    solver_url: String,
    session_id: SessionId,
) -> Result<(), C::Error> {
    if !ctx.is_leader() { return Ok(()); }
    let payload = FinalizedEncKeyPayload::<C::Signature, C::Address>::new(commitments);
    let bytes = serde_json::to_vec(&payload).map_err(|e| C::Error::from(e))?;
    let commitment = Commitment::new(bytes.into(), Some(ctx.address()), session_id);
    let signature = ctx.sign(&commitment)?;
    let mut urls = key_generators.iter().map(|kg| kg.cluster_rpc_url().to_string()).collect::<Vec<_>>();
    urls.push(solver_url);
    info!("Broadcasting finalized encryption keys to {:?}", urls);
    ctx.async_task().multicast(urls, <SyncFinalizedEncKeys<C::Signature, C::Address> as RpcParameter<C>>::method().to_string(), SignedCommitment { signature, commitment });
    Ok(())
}

// pub async fn wait_for_decryption_key<C: Config>(
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
