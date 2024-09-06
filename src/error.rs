#[derive(Debug)]
pub enum Error {
    Database(radius_sequencer_sdk::kvstore::KvStoreError),
    RpcError(radius_sequencer_sdk::json_rpc::Error),

    LoadConfigOption,
    ParseTomlString,
    RemoveConfigDirectory,
    CreateConfigDirectory,
    CreateConfigFile,
    CreatePrivateKeyFile,

    NotFound,
}

unsafe impl Send for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

impl From<radius_sequencer_sdk::json_rpc::Error> for Error {
    fn from(value: radius_sequencer_sdk::json_rpc::Error) -> Self {
        Self::RpcError(value)
    }
}
