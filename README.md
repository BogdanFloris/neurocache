<div align="center">

# NeuroCache

[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Release](https://img.shields.io/badge/release-v0.1.0-blue.svg)](https://github.com/yourusername/neurocache/releases)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/yourusername/neurocache/actions)

**A self-hosted CDN and predictive cache built from scratch.**

</div>

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
