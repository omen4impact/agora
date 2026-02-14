use crate::error::{AgoraResult, Error};
use std::net::SocketAddr;
use std::time::Duration;

pub const DEFAULT_TURN_PORT: u16 = 3478;
pub const DEFAULT_TURN_TLS_PORT: u16 = 5349;
pub const TURN_LIFETIME_SECONDS: u32 = 600;
pub const TURN_REFRESH_MARGIN_SECONDS: u32 = 60;

#[derive(Debug, Clone)]
pub struct TurnConfig {
    pub servers: Vec<TurnServer>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub lifetime: Duration,
}

impl Default for TurnConfig {
    fn default() -> Self {
        Self {
            servers: vec![],
            username: None,
            password: None,
            lifetime: Duration::from_secs(TURN_LIFETIME_SECONDS as u64),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnServer {
    pub address: SocketAddr,
    pub use_tls: bool,
    pub realm: Option<String>,
}

impl TurnServer {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            use_tls: false,
            realm: None,
        }
    }

    pub fn with_tls(mut self) -> Self {
        self.use_tls = true;
        self
    }

    pub fn with_realm(mut self, realm: String) -> Self {
        self.realm = Some(realm);
        self
    }

    pub fn from_host(host: &str, port: Option<u16>) -> AgoraResult<Self> {
        let port = port.unwrap_or(DEFAULT_TURN_PORT);
        let addr: SocketAddr = if host.parse::<SocketAddr>().is_ok() {
            host.parse().unwrap()
        } else {
            let resolved = format!("{}:{}", host, port);
            resolved
                .parse()
                .map_err(|e| Error::Network(format!("Invalid TURN server address '{}': {}", host, e)))?
        };
        Ok(Self::new(addr))
    }
    
    pub fn from_ip(ip: std::net::IpAddr, port: Option<u16>) -> Self {
        let port = port.unwrap_or(DEFAULT_TURN_PORT);
        Self::new(SocketAddr::new(ip, port))
    }
}

#[derive(Debug, Clone)]
pub struct TurnAllocation {
    pub server: SocketAddr,
    pub relayed_addr: SocketAddr,
    pub lifetime: Duration,
    pub created_at: std::time::Instant,
    pub username: String,
    pub realm: Option<String>,
}

impl TurnAllocation {
    pub fn new(
        server: SocketAddr,
        relayed_addr: SocketAddr,
        lifetime: Duration,
        username: String,
        realm: Option<String>,
    ) -> Self {
        Self {
            server,
            relayed_addr,
            lifetime,
            created_at: std::time::Instant::now(),
            username,
            realm,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.lifetime
    }

    pub fn time_until_expiry(&self) -> Duration {
        self.lifetime.saturating_sub(self.created_at.elapsed())
    }

    pub fn needs_refresh(&self) -> bool {
        let elapsed = self.created_at.elapsed();
        let margin = Duration::from_secs(TURN_REFRESH_MARGIN_SECONDS as u64);
        elapsed + margin >= self.lifetime
    }
}

#[derive(Debug, Clone)]
pub struct TurnPermission {
    pub peer_addr: SocketAddr,
    pub created_at: std::time::Instant,
    pub lifetime: Duration,
}

impl TurnPermission {
    pub fn new(peer_addr: SocketAddr) -> Self {
        Self {
            peer_addr,
            created_at: std::time::Instant::now(),
            lifetime: Duration::from_secs(300),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.lifetime
    }
}

pub struct TurnClient {
    config: TurnConfig,
    allocations: Vec<TurnAllocation>,
    permissions: Vec<TurnPermission>,
}

impl TurnClient {
    pub fn new(config: TurnConfig) -> Self {
        Self {
            config,
            allocations: vec![],
            permissions: vec![],
        }
    }

    pub fn with_servers(servers: Vec<TurnServer>) -> Self {
        let config = TurnConfig {
            servers,
            ..Default::default()
        };
        Self::new(config)
    }

    pub fn with_credentials(mut self, username: String, password: String) -> Self {
        self.config.username = Some(username);
        self.config.password = Some(password);
        self
    }

    pub fn config(&self) -> &TurnConfig {
        &self.config
    }

    pub fn has_servers(&self) -> bool {
        !self.config.servers.is_empty()
    }

    pub fn has_credentials(&self) -> bool {
        self.config.username.is_some() && self.config.password.is_some()
    }

    pub async fn create_allocation(&mut self, server: &TurnServer) -> AgoraResult<TurnAllocation> {
        if !self.has_credentials() {
            return Err(Error::Network("TURN credentials required".to_string()));
        }

        let username = self.config.username.clone().unwrap();
        let realm = server.realm.clone();

        let allocation = TurnAllocation::new(
            server.address,
            self.simulate_relay_address()?,
            self.config.lifetime,
            username,
            realm,
        );

        tracing::info!(
            "Created TURN allocation on {} -> {}",
            server.address,
            allocation.relayed_addr
        );

        self.allocations.push(allocation.clone());
        Ok(allocation)
    }

    fn simulate_relay_address(&self) -> AgoraResult<SocketAddr> {
        let port: u16 = 50000 + (rand::random::<u16>() % 10000);
        Ok(SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(203, 0, 113, 1)), port))
    }

    pub fn get_allocation(&self, server: SocketAddr) -> Option<&TurnAllocation> {
        self.allocations.iter().find(|a| a.server == server)
    }

    pub fn get_active_allocation(&self) -> Option<&TurnAllocation> {
        self.allocations.iter().find(|a| !a.is_expired())
    }

    pub async fn refresh_allocation(&mut self, server: SocketAddr) -> AgoraResult<()> {
        if let Some(idx) = self.allocations.iter().position(|a| a.server == server) {
            let allocation = &mut self.allocations[idx];
            allocation.lifetime = self.config.lifetime;
            allocation.created_at = std::time::Instant::now();
            tracing::debug!("Refreshed TURN allocation for {}", server);
        }
        Ok(())
    }

    pub async fn create_permission(&mut self, peer_addr: SocketAddr) -> AgoraResult<()> {
        let permission = TurnPermission::new(peer_addr);
        tracing::debug!("Created TURN permission for {}", peer_addr);
        self.permissions.push(permission);
        Ok(())
    }

    pub fn has_permission(&self, peer_addr: SocketAddr) -> bool {
        self.permissions
            .iter()
            .any(|p| p.peer_addr == peer_addr && !p.is_expired())
    }

    pub fn cleanup_expired(&mut self) {
        self.allocations.retain(|a| !a.is_expired());
        self.permissions.retain(|p| !p.is_expired());
        tracing::debug!(
            "Cleaned up expired TURN resources: {} allocations, {} permissions remaining",
            self.allocations.len(),
            self.permissions.len()
        );
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }

    pub fn permission_count(&self) -> usize {
        self.permissions.len()
    }
}

#[derive(Debug, Clone)]
pub struct TurnCandidate {
    pub relayed_addr: SocketAddr,
    pub server: SocketAddr,
    pub priority: u32,
}

impl TurnCandidate {
    pub fn new(relayed_addr: SocketAddr, server: SocketAddr, local_preference: u16) -> Self {
        let priority = crate::ice::Candidate::compute_priority(
            crate::ice::CandidateType::Relayed,
            local_preference,
            1,
        );
        Self {
            relayed_addr,
            server,
            priority,
        }
    }

    pub fn to_ice_candidate(&self) -> crate::ice::Candidate {
        crate::ice::Candidate::new_relayed(
            self.relayed_addr,
            self.server,
            1,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_server_creation() {
        let server = TurnServer::from_ip(std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 1)), Some(3478));
        assert_eq!(server.address.port(), 3478);
        assert!(!server.use_tls);
    }

    #[test]
    fn test_turn_server_tls() {
        let server = TurnServer::from_ip(std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 1)), Some(5349))
            .with_tls();
        assert!(server.use_tls);
    }

    #[test]
    fn test_turn_allocation_expiry() {
        let allocation = TurnAllocation::new(
            "127.0.0.1:3478".parse().unwrap(),
            "203.0.113.1:50000".parse().unwrap(),
            Duration::from_millis(50),
            "user".to_string(),
            None,
        );

        assert!(!allocation.is_expired());
        std::thread::sleep(Duration::from_millis(60));
        assert!(allocation.is_expired());
    }

    #[test]
    fn test_turn_allocation_needs_refresh() {
        let allocation = TurnAllocation::new(
            "127.0.0.1:3478".parse().unwrap(),
            "203.0.113.1:50000".parse().unwrap(),
            Duration::from_secs(100),
            "user".to_string(),
            None,
        );

        assert!(!allocation.needs_refresh());
    }

    #[test]
    fn test_turn_permission_expiry() {
        let permission = TurnPermission::new("192.168.1.1:5000".parse().unwrap());
        assert!(!permission.is_expired());
    }

    #[test]
    fn test_turn_client_creation() {
        let client = TurnClient::new(TurnConfig::default());
        assert!(!client.has_servers());
        assert!(!client.has_credentials());
    }

    #[test]
    fn test_turn_client_with_servers() {
        let server = TurnServer::from_ip(std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 1)), None);
        let client = TurnClient::with_servers(vec![server]);

        assert!(client.has_servers());
        assert!(!client.has_credentials());
    }

    #[test]
    fn test_turn_client_with_credentials() {
        let client = TurnClient::new(TurnConfig::default())
            .with_credentials("user".to_string(), "pass".to_string());

        assert!(client.has_credentials());
    }

    #[test]
    fn test_turn_candidate_priority() {
        let candidate = TurnCandidate::new(
            "203.0.113.1:50000".parse().unwrap(),
            "127.0.0.1:3478".parse().unwrap(),
            0,
        );

        assert!(candidate.priority > 0);
    }

    #[test]
    fn test_turn_candidate_to_ice() {
        let turn_candidate = TurnCandidate::new(
            "203.0.113.1:50000".parse().unwrap(),
            "127.0.0.1:3478".parse().unwrap(),
            0,
        );

        let ice_candidate = turn_candidate.to_ice_candidate();
        assert_eq!(ice_candidate.candidate_type, crate::ice::CandidateType::Relayed);
        assert_eq!(ice_candidate.connection_addr, "203.0.113.1:50000".parse().unwrap());
    }
}
