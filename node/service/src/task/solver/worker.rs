use super::{do_solve_key, submit_dec_key};
use crate::{SessionInfo, SessionResult, SessionWorker};
use std::sync::Arc;
use tokio::sync::{mpsc::{self, Receiver, Sender}, Mutex};
use dkg_rpc::{Config, SessionId};
use futures_timer::Delay;
use std::time::Instant;
use dkg_primitives::{DbManager, NextOperatorList, OperatorService, RuntimeError, SessionEvent};
use tracing::{error, info};

pub struct SolverWorker<C: Config> {
    rx: Arc<Mutex<Receiver<SessionEvent<C::Signature, C::Address>>>>,
    has_started: bool,
    session_tx: Sender<SessionId>,
    session_rx: Receiver<SessionId>,
}

#[async_trait::async_trait]
impl<C: Config> SessionWorker<C> for SolverWorker<C> {

    async fn on_genesis_session(&mut self, ctx: &C) -> Result<(), C::Error> {
        let operators = ctx.operator_service().get_active_operators().await.expect("Failed to get initial operators");
        loop {
            if ctx.operator_service().is_ready(1).await.unwrap() {
                // Update the key generator list for round 0 
                ctx.db_manager().update_active_operator_list(operators.clone()).expect("Failed to update operator list");
                break;
            }
        }
        Ok(())
    }

    async fn on_session(&mut self, ctx: &mut C, session_info: SessionInfo) -> Option<SessionResult<C::Signature>> {
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
    pub fn new(rx: Receiver<SessionEvent<C::Signature, C::Address>>) -> Self {
        let (session_tx, session_rx) = mpsc::channel(10);
        Self { rx: Arc::new(Mutex::new(rx)), has_started: false, session_tx, session_rx }
    }

    fn handle_new_session(&mut self, ctx: &C) -> Result<(), C::Error> {
        if let Ok(next_operator_list) = NextOperatorList::<C::Address>::get() {
            ctx.db_manager().update_active_operator_list(next_operator_list.inner()).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
            let _ = NextOperatorList::<C::Address>::delete();
        }
        Ok(())
    }

    pub async fn do_end_session(&self, on_session_id: &mut SessionId, amount: u64) -> Result<(), C::Error> {
        on_session_id.next_mut(amount)?.put()?;
        Ok(())
    }

    /// For every next session, worker will wait for the `SolveKey` event and submit the decryption key to the leader.
    /// Each session should be ended before timeout
    pub async fn on_session(&mut self, ctx: &mut C, session_info: SessionInfo) -> Result<SessionResult<C::Signature>, C::Error> {
        let mut on_session_id = session_info.session_id;
        self.handle_new_session(ctx)?;
        // Ends at after this delay 
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
                            SessionEvent::SolveKey { enc_key, session_id } => {
                                let ctx = ctx.clone();
                                let _tx = self.session_tx.clone();
                                // Solve key in background task
                                tokio::spawn(async move {
                                    // Since do_solve_key is a blocking operation, solutions that exceed the session duration
                                    // will be discarded to maintain timing consistency
                                    match do_solve_key(&ctx, session_id, &enc_key) {
                                        Ok(commitment) => {
                                            if let Err(e) = submit_dec_key(&ctx, session_id, commitment).await {
                                                error!("Error submitting dec key: {:?}", e);
                                            }
                                        }
                                        Err(e) => {
                                            // TODO: handle error - store log on db
                                            error!("Error solving key: {:?}", e);
                                        }
                                    }
                                    let _ = _tx.send(session_id).await;
                                });
                            },
                            _ => {
                                info!("Ignore event: {:?}", event);
                                continue;
                            },
                        }
                    }
                },
                Some(solved_session_id) = self.session_rx.recv() => {
                    if solved_session_id < on_session_id {
                        error!("Stale session {:?}. Ignore it", solved_session_id);
                        continue; 
                    } else {
                        info!("End session: {:?}", solved_session_id);
                        if on_session_id.is_initial() {
                            self.has_started = true;
                        }
                        self.do_end_session(&mut on_session_id, 1).await?;
                    }
                    return Ok(SessionResult::<C::Signature>::new());
                },
                _ = &mut timeout => {
                    if self.has_started {
                        error!("Timeout. Force ending session: {:?}", on_session_id);
                        self.do_end_session(&mut on_session_id, 1).await?;
                    } else {
                        error!("Timeout but not started yet. Maybe committee not started yet?")
                    }
                    return Ok(SessionResult::<C::Signature>::new());
                }
            }
        }
    }
}
