#!/bin/bash

# This script cleans up the environment by stopping processes and removing data directories

SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
PROJECT_ROOT_PATH="$( cd $SCRIPT_PATH/../.. >/dev/null 2>&1 ; pwd -P )"

# Kill all key-generator processes
echo "Finding and stopping all key-generator processes..."
pkill -f "key-generator" || echo "No key-generator processes found."
sleep 2  # Give the OS time to clean up

# Check if ports are still in use
echo "Checking if ports are still in use..."
lsof -i :7100 -i :7200 -i :7300 -i :7101 -i :7201 -i :7301 || echo "No processes using these ports."

# Check if the nodes have been set up
NODE1_DATA_PATH="$PROJECT_ROOT_PATH/data/node1"
NODE2_DATA_PATH="$PROJECT_ROOT_PATH/data/node2"
AUTHORITY_DATA_PATH="$PROJECT_ROOT_PATH/data/authority"
SOLVER_DATA_PATH="$PROJECT_ROOT_PATH/data/solver"

if [ ! -d "$NODE1_DATA_PATH" ] && [ ! -d "$NODE2_DATA_PATH" ] && [ ! -d "$SOLVER_DATA_PATH" ] && [ ! -d "$AUTHORITY_DATA_PATH" ]; then
  echo "No nodes found to clean up."
  exit 0
fi

# Check if nodes are running and stop them
stop_node_process() {
  local NODE_PATH=$1
  local NODE_NAME=$2
  local BINARY_NAME=$3

  if pgrep -f "$NODE_PATH/$BINARY_NAME" > /dev/null; then
    echo "$NODE_NAME is running. Stopping it..."
    pkill -f "$NODE_PATH/$BINARY_NAME"
    echo "$NODE_NAME stopped."
  else
    echo "$NODE_NAME is not running."
  fi
}

stop_node_process "$NODE1_DATA_PATH" "Node 1 (Leader)" "key-generator"
stop_node_process "$NODE2_DATA_PATH" "Node 2 (Committee)" "key-generator"
stop_node_process "$SOLVER_DATA_PATH" "Solver Node" "key-generator"
stop_node_process "$AUTHORITY_DATA_PATH" "Authority Node" "authority_node"

# Remove the data directories
echo "Removing node data directories..."
if [ -d "$NODE1_DATA_PATH" ]; then
  rm -rf "$NODE1_DATA_PATH"
  echo "Removed $NODE1_DATA_PATH"
fi

if [ -d "$NODE2_DATA_PATH" ]; then
  rm -rf "$NODE2_DATA_PATH"
  echo "Removed $NODE2_DATA_PATH"
fi

if [ -d "$SOLVER_DATA_PATH" ]; then
  rm -rf "$SOLVER_DATA_PATH"
  echo "Removed $SOLVER_DATA_PATH"
fi

if [ -d "$AUTHORITY_DATA_PATH" ]; then
  echo "Cleaning $AUTHORITY_DATA_PATH except skde_params.json..."
  find "$AUTHORITY_DATA_PATH" ! -name 'skde_params.json' -type f -exec rm -f {} +
  find "$AUTHORITY_DATA_PATH" ! -name 'skde_params.json' -type d -mindepth 1 -exec rm -rf {} +
  echo "Cleaned $AUTHORITY_DATA_PATH (skde_params.json preserved)"
fi

echo "Cleanup complete!" 