use super::{Config, sync_finalized_enc_keys, submit_enc_key, init_genesis_session};
use crate::{SessionWorkerState, SessionWorker, SessionInfo, SessionResult, AuthService, DbManager, KeyService, SessionId};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, Mutex};
use futures_timer::Delay;
use std::time::Instant;

use dkg_primitives::{KeyGenerator, Round, RuntimeError, RuntimeEvent};
use tracing::{error, info};


#[derive(Clone)]
pub struct CommitteeWorker<C: Config> {
    /// RPC url of the solver 
    solver_rpc_url: String,
    /// Receiver for the events
    rx: Arc<Mutex<Receiver<RuntimeEvent<C::Signature, C::Address>>>>,
    /// Internal state of the worker
    state: SessionWorkerState,
    /// Key generators for the current session
    key_generators: Option<Vec<KeyGenerator<C::Address>>>,
    /// Look ahead for the next round
    round_look_ahead: u64,
    /// Amount of sessions to add to the current session id
    add_session_amount: u64,
}

#[async_trait::async_trait]
impl<C: Config> SessionWorker<C> for CommitteeWorker<C> {

    async fn on_genesis_session(&mut self, ctx: &C) -> Result<(), C::Error> {
        let session_id = SessionId::get().expect("Not initialized"); 
        if !session_id.is_initial() { panic!("Session id is not initial"); }
        let current_round = Round::get().expect("Not initialized");
        if !current_round.is_initial() { panic!("Current round is not initial"); }
        let key_generators_for_round_0 = ctx.auth_service().get_key_generators(&current_round).await.expect("Failed to get initial key generators");
        self.key_generators = Some(key_generators_for_round_0.clone());
        let key_generators_for_round_1 = ctx.auth_service().get_key_generators(&(current_round.clone()+ 1)).await.expect("Failed to get key generators for round 1");
        loop {
            if ctx.auth_service().is_ready(&current_round, ctx.threshold()).await.unwrap() {
                // Update the key generator list for round 0 
                ctx.db_manager().update_key_generator_list(&current_round, key_generators_for_round_0.clone()).expect("Failed to update key generator list for round 0");
                // Update the key generator list for round 1
                ctx.db_manager().update_key_generator_list(&(current_round.clone()+1), key_generators_for_round_1.clone()).expect("Failed to update key generator list for round 1");
                init_genesis_session(ctx, key_generators_for_round_0, session_id); 
                break;
            }
        }
        Ok(())
    }

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
    pub fn new(solver_rpc_url: String, rx: Receiver<RuntimeEvent<C::Signature, C::Address>>, round_look_ahead: u64, add_session_amount: u64) -> Self {
        Self { solver_rpc_url, rx: Arc::new(Mutex::new(rx)), state: SessionWorkerState::Init, key_generators: None, round_look_ahead, add_session_amount }
    }

    pub async fn do_end_session(&mut self, ctx: &C, on_session_id: &mut SessionId) -> Result<(), C::Error> {
        self.state = SessionWorkerState::Active(*on_session_id);
        on_session_id.next_mut(self.add_session_amount)?.put()?;
        let current_round = ctx.db_manager().current_round().map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
        if ctx.should_force_generating(&current_round)? || !ctx.is_leader() {
            info!("Force generating encryption key at session: {:?}", *on_session_id);
            // Send the encryption key to the leader before session ends
            let enc_key = ctx.key_service().gen_enc_key(ctx.randomness(*on_session_id), None).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
            submit_enc_key(*on_session_id, enc_key, ctx).map_err(|_| RuntimeError::AnyError("Failed to submit encryption key".into()))?;
        }
        if ctx.should_end_round((*on_session_id + self.round_look_ahead.into()).into()) {
            let next_round = current_round.next().ok_or(RuntimeError::Arithmetic)?;
            let key_generators = ctx.auth_service().get_key_generators(&next_round).await.map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
            ctx.db_manager().update_key_generator_list(&next_round, key_generators).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
        }
        Ok(())
    }

    /// For every next session, worker will wait for the `FinalizeKey` event and broadcast the encryption keys to the participants including the solver.
    /// Each session should be ended before timeout
    pub async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Result<SessionResult<C::Signature>, C::Error> {
        let mut on_session_id = session_info.session_id;
        let mut key_generators = self.key_generators.clone()
            .ok_or(RuntimeError::AnyError("Empty key generators".into()))?
            .into_iter()
            .filter(|kg| kg.address() != ctx.address())
            .collect();
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
                            RuntimeEvent::FinalizeKey { commitments, start_session_id } => {
                                if start_session_id < on_session_id { 
                                    info!("Stale session {:?}. Ignore it", start_session_id);
                                    continue;
                                }
                                if ctx.is_leader() {
                                    sync_finalized_enc_keys::<C>(&ctx, &mut key_generators, commitments, self.solver_rpc_url.clone(), start_session_id).await?;
                                }
                                continue;
                            },
                            RuntimeEvent::EndSession(end_session_id) => {
                                if end_session_id < on_session_id {
                                    info!("Stale session {:?}. Ignore it", end_session_id);
                                    continue;
                                }
                                if self.state.is_init() {
                                    self.state = SessionWorkerState::Active(end_session_id);
                                }
                                self.do_end_session(ctx, &mut on_session_id).await?;
                                return Ok(SessionResult::<C::Signature>::new());
                            }
                            _ => {
                                info!("Ignore event: {:?}", event);
                                continue;
                            },
                        }
                    }
                },
                _ = &mut timeout => {
                    error!("⏳Timeout on session: {:?}. Force ending session: {:?}", on_session_id, on_session_id);
                    self.do_end_session(ctx, &mut on_session_id).await?;
                    return Ok(SessionResult::<C::Signature>::new());
                }
            }
        }
    }
}