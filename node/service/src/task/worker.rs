use super::*;
use dkg_primitives::SessionId;
use dkg_utils::timestamp;
use std::{marker::PhantomData, time::{Duration, Instant}};
use futures_timer::Delay;

pub struct SessionResult<Signature>(PhantomData<Signature>);

impl<Signature> SessionResult<Signature> {
    pub fn new() -> Self {
        Self(Default::default())
    }
}

pub async fn run_session_worker<C, SW>(ctx: &C, worker: &mut SW, session_duration: Duration) -> Result<(), C::Error> 
where
    C: Config,
    SW: SessionWorker<C>
{
    let mut sessions = Sessions::new(session_duration);
    loop {
        let session_info = sessions.next_session().await;
        tracing::info!("Session info: {:?}", session_info.session_id);
        if session_info.session_id.is_initial() {
            worker.on_genesis_session(ctx).await?; 
        }
        let _ = worker.on_session(ctx, session_info).await;
    }   
}

/// Calculate the duration in milliseconds until the next session starts from now 
fn time_until_next_session(session_duration: Duration) -> Duration {
    let now = timestamp();
    let session_duration_millis = session_duration.as_millis();
    let next_session = (now + session_duration_millis) / session_duration_millis;
    let remaining_millis = (next_session * session_duration_millis) - now;
    Duration::from_millis(remaining_millis as u64) 
}

/// Information about the session
pub struct SessionInfo {
    /// Current session number
    pub session_id: SessionId, 
    /// Duration of the session in milliseconds
    pub duration: Duration, 
    /// Instant when the session ends
    pub ends_at: Instant,
}

impl SessionInfo {
    pub fn new(session_id: SessionId, duration: Duration) -> Self {
        Self { session_id, duration, ends_at: Instant::now() + time_until_next_session(duration) }
    }
}

/// A stream that returns every time there is a new session 
pub struct Sessions {
    last_session: SessionId,
    session_duration: Duration,
    until_next_session: Option<Delay>,
}

impl Sessions {
    pub fn new(session_duration: Duration) -> Self {
        let session_id = SessionId::get().expect("Failed to get session id");
        // Just in case 
        assert!(session_id.is_initial(), "Session id is not initial");
        Self {
            last_session: session_id,
            session_duration,
            until_next_session: None, 
        }
    }
    
    /// Simple function that returns the next session info if any
    /// First time this function is called, it will wait until the next session starts and update `until_next_session` with the `Delay` at that time
    /// After that, 
    /// For example, if session length is 2 seconds and current time is 09:00:00, it will wait until 09:00:02 and `until_next_session` will be updated which will wait until 09:00:04
    /// Reason for this is to make sure all nodes start at the same time.
    /// Then, when `next_session` is called, all nodes will start session at the same time 
    pub async fn next_session(&mut self) -> SessionInfo {
        loop {
            // Wait for the next session
            self.until_next_session
                .take()
                .unwrap_or_else(|| {
                    // Delay for the first
                    let wait_dur = time_until_next_session(self.session_duration);
                    Delay::new(wait_dur)
                })
                .await;
            let wait_dur = time_until_next_session(self.session_duration);
            // Delay from `now` to the next session
            self.until_next_session = Some(Delay::new(wait_dur));
            let current_session = match SessionId::get() {
                Ok(session_id) => session_id,
                Err(_) => { tracing::error!("Error getting session id"); continue; }
            };
            tracing::info!("Current session: {:?}", current_session);
            if !current_session.is_initial() {
                if current_session > self.last_session {
                    self.last_session = current_session;
                    break SessionInfo::new(current_session, self.session_duration); 
                } else {
                    // Should never reach here
                    panic!("Session not updated? {:?}", current_session);
                }
            } else {
                break SessionInfo::new(current_session, self.session_duration); 
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionWorkerState {
    /// The session is not started yet
    Init, 
    /// The session has started
    Start(SessionId),
    /// The session has ended
    End(SessionId),
}

#[async_trait::async_trait]
pub trait SessionWorker<C: Config> {
    /// Handle the genesis session
    async fn on_genesis_session(&mut self, ctx: &C) -> Result<(), C::Error>;
    /// Handle for every next session
    async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Option<SessionResult<C::Signature>>;
}