use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

use crate::config::NodeConfig;
use crate::discovery::{NodeAdvertisement, NodeCapability};
use crate::error::NodeError;
use crate::metrics::NodeMetrics;

pub type DashboardState = Arc<RwLock<DashboardData>>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DashboardData {
    pub status: NodeStatus,
    pub peer_id: String,
    pub version: String,
    pub mode: String,
    pub listen_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeStatus {
    pub uptime_seconds: u64,
    pub connections: ConnectionStatus,
    pub rooms: RoomStatus,
    pub audio: AudioStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionStatus {
    pub total: i64,
    pub incoming: i64,
    pub outgoing: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoomStatus {
    pub active: i64,
    pub participants: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioStatus {
    pub packets_in: u64,
    pub packets_out: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

pub struct Dashboard {
    state: DashboardState,
    addr: SocketAddr,
}

impl Dashboard {
    pub fn new(config: &NodeConfig) -> Self {
        let state = Arc::new(RwLock::new(DashboardData {
            version: env!("CARGO_PKG_VERSION").to_string(),
            mode: config.node.mode.to_string(),
            listen_addr: config.listen_socket().to_string(),
            ..Default::default()
        }));

        Self {
            state,
            addr: config.dashboard_socket(),
        }
    }

    pub fn state(&self) -> DashboardState {
        self.state.clone()
    }

    pub async fn start(self) -> Result<(), NodeError> {
        let app = Router::new()
            .route("/", get(index))
            .route("/api/status", get(api_status))
            .route("/api/peers", get(api_peers))
            .route("/api/discover", get(api_discover))
            .route("/health", get(health))
            .route("/metrics", get(metrics))
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any))
            .with_state(self.state);

        tracing::info!("Dashboard listening on {}", self.addr);

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| NodeError::Dashboard(format!("Failed to bind: {}", e)))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| NodeError::Dashboard(format!("Server error: {}", e)))?;

        Ok(())
    }
}

async fn index(State(state): State<DashboardState>) -> Html<String> {
    let data = state.read().await;
    Html(render_html(&data))
}

async fn api_status(State(state): State<DashboardState>) -> Json<DashboardData> {
    let data = state.read().await.clone();
    Json(data)
}

async fn api_peers(State(_state): State<DashboardState>) -> Json<Vec<String>> {
    Json(vec![])
}

#[derive(Debug, Deserialize)]
struct DiscoverQuery {
    region: Option<String>,
    capability: Option<String>,
}

async fn api_discover(
    Query(query): Query<DiscoverQuery>,
) -> Json<Vec<NodeAdvertisement>> {
    let _capability = query.capability.as_ref()
        .map(|c| match c.to_lowercase().as_str() {
            "mixer" => NodeCapability::Mixer,
            "relay" => NodeCapability::Relay,
            "bootstrap" => NodeCapability::Bootstrap,
            _ => NodeCapability::Mixer,
        });
    
    Json(vec![])
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn metrics() -> impl IntoResponse {
    let metrics = NodeMetrics::gather();
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], metrics)
}

fn render_html(data: &DashboardData) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Agora Node Dashboard</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: #0f172a;
            color: #e2e8f0;
            min-height: 100vh;
            padding: 2rem;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        h1 {{ 
            font-size: 2rem;
            margin-bottom: 0.5rem;
            background: linear-gradient(135deg, #6366f1, #8b5cf6);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        .subtitle {{ color: #64748b; margin-bottom: 2rem; }}
        .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; }}
        .card {{ 
            background: #1e293b;
            border-radius: 12px;
            padding: 1.5rem;
            border: 1px solid #334155;
        }}
        .card h2 {{ 
            font-size: 0.875rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: #94a3b8;
            margin-bottom: 1rem;
        }}
        .stat {{ display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0; border-bottom: 1px solid #334155; }}
        .stat:last-child {{ border-bottom: none; }}
        .stat-label {{ color: #94a3b8; }}
        .stat-value {{ font-weight: 600; font-size: 1.25rem; }}
        .status-good {{ color: #22c55e; }}
        .status-warning {{ color: #eab308; }}
        .footer {{ margin-top: 2rem; text-align: center; color: #64748b; font-size: 0.875rem; }}
        .peer-id {{ font-family: monospace; font-size: 0.75rem; background: #334155; padding: 0.5rem; border-radius: 6px; word-break: break-all; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Agora Node</h1>
        <p class="subtitle">{mode} mode • v{version}</p>
        
        <div class="grid">
            <div class="card">
                <h2>Status</h2>
                <div class="stat">
                    <span class="stat-label">Uptime</span>
                    <span class="stat-value status-good">{uptime}</span>
                </div>
                <div class="stat">
                    <span class="stat-label">Listening</span>
                    <span class="stat-value">{listen_addr}</span>
                </div>
                <div class="stat">
                    <span class="stat-label">Peer ID</span>
                </div>
                <div class="peer-id">{peer_id}</div>
            </div>
            
            <div class="card">
                <h2>Connections</h2>
                <div class="stat">
                    <span class="stat-label">Total</span>
                    <span class="stat-value">{connections_total}</span>
                </div>
                <div class="stat">
                    <span class="stat-label">Incoming</span>
                    <span class="stat-value">{connections_in}</span>
                </div>
                <div class="stat">
                    <span class="stat-label">Outgoing</span>
                    <span class="stat-value">{connections_out}</span>
                </div>
            </div>
            
            <div class="card">
                <h2>Rooms</h2>
                <div class="stat">
                    <span class="stat-label">Active Rooms</span>
                    <span class="stat-value">{rooms_active}</span>
                </div>
                <div class="stat">
                    <span class="stat-label">Participants</span>
                    <span class="stat-value">{participants}</span>
                </div>
            </div>
            
            <div class="card">
                <h2>Audio</h2>
                <div class="stat">
                    <span class="stat-label">Packets In</span>
                    <span class="stat-value">{packets_in}</span>
                </div>
                <div class="stat">
                    <span class="stat-label">Packets Out</span>
                    <span class="stat-value">{packets_out}</span>
                </div>
                <div class="stat">
                    <span class="stat-label">Bytes Processed</span>
                    <span class="stat-value">{bytes_total}</span>
                </div>
            </div>
        </div>
        
        <div class="footer">
            <p>Endpoints: <a href="/api/status" style="color: #6366f1">/api/status</a> • <a href="/metrics" style="color: #6366f1">/metrics</a> • <a href="/health" style="color: #6366f1">/health</a></p>
            <p style="margin-top: 0.5rem">Auto-refresh: 5s</p>
        </div>
    </div>
    
    <script>
        setTimeout(() => location.reload(), 5000);
    </script>
</body>
</html>"#,
        mode = data.mode,
        version = data.version,
        uptime = format_uptime(data.status.uptime_seconds),
        listen_addr = data.listen_addr,
        peer_id = if data.peer_id.is_empty() { "Not connected".to_string() } else { data.peer_id.clone() },
        connections_total = data.status.connections.total,
        connections_in = data.status.connections.incoming,
        connections_out = data.status.connections.outgoing,
        rooms_active = data.status.rooms.active,
        participants = data.status.rooms.participants,
        packets_in = format_number(data.status.audio.packets_in),
        packets_out = format_number(data.status.audio.packets_out),
        bytes_total = format_bytes(data.status.audio.bytes_in + data.status.audio.bytes_out),
    )
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    
    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
