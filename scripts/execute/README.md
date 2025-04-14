# Distributed Key Generator Scripts

This directory contains scripts for running the distributed key generator with multiple nodes.

## Quick Guide

1. `./00_cleanup_nodes.sh` - Clean previous environment
2. `./01_run_all_nodes.sh` - Start all nodes (Authority → Leader → Committee) in one go
3. `./02_run_authority.sh` - Start the authority node that manages SKDE parameters 
4. `./03_run_leader.sh` - Setup and run Node 1 (Leader) in terminal 1
5. `./04_run_committee.sh` - Setup and run Node 2 (Committee) in terminal 2
6. `./05_register_nodes.sh` - Register nodes with each other in terminal 3

## Scripts

### 00_cleanup_nodes.sh
Stops all processes and removes data directories.

### 01_run_all_nodes.sh
Cleans up previous data, builds necessary binaries, and starts all nodes in the correct order:
1. Authority node
2. Leader node (waits for authority)
3. Committee node

### 02_run_authority.sh
Starts the `authority_node` binary.  
This node securely initializes and exposes SKDE parameters used by the leader.

### 03_run_leader.sh
Sets up and starts Node 1 (Leader) with proper configuration.

### 04_run_committee.sh
Sets up and starts Node 2 (Committee) with proper configuration.

### 05_register_nodes.sh
Registers nodes with each other and verifies the connection.