#!/bin/bash

# Build project in release mode
cargo b -r

# Get project paths
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT_PATH="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Build binary if needed
BINARY_PATH="$PROJECT_ROOT_PATH/target/release/key-generator"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Building key-generator binary..."
    cd "$PROJECT_ROOT_PATH" && cargo build --release

    if [ ! -f "$BINARY_PATH" ]; then
        echo "Error: Failed to build key-generator binary"
        exit 1
    fi
fi

# Setup directories
DATA_DIR="$PROJECT_ROOT_PATH/data"
NODE_DATA_DIR="$DATA_DIR/solver"
mkdir -p "$NODE_DATA_DIR"
chmod -R 755 "$DATA_DIR" "$NODE_DATA_DIR" 2>/dev/null

# Create Solver's config file (Solver)
cat > "$NODE_DATA_DIR/Config.toml" << EOL
# NODE CONFIG: Solver Node
external_rpc_url = "http://127.0.0.1:8100"
internal_rpc_url = "http://127.0.0.1:8200"
cluster_rpc_url = "http://127.0.0.1:8300"
# The endpoint of the leader rpc server for the solver: Solver -> Leader
solver_rpc_url = "http://127.0.0.1:8500"
# The endpoint of the leader rpc server for the solver: Leader -> Solver
leader_solver_rpc_url = "http://127.0.0.1:8400"

role = "solver"
radius_foundation_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
authority_rpc_url = "http://127.0.0.1:6000"
chain_type = "ethereum"
session_cycle = 1500
EOL

# Set private key
echo -n "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a" > "$NODE_DATA_DIR/signing_key"

# Copy and run binary
cp -f "$BINARY_PATH" "$NODE_DATA_DIR/key-generator"
chmod 755 "$NODE_DATA_DIR/key-generator"
cd "$NODE_DATA_DIR" && "./key-generator" start --path "."