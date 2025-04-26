#!/bin/bash

# This script registers the nodes with each other and verifies the registration

# Register Node 2 (Committee) with Node 1 (Leader)
echo "Registering Node 2 with Node 1..."
curl --request POST \
  --url http://127.0.0.1:7200/ \
  --header 'Content-Type: application/json' \
  --data '{
    "jsonrpc": "2.0",
    "method": "add_key_generator",
    "params": {
        "message": {
            "address": "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
            "cluster_rpc_url": "http://127.0.0.1:7301",
            "external_rpc_url": "http://127.0.0.1:7101"
        }
    },
    "id": 1
}'

echo
# Register Node 1 (Leader) with Node 2 (Committee)
echo "Registering Node 1 with Node 2..."
curl --request POST \
  --url http://127.0.0.1:7201/ \
  --header 'Content-Type: application/json' \
  --data '{
    "jsonrpc": "2.0",
    "method": "add_key_generator",
    "params": {
        "message": {
            "address": "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
            "cluster_rpc_url": "http://127.0.0.1:7300",
            "external_rpc_url": "http://127.0.0.1:7100"
        }
    },
    "id": 1
}'

echo
# Verify that Node 1 has both nodes in its key generator list
echo "Checking Node 1 key generator list..."
curl --request POST \
  --url http://127.0.0.1:7300/ \
  --header 'Content-Type: application/json' \
  --data '{
    "jsonrpc": "2.0",
    "method": "get_key_generator_list",
    "id": 1
}'

echo
# Verify that Node 2 has both nodes in its key generator list
echo "Checking Node 2 key generator list..."
curl --request POST \
  --url http://127.0.0.1:7301/ \
  --header 'Content-Type: application/json' \
  --data '{
    "jsonrpc": "2.0",
    "method": "get_key_generator_list",
    "id": 1
}'

echo
echo "Registration completed."
