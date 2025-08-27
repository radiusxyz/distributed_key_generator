
use dkg_primitives::{Config, DbManager, DecKey, SolverEvent};
use tokio::sync::mpsc::Receiver;

pub struct KeyManager<C: Config> {
    ctx: C,
    event_rx: Receiver<SolverEvent<C::Address>>,
}

impl<C: Config> KeyManager<C> {
    pub fn new(ctx: C, event_rx: Receiver<SolverEvent<C::Address>>) -> Self {
        Self { ctx, event_rx }
    }

    pub async fn run(&mut self) {
        while let Some(event) = self.event_rx.recv().await {
            match event {
                SolverEvent::SubmitDecKey {
                    session_id,
                    timestamp,
                    dec_key,
                    who,
                } => {
                    tracing::info!(
                        "👍 Received dec key for session from solver {:?} {:?}",
                        session_id,
                        who
                    );
                    if let Ok(dec_key_request_record) =
                        self.ctx.db_manager().get_dec_key_request_record(session_id)
                    {
                        tracing::info!(
                            "👍 Dec key request record: {:?} of {:?}",
                            dec_key_request_record,
                            timestamp
                        );
                        if !dec_key_request_record.is_good(timestamp) {
                            tracing::info!("😡😡😡 BAD SOLVER {:?}", who);
                        }
                        // Anyway, we will store the dec key for the session
                        tracing::info!("Storing dec key for session {:?}", session_id);
                        let _ = DecKey::new(dec_key).put(session_id);
                    }
                }
            }
        }
    }
}
