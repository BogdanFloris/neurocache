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
├── raft/                # consensus + storage engine
├── neurod/              # daemon binary  
├── neuroctl/            # client CLI
├── test_cluster/        # test configuration files (node_1.json, etc.)
├── Cargo.toml           # workspace root

Not yet created:
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

### Raft Consensus (port 7001)

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

## 2. Coding conventions

- Write the failing test before Raft logic.
- No panic! in library crates; binaries/tests only.
- Rust ↔ Zig boundary: extern "C"; no Zig headers leak upward.
- Logging: TRACE for IPC frames, DEBUG for elections, INFO for leadership changes.
- Message framing: 4-byte big-endian length prefix before each JSON blob.

## 2. Quick commands

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

## 3. Current Implementation Status

### Completed

#### Core Infrastructure
- **neurod daemon**: Runs Raft node (port 7000), HTTP/3 edge server not yet implemented
- **neuroctl CLI**: Full get/put/del commands with endpoint support
- **In-memory KvStore**: HashMap-based state machine with proper StateMachine trait implementation
- **JSON Frame codec**: Length-delimited (4-byte BE header) + JSON serialization
- **Error handling**: NotFound, InvalidKey, proper error propagation
- **Timeout support**: 5s read/write timeouts in client

#### Raft Implementation (Partial)
- **Basic networking**: PeerNetwork with TCP listener and connection pooling
- **OutboundPool**: Automatic reconnection, lazy connection establishment, per-peer channels
- **Message types**: AppendEntries, AppendEntriesResponse, ClientCommand/Response (missing RequestVote)
- **Log structure**: Vector-based with sentinel entry, Raft consistency checks, comprehensive tests
- **Heartbeats**: 1-second interval empty AppendEntries
- **Direct command execution**: No consensus yet, commands applied directly to state machine

#### Missing Components
- **No Zig integration**: slab.zig allocator not implemented
- **No HTTP/3 edge server**: Port 8080 functionality not implemented
- **No web dashboard**: Vite/React visual dashboard not started
- **No ML/prediction**: GPT-2/ONNX integration not implemented
- **No RL optimization**: PPO with burn-rs not implemented

### Not Yet Implemented

#### Raft Consensus
- Leader election (RequestVote/RequestVoteResponse)
- Role management (Follower/Candidate/Leader states)
- Persistent state (currentTerm, votedFor, log)
- Log replication with match/next indices
- Commit index and last applied tracking
- Snapshot support
- Membership changes

#### Edge CDN Features
- HTTP/3 server on port 8080
- Cache storage and retrieval
- Origin server fetch on cache miss
- Peer-to-peer cache sharing
- Cache metadata in Raft

## 4. Open questions

- TLS handshake details
- When to switch from TCP to QUIC
- Snapshot compression (zstd or raw mmap?)
- Should we add metrics collection before Raft implementation?
- Right now, reads also go through the log. Think about improving processing speed by confirming leadership without going through the log (we need to confirm leadership to make sure we don't serve stale reads). This should be done by splitting read requests from write requests, adding read requests to a pending read requests queue, and sending a heartbeat to confirm leadership. If leadership is confirmed we can process read requests from the queue. (This is the ReadIndex approach used by etcd)
