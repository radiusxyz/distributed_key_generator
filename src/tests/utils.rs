use std::str::FromStr;

use radius_sdk::signature::{Address, ChainType};
use skde::{delay_encryption::SkdeParams, BigUint};

// Constants for SKDE parameters
const MOD_N: &str = "26737688233630987849749538623559587294088037102809480632570023773459222152686633609232230584184543857897813615355225270819491245893096628373370101798393754657209853664433779631579690734503677773804892912774381357280025811519740953667880409246987453978226997595139808445552217486225687511164958368488319372068289768937729234964502681229612929764203977349037219047813560623373035187038018937232123821089208711930458219009895581132844064176371047461419609098259825422421077554570457718558971463292559934623518074946858187287041522976374186587813034651849410990884606427758413847140243755163116582922090226726575253150079";
const GENERATOR: &str = "4";
const TIME_PARAM_T: u32 = 2;
const MAX_KEY_GENERATOR_NUMBER: u32 = 2;

/// Creates SKDE parameters for testing purposes
pub fn create_skde_params() -> SkdeParams {
    let n = BigUint::from_str(MOD_N).expect("Invalid MOD_N");
    let g = BigUint::from_str(GENERATOR).expect("Invalid GENERATOR");
    let max_key_generator_number = BigUint::from(MAX_KEY_GENERATOR_NUMBER);
    let t = 2_u32.pow(TIME_PARAM_T);
    let mut h = g.clone();
    (0..t).for_each(|_| {
        h = (&h * &h) % n.clone();
    });

    SkdeParams {
        t,
        n: n.to_str_radix(10),
        g: g.to_str_radix(10),
        h: h.to_str_radix(10),
        max_sequencer_number: max_key_generator_number.to_str_radix(10),
    }
}

/// Creates a test Ethereum address
pub fn create_test_address(address_str: &str) -> Address {
    Address::from_str(ChainType::Ethereum, address_str).unwrap()
}

/// Port definitions for testing
pub struct TestPorts {
    pub cluster: u16,
    pub external: u16,
    pub internal: u16,
}

/// Predefined test ports for common test scenarios
pub struct TestPortConfig {
    pub leader: TestPorts,
    pub committee: TestPorts,
    pub solver: TestPorts,
    // pub verifier: TestPorts,
}

impl Default for TestPortConfig {
    fn default() -> Self {
        Self {
            leader: TestPorts {
                cluster: 7001,
                external: 7002,
                internal: 7003,
            },
            committee: TestPorts {
                cluster: 8001,
                external: 8002,
                internal: 8003,
            },
            solver: TestPorts {
                cluster: 9001,
                external: 9002,
                internal: 9003,
            },
        }
    }
}

/// Setup tracing for tests
pub fn setup_test_logging() {
    use tracing::Level;
    use tracing_subscriber::fmt;

    let _ = fmt()
        .with_max_level(Level::INFO)
        .with_test_writer()
        .try_init();
}
