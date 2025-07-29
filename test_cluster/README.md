# NeuroCache Test Cluster

This directory contains scripts and configuration for running a 3-node NeuroCache test cluster with monitoring.

## Quick Start

```bash
# Start everything (cluster + monitoring)
./quickstart.sh

# Access Grafana dashboards
open http://localhost:3000  # Login: admin/admin
```

## Cluster Management

The `cluster.sh` script provides full control over the test cluster:

```bash
# Start the cluster
./cluster.sh start

# Check status
./cluster.sh status

# View logs
./cluster.sh logs

# Run tests
./cluster.sh test

# Stop the cluster
./cluster.sh stop

# Restart the cluster
./cluster.sh restart

# Clean up logs
./cluster.sh clean
```

## Shutdown

To stop everything:

```bash
./shutdown.sh
```

## Configuration

The cluster consists of 3 nodes with the following ports:

| Node | Raft Port | HTTP Port | Metrics Port |
|------|-----------|-----------|--------------|
| 1    | 7000      | 8080      | 9090         |
| 2    | 7001      | 8081      | 9091         |
| 3    | 7002      | 8082      | 9092         |

## Testing the Cluster

### Using neuroctl

```bash
# Write a value
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7000 put mykey myvalue

# Read a value
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7000 get mykey

# Delete a value
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7000 del mykey

# Try different nodes
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7001 get mykey
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7002 get mykey
```

### Monitoring

After starting with `quickstart.sh`, you can access:

- **Grafana**: http://localhost:3000 (admin/admin)
  - Raft Consensus Overview dashboard
  - Raft Performance Metrics dashboard
- **Prometheus**: http://localhost:9091
- **Node Metrics**: http://localhost:909{0,1,2}/metrics

## Logs

Logs are stored in `test_cluster/logs/`:
- `node_1.log` - Node 1 output
- `node_2.log` - Node 2 output  
- `node_3.log` - Node 3 output

## Troubleshooting

### Cluster won't start
1. Check if ports are already in use: `lsof -i :7000-7002,9090-9092`
2. Make sure no previous cluster is running: `./cluster.sh status`
3. Check build errors: `cargo build --bin neurod`

### Monitoring not working
1. Ensure Docker is running: `docker ps`
2. Check if ports are free: `lsof -i :3000,9091`
3. Restart monitoring: `cd .. && docker-compose restart`

### Can't connect to cluster
1. Check node status: `./cluster.sh status`
2. Verify metrics endpoints: `curl http://localhost:9090/metrics`
3. Check logs for errors: `./cluster.sh logs`