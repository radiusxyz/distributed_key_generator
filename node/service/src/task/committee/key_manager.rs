use dkg_primitives::{Config, DecKey, DbManager, SolverEvent};
use tokio::sync::mpsc::Receiver;

pub struct KeyManager<C: Config> {
    ctx: C,
    event_rx: Receiver<SolverEvent>,
}

impl<C: Config> KeyManager<C> {
    pub fn new(ctx: C, event_rx: Receiver<SolverEvent>) -> Self {
        Self { ctx, event_rx }
    }

    pub async fn run(&mut self) {
        while let Some(event) = self.event_rx.recv().await {
            match event {
                SolverEvent::SubmitDecKey { session_id, timestamp, dec_key } => {
                    tracing::info!("👍 Received dec key for session from solver {:?}", session_id);
                    if let Ok(dec_key_request_record) = self.ctx.db_manager().get_dec_key_request_record(session_id) {
                        if !dec_key_request_record.is_good(timestamp) {
                            // TODO: Maybe slashing?
                        }
                        let _ = DecKey::new(dec_key).put(session_id);
                    }
                }
            }
        }
    }
}