#!/bin/bash

# This script runs the test nodes and registers them with each other

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT_PATH="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Define binary paths
NODE1_BINARY="$PROJECT_ROOT_PATH/data/node1/key-generator"
NODE2_BINARY="$PROJECT_ROOT_PATH/data/node2/key-generator"

# Check if binaries exist
if [ ! -f "$NODE1_BINARY" ]; then
    echo "Error: Node 1 binary not found at $NODE1_BINARY"
    exit 1
fi

if [ ! -f "$NODE2_BINARY" ]; then
    echo "Error: Node 2 binary not found at $NODE2_BINARY"
    exit 1
fi

# Start Node 1 (Leader)
echo "Starting Node 1 (Leader)..."
cd "$PROJECT_ROOT_PATH/data/node1"
"$NODE1_BINARY" start --path "$PROJECT_ROOT_PATH/data/node1" &
NODE1_PID=$!

# Wait for Node 1 to start
sleep 5

# Start Node 2 (Committee)
echo "Starting Node 2 (Committee)..."
cd "$PROJECT_ROOT_PATH/data/node2"
"$NODE2_BINARY" start --path "$PROJECT_ROOT_PATH/data/node2" &
NODE2_PID=$!

# Wait for Node 2 to start
sleep 5

# Register Node 2 with Node 1
echo "Registering Node 2 with Node 1..."
curl -X POST http://localhost:7200/register -H "Content-Type: application/json" -d '{"address":"0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"}'

# Register Node 1 with Node 2
echo "Registering Node 1 with Node 2..."
curl -X POST http://localhost:7201/register -H "Content-Type: application/json" -d '{"address":"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"}'

# Verify registrations
echo "Verifying registrations..."
echo "Node 1 registrations:"
curl http://localhost:7200/registrations
echo "Node 2 registrations:"
curl http://localhost:7201/registrations

# Keep the script running
echo "Test nodes are running. Press Ctrl+C to stop."
wait $NODE1_PID $NODE2_PID 