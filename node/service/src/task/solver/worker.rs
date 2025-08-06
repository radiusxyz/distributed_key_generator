use super::{do_solve_key, submit_dec_key};
use std::sync::Arc;
use tokio::{sync::{mpsc::Receiver}, time::Duration};
use dkg_rpc::Config;
use dkg_primitives::{NextOperatorList, SessionEvent, DbManager, RuntimeError};
use tracing::error;

pub struct SolverWorker<C: Config> {
    ctx: Arc<C>,
    rx: Receiver<SessionEvent<C::Signature, C::Address>>,
    _session_duration: Duration,
}

impl<C: Config> SolverWorker<C> {

    /// Create a instance of `SessionWorker`
    pub fn new(ctx: C, rx: Receiver<SessionEvent<C::Signature, C::Address>>, session_duration: Duration) -> Self {
        Self { ctx: Arc::new(ctx), rx, _session_duration: session_duration }
    }

    /// Update the operators if the next operator list is available
    fn should_update_operators(&self, ctx: &C) -> Result<(), C::Error> {
        if let Ok(next_operator_list) = NextOperatorList::<C::Address>::get() {
            ctx.db_manager().update_active_operator_list(next_operator_list.inner()).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
            let _ = NextOperatorList::<C::Address>::delete();
        }
        Ok(())
    }

    /// Solver will simply wait for the `SolveKey` event and submit the decryption key to the leader.
    pub async fn run(&mut self) {
        while let Some(event) = self.rx.recv().await {
            match event {
                SessionEvent::SolveKey { enc_key, session_id } => {
                    let ctx = self.ctx.clone();
                    let _ = self.should_update_operators(&ctx);
                    // Solve key in background task and send the result to the leader
                    tokio::spawn(async move {
                        // Since do_solve_key is a blocking operation, solutions that exceed the session duration
                        // will be discarded to maintain timing consistency
                        match do_solve_key(&*ctx, session_id, &enc_key) {
                            Ok(commitment) => {
                                if let Err(e) = submit_dec_key(&*ctx, session_id, commitment).await {
                                    error!("Error submitting dec key: {:?}", e);
                                }
                            }
                            Err(e) => {
                                // TODO: handle error - store log on db
                                error!("Error solving key: {:?}", e);
                            }
                        }
                    });
                },
                _ => continue,
            }
        }
    }
}
