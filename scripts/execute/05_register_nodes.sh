#!/bin/bash

# This script registers the nodes with each other and verifies the registration

# Register Node 2 (Committee) with Node 1 (Leader)
echo "Registering Node 2 with Node 1..."
# Send JSON-RPC request to Node 1's internal RPC endpoint
curl --request POST \
  --url http://127.0.0.1:7200/ \
  --header 'Content-Type: application/json' \
  --data '{
    "jsonrpc": "2.0",
    "method": "add_key_generator",
    "params": {
        "message": {
            "address": "0x70997970c51812dc3a010c7d01b50e0d17dc79c8",
            "cluster_rpc_url": "http://127.0.0.1:7301",
            "external_rpc_url": "http://127.0.0.1:7101"
        }
    },
    "id": 1
}'

echo
# Register Node 1 (Leader) with Node 2 (Committee)
echo "Registering Node 1 with Node 2..."
# Send JSON-RPC request to Node 2's internal RPC endpoint
curl --request POST \
  --url http://127.0.0.1:7201/ \
  --header 'Content-Type: application/json' \
  --data '{
    "jsonrpc": "2.0",
    "method": "add_key_generator",
    "params": {
        "message": {
            "address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
            "cluster_rpc_url": "http://127.0.0.1:7300",
            "external_rpc_url": "http://127.0.0.1:7100"
        }
    },
    "id": 1
}'

echo
# Verify that Node 1 has both nodes in its key generator list
echo "Checking Node 1 key generator list..."
# Send JSON-RPC request to Node 1's cluster RPC endpoint
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
# Send JSON-RPC request to Node 2's cluster RPC endpoint
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