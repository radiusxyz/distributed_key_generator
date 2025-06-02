use std::marker::PhantomData;

mod traits;
mod types;
mod consensus;

pub use traits::*;
pub use types::*;
pub use consensus::*;

pub type TrustedSetupFor<C> = <<C as AppState>::SecureBlock as SecureBlock>::TrustedSetUp;
pub type AuthServiceErrorFor<C> = <<C as AppState>::AuthService as AuthService<<C as AppState>::Address>>::Error;
pub type SecureBlockErrorFor<C> = <<C as AppState>::SecureBlock as SecureBlock>::Error;

pub struct Either<A, B>(PhantomData<(A, B)>);
