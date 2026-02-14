use crate::error::AgoraResult;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    Public,
    FullCone,
    RestrictedCone,
    PortRestricted,
    Symmetric,
    Unknown,
}

impl NatType {
    pub fn can_hole_punch(&self) -> bool {
        match self {
            NatType::Public => true,
            NatType::FullCone => true,
            NatType::RestrictedCone => true,
            NatType::PortRestricted => true,
            NatType::Symmetric => false,
            NatType::Unknown => false,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            NatType::Public => "Public IP (no NAT)",
            NatType::FullCone => "Full Cone NAT (easiest for P2P)",
            NatType::RestrictedCone => "Restricted Cone NAT",
            NatType::PortRestricted => "Port Restricted NAT",
            NatType::Symmetric => "Symmetric NAT (requires TURN relay)",
            NatType::Unknown => "Unknown NAT type",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObservedAddr {
    pub public_ip: IpAddr,
    pub public_port: u16,
    pub local_ip: IpAddr,
    pub local_port: u16,
    pub nat_type: NatType,
}

impl ObservedAddr {
    pub fn new(public: SocketAddr, local: SocketAddr, nat_type: NatType) -> Self {
        Self {
            public_ip: public.ip(),
            public_port: public.port(),
            local_ip: local.ip(),
            local_port: local.port(),
            nat_type,
        }
    }

    pub fn to_multiaddr(&self) -> String {
        match self.public_ip {
            IpAddr::V4(ip) => format!("/ip4/{}/tcp/{}", ip, self.public_port),
            IpAddr::V6(ip) => format!("/ip6/{}/tcp/{}", ip, self.public_port),
        }
    }
}

pub struct StunConfig {
    pub servers: Vec<String>,
    pub timeout: Duration,
    pub retry_count: u32,
}

impl Default for StunConfig {
    fn default() -> Self {
        Self {
            servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
                "stun:stun2.l.google.com:19302".to_string(),
                "stun:stun.stunprotocol.org:3478".to_string(),
            ],
            timeout: Duration::from_secs(5),
            retry_count: 3,
        }
    }
}

pub struct NatTraversal {
    config: StunConfig,
    observed_addr: Option<ObservedAddr>,
}

impl NatTraversal {
    pub fn new(config: Option<StunConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            observed_addr: None,
        }
    }

    pub async fn detect_nat_type(&mut self) -> AgoraResult<NatType> {
        tracing::info!("Detecting NAT type...");

        // Simplified NAT detection
        // In a full implementation, this would use STUN to determine NAT type
        // For now, we assume most users are behind some form of NAT

        let nat_type = self.probe_nat_type().await?;

        tracing::info!(
            nat_type = ?nat_type,
            can_hole_punch = nat_type.can_hole_punch(),
            "NAT detection complete"
        );

        Ok(nat_type)
    }

    async fn probe_nat_type(&self) -> AgoraResult<NatType> {
        // This is a simplified implementation
        // A full implementation would:
        // 1. Send STUN requests to multiple servers
        // 2. Compare mapped addresses
        // 3. Determine NAT type based on RFC 3489

        // For now, we'll return Unknown and let the connection logic figure it out
        Ok(NatType::Unknown)
    }

    pub async fn get_observed_address(&mut self) -> AgoraResult<&ObservedAddr> {
        if self.observed_addr.is_none() {
            self.detect_nat_type().await?;
        }
        Ok(self.observed_addr.as_ref().unwrap())
    }

    pub fn get_stun_servers(&self) -> &[String] {
        &self.config.servers
    }
}

#[derive(Debug, Clone)]
pub struct HolePunchResult {
    pub success: bool,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub method: HolePunchMethod,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolePunchMethod {
    Direct,
    TcpHolePunch,
    UdpHolePunch,
    Upnp,
    TurnRelay,
}

impl HolePunchResult {
    pub fn direct(local: SocketAddr, remote: SocketAddr, latency_ms: u64) -> Self {
        Self {
            success: true,
            local_addr: local,
            remote_addr: remote,
            method: HolePunchMethod::Direct,
            latency_ms,
        }
    }

    pub fn hole_punched(
        local: SocketAddr,
        remote: SocketAddr,
        method: HolePunchMethod,
        latency_ms: u64,
    ) -> Self {
        Self {
            success: true,
            local_addr: local,
            remote_addr: remote,
            method,
            latency_ms,
        }
    }

    pub fn failed() -> Self {
        Self {
            success: false,
            local_addr: "0.0.0.0:0".parse().unwrap(),
            remote_addr: "0.0.0.0:0".parse().unwrap(),
            method: HolePunchMethod::Direct,
            latency_ms: 0,
        }
    }
}

pub async fn attempt_hole_punch(
    local_addrs: &[SocketAddr],
    remote_addrs: &[SocketAddr],
    _timeout: Duration,
) -> AgoraResult<HolePunchResult> {
    let start = std::time::Instant::now();

    tracing::debug!(
        local_addrs = ?local_addrs,
        remote_addrs = ?remote_addrs,
        "Attempting hole punch"
    );

    // Try direct connections first
    for local in local_addrs {
        for remote in remote_addrs {
            if let Ok(Ok(_)) = tokio::time::timeout(
                Duration::from_millis(500),
                attempt_direct_connection(*local, *remote),
            )
            .await
            {
                let latency = start.elapsed().as_millis() as u64;
                tracing::info!(
                    local = ?local,
                    remote = ?remote,
                    latency_ms = latency,
                    "Direct connection successful"
                );
                return Ok(HolePunchResult::direct(*local, *remote, latency));
            }
        }
    }

    // If direct fails, try hole punching (simplified)
    tracing::info!("Direct connection failed, hole punch required");

    // For now, return failure - actual hole punch implementation would
    // coordinate with the remote peer via a signaling channel
    Ok(HolePunchResult::failed())
}

async fn attempt_direct_connection(_local: SocketAddr, remote: SocketAddr) -> std::io::Result<()> {
    use tokio::net::TcpStream;

    let stream = TcpStream::connect(remote).await?;
    let _ = stream.peer_addr()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nat_type_can_hole_punch() {
        assert!(NatType::Public.can_hole_punch());
        assert!(NatType::FullCone.can_hole_punch());
        assert!(!NatType::Symmetric.can_hole_punch());
    }

    #[test]
    fn test_nat_type_description() {
        assert!(NatType::Public.description().contains("Public"));
        assert!(NatType::Symmetric.description().contains("TURN"));
    }

    #[test]
    fn test_observed_addr_multiaddr() {
        let addr = SocketAddr::new("1.2.3.4".parse().unwrap(), 12345);
        let local = SocketAddr::new("192.168.1.1".parse().unwrap(), 54321);
        let observed = ObservedAddr::new(addr, local, NatType::FullCone);

        assert_eq!(observed.to_multiaddr(), "/ip4/1.2.3.4/tcp/12345");
    }

    #[test]
    fn test_stun_config_default() {
        let config = StunConfig::default();
        assert!(!config.servers.is_empty());
        assert!(config.servers[0].contains("google.com"));
    }
}
