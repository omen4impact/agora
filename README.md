# Agora - Decentralized P2P Voice Chat

A privacy-first, decentralized voice chat application built with Rust, Tauri, and Flutter.

## Architecture

```
agora/
├── core/           # Rust core library (libp2p, audio, crypto)
├── desktop/        # Tauri desktop app (Windows, macOS, Linux)
├── mobile/         # Flutter mobile app (iOS, Android)
├── cli/            # Command-line testing tool
├── docs/           # Documentation
└── tools/          # Build scripts and utilities
```

## Features (Phase 1)

- [x] Decentralized P2P networking via libp2p
- [x] Cryptographic identity (Ed25519)
- [x] Kademlia DHT for peer discovery
- [x] Room system with shareable links
- [x] CLI testing tool
- [x] NAT traversal framework (AutoNAT, DCUtR, STUN)
- [x] End-to-end encryption framework (Session keys, replay protection)
- [ ] Audio pipeline (Opus, RNNoise)
- [ ] Dynamic mixer selection
- [ ] Desktop UI (Tauri)
- [ ] Mobile app (Flutter FFI)

## Quick Start

### Prerequisites

- Rust 1.70+
- Node.js 18+ (for Tauri)
- Flutter 3.0+ (for Mobile)
- GTK dependencies: `sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`

### CLI Tool

```bash
# Generate identity
cargo run -p agora-cli -- identity --name "YourName"

# Create room
cargo run -p agora-cli -- create-room --name "MyRoom"

# Parse room link
cargo run -p agora-cli -- parse-link "agora://room/abc123"

# Start network node (with NAT traversal)
cargo run -p agora-cli -- start-node --port 4001 --verbose

# Test encryption
cargo run -p agora-cli -- test-encrypt --message "Hello, Agora!"

# Detect NAT type
cargo run -p agora-cli -- detect-nat
```

### Build & Run

```bash
# Build everything
cargo build

# Run tests
cargo test

# Run desktop app
cd desktop && cargo tauri dev

# Run mobile app (requires Flutter)
cd mobile && flutter run
```

## Development Model

This project follows **Shape Up** methodology with 6-week cycles.

See [shape-up-phase1.md](./shape-up-phase1.md) for detailed planning.

## Current Status

### Cycle 1 ✅ Complete
- [x] Project structure
- [x] Identity system (Ed25519)
- [x] Basic network node (libp2p/Kademlia)
- [x] Room system with links
- [x] CLI testing tool
- [x] Flutter mobile scaffold
- [x] Tauri desktop scaffold

### Cycle 2 ✅ Complete
- [x] NAT traversal framework
  - AutoNAT for automatic NAT detection
  - DCUtR for hole punching through relays
  - STUN server configuration
  - NAT type detection (Full Cone, Symmetric, etc.)
- [x] E2E encryption framework
  - Session key management with expiry
  - Encrypted channel with replay attack protection
  - Key derivation for encryption
  - Fingerprint verification

## Test Coverage

```
17 tests passing:
- Identity: generation, serialization, signing
- Room: creation, links, passwords
- NAT: type detection, hole punch capability
- Crypto: encrypt/decrypt, replay protection, key expiry
```

## License

MIT OR Apache-2.0
