use std::marker::PhantomData;

mod traits;
mod types;
mod consensus;

pub use traits::*;
pub use types::*;
pub use consensus::*;

pub type RuntimeResult<T> = Result<T, RuntimeError>;

pub type TrustedSetupFor<C> = <<C as Config>::KeyService as KeyService>::TrustedSetUp;
pub type AuthTrustedSetupFor<C> = <<C as Config>::AuthService as AuthService<<C as Config>::Address>>::TrustedSetup;
pub type AuthServiceErrorFor<C> = <<C as Config>::AuthService as AuthService<<C as Config>::Address>>::Error;
pub type KeyServiceErrorFor<C> = <<C as Config>::KeyService as KeyService>::Error;
pub type DbManagerErrorFor<C> = <<C as Config>::DbManager as DbManager<<C as Config>::Address>>::Error;
pub type ConfigErrorFor<C> = <C as Config>::Error;
pub type SignatureFor<C> = <C as Config>::Signature;
pub type AddressFor<C> = <C as Config>::Address;

pub struct Either<A, B>(PhantomData<(A, B)>);
