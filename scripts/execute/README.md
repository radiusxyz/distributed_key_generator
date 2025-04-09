# Distributed Key Generator Scripts

This directory contains scripts for running the distributed key generator with multiple nodes.

## Quick Guide

1. `./01_cleanup_nodes.sh` - Clean previous environment
2. `./02_run_leader.sh` - Setup and run Node 1 (Leader) in terminal 1
3. `./03_run_committee.sh` - Setup and run Node 2 (Committee) in terminal 2
4. `./04_register_nodes.sh` - Register nodes with each other in terminal 3

## Scripts

### 01_cleanup_nodes.sh
Stops all processes and removes data directories.

### 02_run_leader.sh
Sets up and starts Node 1 (Leader) with proper configuration.

### 03_run_committee.sh
Sets up and starts Node 2 (Committee) with proper configuration.

### 04_register_nodes.sh
Registers nodes with each other and verifies the connection.