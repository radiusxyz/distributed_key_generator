# Scripts for Key Generator

This directory contains scripts to help you run and test the key generator.

## Setup

1. First, make sure you have the key generator binary built:
```bash
cargo build --release
```

2. Make the scripts executable:
```bash
chmod +x *.sh
```

## Scripts

### 01_init_key_generator.sh
Initializes a new key generator node. This script:
- Creates a data directory
- Copies the key generator binary
- Initializes the node with default configuration
- Updates the configuration with your settings

Usage:
```bash
./01_init_key_generator.sh
```

### 02_run_key_generator.sh
Runs the key generator node. This script:
- Checks if the node is initialized
- Starts the node with the configured settings
- Keeps the node running until you press Ctrl+C

Usage:
```bash
./02_run_key_generator.sh
```

### 03_setup_test_nodes.sh
Sets up two test nodes (Node 1 as Leader and Node 2 as Committee) for testing. This script:
- Creates separate data directories for each node
- Copies the key generator binary to each node
- Initializes each node with appropriate configuration
- Sets up the correct private keys and addresses for each node

Usage:
```bash
./03_setup_test_nodes.sh
```

### 04_run_test_nodes.sh
Runs the test nodes and registers them with each other. This script:
- Starts Node 1 (Leader) and Node 2 (Committee)
- Waits for the nodes to initialize
- Registers Node 2 with Node 1
- Verifies the registration
- Keeps both nodes running until you press Ctrl+C

Usage:
```bash
./04_run_test_nodes.sh
```

### 05_cleanup_test_nodes.sh
Cleans up the test environment. This script:
- Stops any running test nodes
- Removes the test node data directories
- Cleans up the test environment

Usage:
```bash
./05_cleanup_test_nodes.sh
```

## Complete Test Workflow

To test the DKG system with multiple nodes:

1. Set up the test nodes:
```bash
./03_setup_test_nodes.sh
```

2. Run the test nodes:
```bash
./04_run_test_nodes.sh
```

3. When you're done testing, clean up:
```bash
./05_cleanup_test_nodes.sh
```

This workflow allows you to easily test the DKG system with multiple nodes without having to manually copy the project and configure each node.

## Environment Variables

The scripts use the following environment variables:

- `PROJECT_ROOT_PATH`: The root path of the project (default: current directory)
- `DATA_DIR`: The directory where node data is stored (default: `$PROJECT_ROOT_PATH/data`)
- `BINARY_PATH`: The path to the key generator binary (default: `$PROJECT_ROOT_PATH/target/release/key-generator`)
- `NODE1_PRIVATE_KEY`: The private key for Node 1 (Leader)
- `NODE2_PRIVATE_KEY`: The private key for Node 2 (Committee)

You can set these variables in your environment or modify them in the scripts.

# DKG Node Operations

This directory contains scripts to initialize and run DKG nodes with different roles.

## Setup

1. Copy the `env_example.sh` to `env.sh` and configure the environment variables:
   ```bash
   cp env_example.sh env.sh
   ```

2. Edit `env.sh` to set your configuration:
   - Set the RPC URLs to appropriate values
   - For non-leader nodes, set the `LEADER_CLUSTER_RPC_URL` to the leader's cluster RPC URL
   - Set a unique private key for each node

## Node Roles

The DKG system supports the following roles:

- **Leader**: Coordinates the key generation process, collects partial keys, and broadcasts aggregated keys
- **Committee**: Generates partial keys and sends them to the leader
- **Solver**: Performs time-lock puzzle solving to recover decryption keys
- **Verifier**: Monitors the network for Byzantine behavior

## Initializing Nodes

Initialize a node with a specific role using:

```bash
./01_init_key_generator.sh [role]
```

Where `[role]` is one of:
- `leader`
- `committee` (default)
- `solver`
- `verifier`

Example:
```bash
# Initialize a leader node
./01_init_key_generator.sh leader

# Initialize a committee node
./01_init_key_generator.sh committee

# Initialize a solver node
./01_init_key_generator.sh solver

# Initialize a verifier node
./01_init_key_generator.sh verifier
```

## Running Nodes

Run a node with:

```bash
./02_run_key_generator.sh [role]
```

If `[role]` is provided, it will override the role in the config file.
If not provided, the node will use the role from its configuration.

Example:
```bash
# Run with the role from config
./02_run_key_generator.sh

# Run with a specific role
./02_run_key_generator.sh leader
```

## Network Setup Example

To set up a complete network, run:

```bash
# First terminal - Leader node
./01_init_key_generator.sh leader
./02_run_key_generator.sh

# Second terminal - Committee node
./01_init_key_generator.sh committee
./02_run_key_generator.sh

# Third terminal - Solver node
./01_init_key_generator.sh solver
./02_run_key_generator.sh

# Fourth terminal - Verifier node
./01_init_key_generator.sh verifier
./02_run_key_generator.sh
```

Make sure to use different data directories and proper network configurations for multiple nodes running on the same machine.

## Quick Test Setup

For quick testing with multiple nodes, you can use the following scripts:

### 1. Setup Test Nodes

```bash
./03_setup_test_nodes.sh
```

This script will:
1. Create separate data directories for each node
2. Initialize a leader node and a committee node with appropriate configurations
3. Set different private keys for each node
4. Provide instructions for running the nodes and registering them with each other

### 2. Run Test Nodes

```bash
./04_run_test_nodes.sh
```

This script will:
1. Start both the leader and committee nodes in the background
2. Ask if you want to register the nodes with each other
3. Keep the nodes running until you press Ctrl+C to stop them

To register the nodes later, run:
```bash
./04_run_test_nodes.sh register
```

This will send the registration requests to the leader node without starting the nodes.

### 3. Cleanup Test Nodes

```bash
./05_cleanup_test_nodes.sh
```

This script will:
1. Stop any running test nodes
2. Remove the test node data directories
3. Clean up the test environment

## Complete Test Workflow

For a complete test workflow, you can run:

```bash
# 1. Setup the test nodes
./03_setup_test_nodes.sh

# 2. Run the test nodes
./04_run_test_nodes.sh

# 3. When done, clean up
./05_cleanup_test_nodes.sh
``` 