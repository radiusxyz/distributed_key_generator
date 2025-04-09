#!/bin/bash
SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
PROJECT_ROOT_PATH="$( cd $SCRIPT_PATH/../.. >/dev/null 2>&1 ; pwd -P )"

# Check if the test nodes have been set up
NODE1_DATA_PATH="$PROJECT_ROOT_PATH/data/node1"
NODE2_DATA_PATH="$PROJECT_ROOT_PATH/data/node2"

if [ ! -d "$NODE1_DATA_PATH" ] && [ ! -d "$NODE2_DATA_PATH" ]; then
  echo "No test nodes found to clean up."
  exit 0
fi

# Function to check if a node is running
check_node_running() {
  local port=$1
  if curl -s --request POST \
    --url "http://127.0.0.1:$port/" \
    --header 'Content-Type: application/json' \
    --data '{"jsonrpc": "2.0", "method": "get_key_generator_list", "id": 1}' > /dev/null; then
    return 0
  else
    return 1
  fi
}

# Check if nodes are running and stop them
if check_node_running 7200; then
  echo "Node 1 (Leader) is running. Stopping it..."
  # Find the PID of the process using port 7200
  NODE1_PID=$(lsof -ti:7200)
  if [ -n "$NODE1_PID" ]; then
    kill $NODE1_PID
    echo "Node 1 stopped."
  else
    echo "Could not find PID for Node 1."
  fi
fi

if check_node_running 7201; then
  echo "Node 2 (Committee) is running. Stopping it..."
  # Find the PID of the process using port 7201
  NODE2_PID=$(lsof -ti:7201)
  if [ -n "$NODE2_PID" ]; then
    kill $NODE2_PID
    echo "Node 2 stopped."
  else
    echo "Could not find PID for Node 2."
  fi
fi

# Remove the data directories
echo "Removing test node data directories..."
if [ -d "$NODE1_DATA_PATH" ]; then
  rm -rf "$NODE1_DATA_PATH"
  echo "Removed $NODE1_DATA_PATH"
fi

if [ -d "$NODE2_DATA_PATH" ]; then
  rm -rf "$NODE2_DATA_PATH"
  echo "Removed $NODE2_DATA_PATH"
fi

echo "Cleanup complete!" 