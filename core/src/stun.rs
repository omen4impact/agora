use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use stun::agent::TransactionId;
use stun::message::{Getter, Message, BINDING_REQUEST, BINDING_SUCCESS};
use stun::xoraddr::XorMappedAddress;
use crate::error::{Error, AgoraResult};
use crate::nat::NatType;

pub const DEFAULT_STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun2.l.google.com:19302",
    "stun3.l.google.com:19302",
    "stun4.l.google.com:19302",
];

#[derive(Debug, Clone)]
pub struct StunBinding {
    pub public_ip: IpAddr,
    pub public_port: u16,
    pub server: String,
}

#[derive(Debug, Clone)]
pub struct StunResult {
    pub binding: Option<StunBinding>,
    pub nat_type: NatType,
    pub can_hole_punch: bool,
}

pub struct StunClient {
    servers: Vec<String>,
    timeout: Duration,
    local_port: u16,
}

impl StunClient {
    pub fn new() -> Self {
        Self {
            servers: DEFAULT_STUN_SERVERS.iter().map(|s| s.to_string()).collect(),
            timeout: Duration::from_secs(5),
            local_port: 0,
        }
    }
    
    pub fn with_servers(servers: Vec<String>) -> Self {
        Self {
            servers,
            timeout: Duration::from_secs(5),
            local_port: 0,
        }
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    pub fn with_local_port(mut self, port: u16) -> Self {
        self.local_port = port;
        self
    }
    
    pub async fn get_public_address(&self) -> AgoraResult<StunBinding> {
        let mut last_error = None;
        
        for server in &self.servers {
            match self.query_server(server).await {
                Ok(binding) => return Ok(binding),
                Err(e) => {
                    tracing::debug!("STUN query to {} failed: {}", server, e);
                    last_error = Some(e);
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| Error::Network("No STUN servers available".to_string())))
    }
    
    async fn query_server(&self, server: &str) -> AgoraResult<StunBinding> {
        let server_str = server.to_string();
        let server_addr: SocketAddr = tokio::task::spawn_blocking(move || {
            server_str.to_socket_addrs()
                .ok()
                .and_then(|mut addrs| addrs.next())
        })
        .await
        .map_err(|e| Error::Network(format!("Task join error: {}", e)))?
        .ok_or_else(|| Error::Network(format!("Failed to resolve STUN server: {}", server)))?;
        
        let local_addr: SocketAddr = if self.local_port > 0 {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.local_port)
        } else {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
        };
        
        let socket = UdpSocket::bind(local_addr)
            .map_err(|e| Error::Network(format!("Failed to bind UDP socket: {}", e)))?;
        
        socket.set_read_timeout(Some(self.timeout))
            .map_err(|e| Error::Network(format!("Failed to set timeout: {}", e)))?;
        
        socket.connect(server_addr)
            .map_err(|e| Error::Network(format!("Failed to connect to STUN server: {}", e)))?;
        
        let mut msg = Message::new();
        msg.build(&[
            Box::new(BINDING_REQUEST),
            Box::new(TransactionId::new()),
        ])
        .map_err(|e| Error::Network(format!("Failed to build STUN request: {}", e)))?;
        
        socket.send(&msg.raw)
            .map_err(|e| Error::Network(format!("Failed to send STUN request: {}", e)))?;
        
        let mut buf = vec![0u8; 1500];
        let n = socket.recv(&mut buf)
            .map_err(|e| Error::Network(format!("Failed to receive STUN response: {}", e)))?;
        
        let mut response = Message::new();
        response.raw = buf[..n].to_vec();
        response.decode()
            .map_err(|e| Error::Network(format!("Failed to decode STUN response: {}", e)))?;
        
        if response.typ != BINDING_SUCCESS {
            return Err(Error::Network("Invalid STUN response type".to_string()));
        }
        
        let mut xor_addr = XorMappedAddress::default();
        xor_addr.get_from(&response)
            .map_err(|e| Error::Network(format!("Failed to get XOR-MAPPED-ADDRESS: {}", e)))?;
        
        Ok(StunBinding {
            public_ip: xor_addr.ip,
            public_port: xor_addr.port,
            server: server.to_string(),
        })
    }
    
    pub async fn detect_nat_type(&self) -> AgoraResult<StunResult> {
        let binding = match self.get_public_address().await {
            Ok(b) => b,
            Err(_) => {
                return Ok(StunResult {
                    binding: None,
                    nat_type: NatType::Unknown,
                    can_hole_punch: false,
                });
            }
        };
        
        let nat_type = if self.servers.len() >= 2 {
            self.determine_nat_type(&binding).await
        } else {
            NatType::Unknown
        };
        
        let can_hole_punch = nat_type.can_hole_punch();
        
        Ok(StunResult {
            binding: Some(binding),
            nat_type,
            can_hole_punch,
        })
    }
    
    async fn determine_nat_type(&self, first_binding: &StunBinding) -> NatType {
        if self.servers.len() < 2 {
            return NatType::Unknown;
        }
        
        let second_server = self.servers.get(1).unwrap();
        
        let second_binding = match self.query_server(second_server).await {
            Ok(b) => b,
            Err(_) => return NatType::Unknown,
        };
        
        if first_binding.public_ip == second_binding.public_ip 
            && first_binding.public_port == second_binding.public_port {
            NatType::FullCone
        } else {
            NatType::Symmetric
        }
    }
}

impl Default for StunClient {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_stun_url(url: &str) -> Option<SocketAddr> {
    url.to_socket_addrs().ok()?.next()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stun_client_creation() {
        let client = StunClient::new();
        assert!(!client.servers.is_empty());
        assert_eq!(client.timeout, Duration::from_secs(5));
    }
    
    #[test]
    fn test_stun_client_with_custom_servers() {
        let client = StunClient::with_servers(vec!["stun.example.com:3478".to_string()]);
        assert_eq!(client.servers.len(), 1);
    }
    
    #[tokio::test]
    async fn test_stun_client_get_public_address() {
        let client = StunClient::new().with_timeout(Duration::from_secs(10));
        let result = client.get_public_address().await;
        
        if let Ok(binding) = result {
            assert!(!binding.public_ip.is_unspecified());
            assert!(binding.public_port > 0);
            println!("Public IP: {}:{}", binding.public_ip, binding.public_port);
        }
    }
    
    #[tokio::test]
    async fn test_stun_client_detect_nat_type() {
        let client = StunClient::new().with_timeout(Duration::from_secs(10));
        let result = client.detect_nat_type().await;
        
        if let Ok(stun_result) = result {
            println!("NAT Type: {}", stun_result.nat_type.description());
            println!("Can Hole Punch: {}", stun_result.can_hole_punch);
            if let Some(binding) = stun_result.binding {
                println!("Public Address: {}:{}", binding.public_ip, binding.public_port);
            }
        }
    }
}
