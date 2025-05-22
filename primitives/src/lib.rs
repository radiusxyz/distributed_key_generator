mod traits;
mod types;

use std::marker::PhantomData;

pub use traits::*;
pub use types::*;

pub struct Either<A, B>(PhantomData<(A, B)>);