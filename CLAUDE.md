# CLAUDE.md – Project Memory for NeuroCache

> **NeuroCache** is a self-hosted CDN and predictive cache.  
> Core: hand-rolled Raft key-value store (Rust) + lock-free slab allocator (Zig).  
> Binaries  
> • `neurod` – runs a Raft node + HTTP/3 edge server  
> • `neuroctl` – client CLI for KV operations and cluster admin

---

## 0. Repository layout

```
.
├── crates/
│   ├── raft/          # consensus + storage engine
│   ├── neurod/          # daemon binary
│   └── neuroctl/        # client CLI
├── zig/                 # slab.zig (+ future SIMD helpers)
├── web/                 # Vite/React visual dashboard
└── docs/
    └── benchmarks/

```

## 1. Technology choices

| Layer      | Tool / Library                             | Reason                                 |
| ---------- | ------------------------------------------ | -------------------------------------- |
| Consensus  | custom Raft (async Rust)                   | learning goal, full control            |
| Transport  | TCP with length-prefixed JSON (QUIC later) | easiest first step, upgrade path ready |
| Storage    | `slab.zig` (mmap arena)                    | zero-copy, lock-free, FFI to Rust      |
| Edge API   | HTTP/3                                     | modern CDN semantics                   |
| Prediction | small GPT-2 via ONNX / GGML                | self-hosted inference                  |
| RL         | PPO with `burn-rs`                         | online cache optimiser                 |

## 2. TODOs (keep each card ≤ 2 h)

- [x] 0-A Workspace scaffolding (`cargo new`, `zig init`)
- [x] 0-B Add core Rust deps (`tokio`, `serde_json`, `tracing`)
- [x] 0-C `neuroctl` initial CLI arguments parsing
- [ ] 1 `types.rs` – `Term`, `Index`, `Role`
- [ ] 2 `log.rs` – `Vec<Entry>` + append/slice tests
- [ ] 3 `state.rs` – term, voted_for, commit_idx
- [ ] 4 Tick-based election FSM (single node)
- [ ] 5 Raft RPC structs (JSON)
- [ ] 6 Vote handler (grant / deny)
- [ ] 7 AppendEntries handler
- [ ] 8 `SimNetwork` with mpsc channels + `step()`
- [ ] 9 Happy-path replication test (three nodes)
- [ ] 10 Snapshot to `/tmp/raft-#.snap`
- [ ] 11 Replace channels with real TCP transport
- [ ] 12 `nodes.yaml` config loader
- [ ] 13 `neuroctl put/get` commands
- [ ] 14 `--json` flag for scripting
- [ ] 15 `/metrics.json` endpoint
- [ ] 16 Parse `NEURO_ENDPOINTS=` env / flag
- [ ] 17 Redirect logic on `NotLeader`
- [ ] 18 Fuzz test: 5 % packet drop
- [ ] 19 `playground.sh` (launch 3 nodes + demo ops)
- [ ] 20 SSE endpoint `/wire` for Web Viz
- [ ] 21 React `ClusterGraph` animation + replay slider
- [ ] 22 GIF + launch blog-post draft

## 3. Coding conventions

- Write the failing test before Raft logic.
- No panic! in library crates; binaries/tests only.
- Rust ↔ Zig boundary: extern "C"; no Zig headers leak upward.
- Logging: TRACE for IPC frames, DEBUG for elections, INFO for leadership changes.
- Message framing: 4-byte big-endian length prefix before each JSON blob.

## 4. Quick commands

```
# build everything
cargo build --workspace

# run a node
cargo run --bin neurod -- --id 1 --config nodes.yaml

# client operations
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7001 put alpha 123
cargo run --bin neuroctl -- get alpha

# tests and lints
cargo test --workspace
cargo clippy --workspace -- -D warnings
zig test zig/slab.zig
```

## 5. Open questions

- TLS handshake details
- When to switch from TCP to QUIC
- Snapshot compression (zstd or raw mmap?)

Add new decisions or issues here as the project evolves.
