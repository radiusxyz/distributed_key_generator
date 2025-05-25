use crate::Args;
use dirs::home_dir;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct DataDirArgs {
    #[arg(long = "datadir.db")]
    pub db_path: Option<PathBuf>,
    #[arg(long = "datadir.private-key")]
    pub private_key: Option<PathBuf>,
    #[arg(long = "datadir.trusted-setup")]
    pub trusted_setup: Option<PathBuf>,
}

impl Default for DataDirArgs {
    fn default() -> Self {
        let home = home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home.join(".dkg");

        Self {
            db_path: Some(config_dir.join("db")),
            private_key: Some(config_dir.join("private_key")),
            trusted_setup: Some(config_dir.join("trusted_setup")),
        }
    }
}
