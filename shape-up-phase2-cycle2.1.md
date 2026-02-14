# Shape Up - Phase 2, Cycle 2.1: NAT-Traversal & E2E-Verschlüsselung

## Übersicht

Cycle 2.1 ist der erste Cycle von Phase 2 und fokussiert sich auf zwei kritische Pitches:
1. **STUN/ICE Integration** - Echte NAT-Traversal-Implementierung
2. **E2E-Verschlüsselung** - ChaCha20-Poly1305 mit Noise Protocol

**Dauer: 6 Wochen**

**Startdatum: 2026-02-14**

**Status: GRÖSSENTEILS ABGESCHLOSSEN (Audit: 2026-02-14)**

---

## Audit-Ergebnisse

### Implementierungsstatus nach Code-Review

| Komponente | Geplant | Implementiert | Tests | Status |
|------------|---------|---------------|-------|--------|
| STUN Client | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| ICE Agent | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| TURN Client | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| UPnP | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| NAT-PMP | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| TCP Hole-Punching | ✅ | ❌ | - | NICHT IMPLEMENTIERT |
| Echte Netzwerk-Tests | ✅ | ❌ | - | NICHT DURCHGEFÜHRT |
| ChaCha20-Poly1305 | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| X25519 Key Exchange | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| Noise Protocol | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| Session Key Rotation | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| SecureAudioChannel | ✅ | ✅ | ✅ | VOLLSTÄNDIG |
| Performance Benchmarks | ✅ | ❌ | - | NICHT DURCHGEFÜHRT |
| Security Review | ✅ | ❌ | - | NICHT DURCHGEFÜHRT |

**Gesamt-Teststatus: 133 Tests bestanden (109 Unit + 24 Integration)**

---

## Pitch 2.1.1: STUN/ICE Integration

### Problem

Die aktuelle NAT-Traversal-Implementierung in `core/src/nat.rs` ist ein Stub. Für >85% direkte P2P-Verbindungen benötigen wir:
- Echte STUN-Client-Implementierung
- ICE-Framework für Connection Candidates
- TCP/UDP Hole-Punching

### Appetite: 6 Wochen

### Solution

```
Woche 1-2: STUN-Client implementieren ✅
- STUN Binding Request/Response ✅
- Public IP/Port Detection ✅
- NAT-Typ-Erkennung verbessern ✅

Woche 3-4: ICE-Framework ✅
- ICE Agent Implementation ✅
- Candidate Gathering (Host, Server Reflexive, Relayed) ✅
- Connectivity Checks ✅
- Candidate Selection ✅

Woche 5-6: Hole-Punching & Integration ⚠️ TEILWEISE
- TCP Hole-Punching ❌ NICHT IMPLEMENTIERT
- UDP Hole-Punching ✅ (via ICE connectivity checks)
- UPnP Port Mapping ✅
- NAT-PMP Port Mapping ✅
- Integration mit NetworkNode ✅
- Fallback zu TURN bei Symmetric NAT ✅
- End-to-End Tests (real network) ❌
```

### Architecture

```
┌─────────────────────────────────────────────────────┐
│                    ICE Agent                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │ Host     │  │ SRFLX    │  │ RELAY    │          │
│  │ Candidate│  │ Candidate│  │ Candidate│          │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘          │
│       │             │             │                 │
│       └─────────────┼─────────────┘                 │
│                     │                               │
│              ┌──────┴──────┐                        │
│              │ Connectivity│                        │
│              │   Checks    │                        │
│              └──────┬──────┘                        │
│                     │                               │
│              ┌──────┴──────┐                        │
│              │   STUN      │                        │
│              │   Client    │                        │
│              └─────────────┘                        │
└─────────────────────────────────────────────────────┘
```

### Tasks

#### Woche 1-2: STUN-Client ✅ VOLLSTÄNDIG

- [x] STUN Message Types (Binding Request/Response)
- [x] STUN Attribute Parsing (XOR-MAPPED-ADDRESS, etc.)
- [x] STUN Client mit konfigurierbaren Servern
- [x] Public IP/Port Detection
- [x] NAT-Typ Bestimmung (Full Cone, Symmetric, etc.)
- [x] Tests für STUN-Protokoll

**Implementierung:** `core/src/stun.rs` (241 Zeilen)
- Echte STUN-Implementierung mit `stun` crate
- Google STUN Server standardmäßig konfiguriert
- Async-Support via tokio

#### Woche 3-4: ICE-Framework ✅ VOLLSTÄNDIG

- [x] ICE Agent Struct
- [x] Host Candidate Gathering
- [x] Server Reflexive Candidate (via STUN)
- [x] Relayed Candidate (TURN)
- [x] Candidate Pair Formation
- [x] Connectivity Check Implementation
- [x] ICE Checklist State Machine

**Implementierung:** `core/src/ice.rs` (887 Zeilen)
- Vollständige ICE Agent Implementierung
- SDP Candidate Format Support
- Priority Calculation nach RFC 5245

#### Woche 5-6: Hole-Punching & Integration ⚠️ TEILWEISE

- [ ] TCP Hole-Punching Algorithm ❌
- [x] UDP Hole-Punching Algorithm (via ICE connectivity checks)
- [x] UPnP Port Mapping
- [x] NAT-PMP Port Mapping
- [x] Integration mit `NetworkNode`
- [x] Fallback zu TURN bei Symmetric NAT
- [ ] End-to-End Tests (real network) ❌

**Implementierung:** 
- `core/src/upnp.rs` (667 Zeilen) - UPnP & NAT-PMP
- `core/src/turn.rs` (390 Zeilen) - TURN Client

### Rabbit Holes

- STUN-Server können unterschiedliche Formate nutzen → RFC 5389 strikt folgen ✅
- IPv6 vs IPv4 Candidates → Beide unterstützen ✅
- Hole-Punching-Timing ist kritisch → Retry-Logik mit Backoff ✅

### No-Gos

- Kein eigener TURN-Server (nur Client) ✅
- Keine Tor/I2P-Integration (später) ✅
- Keine Mobile-Optimierung (Cycle 2.6) ✅

### Erfolgskriterien

- [ ] Hole-Punching-Erfolgsrate > 80% in Test-Umgebungen **(NICHT GETESTET)**
- [x] Automatischer Fallback zu TURN bei Symmetric NAT
- [ ] Verbindungsaufbau < 5 Sekunden **(NICHT GETESTET)**
- [x] UPnP Auto-Port-Forwarding funktioniert
- [x] Alle STUN/ICE/TURN/UPnP Tests bestehen

---

## Pitch 2.1.2: E2E-Verschlüsselung

### Problem

Die aktuelle Verschlüsselung in `core/src/crypto.rs` verwendet einfaches XOR - nicht sicher!
Für vertrauenswürdigen P2P Voice Chat brauchen wir echte E2E-Verschlüsselung.

### Appetite: 4 Wochen

### Solution

```
Woche 1: ChaCha20-Poly1305 Integration ✅
- AEAD Cipher Implementierung ✅
- Nonce Management ✅
- Key Derivation ✅

Woche 2: Noise Protocol Integration ✅
- Noise_XX Pattern (mutual authentication) ✅
- X25519 Key Exchange ✅
- Handshake State Machine ✅

Woche 3: Session Key Management ✅
- Ephemere Session Keys pro Raum ✅
- Key Rotation (jede Stunde) ✅
- Forward Secrecy ✅

Woche 4: Integration & Testing ⚠️ TEILWEISE
- Integration mit AudioPacket ✅
- Fingerprint Display ✅
- Performance Tests ❌
- Security Review ❌
```

### Architecture

```
┌─────────────────────────────────────────────────────┐
│                Encrypted Channel                     │
│                                                     │
│  Handshake Phase (Noise_XX):                        │
│  ┌─────────┐    e, s     ┌─────────┐               │
│  │ ClientA │ ──────────> │ ClientB │               │
│  │         │    e, ee,es │         │               │
│  │         │ <────────── │         │               │
│  │         │    s, se    │         │               │
│  └─────────┘             └─────────┘               │
│                                                     │
│  Data Phase (ChaCha20-Poly1305):                    │
│  ┌─────────┐    Encrypted    ┌─────────┐           │
│  │ Audio   │ ───────────────>│ Audio   │           │
│  │ Packet  │   [nonce|ciphertext|tag]│           │
│  └─────────┘                 └─────────┘           │
└─────────────────────────────────────────────────────┘
```

### Tasks

#### Woche 1: ChaCha20-Poly1305 ✅ VOLLSTÄNDIG

- [x] `chacha20poly1305` Crate eingebunden
- [x] `Aead` Trait Implementierung
- [x] Nonce Generation (Counter-basiert)
- [x] Key Derivation mit HKDF
- [x] Tests für Encrypt/Decrypt

**Implementierung:** `core/src/crypto.rs` (1000 Zeilen)

#### Woche 2: Noise Protocol ✅ VOLLSTÄNDIG

- [x] `snow` Crate eingebunden (Rust Noise Implementation)
- [x] Noise_XX Pattern Konfiguration
- [x] Handshake Initiator/Responder
- [x] Session State Management
- [x] Tests für Handshake

**Implementierung:** `core/src/handshake.rs` (424 Zeilen)

#### Woche 3: Session Key Management ✅ VOLLSTÄNDIG

- [x] Session Key Struct
- [x] Key Rotation Timer (1 Stunde)
- [x] Re-Keying Implementation
- [x] Forward Secrecy durch Key Deletion
- [x] Tests für Key Rotation

#### Woche 4: Integration ⚠️ TEILWEISE

- [x] `EncryptedChannel` in `crypto.rs` aktualisiert
- [x] Integration mit `AudioPacket` (`EncryptedAudioPacket`, `SecureAudioChannel`)
- [x] Fingerprint Display (SHA-256 Hash)
- [ ] Performance Benchmarks ❌
- [ ] Security Review ❌

### Rabbit Holes

- Key-Exchange bei vielen Teilnehmern → Group Key Agreement vermeiden, stattdessen Pairwise ✅
- Performance bei 100+ Audio-Paketen/Sekunde → ChaCha20 ist sehr schnell ✅ (nicht benchmarked)
- Key Rotation ohne Unterbrechung → Vorab neue Keys aushandeln ✅

### No-Gos

- Keine Post-Quantum-Kryptografie (zu experimentell) ✅
- Keine QR-Code-Verifikation (Cycle 2.5) ✅
- Keine Multi-Device-Support (Phase 2 später) ✅

### Erfolgskriterien

- [x] Alle Audio-Pakete mit ChaCha20-Poly1305 verschlüsselt
- [x] Mixer können Pakete nicht entschlüsseln (End-to-End)
- [ ] Performance-Impact < 5ms Latenz **(NICHT GETESTET)**
- [x] Key Rotation funktioniert transparent
- [ ] Handshake < 500ms **(NICHT GETESTET)**
- [x] Alle Crypto-Tests bestehen

---

## Dependencies (Implementiert)

```toml
# Cargo.toml - tatsächlich verwendet
[dependencies]
# STUN/ICE
stun = "0.5"           # STUN protocol ✅

# Crypto
chacha20poly1305 = "0.10"  # AEAD cipher ✅
hkdf = "0.12"              # Key derivation ✅
sha2 = "0.10"              # Hashing ✅
snow = "0.9"               # Noise protocol framework ✅
x25519-dalek = "2.0"       # X25519 key exchange ✅
base64 = "0.22"            # Encoding ✅
```

---

## Testing Strategy

### Unit Tests ✅
- STUN Message Parsing ✅
- ICE Candidate Formation ✅
- ChaCha20-Poly1305 Encrypt/Decrypt ✅
- Noise Handshake State Machine ✅
- Key Rotation Logic ✅

### Integration Tests ✅
- Full STUN Binding Flow ✅
- ICE Connectivity Checks (mit Mock STUN) ✅
- End-to-End Encrypted Audio Stream ✅
- Key Rotation ohne Unterbrechung ✅

### Performance Tests ❌ NICHT DURCHGEFÜHRT
- Encrypt/Decrypt Throughput
- STUN Round-Trip Time
- ICE Connectivity Check Duration

---

## Risks

| Risiko | Wahrscheinlichkeit | Impact | Mitigation | Status |
|--------|-------------------|--------|------------|--------|
| STUN-Server unzuverlässig | Medium | Medium | Multiple Server konfigurieren | ✅ |
| Symmetric NAT unüberwindbar | High | High | TURN Fallback implementieren | ✅ |
| Crypto-Performance | Low | Medium | ChaCha20 ist sehr schnell | ⚠️ Nicht benchmarked |
| Noise-Komplexität | Medium | Medium | `snow` Crate nutzen | ✅ |

---

## Exit Criteria

- [x] Alle Tests bestehen (Unit + Integration) - **133 Tests bestanden**
- [ ] Hole-Punching Rate > 80% **(NICHT IN ECHTEM NETZWERK GETESTET)**
- [x] E2E-Verschlüsselung aktiviert
- [ ] Performance < 5ms Latenz-Overhead **(NICHT GETESTET)**
- [ ] Code Review durchgeführt **(NICHT DURCHGEFÜHRT)**
- [x] Dokumentation aktualisiert

---

## Offene Aufgaben (Für Cool-down oder nächsten Cycle)

1. **TCP Hole-Punching** - Explizite Implementierung für TCP-basierte Verbindungen
2. **Echte Netzwerk-Tests** - Hole-Punching Rate in realen Szenarien testen
3. **Performance Benchmarks** - Latenz-Overhead der Verschlüsselung messen
4. **Security Review** - Externe Überprüfung der Crypto-Implementierung

---

*Dokument erstellt: 2026-02-14*
*Letztes Update: 2026-02-14 (Audit)*
*Cycle 2.1 Status: GRÖSSENTEILS ABGESCHLOSSEN (85%) ✅*