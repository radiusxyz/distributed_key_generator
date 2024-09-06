#!/bin/bash
if [ "$#" -ne 1 ]; then
    echo "Usage: ./10_init_key_generator.sh <NODE_COUNT>"
    exit 1
fi

SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
source $SCRIPT_PATH/env.sh

NODE_COUNT=$1

rm -rf $CURRENT_PATH/key_generators
mkdir -p $CURRENT_PATH/key_generators

for (( node_index=0; node_index<NODE_COUNT; node_index++ )) do
    echo "Initialize key_generator $node_index" 
    data_path=$CURRENT_PATH/key_generators/key_generator_$node_index
    
    $KEY_GENERATOR_BIN_PATH init --path $data_path

    config_file_path=$data_path/config.toml
    
    sed -i.temp "s/external_rpc_url = \"http:\/\/127.0.0.1:3000\"/external_rpc_url = \"http:\/\/127.0.0.1:300$node_index\"/g" $config_file_path
    sed -i.temp "s/internal_rpc_url = \"http:\/\/127.0.0.1:4000\"/internal_rpc_url = \"http:\/\/127.0.0.1:400$node_index\"/g" $config_file_path
    sed -i.temp "s/cluster_rpc_url = \"http:\/\/127.0.0.1:5000\"/cluster_rpc_url = \"http:\/\/127.0.0.1:500$node_index\"/g" $config_file_path

    # TODO: remove
    private_key_path=$data_path/signing_key
    sed -i.temp "s/0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80/0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff8$node_index/g" $private_key_path

    rm $config_file_path.temp
    rm $private_key_path.temp
done  

