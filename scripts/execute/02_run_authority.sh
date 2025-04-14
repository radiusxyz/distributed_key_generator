#!/bin/bash

# Get project paths
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT_PATH="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Build authority_node binary if needed
BINARY_PATH="$PROJECT_ROOT_PATH/target/release/authority_node"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Building authority_node binary..."
    cd "$PROJECT_ROOT_PATH" && cargo build --release --bin authority_node

    if [ ! -f "$BINARY_PATH" ]; then
        echo "Error: Failed to build authority_node binary"
        exit 1
    fi
fi

# Setup directories
DATA_DIR="$PROJECT_ROOT_PATH/data"
AUTHORITY_DATA_DIR="$DATA_DIR/authority"
mkdir -p "$AUTHORITY_DATA_DIR"
chmod -R 755 "$DATA_DIR" "$AUTHORITY_DATA_DIR" 2>/dev/null

# Create Authority Node config file (including dummy RPCs to satisfy Config::load)
cat > "$AUTHORITY_DATA_DIR/Config.toml" << EOL
# NODE CONFIG: Authority Node
authority_rpc_url = "http://127.0.0.1:7400"
role = "authority"

# These are not used in authority, but required for Config::load
external_rpc_url = "http://127.0.0.1:3000"
internal_rpc_url = "http://127.0.0.1:4000"
cluster_rpc_url = "http://127.0.0.1:5000"

radius_foundation_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
chain_type = "ethereum"
partial_key_generation_cycle = 5
partial_key_aggregation_cycle = 4
EOL

# Set private key (if not exists)
SIGNING_KEY_PATH="$AUTHORITY_DATA_DIR/signing_key"
if [ ! -f "$SIGNING_KEY_PATH" ]; then
    echo "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" > "$SIGNING_KEY_PATH"
    echo "Default signing key generated."
fi

# Copy and run binary
cp -f "$BINARY_PATH" "$AUTHORITY_DATA_DIR/authority_node"
chmod 755 "$AUTHORITY_DATA_DIR/authority_node"
cd "$AUTHORITY_DATA_DIR" && ./authority_node start --path .
