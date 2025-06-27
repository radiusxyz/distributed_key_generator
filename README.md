# Distributed Key Generation

:warning: Under Construction

> This crate is actively being developed. Breaking changes will occur until mainnet when we will start [Semantic Versioning](https://semver.org/).

Distributed key-generator of [Radius Block Building Solution](https://github.com/radiusxyz/radius-docs-bbs/blob/main/docs/radius_block_building_solution.md) written in Rust programming language.

Distributed Key Generator is an implementation of aggregated encryption/decryption mechanism which generates both keys at a regular interval. At each interval, an encryption key and its ID pair is generated and made accessible to other entities such as Secure RPC. A decryption key is generated only after a certain amount of time and can be accessed using the corresponding encryption key ID.

## How to run

1. Build

```
cargo build -release
```

2. Create `skde` params

```
./target/release/dkg trusted-setup skde \
--skde.generator 4 \
--skde.time 1258291 \
--skde.max-sequencer 2
```

3. Store `skde` params on contract

For test use `anvil` or other testnet. `trusted-address` is the contract address.

```
./target/release/dkg node \
--dkg.role authority \
--dkg.trusted-address 0x5FbDB2315678afecb367f032d93F642f64180aa3 \
--internal.port 7102 \
--external.port 7202 \
--cluster.port 7302
```

4. Run node

- Committee

  - Register as `Committee`(e.g smart contract)

  - Run node

    ```bash
    ./target/release/dkg node \
    --dkg.role committee \
    --dkg.trusted-address 0x5FbDB2315678afecb367f032d93F642f64180aa3 \
    --internal.port 7100 \
    --external.port 7200 \
    --cluster.port 7300
    ```

- Solver

  - `Solver` should be registered by admin

  - Run node

    ```bash
    ./target/release/dkg node \
    --dkg.role solver \
    --dkg.trusted-address 0x5FbDB2315678afecb367f032d93F642f64180aa3 \
    --internal.port 8100 \
    --external.port 8200 \
    --cluster.port 8300
    ```

## Running the DKG tests

Execute the tests in single‑threaded mode:

```bash
cargo test -- --test-threads=1
```

If the tests don't exit, run the following command to kill all related processes:
`pkill -f key-generator`

## Contributing

We appreciate your contributions to our project. Visit [issues](https://github.com/radiusxyz/distributed_key_generation/issues) page to start with or refer to the [Contributing guide](https://github.com/radiusxyz/radius-docs-bbs/blob/main/docs/contributing_guide.md).

## Getting Help

Our developers are willing to answer your questions. If you are first and bewildered, refer to the [Getting Help](https://github.com/radiusxyz/radius-docs-bbs/blob/main/docs/getting_help.md) page.
