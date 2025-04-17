#!/bin/bash

set -e

SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
PROJECT_ROOT_PATH="$( cd $SCRIPT_PATH/../.. >/dev/null 2>&1 ; pwd -P )"

# 1. Cleanup
echo "[1/4] Cleaning up old data and processes..."
"$PROJECT_ROOT_PATH/scripts/execute/00_cleanup_nodes.sh"

# 2. Start authority
echo "[2/4] Starting authority node..."
"$PROJECT_ROOT_PATH/scripts/execute/02_run_authority.sh" &
sleep 1.5  # Give it time to bind the port

# 3. Start leader
echo "[3/4] Starting leader node..."
"$PROJECT_ROOT_PATH/scripts/execute/03_run_leader.sh" &
sleep 1.5

# 4. Start committee (node2)
echo "[4/4] Starting committee node..."
"$PROJECT_ROOT_PATH/scripts/execute/04_run_committee.sh" &
sleep 1.5

echo "All nodes started: authority, leader, and committee."
