# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

- **Build**: `cargo build --release`
- **Test**: `cargo test -- --test-threads=1` (single-threaded required for DKG tests)
- **Kill hanging processes**: `pkill -f key-generator` (if tests don't exit properly)

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

3. **Run Committee/Solver nodes**: Similar pattern with appropriate role and port configurations

### Key Implementation Details

- **SKDE Integration**: Uses Shared Key Derivation Extension for cryptographic operations
- **RPC Architecture**: Three-tier RPC system (external, internal, cluster)
- **Task Workers**: Asynchronous task processing for committee and solver operations
- **Consensus**: Custom consensus mechanism with commitment and payload types
- **Database**: RocksDB for persistent storage (stored in `./tmp/{role}/db/`)

### Development Notes

- Tests require single-threaded execution due to shared resources
- Configuration loading uses `as_ref()` and `clone()` to avoid move issues
- All RPC endpoints have configurable URLs with sensible defaults
- Node data is stored under `./tmp/{role}/` directory structure