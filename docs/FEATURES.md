# Feature Overview - Agora v0.3.0-beta.1

**What works and how it works**

---

## Core Features

### 1. P2P Networking ✅

**How it works:**
- Uses libp2p swarm for peer-to-peer connections
- Kademlia DHT for peer discovery
- AutoNAT for automatic NAT detection
- DCUtR for hole punching through NATs

**Commands:**
```bash
agora start-node --room <room-id>     # Join a room
agora-node start                       # Start dedicated server
```

**What happens:**
1. Node generates Ed25519 identity
2. Announces itself to DHT
3. Discovers other peers in same room
4. Establishes direct P2P connections

---

### 2. End-to-End Encryption ✅

**How it works:**
- ChaCha20-Poly1305 AEAD cipher
- X25519 Diffie-Hellman key exchange
- Noise_XX handshake protocol
- Automatic session key rotation (every hour)
- Replay attack protection with nonces

**Command:**
```bash
agora test-encrypt --message "Hello"
```

**What happens:**
1. Generate ephemeral X25519 keypair
2. Perform Noise_XX handshake
3. Derive shared secret
4. Encrypt with ChaCha20-Poly1305
5. Verify decryption and replay protection

---

### 3. Room Management ✅

**How it works:**
- Rooms identified by 16-char hex ID
- Optional password protection (HKDF + salt hashing)
- Shareable via `agora://room/<id>` links

**Commands:**
```bash
agora create-room --name "My Room"              # Public room
agora create-room --name "Private" --password   # Protected room
agora parse-link "agora://room/abc123"          # Extract room ID
```

**What happens:**
1. Generate random room ID
2. Create shareable link
3. Announce room to DHT
4. Peers discover room via DHT lookup

---

### 4. Audio Pipeline ✅

**How it works:**
- cpal for cross-platform audio I/O
- Opus codec for compression (16-96 kbps)
- RNNoise for ML-based noise suppression
- Acoustic Echo Cancellation (AEC)
- Adaptive bitrate based on network

**Commands:**
```bash
agora list-audio-devices    # Show input/output devices
agora test-audio            # Test audio pipeline
```

**Audio flow:**
```
Microphone → RNNoise (denoise) → Opus (encode) → Encrypt → Network
Network → Decrypt → Opus (decode) → AEC → Speaker
```

---

### 5. Mixer Algorithm ✅

**How it works:**
- Full-Mesh for ≤5 participants (low latency)
- SFU for >5 participants (scalability)
- Automatic topology switching
- Score-based mixer selection
- Deterministic tie resolution

**Command:**
```bash
agora test-mixer --participants 10
```

**Selection criteria:**
| Factor | Weight |
|--------|--------|
| Bandwidth | 40% |
| Stability | 25% |
| CPU/Memory | 20% |
| Session Duration | 15% |

---

### 6. NAT Traversal ✅

**How it works:**
- STUN for public IP detection
- ICE for candidate gathering
- TURN for relay fallback
- UPnP/NAT-PMP for automatic port forwarding
- TCP hole punching for restrictive NATs

**Command:**
```bash
agora detect-nat
```

**Detection flow:**
1. Query STUN servers for public IP
2. Determine NAT type (Full Cone, Symmetric, etc.)
3. Report hole-punching capability
4. Suggest relay if needed

---

### 7. Dedicated Node ✅

**How it works:**
- Headless server for 24/7 operation
- Web dashboard for monitoring
- Prometheus metrics export
- DHT-based node discovery

**Commands:**
```bash
agora-node start --config /etc/agora/node.toml
agora-node status --endpoint http://localhost:8080
agora-node discover --region eu-west
```

**Dashboard endpoints:**
| Endpoint | Purpose |
|----------|---------|
| `/` | HTML dashboard |
| `/api/status` | JSON status |
| `/api/peers` | Connected peers |
| `/metrics` | Prometheus metrics |

---

### 8. Reputation System ✅

**How it works:**
- Proof-of-Bandwidth challenges
- Web-of-Trust vouching
- Score-based trust metrics
- DHT storage for reputation data

**Components:**
- `reputation/score.rs` - Score calculation
- `reputation/challenge.rs` - Bandwidth verification
- `reputation/vouch.rs` - Peer vouching

---

### 9. Identity System ✅

**How it works:**
- Ed25519 cryptographic identity
- Persistent storage in `~/.config/agora/`
- Export/import functionality
- Display name support

**Commands:**
```bash
agora identity                    # Generate & save
agora identity --load             # Load stored
agora export-identity backup.json # Export
agora import-identity backup.json  # Import
```

---

## Platform Support

| Platform | CLI | Node | Desktop | Mobile |
|----------|-----|------|---------|--------|
| Linux x64 | ✅ | ✅ | ✅ | - |
| macOS ARM | ✅ | ✅ | ⚠️ | - |
| Windows x64 | ✅ | ✅ | ⚠️ | - |
| Android | - | - | - | ⚠️ |
| iOS | - | - | - | ⚠️ |
| Web | - | - | - | ✅ |

Legend: ✅ Working | ⚠️ Partial/Needs Fix | - Not Applicable

---

## Known Limitations

1. **Audio devices** - Requires JACK/PulseAudio on Linux
2. **P2P connection** - Needs bootstrap peers for discovery
3. **Mobile builds** - Kotlin/Gradle compatibility issues
4. **Desktop builds** - Tauri release pipeline needs work
5. **TURN servers** - Not yet deployed for relay fallback

---

## Quick Start

```bash
# 1. Download release
curl -sL https://github.com/omen4impact/agora/releases/download/v0.3.0-beta.1/agora-x86_64-unknown-linux-gnu.tar.gz | tar -xzf -

# 2. Generate identity
./agora identity

# 3. Create room
./agora create-room --name "My Room"

# 4. Start node (share the link with others)
./agora start-node --room <room-id>
```