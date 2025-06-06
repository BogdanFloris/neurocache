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

## 1.1 Architecture: Edge vs Consensus

Each `neurod` instance runs two services on different ports:

### Raft Consensus (port 7000)
- Manages cache metadata only (what's cached where)
- Tracks access patterns and popularity metrics
- Coordinates predictive pre-loading decisions
- Small data requiring strong consistency

### HTTP/3 Edge Server (port 8080)
- Serves cached content directly from local slab
- Cache hits bypass Raft entirely (fast path)
- Cache misses can fetch from origin or peer edges
- Large data prioritizing speed over consistency

This hybrid approach gives us:
- Fast edge performance without consensus overhead for reads
- Consistent metadata for intelligent cache coordination
- Clear separation of concerns between control and data planes
- Realistic CDN behavior while still implementing full Raft

## 2. TODOs (keep each card ≤ 2 h)

### Foundation
- [x] 0-A Workspace scaffolding (`cargo new`, `zig init`)
- [x] 0-B Add core Rust deps (`tokio`, `serde_json`, `tracing`)
- [x] 0-C `neuroctl` initial CLI arguments parsing
- [x] 0-D GitHub Actions CI (test, clippy, fmt check)

### Core Raft Implementation
- [ ] 1 `types.rs` – `Term`, `Index`, `Role` enums
- [ ] 2 `log.rs` – `Vec<Entry>` + append/slice tests
- [ ] 3 `state.rs` – term, voted_for, commit_idx
- [ ] 4 Tick-based election FSM (single node)
- [ ] 5 Raft RPC structs (RequestVote, AppendEntries as JSON)
- [ ] 6 Vote handler (grant / deny logic)
- [ ] 7 AppendEntries handler (accept / reject)
- [ ] 8 `SimNetwork` with mpsc channels + `step()`
- [ ] 9 Happy-path replication test (three nodes)
- [ ] 10 Snapshot to `/tmp/raft-{id}.snap` (gzip JSON)
- [ ] 11 Replace channels with real TCP transport
- [ ] 12 `nodes.yaml` config loader (id, host:port pairs)

### Basic KV Operations
- [ ] 13 `neuroctl put/get/del` commands via Raft
- [ ] 14 `--json` flag for scripting output
- [ ] 15 Leader redirect logic on `NotLeader` error

### Cache/CDN Layer
- [ ] 16 HTTP/3 edge server scaffold on :8080 (using quinn)
- [ ] 17 Cache metadata types: `CacheEntry { key, size, hash, ttl, access_count, edges: Vec<NodeId> }`
- [ ] 18 REST API: `GET /cache/{key}`, `PUT /cache/{key}`, `DELETE /cache/{key}`
- [ ] 19 Edge-to-edge transfer protocol (HTTP/3 with range requests)
- [ ] 20 Origin config: `origins.yaml` with `{ pattern: "*.js", origin: "https://backend.com" }`
- [ ] 21 LRU eviction when slab full (track in Raft metadata)

### Monitoring & Operations
- [ ] 22 `/metrics.json` endpoint (Raft state, cache hit rate, storage usage)
- [ ] 23 Parse `NEURO_ENDPOINTS=` env for client discovery
- [ ] 24 Health check endpoint `/health` (HTTP 200/503)
- [ ] 25 `playground.sh` (launch 3 nodes + demo curl commands)

### Resilience Testing
- [ ] 26 Fuzz test: 5% packet drop simulation
- [ ] 27 Split-brain test (network partition)
- [ ] 28 Chaos test: random node kills

### Visualization
- [ ] 29 SSE endpoint `/wire` for real-time cluster events
- [ ] 30 React `ClusterGraph` D3.js animation
- [ ] 31 Event replay slider with time travel
- [ ] 32 Demo GIF generation + blog post draft

### Advanced Features
- [ ] 33 ONNX runtime integration for GPT-2 inference
- [ ] 34 Predictive pre-loading based on access patterns
- [ ] 35 `burn-rs` PPO agent for cache eviction optimization

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

# run a node (Raft on :7000, HTTP/3 on :8080)
cargo run --bin neurod -- --id 1 --config nodes.yaml

# client operations (Raft KV)
cargo run --bin neuroctl -- --endpoints 127.0.0.1:7000 put alpha 123
cargo run --bin neuroctl -- get alpha

# edge operations (HTTP/3 CDN)
curl http://localhost:8080/cache/my-asset.js
curl -X PUT http://localhost:8080/cache/my-asset.js --data-binary @file.js

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
