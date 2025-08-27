use std::marker::PhantomData;

mod consensus;
mod traits;
mod types;

pub use consensus::*;
pub use traits::*;
pub use types::*;

/// Overarching result type for the runtime
pub type RuntimeResult<T> = Result<T, RuntimeError>;

pub type ValidationServiceFor<C> = <C as Config>::ValidationService; 

/// Type of error that the key generator returns
pub type KeyGeneratorErrorFor<C> = <<C as Config>::KeyGenerator as KeyGenerator>::Error;
/// Type of error that the db manager returns
pub type DbManagerErrorFor<C> =
    <<C as Config>::DbManager as DbManager<<C as Config>::Address>>::Error;
/// Type of error that the config returns
pub type ConfigErrorFor<C> = <C as Config>::Error;
/// Type of signature that the config uses
pub type SignatureFor<C> = <C as Config>::Signature;
/// Type of address that the config uses
pub type AddressFor<C> = <C as Config>::Address;

/// Infallible type of either a or b
pub struct Either<A, B>(PhantomData<(A, B)>);
