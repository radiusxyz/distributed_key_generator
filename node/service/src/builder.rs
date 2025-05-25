use dkg_primitives::{AppState, TrustedSetupFor};
use skde::delay_encryption::SkdeParams;
use tokio::runtime::Handle;
use std::path::PathBuf;

pub struct ServiceBuilder<Service: DkgService> {
    service: Service,
    handle: Handle,
}

impl<Service: DkgService> ServiceBuilder<Service> {
    pub fn new(service: Service, handle: Handle) -> Self {
        Self { service, handle }
    }

    pub fn run(&self) {}
}

pub trait DkgService<C: AppState> {
    
    /// Type of error that this service builder can produce
    type Error: std::error::Error + Send + Sync;
    type AppState: AppState;

    /// Setup the trusted setup
    fn trusted_setup(&self, path: PathBuf) -> Result<(), Self::Error>;
    
    /// Fetch the trusted setup
    fn fetch_trusted_setup(&self) -> Result<TrustedSetupFor<C>, Self::Error>;

    /// Fetch the key generator list
    fn fetch_key_generator_list(&self) -> Result<(), Self::Error>;  

    fn full_context(&self) -> Self::AppState;

    fn rpc_handle(&self) -> Result<(), Self::Error>;
}