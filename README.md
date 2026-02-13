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
- [x] Audio pipeline (cpal, noise gate, audio mixing)
- [x] Mixer algorithm (Full-Mesh → SFU, score-based selection)
- [ ] Opus codec integration
- [ ] Desktop UI (Tauri)
- [ ] Mobile app (Flutter FFI)

## Quick Start

### Prerequisites

- Rust 1.70+
- Node.js 18+ (for Tauri)
- Flutter 3.0+ (for Mobile)
- Linux: `sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev`

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

# List audio devices
cargo run -p agora-cli -- list-audio-devices

# Test audio (5 seconds)
cargo run -p agora-cli -- test-audio --duration 5 --noise-suppression

# Test mixer algorithm
cargo run -p agora-cli -- test-mixer --participants 8
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

### Cycle 3 ✅ Complete
- [x] Audio pipeline
  - cpal integration for cross-platform audio
  - Input/output device enumeration
  - Real-time audio capture and playback
  - Noise gate for background noise reduction
  - Audio mixing for multiple streams
  - RMS and dB level calculation
  - Audio normalization
  - Resampling support

### Cycle 4 ✅ Complete
- [x] Mixer algorithm
  - Full-Mesh mode for ≤5 participants
  - SFU mode for >5 participants
  - Score-based mixer selection
  - Automatic topology switching
  - Mixer rotation (30 min intervals)
  - Tie resolution (deterministic)
  - Participant statistics tracking

## Test Coverage

```
33 tests passing:
- Identity: generation, serialization, signing
- Room: creation, links, passwords
- NAT: type detection, hole punch capability
- Crypto: encrypt/decrypt, replay protection, key expiry
- Audio: config, RMS, dB, normalize, mix, noise gate, resample
- Mixer: topology switching, score calculation, selection, rotation, tie resolution
```

## Mixer Algorithm

### Score Calculation

```
┌─────────────────────────────────────────┐
│ Bandwidth Score    (40% weight)         │
│ Stability Score    (25% weight)         │
│ Resource Score     (20% weight)         │
│ Duration Score     (15% weight)         │
└─────────────────────────────────────────┘
              │
              ▼
    [Highest Score = New Mixer]
              │
              ▼
    [Broadcast decision via DHT]
```

### Topology Switching

```
Participants: 1-5  → Full-Mesh (all-to-all)
Participants: 6+   → SFU (one mixer for all)
```

### Mixer Rotation

- Every 30 minutes
- Duration score resets to force rotation
- Deterministic tie resolution (lexicographically smallest peer ID)

## License

MIT OR Apache-2.0
