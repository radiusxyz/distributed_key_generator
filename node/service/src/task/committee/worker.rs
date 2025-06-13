use super::{Config, sync_finalized_enc_keys, submit_enc_key};
use crate::{SessionWorkerState, SessionWorker, SessionInfo, SessionResult, AuthService, DbManager, KeyService};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, Mutex};
use futures_timer::Delay;
use std::time::Instant;

use dkg_primitives::{KeyGenerator, RuntimeError, RuntimeEvent};
use tracing::{debug, error, info};


#[derive(Clone)]
pub struct CommitteeWorker<C: Config> {
    /// RPC url of the solver 
    solver_rpc_url: String,
    /// Receiver for the events
    rx: Arc<Mutex<Receiver<RuntimeEvent<C::Signature, C::Address>>>>,
    /// Internal state of the worker
    state: SessionWorkerState,
    /// Key generators for the current session
    key_generators: Vec<KeyGenerator<C::Address>>,
    /// Look ahead for the next round
    round_look_ahead: u64,
    /// Amount of sessions to add to the current session id
    add_session_amount: u64,
}

#[async_trait::async_trait]
impl<C: Config> SessionWorker<C> for CommitteeWorker<C> {
    async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Option<SessionResult<C::Signature>> {
        match self.on_session(ctx, session_info).await {
            Ok(res) => Some(res),
            Err(e) => {
                error!("Something wrong on session: {:?}", e);
                None
            }
        }
    }
}

impl<C: Config> CommitteeWorker<C> {

    /// Create a new instance of `CommitteeWorker`
    pub fn new(solver_rpc_url: String, rx: Receiver<RuntimeEvent<C::Signature, C::Address>>, key_generators: Vec<KeyGenerator<C::Address>>, round_look_ahead: u64, add_session_amount: u64) -> Self {
        Self { solver_rpc_url, rx: Arc::new(Mutex::new(rx)), state: SessionWorkerState::Init, key_generators, round_look_ahead, add_session_amount }
    }

    /// For every next session, worker will wait for the `FinalizeKey` event and broadcast the encryption keys to the participants including the solver.
    /// Each session should be ended before timeout
    pub async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Result<SessionResult<C::Signature>, C::Error> {
        let on_session_id = session_info.session_id;
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
                            RuntimeEvent::FinalizeKey { commitments, start_session_id } => {
                                if start_session_id != on_session_id { 
                                    info!("Session id mismatch: {:?} != {:?}", start_session_id, on_session_id);
                                    continue;
                                }
                                match self.state {
                                    SessionWorkerState::Init => {
                                        self.state = SessionWorkerState::Start(start_session_id);
                                    },
                                    SessionWorkerState::End(last_session_id) => {
                                        // It should be greater 
                                        if last_session_id > start_session_id {
                                            continue;
                                        }
                                        self.state = SessionWorkerState::Start(start_session_id);
                                    },
                                    _ => {
                                        debug!("Wrong internal state");
                                        continue;
                                    },
                                }
                                if ctx.is_leader() {
                                    sync_finalized_enc_keys::<C>(&ctx, &mut key_generators, commitments, self.solver_rpc_url.clone(), start_session_id).await?;
                                }
                                continue;
                            },
                            RuntimeEvent::EndSession(mut end_session_id) => {
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
                                end_session_id.next_mut(self.add_session_amount)?.put()?;
                                if !ctx.is_leader() {
                                    // Send the encryption key to the leader before session ends
                                    let enc_key = ctx.key_service().gen_enc_key(ctx.randomness(on_session_id), None).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
                                    submit_enc_key(end_session_id, enc_key, ctx).map_err(|_| RuntimeError::AnyError("Failed to submit encryption key".into()))?;
                                }
                                if ctx.should_end_round((end_session_id + self.round_look_ahead.into()).into()) {
                                    let round = ctx.db_manager().current_round().map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
                                    let next_round = round.next().ok_or(RuntimeError::Arithmetic)?;
                                    let key_generators = ctx.auth_service().get_key_generators(next_round.clone().into()).await.map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
                                    ctx.db_manager().update_key_generator_list(next_round, key_generators).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
                                }
                                return Ok(SessionResult::<C::Signature>::new());
                            }
                            _ => continue,
                        }
                    }
                },
                _ = &mut timeout => {
                    info!("Timeout for session {:?}", session_info.session_id);
                    return Err(RuntimeError::AnyError("Timeout".into()).into());
                }
            }
        }
    }
}