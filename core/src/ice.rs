use crate::error::{AgoraResult, Error};
use crate::stun::{StunBinding, StunClient};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

pub const ICE_DEFAULT_STUN_SERVERS: &[&str] =
    &["stun.l.google.com:19302", "stun1.l.google.com:19302"];

pub const ICE_CONNECTIVITY_TIMEOUT: Duration = Duration::from_secs(5);
pub const ICE_CHECK_INTERVAL: Duration = Duration::from_millis(50);
pub const ICE_NOMINATION_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateType {
    Host,
    ServerReflexive,
    PeerReflexive,
    Relayed,
}

impl std::fmt::Display for CandidateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandidateType::Host => write!(f, "host"),
            CandidateType::ServerReflexive => write!(f, "srflx"),
            CandidateType::PeerReflexive => write!(f, "prflx"),
            CandidateType::Relayed => write!(f, "relay"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Candidate {
    pub foundation: String,
    pub component_id: u16,
    pub transport: TransportType,
    pub priority: u32,
    pub connection_addr: SocketAddr,
    pub base_addr: SocketAddr,
    pub candidate_type: CandidateType,
    pub related_addr: Option<SocketAddr>,
}

impl Candidate {
    pub fn new_host(addr: SocketAddr, component_id: u16) -> Self {
        let foundation = format!("host-{}", addr);
        let priority = Self::calculate_priority(CandidateType::Host, 0, component_id);

        Self {
            foundation,
            component_id,
            transport: TransportType::Udp,
            priority,
            connection_addr: addr,
            base_addr: addr,
            candidate_type: CandidateType::Host,
            related_addr: None,
        }
    }

    pub fn new_server_reflexive(
        public_addr: SocketAddr,
        base_addr: SocketAddr,
        component_id: u16,
    ) -> Self {
        let foundation = format!("srflx-{}", public_addr);
        let priority = Self::calculate_priority(CandidateType::ServerReflexive, 0, component_id);

        Self {
            foundation,
            component_id,
            transport: TransportType::Udp,
            priority,
            connection_addr: public_addr,
            base_addr,
            candidate_type: CandidateType::ServerReflexive,
            related_addr: Some(base_addr),
        }
    }

    pub fn new_relayed(relay_addr: SocketAddr, server_addr: SocketAddr, component_id: u16) -> Self {
        let foundation = format!("relay-{}", relay_addr);
        let priority = Self::calculate_priority(CandidateType::Relayed, 0, component_id);

        Self {
            foundation,
            component_id,
            transport: TransportType::Udp,
            priority,
            connection_addr: relay_addr,
            base_addr: server_addr,
            candidate_type: CandidateType::Relayed,
            related_addr: Some(server_addr),
        }
    }

    fn calculate_priority(
        candidate_type: CandidateType,
        local_pref: u16,
        component_id: u16,
    ) -> u32 {
        let type_pref = match candidate_type {
            CandidateType::Host => 126,
            CandidateType::PeerReflexive => 110,
            CandidateType::ServerReflexive => 100,
            CandidateType::Relayed => 0,
        };

        2u32.pow(24) * type_pref as u32 + 2u32.pow(8) * local_pref as u32 + 256
            - component_id as u32
    }

    pub fn compute_priority(
        candidate_type: CandidateType,
        local_pref: u16,
        component_id: u16,
    ) -> u32 {
        Self::calculate_priority(candidate_type, local_pref, component_id)
    }

    pub fn to_sdp(&self) -> String {
        let ip = self.connection_addr.ip();
        let port = self.connection_addr.port();
        let typ = self.candidate_type.to_string();

        let mut sdp = format!(
            "candidate:{} {} {} {} {} {} typ {}",
            self.foundation,
            self.component_id,
            self.transport.to_string().to_lowercase(),
            self.priority,
            ip,
            port,
            typ
        );

        if let Some(related) = self.related_addr {
            sdp.push_str(&format!(" raddr {} rport {}", related.ip(), related.port()));
        }

        sdp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportType {
    Udp,
    Tcp,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Udp => write!(f, "UDP"),
            TransportType::Tcp => write!(f, "TCP"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidatePairState {
    Frozen,
    Waiting,
    InProgress,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone)]
pub struct CandidatePair {
    pub local: Candidate,
    pub remote: Candidate,
    pub priority: u64,
    pub state: CandidatePairState,
    pub nominated: bool,
    pub last_check: Option<Instant>,
    pub round_trip_time: Option<Duration>,
}

impl CandidatePair {
    pub fn new(local: Candidate, remote: Candidate) -> Self {
        let priority = Self::calculate_pair_priority(&local, &remote);

        Self {
            local,
            remote,
            priority,
            state: CandidatePairState::Frozen,
            nominated: false,
            last_check: None,
            round_trip_time: None,
        }
    }

    fn calculate_pair_priority(local: &Candidate, remote: &Candidate) -> u64 {
        let (g, d) = if local.priority >= remote.priority {
            (local.priority as u64, remote.priority as u64)
        } else {
            (remote.priority as u64, local.priority as u64)
        };

        (2u64.pow(32) * g.min(d)) + (2u64 * g.max(d)) + (g.max(d) - g.min(d))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceRole {
    Controlling,
    Controlled,
}

#[derive(Debug, Clone)]
pub struct IceConfig {
    pub stun_servers: Vec<String>,
    pub turn_servers: Vec<TurnServer>,
    pub local_preferences: u16,
    pub connectivity_timeout: Duration,
    pub nomination_mode: NominationMode,
}

impl Default for IceConfig {
    fn default() -> Self {
        Self {
            stun_servers: ICE_DEFAULT_STUN_SERVERS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            turn_servers: Vec::new(),
            local_preferences: 65535,
            connectivity_timeout: ICE_CONNECTIVITY_TIMEOUT,
            nomination_mode: NominationMode::Aggressive,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnServer {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NominationMode {
    Aggressive,
    Regular,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    New,
    Checking,
    Connected,
    Completed,
    Failed,
    Disconnected,
    Closed,
}

pub struct IceAgent {
    config: IceConfig,
    role: IceRole,
    local_candidates: Vec<Candidate>,
    remote_candidates: Vec<Candidate>,
    candidate_pairs: Vec<CandidatePair>,
    checklist: Vec<CandidatePair>,
    selected_pair: Option<CandidatePair>,
    state: ConnectionState,
    tie_breaker: u64,
    component_id: u16,
}

impl IceAgent {
    pub fn new(config: Option<IceConfig>) -> Self {
        use rand::Rng;

        Self {
            config: config.unwrap_or_default(),
            role: IceRole::Controlling,
            local_candidates: Vec::new(),
            remote_candidates: Vec::new(),
            candidate_pairs: Vec::new(),
            checklist: Vec::new(),
            selected_pair: None,
            state: ConnectionState::New,
            tie_breaker: rand::thread_rng().gen(),
            component_id: 1,
        }
    }

    pub fn with_role(mut self, role: IceRole) -> Self {
        self.role = role;
        self
    }

    pub fn with_component(mut self, component_id: u16) -> Self {
        self.component_id = component_id;
        self
    }

    pub fn with_turn_server(
        mut self,
        url: String,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        self.config.turn_servers.push(TurnServer {
            url,
            username,
            password,
        });
        self
    }

    pub fn with_turn_servers(mut self, servers: Vec<TurnServer>) -> Self {
        self.config.turn_servers = servers;
        self
    }

    pub fn has_turn_servers(&self) -> bool {
        !self.config.turn_servers.is_empty()
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn selected_pair(&self) -> Option<&CandidatePair> {
        self.selected_pair.as_ref()
    }

    pub fn local_candidates(&self) -> &[Candidate] {
        &self.local_candidates
    }

    pub fn remote_candidates(&self) -> &[Candidate] {
        &self.remote_candidates
    }

    pub fn role(&self) -> IceRole {
        self.role
    }

    pub fn tie_breaker(&self) -> u64 {
        self.tie_breaker
    }

    pub async fn gather_candidates(&mut self) -> AgoraResult<()> {
        tracing::info!("Starting ICE candidate gathering");

        self.gather_host_candidates()?;
        self.gather_server_reflexive_candidates().await?;
        self.gather_relayed_candidates().await?;

        self.form_candidate_pairs();

        tracing::info!(
            local_count = self.local_candidates.len(),
            remote_count = self.remote_candidates.len(),
            pair_count = self.candidate_pairs.len(),
            "Candidate gathering complete"
        );

        Ok(())
    }

    fn gather_host_candidates(&mut self) -> AgoraResult<()> {
        use std::net::UdpSocket;

        let local_addrs = get_local_addresses()
            .map_err(|e| Error::Network(format!("Failed to get local addresses: {}", e)))?;

        for addr in local_addrs {
            let socket_addr = SocketAddr::new(addr, 0);

            if let Ok(socket) = UdpSocket::bind(socket_addr) {
                if let Ok(local_addr) = socket.local_addr() {
                    let candidate = Candidate::new_host(local_addr, self.component_id);
                    self.local_candidates.push(candidate);
                    tracing::debug!("Added host candidate: {}", local_addr);
                }
            }
        }

        Ok(())
    }

    async fn gather_server_reflexive_candidates(&mut self) -> AgoraResult<()> {
        for stun_server in &self.config.stun_servers {
            let stun_client = StunClient::with_servers(vec![stun_server.clone()]);

            if let Ok(binding) = stun_client.get_public_address().await {
                let base_addr = self.find_base_address(&binding);

                if let Some(base) = base_addr {
                    let candidate = Candidate::new_server_reflexive(
                        SocketAddr::new(binding.public_ip, binding.public_port),
                        base,
                        self.component_id,
                    );

                    self.local_candidates.push(candidate);
                    tracing::debug!(
                        "Added srflx candidate: {} (base: {})",
                        SocketAddr::new(binding.public_ip, binding.public_port),
                        base
                    );
                }
            }
        }

        Ok(())
    }

    async fn gather_relayed_candidates(&mut self) -> AgoraResult<()> {
        for turn_server in &self.config.turn_servers {
            if turn_server.username.is_none() || turn_server.password.is_none() {
                tracing::warn!(
                    "TURN server {} has no credentials, skipping",
                    turn_server.url
                );
                continue;
            }

            tracing::info!(
                "Attempting to create TURN allocation on {}",
                turn_server.url
            );

            let server = if let Ok(addr) = turn_server.url.parse::<SocketAddr>() {
                crate::turn::TurnServer::new(addr)
            } else if let Ok(ip) = turn_server.url.parse::<std::net::IpAddr>() {
                crate::turn::TurnServer::from_ip(ip, None)
            } else {
                let port = turn_server
                    .url
                    .split(':')
                    .nth(1)
                    .and_then(|p| p.parse().ok());
                crate::turn::TurnServer::from_host(
                    turn_server
                        .url
                        .split(':')
                        .next()
                        .unwrap_or(&turn_server.url),
                    port,
                )
                .map_err(|e| Error::Network(format!("Invalid TURN server: {}", e)))?
            };

            let turn_config = crate::turn::TurnConfig {
                servers: vec![server.clone()],
                username: turn_server.username.clone(),
                password: turn_server.password.clone(),
                ..Default::default()
            };

            let mut turn_client = crate::turn::TurnClient::new(turn_config);

            match turn_client.create_allocation(&server).await {
                Ok(allocation) => {
                    let candidate = Candidate::new_relayed(
                        allocation.relayed_addr,
                        server.address,
                        self.component_id,
                    );

                    self.local_candidates.push(candidate);
                    tracing::info!(
                        "Added relayed candidate: {} via {}",
                        allocation.relayed_addr,
                        server.address
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to create TURN allocation: {}", e);
                }
            }
        }

        Ok(())
    }

    fn find_base_address(&self, _binding: &StunBinding) -> Option<SocketAddr> {
        for candidate in &self.local_candidates {
            if candidate.candidate_type == CandidateType::Host {
                if let IpAddr::V4(local_ip) = candidate.connection_addr.ip() {
                    if !local_ip.is_loopback() && !local_ip.is_link_local() {
                        return Some(candidate.connection_addr);
                    }
                }
            }
        }

        self.local_candidates
            .iter()
            .find(|c| c.candidate_type == CandidateType::Host)
            .map(|c| c.connection_addr)
    }

    pub fn add_remote_candidate(&mut self, candidate: Candidate) {
        if !self.remote_candidates.contains(&candidate) {
            tracing::debug!("Adding remote candidate: {}", candidate.connection_addr);
            self.remote_candidates.push(candidate);
            self.form_candidate_pairs();
        }
    }

    pub fn add_remote_candidates(&mut self, candidates: Vec<Candidate>) {
        for candidate in candidates {
            self.add_remote_candidate(candidate);
        }
    }

    fn form_candidate_pairs(&mut self) {
        let mut new_pairs: Vec<CandidatePair> = Vec::new();

        for local in &self.local_candidates {
            for remote in &self.remote_candidates {
                let pair = CandidatePair::new(local.clone(), remote.clone());

                let exists = self.candidate_pairs.iter().any(|p| {
                    p.local.connection_addr == pair.local.connection_addr
                        && p.remote.connection_addr == pair.remote.connection_addr
                });

                if !exists {
                    new_pairs.push(pair);
                }
            }
        }

        self.candidate_pairs.extend(new_pairs);
        self.sort_pairs_by_priority();
        self.update_checklist();
    }

    fn sort_pairs_by_priority(&mut self) {
        self.candidate_pairs
            .sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    fn update_checklist(&mut self) {
        self.checklist = self.candidate_pairs.clone();

        let mut seen_foundation: HashSet<String> = HashSet::new();

        for pair in &mut self.checklist {
            if !seen_foundation.contains(&pair.local.foundation) {
                pair.state = CandidatePairState::Waiting;
                seen_foundation.insert(pair.local.foundation.clone());
            }
        }

        if self.state == ConnectionState::New && !self.checklist.is_empty() {
            self.state = ConnectionState::Checking;
        }
    }

    pub async fn perform_connectivity_checks(&mut self) -> AgoraResult<()> {
        tracing::info!("Starting connectivity checks");

        let start_time = Instant::now();
        let timeout = self.config.connectivity_timeout;

        while start_time.elapsed() < timeout {
            let pending_pairs: Vec<(usize, SocketAddr, SocketAddr)> = self
                .checklist
                .iter()
                .enumerate()
                .filter(|(_, pair)| {
                    matches!(
                        pair.state,
                        CandidatePairState::Waiting | CandidatePairState::InProgress
                    )
                })
                .map(|(i, pair)| (i, pair.local.base_addr, pair.remote.connection_addr))
                .collect();

            if pending_pairs.is_empty() {
                break;
            }

            for (idx, local_addr, remote_addr) in pending_pairs {
                self.checklist[idx].state = CandidatePairState::InProgress;
                self.checklist[idx].last_check = Some(Instant::now());

                let result = self.check_connectivity_impl(local_addr, remote_addr).await;

                match result {
                    Ok(rtt) => {
                        self.checklist[idx].state = CandidatePairState::Succeeded;
                        self.checklist[idx].round_trip_time = Some(rtt);

                        tracing::debug!(
                            local = ?local_addr,
                            remote = ?remote_addr,
                            rtt_ms = rtt.as_millis(),
                            "Connectivity check succeeded"
                        );

                        if self.config.nomination_mode == NominationMode::Aggressive {
                            self.nominate_pair(idx);
                            break;
                        }
                    }
                    Err(_) => {
                        self.checklist[idx].state = CandidatePairState::Failed;
                        tracing::debug!(
                            local = ?local_addr,
                            remote = ?remote_addr,
                            "Connectivity check failed"
                        );
                    }
                }
            }

            tokio::time::sleep(ICE_CHECK_INTERVAL).await;
        }

        self.finalize_selection()
    }

    async fn check_connectivity_impl(
        &self,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> AgoraResult<Duration> {
        use tokio::net::UdpSocket;

        let socket = UdpSocket::bind(local_addr)
            .await
            .map_err(|e| Error::Network(format!("Failed to bind socket: {}", e)))?;

        let start = Instant::now();

        let stun_request = self.create_binding_request();

        socket
            .send_to(&stun_request, remote_addr)
            .await
            .map_err(|e| Error::Network(format!("Failed to send STUN request: {}", e)))?;

        let mut buf = vec![0u8; 1500];

        let recv_result =
            tokio::time::timeout(Duration::from_millis(500), socket.recv_from(&mut buf)).await;

        match recv_result {
            Ok(Ok((n, from_addr))) => {
                if from_addr == remote_addr && n > 0 {
                    Ok(start.elapsed())
                } else {
                    Err(Error::Network("Response from wrong address".to_string()))
                }
            }
            _ => Err(Error::Network("No response received".to_string())),
        }
    }

    fn create_binding_request(&self) -> Vec<u8> {
        use stun::agent::TransactionId;
        use stun::message::{Message, BINDING_REQUEST};

        let mut msg = Message::new();
        let _ = msg.build(&[Box::new(BINDING_REQUEST), Box::new(TransactionId::new())]);

        msg.raw.clone()
    }

    fn nominate_pair(&mut self, idx: usize) {
        if let Some(pair) = self.checklist.get_mut(idx) {
            pair.nominated = true;
            self.selected_pair = Some(pair.clone());

            tracing::info!(
                local = ?pair.local.connection_addr,
                remote = ?pair.remote.connection_addr,
                "Nominated pair selected"
            );
        }
    }

    fn finalize_selection(&mut self) -> AgoraResult<()> {
        let succeeded_pairs: Vec<&CandidatePair> = self
            .checklist
            .iter()
            .filter(|p| p.state == CandidatePairState::Succeeded)
            .collect();

        if succeeded_pairs.is_empty() {
            self.state = ConnectionState::Failed;
            return Err(Error::Network("No valid candidate pairs found".to_string()));
        }

        if self.selected_pair.is_none() {
            if let Some(best_pair) = succeeded_pairs.first() {
                self.selected_pair = Some((*best_pair).clone());
            }
        }

        self.state = ConnectionState::Connected;

        tracing::info!(
            selected_local = ?self.selected_pair.as_ref().map(|p| p.local.connection_addr),
            selected_remote = ?self.selected_pair.as_ref().map(|p| p.remote.connection_addr),
            succeeded_pairs = succeeded_pairs.len(),
            "ICE connection established"
        );

        Ok(())
    }

    pub fn get_local_sdp(&self) -> String {
        self.local_candidates
            .iter()
            .map(|c| c.to_sdp())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn get_selected_connection(&self) -> Option<(SocketAddr, SocketAddr)> {
        self.selected_pair
            .as_ref()
            .map(|p| (p.local.base_addr, p.remote.connection_addr))
    }
}

fn get_local_addresses() -> std::io::Result<Vec<IpAddr>> {
    use std::net::UdpSocket;

    let mut addresses = Vec::new();

    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                addresses.push(local_addr.ip());
            }
        }
    }

    if let Ok(socket) = UdpSocket::bind("[::]:0") {
        if socket.connect("[2001:4860:4860::8888]:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                addresses.push(local_addr.ip());
            }
        }
    }

    if addresses.is_empty() {
        addresses.push(IpAddr::V4(Ipv4Addr::LOCALHOST));
    }

    Ok(addresses)
}

pub fn parse_candidate_from_sdp(sdp: &str) -> Option<Candidate> {
    let parts: Vec<&str> = sdp.split_whitespace().collect();

    if parts.len() < 8 {
        return None;
    }

    let first = parts.first()?;
    let foundation = if first.starts_with("candidate:") {
        first.strip_prefix("candidate:")?.to_string()
    } else {
        return None;
    };

    let component_id: u16 = parts.get(1)?.parse().ok()?;
    let transport = match parts.get(2)?.to_lowercase().as_str() {
        "udp" => TransportType::Udp,
        "tcp" => TransportType::Tcp,
        _ => return None,
    };
    let priority: u32 = parts.get(3)?.parse().ok()?;

    let ip: IpAddr = parts.get(4)?.parse().ok()?;
    let port: u16 = parts.get(5)?.parse().ok()?;

    if parts.get(6)? != &"typ" {
        return None;
    }

    let candidate_type = match *parts.get(7)? {
        "host" => CandidateType::Host,
        "srflx" => CandidateType::ServerReflexive,
        "prflx" => CandidateType::PeerReflexive,
        "relay" => CandidateType::Relayed,
        _ => return None,
    };

    let connection_addr = SocketAddr::new(ip, port);
    let base_addr = connection_addr;

    Some(Candidate {
        foundation,
        component_id,
        transport,
        priority,
        connection_addr,
        base_addr,
        candidate_type,
        related_addr: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_priority() {
        let host = Candidate::new_host(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 1234),
            1,
        );
        let srflx = Candidate::new_server_reflexive(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 5678),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 1234),
            1,
        );

        assert!(host.priority > srflx.priority);
    }

    #[test]
    fn test_candidate_to_sdp() {
        let candidate = Candidate::new_host(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 1234),
            1,
        );

        let sdp = candidate.to_sdp();
        assert!(sdp.contains("host"));
        assert!(sdp.contains("192.168.1.1"));
        assert!(sdp.contains("1234"));
    }

    #[test]
    fn test_candidate_pair_priority() {
        let local = Candidate::new_host(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 1234),
            1,
        );
        let remote = Candidate::new_host(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 5678),
            1,
        );

        let pair = CandidatePair::new(local, remote);
        assert!(pair.priority > 0);
    }

    #[test]
    fn test_parse_candidate_from_sdp() {
        let sdp = "candidate:123 1 UDP 12345 192.168.1.1 1234 typ host";
        let candidate = parse_candidate_from_sdp(sdp);

        assert!(candidate.is_some());
        let c = candidate.unwrap();
        assert_eq!(c.foundation, "123");
        assert_eq!(c.component_id, 1);
        assert_eq!(c.transport, TransportType::Udp);
        assert_eq!(c.priority, 12345);
        assert_eq!(c.candidate_type, CandidateType::Host);
    }

    #[test]
    fn test_ice_agent_creation() {
        let agent = IceAgent::new(None);
        assert_eq!(agent.state(), ConnectionState::New);
        assert!(agent.local_candidates().is_empty());
    }

    #[tokio::test]
    async fn test_ice_agent_gather_candidates() {
        let mut agent = IceAgent::new(None);

        if agent.gather_candidates().await.is_ok() {
            assert!(!agent.local_candidates().is_empty());
        }
    }

    #[test]
    fn test_ice_agent_with_turn_server() {
        let agent = IceAgent::new(None).with_turn_server(
            "192.0.2.1:3478".to_string(),
            Some("user".to_string()),
            Some("pass".to_string()),
        );

        assert!(agent.has_turn_servers());
    }

    #[test]
    fn test_relayed_candidate_creation() {
        let relay_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)), 50000);
        let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 3478);

        let candidate = Candidate::new_relayed(relay_addr, server_addr, 1);

        assert_eq!(candidate.candidate_type, CandidateType::Relayed);
        assert_eq!(candidate.connection_addr, relay_addr);
        assert_eq!(candidate.related_addr, Some(server_addr));
    }

    #[test]
    fn test_relayed_candidate_to_sdp() {
        let relay_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)), 50000);
        let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 3478);

        let candidate = Candidate::new_relayed(relay_addr, server_addr, 1);
        let sdp = candidate.to_sdp();

        assert!(sdp.contains("typ relay"));
        assert!(sdp.contains("203.0.113.1"));
    }
}
