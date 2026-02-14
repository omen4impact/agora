use async_trait::async_trait;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};
use tokio::net::TcpStream as TokioTcpStream;
use tokio::time::timeout;

use crate::error::Error;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpPunchMethod {
    SimultaneousOpen,
    Sequential,
    Direct,
}

#[derive(Debug, Clone)]
pub struct TcpHolePunchConfig {
    pub local_port: u16,
    pub remote_endpoints: Vec<SocketAddr>,
    pub timeout: Duration,
    pub retry_count: u8,
}

impl Default for TcpHolePunchConfig {
    fn default() -> Self {
        Self {
            local_port: 0,
            remote_endpoints: Vec::new(),
            timeout: Duration::from_secs(10),
            retry_count: 3,
        }
    }
}

#[derive(Debug)]
pub struct TcpHolePunchResult {
    pub success: bool,
    pub connected_addr: Option<SocketAddr>,
    pub latency_ms: u64,
    pub method: TcpPunchMethod,
}

#[async_trait]
pub trait SignalingChannel: Send + Sync {
    async fn send_ready(&self, endpoints: &[SocketAddr]) -> Result<()>;
    async fn wait_for_peer_ready(&self, timeout: Duration) -> Result<Vec<SocketAddr>>;
}

pub struct TcpHolePuncher {
    config: TcpHolePunchConfig,
}

impl TcpHolePuncher {
    pub fn new(config: TcpHolePunchConfig) -> Self {
        Self { config }
    }

    pub async fn punch(
        &self,
        signaling_channel: &dyn SignalingChannel,
    ) -> Result<TcpHolePunchResult> {
        let start = Instant::now();

        let local_endpoints = self.get_local_endpoints()?;
        signaling_channel.send_ready(&local_endpoints).await?;

        let peer_endpoints = signaling_channel
            .wait_for_peer_ready(self.config.timeout)
            .await?;

        for attempt in 0..self.config.retry_count {
            tracing::info!(
                "TCP hole punch attempt {} of {}",
                attempt + 1,
                self.config.retry_count
            );

            match self.attempt_simultaneous_open(&peer_endpoints).await {
                Ok(stream) => {
                    let addr = stream.peer_addr().map_err(|e| {
                        Error::Network(format!("Failed to get peer address: {}", e))
                    })?;
                    tracing::info!("TCP hole punch successful via {}", addr);
                    return Ok(TcpHolePunchResult {
                        success: true,
                        connected_addr: Some(addr),
                        latency_ms: start.elapsed().as_millis() as u64,
                        method: TcpPunchMethod::SimultaneousOpen,
                    });
                }
                Err(e) => {
                    tracing::debug!("Simultaneous open failed: {}", e);
                }
            }

            if attempt < self.config.retry_count - 1 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        match self.attempt_sequential(&peer_endpoints).await {
            Ok(stream) => {
                let addr = stream
                    .peer_addr()
                    .map_err(|e| Error::Network(format!("Failed to get peer address: {}", e)))?;
                tracing::info!("Sequential connection successful via {}", addr);
                Ok(TcpHolePunchResult {
                    success: true,
                    connected_addr: Some(addr),
                    latency_ms: start.elapsed().as_millis() as u64,
                    method: TcpPunchMethod::Sequential,
                })
            }
            Err(e) => {
                tracing::warn!("All TCP hole punch attempts failed: {}", e);
                Ok(TcpHolePunchResult {
                    success: false,
                    connected_addr: None,
                    latency_ms: start.elapsed().as_millis() as u64,
                    method: TcpPunchMethod::Direct,
                })
            }
        }
    }

    pub async fn attempt_simultaneous_open(
        &self,
        peer_endpoints: &[SocketAddr],
    ) -> Result<TokioTcpStream> {
        if peer_endpoints.is_empty() {
            return Err(Error::Network("No peer endpoints provided".to_string()));
        }

        let local_addr: SocketAddr = if self.config.local_port == 0 {
            "0.0.0.0:0".parse().unwrap()
        } else {
            format!("0.0.0.0:{}", self.config.local_port)
                .parse()
                .unwrap()
        };

        let mut join_set = tokio::task::JoinSet::new();

        for peer_addr in peer_endpoints {
            let local = local_addr;
            let peer = *peer_addr;
            let timeout_dur = self.config.timeout;

            join_set
                .spawn(async move { Self::bind_and_connect_async(local, peer, timeout_dur).await });
        }

        while let Some(result) = join_set.join_next().await {
            if let Ok(Ok(stream)) = result {
                return Ok(stream);
            }
        }

        Err(Error::Network(
            "Simultaneous open failed for all endpoints".to_string(),
        ))
    }

    async fn bind_and_connect_async(
        local: SocketAddr,
        remote: SocketAddr,
        timeout_dur: Duration,
    ) -> Result<TokioTcpStream> {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .map_err(|e| Error::Network(format!("Failed to create socket: {}", e)))?;

        socket
            .set_reuse_address(true)
            .map_err(|e| Error::Network(format!("Failed to set SO_REUSEADDR: {}", e)))?;

        #[cfg(unix)]
        {
            socket
                .set_reuse_port(true)
                .map_err(|e| Error::Network(format!("Failed to set SO_REUSEPORT: {}", e)))?;
        }

        socket
            .bind(&local.into())
            .map_err(|e| Error::Network(format!("Failed to bind to {}: {}", local, e)))?;

        socket
            .set_nonblocking(true)
            .map_err(|e| Error::Network(format!("Failed to set non-blocking: {}", e)))?;

        match socket.connect(&remote.into()) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(libc::EINPROGRESS) => {}
            Err(e) => {
                return Err(Error::Network(format!(
                    "Connection to {} failed: {}",
                    remote, e
                )));
            }
        }

        let std_stream: std::net::TcpStream = socket.into();
        let tokio_stream = TokioTcpStream::from_std(std_stream)
            .map_err(|e| Error::Network(format!("Failed to create tokio stream: {}", e)))?;

        let write_ready = timeout(timeout_dur, tokio_stream.writable()).await;

        match write_ready {
            Ok(Ok(_)) => {
                match tokio_stream.try_write(&[]) {
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => {
                        return Err(Error::Network(format!(
                            "Connection to {} failed: {}",
                            remote, e
                        )));
                    }
                }
                let local_addr = tokio_stream
                    .local_addr()
                    .map_err(|e| Error::Network(format!("Failed to get local address: {}", e)))?;
                tracing::debug!("Connected from {} to {}", local_addr, remote);
                Ok(tokio_stream)
            }
            Ok(Err(e)) => Err(Error::Network(format!(
                "Connection to {} failed: {}",
                remote, e
            ))),
            Err(_) => Err(Error::Network(format!(
                "Connection to {} timed out",
                remote
            ))),
        }
    }

    async fn attempt_sequential(&self, peer_endpoints: &[SocketAddr]) -> Result<TokioTcpStream> {
        for peer_addr in peer_endpoints {
            let local: SocketAddr = if self.config.local_port == 0 {
                "0.0.0.0:0".parse().unwrap()
            } else {
                format!("0.0.0.0:{}", self.config.local_port)
                    .parse()
                    .unwrap()
            };

            match Self::bind_and_connect_async(local, *peer_addr, self.config.timeout).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    tracing::debug!("Sequential attempt to {} failed: {}", peer_addr, e);
                    continue;
                }
            }
        }

        Err(Error::Network(
            "All sequential connection attempts failed".to_string(),
        ))
    }

    fn get_local_endpoints(&self) -> Result<Vec<SocketAddr>> {
        let mut endpoints = Vec::new();

        if self.config.local_port != 0 {
            endpoints.push(
                format!("0.0.0.0:{}", self.config.local_port)
                    .parse()
                    .unwrap(),
            );
        }

        for iface in get_local_addresses() {
            if self.config.local_port != 0 {
                endpoints.push(SocketAddr::new(iface, self.config.local_port));
            }
        }

        if endpoints.is_empty() {
            endpoints.push("0.0.0.0:0".parse().unwrap());
        }

        Ok(endpoints)
    }
}

pub fn bind_and_connect(local: SocketAddr, remote: SocketAddr) -> Result<TcpStream> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
        .map_err(|e| Error::Network(format!("Failed to create socket: {}", e)))?;

    socket
        .set_reuse_address(true)
        .map_err(|e| Error::Network(format!("Failed to set SO_REUSEADDR: {}", e)))?;

    #[cfg(unix)]
    {
        socket
            .set_reuse_port(true)
            .map_err(|e| Error::Network(format!("Failed to set SO_REUSEPORT: {}", e)))?;
    }

    socket
        .bind(&local.into())
        .map_err(|e| Error::Network(format!("Failed to bind: {}", e)))?;

    socket
        .connect(&remote.into())
        .map_err(|e| Error::Network(format!("Failed to connect: {}", e)))?;

    Ok(socket.into())
}

fn get_local_addresses() -> Vec<std::net::IpAddr> {
    let mut addresses = Vec::new();

    match if_addrs::get_if_addrs() {
        Ok(ifaces) => {
            for iface in ifaces {
                if !iface.is_loopback() {
                    addresses.push(iface.addr.ip());
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to get network interfaces: {}", e);
        }
    }

    addresses
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_config_default() {
        let config = TcpHolePunchConfig::default();
        assert_eq!(config.local_port, 0);
        assert!(config.remote_endpoints.is_empty());
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.retry_count, 3);
    }

    #[test]
    fn test_config_clone() {
        let config = TcpHolePunchConfig {
            local_port: 8080,
            remote_endpoints: vec!["127.0.0.1:8081".parse().unwrap()],
            timeout: Duration::from_secs(5),
            retry_count: 5,
        };
        let cloned = config.clone();
        assert_eq!(config.local_port, cloned.local_port);
        assert_eq!(config.remote_endpoints.len(), cloned.remote_endpoints.len());
    }

    #[test]
    fn test_punch_method_equality() {
        assert_eq!(
            TcpPunchMethod::SimultaneousOpen,
            TcpPunchMethod::SimultaneousOpen
        );
        assert_ne!(TcpPunchMethod::SimultaneousOpen, TcpPunchMethod::Sequential);
    }

    struct MockSignalingChannel {
        ready_sent: Arc<Mutex<bool>>,
        peer_endpoints: Vec<SocketAddr>,
    }

    impl MockSignalingChannel {
        fn new(peer_endpoints: Vec<SocketAddr>) -> Self {
            Self {
                ready_sent: Arc::new(Mutex::new(false)),
                peer_endpoints,
            }
        }
    }

    #[async_trait]
    impl SignalingChannel for MockSignalingChannel {
        async fn send_ready(&self, _endpoints: &[SocketAddr]) -> Result<()> {
            *self.ready_sent.lock().await = true;
            Ok(())
        }

        async fn wait_for_peer_ready(&self, _timeout: Duration) -> Result<Vec<SocketAddr>> {
            Ok(self.peer_endpoints.clone())
        }
    }

    #[tokio::test]
    async fn test_hole_puncher_no_endpoints() {
        let config = TcpHolePunchConfig {
            local_port: 0,
            remote_endpoints: vec![],
            timeout: Duration::from_millis(100),
            retry_count: 1,
        };
        let puncher = TcpHolePuncher::new(config);
        let signaling = MockSignalingChannel::new(vec![]);

        let result = puncher.punch(&signaling).await.unwrap();
        assert!(!result.success);
        assert!(result.connected_addr.is_none());
    }

    #[tokio::test]
    async fn test_hole_puncher_result_failure() {
        let result = TcpHolePunchResult {
            success: false,
            connected_addr: None,
            latency_ms: 100,
            method: TcpPunchMethod::Direct,
        };
        assert!(!result.success);
        assert!(result.connected_addr.is_none());
    }

    #[test]
    fn test_hole_punch_result_success() {
        let result = TcpHolePunchResult {
            success: true,
            connected_addr: Some("127.0.0.1:8080".parse().unwrap()),
            latency_ms: 50,
            method: TcpPunchMethod::SimultaneousOpen,
        };
        assert!(result.success);
        assert!(result.connected_addr.is_some());
        assert_eq!(result.method, TcpPunchMethod::SimultaneousOpen);
    }

    #[tokio::test]
    async fn test_simultaneous_open_no_endpoints() {
        let config = TcpHolePunchConfig::default();
        let puncher = TcpHolePuncher::new(config);

        let result = puncher.attempt_simultaneous_open(&[]).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_get_local_endpoints() {
        let config = TcpHolePunchConfig {
            local_port: 8080,
            remote_endpoints: vec![],
            timeout: Duration::from_secs(5),
            retry_count: 3,
        };
        let puncher = TcpHolePuncher::new(config);

        let endpoints = puncher.get_local_endpoints().unwrap();
        assert!(!endpoints.is_empty());
    }

    #[test]
    fn test_signaling_channel_trait() {
        let signaling = MockSignalingChannel::new(vec!["127.0.0.1:8080".parse().unwrap()]);
        let _trait_obj: &dyn SignalingChannel = &signaling;
        assert!(Arc::strong_count(&signaling.ready_sent) == 1);
    }

    #[tokio::test]
    async fn test_signaling_send_ready() {
        let signaling = MockSignalingChannel::new(vec![]);
        let endpoints: Vec<SocketAddr> = vec!["127.0.0.1:8080".parse().unwrap()];

        signaling.send_ready(&endpoints).await.unwrap();

        let ready = *signaling.ready_sent.lock().await;
        assert!(ready);
    }

    #[tokio::test]
    async fn test_signaling_wait_for_peer() {
        let peer_endpoints: Vec<SocketAddr> = vec!["127.0.0.1:9090".parse().unwrap()];
        let signaling = MockSignalingChannel::new(peer_endpoints.clone());

        let result = signaling
            .wait_for_peer_ready(Duration::from_secs(1))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], peer_endpoints[0]);
    }

    #[test]
    fn test_result_debug() {
        let result = TcpHolePunchResult {
            success: true,
            connected_addr: Some("127.0.0.1:8080".parse().unwrap()),
            latency_ms: 50,
            method: TcpPunchMethod::SimultaneousOpen,
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("success: true"));
    }

    #[test]
    fn test_config_debug() {
        let config = TcpHolePunchConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("local_port"));
    }
}
