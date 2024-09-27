#!/bin/bash
CURRENT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

KEY_GENERATOR_BIN_PATH=$CURRENT_PATH/../target/release/key_generator

HOST="192.168.12.14"

ADDRESS_1="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

INTERNAL_IP_ADDRESS_1="http://$HOST:7200"

CLUSTER_IP_ADDRESS_1="http://$HOST:7300"