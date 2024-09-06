#!/bin/bash
if [ "$#" -ne 1 ]; then
    echo "Usage: ./11_run_key_generator.sh <node_index>"
    exit 1
fi

SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
source $SCRIPT_PATH/env.sh

node_index=$1

DATA_PATH=$CURRENT_PATH/key_generators/key_generator_$node_index

$KEY_GENERATOR_BIN_PATH start --path $DATA_PATH