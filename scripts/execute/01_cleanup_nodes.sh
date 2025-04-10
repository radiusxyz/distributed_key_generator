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

if [ ! -d "$NODE1_DATA_PATH" ] && [ ! -d "$NODE2_DATA_PATH" ]; then
  echo "No nodes found to clean up."
  exit 0
fi

# Check if nodes are running and stop them
echo "Stopping any remaining node processes..."
if pgrep -f "$NODE1_DATA_PATH/key-generator" > /dev/null; then
  echo "Node 1 (Leader) is running. Stopping it..."
  pkill -f "$NODE1_DATA_PATH/key-generator"
  echo "Node 1 stopped."
else
  echo "Node 1 is not running."
fi

if pgrep -f "$NODE2_DATA_PATH/key-generator" > /dev/null; then
  echo "Node 2 (Committee) is running. Stopping it..."
  pkill -f "$NODE2_DATA_PATH/key-generator"
  echo "Node 2 stopped."
else
  echo "Node 2 is not running."
fi

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

echo "Cleanup complete!" 