#!/bin/bash

# Shutdown script for NeuroCache cluster and monitoring

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

print_color "Shutting down NeuroCache and monitoring..." "$YELLOW"

# Stop the cluster
print_color "\n1. Stopping NeuroCache cluster..." "$YELLOW"
"$SCRIPT_DIR/cluster.sh" stop

# Stop monitoring stack
print_color "\n2. Stopping monitoring stack..." "$YELLOW"
cd "$PROJECT_ROOT"
docker-compose down

print_color "\n✓ All services stopped" "$GREEN"

# Ask about logs
echo
read -p "Do you want to clean up logs? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
	"$SCRIPT_DIR/cluster.sh" clean
fi
