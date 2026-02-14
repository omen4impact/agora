use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DHT_KEY_PREFIX: &str = "/agora/nodes";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAdvertisement {
    pub peer_id: String,
    pub version: String,
    pub region: String,
    pub capabilities: Vec<NodeCapability>,
    pub max_clients: u32,
    pub current_load: u32,
    pub uptime_seconds: u64,
    pub reputation: f32,
    pub last_seen: u64,
    pub listen_addrs: Vec<String>,
    pub node_mode: NodeMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeCapability {
    Mixer,
    Relay,
    Bootstrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeMode {
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

impl NodeAdvertisement {
    pub fn new(peer_id: PeerId, mode: NodeMode) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            region: detect_region(),
            capabilities: capabilities_for_mode(mode),
            max_clients: 100,
            current_load: 0,
            uptime_seconds: 0,
            reputation: 1.0,
            last_seen: current_timestamp(),
            listen_addrs: Vec::new(),
            node_mode: mode,
        }
    }

    pub fn dht_key(&self) -> String {
        format!("{}/{}/{}", DHT_KEY_PREFIX, self.region, self.peer_id)
    }

    #[allow(dead_code)]
    pub fn dht_key_for_region(region: &str) -> String {
        format!("{}/{}", DHT_KEY_PREFIX, region)
    }

    #[allow(dead_code)]
    pub fn global_dht_key() -> String {
        DHT_KEY_PREFIX.to_string()
    }

    pub fn update_uptime(&mut self, seconds: u64) {
        self.uptime_seconds = seconds;
        self.last_seen = current_timestamp();
    }

    #[allow(dead_code)]
    pub fn update_load(&mut self, current: u32) {
        self.current_load = current;
    }

    #[allow(dead_code)]
    pub fn add_listen_addr(&mut self, addr: String) {
        if !self.listen_addrs.contains(&addr) {
            self.listen_addrs.push(addr);
        }
    }

    pub fn load_percentage(&self) -> f32 {
        if self.max_clients == 0 {
            return 0.0;
        }
        (self.current_load as f32 / self.max_clients as f32) * 100.0
    }

    pub fn score(&self) -> f32 {
        let load_score = 1.0 - (self.load_percentage() / 100.0).min(1.0);
        let uptime_score = (self.uptime_seconds as f32 / 86400.0).min(1.0);
        let reputation_score = self.reputation;

        (load_score * 0.4) + (uptime_score * 0.3) + (reputation_score * 0.3)
    }

    pub fn serialize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    #[allow(dead_code)]
    pub fn deserialize(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

fn capabilities_for_mode(mode: NodeMode) -> Vec<NodeCapability> {
    match mode {
        NodeMode::Dedicated => vec![NodeCapability::Mixer],
        NodeMode::Relay => vec![NodeCapability::Relay],
        NodeMode::Bootstrap => vec![NodeCapability::Bootstrap, NodeCapability::Relay],
    }
}

fn detect_region() -> String {
    std::env::var("AGORA_REGION").unwrap_or_else(|_| "unknown".to_string())
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone)]
pub struct DiscoveredNode {
    pub advertisement: NodeAdvertisement,
    pub discovered_at: Instant,
    pub latency_ms: Option<u64>,
}

impl DiscoveredNode {
    pub fn new(advertisement: NodeAdvertisement) -> Self {
        Self {
            advertisement,
            discovered_at: Instant::now(),
            latency_ms: None,
        }
    }

    pub fn age(&self) -> Duration {
        self.discovered_at.elapsed()
    }

    pub fn is_fresh(&self) -> bool {
        self.age() < Duration::from_secs(300)
    }

    pub fn final_score(&self) -> f32 {
        let base_score = self.advertisement.score();

        let latency_penalty = match self.latency_ms {
            Some(ms) if ms > 200 => 0.3,
            Some(ms) if ms > 100 => 0.1,
            _ => 0.0,
        };

        (base_score - latency_penalty).max(0.0)
    }
}

pub struct NodeDiscovery {
    nodes: HashMap<PeerId, DiscoveredNode>,
    max_age: Duration,
}

impl NodeDiscovery {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            max_age: Duration::from_secs(300),
        }
    }

    #[allow(dead_code)]
    pub fn add_node(&mut self, advertisement: NodeAdvertisement) {
        if let Ok(peer_id) = advertisement.peer_id.parse::<PeerId>() {
            let discovered = DiscoveredNode::new(advertisement);
            self.nodes.insert(peer_id, discovered);
        }
    }

    #[allow(dead_code)]
    pub fn remove_node(&mut self, peer_id: &PeerId) {
        self.nodes.remove(peer_id);
    }

    #[allow(dead_code)]
    pub fn get_node(&self, peer_id: &PeerId) -> Option<&DiscoveredNode> {
        self.nodes.get(peer_id)
    }

    #[allow(dead_code)]
    pub fn update_latency(&mut self, peer_id: &PeerId, latency_ms: u64) {
        if let Some(node) = self.nodes.get_mut(peer_id) {
            node.latency_ms = Some(latency_ms);
        }
    }

    #[allow(dead_code)]
    pub fn prune_stale(&mut self) {
        self.nodes.retain(|_, node| node.age() < self.max_age);
    }

    #[allow(dead_code)]
    pub fn get_all(&self) -> Vec<&DiscoveredNode> {
        self.nodes.values().collect()
    }

    #[allow(dead_code)]
    pub fn get_by_region(&self, region: &str) -> Vec<&DiscoveredNode> {
        self.nodes
            .values()
            .filter(|n| n.advertisement.region == region)
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_by_capability(&self, capability: NodeCapability) -> Vec<&DiscoveredNode> {
        self.nodes
            .values()
            .filter(|n| n.advertisement.capabilities.contains(&capability))
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_best_nodes(&self, limit: usize) -> Vec<&DiscoveredNode> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by(|a, b| {
            b.final_score()
                .partial_cmp(&a.final_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        nodes.into_iter().take(limit).collect()
    }

    #[allow(dead_code)]
    pub fn get_best_for_region(&self, region: &str, limit: usize) -> Vec<&DiscoveredNode> {
        let mut nodes: Vec<_> = self.get_by_region(region);
        nodes.sort_by(|a, b| {
            b.final_score()
                .partial_cmp(&a.final_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        nodes.into_iter().take(limit).collect()
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for NodeDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub advertise_interval: Duration,
    pub refresh_interval: Duration,
    pub max_results: usize,
    pub region: Option<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            advertise_interval: Duration::from_secs(300),
            refresh_interval: Duration::from_secs(60),
            max_results: 10,
            region: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;

    fn create_test_peer_id() -> PeerId {
        Keypair::generate_ed25519().public().to_peer_id()
    }

    #[test]
    fn test_node_advertisement_creation() {
        let peer_id = create_test_peer_id();
        let ad = NodeAdvertisement::new(peer_id, NodeMode::Dedicated);

        assert_eq!(ad.node_mode, NodeMode::Dedicated);
        assert!(!ad.peer_id.is_empty());
        assert!(ad.capabilities.contains(&NodeCapability::Mixer));
        assert!(ad.score() > 0.0);
    }

    #[test]
    fn test_dht_key_generation() {
        let peer_id = create_test_peer_id();
        let ad = NodeAdvertisement::new(peer_id, NodeMode::Dedicated);

        let key = ad.dht_key();
        assert!(key.starts_with("/agora/nodes/"));
        assert!(key.contains(&ad.peer_id));
    }

    #[test]
    fn test_load_percentage() {
        let peer_id = create_test_peer_id();
        let mut ad = NodeAdvertisement::new(peer_id, NodeMode::Dedicated);

        ad.max_clients = 100;
        ad.current_load = 25;
        assert!((ad.load_percentage() - 25.0).abs() < 0.01);

        ad.current_load = 0;
        assert!((ad.load_percentage() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_node_discovery() {
        let mut discovery = NodeDiscovery::new();

        let peer_id1 = create_test_peer_id();
        let peer_id2 = create_test_peer_id();

        let mut ad1 = NodeAdvertisement::new(peer_id1, NodeMode::Dedicated);
        ad1.region = "eu-west".to_string();

        let mut ad2 = NodeAdvertisement::new(peer_id2, NodeMode::Relay);
        ad2.region = "us-east".to_string();

        discovery.add_node(ad1);
        discovery.add_node(ad2);

        assert_eq!(discovery.count(), 2);

        let eu_nodes = discovery.get_by_region("eu-west");
        assert_eq!(eu_nodes.len(), 1);

        let mixer_nodes = discovery.get_by_capability(NodeCapability::Mixer);
        assert_eq!(mixer_nodes.len(), 1);
    }

    #[test]
    fn test_serialization() {
        let peer_id = create_test_peer_id();
        let ad = NodeAdvertisement::new(peer_id, NodeMode::Dedicated);

        let serialized = ad.serialize().unwrap();
        let deserialized: NodeAdvertisement = NodeAdvertisement::deserialize(&serialized).unwrap();

        assert_eq!(ad.peer_id, deserialized.peer_id);
        assert_eq!(ad.region, deserialized.region);
        assert_eq!(ad.node_mode, deserialized.node_mode);
    }

    #[test]
    fn test_score_calculation() {
        let peer_id = create_test_peer_id();
        let mut ad = NodeAdvertisement::new(peer_id, NodeMode::Dedicated);

        ad.max_clients = 100;
        ad.current_load = 0;
        ad.uptime_seconds = 86400;
        ad.reputation = 1.0;

        let score = ad.score();
        assert!(score > 0.9);

        ad.current_load = 100;
        let loaded_score = ad.score();
        assert!(loaded_score < score);
    }
}
