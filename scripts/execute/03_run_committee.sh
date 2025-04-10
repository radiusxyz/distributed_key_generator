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
NODE2_DATA_DIR="$DATA_DIR/node2"
mkdir -p "$NODE2_DATA_DIR"
chmod -R 755 "$DATA_DIR" "$NODE2_DATA_DIR" 2>/dev/null

# Create Node 2's config file (Committee)
cat > "$NODE2_DATA_DIR/Config.toml" << EOL
# NODE CONFIG: Committee Node (Node 2)
# Change the following values as needed:
external_rpc_url = "http://127.0.0.1:7101"
internal_rpc_url = "http://127.0.0.1:7201"
cluster_rpc_url = "http://127.0.0.1:7301"
leader_cluster_rpc_url = "http://127.0.0.1:7300"
role = "committee"
radius_foundation_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
chain_type = "ethereum"
partial_key_generation_cycle = 5
partial_key_aggregation_cycle = 4
EOL

# Set private key (Account 1)
echo -n "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" > "$NODE2_DATA_DIR/signing_key"

# Copy and run binary
cp -f "$BINARY_PATH" "$NODE2_DATA_DIR/key-generator"
chmod 755 "$NODE2_DATA_DIR/key-generator"
cd "$NODE2_DATA_DIR" && "./key-generator" start --path "." 