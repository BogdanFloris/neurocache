#!/bin/bash

# NeuroCache Test Cluster Management Script
# This script helps manage a 3-node NeuroCache cluster for testing

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PIDS_FILE="$SCRIPT_DIR/.cluster_pids"
LOGS_DIR="$SCRIPT_DIR/logs"

# Node configurations
NODE_CONFIGS=(
	"node_1.json:9091"
	"node_2.json:9092"
	"node_3.json:9093"
)

# Function to print colored output
print_color() {
	printf "${2}${1}${NC}\n"
}

# Function to check if a process is running
is_running() {
	kill -0 $1 2>/dev/null
}

# Function to start a single node
start_node() {
	local config_file=$1
	local metrics_port=$2
	local node_num=$(echo $config_file | grep -o '[0-9]')

	print_color "Starting Node $node_num (Metrics: $metrics_port)..." "$YELLOW"

	# Create log directory if it doesn't exist
	mkdir -p "$LOGS_DIR"

	# Start the node
	cd "$PROJECT_ROOT"
	nohup cargo run --bin neurod -- \
		--config-file "$SCRIPT_DIR/$config_file" \
		--metrics-addr "0.0.0.0:$metrics_port" \
		>"$LOGS_DIR/node_$node_num.log" 2>&1 &

	local pid=$!
	echo "$pid:$node_num" >>"$PIDS_FILE"

	# Wait a bit to see if it started successfully
	sleep 2
	if is_running $pid; then
		print_color "✓ Node $node_num started (PID: $pid)" "$GREEN"
	else
		print_color "✗ Failed to start Node $node_num" "$RED"
		return 1
	fi
}

# Function to stop all nodes
stop_cluster() {
	print_color "Stopping NeuroCache cluster..." "$YELLOW"

	if [ ! -f "$PIDS_FILE" ]; then
		print_color "No running cluster found" "$YELLOW"
		return 0
	fi

	while IFS=':' read -r pid node_num; do
		if is_running $pid; then
			print_color "Stopping Node $node_num (PID: $pid)..." "$YELLOW"
			kill $pid

			# Wait for graceful shutdown
			local count=0
			while is_running $pid && [ $count -lt 10 ]; do
				sleep 1
				count=$((count + 1))
			done

			if is_running $pid; then
				print_color "Force killing Node $node_num..." "$RED"
				kill -9 $pid
			fi

			print_color "✓ Node $node_num stopped" "$GREEN"
		fi
	done <"$PIDS_FILE"

	rm -f "$PIDS_FILE"
}

# Function to start the cluster
start_cluster() {
	# Check if cluster is already running
	if [ -f "$PIDS_FILE" ]; then
		print_color "Cluster appears to be running. Please stop it first." "$RED"
		return 1
	fi

	print_color "Starting NeuroCache cluster..." "$GREEN"

	# Build the project first
	print_color "Building project..." "$YELLOW"
	cd "$PROJECT_ROOT"
	if ! cargo build --bin neurod; then
		print_color "Build failed!" "$RED"
		return 1
	fi

	# Start each node
	for config in "${NODE_CONFIGS[@]}"; do
		IFS=':' read -r config_file metrics_port <<<"$config"
		start_node "$config_file" "$metrics_port"
	done

	print_color "\nCluster started successfully!" "$GREEN"
	print_color "Metrics ports: 9091, 9092, 9093" "$GREEN"
	print_color "\nLogs available in: $LOGS_DIR" "$GREEN"
}

# Function to check cluster status
status_cluster() {
	print_color "Checking cluster status..." "$YELLOW"

	if [ ! -f "$PIDS_FILE" ]; then
		print_color "No cluster is running" "$YELLOW"
		return 0
	fi

	local running=0
	while IFS=':' read -r pid node_num; do
		if is_running $pid; then
			print_color "✓ Node $node_num is running (PID: $pid)" "$GREEN"
			running=$((running + 1))
		else
			print_color "✗ Node $node_num is not running (PID: $pid)" "$RED"
		fi
	done <"$PIDS_FILE"

	if [ $running -eq 0 ]; then
		print_color "\nNo nodes are running" "$RED"
		rm -f "$PIDS_FILE"
	else
		print_color "\n$running node(s) running" "$GREEN"

		# Check metrics endpoints
		print_color "\nChecking metrics endpoints..." "$YELLOW"
		for port in 9091 9092 9093; do
			if curl -s -o /dev/null -w "%{http_code}" http://localhost:$port/metrics | grep -q "200"; then
				print_color "✓ Metrics available on port $port" "$GREEN"
			else
				print_color "✗ Metrics not available on port $port" "$RED"
			fi
		done
	fi
}

# Function to tail logs
tail_logs() {
	if [ ! -d "$LOGS_DIR" ]; then
		print_color "No logs directory found" "$RED"
		return 1
	fi

	print_color "Tailing logs (Ctrl+C to stop)..." "$YELLOW"
	tail -f "$LOGS_DIR"/*.log
}

# Function to clean logs
clean_logs() {
	print_color "Cleaning logs..." "$YELLOW"
	rm -rf "$LOGS_DIR"
	print_color "✓ Logs cleaned" "$GREEN"
}

# Function to restart the cluster
restart_cluster() {
	stop_cluster
	sleep 2
	start_cluster
}

# Function to test the cluster
test_cluster() {
	print_color "Testing cluster connectivity..." "$YELLOW"

	# leader port
	local port=3001

	if cargo run --bin neuroctl -- --endpoints 127.0.0.1:$port put test-key test-value 2>/dev/null; then
		print_color "✓ PUT operation successful" "$GREEN"

		if cargo run --bin neuroctl -- --endpoints 127.0.0.1:$port get test-key 2>/dev/null | grep -q "test-value"; then
			print_color "✓ GET operation successful" "$GREEN"
		else
			print_color "✗ GET operation failed" "$RED"
		fi
	else
		print_color "✗ PUT operation failed" "$RED"
	fi
}

# Function to show help
show_help() {
	cat <<EOF
NeuroCache Test Cluster Management Script

Usage: $0 [command]

Commands:
    start       Start the 3-node cluster
    stop        Stop all running nodes
    restart     Restart the cluster
    status      Check cluster status
    logs        Tail all node logs
    clean       Clean log files
    test        Run basic connectivity tests
    help        Show this help message

Example:
    $0 start    # Start the cluster
    $0 status   # Check if nodes are running
    $0 logs     # View logs
    $0 stop     # Stop the cluster

Monitoring:
    After starting the cluster, you can:
    - View metrics at http://localhost:909{1,2,3}/metrics
    - Start monitoring stack: cd .. && docker-compose up -d
    - Access Grafana dashboards at http://localhost:3000

EOF
}

# Main script logic
case "$1" in
start)
	start_cluster
	;;
stop)
	stop_cluster
	;;
restart)
	restart_cluster
	;;
status)
	status_cluster
	;;
logs)
	tail_logs
	;;
clean)
	clean_logs
	;;
test)
	test_cluster
	;;
help | --help | -h)
	show_help
	;;
*)
	print_color "Unknown command: $1" "$RED"
	print_color "Use '$0 help' for usage information" "$YELLOW"
	exit 1
	;;
esac
