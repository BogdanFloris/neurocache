<div align="center">

# NeuroCache

<a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/1.75+-orange?style=flat&logo=rust&label=Rust&color=%23dea584"></a>
<a href="https://github.com/BogdanFloris/neurocache/releases"><img src="https://img.shields.io/github/v/release/BogdanFloris/neurocache?link=https%3A%2F%2Fgithub.com%2FBogdanFloris%2Fneurocache%2Freleases"></a>
<a href="https://github.com/BogdanFloris/neurocache/actions"><img src="https://img.shields.io/github/actions/workflow/status/BogdanFloris/neurocache/ci.yml?link=https%3A%2F%2Fgithub.com%2FBogdanFloris%2Fneurocache%2Factions"></a>

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
- `neurotest` - Test cluster management tool (see [`neurotest/README.md`](neurotest/README.md))

## Quick Start

```bash
# Build everything
cargo build --workspace

# Run a node
cargo run --bin neurod -- -c test_cluster/node_1.json

# Client operations
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7001 put alpha 123
cargo run --bin neuroctl -- get alpha

# Tests
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

See `CLAUDE.md` for detailed project documentation and development roadmap.
