#!/bin/bash
SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
source $SCRIPT_PATH/env.sh

mkdir -p $DATA_PATH
rm -rf $DATA_PATH/* $DATA_PATH/.* 2>/dev/null || true

echo "Initialize key generator" 

$BIN_PATH init --path $DATA_PATH

sed -i.temp "s|internal_rpc_url = \"http://127.0.0.1:4000\"|internal_rpc_url = \"$KEY_GENERATOR_INTERNAL_RPC_URL\"|g" $CONFIG_FILE_PATH
sed -i.temp "s|external_rpc_url = \"http://127.0.0.1:3000\"|external_rpc_url = \"$KEY_GENERATOR_EXTERNAL_RPC_URL\"|g" $CONFIG_FILE_PATH
sed -i.temp "s|cluster_rpc_url = \"http://127.0.0.1:5000\"|cluster_rpc_url = \"$KEY_GENERATOR_CLUSTER_RPC_URL\"|g" $CONFIG_FILE_PATH

sed -i.temp "s|0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80|$KEY_GENERATOR_PRIVATE_KEY|g" $PRIVATE_KEY_PATH

rm $CONFIG_FILE_PATH.temp
rm $PRIVATE_KEY_PATH.temp

