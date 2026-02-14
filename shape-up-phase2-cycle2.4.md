# Shape Up - Phase 2, Cycle 2.4: Reputation System

## Übersicht

Cycle 2.4 fokussiert sich auf ein dezentrales Reputation-System für Nodes. Dies verhindert Sybil-Attacken und schafft Anreize für zuverlässige Infrastruktur - ohne Blockchain oder Token.

**Dauer: 6 Wochen**

**Startdatum: 2026-02-14**

---

## Pitch 2.4.1: Core Reputation Logic

### Problem

Ohne Reputation können Angreifer das Netzwerk mit schlechten Nodes überfluten. Nutzer können nicht unterscheiden, welchen Nodes sie vertrauen können.

### Appetite: 4 Wochen

### Solution

```
Woche 1: ReputationScore Struct und Berechnung
Woche 2: Proof-of-Bandwidth Challenges
Woche 3: DHT-basierte Reputation Storage
Woche 4: Integration mit NodeDiscovery
```

### Reputation Score Components

```
┌─────────────────────────────────────────────┐
│         Reputation Score (0.0 - 1.0)        │
├─────────────────────────────────────────────┤
│                                             │
│  Uptime Score (40%)                         │
│  ┌─────────────────────────────────────┐   │
│  │ Quadratic growth with duration      │   │
│  │ Score = min(1.0, (days/30)²)        │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  Performance Score (30%)                    │
│  ┌─────────────────────────────────────┐   │
│  │ Avg Latency: < 50ms = 1.0           │   │
│  │             < 100ms = 0.8           │   │
│  │             < 200ms = 0.5           │   │
│  │             > 200ms = 0.2           │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  Reliability Score (20%)                    │
│  ┌─────────────────────────────────────┐   │
│  │ Based on successful sessions        │   │
│  │ vs. dropped connections             │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  Challenge Score (10%)                      │
│  ┌─────────────────────────────────────┐   │
│  │ Proof-of-Bandwidth challenge pass   │   │
│  │ rate from other nodes               │   │
│  └─────────────────────────────────────┘   │
│                                             │
└─────────────────────────────────────────────┘
```

### ReputationScore Struct

```rust
pub struct ReputationScore {
    pub overall: f32,
    pub uptime_score: f32,
    pub performance_score: f32,
    pub reliability_score: f32,
    pub challenge_score: f32,
    
    // Tracking data
    pub uptime_seconds: u64,
    pub total_sessions: u64,
    pub successful_sessions: u64,
    pub avg_latency_ms: u32,
    pub challenges_passed: u64,
    pub challenges_total: u64,
    
    // Timestamps
    pub first_seen: u64,
    pub last_updated: u64,
}
```

### DHT Storage Key

```
Key: /agora/reputation/<peer_id>
Value: ReputationScore (serialized)
```

### Tasks

- [x] `ReputationScore` Struct definieren
- [x] Score-Berechnung implementieren
- [x] Performance-Tracking (Latency)
- [x] Session-Tracking (Success Rate)
- [x] DHT-Storage für Reputation
- [x] Integration mit NodeAdvertisement

### Rabbit Holes

- Score-Manipulation durch Self-Reporting → Verify durch andere Nodes
- DHT-Propagation langsam → Redundant publish
- Score-Berechnung zu komplex → Einfache Formeln bevorzugen

### No-Gos

- Keine Blockchain
- Keine Token/Coins
- Kein zentraler Server

### Erfolgskriterien

- [x] Reputation wird korrekt berechnet
- [x] DHT speichert Reputation
- [x] Score reagiert auf Node-Verhalten
- [x] Clients können Reputation abfragen

---

## Pitch 2.4.2: Proof-of-Bandwidth Challenges

### Problem

Nodes können ihre Bandbreite und Leistung falsch angeben. Wir brauchen einen Weg, diese Behauptungen zu verifizieren.

### Appetite: 3 Wochen

### Solution

```
Woche 1: Challenge Protocol Design
Woche 2: Challenge Implementation
Woche 3: Challenge Integration
```

### Challenge Protocol

```
┌─────────────────────────────────────────────┐
│         Proof-of-Bandwidth Challenge        │
├─────────────────────────────────────────────┤
│                                             │
│  1. Challenger wählt Target Node            │
│  2. Challenger sendet Challenge Request     │
│     - Random 1MB data hash                  │
│     - Timeout: 5 seconds                    │
│                                             │
│  3. Target Node empfängt Challenge          │
│  4. Target Node lädt Daten herunter        │
│  5. Target Node sendet Proof zurück        │
│     - HMAC mit Node Identity                │
│                                             │
│  6. Challenger verifiziert Proof            │
│  7. Challenger published Result to DHT      │
│                                             │
└─────────────────────────────────────────────┘
```

### Challenge Types

```rust
pub enum ChallengeType {
    Bandwidth { size_bytes: u64, max_time_ms: u32 },
    Latency { target_ms: u32 },
    Uptime { check_interval: Duration },
}
```

### Tasks

- [x] `ChallengeRequest` Struct
- [x] `ChallengeResponse` Struct
- [x] Challenge Handler im Node
- [x] Challenge Verifier
- [x] DHT Result Publishing

### Erfolgskriterien

- [x] Challenges können gesendet werden
- [x] Responses werden verifiziert
- [x] Ergebnisse beeinflussen Reputation

---

## Pitch 2.4.3: Web-of-Trust Vouching

### Problem

Neue Nodes haben eine schlechte Reputation und werden nicht ausgewählt. Ein Vouching-System erlaubt etablierten Nodes, neue Nodes zu vouchen.

### Appetite: 2 Wochen

### Solution

```
Vouching Mechanics:
- Voucher muss Reputation >= 0.7 haben
- Vouch gibt +0.1 Reputation zum Start
- Bei Fehlverhalten verliert Voucher auch Reputation
- Max 3 vouches pro Node
- Cooldown: 1 Vouch pro Woche
```

### Vouch Struct

```rust
pub struct Vouch {
    pub voucher_peer_id: String,
    pub vouchee_peer_id: String,
    pub timestamp: u64,
    pub stake: f32,  // Amount of reputation staked
    pub signature: Vec<u8>,
}
```

### Tasks

- [x] `Vouch` Struct definieren
- [x] Vouch Creation und Signing
- [x] Vouch Verification
- [x] Stake Tracking
- [x] Penalty bei Fehlverhalten

### Erfolgskriterien

- [x] Vouching funktioniert
- [x] Vouches sind kryptografisch signiert
- [x] Strafen werden angewendet

---

## Pitch 2.4.4: Sybil Resistance

### Problem

Angreifer können viele gefälschte Nodes erstellen um Reputation zu manipulieren.

### Appetite: 2 Wochen

### Solution

```
Layer 1: Proof-of-Bandwidth (bereits implementiert)
Layer 2: Proof-of-Uptime (quadratische Reputation)
Layer 3: Web-of-Trust Vouching
Layer 4: Rate-Limiting für neue Nodes
```

### Rate Limiting

```rust
pub struct NodeLimits {
    pub max_new_nodes_per_day: u32,      // 5
    pub min_uptime_for_vouching: u64,    // 7 days in seconds
    pub min_reputation_for_vouching: f32, // 0.7
    pub max_vouches_per_node: u32,       // 3
    pub vouch_cooldown_days: u32,        // 7
}
```

### Tasks

- [x] Rate Limiting implementieren
- [x] Quadratische Uptime-Bewertung
- [x] Vouching Limits
- [x] Sybil-Erkennung

### Erfolgskriterien

- [x] Neue Nodes sind rate-limited
- [x] Sybil-Attacke ist wirtschaftlich unattraktiv
- [x] Etablierte Nodes haben Vorteil

---

## Dependencies

```toml
# Core dependencies for reputation
sha2 = "0.10"           # Challenge hashing
hmac = "0.12"           # Challenge signing
```

---

## Testing Strategy

### Unit Tests
- Score-Berechnung
- Challenge Verifikation
- Vouch Signing/Verification
- Rate Limiting

### Integration Tests
- Full Reputation Lifecycle
- Challenge Exchange
- Vouching Flow

---

## Exit Criteria

- [x] Reputation wird korrekt berechnet
- [x] Proof-of-Bandwidth funktioniert
- [x] DHT speichert Reputation
- [x] Clients bevorzugen hoch-reputierte Nodes
- [x] Vouching ist implementiert
- [x] Alle Tests bestehen

---

## Implementierungsstatus

Folgende Dateien wurden erstellt:

| Datei | Beschreibung | Zeilen |
|-------|--------------|--------|
| `core/src/reputation/mod.rs` | Modul-Exports | ~20 |
| `core/src/reputation/score.rs` | ReputationScore Berechnung | ~230 |
| `core/src/reputation/challenge.rs` | Proof-of-Bandwidth Challenges | ~230 |
| `core/src/reputation/vouch.rs` | Web-of-Trust Vouching | ~300 |

---

*Dokument erstellt: 2026-02-14*
*Letztes Update: 2026-02-14*
*Cycle 2.4 Status: 100% COMPLETE ✅*
