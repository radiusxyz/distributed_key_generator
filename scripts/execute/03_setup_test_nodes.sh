#!/bin/bash

# This script sets up two test nodes (Node 1 as Leader and Node 2 as Committee)

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT_PATH="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Check if key-generator binary exists
BINARY_PATH="$PROJECT_ROOT_PATH/target/release/key-generator"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: key-generator binary not found at $BINARY_PATH"
    echo "Building the binary with cargo build --release..."
    cd "$PROJECT_ROOT_PATH" && cargo build --release
    
    if [ ! -f "$BINARY_PATH" ]; then
        echo "Error: Failed to build key-generator binary"
        exit 1
    fi
fi

# Set up data directory
echo "Setting up data directory..."
DATA_DIR="$PROJECT_ROOT_PATH/data"
mkdir -p "$DATA_DIR"
chmod -R 755 "$DATA_DIR" || echo "Warning: Could not change permissions on $DATA_DIR"

# Set up Node 1 (Leader)
echo "Setting up Node 1 (Leader)..."
NODE1_DATA_DIR="$DATA_DIR/node1"
mkdir -p "$NODE1_DATA_DIR"
chmod -R 755 "$NODE1_DATA_DIR" || echo "Warning: Could not change permissions on $NODE1_DATA_DIR"

# Create Node 1's config
cat > "$NODE1_DATA_DIR/Config.toml" << EOL
# Set the external rpc url
external_rpc_url = "http://127.0.0.1:7100"

# Set the internal rpc url
internal_rpc_url = "http://127.0.0.1:7200"

# Set the cluster rpc url
cluster_rpc_url = "http://127.0.0.1:7300"

# Set the leader cluster rpc url (previously seed-cluster-rpc-url)
# leader_cluster_rpc_url = "http://127.0.0.1:7300"

# Set the node role (leader, committee, solver, verifier)
role = "leader"

# Set the radius foundation address
radius_foundation_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

# Set the chain type (for verifying signature for foundation address)
chain_type = "ethereum"

# Set partial key generation cycle
partial_key_generation_cycle = 5

# Set partial key aggregation cycle
partial_key_aggregation_cycle = 4
EOL

# Set up Node 1's private key
NODE1_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"  # Account 0
echo -n "$NODE1_PRIVATE_KEY" > "$NODE1_DATA_DIR/signing_key"
echo "Node 1 private key set to Account 0"

# Set up Node 2 (Committee)
echo "Setting up Node 2 (Committee)..."
NODE2_DATA_DIR="$DATA_DIR/node2"
mkdir -p "$NODE2_DATA_DIR"
chmod -R 755 "$NODE2_DATA_DIR" || echo "Warning: Could not change permissions on $NODE2_DATA_DIR"

# Create Node 2's config
cat > "$NODE2_DATA_DIR/Config.toml" << EOL
# Set the external rpc url
external_rpc_url = "http://127.0.0.1:7101"

# Set the internal rpc url
internal_rpc_url = "http://127.0.0.1:7201"

# Set the cluster rpc url
cluster_rpc_url = "http://127.0.0.1:7301"

# Set the leader cluster rpc url (previously seed-cluster-rpc-url)
leader_cluster_rpc_url = "http://127.0.0.1:7300"

# Set the node role (leader, committee, solver, verifier)
role = "committee"

# Set the radius foundation address
radius_foundation_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

# Set the chain type (for verifying signature for foundation address)
chain_type = "ethereum"

# Set partial key generation cycle
partial_key_generation_cycle = 5

# Set partial key aggregation cycle
partial_key_aggregation_cycle = 4
EOL

# Set up Node 2's private key
NODE2_PRIVATE_KEY="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"  # Account 1
echo -n "$NODE2_PRIVATE_KEY" > "$NODE2_DATA_DIR/signing_key"
echo "Node 2 private key set to Account 1"

# Final step: Copy binaries
echo "Copying binaries to nodes..."

# Copy binary to Node 1
echo "Copying binary to Node 1..."
/bin/cp -f "$BINARY_PATH" "$NODE1_DATA_DIR/key-generator"
if [ ! -f "$NODE1_DATA_DIR/key-generator" ]; then
    echo "Error: Failed to copy binary to Node 1"
    exit 1
fi
chmod 755 "$NODE1_DATA_DIR/key-generator"

# Copy binary to Node 2
echo "Copying binary to Node 2..."
/bin/cp -f "$BINARY_PATH" "$NODE2_DATA_DIR/key-generator"
if [ ! -f "$NODE2_DATA_DIR/key-generator" ]; then
    echo "Error: Failed to copy binary to Node 2"
    exit 1
fi
chmod 755 "$NODE2_DATA_DIR/key-generator"

echo "Test nodes setup complete!"
echo "Node 1 (Leader) is configured at $NODE1_DATA_DIR"
echo "Node 2 (Committee) is configured at $NODE2_DATA_DIR" 