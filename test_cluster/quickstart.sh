#!/bin/bash

# Quick start script for NeuroCache cluster with monitoring

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

print_color() {
	printf "${2}${1}${NC}\n"
}

print_color "NeuroCache Quick Start with Monitoring" "$GREEN"
print_color "=====================================" "$GREEN"

# Start the cluster
print_color "\n1. Starting NeuroCache cluster..." "$YELLOW"
"$SCRIPT_DIR/cluster.sh" start

if [ $? -ne 0 ]; then
	print_color "Failed to start cluster!" "$RED"
	exit 1
fi

# Give the cluster a moment to stabilize
sleep 3

# Start monitoring stack
print_color "\n2. Starting monitoring stack (Prometheus + Grafana)..." "$YELLOW"
cd "$PROJECT_ROOT"
docker-compose up -d

if [ $? -ne 0 ]; then
	print_color "Failed to start monitoring stack!" "$RED"
	print_color "Make sure Docker is running and docker-compose is installed" "$YELLOW"
else
	print_color "✓ Monitoring stack started" "$GREEN"
fi

# Wait for services to be ready
print_color "\n3. Waiting for services to be ready..." "$YELLOW"
sleep 5

# Check services
print_color "\n4. Checking services..." "$YELLOW"

# Check cluster nodes
"$SCRIPT_DIR/cluster.sh" status

# Check monitoring
echo
if curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/api/health | grep -q "200"; then
	print_color "✓ Grafana is running at http://localhost:3000 (admin/admin)" "$GREEN"
else
	print_color "✗ Grafana is not accessible" "$RED"
fi

if curl -s -o /dev/null -w "%{http_code}" http://localhost:9090/api/v1/query?query=up | grep -q "200"; then
	print_color "✓ Prometheus is running at http://localhost:9090" "$GREEN"
else
	print_color "✗ Prometheus is not accessible" "$RED"
fi

# Run a quick test
print_color "\n5. Running connectivity test..." "$YELLOW"
sleep 2
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7000 put quickstart-test success 2>/dev/null
if cargo run --bin neuroctl -- --endpoints 127.0.0.1:7000 get quickstart-test 2>/dev/null | grep -q "success"; then
	print_color "✓ Cluster is responding to commands" "$GREEN"
else
	print_color "✗ Cluster test failed" "$RED"
fi

# Print summary
print_color "\n=============== Summary ===============" "$GREEN"
print_color "Cluster Status: Running" "$GREEN"
print_color "Metrics Ports: 9091, 9092, 9093" "$YELLOW"
print_color "\nMonitoring:" "$GREEN"
print_color "- Grafana: http://localhost:3000 (admin/admin)" "$YELLOW"
print_color "- Prometheus: http://localhost:9090" "$YELLOW"
print_color "\nDashboards:" "$GREEN"
print_color "- Raft Consensus Overview" "$YELLOW"
print_color "- Raft Performance Metrics" "$YELLOW"
print_color "\nUseful Commands:" "$GREEN"
print_color "- View logs: $SCRIPT_DIR/cluster.sh logs" "$YELLOW"
print_color "- Stop everything: $SCRIPT_DIR/shutdown.sh" "$YELLOW"
print_color "- Test cluster: $SCRIPT_DIR/cluster.sh test" "$YELLOW"
print_color "======================================" "$GREEN"
