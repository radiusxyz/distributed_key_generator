use crate::KeyServiceError;

mod skde;
pub use skde::*;

pub type KeyServiceResult<T> = Result<T, KeyServiceError>;