use crate::error::NodeError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;

pub const DEFAULT_LISTEN_PORT: u16 = 7001;
pub const DEFAULT_DASHBOARD_PORT: u16 = 8080;
pub const DEFAULT_METRICS_PORT: u16 = 9090;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeMode {
    #[default]
    Dedicated,
    Relay,
    Bootstrap,
}

impl std::fmt::Display for NodeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeMode::Dedicated => write!(f, "dedicated"),
            NodeMode::Relay => write!(f, "relay"),
            NodeMode::Bootstrap => write!(f, "bootstrap"),
        }
    }
}

impl std::str::FromStr for NodeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dedicated" => Ok(NodeMode::Dedicated),
            "relay" => Ok(NodeMode::Relay),
            "bootstrap" => Ok(NodeMode::Bootstrap),
            _ => Err(format!("Invalid node mode: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serde(default)]
    pub node: NodeSection,
    #[serde(default)]
    pub identity: IdentitySection,
    #[serde(default)]
    pub network: NetworkSection,
    #[serde(default)]
    pub turn: TurnSection,
    #[serde(default)]
    pub dashboard: DashboardSection,
    #[serde(default)]
    pub metrics: MetricsSection,
    #[serde(default)]
    pub logging: LoggingSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSection {
    #[serde(default = "default_mode")]
    pub mode: NodeMode,
    #[serde(default = "default_listen_addr")]
    pub listen_addr: IpAddr,
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_max_mixers")]
    pub max_mixers: usize,
}

fn default_mode() -> NodeMode {
    NodeMode::Dedicated
}
fn default_listen_addr() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}
fn default_listen_port() -> u16 {
    DEFAULT_LISTEN_PORT
}
fn default_max_connections() -> usize {
    100
}
fn default_max_mixers() -> usize {
    5
}

impl Default for NodeSection {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            listen_addr: default_listen_addr(),
            listen_port: default_listen_port(),
            max_connections: default_max_connections(),
            max_mixers: default_max_mixers(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitySection {
    #[serde(default = "default_key_file")]
    pub key_file: String,
    #[serde(default)]
    pub name: Option<String>,
}

fn default_key_file() -> String {
    "/var/lib/agora/identity.bin".to_string()
}

impl Default for IdentitySection {
    fn default() -> Self {
        Self {
            key_file: default_key_file(),
            name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSection {
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,
    #[serde(default = "default_true")]
    pub enable_upnp: bool,
    #[serde(default = "default_stun_servers")]
    pub stun_servers: Vec<String>,
    #[serde(default)]
    pub region: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_stun_servers() -> Vec<String> {
    vec![
        "stun.l.google.com:19302".to_string(),
        "stun1.l.google.com:19302".to_string(),
    ]
}

impl Default for NetworkSection {
    fn default() -> Self {
        Self {
            bootstrap_peers: Vec::new(),
            enable_upnp: default_true(),
            stun_servers: default_stun_servers(),
            region: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TurnSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_dashboard_addr")]
    pub listen_addr: IpAddr,
    #[serde(default = "default_dashboard_port")]
    pub port: u16,
}

fn default_dashboard_addr() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
}
fn default_dashboard_port() -> u16 {
    DEFAULT_DASHBOARD_PORT
}

impl Default for DashboardSection {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            listen_addr: default_dashboard_addr(),
            port: default_dashboard_port(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_metrics_port")]
    pub port: u16,
    #[serde(default = "default_metrics_endpoint")]
    pub endpoint: String,
}

fn default_metrics_port() -> u16 {
    DEFAULT_METRICS_PORT
}
fn default_metrics_endpoint() -> String {
    "/metrics".to_string()
}

impl Default for MetricsSection {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            port: default_metrics_port(),
            endpoint: default_metrics_endpoint(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSection {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default = "default_log_format")]
    pub format: LogFormat,
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> LogFormat {
    LogFormat::Text
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Text,
    Json,
}

impl Default for LoggingSection {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
            format: default_log_format(),
        }
    }
}

impl NodeConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, NodeError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| NodeError::Config(format!("Failed to read config file: {}", e)))?;

        let config: Self = toml::from_str(&content)
            .map_err(|e| NodeError::Config(format!("Failed to parse config: {}", e)))?;

        config.validate()?;

        Ok(config)
    }

    pub fn validate(&self) -> Result<(), NodeError> {
        if self.node.max_connections == 0 {
            return Err(NodeError::Config("max_connections must be > 0".to_string()));
        }

        if self.node.max_mixers == 0 {
            return Err(NodeError::Config("max_mixers must be > 0".to_string()));
        }

        Ok(())
    }

    pub fn to_toml(&self) -> Result<String, NodeError> {
        toml::to_string_pretty(self)
            .map_err(|e| NodeError::Config(format!("Failed to serialize config: {}", e)))
    }

    pub fn listen_socket(&self) -> SocketAddr {
        SocketAddr::new(self.node.listen_addr, self.node.listen_port)
    }

    pub fn dashboard_socket(&self) -> SocketAddr {
        SocketAddr::new(self.dashboard.listen_addr, self.dashboard.port)
    }

    pub fn metrics_socket(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), self.metrics.port)
    }
}

pub fn generate_default<P: AsRef<Path>>(output: P) -> Result<(), NodeError> {
    let config = NodeConfig::default();
    let toml = config.to_toml()?;

    fs::write(output.as_ref(), toml)
        .map_err(|e| NodeError::Config(format!("Failed to write config: {}", e)))?;

    println!("Generated default config at: {}", output.as_ref().display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NodeConfig::default();
        assert_eq!(config.node.mode, NodeMode::Dedicated);
        assert_eq!(config.node.listen_port, DEFAULT_LISTEN_PORT);
        assert!(config.dashboard.enabled);
    }

    #[test]
    fn test_config_validation() {
        let mut config = NodeConfig::default();
        config.node.max_connections = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = NodeConfig::default();
        let toml = config.to_toml().unwrap();
        assert!(toml.contains("mode = \"dedicated\""));
    }

    #[test]
    fn test_listen_socket() {
        let config = NodeConfig::default();
        let socket = config.listen_socket();
        assert_eq!(socket.port(), DEFAULT_LISTEN_PORT);
    }
}
