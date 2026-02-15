# AGENTS.md - Development Guide for AI Assistants

## Project Overview

Agora is a decentralized P2P voice chat application. This file provides context for AI assistants working on the codebase.

## Current Status (Audit: 2026-02-15)

**Phase 1: COMPLETED** ✅  
**Phase 2: COMPLETED** ✅
**Status: BETA RELEASED** 🚀

**Release:** v0.3.0-beta.1 - https://github.com/omen4impact/agora/releases/tag/v0.3.0-beta.1

### Test Coverage
- **Rust**: 243 tests passing (202 unit + 24 integration + 17 E2E + 14 node)
- **Flutter**: 16 tests passing
- **Live Tests**: 29 manual tests passed (see docs/TEST_REPORT.md)

### Security Audit (2026-02-15)
- ✅ Migrated from unmaintained `bincode` to `postcard`
- ✅ Password hashing with HKDF + random salt
- ✅ FFI error handling (no panics)
- ⚠️ Transitive dependency advisories documented in CI (Tauri/GTK, libp2p)

### Cycle 2.8 Progress:
- ✅ README.md Overhaul
- ✅ ARCHITECTURE.md Documentation
- ✅ DEPLOYMENT.md Guide
- ✅ SECURITY.md Policy
- ✅ CONTRIBUTING.md
- ✅ GitHub Issue Templates
- ✅ PR Template
- ✅ rustdoc Generation
- ✅ CHANGELOG.md
- ✅ Release Pipeline (GitHub Actions)
- ✅ E2E Tests
- ✅ Security Hardening (FFI safety docs)

### Cycle 2.7 Progress:
- ✅ WebSocket Signaling Server (`node/src/signaling.rs`)
- ✅ WebRTC Service (`mobile/lib/services/webrtc_service.dart`)
- ✅ Flutter Web Configuration (PWA manifest, index.html)
- ✅ Browser Compatibility Tests (`mobile/test/webrtc_browser_test.dart`)
- ✅ PWA Installation Support (service worker, icons)

### Cycle 2.6 Progress:
- ✅ Flutter Project Setup (mobile/)
- ✅ FFI Bindings (`mobile/lib/services/agora_ffi.dart`)
- ✅ Home Screen UI
- ✅ Identity Service
- ✅ Create/Join Room Screens
- ✅ Audio Integration (`mobile/lib/services/audio_service.dart`)
- ✅ iOS Background Audio (AVAudioSession in AppDelegate.swift)
- ✅ Android Foreground Service (flutter_foreground_task)

### Cycle 2.5 Progress:
- ✅ Settings Panel (Audio, Network, Identity, About tabs)
- ✅ GitHub Actions CI/CD Pipeline
- ✅ Multi-platform builds (Windows, macOS x64/ARM, Linux)
- ✅ FFI unsafe function markers fixed

### Cycle 2.4 Progress:
- ✅ ReputationScore (`reputation/score.rs`) - Score calculation with uptime, performance, reliability
- ✅ Proof-of-Bandwidth Challenges (`reputation/challenge.rs`) - Bandwidth verification
- ✅ Web-of-Trust Vouching (`reputation/vouch.rs`) - Vouching system with stakes
- ✅ DHT-based Reputation Storage

### Cycle 2.3 Progress:
- ✅ Node Configuration (`node/src/config.rs`) - TOML-basierte Konfiguration
- ✅ Headless CLI (`node/src/main.rs`) - start, stop, status, config, identity, discover
- ✅ Web Dashboard (`node/src/dashboard.rs`) - HTML Dashboard, REST API
- ✅ Prometheus Metrics (`node/src/metrics.rs`) - Node Metriken
- ✅ Signal Handling (`node/src/node.rs`) - Graceful Shutdown
- ✅ Docker Image (`Dockerfile`, `docker/docker-compose.yml`)
- ✅ Systemd Service Unit (`docker/agora-node.service`)
- ✅ Node Discovery (`node/src/discovery.rs`) - DHT-based registration

### Cycle 2.2 Progress:
- ✅ Opus Codec (`codec/opus.rs`) - Encoder/Decoder mit FEC/DTX
- ✅ RNNoise Denoiser (`denoise/rnnoise.rs`) - ML-basierte Rauschunterdrückung
- ✅ AudioProcessor (`audio_processor.rs`) - Kombinierte Pipeline
- ✅ Adaptive Bitrate Controller - Netzwerk-adaptive Bitrate

### Cycle 2.1 Progress:
- ✅ STUN Client (`stun.rs`) - Public IP detection, NAT type detection
- ✅ ICE Agent (`ice.rs`) - Candidate gathering, connectivity checks
- ✅ ChaCha20-Poly1305 Encryption (`crypto.rs`) - Real AEAD cipher
- ✅ X25519 Key Exchange - Diffie-Hellman shared secrets
- ✅ ICE Integration with NetworkNode
- ✅ Noise Protocol Handshake (`handshake.rs`) - Noise_XX pattern
- ✅ Session Key Rotation (`crypto.rs`) - Automatic key rotation with `SessionKeyManager`
- ✅ Audio Packet Encryption (`protocol.rs`) - `EncryptedAudioPacket` with `SecureAudioChannel`

**Test Status: 208 Tests bestanden (174 Unit + 24 Integration + 10 Node)**

See:
- `shape-up-phase1-complete.md` - Phase 1 completion report (Audit 2026-02-14)
- `shape-up-phase2.md` - Phase 2 planning (Audit 2026-02-14)
- `shape-up-phase2-cycle2.1.md` - Cycle 2.1 details (Audit 2026-02-14)

## Tech Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Core Library | Rust, libp2p | P2P networking, identity, crypto |
| Desktop App | Tauri v2, HTML/CSS/JS | Native desktop client |
| Mobile App | Flutter, Dart | iOS/Android client |
| Audio | cpal, Opus, RNNoise | Low-latency audio processing |
| Crypto | Ed25519, Noise Protocol, ChaCha20-Poly1305, X25519 | Identity & E2E encryption |
| Serialization | postcard, serde_json | Wire protocol & config |

## Project Structure

```
core/
├── src/
│   ├── lib.rs              # Module exports
│   ├── identity.rs         # Cryptographic identity (Ed25519)
│   ├── storage.rs          # Identity persistence
│   ├── network.rs          # libp2p networking layer (with ICE integration)
│   ├── nat.rs              # NAT traversal utilities (273 lines)
│   ├── stun.rs             # STUN client for NAT detection (241 lines)
│   ├── ice.rs              # ICE agent for hole punching (887 lines)
│   ├── turn.rs             # TURN client for relayed connections (390 lines)
│   ├── upnp.rs             # UPnP/NAT-PMP auto port forwarding (667 lines)
│   ├── crypto.rs           # ChaCha20-Poly1305 encryption with X25519 (1000 lines)
│   ├── handshake.rs        # Noise Protocol handshake (Noise_XX) (424 lines)
│   ├── audio.rs            # Audio pipeline with cpal
│   ├── audio_processor.rs  # Opus + RNNoise + AEC pipeline (500 lines)
│   ├── aec/
│   │   ├── mod.rs          # AEC module exports
│   │   └── echo_canceller.rs # Acoustic Echo Cancellation (500 lines)
│   ├── codec/
│   │   ├── mod.rs          # AudioEncoder/AudioDecoder traits
│   │   └── opus.rs         # Opus encoder/decoder (350 lines)
│   ├── denoise/
│   │   ├── mod.rs          # Denoiser trait
│   │   └── rnnoise.rs      # RNNoise wrapper (120 lines)
│   ├── mixer.rs            # Full-mesh/SFU mixer logic
│   ├── room.rs             # Room creation and management
│   ├── protocol.rs         # Wire protocol (AudioPacket, EncryptedAudioPacket)
│   ├── reputation/
│   │   ├── mod.rs          # Reputation module exports
│   │   ├── score.rs        # Reputation score calculation (230 lines)
│   │   ├── challenge.rs    # Proof-of-Bandwidth challenges (230 lines)
│   │   └── vouch.rs        # Web-of-Trust vouching (300 lines)
│   ├── ffi.rs              # FFI bindings for mobile
│   └── error.rs            # Error types

node/
├── Cargo.toml              # Dependencies (axum, prometheus, etc.)
└── src/
    ├── main.rs             # CLI entry point (start, stop, status, config, discover)
    ├── config.rs           # TOML configuration (340 lines)
    ├── dashboard.rs        # Web dashboard & API (310 lines)
    ├── discovery.rs        # Node discovery & DHT registration (320 lines)
    ├── metrics.rs          # Prometheus metrics (130 lines)
    ├── node.rs             # Node runtime (310 lines)
    └── error.rs            # Error types

cli/
├── src/
│   └── main.rs         # CLI interface
└── Cargo.toml

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

# Run CLI
cargo run -p agora-cli -- identity --help

# Run desktop app (after Tauri setup)
cd desktop && cargo tauri dev
```

## CLI Commands

```bash
# Identity Management
agora identity                    # Create new identity & save
agora identity --load             # Load stored identity
agora identity --show             # Show stored identity
agora save-identity --name "Bob"  # Set name & save
agora delete-identity             # Delete identity
agora export-identity backup.json # Export
agora import-identity backup.json # Import

# Room Management
agora create-room --name "Gaming"
agora create-room --password "secret"
agora parse-link "agora://room/abc123"

# Networking
agora start-node --port 7001
agora start-node --room abc123
agora start-node --bootstrap "/ip4/.../tcp/..."

# Testing
agora test-audio --duration 10
agora test-mixer --participants 10
agora test-encrypt --message "Hello"
agora detect-nat
agora list-audio-devices
```

## Node Commands

```bash
# Node Management
agora-node start --config /etc/agora/node.toml
agora-node stop
agora-node status --endpoint http://localhost:8080
agora-node config --output node.toml
agora-node identity --output identity.bin --name "MyNode"
agora-node discover --endpoint http://localhost:8080 --region eu-west --capability mixer
```

## Node Dashboard Endpoints

```
GET /              # HTML Dashboard
GET /api/status    # JSON Status
GET /api/peers     # Connected Peers
GET /api/discover  # Discover nodes (query: region, capability)
GET /health        # Health Check
GET /metrics       # Prometheus Metrics
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
- `cpal` - Cross-platform audio I/O
- `dirs` - OS directory paths
- `stun` - STUN protocol
- `chacha20poly1305` - AEAD cipher
- `x25519-dalek` - X25519 key exchange
- `snow` - Noise protocol framework
- `hkdf` - Key derivation
- `base64` - Encoding
- `postcard` - No_std compatible serialization (replaces unmaintained bincode)

## Development Workflow

We follow **Shape Up** methodology:

1. **Shaping** (Cool-down): Define problems, set appetite, sketch solutions
2. **Betting**: Decide which pitches to build next cycle
3. **Building** (6 weeks): Implement without scope creep
4. **Cooldown** (2 weeks): Deploy, shape next cycle, rest

### Current Status

**Phase 2: COMPLETE** ✅
- All 8 cycles completed: 2.1-2.8

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
- Persistent across sessions (stored in `~/.config/agora/identity.bin`)

### Room
- Identified by 16-character hex ID
- Optionally password-protected (SHA-256 hash)
- Shareable via `agora://room/<id>` link

### Network Node
- Swarm-based with Kademlia DHT
- Behaviours: Kademlia, Identify, Ping, AutoNAT, DCUtR
- Auto-assigned port if not specified

### Mixer
- Full-Mesh for ≤5 participants
- SFU for >5 participants
- Score-based mixer selection (bandwidth, stability, resources, duration)
- Automatic rotation every 30 minutes

### NAT Traversal (Cycle 2.1)
- **STUN**: Public IP/Port detection, NAT type detection
- **ICE**: Candidate gathering (host, srflx, relayed), connectivity checks
- **TURN**: Fallback for Symmetric NAT
- **UPnP/NAT-PMP**: Automatic port forwarding

### Encryption (Cycle 2.1)
- **ChaCha20-Poly1305**: AEAD cipher for audio packets
- **X25519**: Diffie-Hellman key exchange
- **Noise_XX**: Mutual authentication handshake
- **Session Key Rotation**: Automatic every hour
- **Replay Protection**: Nonce-based counter

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

### Cycle 2.1 - 100% COMPLETE ✅
- [x] Real E2E encryption (ChaCha20-Poly1305)
- [x] X25519 Key Exchange
- [x] STUN Client Implementation
- [x] ICE Agent Implementation
- [x] ICE Integration with NetworkNode
- [x] Noise Protocol Handshake
- [x] Session Key Rotation
- [x] Audio Packet Encryption
- [x] TURN Server Support (fallback for Symmetric NAT)
- [x] UPnP/NAT-PMP Auto-Port-Forwarding
- [x] TCP Hole-Punching (`core/src/tcp_punch.rs`)
- [x] Network Tests (comprehensive tests in modules)
- [x] Performance Benchmarks (`core/benches/` with criterion)

### Cycle 2.2 - 100% COMPLETE ✅
- [x] Opus codec integration (Encoder/Decoder with FEC/DTX)
- [x] RNNoise for noise suppression
- [x] AudioProcessor integration
- [x] Adaptive Bitrate Controller

### Cycle 2.3 - 100% COMPLETE ✅
- [x] Node Configuration (TOML-based)
- [x] Headless CLI (agora-node binary)
- [x] Web Dashboard & REST API
- [x] Prometheus Metrics Export
- [x] Signal Handling & Graceful Shutdown
- [x] Docker Image (multi-stage build, health check)
- [x] Systemd Service Unit (hardened security)
- [x] Node Discovery (DHT-based registration)

### Cycle 2.4 - 100% COMPLETE ✅
- [x] ReputationScore calculation (uptime, performance, reliability, challenge)
- [x] Proof-of-Bandwidth Challenges
- [x] Web-of-Trust Vouching with stakes
- [x] DHT-based Reputation Storage

### Cycle 2.5 - 100% COMPLETE ✅
- [x] Settings Panel (Audio, Network, Identity, About tabs)
- [x] GitHub Actions CI/CD Pipeline
- [x] Multi-platform builds (Windows, macOS x64/ARM, Linux)
- [x] FFI unsafe function markers fixed

### Cycle 2.6 - 100% COMPLETE ✅
- [x] Flutter Project Setup
- [x] FFI Bindings (agora_ffi.dart)
- [x] Home Screen UI
- [x] Identity Service
- [x] Create/Join Room Screens
- [x] Audio Integration (audio_service.dart with WebRTC)
- [x] iOS Background Audio (AVAudioSession in AppDelegate.swift)
- [x] Android Foreground Service (flutter_foreground_task)

### Cycle 2.7 - 100% COMPLETE ✅
- [x] WebSocket Signaling Server (`node/src/signaling.rs`)
- [x] WebRTC Service (`mobile/lib/services/webrtc_service.dart`)
- [x] Flutter Web Build working
- [x] Browser Compatibility Tests
- [x] PWA Installation Support

### Cycle 2.8 - 100% COMPLETE ✅
- [x] README.md Overhaul
- [x] ARCHITECTURE.md Documentation
- [x] DEPLOYMENT.md Guide
- [x] SECURITY.md Policy
- [x] CONTRIBUTING.md
- [x] GitHub Issue Templates
- [x] PR Template
- [x] rustdoc Generation

### Future Work
- [x] Echo Cancellation (WebRTC AEC) ✅
- [x] Security: bincode → postcard migration ✅
- [x] Security: Password hashing with salt ✅
- [x] Beta Release ✅
- [ ] Phase 3: Production Ready
- [ ] Community Building

## Documentation

### User Documentation
- [docs/FEATURES.md](./docs/FEATURES.md) - Feature overview and usage
- [docs/TEST_REPORT.md](./docs/TEST_REPORT.md) - Test results and verification
- [docs/ROADMAP.md](./docs/ROADMAP.md) - Phase 3 planning and priorities
- [docs/DEPLOYMENT.md](./docs/DEPLOYMENT.md) - Deployment guide
- [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) - System architecture

### References

- [libp2p docs](https://docs.libp2p.io/)
- [Tauri v2 guide](https://v2.tauri.app/)
- [Shape Up book](https://basecamp.com/shapeup)
- [Opus Codec](https://opus-codec.org/)
- [RNNoise](https://jmvalin.ca/demo/rnnoise/)
- [konzept.md](./konzept.md) - Full project concept (3-Phase Strategy)
- [shape-up-phase1.md](./shape-up-phase1.md) - Phase 1 planning
- [shape-up-phase1-complete.md](./shape-up-phase1-complete.md) - Phase 1 completion report (Audit 2026-02-14)
- [shape-up-phase2.md](./shape-up-phase2.md) - Phase 2 planning (Audit 2026-02-14)
- [shape-up-phase2-cycle2.1.md](./shape-up-phase2-cycle2.1.md) - Cycle 2.1 details (Audit 2026-02-14)
- [shape-up-phase2-cycle2.2.md](./shape-up-phase2-cycle2.2.md) - Cycle 2.2 details (2026-02-14)