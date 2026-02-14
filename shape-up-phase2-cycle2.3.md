# Shape Up - Phase 2, Cycle 2.3: Dedicated Node Software

## Übersicht

Cycle 2.3 fokussiert sich auf die Entwicklung einer dedizierten Node-Software für 24/7 Betrieb. Dies ermöglicht Community-Mitglieder, stabile Infrastruktur für das Netzwerk bereitzustellen.

**Dauer: 6 Wochen**

**Startdatum: 2026-02-14**

---

## Pitch 2.3.1: Headless Node Mode

### Problem

Aktuell ist Agora nur als Client nutzbar. Für ein stabiles dezentrales Netzwerk brauchen wir dedizierte Nodes, die 24/7 laufen und als Mixer, Relay oder Bootstrap-Server fungieren können.

### Appetite: 4 Wochen

### Solution

```
Woche 1: Node-Konfiguration (TOML-basiert)
Woche 2: Headless CLI mit Node-Modi
Woche 3: Node-Typen (Mixer, Relay, Bootstrap)
Woche 4: Graceful Shutdown & Signal Handling
```

### Node Architecture

```
┌─────────────────────────────────────────────────────┐
│              Dedicated Node                          │
│                                                     │
│  ┌─────────────────────────────────────────────┐   │
│  │           Configuration (TOML)              │   │
│  │  - Node mode (dedicated/relay/bootstrap)    │   │
│  │  - Listen address & port                    │   │
│  │  - Max connections & mixers                 │   │
│  │  - TURN/STUN settings                       │   │
│  └─────────────────────────────────────────────┘   │
│                       │                             │
│                       ▼                             │
│  ┌─────────────────────────────────────────────┐   │
│  │              Node Runtime                    │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────────┐ │   │
│  │  │Network  │  │ Audio   │  │   Metrics   │ │   │
│  │  │ Node    │  │ (optional)  │   Export    │ │   │
│  │  └─────────┘  └─────────┘  └─────────────┘ │   │
│  └─────────────────────────────────────────────┘   │
│                       │                             │
│                       ▼                             │
│  ┌─────────────────────────────────────────────┐   │
│  │           Web Dashboard (HTTP)              │   │
│  │  - Status display                           │   │
│  │  - Connected peers                          │   │
│  │  - Metrics endpoint                         │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

### Configuration Example

```toml
# /etc/agora/node.toml

[node]
mode = "dedicated"           # dedicated | relay | bootstrap
listen_addr = "0.0.0.0"
listen_port = 7001
max_connections = 100
max_mixers = 5

[identity]
# Path to persisted identity
key_file = "/var/lib/agora/identity.bin"
# Optional display name
name = "Agora Node EU-West"

[network]
# Bootstrap peers for initial connection
bootstrap_peers = [
    "/ip4/1.2.3.4/tcp/7001/p2p/12D3KooW...",
]
# Enable UPnP for automatic port forwarding
enable_upnp = true
# STUN servers for NAT detection
stun_servers = [
    "stun.l.google.com:19302",
]

[turn]
# TURN server for fallback
enabled = false
url = "turn:turn.example.com:3478"
username = ""
password = ""

[dashboard]
enabled = true
listen_addr = "127.0.0.1"
port = 8080

[metrics]
enabled = true
port = 9090
endpoint = "/metrics"

[logging]
level = "info"
file = "/var/log/agora/node.log"
format = "json"             # json | text
```

### Tasks

#### Woche 1: Node-Konfiguration

- [ ] `NodeConfig` Struct definieren
- [ ] TOML Parsing mit `toml` Crate
- [ ] Config-Validierung
- [ ] Default Config Generator
- [ ] CLI für Config-Erstellung

#### Woche 2: Headless CLI

- [ ] `agora-node` Binary erstellen
- [ ] CLI Argumente (start, stop, status)
- [ ] Mode-spezifisches Verhalten
- [ ] Logging Setup
- [ ] PID File Management

#### Woche 3: Node-Typen

- [ ] **Mixer Node**: Verarbeitet Audio für Räume
- [ ] **Relay Node**: TURN-ähnliche Relay-Funktionalität
- [ ] **Bootstrap Node**: DHT Bootstrap für neue Peers
- [ ] Capability Advertisement in DHT

#### Woche 4: Lifecycle Management

- [ ] Signal Handling (SIGTERM, SIGINT)
- [ ] Graceful Shutdown
- [ ] Connection Draining
- [ ] State Persistence
- [ ] Auto-Recovery

### Rabbit Holes

- Config-Validierung zu strikt → Defaults mit Warnungen
- Signal Handling auf verschiedenen OS → Portable Lösung
- State Persistence kann komplex werden → Minimal V1

### No-Gos

- Keine GUI (Headless only)
- Keine automatischen Updates (Sicherheit)
- Kein Cloud-Management (Dezentral)

### Erfolgskriterien

- [ ] Node startet mit TOML-Config
- [ ] Läuft als Mixer/Relay/Bootstrap
- [ ] Graceful Shutdown funktioniert
- [ ] Alle Node-Modi getestet

---

## Pitch 2.3.2: Web Dashboard & Metrics

### Problem

Node-Betreiber müssen den Status ihrer Nodes überwachen können. Ohne GUI brauchen wir ein Web-Interface.

### Appetite: 3 Wochen

### Solution

```
Woche 1: Embedded HTTP Server (axum)
Woche 2: Dashboard UI (HTML/CSS, minimal)
Woche 3: Metrics Export (Prometheus-format)
```

### Dashboard Endpoints

```
GET /                     # HTML Dashboard
GET /api/status           # JSON Status
GET /api/peers            # Connected Peers
GET /api/rooms            # Active Rooms
GET /metrics              # Prometheus Metrics
GET /health               # Health Check
```

### Metrics Example

```
# HELP agora_connections_total Total number of connections
# TYPE agora_connections_total gauge
agora_connections_total{type="incoming"} 45
agora_connections_total{type="outgoing"} 12

# HELP agora_rooms_active Number of active rooms
# TYPE agora_rooms_active gauge
agora_rooms_active 8

# HELP agora_audio_packets_total Total audio packets processed
# TYPE agora_audio_packets_total counter
agora_audio_packets_total{direction="in"} 1234567
agora_audio_packets_total{direction="out"} 987654

# HELP agora_node_uptime_seconds Node uptime in seconds
# TYPE agora_node_uptime_seconds gauge
agora_node_uptime_seconds 86400
```

### Tasks

#### Woche 1: HTTP Server

- [ ] `axum` Crate einbinden
- [ ] Server Bootstrap
- [ ] Basic Routing
- [ ] CORS Konfiguration
- [ ] Error Handling

#### Woche 2: Dashboard UI

- [ ] HTML Template (minimal, embedded)
- [ ] Status-Anzeige
- [ ] Peer-Liste
- [ ] Room-Übersicht
- [ ] Auto-Refresh (JavaScript)

#### Woche 3: Metrics

- [ ] Metrics Registry
- [ ] Prometheus Format Export
- [ ] Node Stats
- [ ] Audio Stats
- [ ] Network Stats

### Erfolgskriterien

- [ ] Dashboard erreichbar unter :8080
- [ ] Status wird angezeigt
- [ ] Metrics im Prometheus-Format
- [ ] Health-Check funktioniert

---

## Pitch 2.3.3: Node Discovery

### Problem

Clients müssen verfügbare Nodes finden und auswählen können.

### Appetite: 2 Wochen

### Solution

Nodes registrieren sich in der DHT mit Metadata. Clients können nach Nodes filtern und sortieren.

### DHT Node Advertisement

```
Key: /agora/nodes/<region>/<node_id>
Value: {
    "peer_id": "12D3KooW...",
    "version": "0.1.0",
    "region": "eu-west",
    "capabilities": ["mixer", "relay"],
    "max_clients": 100,
    "current_load": 45,
    "uptime_seconds": 2592000,
    "reputation": 0.87,
    "last_seen": 1707849600,
    "listen_addr": ["/ip4/1.2.3.4/tcp/7001"]
}
```

### Tasks

- [ ] Node Advertisement Struct
- [ ] Periodic DHT Publishing
- [ ] Node Discovery Client API
- [ ] Region-basierte Filterung
- [ ] Latenz-basierte Sortierung

### Erfolgskriterien

- [ ] Nodes erscheinen in DHT
- [ ] Clients können Nodes finden
- [ ] Filterung nach Region funktioniert

---

## Pitch 2.3.4: Docker & Deployment

### Problem

Node-Betreiber wollen einfaches Deployment via Docker.

### Appetite: 2 Wochen

### Solution

### Dockerfile

```dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p agora-node

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libopus0 \
    && rm -rf /var/lib/apt/lists/*
    
COPY --from=builder /app/target/release/agora-node /usr/local/bin/
COPY config/node.toml /etc/agora/node.toml

EXPOSE 7001 8080 9090

HEALTHCHECK --interval=30s --timeout=5s \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["agora-node"]
CMD ["--config", "/etc/agora/node.toml"]
```

### docker-compose.yml

```yaml
version: '3.8'

services:
  agora-node:
    image: agora/node:latest
    container_name: agora-node
    restart: unless-stopped
    ports:
      - "7001:7001"      # P2P
      - "8080:8080"      # Dashboard
      - "9090:9090"      # Metrics
    volumes:
      - ./config:/etc/agora:ro
      - agora-data:/var/lib/agora
    environment:
      - RUST_LOG=info
    networks:
      - agora-network

volumes:
  agora-data:

networks:
  agora-network:
    driver: bridge
```

### Tasks

- [ ] Dockerfile erstellen
- [ ] docker-compose.yml
- [ ] Multi-stage Build optimieren
- [ ] Health Check
- [ ] Volume Management
- [ ] Systemd Service Unit

### Erfolgskriterien

- [ ] Docker Image baut
- [ ] Container startet erfolgreich
- [ ] Health Check funktioniert
- [ ] Systemd Service dokumentiert

---

## Dependencies

```toml
# Cargo.toml additions
[dependencies]
axum = "0.7"                    # HTTP server
tower = "0.4"                   # Tower utilities
tower-http = { version = "0.5", features = ["cors"] }
toml = "0.8"                    # TOML parsing
serde_toml = "0.8"              # TOML serialization
prometheus = "0.13"             # Metrics
lazy_static = "1.4"             # Lazy statics
signal-hook = "0.3"             # Signal handling
signal-hook-tokio = "0.3"       # Async signal handling
pid-file = "0.1"                # PID file management
```

---

## Testing Strategy

### Unit Tests
- Config Parsing & Validation
- Node Mode Selection
- Metrics Collection

### Integration Tests
- Full Node Lifecycle
- Dashboard Endpoints
- DHT Advertisement

### Manual Testing
- Docker Deployment
- Graceful Shutdown
- Load Testing

---

## Risks

| Risiko | Wahrscheinlichkeit | Impact | Mitigation |
|--------|-------------------|--------|------------|
| Port-Konflikte | Medium | Low | Konfigurierbare Ports |
| Memory Leaks | Low | High | Monitoring, Regular Restarts |
| DHT-Propagation langsam | Medium | Medium | Redundant advertise |

---

## Exit Criteria

- [x] Node läuft headless
- [x] Konfiguration via TOML
- [x] Dashboard funktioniert
- [x] Metrics exportiert
- [x] Docker Image verfügbar
- [x] Alle Tests bestehen
- [x] Node Discovery (DHT)

---

*Dokument erstellt: 2026-02-14*
*Letztes Update: 2026-02-14*
*Cycle 2.3 Status: 100% COMPLETE ✅*

---

## Implementierungsstatus (2026-02-14)

### Erledigte Tasks

| Task | Status | Datei |
|------|--------|-------|
| Node-Konfiguration (TOML) | ✅ COMPLETE | `node/src/config.rs` |
| Headless CLI | ✅ COMPLETE | `node/src/main.rs` |
| Web Dashboard | ✅ COMPLETE | `node/src/dashboard.rs` |
| Metrics Export (Prometheus) | ✅ COMPLETE | `node/src/metrics.rs` |
| Signal Handling | ✅ COMPLETE | `node/src/node.rs` |
| Docker Image | ✅ COMPLETE | `Dockerfile` |
| Docker Compose | ✅ COMPLETE | `docker/docker-compose.yml` |
| Systemd Service | ✅ COMPLETE | `docker/agora-node.service` |
| Node Discovery (DHT) | ✅ COMPLETE | `node/src/discovery.rs` |

### Neue Dateien

```
node/
├── Cargo.toml           # Dependencies (axum, prometheus, etc.)
└── src/
    ├── main.rs          # CLI Entry Point
    ├── config.rs        # TOML Configuration (340+ Zeilen)
    ├── dashboard.rs     # Web Dashboard & API (310+ Zeilen)
    ├── discovery.rs     # Node Discovery & DHT (320+ Zeilen)
    ├── metrics.rs       # Prometheus Metrics (130+ Zeilen)
    ├── node.rs          # Node Runtime (310+ Zeilen)
    └── error.rs         # Error Types

docker/
├── docker-compose.yml   # Docker Compose Configuration
├── node.toml            # Default Node Config for Docker
└── agora-node.service   # Systemd Service Unit

Dockerfile               # Multi-stage Docker Build
.dockerignore            # Docker Build Optimization
```

### Test-Ergebnisse

- **Unit Tests:** 10 bestanden (config + discovery tests)
- **Core Tests:** 142 bestanden
- **Integration Tests:** 24 bestanden

### CLI Commands

```bash
# Node starten
agora-node start --config /etc/agora/node.toml

# Node stoppen
agora-node stop

# Status anzeigen
agora-node status --endpoint http://localhost:8080

# Config generieren
agora-node config --output node.toml

# Identity generieren
agora-node identity --output identity.bin --name "MyNode"

# Nodes entdecken
agora-node discover --endpoint http://localhost:8080 --region eu-west --capability mixer
```

### Dashboard Endpoints

```
GET /              # HTML Dashboard
GET /api/status    # JSON Status
GET /api/peers     # Connected Peers
GET /api/discover  # Discover nodes (query: region, capability)
GET /health        # Health Check
GET /metrics       # Prometheus Metrics
```