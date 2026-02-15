# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **Desktop App Initialization**: Fixed Tauri v2 JavaScript API integration
  - Added `withGlobalTauri: true` to make `window.__TAURI__` available
  - Fixed snake_case vs camelCase argument naming mismatch
  - Reverted to direct `invoke()` pattern instead of event-based init
  - All commands now use `#[tauri::command(rename_all = "snake_case")]`

## [0.3.0-beta.1] - 2026-02-14

### First Beta Release! 🎉

This is the first beta release of Agora, a decentralized P2P voice chat application.

#### Features
- **P2P Networking**: libp2p-based peer discovery and communication
- **E2E Encryption**: ChaCha20-Poly1305 + X25519 + Noise_XX handshake
- **NAT Traversal**: STUN, ICE, TURN, UPnP, TCP hole-punching
- **Audio**: Opus codec, RNNoise denoising, Acoustic Echo Cancellation
- **Reputation**: Proof-of-Bandwidth challenges, Web-of-Trust vouching
- **Multi-Platform**: Desktop (Tauri), Mobile (Flutter), Headless Node

#### Security
- **bincode → postcard Migration**: Replaced unmaintained bincode
- **Password Hashing**: HKDF + random salt
- **FFI Safety**: No panics in FFI functions

#### Test Coverage
- 243 Rust tests (unit + integration + E2E)
- 16 Flutter tests

---

## [0.2.9] - 2026-02-14

### Security
- **bincode → postcard Migration**: Replaced unmaintained bincode with postcard
  - Resolves RUSTSEC-2025-0141 (bincode is unmaintained)
  - postcard is actively maintained and no_std compatible
  - Smaller serialized size for wire protocol
  - Updated: `storage.rs`, `protocol.rs`

- **Password Hashing**: Improved password security with HKDF + random salt
  - Replaced static SHA-256 with HKDF-based key derivation
  - Each password hash now has a unique salt
  - Protects against rainbow table attacks

- **FFI Safety**: Improved FFI error handling
  - Removed panics from FFI functions
  - Added proper error string returns
  - Documented safety requirements

### Changed
- Updated CI to document ignored security advisories (transitive dependencies)
- Fixed all clippy warnings across workspace
- Added `#[allow(dead_code)]` for intentional public API functions

## [0.2.8] - 2026-02-14

### Added
- **Echo Cancellation**: Acoustic Echo Cancellation (AEC) module with adaptive filter
  - `core/src/aec/echo_canceller.rs` - 500+ lines
  - Double-talk detection
  - Residual echo suppression
  - Integrated into AudioProcessor

- **E2E Tests**: Comprehensive end-to-end test suite
  - `core/tests/e2e_tests.rs` - 17 tests
  - Encryption/decryption roundtrip
  - Audio pipeline latency tests
  - Key exchange verification

- **Release Pipeline**: GitHub Actions release workflow
  - Multi-platform builds (Linux, macOS x64/ARM, Windows)
  - Mobile builds (Android APK, Web)
  - Automatic GitHub releases

- **Security Hardening**: FFI safety documentation
  - Added `# Safety` comments to all unsafe functions
  - Documented pointer validity requirements

### Changed
- Updated CI workflow with security audit and Flutter tests
- Improved AudioProcessor with AEC integration
- Updated documentation (README, ARCHITECTURE, DEPLOYMENT)

## [0.2.7] - 2026-02-14

### Added
- **WebSocket Signaling Server**: `node/src/signaling.rs`
  - Room/peer management
  - SDP/ICE exchange
  - Broadcast channels

- **WebRTC Service**: `mobile/lib/services/webrtc_service.dart`
  - RTCPeerConnection management
  - Audio stream via getUserMedia

- **Flutter Web**: PWA support
  - Service worker
  - Web app manifest
  - Browser compatibility tests

## [0.2.6] - 2026-02-14

### Added
- **Flutter Mobile App**: Complete mobile foundation
  - FFI bindings for Rust core
  - Home screen UI
  - Identity service
  - Create/Join room screens
  - Audio integration with WebRTC
  - iOS background audio
  - Android foreground service

## [0.2.5] - 2026-02-14

### Added
- **Settings Panel**: Desktop UI settings
  - Audio, Network, Identity, About tabs
- **CI/CD Pipeline**: GitHub Actions
  - Multi-platform builds
  - Clippy, fmt, test automation

## [0.2.4] - 2026-02-14

### Added
- **Reputation System**: Decentralized reputation
  - `reputation/score.rs` - Score calculation
  - `reputation/challenge.rs` - Proof-of-Bandwidth
  - `reputation/vouch.rs` - Web-of-Trust vouching

## [0.2.3] - 2026-02-14

### Added
- **Dedicated Node**: Headless server mode
  - TOML configuration
  - Web dashboard
  - Prometheus metrics
  - Docker image
  - Systemd service

## [0.2.2] - 2026-02-14

### Added
- **Opus Codec**: High-quality audio compression
  - Encoder/Decoder with FEC/DTX
  - 16-96 kbps adaptive bitrate
- **RNNoise**: ML-based noise suppression
- **AudioProcessor**: Combined pipeline

## [0.2.1] - 2026-02-14

### Added
- **NAT Traversal**: Complete NAT traversal stack
  - STUN client
  - ICE agent
  - TURN relay
  - UPnP/NAT-PMP
  - TCP hole-punching

- **E2E Encryption**: End-to-end encryption
  - ChaCha20-Poly1305 cipher
  - X25519 key exchange
  - Noise_XX handshake
  - Session key rotation
  - Replay attack protection

## [0.1.0] - Phase 1 Complete

### Added
- P2P networking via libp2p
- Ed25519 identity system
- Kademlia DHT peer discovery
- Room system with shareable links
- Audio pipeline with cpal
- Mixer algorithm (Full-Mesh/SFU)
- Basic CLI tool
- Desktop app scaffold (Tauri)
- Mobile app scaffold (Flutter)

[0.3.0-beta.1]: https://github.com/omen4impact/agora/releases/tag/v0.3.0-beta.1
[0.2.9]: https://github.com/omen4impact/agora/compare/v0.2.8...v0.2.9
[0.2.8]: https://github.com/omen4impact/agora/compare/v0.2.7...v0.2.8
[0.2.7]: https://github.com/omen4impact/agora/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/omen4impact/agora/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/omen4impact/agora/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/omen4impact/agora/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/omen4impact/agora/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/omen4impact/agora/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/omen4impact/agora/compare/v0.1.0...v0.2.1
[0.1.0]: https://github.com/omen4impact/agora/releases/tag/v0.1.0
