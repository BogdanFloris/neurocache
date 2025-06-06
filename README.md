# NeuroCache

A self-hosted CDN and predictive cache built from scratch.

## Architecture

- **Consensus**: Hand-rolled Raft implementation in Rust
- **Storage**: Lock-free slab allocator in Zig with FFI to Rust
- **Transport**: TCP with JSON (upgrading to QUIC)
- **Edge API**: HTTP/3 server for CDN semantics
- **ML**: GPT-2 via ONNX/GGML for cache prediction + PPO reinforcement learning

## Binaries

- `neurod` - Raft node + HTTP/3 edge server
- `neuroctl` - CLI client for KV operations and cluster admin

## Quick Start

```bash
# Build everything
cargo build --workspace

# Run a node
cargo run --bin neurod -- --id 1 --config nodes.yaml

# Client operations
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7001 put alpha 123
cargo run --bin neuroctl -- get alpha

# Tests
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

See `CLAUDE.md` for detailed project documentation and development roadmap.
