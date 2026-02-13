# Agora - Decentralized P2P Voice Chat

A privacy-first, decentralized voice chat application built with Rust, Tauri, and Flutter.

## Architecture

```
agora/
├── core/           # Rust core library (libp2p, audio, crypto)
├── desktop/        # Tauri desktop app (Windows, macOS, Linux)
├── mobile/         # Flutter mobile app (iOS, Android)
├── docs/           # Documentation
└── tools/          # Build scripts and utilities
```

## Features (Phase 1)

- [x] Decentralized P2P networking via libp2p
- [x] Cryptographic identity (Ed25519)
- [x] Kademlia DHT for peer discovery
- [ ] NAT traversal (Hole punching, STUN/TURN)
- [ ] End-to-end encryption (Noise Protocol)
- [ ] Audio pipeline (Opus, RNNoise)
- [ ] Dynamic mixer selection
- [ ] Desktop UI (Tauri)
- [ ] Mobile app (Flutter)

## Quick Start

### Prerequisites

- Rust 1.70+
- Node.js 18+
- Tauri CLI: `cargo install tauri-cli`

### Build & Run

```bash
# Build core library
cargo build

# Run tests
cargo test

# Run desktop app (requires Tauri setup)
cd desktop
cargo tauri dev
```

## Development Model

This project follows **Shape Up** methodology with 6-week cycles.

See [shape-up-phase1.md](./shape-up-phase1.md) for detailed planning.

## Current Status

**Cycle 1, Pitch 1.1**: libp2p Core Integration

- [x] Project structure
- [x] Identity system
- [x] Basic network node
- [ ] Room system integration
- [ ] CLI testing tool

## License

MIT OR Apache-2.0
