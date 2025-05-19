use dkg_primitives::AppState;
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

pub trait DkgService {
    
    /// Type of error that this service builder can produce
    type Error: std::error::Error + Send + Sync;
    type AppState: AppState;

    /// Setup the SKDE params
    fn setup_skde_params(&self, path: PathBuf) -> Result<(), Self::Error>;
    
    /// Fetch the SKDE params
    fn fetch_skde_params(&self) -> Result<SkdeParams, Self::Error>;

    /// Fetch the key generator list
    fn fetch_key_generator_list(&self) -> Result<(), Self::Error>;  

    fn full_context(&self) -> Self::AppState;

    fn rpc_handle(&self) -> Result<(), Self::Error>;
}