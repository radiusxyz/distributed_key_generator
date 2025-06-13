use super::*;
use dkg_primitives::{SessionId, KeyGenerator, AuthService};
use dkg_utils::timestamp;
use std::{marker::PhantomData, time::{Duration, Instant}};
use futures_timer::Delay;

pub struct SessionResult<Signature>(PhantomData<Signature>);

impl<Signature> SessionResult<Signature> {
    pub fn new() -> Self {
        Self(Default::default())
    }
}

/// Run the genesis session. Session will be started by the leader
pub async fn run_genesis_session<C: Config>(ctx: &C, current_round: u64, threshold: u16, key_generators: Vec<KeyGenerator<C::Address>>) -> Result<(), C::Error> {
    if ctx.is_solver() { return Ok(()); }
    let session_id = SessionId::get().expect("Not initialized"); 
    if current_round != 0 { panic!("Current round should be 0"); }
    if !session_id.is_initial() { panic!("Session id is not initial"); }
    loop {
        if ctx.auth_service().is_ready(current_round, threshold).await.unwrap() {
            break;
        }
    }
    committee::init(ctx, key_generators, session_id); 
    Ok(())
}

pub async fn run_session_worker<C, SW>(ctx: &C, worker: &mut SW, session_duration: Duration) -> Result<(), C::Error> 
where
    C: Config,
    SW: SessionWorker<C>
{
    let mut sessions = Sessions::new(session_duration);
    loop {
        let session_info = sessions.next_session().await;
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
            if self.last_session.is_initial() {
                // We don't update the last session for the initial session
                break SessionInfo::new(self.last_session, self.session_duration);
            } else {
                let current_session = match SessionId::get() {
                    Ok(session_id) => session_id,
                    Err(_) => { tracing::error!("Error getting session id"); continue; }
                };
                // Current session should be greater than the last session
                if current_session > self.last_session {
                    self.last_session = current_session;
                    break SessionInfo::new(current_session, self.session_duration);
                }
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
    /// Handle for every next session
    async fn on_session(&mut self, ctx: &C, session_info: SessionInfo) -> Option<SessionResult<C::Signature>>;
}