use super::{solve, submit_dec_key};
use crate::{SessionWorkerState, SessionInfo, SessionResult, SessionWorker};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, Mutex};
use dkg_rpc::Config;
use futures_timer::Delay;
use std::time::Instant;

use dkg_primitives::{RuntimeEvent, RuntimeError};
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct SolverWorker<C: Config> {
    rx: Arc<Mutex<Receiver<RuntimeEvent<C::Signature, C::Address>>>>,
    state: SessionWorkerState,
}

#[async_trait::async_trait]
impl<C: Config> SessionWorker<C> for SolverWorker<C> {
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

impl<C: Config> SolverWorker<C> {

    /// Create a instance of `SessionWorker`
    pub fn new(rx: Receiver<RuntimeEvent<C::Signature, C::Address>>) -> Self {
        Self { rx: Arc::new(Mutex::new(rx)), state: SessionWorkerState::Init }
    }

    /// For every next session, worker will wait for the `SolveKey` event and submit the decryption key to the leader.
    /// Each session should be ended before timeout
    pub async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Result<SessionResult<C::Signature>, C::Error> {
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
                            RuntimeEvent::SolveKey { enc_key, session_id } => {
                                match solve(ctx, session_id, &enc_key) {
                                    Ok(commitment) => {
                                        if let Err(e) = submit_dec_key(ctx, commitment).await {
                                            error!("Error submitting dec key: {:?}", e);
                                            continue;
                                        }
                                    }
                                    Err(e) => {
                                        error!("Error solving key: {:?}", e);
                                        continue;
                                    }
                                }
                            }
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
                                end_session_id.next_mut(1)?.put()?;
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
