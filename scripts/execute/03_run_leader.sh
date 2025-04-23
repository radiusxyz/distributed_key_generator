#!/bin/bash

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
NODE1_DATA_DIR="$DATA_DIR/node1"
mkdir -p "$NODE1_DATA_DIR"
chmod -R 755 "$DATA_DIR" "$NODE1_DATA_DIR" 2>/dev/null

# Create Node 1's config file (Leader)
cat > "$NODE1_DATA_DIR/Config.toml" << EOL
# NODE CONFIG: Leader Node (Node 1)
# Change the following values as needed:
external_rpc_url = "http://127.0.0.1:7100"
internal_rpc_url = "http://127.0.0.1:7200"
cluster_rpc_url = "http://127.0.0.1:7300"
solver_rpc_url = "http://127.0.0.1:8400"
solver_solver_rpc_url = "http://127.0.0.1:8500"

role = "leader"
radius_foundation_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
authority_rpc_url = "http://127.0.0.1:6000"
chain_type = "ethereum"
partial_key_generation_cycle_ms = 500
partial_key_aggregation_cycle_ms = 500
EOL

# Set private key (Account 0)
echo -n "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" > "$NODE1_DATA_DIR/signing_key"

# Copy and run binary
cp -f "$BINARY_PATH" "$NODE1_DATA_DIR/key-generator"
chmod 755 "$NODE1_DATA_DIR/key-generator"
cd "$NODE1_DATA_DIR" && "./key-generator" start --path "." 