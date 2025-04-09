#!/bin/bash
SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
source $SCRIPT_PATH/env.sh

# Optional role parameter (leader, committee, solver, verifier)
# If provided, this will override the role in the config file
ROLE=${1}

if [ -z "$ROLE" ]; then
  echo "Starting node with role from config..."
  $BIN_PATH start --path $DATA_PATH
else
  echo "Starting node with role: $ROLE..."
  $BIN_PATH start --path $DATA_PATH --role "$ROLE"
fi