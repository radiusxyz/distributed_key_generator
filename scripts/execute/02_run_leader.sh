#!/bin/bash

# This script sets up and runs Node 1 (Leader)

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
# Calculate the project root directory path
PROJECT_ROOT_PATH="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Check if key-generator binary exists
BINARY_PATH="$PROJECT_ROOT_PATH/target/release/key-generator"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: key-generator binary not found at $BINARY_PATH"
    echo "Building the binary with cargo build --release..."
    # Try to build if binary doesn't exist
    cd "$PROJECT_ROOT_PATH" && cargo build --release
    
    if [ ! -f "$BINARY_PATH" ]; then
        echo "Error: Failed to build key-generator binary"
        exit 1
    fi
fi

# Setup data directory
echo "Setting up data directory..."
DATA_DIR="$PROJECT_ROOT_PATH/data"
mkdir -p "$DATA_DIR"
chmod -R 755 "$DATA_DIR" || echo "Warning: Could not change permissions on $DATA_DIR"

# Setup Node 1 (Leader)
echo "Setting up Node 1 (Leader)..."
NODE1_DATA_DIR="$DATA_DIR/node1"
mkdir -p "$NODE1_DATA_DIR"
chmod -R 755 "$NODE1_DATA_DIR" || echo "Warning: Could not change permissions on $NODE1_DATA_DIR"

# Create Node 1's config file (Config.toml)
cat > "$NODE1_DATA_DIR/Config.toml" << EOL
# External RPC URL (endpoint for communicating with external clients)
external_rpc_url = "http://127.0.0.1:7100"

# Internal RPC URL (endpoint for node management)
internal_rpc_url = "http://127.0.0.1:7200"

# Cluster RPC URL (endpoint for node-to-node communication)
cluster_rpc_url = "http://127.0.0.1:7300"

# Leader cluster RPC URL (not needed for leader node)
# leader_cluster_rpc_url = "http://127.0.0.1:7300"

# Node role (leader, committee, solver, verifier)
role = "leader"

# Radius Foundation address
radius_foundation_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

# Chain type (for verifying foundation address signature)
chain_type = "ethereum"

# Partial key generation cycle
partial_key_generation_cycle = 5

# Partial key aggregation cycle
partial_key_aggregation_cycle = 4
EOL

# Set Node 1's private key (Account 0)
NODE1_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
# Save to file without newline (newline causes errors)
echo -n "$NODE1_PRIVATE_KEY" > "$NODE1_DATA_DIR/signing_key"
echo "Node 1 private key set to Account 0"

# Copy binary to Node 1
echo "Copying binary to Node 1..."
/bin/cp -f "$BINARY_PATH" "$NODE1_DATA_DIR/key-generator"
if [ ! -f "$NODE1_DATA_DIR/key-generator" ]; then
    echo "Error: Failed to copy binary to Node 1"
    exit 1
fi
chmod 755 "$NODE1_DATA_DIR/key-generator"

# Start Node 1 (Leader)
echo "Starting Node 1 (Leader)..."
# Change to Node 1's directory and run the binary
cd "$NODE1_DATA_DIR"
# Start key-generator with current directory ('.') as the config path
"./key-generator" start --path "."

# This message will only be shown if the binary exits (which it normally doesn't)
echo "Node 1 has been started." 