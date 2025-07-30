# CLAUDE.md – Project Memory for NeuroCache

> **NeuroCache** is a self-hosted CDN and predictive cache.  
> Core: hand-rolled Raft key-value store (Rust) + lock-free slab allocator (Zig).  
> Binaries  
> • `neurod` – runs a Raft node + HTTP/3 edge server  
> • `neuroctl` – client CLI for KV operations and cluster admin
> • `neurotest` – test cluster management tool

---

## 0. Repository layout

```
.
├── raft/                # consensus + storage engine
├── neurod/              # daemon binary  
├── neuroctl/            # client CLI
├── neurotest/           # test cluster management tool
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

# manage test clusters
cargo run --bin neurotest -- start        # Start 3-node cluster
cargo run --bin neurotest -- status       # Check cluster health
cargo run --bin neurotest -- bench        # Run load tests
cargo run --bin neurotest -- stop         # Stop cluster

# client operations (Raft KV)
cargo run --bin neuroctl -- --endpoints 127.0.0.1:3001 put alpha 123
cargo run --bin neuroctl -- --endpoints 127.0.0.1:3001 get alpha

# edge operations (HTTP/3 CDN) - NOT YET IMPLEMENTED
curl http://localhost:8080/cache/my-asset.js
curl -X PUT http://localhost:8080/cache/my-asset.js --data-binary @file.js

# tests and lints
cargo test --workspace
cargo clippy --workspace -- -D warnings
# zig test zig/slab.zig  # NOT YET IMPLEMENTED
```

## 3. Current Implementation Status

### Completed

#### Core Infrastructure
- **neurod daemon**: Runs Raft node (port 3001-3003), HTTP/3 edge server not yet implemented
- **neuroctl CLI**: Full get/put/del commands with endpoint support
- **neurotest CLI**: Rust-based test cluster management replacing bash scripts
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

## 4. Architecture Deep Dive

### What NeuroCache Actually Is

NeuroCache is a **distributed caching CDN** that uses machine learning to predict and pre-fetch content. Think of it as a self-hosted Cloudflare with AI-powered cache warming. The system has two distinct layers:

1. **Control Plane (Raft)**: Manages metadata about what's cached where
2. **Data Plane (Edge Servers)**: Serves actual content from local memory

### How Components Interact

```
Client Request → Edge Server (HTTP/3)
                      ↓
                 Local Slab (HIT) → Serve immediately (~2ms)
                      ↓
                 Local Slab (MISS) → Check Raft metadata
                                          ↓
                                    Found on peer? → Fetch & cache
                                          ↓
                                    Not cached? → Fetch from origin
                                          ↓
                                    Update Raft → All nodes know about it
                                          ↓
                                    ML Engine → Predicts next requests
                                          ↓
                                    PPO Agent → Optimizes cache strategy
```

### What Raft Stores (Metadata Only!)

Raft does NOT store file content. It only stores:

```rust
struct CacheEntry {
    key: String,              // "/assets/app.js"
    content_hash: [u8; 32],   // SHA256 for validation
    size: u64,                // 145KB
    nodes: Vec<NodeId>,       // Which nodes have it cached
    expires_at: Timestamp,    // From Cache-Control headers
    access_count: u64,        // Popularity tracking
}

struct AccessPattern {
    client_id: String,        // Hashed IP
    sequence: Vec<String>,    // ["/index.html", "/app.js", "/app.css"]
    timestamp: Timestamp,
}
```

### Cache Decision Logic

When content is found on another node:
- **< 50MB**: Fetch and cache locally (reduce future latency)
- **> 100MB**: Redirect client to that node (save bandwidth)
- **ML says popular**: Always cache locally
- **Cold content**: Proxy without caching

### End-to-End Example: Cache Hit

1. Client requests `https://cdn.example.com/static/app.js`
2. Edge server checks local Zig slab (memory-mapped file)
3. Found! Serve directly from memory (~2-3ms total)
4. Async: Log access pattern to Raft for ML training
5. ML predicts client will want `/static/app.css` next
6. Pre-fetch app.css in background

### End-to-End Example: Cache Miss

1. Client requests new file `/images/hero.jpg`
2. Local slab doesn't have it
3. Query Raft: "Who has /images/hero.jpg?"
4. Raft says: "Node 1 has it, 245KB, accessed 1523 times"
5. Fetch from Node 1 over internal network (~10ms)
6. Store in local slab for next time
7. Update Raft: "Node 2 also has it now"
8. ML learns this access pattern
9. PPO agent adjusts caching thresholds

### Why This Architecture?

- **Performance**: Cache hits never touch Raft (just memory reads)
- **Consistency**: All nodes agree on what's cached where
- **Intelligence**: ML predicts what to cache before it's requested
- **Flexibility**: Can cache at edge while maintaining consistency

## 5. Development Roadmap

### Phase 1: Complete Raft (Current)
- [ ] Leader election (RequestVote RPC)
- [ ] Log replication with proper indices
- [ ] Safety properties (term checks, etc.)
- [ ] Basic persistence

### Phase 2: Zig Slab Storage
- [ ] Memory-mapped arena allocator
- [ ] FFI bindings to Rust
- [ ] Zero-copy reads
- [ ] Benchmarks vs. standard allocators

### Phase 3: HTTP/3 Edge Server  
- [ ] Quinn for QUIC transport
- [ ] Cache-Control header parsing
- [ ] Origin fetch with timeout/retry
- [ ] Peer-to-peer cache protocol

### Phase 4: ML Integration
- [ ] ONNX runtime for GPT-2 inference
- [ ] Access pattern collection
- [ ] Next-request prediction
- [ ] Pre-fetch queue

### Phase 5: Reinforcement Learning
- [ ] PPO implementation with burn-rs
- [ ] Reward: hit rate + latency reduction
- [ ] Actions: pre-fetch, evict, cache duration
- [ ] Online learning from real traffic

## 6. Testing

Use `neurotest` for all cluster testing:

```bash
# Quick test
cargo run --bin neurotest -- start
cargo run --bin neurotest -- test
cargo run --bin neurotest -- bench --duration 60
cargo run --bin neurotest -- stop

# Monitor performance
cargo run --bin neurotest -- monitor start
# Visit http://localhost:3000 for Grafana dashboards
```

## 7. Open Questions

- TLS handshake details for peer connections
- When to switch from TCP to QUIC for Raft
- Snapshot compression (zstd or raw mmap?)
- Should we add metrics collection before completing Raft?
- Read optimization: Implement ReadIndex to avoid log for read operations
