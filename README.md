# Agora - Decentralized P2P Voice Chat

A privacy-first, decentralized voice chat application with end-to-end encryption. No servers, no tracking, just voice.

## Features

### Core
- **P2P Networking** - Direct peer-to-peer connections via libp2p/Kademlia DHT
- **End-to-End Encryption** - ChaCha20-Poly1305 with X25519 key exchange, Noise_XX handshake
- **NAT Traversal** - STUN, ICE, TURN, UPnP/NAT-PMP, TCP/UDP hole punching
- **Identity** - Ed25519 cryptographic identities, persistent across sessions

### Audio
- **Opus Codec** - 16-96 kbps adaptive bitrate with FEC/DTX
- **RNNoise** - ML-based noise suppression
- **Audio Processing** - Combined pipeline with < 5ms latency

### Infrastructure
- **Dedicated Nodes** - Headless 24/7 mixer/relay nodes
- **Reputation System** - Proof-of-Bandwidth, Web-of-Trust vouching
- **Web Version** - WebRTC-based, PWA installable

## Platforms

| Platform | Status | Technology |
|----------|--------|------------|
| Desktop (Win/Mac/Linux) | ✅ Ready | Tauri v2 |
| Mobile (iOS/Android) | ✅ Ready | Flutter |
| Web (Chrome/Firefox/Safari) | ✅ Ready | Flutter Web + WebRTC |
| Headless Node | ✅ Ready | Rust CLI |

## Quick Start

### Prerequisites

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Linux dependencies
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev

# Flutter (optional, for mobile/web)
flutter doctor
```

### Build & Test

```bash
# Clone
git clone https://github.com/your-org/agora.git
cd agora

# Build
cargo build

# Run tests
cargo test

# Generate documentation
cargo doc --open
```

### CLI Usage

```bash
# Identity management
cargo run -p agora-cli -- identity --name "YourName"
cargo run -p agora-cli -- identity --show

# Room management
cargo run -p agora-cli -- create-room --name "MyRoom"
cargo run -p agora-cli -- parse-link "agora://room/abc123"

# Network
cargo run -p agora-cli -- start-node --port 4001
cargo run -p agora-cli -- detect-nat

# Audio
cargo run -p agora-cli -- list-audio-devices
cargo run -p agora-cli -- test-audio --duration 5
```

### Node Server

```bash
# Start dedicated node
cargo run -p agora-node -- start --config node.toml

# Node commands
cargo run -p agora-node -- status
cargo run -p agora-node -- discover --region eu-west
```

## Architecture

```
agora/
├── core/                    # Rust core library
│   ├── src/
│   │   ├── identity.rs      # Ed25519 identity
│   │   ├── network.rs       # libp2p networking
│   │   ├── crypto.rs        # ChaCha20-Poly1305, X25519
│   │   ├── handshake.rs     # Noise_XX protocol
│   │   ├── stun.rs          # STUN client
│   │   ├── ice.rs           # ICE agent
│   │   ├── turn.rs          # TURN relay
│   │   ├── upnp.rs          # UPnP/NAT-PMP
│   │   ├── audio.rs         # Audio pipeline
│   │   ├── codec/           # Opus encoder/decoder
│   │   ├── denoise/         # RNNoise wrapper
│   │   ├── reputation/      # Reputation system
│   │   └── protocol.rs      # Wire protocol
│   └── benches/             # Performance benchmarks
├── node/                    # Headless node server
│   ├── src/
│   │   ├── main.rs          # CLI entry point
│   │   ├── config.rs        # TOML configuration
│   │   ├── dashboard.rs     # Web dashboard
│   │   ├── metrics.rs       # Prometheus metrics
│   │   ├── signaling.rs     # WebSocket signaling
│   │   └── discovery.rs     # Node discovery
│   └── tests/               # Integration tests
├── cli/                     # CLI testing tool
├── desktop/                 # Tauri desktop app
├── mobile/                  # Flutter mobile/web app
└── docker/                  # Docker deployment
```

## Test Coverage

```
Rust Tests: 232 passing
├── Core: 187 tests
├── Node: 24 tests
├── Config: 14 tests
└── Signaling: 7 tests

Flutter Tests: 16 passing
├── Widget tests: 10
└── WebRTC tests: 6
```

## Deployment

### Docker

```bash
# Build image
docker build -t agora-node .

# Run node
docker run -d -p 7001:7001 -p 8080:8080 agora-node
```

### Systemd

```bash
# Install service
sudo cp docker/agora-node.service /etc/systemd/system/
sudo systemctl enable agora-node
sudo systemctl start agora-node
```

### Node Configuration

```toml
# /etc/agora/node.toml
[node]
mode = "dedicated"
listen_addr = "0.0.0.0:7001"
max_connections = 100

[dashboard]
enabled = true
port = 8080

[metrics]
enabled = true
port = 9090
```

## Development Status

**Phase 1: ✅ COMPLETE** - Core infrastructure
**Phase 2: ✅ COMPLETE** - Community infrastructure

**Status: BETA READY** 🎉

| Cycle | Focus | Status |
|-------|-------|--------|
| 2.1 | NAT Traversal & E2E Encryption | ✅ 100% |
| 2.2 | Opus Codec & RNNoise | ✅ 100% |
| 2.3 | Dedicated Node | ✅ 100% |
| 2.4 | Reputation System | ✅ 100% |
| 2.5 | Desktop UI & CI/CD | ✅ 100% |
| 2.6 | Mobile App | ✅ 100% |
| 2.7 | Web Version (WebRTC) | ✅ 100% |
| 2.8 | Community & Governance | ✅ 100% |

### Test Coverage

```
Rust Tests: 243 passing
├── Unit Tests: 202
├── Integration Tests: 24
├── E2E Tests: 17
└── Node Tests: 14

Flutter Tests: 16 passing
```

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development guidelines.

## Security

See [SECURITY.md](./SECURITY.md) for vulnerability reporting.

## License

MIT OR Apache-2.0
