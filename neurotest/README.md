# NeuroTest - Test Cluster Management for NeuroCache

`neurotest` is a Rust-based CLI tool for managing NeuroCache test cluster.

## Installation

From the project root:

```bash
cargo install --path neurotest
```

Or run directly without installation:

```bash
cargo run --bin neurotest -- [COMMAND]
```

## Usage

### Starting a Cluster

```bash
# Start a 3-node cluster (default)
neurotest start

# Start a 5-node cluster
neurotest start --nodes 5

# Start a single-node cluster (no consensus)
neurotest start --single
```

### Managing the Cluster

```bash
# Check cluster status
neurotest status

# Stop the cluster
neurotest stop

# View logs (all nodes)
neurotest logs

# View logs for a specific node
neurotest logs 1

# Follow logs in real-time
neurotest logs --follow
```

### Testing and Benchmarking

```bash
# Run connectivity tests
neurotest test

# Run load benchmark (60s, 8 threads, 50% reads)
neurotest bench

# Custom benchmark settings
neurotest bench --duration 120 --threads 16 --read-ratio 0.8
```

### Monitoring Stack

```bash
# Start Prometheus and Grafana
neurotest monitor start

# Check monitoring status
neurotest monitor status

# Stop monitoring
neurotest monitor stop
```

### Cleanup

```bash
# Clean cluster state
neurotest clean

# Clean everything including logs
neurotest clean --logs
```

## Configuration

The tool uses `neurotest/cluster.toml` for configuration:

```toml
[cluster]
base_raft_port = 3001
base_metrics_port = 9091
default_nodes = 3

[paths]
logs_dir = "logs"
state_file = ".cluster_state.json"
test_cluster_dir = "."

[monitoring]
enabled = true
prometheus_port = 9090
grafana_port = 3000
compose_file = "docker-compose.yml"
```

## Features

- **Colored Output**: Clear visual feedback with color-coded messages
- **Progress Indicators**: Visual progress bars for long-running operations
- **Automatic Health Checks**: Verifies nodes are running and metrics are accessible
- **Graceful Shutdown**: Attempts graceful shutdown before force-killing processes
- **State Persistence**: Tracks cluster state across restarts
- **Error Recovery**: Robust error handling with helpful messages

## Port Allocation

Ports are allocated sequentially based on the base ports in configuration:

| Node | Raft Port | Metrics Port |
| ---- | --------- | ------------ |
| 1    | 3001      | 9091         |
| 2    | 3002      | 9092         |
| 3    | 3003      | 9093         |
| ...  | ...       | ...          |

## Requirements

- Rust and Cargo
- Docker (for monitoring features)
- Linux/macOS (uses Unix signals for process management)
