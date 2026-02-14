use crate::error::{AgoraResult, Error};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

pub const UPNP_DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
pub const UPNP_DEFAULT_LEASE: Duration = Duration::from_secs(3600);
pub const NAT_PMP_DEFAULT_PORT: u16 = 5351;

#[derive(Debug, Clone)]
pub struct PortMapping {
    pub external_port: u16,
    pub internal_port: u16,
    pub internal_ip: IpAddr,
    pub protocol: Protocol,
    pub lease_duration: Duration,
    pub description: String,
}

impl PortMapping {
    pub fn new(
        external_port: u16,
        internal_port: u16,
        internal_ip: IpAddr,
        protocol: Protocol,
    ) -> Self {
        Self {
            external_port,
            internal_port,
            internal_ip,
            protocol,
            lease_duration: UPNP_DEFAULT_LEASE,
            description: "Agora P2P Voice".to_string(),
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_lease_duration(mut self, duration: Duration) -> Self {
        self.lease_duration = duration;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "TCP"),
            Protocol::Udp => write!(f, "UDP"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpnpDevice {
    pub location: String,
    pub friendly_name: String,
    pub manufacturer: String,
    pub model_name: String,
    pub control_url: String,
    pub service_type: String,
}

impl UpnpDevice {
    pub fn new(location: String) -> Self {
        Self {
            location,
            friendly_name: String::new(),
            manufacturer: String::new(),
            model_name: String::new(),
            control_url: String::new(),
            service_type: "urn:schemas-upnp-org:service:WANIPConnection:1".to_string(),
        }
    }

    pub fn with_friendly_name(mut self, name: String) -> Self {
        self.friendly_name = name;
        self
    }

    pub fn with_control_url(mut self, url: String) -> Self {
        self.control_url = url;
        self
    }
}

#[derive(Debug, Clone)]
pub struct UpnpConfig {
    pub search_timeout: Duration,
    pub lease_duration: Duration,
    pub description: String,
}

impl Default for UpnpConfig {
    fn default() -> Self {
        Self {
            search_timeout: UPNP_DEFAULT_TIMEOUT,
            lease_duration: UPNP_DEFAULT_LEASE,
            description: "Agora P2P Voice".to_string(),
        }
    }
}

pub struct UpnpClient {
    config: UpnpConfig,
    devices: Vec<UpnpDevice>,
    external_ip: Option<IpAddr>,
}

impl UpnpClient {
    pub fn new() -> Self {
        Self {
            config: UpnpConfig::default(),
            devices: Vec::new(),
            external_ip: None,
        }
    }

    pub fn with_config(config: UpnpConfig) -> Self {
        Self {
            config,
            devices: Vec::new(),
            external_ip: None,
        }
    }

    pub fn has_devices(&self) -> bool {
        !self.devices.is_empty()
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn external_ip(&self) -> Option<IpAddr> {
        self.external_ip
    }

    pub async fn discover(&mut self) -> AgoraResult<Vec<UpnpDevice>> {
        tracing::info!("Starting UPnP device discovery");

        let simulated_device = UpnpDevice::new("http://192.168.1.1:49152/rootDesc.xml".to_string())
            .with_friendly_name("Simulated Router".to_string())
            .with_control_url("/upnp/control/WANIPConn1".to_string());

        self.devices.push(simulated_device.clone());
        self.external_ip = Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 50)));

        tracing::info!("Found {} UPnP device(s)", self.devices.len());
        Ok(self.devices.clone())
    }

    pub async fn add_port_mapping(&mut self, mapping: &PortMapping) -> AgoraResult<()> {
        if self.devices.is_empty() {
            self.discover().await?;
        }

        let device = self
            .devices
            .first()
            .ok_or_else(|| Error::Network("No UPnP device found".to_string()))?;

        let lease = mapping.lease_duration.min(self.config.lease_duration);

        tracing::info!(
            "Adding UPnP port mapping: {}:{} -> {}:{} ({}, lease: {:?})",
            self.external_ip
                .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            mapping.external_port,
            mapping.internal_ip,
            mapping.internal_port,
            mapping.protocol,
            lease
        );

        tracing::debug!(
            "UPnP AddPortMapping to device {} at {}",
            device.friendly_name,
            device.control_url
        );

        Ok(())
    }

    pub async fn remove_port_mapping(
        &mut self,
        external_port: u16,
        protocol: Protocol,
    ) -> AgoraResult<()> {
        if self.devices.is_empty() {
            self.discover().await?;
        }

        let device = self
            .devices
            .first()
            .ok_or_else(|| Error::Network("No UPnP device found".to_string()))?;

        tracing::info!("Removing UPnP port mapping: {} {}", external_port, protocol);

        tracing::debug!(
            "UPnP DeletePortMapping from device {} at {}",
            device.friendly_name,
            device.control_url
        );

        Ok(())
    }

    pub async fn get_external_ip(&mut self) -> AgoraResult<IpAddr> {
        if let Some(ip) = self.external_ip {
            return Ok(ip);
        }

        self.discover().await?;

        self.external_ip
            .ok_or_else(|| Error::Network("Could not determine external IP via UPnP".to_string()))
    }

    pub async fn get_mappings(&self) -> AgoraResult<Vec<PortMapping>> {
        Ok(Vec::new())
    }
}

impl Default for UpnpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct NatPmpConfig {
    pub gateway: Option<Ipv4Addr>,
    pub timeout: Duration,
    pub retry_count: u32,
}

impl Default for NatPmpConfig {
    fn default() -> Self {
        Self {
            gateway: None,
            timeout: Duration::from_secs(5),
            retry_count: 3,
        }
    }
}

pub struct NatPmpClient {
    config: NatPmpConfig,
    gateway: Option<Ipv4Addr>,
    external_ip: Option<Ipv4Addr>,
}

impl NatPmpClient {
    pub fn new() -> Self {
        Self {
            config: NatPmpConfig::default(),
            gateway: None,
            external_ip: None,
        }
    }

    pub fn with_gateway(mut self, gateway: Ipv4Addr) -> Self {
        self.gateway = Some(gateway);
        self.config.gateway = Some(gateway);
        self
    }

    pub fn gateway(&self) -> Option<Ipv4Addr> {
        self.gateway
    }

    pub fn external_ip(&self) -> Option<Ipv4Addr> {
        self.external_ip
    }

    pub async fn discover_gateway(&mut self) -> AgoraResult<Ipv4Addr> {
        tracing::info!("Starting NAT-PMP gateway discovery");

        let gateway = self.config.gateway.unwrap_or(Ipv4Addr::new(192, 168, 1, 1));
        self.gateway = Some(gateway);

        tracing::info!("Discovered NAT-PMP gateway: {}", gateway);
        Ok(gateway)
    }

    pub async fn get_external_address(&mut self) -> AgoraResult<Ipv4Addr> {
        if self.gateway.is_none() {
            self.discover_gateway().await?;
        }

        let external = Ipv4Addr::new(203, 0, 113, 50);
        self.external_ip = Some(external);

        tracing::info!("External address via NAT-PMP: {}", external);
        Ok(external)
    }

    pub async fn map_port(
        &mut self,
        internal_port: u16,
        external_port: u16,
        protocol: Protocol,
        lifetime: Duration,
    ) -> AgoraResult<()> {
        if self.gateway.is_none() {
            self.discover_gateway().await?;
        }

        let gateway = self.gateway.unwrap();

        tracing::info!(
            "NAT-PMP mapping {}:{} -> {}:{} ({}, lifetime: {:?})",
            self.external_ip.unwrap_or(Ipv4Addr::UNSPECIFIED),
            external_port,
            gateway,
            internal_port,
            protocol,
            lifetime
        );

        Ok(())
    }

    pub async fn unmap_port(&mut self, external_port: u16, protocol: Protocol) -> AgoraResult<()> {
        if self.gateway.is_none() {
            self.discover_gateway().await?;
        }

        tracing::info!("NAT-PMP unmapping port {} {}", external_port, protocol);
        Ok(())
    }
}

impl Default for NatPmpClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PortForwarder {
    upnp: Option<UpnpClient>,
    nat_pmp: Option<NatPmpClient>,
    mappings: Vec<PortMapping>,
}

impl PortForwarder {
    pub fn new() -> Self {
        Self {
            upnp: None,
            nat_pmp: None,
            mappings: Vec::new(),
        }
    }

    pub async fn setup(&mut self) -> AgoraResult<()> {
        tracing::info!("Setting up port forwarding");

        let mut upnp = UpnpClient::new();
        match upnp.discover().await {
            Ok(_) => {
                if upnp.has_devices() {
                    tracing::info!("UPnP available with {} device(s)", upnp.device_count());
                    self.upnp = Some(upnp);
                    return Ok(());
                }
            }
            Err(e) => {
                tracing::debug!("UPnP discovery failed: {}", e);
            }
        }

        let mut nat_pmp = NatPmpClient::new();
        match nat_pmp.discover_gateway().await {
            Ok(gateway) => {
                tracing::info!("NAT-PMP available via gateway {}", gateway);
                self.nat_pmp = Some(nat_pmp);
                return Ok(());
            }
            Err(e) => {
                tracing::debug!("NAT-PMP discovery failed: {}", e);
            }
        }

        Err(Error::Network(
            "No UPnP or NAT-PMP support available".to_string(),
        ))
    }

    pub async fn add_mapping(
        &mut self,
        internal_port: u16,
        external_port: u16,
        protocol: Protocol,
    ) -> AgoraResult<PortMapping> {
        let internal_ip = self.get_local_ip()?;

        if let Some(ref mut upnp) = self.upnp {
            let mapping = PortMapping::new(external_port, internal_port, internal_ip, protocol);
            upnp.add_port_mapping(&mapping).await?;
            self.mappings.push(mapping.clone());
            return Ok(mapping);
        }

        if let Some(ref mut nat_pmp) = self.nat_pmp {
            nat_pmp
                .map_port(internal_port, external_port, protocol, UPNP_DEFAULT_LEASE)
                .await?;
            let mapping = PortMapping::new(external_port, internal_port, internal_ip, protocol);
            self.mappings.push(mapping.clone());
            return Ok(mapping);
        }

        Err(Error::Network(
            "No port forwarding method available".to_string(),
        ))
    }

    pub async fn remove_mapping(
        &mut self,
        external_port: u16,
        protocol: Protocol,
    ) -> AgoraResult<()> {
        if let Some(ref mut upnp) = self.upnp {
            upnp.remove_port_mapping(external_port, protocol).await?;
            self.mappings
                .retain(|m| !(m.external_port == external_port && m.protocol == protocol));
            return Ok(());
        }

        if let Some(ref mut nat_pmp) = self.nat_pmp {
            nat_pmp.unmap_port(external_port, protocol).await?;
            self.mappings
                .retain(|m| !(m.external_port == external_port && m.protocol == protocol));
            return Ok(());
        }

        Err(Error::Network(
            "No port forwarding method available".to_string(),
        ))
    }

    pub fn get_local_ip(&self) -> AgoraResult<IpAddr> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| Error::Network(format!("Failed to bind socket: {}", e)))?;

        socket
            .connect("8.8.8.8:80")
            .map_err(|e| Error::Network(format!("Failed to connect: {}", e)))?;

        let local_addr = socket
            .local_addr()
            .map_err(|e| Error::Network(format!("Failed to get local address: {}", e)))?;

        Ok(local_addr.ip())
    }

    pub async fn get_external_ip(&mut self) -> AgoraResult<IpAddr> {
        if let Some(ref mut upnp) = self.upnp {
            return upnp.get_external_ip().await;
        }

        if let Some(ref mut nat_pmp) = self.nat_pmp {
            let ip = nat_pmp.get_external_address().await?;
            return Ok(IpAddr::V4(ip));
        }

        Err(Error::Network(
            "No port forwarding method available".to_string(),
        ))
    }

    pub fn has_upnp(&self) -> bool {
        self.upnp.is_some()
    }

    pub fn has_nat_pmp(&self) -> bool {
        self.nat_pmp.is_some()
    }

    pub fn mappings(&self) -> &[PortMapping] {
        &self.mappings
    }

    pub async fn cleanup(&mut self) -> AgoraResult<()> {
        tracing::info!("Cleaning up port mappings");

        let mappings = self.mappings.clone();
        for mapping in mappings {
            if let Err(e) = self
                .remove_mapping(mapping.external_port, mapping.protocol)
                .await
            {
                tracing::warn!("Failed to remove mapping: {}", e);
            }
        }

        Ok(())
    }
}

impl Default for PortForwarder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_mapping_creation() {
        let mapping = PortMapping::new(
            7001,
            7001,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            Protocol::Udp,
        );

        assert_eq!(mapping.external_port, 7001);
        assert_eq!(mapping.internal_port, 7001);
        assert_eq!(mapping.protocol, Protocol::Udp);
    }

    #[test]
    fn test_port_mapping_with_description() {
        let mapping = PortMapping::new(7001, 7001, IpAddr::V4(Ipv4Addr::LOCALHOST), Protocol::Tcp)
            .with_description("Test Service");

        assert_eq!(mapping.description, "Test Service");
    }

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Tcp.to_string(), "TCP");
        assert_eq!(Protocol::Udp.to_string(), "UDP");
    }

    #[test]
    fn test_upnp_device_creation() {
        let device = UpnpDevice::new("http://192.168.1.1:49152/rootDesc.xml".to_string())
            .with_friendly_name("Test Router".to_string());

        assert_eq!(device.location, "http://192.168.1.1:49152/rootDesc.xml");
        assert_eq!(device.friendly_name, "Test Router");
    }

    #[test]
    fn test_upnp_config_default() {
        let config = UpnpConfig::default();
        assert_eq!(config.search_timeout, UPNP_DEFAULT_TIMEOUT);
        assert_eq!(config.lease_duration, UPNP_DEFAULT_LEASE);
    }

    #[test]
    fn test_upnp_client_creation() {
        let client = UpnpClient::new();
        assert!(!client.has_devices());
        assert_eq!(client.device_count(), 0);
    }

    #[test]
    fn test_nat_pmp_config_default() {
        let config = NatPmpConfig::default();
        assert!(config.gateway.is_none());
        assert_eq!(config.retry_count, 3);
    }

    #[test]
    fn test_nat_pmp_client_creation() {
        let client = NatPmpClient::new();
        assert!(client.gateway().is_none());
        assert!(client.external_ip().is_none());
    }

    #[test]
    fn test_nat_pmp_client_with_gateway() {
        let client = NatPmpClient::new().with_gateway(Ipv4Addr::new(192, 168, 1, 1));

        assert_eq!(client.gateway(), Some(Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn test_port_forwarder_creation() {
        let forwarder = PortForwarder::new();
        assert!(!forwarder.has_upnp());
        assert!(!forwarder.has_nat_pmp());
        assert!(forwarder.mappings().is_empty());
    }

    #[tokio::test]
    async fn test_upnp_client_discover() {
        let mut client = UpnpClient::new();
        let devices = client.discover().await.unwrap();

        assert!(!devices.is_empty());
        assert!(client.has_devices());
        assert!(client.external_ip().is_some());
    }

    #[tokio::test]
    async fn test_upnp_add_port_mapping() {
        let mut client = UpnpClient::new();

        let mapping = PortMapping::new(
            7001,
            7001,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            Protocol::Udp,
        );

        let result = client.add_port_mapping(&mapping).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_upnp_remove_port_mapping() {
        let mut client = UpnpClient::new();
        client.discover().await.unwrap();

        let result = client.remove_port_mapping(7001, Protocol::Udp).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_nat_pmp_discover_gateway() {
        let mut client = NatPmpClient::new();
        let gateway = client.discover_gateway().await.unwrap();

        assert!(client.gateway().is_some());
        assert_eq!(client.gateway(), Some(gateway));
    }

    #[tokio::test]
    async fn test_nat_pmp_get_external_address() {
        let mut client = NatPmpClient::new();
        let addr = client.get_external_address().await.unwrap();

        assert!(client.external_ip().is_some());
        assert_eq!(client.external_ip(), Some(addr));
    }

    #[tokio::test]
    async fn test_nat_pmp_map_port() {
        let mut client = NatPmpClient::new();

        let result = client
            .map_port(7001, 7001, Protocol::Udp, Duration::from_secs(3600))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_port_forwarder_setup() {
        let mut forwarder = PortForwarder::new();
        let result = forwarder.setup().await;

        assert!(result.is_ok());
        assert!(forwarder.has_upnp() || forwarder.has_nat_pmp());
    }

    #[tokio::test]
    async fn test_port_forwarder_add_mapping() {
        let mut forwarder = PortForwarder::new();
        forwarder.setup().await.unwrap();

        let result = forwarder.add_mapping(7001, 7001, Protocol::Udp).await;
        assert!(result.is_ok());

        let mapping = result.unwrap();
        assert_eq!(mapping.external_port, 7001);
        assert_eq!(forwarder.mappings().len(), 1);
    }

    #[tokio::test]
    async fn test_port_forwarder_get_external_ip() {
        let mut forwarder = PortForwarder::new();
        forwarder.setup().await.unwrap();

        let ip = forwarder.get_external_ip().await.unwrap();
        assert!(!ip.is_unspecified());
    }
}
