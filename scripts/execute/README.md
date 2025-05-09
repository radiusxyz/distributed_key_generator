# Distributed Key Generator Scripts

This directory contains scripts for running the distributed key generator with multiple nodes.

## Quick Guide

1. `./00_cleanup_nodes.sh` - Clean previous environment
2. `./01_run_all_nodes.sh` - Start all nodes (Authority → Leader → Committee → Solver) in one go
3. `./02_run_authority.sh` - Start the Authority Node that manages SKDE parameters
4. `./03_run_leader.sh` - Setup and run Node 1 (Leader) in terminal 1
5. `./04_run_committee.sh` - Setup and run Node 2 (Committee) in terminal 2
6. `./05_run_solver.sh` - Setup and run Solver Node in terminal 3
7. `./06_register_nodes.sh` - Register nodes with each other in terminal 4

## Scripts

### 00_cleanup_nodes.sh
Stops all processes and removes data directories.

### 01_run_all_nodes.sh
Cleans up previous data, builds necessary binaries, and starts all nodes in the correct order:
1. Authority Node
2. Leader Node (waits for Authority)
3. Committee Node
4. Solver Node

### 02_run_authority.sh
Starts the `authority_node` binary.  
This node securely initializes and exposes SKDE parameters used by the Leader.

### 03_run_leader.sh
Sets up and starts Node 1 (Leader) with proper configuration.

### 04_run_committee.sh
Sets up and starts Node 2 (Committee) with proper configuration.

### 05_run_solver.sh
Sets up and starts the Solver Node that solves Time Lock Puzzles to generate Decryption Keys.

### 06_register_nodes.sh
Registers nodes with each other and verifies the connection.