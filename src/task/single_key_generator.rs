use std::{str::FromStr, sync::Arc, time::Duration};

use skde::{
    key_generation::{
        generate_partial_key, prove_partial_key_validity, PartialKey, PartialKeyProof,
    },
    setup, BigUint,
};
use tokio::time::sleep;

use crate::{rpc::cluster::SyncPartialKey, state::AppState, types::Address};

pub const PRIME_P: &str = "8155133734070055735139271277173718200941522166153710213522626777763679009805792017274916613411023848268056376687809186180768200590914945958831360737612803";
pub const PRIME_Q: &str = "13379153270147861840625872456862185586039997603014979833900847304743997773803109864546170215161716700184487787472783869920830925415022501258643369350348243";
pub const GENERATOR: &str = "4";
pub const TIME_PARAM_T: u32 = 2;
pub const MAX_KEY_GENERATOR_NUMBER: u32 = 2;

pub fn run_single_key_generator(context: Arc<AppState>, key_id: u64, address: Address) {
    let time = 2_u32.pow(TIME_PARAM_T);
    let p = BigUint::from_str(PRIME_P).expect("Invalid PRIME_P");
    let q = BigUint::from_str(PRIME_Q).expect("Invalid PRIME_Q");
    let g = BigUint::from_str(GENERATOR).expect("Invalid GENERATOR");
    let max_key_generator_number = BigUint::from(MAX_KEY_GENERATOR_NUMBER);

    let skde_params = setup(time, p, q, g, max_key_generator_number);

    tokio::spawn(async move {
        // TODO
        sleep(Duration::from_secs(2)).await;

        let (secret_value, partial_key) = generate_partial_key(&skde_params);
        let partial_key_proof = prove_partial_key_validity(&skde_params, &secret_value);

        sync_partial_key(address.clone(), partial_key, partial_key_proof);
    });
}

pub fn sync_partial_key(
    address: Address,
    partial_key: PartialKey,
    partial_key_proof: PartialKeyProof,
) {
    tokio::spawn(async move {
        let parameter = SyncPartialKey {
            address,
            partial_key,
            partial_key_proof,
        };

        // let key_generator_rpc_clients = cluster.key_generator_rpc_clients().await;

        // info!(
        //     "sync_partial_key - rpc_client_count: {:?}",
        //     key_generator_rpc_clients.len()
        // );

        // for key_generator_rpc_client in key_generator_rpc_clients {
        //     let key_generator_rpc_client = key_generator_rpc_client.clone();
        //     let parameter = parameter.clone();

        //     tokio::spawn(async move {
        //         match key_generator_rpc_client.sync_partial_key(parameter).await {
        //             Ok(_) => {
        //                 info!("Complete to sync partial key");
        //             }
        //             Err(err) => {
        //                 info!("Failed to sync partial key - error: {:?}", err);
        //             }
        //         }
        //     });
        // }
    });
}
