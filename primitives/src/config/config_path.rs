use std::{
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use clap::Parser;
use serde::{Deserialize, Serialize};
use skde::{delay_encryption::SkdeParams, BigUint};
use tracing::info;

use crate::{
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

        info!("Config directory at {:?}", self.as_ref());
        Ok(())
    }
    /// Initialize SKDE parameter file with default values if it doesn't exist.
    /// This is only used in authority nodes.
    pub fn init_skde_params_if_missing(&self) {
        let skde_path = self.as_ref().join("skde_params.json");

        // Skip if the file already exists
        if skde_path.exists() {
            return;
        }

        // Generate SKDE parameters
        let default_params = default_skde_params();

        // Serialize to JSON (POC: unwrap used)
        let serialized = serde_json::to_string_pretty(&default_params).unwrap();

        // Write to file (POC: unwrap used)
        fs::write(&skde_path, serialized).unwrap();

        info!("Default SKDE params written to {:?}", skde_path);
    }
}

/// Only for authority node
pub fn default_skde_params() -> SkdeParams {
    const MOD_N: &str = "26737688233630987849749538623559587294088037102809480632570023773459222152686633609232230584184543857897813615355225270819491245893096628373370101798393754657209853664433779631579690734503677773804892912774381357280025811519740953667880409246987453978226997595139808445552217486225687511164958368488319372068289768937729234964502681229612929764203977349037219047813560623373035187038018937232123821089208711930458219009895581132844064176371047461419609098259825422421077554570457718558971463292559934623518074946858187287041522976374186587813034651849410990884606427758413847140243755163116582922090226726575253150079";
    const GENERATOR: &str = "4";
    const TIME_PARAM_T: u32 = 2;
    const MAX_SEQUENCER_NUMBER: u32 = 2;

    let n = BigUint::from_str(MOD_N).expect("Invalid MOD_N");
    let g = BigUint::from_str(GENERATOR).expect("Invalid GENERATOR");
    let max = BigUint::from(MAX_SEQUENCER_NUMBER);
    let t = 2_u32.pow(TIME_PARAM_T);

    let mut h = g.clone();
    (0..t).for_each(|_| {
        h = (&h * &h) % &n;
    });

    SkdeParams {
        t,
        n: n.to_str_radix(10),
        g: g.to_str_radix(10),
        h: h.to_str_radix(10),
        max_sequencer_number: max.to_str_radix(10),
    }
}
