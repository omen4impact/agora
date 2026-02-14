# Architecture Overview

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                           Clients                                │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Desktop    │  │    Mobile    │  │     Web      │          │
│  │   (Tauri)    │  │   (Flutter)  │  │  (WebRTC)    │          │
│  │              │  │              │  │              │          │
│  │  Rust Core   │  │  FFI Bridge  │  │  WebSocket   │          │
│  │  Direct      │  │  to Core     │  │  Signaling   │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                 │                 │                   │
└─────────┼─────────────────┼─────────────────┼───────────────────┘
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │
┌───────────────────────────┼───────────────────────────────────┐
│                      Core Library                               │
│                            │                                    │
│  ┌─────────────────────────┼────────────────────────────────┐ │
│  │                         │                                │ │
│  │  ┌─────────────┐  ┌─────┴─────┐  ┌─────────────┐        │ │
│  │  │   Network   │  │   Audio   │  │    Crypto   │        │ │
│  │  │             │  │           │  │             │        │ │
│  │  │ • libp2p    │  │ • cpal    │  │ • ChaCha20  │        │ │
│  │  │ • Kademlia  │  │ • Opus    │  │ • X25519    │        │ │
│  │  │ • Noise     │  │ • RNNoise │  │ • Noise_XX  │        │ │
│  │  │ • STUN/ICE  │  │ • Mixer   │  │ • Ed25519   │        │ │
│  │  └─────────────┘  └───────────┘  └─────────────┘        │ │
│  │                                                          │ │
│  │  ┌─────────────┐  ┌───────────┐  ┌─────────────┐        │ │
│  │  │    Room     │  │    NAT    │  │ Reputation  │        │ │
│  │  │             │  │           │  │             │        │ │
│  │  │ • Create    │  │ • STUN    │  │ • Score     │        │ │
│  │  │ • Join      │  │ • ICE     │  │ • Challenge │        │ │
│  │  │ • Password  │  │ • TURN    │  │ • Vouch     │        │ │
│  │  │ • Links     │  │ • UPnP    │  │ • Storage   │        │ │
│  │  └─────────────┘  └───────────┘  └─────────────┘        │ │
│  │                                                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
└────────────────────────────────────────────────────────────────┘
                            │
┌───────────────────────────┼───────────────────────────────────┐
│                     Node (Headless)                             │
│                            │                                    │
│  ┌─────────────────────────┼────────────────────────────────┐ │
│  │                                                          │ │
│  │  ┌─────────────┐  ┌───────────┐  ┌─────────────┐        │ │
│  │  │  Signaling  │  │ Dashboard │  │   Metrics   │        │ │
│  │  │             │  │           │  │             │        │ │
│  │  │ • WebSocket │  │ • HTTP    │  │ • Prometheus│        │ │
│  │  │ • SDP/ICE   │  │ • REST    │  │ • Grafana   │        │ │
│  │  │ • Rooms     │  │ • Status  │  │ • Alerts    │        │ │
│  │  └─────────────┘  └───────────┘  └─────────────┘        │ │
│  │                                                          │ │
│  │  ┌─────────────┐  ┌───────────┐  ┌─────────────┐        │ │
│  │  │  Discovery  │  │   Config  │  │   Runtime   │        │ │
│  │  │             │  │           │  │             │        │ │
│  │  │ • DHT Reg   │  │ • TOML    │  │ • Signals   │        │ │
│  │  │ • Broadcast │  │ • CLI     │  │ • Graceful  │        │ │
│  │  │ • Query     │  │ • Env     │  │ • Logging   │        │ │
│  │  └─────────────┘  └───────────┘  └─────────────┘        │ │
│  │                                                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## Components

### Core Library (`core/`)

| Module | File | Purpose |
|--------|------|---------|
| Identity | `identity.rs` | Ed25519 keypair, signing, peer ID |
| Network | `network.rs` | libp2p swarm, Kademlia DHT, behaviors |
| Crypto | `crypto.rs` | ChaCha20-Poly1305, X25519, session keys |
| Handshake | `handshake.rs` | Noise_XX protocol handshake |
| STUN | `stun.rs` | Public IP detection, NAT type |
| ICE | `ice.rs` | Candidate gathering, connectivity |
| TURN | `turn.rs` | Relay for symmetric NAT |
| UPnP | `upnp.rs` | Auto port forwarding |
| Audio | `audio.rs` | cpal integration, device enumeration |
| AEC | `aec/echo_canceller.rs` | Acoustic echo cancellation |
| Codec | `codec/opus.rs` | Opus encoder/decoder |
| Denoise | `denoise/rnnoise.rs` | Noise suppression |
| Mixer | `mixer.rs` | Full-mesh/SFU topology |
| Room | `room.rs` | Room creation, management |
| Protocol | `protocol.rs` | Wire format, packets |
| Reputation | `reputation/` | Score, challenges, vouching |

### Node Server (`node/`)

| Module | File | Purpose |
|--------|------|---------|
| Main | `main.rs` | CLI commands (start, stop, status) |
| Config | `config.rs` | TOML configuration parsing |
| Dashboard | `dashboard.rs` | Web UI, REST API |
| Metrics | `metrics.rs` | Prometheus export |
| Signaling | `signaling.rs` | WebSocket for WebRTC |
| Discovery | `discovery.rs` | Node registration in DHT |

## Data Flow

### Room Join Flow

```
┌─────────┐                    ┌─────────┐                    ┌─────────┐
│ Client  │                    │   DHT   │                    │  Room   │
│         │                    │         │                    │ Creator │
└────┬────┘                    └────┬────┘                    └────┬────┘
     │                              │                              │
     │  1. Find room in DHT         │                              │
     │ ─────────────────────────────>│                              │
     │                              │                              │
     │  2. Return room peers        │                              │
     │ <─────────────────────────────│                              │
     │                              │                              │
     │  3. Connect to peers         │                              │
     │ ────────────────────────────────────────────────────────────>
     │                              │                              │
     │  4. Noise_XX Handshake       │                              │
     │ <───────────────────────────────────────────────────────────>
     │                              │                              │
     │  5. Exchange X25519 keys     │                              │
     │ <───────────────────────────────────────────────────────────>
     │                              │                              │
     │  6. Derive session key       │                              │
     │                              │                              │
     │  7. Encrypted audio stream   │                              │
     │ <───────────────────────────────────────────────────────────>
     │                              │                              │
```

### Audio Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                       Audio Pipeline                             │
│                                                                  │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐     │
│  │ Micro-  │    │   AEC   │    │ RNNoise │    │  Opus   │     │
│  │ phone   │───>│  Echo   │───>│ Denoise │───>│ Encode  │     │
│  │         │    │ Cancel  │    │         │    │         │     │
│  └─────────┘    └─────────┘    └─────────┘    └────┬────┘     │
│                                                     │           │
│                                               ┌─────┴─────┐     │
│                                               │   Network │     │
│                                               │   Send    │     │
│                                               └───────────┘     │
│                                                                  │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐     │
│  │ Speaker │    │  Mixer  │    │  Opus   │    │ Decrypt │     │
│  │         │<───│  Mix    │<───│ Decode  │<───│ ChaCha  │     │
│  │         │    │         │    │         │    │         │     │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Security Model

### Encryption

```
┌─────────────────────────────────────────────────────────────┐
│                    Encryption Layers                         │
│                                                              │
│  Layer 1: Transport (Noise Protocol)                        │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ • Noise_XX handshake with Ed25519 identity          │    │
│  │ • Mutual authentication                             │    │
│  │ • Forward secrecy via X25519 key rotation           │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Layer 2: Application (ChaCha20-Poly1305)                   │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ • Per-room session keys                             │    │
│  │ • Automatic key rotation (1 hour)                   │    │
│  │ • Replay protection (nonce counter)                 │    │
│  │ • AEAD authentication                               │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Identity

```
┌─────────────────────────────────────────────────────────────┐
│                    Identity System                           │
│                                                              │
│  Ed25519 Keypair                                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Private Key: 32 bytes (stored in ~/.config/agora/)  │    │
│  │ Public Key:  32 bytes                               │    │
│  │ Peer ID:     libp2p CID format (12D3KooW...)        │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Usage                                                       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ • Sign messages (authentication)                    │    │
│  │ • Derive X25519 keys (encryption)                   │    │
│  │ • Generate Peer ID (networking)                     │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Mixer Topology

### Selection Algorithm

```
┌─────────────────────────────────────────────────────────────┐
│                    Mixer Selection                           │
│                                                              │
│  Participants ≤ 5: Full-Mesh                                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │   A ←────────→ B                                     │    │
│  │    ↑            ↑                                    │    │
│  │    │            │                                    │    │
│  │    ↓            ↓                                    │    │
│  │   C ←────────→ D                                     │    │
│  │                                                      │    │
│  │   All connect to all (n-1 connections each)         │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Participants > 5: SFU                                       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                  [Mixer]                             │    │
│  │                   /│\                               │    │
│  │                  / │ \                              │    │
│  │                 /  │  \                             │    │
│  │               A    B    C ...                       │    │
│  │                                                      │    │
│  │   One mixer, all connect to mixer (1 connection)    │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Score = Bandwidth(40%) + Stability(25%) +                   │
│          Resources(20%) + Duration(15%)                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## NAT Traversal Strategy

```
┌─────────────────────────────────────────────────────────────┐
│                    NAT Traversal                             │
│                                                              │
│  1. Try UPnP/NAT-PMP (auto port forward)                    │
│     ┌─────────────────────────────────────────────────┐     │
│     │ Router supports UPnP? → Open port automatically │     │
│     └─────────────────────────────────────────────────┘     │
│                           │                                  │
│                           ▼ (failed)                         │
│                                                              │
│  2. STUN + ICE (hole punching)                              │
│     ┌─────────────────────────────────────────────────┐     │
│     │ Get public IP:port, gather candidates           │     │
│     │ Try direct UDP/TCP connection                   │     │
│     │ Success rate: ~85% for Cone NAT                 │     │
│     └─────────────────────────────────────────────────┘     │
│                           │                                  │
│                           ▼ (failed)                         │
│                                                              │
│  3. TURN Relay (fallback)                                   │
│     ┌─────────────────────────────────────────────────┐     │
│     │ Relay through TURN server                       │     │
│     │ Works with all NAT types                        │     │
│     │ Cost: bandwidth, latency                        │     │
│     └─────────────────────────────────────────────────┘     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```
