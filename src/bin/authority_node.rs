use std::str::FromStr;

use clap::{Parser, Subcommand};
use distributed_key_generation::{
    config::{Config, ConfigOption, ConfigPath},
    error::Error,
    rpc::authority::GetAuthorizedSkdeParams,
    state::AppState,
};
use radius_sdk::json_rpc::server::RpcServer;
use serde::{Deserialize, Serialize};
use skde::{delay_encryption::SkdeParams, BigUint};

#[derive(Debug, Deserialize, Parser, Serialize)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Deserialize, Serialize)]
pub enum Commands {
    Init {
        #[clap(flatten)]
        config_path: Box<ConfigPath>,
    },

    Start {
        #[clap(flatten)]
        config_option: Box<ConfigOption>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { config_path } => {
            ConfigPath::init(&config_path)?;
        }

        Commands::Start { mut config_option } => {
            let config = Config::load(&mut config_option)?;
            tracing::info!("Loaded config at {:?}", config.path());

            // calculate or load SkdeParams
            let skde_params = calculate_default_skde_params(); // or load from file in future

            let app_state = AppState::new(config.clone(), skde_params);

            let authority_url = config
                .authority_rpc_url()
                .expect("Missing authority_rpc_url in config");

            let rpc_server = RpcServer::new(app_state.clone())
                .register_rpc_method::<GetAuthorizedSkdeParams>()?
                .init(authority_url)
                .await?;

            tracing::info!("Authority RPC server running at {}", authority_url);
            rpc_server.stopped().await;
        }
    }

    Ok(())
}

fn calculate_default_skde_params() -> SkdeParams {
    const MOD_N: &str = "26737688233630987849749538623559587294088037102809480632570023773459222152686633609232230584184543857897813615355225270819491245893096628373370101798393754657209853664433779631579690734503677773804892912774381357280025811519740953667880409246987453978226997595139808445552217486225687511164958368488319372068289768937729234964502681229612929764203977349037219047813560623373035187038018937232123821089208711930458219009895581132844064176371047461419609098259825422421077554570457718558971463292559934623518074946858187287041522976374186587813034651849410990884606427758413847140243755163116582922090226726575253150079";
    const GENERATOR: &str = "4";
    const TIME_PARAM_T: u32 = 2;
    const MAX_KEY_GENERATOR_NUMBER: u32 = 2;

    let n = BigUint::from_str(MOD_N).unwrap();
    let g = BigUint::from_str(GENERATOR).unwrap();
    let t = 2_u32.pow(TIME_PARAM_T);
    let mut h = g.clone();
    (0..t).for_each(|_| h = (&h * &h) % &n);

    SkdeParams {
        t,
        n: n.to_str_radix(10),
        g: g.to_str_radix(10),
        h: h.to_str_radix(10),
        max_sequencer_number: BigUint::from(MAX_KEY_GENERATOR_NUMBER).to_str_radix(10),
    }
}
