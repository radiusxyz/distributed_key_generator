use skde::delay_encryption::SkdeParams;

use crate::Config;

pub trait AppState: Clone + Send + Sync {
    /// Configuration for this app
    fn config(&self) -> &Config;
    /// Get pre-set skde parameter
    fn skde_params(&self) -> SkdeParams;
}
