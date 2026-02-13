# AGENTS.md - Development Guide for AI Assistants

## Project Overview

Agora is a decentralized P2P voice chat application. This file provides context for AI assistants working on the codebase.

## Tech Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Core Library | Rust, libp2p | P2P networking, identity, crypto |
| Desktop App | Tauri v2, HTML/CSS/JS | Native desktop client |
| Mobile App | Flutter, Dart | iOS/Android client |
| Audio | cpal, Opus, RNNoise | Low-latency audio processing |
| Crypto | Ed25519, Noise Protocol | Identity & E2E encryption |

## Project Structure

```
core/
├── src/
│   ├── lib.rs          # Module exports
│   ├── identity.rs     # Cryptographic identity (Ed25519)
│   ├── network.rs      # libp2p networking layer
│   ├── room.rs         # Room creation and management
│   └── error.rs        # Error types

desktop/
├── src/
│   └── main.rs         # Tauri app entry point
├── ui/
│   └── index.html      # Frontend UI
└── tauri.conf.json     # Tauri configuration
```

## Commands

```bash
# Build everything
cargo build

# Run tests
cargo test

# Run core tests only
cargo test -p agora-core

# Check for linting issues
cargo clippy

# Format code
cargo fmt

# Run desktop app (after Tauri setup)
cd desktop && cargo tauri dev
```

## Key Dependencies

### Workspace (Cargo.toml)
- `libp2p` - P2P networking (Kademlia DHT, Noise, TCP, Yamux)
- `tokio` - Async runtime
- `ed25519-dalek` - Ed25519 signatures
- `serde`/`serde_json` - Serialization

### Core-only
- `multibase` - Base encoding for peer IDs
- `sha2` - SHA-256 hashing
- `hex` - Hex encoding

## Development Workflow

We follow **Shape Up** methodology:

1. **Shaping** (Cool-down): Define problems, set appetite, sketch solutions
2. **Betting**: Decide which pitches to build next cycle
3. **Building** (6 weeks): Implement without scope creep
4. **Cooldown** (2 weeks): Deploy, shape next cycle, rest

### Current Cycle

**Cycle 1: Network Foundation & Identity**
- Pitch 1.1: libp2p Core Integration (6 weeks)
- Pitch 1.2: Identity System (3 weeks, parallel)

## Code Conventions

### Rust
- Use `thiserror` for error types
- Async functions return `Result<T>` from `error.rs`
- Module structure: `pub mod X;` in lib.rs, implementation in X.rs
- Tests in same file with `#[cfg(test)] mod tests`

### Naming
- Types: PascalCase (e.g., `NetworkNode`, `RoomConfig`)
- Functions: snake_case (e.g., `generate_room_id`, `start_network`)
- Constants: SCREAMING_SNAKE_CASE

### Error Handling
```rust
// Use thiserror in error.rs
#[derive(Error, Debug)]
pub enum Error {
    #[error("Description: {0}")]
    Variant(String),
}

// Return Result<T>
pub fn do_something() -> Result<String> {
    // Use ? for error propagation
}
```

## Key Concepts

### Peer ID
- Derived from Ed25519 public key
- Format: `12D3KooW...` (libp2p CID format)
- Persistent across sessions

### Room
- Identified by 16-character hex ID
- Optionally password-protected (SHA-256 hash)
- Shareable via `agora://room/<id>` link

### Network Node
- Swarm-based with Kademlia DHT
- Behaviours: Kademlia, Identify, Ping
- Auto-assigned port if not specified

## What to Run After Changes

1. **After Rust code changes:**
   ```bash
   cargo build
   cargo test
   cargo clippy
   ```

2. **After Cargo.toml changes:**
   ```bash
   cargo build
   ```

3. **Before committing:**
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   ```

## Known Issues / TODOs

- [ ] Tauri icons missing (placeholder needed)
- [ ] Mobile Flutter setup not initialized
- [ ] NAT traversal not implemented
- [ ] Audio pipeline not implemented
- [ ] E2E encryption not implemented

## References

- [libp2p docs](https://docs.libp2p.io/)
- [Tauri v2 guide](https://v2.tauri.app/)
- [Shape Up book](https://basecamp.com/shapeup)
