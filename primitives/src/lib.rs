mod traits;
mod types;
mod consensus;

use std::marker::PhantomData;

pub use traits::*;
pub use types::*;
pub use consensus::*;

pub type TrustedSetupFor<C> = <<C as AppState>::SecureBlock as SecureBlock>::TrustedSetUp;

pub struct Either<A, B>(PhantomData<(A, B)>);