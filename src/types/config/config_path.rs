use std::{
    env, fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::types::{
    config::{config_option::ConfigOption, ConfigError},
    CONFIG_FILE_NAME, SIGNING_KEY,
};

#[derive(Debug, Deserialize, Parser, Serialize)]
pub struct ConfigPath {
    #[doc = "Set the key generator configuration path"]
    #[clap(long = "path", default_value_t = Self::default().to_string())]
    path: String,
}

impl std::fmt::Display for ConfigPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl AsRef<Path> for ConfigPath {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Default for ConfigPath {
    fn default() -> Self {
        // 명령줄에서 --path 인자 찾기
        let args: Vec<String> = std::env::args().collect();
        let path_arg = args.iter().enumerate().find_map(|(i, arg)| {
            if arg == "--path" && i + 1 < args.len() {
                Some(args[i + 1].clone())
            } else if arg.starts_with("--path=") {
                Some(arg.trim_start_matches("--path=").to_string())
            } else {
                None
            }
        });

        // 찾은 인자 또는 환경 변수 사용
        let path = match path_arg {
            Some(p) => p,
            None => match std::env::var("RADIUS_NODE_PATH") {
                Ok(env_path) => env_path,
                Err(_) => {
                    // 현재 디렉토리 확인
                    let current_dir = std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .to_string_lossy()
                        .to_string();

                    // 첫 번째로 확인: 현재 디렉토리/data/node1
                    let node1_path = PathBuf::from(&current_dir).join("data").join("node1");

                    if node1_path.exists() && node1_path.join(CONFIG_FILE_NAME).exists() {
                        node1_path.to_string_lossy().to_string()
                    } else {
                        // 기본 경로 사용
                        PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                            .join(super::DEFAULT_HOME_PATH)
                            .to_string_lossy()
                            .to_string()
                    }
                }
            },
        };

        tracing::info!("Using config path: {}", path);
        Self { path }
    }
}

impl ConfigPath {
    pub fn init(&self) -> Result<(), ConfigError> {
        // 디렉토리가 없으면 생성, 있으면 유지
        if !self.as_ref().exists() {
            fs::create_dir_all(self).map_err(ConfigError::CreateConfigDirectory)?;
        }

        // 설정 파일이 없으면 생성
        let config_file_path = self.as_ref().join(CONFIG_FILE_NAME);
        if !config_file_path.exists() {
            let config_toml_string = ConfigOption::default().get_toml_string();
            fs::write(config_file_path, config_toml_string)
                .map_err(ConfigError::CreateConfigFile)?;
        }

        // 서명 키가 없으면 생성
        let signing_key_path = self.as_ref().join(SIGNING_KEY);
        if !signing_key_path.exists() {
            // 기본 서명 키
            let signing_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
            fs::write(signing_key_path, signing_key).map_err(ConfigError::CreatePrivateKeyFile)?;
        }

        tracing::info!("Config directory at {:?}", self.as_ref());
        Ok(())
    }
}
