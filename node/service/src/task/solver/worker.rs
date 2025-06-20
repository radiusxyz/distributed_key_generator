use super::{do_solve_key, submit_dec_key};
use crate::{SessionWorkerState, SessionInfo, SessionResult, SessionWorker};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, Mutex, Notify};
use dkg_rpc::Config;
use futures_timer::Delay;
use std::time::Instant;

use dkg_primitives::{RuntimeEvent, RuntimeError, Round, AuthService, DbManager};
use tracing::{error, info};

#[derive(Clone)]
pub struct SolverWorker<C: Config> {
    rx: Arc<Mutex<Receiver<RuntimeEvent<C::Signature, C::Address>>>>,
    state: Arc<Mutex<SessionWorkerState>>,
    notify: Arc<Notify>,
}

#[async_trait::async_trait]
impl<C: Config> SessionWorker<C> for SolverWorker<C> {

    async fn on_genesis_session(&mut self, ctx: &C) -> Result<(), C::Error> {
        let current_round = Round::get().expect("Not initialized");
        if !current_round.is_initial() { panic!("Current round is not initial"); }
        let key_generators_for_round_0 = ctx.auth_service().get_key_generators(&current_round).await.expect("Failed to get initial key generators");
        let key_generators_for_round_1 = ctx.auth_service().get_key_generators(&(current_round.clone()+ 1)).await.expect("Failed to get key generators for round 1");
        loop {
            if ctx.auth_service().is_ready(&current_round, ctx.threshold()).await.unwrap() {
                // Update the key generator list for round 0 
                ctx.db_manager().update_key_generator_list(&current_round, key_generators_for_round_0.clone()).expect("Failed to update key generator list for round 0");
                // Update the key generator list for round 1
                ctx.db_manager().update_key_generator_list(&(current_round.clone()+1), key_generators_for_round_1.clone()).expect("Failed to update key generator list for round 1");
                break;
            }
        }
        Ok(())
    }

    async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Option<SessionResult<C::Signature>> {
        let session_id = session_info.session_id;
        match self.on_session(ctx, session_info).await {
            Ok(res) => Some(res),
            Err(e) => {
                error!("Something wrong on session {:?}: {:?}", session_id, e);
                None
            }
        }
    }
}

impl<C: Config> SolverWorker<C> {

    /// Create a instance of `SessionWorker`
    pub fn new(rx: Receiver<RuntimeEvent<C::Signature, C::Address>>) -> Self {
        Self { rx: Arc::new(Mutex::new(rx)), state: Arc::new(Mutex::new(SessionWorkerState::Init)), notify: Arc::new(Notify::new()) }
    }

    /// For every next session, worker will wait for the `SolveKey` event and submit the decryption key to the leader.
    /// Each session should be ended before timeout
    pub async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Result<SessionResult<C::Signature>, C::Error> {
        // Ends at after this delay 
        let mut on_session_id = session_info.session_id;
        let mut timeout = Delay::new(session_info.ends_at.duration_since(Instant::now()));
        loop {
            tokio::select! {
                event = async {
                    let mut rx = self.rx.lock().await;
                    rx.recv().await
                } => {
                    if let Some(event) = event {
                        info!("{:?}", event);
                        match event {
                            RuntimeEvent::SolveKey { enc_key, session_id } => {
                                info!("Solving key for session {:?}", session_id);
                                let ctx = ctx.clone();
                                let state = self.state.clone();
                                let notify = self.notify.clone();
                                tokio::spawn(async move {
                                    match do_solve_key(&ctx, session_id, &enc_key) {
                                        Ok(commitment) => {
                                            if let Err(e) = submit_dec_key(&ctx, commitment).await {
                                                error!("Error submitting dec key: {:?}", e);
                                            }
                                            let mut _lock = state.lock().await;
                                            *_lock = SessionWorkerState::End(session_id);
                                            notify.notify_one();
                                        }
                                        Err(e) => {
                                            // TODO: handle error - store log on db
                                            error!("Error solving key: {:?}", e);
                                            // Just proceed to the next session
                                            let mut _lock = state.lock().await;
                                            *_lock = SessionWorkerState::End(session_id);
                                            notify.notify_one();
                                        }
                                    }
                                });
                            },
                            _ => {
                                info!("Ignore event: {:?}", event);
                                continue;
                            },
                        }
                    }
                },
                _ = self.notify.notified() => {
                    match *self.state.lock().await {
                        SessionWorkerState::End(end_session_id) => {
                            if end_session_id != on_session_id {
                                return Err(RuntimeError::AnyError(format!("End session id mismatch: {:?} != {:?}", end_session_id, on_session_id).into()).into());
                            }
                            on_session_id.next_mut(1)?.put()?;
                            return Ok(SessionResult::<C::Signature>::new());
                        },
                        _ => { return Err(RuntimeError::AnyError("Wrong state".into()).into()); }
                    }
                },
                _ = &mut timeout => {
                    return Err(RuntimeError::AnyError("Timeout".into()).into());
                }
            }
        }
    }
}
