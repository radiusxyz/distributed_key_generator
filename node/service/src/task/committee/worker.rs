use super::{Config, sync_finalized_enc_keys};
use crate::{SessionWorkerState, SessionWorker, SessionInfo, SessionResult, AuthService};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, Mutex};
use futures_timer::Delay;
use std::time::Instant;

use dkg_primitives::{Event, KeyGenerator};
use tracing::{debug, error, info};


#[derive(Clone)]
pub struct CommitteeWorker<C: Config> {
    /// RPC url of the solver 
    solver_rpc_url: String,
    /// Receiver for the events
    rx: Arc<Mutex<Receiver<Event<C::Signature, C::Address>>>>,
    /// Internal state of the worker
    state: SessionWorkerState,
    /// Key generators for the current session
    key_generators: Vec<KeyGenerator<C::Address>>,
    /// Look ahead for the next round
    round_look_ahead: u64,
}

#[async_trait::async_trait]
impl<C: Config> SessionWorker<C> for CommitteeWorker<C> {
    async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Option<SessionResult<C::Signature>> {
        self.on_session(ctx, session_info).await
    }
}

impl<C: Config> CommitteeWorker<C> {

    /// Create a new instance of `CommitteeWorker`
    pub fn new(solver_rpc_url: String, rx: Receiver<Event<C::Signature, C::Address>>, key_generators: Vec<KeyGenerator<C::Address>>, round_look_ahead: u64) -> Self {
        Self { solver_rpc_url, rx: Arc::new(Mutex::new(rx)), state: SessionWorkerState::Init, key_generators, round_look_ahead }
    }

    /// For every next session, worker will wait for the `FinalizeKey` event and broadcast the encryption keys to the participants including the solver.
    /// Each session should be ended before timeout
    pub async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Option<SessionResult<C::Signature>> {
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
                            Event::FinalizeKey { commitments, session_id } => {
                                match self.state {
                                    SessionWorkerState::Init => {
                                        self.state = SessionWorkerState::Start(session_id);
                                    },
                                    SessionWorkerState::End(last_session_id) => {
                                        // It should be greater 
                                        if last_session_id > session_id {
                                            continue;
                                        }
                                        self.state = SessionWorkerState::Start(session_id);
                                    },
                                    _ => {
                                        debug!("Wrong internal state");
                                        continue;
                                    },
                                }
                                if let Err(err) =
                                    sync_finalized_enc_keys::<C>(&ctx, &mut key_generators, commitments, self.solver_rpc_url.clone(), session_id).await
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
                                            debug!("End and start session id should be equal!");
                                            continue;
                                        }
                                        self.state = SessionWorkerState::End(end_session_id);
                                    }, 
                                    _ => continue,
                                }
                                // Update the session id 
                                if let Ok(()) = end_session_id.next_mut() {
                                    if let Err(e) = end_session_id.put() {
                                        error!("Error during session id put: {:?}", e);
                                        return None;
                                    }
                                } else {
                                    return None;
                                }
                                if let Err(e) = end_session_id.put() {
                                    error!("Error during session id put: {:?}", e);
                                    return None;
                                }
                                if ctx.should_end_round((end_session_id + self.round_look_ahead.into()).into()) {
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
                                return Some(SessionResult::<C::Signature>::new());
                            }
                            _ => continue,
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