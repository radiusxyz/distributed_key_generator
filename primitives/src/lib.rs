mod traits;
mod types;

use std::marker::PhantomData;

pub use traits::*;
pub use types::*;

pub type EncKeyFor<C> = <<C as AppState>::SecureBlock as SecureBlock>::EncKey;
pub type DecKeyFor<C> = <<C as AppState>::SecureBlock as SecureBlock>::DecKey;

pub struct Either<A, B>(PhantomData<(A, B)>);