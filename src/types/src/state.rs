use std::sync::Arc;

use crate::types::{Config, Role};
use skde::delay_encryption::SkdeParams;

pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: Config,
    skde_params: SkdeParams,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl AppState {
    pub fn new(config: Config, skde_params: skde::delay_encryption::SkdeParams) -> Self {
        let inner = AppStateInner {
            config,
            skde_params,
        };

        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub fn skde_params(&self) -> SkdeParams {
        self.inner.skde_params.clone()
    }

    // Helper methods for role-based configuration
    pub fn is_leader(&self) -> bool {
        self.config().is_leader()
    }

    pub fn is_committee(&self) -> bool {
        self.config().is_committee()
    }

    pub fn is_solver(&self) -> bool {
        self.config().is_solver()
    }

    pub fn is_verifier(&self) -> bool {
        self.config().is_verifier()
    }

    pub fn role(&self) -> &Role {
        self.config().role()
    }
}
