# Deployment Guide

## Overview

Agora nodes can be deployed in several ways:
- **Docker** - Recommended for most deployments
- **Systemd** - For Linux servers with direct installation
- **Manual** - For custom setups

## Docker Deployment

### Quick Start

```bash
# Build image
docker build -t agora-node .

# Run node
docker run -d \
  --name agora-node \
  -p 7001:7001 \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config:/etc/agora \
  agora-node
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  agora-node:
    build: .
    ports:
      - "7001:7001"   # P2P networking
      - "8080:8080"   # Dashboard
      - "9090:9090"   # Metrics
    volumes:
      - ./config:/etc/agora
      - agora-data:/var/lib/agora
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  agora-data:
```

```bash
# Start
docker-compose up -d

# View logs
docker-compose logs -f

# Stop
docker-compose down
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AGORA_CONFIG` | `/etc/agora/node.toml` | Config file path |
| `RUST_LOG` | `info` | Log level |
| `AGORA_LISTEN_ADDR` | `0.0.0.0:7001` | P2P listen address |
| `AGORA_DASHBOARD_PORT` | `8080` | Dashboard port |

## Systemd Deployment

### Installation

```bash
# Build release binary
cargo build --release -p agora-node

# Install binary
sudo cp target/release/agora-node /usr/local/bin/

# Create config directory
sudo mkdir -p /etc/agora
sudo mkdir -p /var/lib/agora

# Create config file
sudo tee /etc/agora/node.toml > /dev/null <<EOF
[node]
mode = "dedicated"
listen_addr = "0.0.0.0:7001"
max_connections = 100
identity_path = "/var/lib/agora/identity.bin"

[dashboard]
enabled = true
bind = "0.0.0.0"
port = 8080

[metrics]
enabled = true
port = 9090

[logging]
level = "info"
file = "/var/log/agora/node.log"
EOF

# Create log directory
sudo mkdir -p /var/log/agora
sudo chown $USER:$USER /var/log/agora
```

### Systemd Service

```bash
# Create service file
sudo tee /etc/systemd/system/agora-node.service > /dev/null <<EOF
[Unit]
Description=Agora P2P Voice Chat Node
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=agora
Group=agora
ExecStart=/usr/local/bin/agora-node start --config /etc/agora/node.toml
Restart=always
RestartSec=10

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadWritePaths=/var/lib/agora /var/log/agora

# Resource limits
LimitNOFILE=65536
MemoryMax=2G

[Install]
WantedBy=multi-user.target
EOF

# Create agora user
sudo useradd -r -s /bin/false agora
sudo chown -R agora:agora /var/lib/agora /var/log/agora

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable agora-node
sudo systemctl start agora-node

# Check status
sudo systemctl status agora-node
```

### Management Commands

```bash
# Start
sudo systemctl start agora-node

# Stop
sudo systemctl stop agora-node

# Restart
sudo systemctl restart agora-node

# View logs
sudo journalctl -u agora-node -f

# Check status
sudo systemctl status agora-node
```

## Configuration

### Full Configuration Reference

```toml
# /etc/agora/node.toml

[node]
# Operating mode: "dedicated", "relay", "bootstrap"
mode = "dedicated"

# P2P listen address
listen_addr = "0.0.0.0:7001"

# Maximum concurrent connections
max_connections = 100

# Maximum rooms this node can host
max_rooms = 50

# Identity file path (auto-generated if not exists)
identity_path = "/var/lib/agora/identity.bin"

# Node name for discovery
name = "my-agora-node"

# Region for discovery
region = "eu-west"

# Capabilities: "mixer", "relay", "bootstrap"
capabilities = ["mixer", "relay"]

[dashboard]
# Enable web dashboard
enabled = true

# Bind address
bind = "0.0.0.0"

# Port
port = 8080

# Enable REST API
api_enabled = true

[metrics]
# Enable Prometheus metrics
enabled = true

# Metrics port
port = 9090

# Metrics endpoint
endpoint = "/metrics"

[signaling]
# Enable WebSocket signaling (for WebRTC clients)
enabled = true

# Signaling port (uses dashboard port if not specified)
port = 8080

[network]
# Bootstrap peers (for joining the network)
bootstrap_peers = [
    # "/ip4/1.2.3.4/tcp/7001/p2p/12D3KooW...",
]

# Enable UPnP auto port forwarding
upnp_enabled = true

# STUN servers for NAT detection
stun_servers = [
    "stun:stun.l.google.com:19302",
    "stun:stun1.l.google.com:19302",
]

# TURN servers (fallback for symmetric NAT)
# [[network.turn_servers]]
# url = "turn:turn.example.com:3478"
# username = "user"
# password = "pass"

[reputation]
# Initial reputation score (0.0 - 1.0)
initial_score = 0.5

# Minimum score to accept connections
min_score = 0.1

# Enable vouching system
vouching_enabled = true

[logging]
# Log level: "trace", "debug", "info", "warn", "error"
level = "info"

# Log file path (optional, also logs to stdout)
file = "/var/log/agora/node.log"

# Log format: "pretty", "json"
format = "pretty"
```

## Monitoring

### Dashboard

Access the web dashboard at `http://your-node:8080/`

Shows:
- Node status and uptime
- Connected peers
- Active rooms
- Network statistics

### Prometheus Metrics

Metrics available at `http://your-node:9090/metrics`

Key metrics:
```
agora_node_uptime_seconds
agora_node_connections_total
agora_node_rooms_active
agora_node_bandwidth_bytes_total
agora_node_audio_latency_seconds
```

### Grafana Dashboard

Example Grafana dashboard configuration:

```json
{
  "dashboard": {
    "title": "Agora Node",
    "panels": [
      {
        "title": "Uptime",
        "targets": [{"expr": "agora_node_uptime_seconds"}]
      },
      {
        "title": "Connections",
        "targets": [{"expr": "agora_node_connections_total"}]
      },
      {
        "title": "Active Rooms",
        "targets": [{"expr": "agora_node_rooms_active"}]
      },
      {
        "title": "Bandwidth",
        "targets": [{"expr": "rate(agora_node_bandwidth_bytes_total[5m])"}]
      }
    ]
  }
}
```

## High Availability

### Multiple Nodes

For redundancy, run multiple nodes in different regions:

```
┌─────────────────────────────────────────────────────────────┐
│                      Load Balancer                           │
│                    (optional, for signaling)                │
└─────────────────────────┬───────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
   ┌─────────┐       ┌─────────┐       ┌─────────┐
   │ Node 1  │       │ Node 2  │       │ Node 3  │
   │ EU-West │       │ US-East │       │ Asia    │
   └─────────┘       └─────────┘       └─────────┘
```

### Bootstrap Nodes

Run dedicated bootstrap nodes for network stability:

```toml
# bootstrap-node.toml
[node]
mode = "bootstrap"
capabilities = ["bootstrap"]

# No audio processing needed
max_rooms = 0
```

## Troubleshooting

### Common Issues

**Port already in use:**
```bash
# Check what's using the port
sudo lsof -i :7001

# Kill the process or change port in config
```

**NAT traversal not working:**
```bash
# Check NAT type
cargo run -p agora-cli -- detect-nat

# Enable UPnP on router
# Or configure TURN server
```

**High CPU usage:**
```bash
# Reduce max_connections
# Disable audio processing on relay nodes
# Check for runaway processes
```

### Logs

```bash
# Docker logs
docker logs agora-node -f

# Systemd logs
sudo journalctl -u agora-node -f

# Log file
tail -f /var/log/agora/node.log
```

### Health Check

```bash
# Check node is running
curl http://localhost:8080/health

# Check metrics
curl http://localhost:9090/metrics

# Check status
agora-node status --endpoint http://localhost:8080
```
