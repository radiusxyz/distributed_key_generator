#!/bin/bash
SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
source $SCRIPT_PATH/env.sh

if [[ ! -f "$KEY_GENERATOR_BIN_PATH" ]]; then
    echo "Error: Keygenerator binary not found at $KEY_GENERATOR_BIN_PATH"
    echo "Please run this command 'cp $PROJECT_ROOT_PATH/target/release/key_generator $PROJECT_ROOT_PATH/scripts'"
    exit 1
fi

$KEY_GENERATOR_BIN_PATH start --path $DATA_PATH