# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

- **Build**: `cargo build --release`
- **Debug Build**: `cargo build` (creates binary at `target/debug/dkg`)
- **Test**: `cargo test -- --test-threads=1` (single-threaded required for DKG tests)
- **Format**: `cargo fmt` (uses custom formatting rules in `rustfmt.toml`)
- **Lint**: `cargo clippy`
- **Kill hanging processes**: `pkill -f key-generator` (if tests don't exit properly)

### Running Individual Tests

Use specific test file names or test functions:
```bash
cargo test integration::run_single_node_for_each_role -- --test-threads=1
```

## Architecture Overview

This is a Distributed Key Generator for the Radius Block Building Solution. The system implements aggregated encryption/decryption with timed key generation.

### Workspace Structure

- **cli/**: Command-line interface and configuration management
- **node/**: Core node implementation
  - **primitives/**: Node configuration, auth, and key services
  - **service/**: Main service logic with task workers (committee, solver)
- **primitives/**: Core types, traits, and consensus mechanisms
- **rpc/**: RPC handlers for cluster and external communication
- **utils/**: Shared utilities

### Key Components

#### Node Roles
- **Authority**: Constructs trusted setup parameters
- **Committee**: Generates encryption keys and acts as leader
- **Solver**: Computes decryption keys
- **Verifier**: Monitors network for Byzantine behavior

#### Configuration Priority
Configuration values follow this priority order:
1. CLI arguments (highest priority)
2. TOML config file values (`--config` or `config.toml`)
3. DEFAULT constants (fallback)

### Running Nodes

1. **Create trusted setup**:
   ```bash
   ./target/release/dkg trusted-setup skde --skde.generator 4 --skde.time 1258291 --skde.max-sequencer 2
   ```

2. **Run Authority node**:
   ```bash
   ./target/release/dkg node --dkg.role authority --dkg.trusted-address 0x5FbDB2315678afecb367f032d93F642f64180aa3
   ```

3. **Run Committee node**:
   ```bash
   ./target/release/dkg node --dkg.role committee --dkg.trusted-address 0x5FbDB2315678afecb367f032d93F642f64180aa3 --internal.port 7100 --external.port 7200 --cluster.port 7300
   ```

4. **Run Solver node**:
   ```bash
   ./target/release/dkg node --dkg.role solver --dkg.trusted-address 0x5FbDB2315678afecb367f032d93F642f64180aa3 --internal.port 8100 --external.port 8200 --cluster.port 8300
   ```

### Development Scripts

The `scripts/execute/` directory contains automation scripts:
- **00_cleanup_nodes.sh**: Clean up processes and data directories
- **01_run_all_nodes.sh**: Start all nodes in correct sequence
- **02_run_authority.sh** through **05_run_solver.sh**: Individual node startup
- **06_register_nodes.sh**: Register nodes with each other

### Key Implementation Details

- **SKDE Integration**: Uses Shared Key Derivation Extension for cryptographic operations
- **RPC Architecture**: Three-tier RPC system (external, internal, cluster)  
- **Task Workers**: Asynchronous task processing for committee and solver operations
- **Consensus**: Custom consensus mechanism with commitment and payload types
- **Database**: RocksDB for persistent storage (stored in `./tmp/{role}/db/`)
- **Binary Output**: `cargo build --release` produces `./target/release/dkg` executable

### Development Notes

- **Rust Toolchain**: Uses nightly-2024-10-24 (see `rust-toolchain`)
- **Tests**: Require single-threaded execution due to shared resources
- **Configuration**: Loading uses `as_ref()` and `clone()` to avoid move issues
- **RPC Endpoints**: All have configurable URLs with sensible defaults
- **Storage**: Node data stored under `./tmp/{role}/db/` using RocksDB
- **Private Keys**: Test private keys defined in `tests/utils.rs:38-49`
- **Formatting**: Uses custom rustfmt.toml with StdExternalCrate grouping and crate-level import granularity
- **Dependencies**: External dependencies include radius-sdk, skde (from GitHub), and standard async/crypto libraries

### Event System Architecture

The system uses a dual-channel event system with two main event types:
- **SessionEvent**: Committee session management (genesis, timeouts, key submissions)
- **SolverEvent**: Solver-specific events (decryption key submissions)
- **DkgEvent**: Wrapper enum combining both event types

### Configuration System

Configuration follows a three-tier priority system:
1. CLI arguments (highest priority)
2. TOML config file values (`--config` or `config.toml`)  
3. DEFAULT constants (fallback)

Key configuration files are generated per role in `tmp/{role}/config.toml`

### Testing Infrastructure

- **Test Utilities**: Comprehensive test helpers in `tests/utils.rs`
- **Integration Tests**: Located in `tests/integration/` directory
- **Port Allocation**: Tests use predictable port ranges (7100+ internal, 7200+ external, 7300+ cluster)
- **Process Management**: Automatic cleanup and process spawning for multi-node tests
- **Process Cleanup**: Use `pkill -f key-generator` if tests hang or don't exit properly

### Workspace Structure Details

The project uses a Cargo workspace with the following key crates:
- **src/**: Main binary crate producing the `dkg` executable
- **node/operator/**: Blockchain operator integration for Radius network
- **node/key_generator/skde/**: SKDE-specific key generation implementation
- **External Dependencies**: Uses radius-sdk from local path, SKDE from GitHub commit, and ethers for blockchain interaction