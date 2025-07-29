#!/bin/bash

# Simple continuous load generator
# Usage: ./simple_load.sh [duration_seconds]

DURATION=${1:-60}
END=$(($(date +%s) + DURATION))

echo "Running load test for ${DURATION}s..."

while [[ $(date +%s) -lt $END ]]; do
    # Random key between 0-99
    KEY=$((RANDOM % 100))
    
    # 50/50 read/write
    if [[ $((RANDOM % 2)) -eq 0 ]]; then
        echo "put key_$KEY value_$RANDOM" | bash test_cluster/cluster.sh test
    else
        echo "get key_$KEY" | bash test_cluster/cluster.sh test
    fi
done &

# Run multiple in parallel
for i in {1..8}; do
    while [[ $(date +%s) -lt $END ]]; do
        KEY=$((RANDOM % 100))
        if [[ $((RANDOM % 2)) -eq 0 ]]; then
            echo "put key_$KEY value_$RANDOM" | bash test_cluster/cluster.sh test
        else
            echo "get key_$KEY" | bash test_cluster/cluster.sh test
        fi
    done &
done

wait
echo "Load test complete"