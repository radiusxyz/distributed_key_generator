#!/bin/bash

SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
source $SCRIPT_PATH/env.sh

curl --location $INTERNAL_IP_ADDRESS_1 \
--header 'Content-Type: application/json' \
--data '{
    "jsonrpc": "2.0",
    "method": "add_key_generator",
    "params": {
        "message": {
            "address": "'"$ADDRESS_1"'",
            "ip_address": "'"$CLUSTER_IP_ADDRESS_1"'"
        }
    },
    "id": 1
}'

echo ""