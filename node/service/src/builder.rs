use dkg_primitives::{Config, TrustedSetupFor};
use dkg_node_primitives::Role;
use tokio::runtime::Handle;
use std::{path::PathBuf, marker::PhantomData};

pub struct ServiceBuilder<S: DkgService<C>, C: Config> {
    service: S,
    handle: Handle,
    role: Role,
    _phantom: PhantomData<C>,
}

impl<S: DkgService<C>, C: Config> ServiceBuilder<S, C> {
    pub fn new(service: S, handle: Handle, role: Role) -> Self {
        Self { service, handle, role, _phantom: PhantomData }
    }

    pub fn run(&self) {}
}

pub trait DkgService<C: Config> {
    
    /// Type of error that this service builder can produce
    type Error: std::error::Error + Send + Sync;
    type Config: Config;

    /// Setup the trusted setup
    fn trusted_setup(&self, path: PathBuf) -> Result<(), Self::Error>;
    
    /// Fetch the trusted setup
    fn fetch_trusted_setup(&self) -> Result<TrustedSetupFor<C>, Self::Error>;

    /// Fetch the key generator list
    fn fetch_key_generator_list(&self) -> Result<(), Self::Error>;  

    fn full_context(&self) -> Self::Config;

    fn rpc_handle(&self) -> Result<(), Self::Error>;
}