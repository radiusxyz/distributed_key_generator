#!/bin/bash

# Get project paths
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT_PATH="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Build key-generator binary if needed
BINARY_PATH="$PROJECT_ROOT_PATH/target/release/key-generator"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Building key-generator binary..."
    cd "$PROJECT_ROOT_PATH" && cargo build --release --bin key-generator

    if [ ! -f "$BINARY_PATH" ]; then
        echo "Error: Failed to build key-generator binary"
        exit 1
    fi
fi

# Setup directories
DATA_DIR="$PROJECT_ROOT_PATH/data"
AUTHORITY_DATA_DIR="$DATA_DIR/authority"
mkdir -p "$AUTHORITY_DATA_DIR"
chmod -R 755 "$DATA_DIR" "$AUTHORITY_DATA_DIR" 2>/dev/null

# Create Authority Node config file
cat > "$AUTHORITY_DATA_DIR/Config.toml" << EOL
# NODE CONFIG: Authority Node
authority_rpc_url = "http://127.0.0.1:6000"
role = "authority"

# The following are unused by authority, but required for Config::load
external_rpc_url = "http://127.0.0.1:7102"
internal_rpc_url = "http://127.0.0.1:7202"
cluster_rpc_url = "http://127.0.0.1:7302"
solver_rpc_url = "http://127.0.0.1:8500"
radius_foundation_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
chain_type = "ethereum"
partial_key_generation_cycle_ms = 1500
partial_key_aggregation_cycle_ms = 1500
EOL

# Set private key (if not exists)
SIGNING_KEY_PATH="$AUTHORITY_DATA_DIR/signing_key"
if [ ! -f "$SIGNING_KEY_PATH" ]; then
    echo "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" > "$SIGNING_KEY_PATH"
    echo "Default signing key generated."
fi

# Copy binary to authority directory
cp -f "$BINARY_PATH" "$AUTHORITY_DATA_DIR/key-generator"
chmod 755 "$AUTHORITY_DATA_DIR/key-generator"

# Run setup-skde-params to generate skde_params.json
cd "$AUTHORITY_DATA_DIR"
./key-generator setup-skde-params --path .

# Start authority node
./key-generator start --path .