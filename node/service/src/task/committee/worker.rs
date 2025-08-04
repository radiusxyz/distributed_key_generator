use super::{Config, sync_finalized_enc_keys, submit_enc_key, init_genesis_session, sync_start_time};
use crate::{committee::check_heartbeat, OperatorService, DbManager, SessionId, SessionInfo, SessionResult, SessionWorker};
use std::{sync::Arc, time::Duration};
use dkg_utils::timestamp;
use tokio::sync::{mpsc::{Sender, Receiver}, Mutex};
use dkg_primitives::{to_signed_commitment, EncKeyCommitment, NextOperatorList, OperatorTask};
use dkg_rpc::{helper::multicast_enc_key_ack, AsyncTask};
use futures_timer::Delay;
use std::time::Instant;
use dkg_primitives::{Operator, RuntimeError, SessionEvent, KeyGenerator};
use tracing::{error, info};

pub struct CommitteeWorker<C: Config> {
    /// RPC url of the solver 
    solver_rpc_url: String,
    /// Sender for the events
    event_tx: Sender<SessionEvent<C::Signature, C::Address>>,
    /// Receiver for the events
    event_rx: Arc<Mutex<Receiver<SessionEvent<C::Signature, C::Address>>>>,
    /// Whether the solver has solved the key
    has_solved: bool,
    /// Operators for the current session stored in memory
    operators: Option<Vec<Operator<C::Address>>>,
    /// Amount of sessions to add to the current session id
    add_session_amount: u64,
    /// Pending events
    pending: Option<SessionEvent<C::Signature, C::Address>>,
    /// Whether collecting key has been started
    start_collecting_key: Mutex<bool>,
    /// Sessions per round
    sessions_per_round: u64,
    /// Duration of collecting key
    collecting_duration: Duration,
    /// List of submitters 
    submitters: Option<Vec<C::Address>>,
}

#[async_trait::async_trait]
impl<C: Config> SessionWorker<C> for CommitteeWorker<C> {

    async fn on_genesis_session(&mut self, ctx: &C) -> Result<(), C::Error> {
        let session_id = SessionId::get().expect("Not initialized"); 
        if !session_id.is_initial() { panic!("Session id is not initial"); }
        let operators = ctx.operator_service().get_active_operators().await.expect("Failed to get initial operators");
        self.operators = Some(operators.clone());
        // Update the operator list for round 0 
        ctx.db_manager().update_active_operator_list(operators.clone()).expect("Failed to update operator list for round 0");
        // Wait for the nodes to be ready
        if self.is_ready(ctx, session_id, None, &operators).await? {
            // Initialize the genesis session
            let should_force_generating = ctx.should_force_generating()?;
            init_genesis_session(ctx, operators, session_id, should_force_generating); 
        }
        Ok(())
    }

    async fn on_session(&mut self, ctx: &mut C, session_info: SessionInfo) -> Option<SessionResult<C::Signature>> {
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
    pub fn new(solver_rpc_url: String, event_tx: Sender<SessionEvent<C::Signature, C::Address>>, event_rx: Receiver<SessionEvent<C::Signature, C::Address>>, add_session_amount: u64, sessions_per_round: u64, collecting_duration: Duration) -> Self {
        Self { 
            solver_rpc_url, 
            event_tx, 
            event_rx: Arc::new(Mutex::new(event_rx)), 
            add_session_amount, 
            has_solved: false, 
            operators: None, 
            pending: None, 
            start_collecting_key: Mutex::new(true), 
            sessions_per_round,
            collecting_duration,
            submitters: None,
        }
    }

    fn should_next_round(&self, session_id: SessionId) -> bool {
        let current_session: u64 = session_id.into();
        current_session % self.sessions_per_round == 0
    }

    fn handle_new_session(&mut self, ctx: &C) -> Result<(), C::Error> {
        info!("Handling new session");
        self.update_operators(ctx)?;
        Ok(())
    }

    fn update_operators(&mut self, ctx: &C) -> Result<(), C::Error> {
        if let Ok(next_operator_list) = NextOperatorList::<C::Address>::get() {
            self.operators = Some(next_operator_list.inner().clone());
            ctx.db_manager().update_active_operator_list(next_operator_list.inner()).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
            let _ = NextOperatorList::<C::Address>::delete();
        }
        Ok(())
    }

    pub fn add_pending_event(&mut self, event: SessionEvent<C::Signature, C::Address>) {
        if self.pending.is_none() {
            self.pending = Some(event);
        }
    }

    pub fn remove_pending_event(&mut self) {
        if self.pending.is_some() {
            self.pending = None;
        }
    }

    fn start_at() -> u128 {
        timestamp() + Duration::from_secs(10).as_millis()
    }

    fn key_len(&self) -> usize {
        self.submitters.as_ref().map_or(0, |submitters| submitters.len())
    }

    async fn create_task(&self, ctx: &C, data: Vec<u8>) -> Result<(), C::Error> {
        ctx.operator_service().create_new_task(data).await.map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
        Ok(())
    }

    async fn start_collecting(&mut self, ctx: &C, event_tx: Sender<SessionEvent<C::Signature, C::Address>>) {
        *self.start_collecting_key.lock().await = false;
        let duration = self.collecting_duration;
        ctx.async_task().spawn_task(async move { 
            Delay::new(duration).await;
            let _ = event_tx.send(SessionEvent::CollectingTimeout).await;
        });
    }

    async fn reset_collecting(&mut self) {
        self.submitters = None;
        *self.start_collecting_key.lock().await = true;
    }

    // TODO: Do we need to check node is live at this time?
    async fn wait_for_start_time(&self, start_time: u128) {
        let now = timestamp();
        if start_time > now {
            let wait_ms = start_time - now;
            let wait_secs = wait_ms / 1000;
            info!("Waiting for {:?}s before starting the protocol", wait_secs);

            let log_interval = if wait_secs > 30 { 5 } else { 1 };
            let mut remaining_secs = wait_secs;
            while remaining_secs > 0 {
                let sleep_secs = std::cmp::min(remaining_secs, log_interval);
                tokio::time::sleep(Duration::from_secs(sleep_secs as u64)).await;
                remaining_secs -= sleep_secs;
                if remaining_secs > 0 {
                    info!("⏳ Protocol starts in {:?}s", remaining_secs);
                }
            }

            let final_wait_ms = wait_ms % 1000;
            // Wait for the remaining time
            if final_wait_ms > 0 {
                tokio::time::sleep(Duration::from_millis(final_wait_ms as u64)).await;
            }
        }
        info!("✅ Protocol starting now")
    }

    /// Wait for the number of nodes(e.g committee) to be ready 
    pub async fn is_ready(&self, ctx: &C, session_id: SessionId, threshold: Option<u16>, operators: &Vec<Operator<C::Address>>) -> Result<bool, C::Error> {
        if ctx.is_leader(session_id) {
            info!("Leader of the session: {:?}, threshold: {:?}, operators: {:?}", session_id, threshold, operators);
            loop {
                let (is_ready, ready_nodes) = self.check_is_ready(ctx, threshold, &operators).await?;
                if is_ready {
                    info!("({}/{}) committee members are ready", ready_nodes, operators.len());
                    let start_time = Self::start_at();
                    info!("⏰ Protocol starts at: {:?}", start_time);
                    sync_start_time(ctx, start_time, operators)?;
                    self.wait_for_start_time(start_time).await;
                    return Ok(true);
                } else {
                    // `self` is also included in the count
                    info!("Waiting for other committee members to be ready. ({}/{}) are ready", ready_nodes+1, operators.len());
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        } else {
            info!("Follower of the session: {:?}", session_id);
            loop {
                let mut rx = self.event_rx.lock().await;
                if let Some(event) = rx.recv().await {
                    match event {
                        SessionEvent::GenesisSession(start_time) => {
                            info!("⏰ Protocol starts at: {:?}", start_time);
                            self.wait_for_start_time(start_time).await;
                            return Ok(true);
                        },
                        _ => continue,
                    }
                }
            }
        }
    }

    /// Count the number of nodes that are ready excluding `self` 
    pub async fn check_is_ready(&self, ctx: &C, threshold: Option<u16>, operators: &Vec<Operator<C::Address>>) -> Result<(bool, usize), C::Error> {
        let mut is_ready = false;
        if operators.len() == 0 {
            panic!("No key generators");
        }
        if operators.len() == 1 {
            return Ok((true, 1));
        }
        // Exclude `self` from the count
        let expected_nodes_num = threshold.map_or(operators.len()-1, |t| t as usize);
        let mut ready_nodes = 0;
        for operator in operators {
            if operator.address() == ctx.address() { continue; }
            match check_heartbeat(ctx, operator.external_rpc_url().to_string(), operator.address()).await {
                Ok(true) => { 
                    ready_nodes += 1; 
                    if ready_nodes >= expected_nodes_num {
                        is_ready = true;
                    }
                },
                _ => continue,
            }
        }
        return Ok((is_ready, ready_nodes));
    }

    async fn add_submitter(&mut self, ctx: &C, event_tx: Sender<SessionEvent<C::Signature, C::Address>>, session_id: SessionId, submitter: C::Address) {
        if let Some(submitters) = self.submitters.as_mut() {
            submitters.push(submitter);
        } else {
            info!("Start collecting key at session: {:?}", session_id);
            self.submitters = Some(vec![submitter]);
            self.start_collecting(ctx, event_tx).await;
        }
    }

    async fn handle_submit_enc_key(&mut self, ctx: &C, event_tx: Sender<SessionEvent<C::Signature, C::Address>>, session_id: SessionId, submitter: C::Address) -> Result<(), C::Error> {
        let is_collecting = {
            let lock = self.start_collecting_key.lock().await;
            *lock
        };
        if is_collecting {
            self.add_submitter(ctx, event_tx, session_id, submitter).await;
        }
        Ok(())
    }

    fn do_gen_enc_key(&self, ctx: &C, session_id: SessionId) -> Result<EncKeyCommitment<C::Signature, C::Address>, C::Error> {
        let enc_key = ctx.key_generator().gen_enc_key(ctx.randomness(session_id), None).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
        let commitment = to_signed_commitment(ctx, session_id, enc_key)?;
        Ok(EncKeyCommitment::<C::Signature, C::Address>::new(commitment))
    }

    async fn finalize_key(&mut self, ctx: &C, session_id: SessionId) -> Result<(), C::Error> {
        let mut commitments = Vec::new();
        if let Some(submitters) = self.submitters.take() {
            commitments = submitters.into_iter().map(|submitter| {
                EncKeyCommitment::<C::Signature, C::Address>::get(&session_id, &submitter)
            }).collect::<Result<Vec<_>, _>>()?;
        } else {
            error!("No submitters to finalize key. Creating a new key at session: {:?}", session_id);
            let enc_key_commitment = self.do_gen_enc_key(ctx, session_id)?;
            enc_key_commitment.put(&session_id, &ctx.address())?;
            let _ = multicast_enc_key_ack(ctx, session_id, enc_key_commitment.clone());
            commitments.push(enc_key_commitment);
        }
        self.reset_collecting().await;
        self.event_tx.send(SessionEvent::FinalizeKey { commitments, start_session_id: session_id }).await.map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
        Ok(())
    }

    async fn do_submit_enc_key(&mut self, ctx: &C, session_id: SessionId) -> Result<(), C::Error> {
        let enc_key = ctx.key_generator().gen_enc_key(ctx.randomness(session_id), None).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
        submit_enc_key(ctx, session_id, enc_key).map_err(|_| RuntimeError::AnyError("Failed to submit encryption key".into()))?;
        Ok(())
    }

    /// End the current session
    /// 1. Update the current session
    /// 2. (Optional) Force generating encryption key if there is only one committee member
    pub async fn do_end_session(&mut self, ctx: &mut C, on_session_id: &mut SessionId) -> Result<(), C::Error> {
        // Update the current session
        self.remove_pending_event();
        let current_session_id = on_session_id.clone();
        on_session_id.next_mut(self.add_session_amount)?.put()?;
        let is_leader = ctx.is_leader(current_session_id);
        let should_next_round = self.should_next_round(*on_session_id);
        // If there is only one committee member, we need to force generating
        // Otherwise, submit the encryption key to the leader
        if ctx.should_force_generating()? {
            info!("Force generating encryption key at session: {:?}", *on_session_id);
            self.do_submit_enc_key(ctx, *on_session_id).await?;
        } else if !is_leader {
            // Submit the encryption key to the leader if not a leader on current session
            self.do_submit_enc_key(ctx, *on_session_id).await?;
        } 
        // Current leader should create the task before the next round 
        if is_leader && should_next_round {
            info!("Creating task for the next round: {:?}", *on_session_id);
            let task = OperatorTask::get().map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
            self.create_task(ctx, task.inner()).await?;
        }
        Ok(())
    }

    /// For every next session, worker will wait for the `FinalizeKey` event and broadcast the encryption keys to the participants including the solver.
    /// Each session should be ended before timeout
    pub async fn on_session(&mut self, ctx: &mut C, session_info: SessionInfo) -> Result<SessionResult<C::Signature>, C::Error> {
        let mut on_session_id = session_info.session_id;
        let is_leader = ctx.is_leader(on_session_id);
        if is_leader { info!("👤 Leader of the session: {:?}", on_session_id); }
        self.handle_new_session(ctx)?;
        let mut operators = self.operators.clone().ok_or(RuntimeError::AnyError("No operators".into()))?;
        if let Some(event) = self.pending.take() {
            self.event_tx.send(event).await.map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
        }
        self.has_solved = false;
        // Ends at after this delay 
        let mut timeout = Delay::new(session_info.ends_at.duration_since(Instant::now()));
        loop {
            tokio::select! {
                // Event from the the sender channel
                event = async {
                    let mut rx = self.event_rx.lock().await;
                    rx.recv().await
                } => {
                    if let Some(event) = event {
                        info!("{:?}", event);
                        match event.clone() {
                            SessionEvent::SubmitEncKey { submitter, session_id } => {
                                if session_id < on_session_id {
                                    info!("Stale session {:?}. Ignore it", session_id);
                                    continue;
                                }
                                self.handle_submit_enc_key(ctx, self.event_tx.clone(), on_session_id, submitter).await?;
                                continue;
                            },
                            SessionEvent::FinalizeKey { commitments, start_session_id } => {
                                if start_session_id < on_session_id { 
                                    info!("Stale session {:?}. Ignore it", start_session_id);
                                    continue;
                                }
                                if is_leader {
                                    // TODO: Wait for some time for other committees to send encryption key?
                                    sync_finalized_enc_keys::<C>(&ctx, &mut operators, commitments, self.solver_rpc_url.clone(), start_session_id).await?;
                                    self.add_pending_event(event);
                                }
                                continue;
                            },
                            SessionEvent::EndSession(end_session_id) => {
                                if end_session_id < on_session_id {
                                    info!("Stale session {:?}. Ignore it", end_session_id);
                                    continue;
                                }
                                // If solver has decrypted the key, it means the key generation process has started
                                // We only update `has_started` at session `0`
                                self.has_solved = true;
                                self.do_end_session(ctx, &mut on_session_id).await?;
                                return Ok(SessionResult::<C::Signature>::new());
                            },
                            SessionEvent::CollectingTimeout => {
                                info!("Collecting key timeout. Finalizing {} keys at session: {:?}", self.key_len(), on_session_id);
                                self.finalize_key(ctx, on_session_id).await?;
                                continue;
                            },
                            _ => {
                                info!("Ignore event: {:?}", event);
                                continue;
                            },
                        }
                    }
                },
                // Timeout for the session
                _ = &mut timeout => {
                    if self.has_solved {
                        error!("Timeout. Force ending session: {:?}", on_session_id);
                        self.do_end_session(ctx, &mut on_session_id).await?;
                    } else {
                        // Don't update the session id until the solver has solved the key
                        error!("Timeout. Maybe solver has turned off?")
                    }
                    return Ok(SessionResult::<C::Signature>::new());
                }
            }
        }
    }
}