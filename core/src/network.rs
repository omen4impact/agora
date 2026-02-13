use futures::StreamExt;
use libp2p::{
    kad::{
        Behaviour as Kademlia, 
        Event as KademliaEvent, 
        RecordKey, 
        store::MemoryStore,
        QueryResult,
        GetProvidersOk,
    },
    swarm::{Swarm, SwarmEvent},
    PeerId, Multiaddr,
    noise, tcp, yamux, dns, 
    identify,
    ping,
    dcutr,
    autonat,
    Transport,
};
use libp2p_swarm_derive::NetworkBehaviour;
use std::collections::HashSet;
use std::time::Duration;
use crate::error::{Error, AgoraResult};
use crate::nat::{NatTraversal, NatType, StunConfig};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "AgoraBehaviourEvent")]
pub struct AgoraBehaviour {
    kademlia: Kademlia<MemoryStore>,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    autonat: autonat::Behaviour,
    dcutr: dcutr::Behaviour,
}

#[derive(Debug)]
pub enum AgoraBehaviourEvent {
    Kademlia(KademliaEvent),
    Identify(identify::Event),
    Ping(ping::Event),
    Autonat(autonat::Event),
    Dcutr(dcutr::Event),
}

impl From<KademliaEvent> for AgoraBehaviourEvent {
    fn from(event: KademliaEvent) -> Self {
        AgoraBehaviourEvent::Kademlia(event)
    }
}

impl From<identify::Event> for AgoraBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        AgoraBehaviourEvent::Identify(event)
    }
}

impl From<ping::Event> for AgoraBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        AgoraBehaviourEvent::Ping(event)
    }
}

impl From<autonat::Event> for AgoraBehaviourEvent {
    fn from(event: autonat::Event) -> Self {
        AgoraBehaviourEvent::Autonat(event)
    }
}

impl From<dcutr::Event> for AgoraBehaviourEvent {
    fn from(event: dcutr::Event) -> Self {
        AgoraBehaviourEvent::Dcutr(event)
    }
}

pub struct NetworkNodeConfig {
    pub listen_addr: Option<String>,
    pub stun_servers: Vec<String>,
    pub enable_relay: bool,
    pub bootstrap_peers: Vec<String>,
}

impl Default for NetworkNodeConfig {
    fn default() -> Self {
        Self {
            listen_addr: None,
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
            ],
            enable_relay: true,
            bootstrap_peers: vec![],
        }
    }
}

pub struct NetworkNode {
    swarm: Swarm<AgoraBehaviour>,
    local_peer_id: PeerId,
    known_peers: HashSet<PeerId>,
    nat_traversal: NatTraversal,
    config: NetworkNodeConfig,
    listen_addrs: Vec<Multiaddr>,
}

impl NetworkNode {
    pub async fn new(listen_addr: Option<&str>) -> AgoraResult<Self> {
        let config = NetworkNodeConfig {
            listen_addr: listen_addr.map(|s| s.to_string()),
            ..Default::default()
        };
        Self::with_config(config).await
    }
    
    pub async fn with_config(config: NetworkNodeConfig) -> AgoraResult<Self> {
        let local_keypair = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_keypair.public());
        
        let transport = dns::tokio::Transport::system(
            tcp::tokio::Transport::new(tcp::Config::new().nodelay(true))
        )
        .map_err(|e| Error::Network(format!("DNS transport error: {}", e)))?
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(
            noise::Config::new(&local_keypair)
                .map_err(|e| Error::Network(format!("Noise config error: {}", e)))?
        )
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed();
        
        let store = MemoryStore::new(local_peer_id);
        let kademlia = Kademlia::new(local_peer_id, store);
        
        let identify = identify::Behaviour::new(
            identify::Config::new("agora/0.1.0".to_string(), local_keypair.public())
                .with_agent_version(format!("agora/0.1.0 rust/{}", env!("CARGO_PKG_VERSION")))
        );
        
        let autonat = autonat::Behaviour::new(
            local_peer_id,
            autonat::Config {
                only_global_ips: false,
                ..Default::default()
            },
        );
        
        let dcutr = dcutr::Behaviour::new(local_peer_id);
        
        let behaviour = AgoraBehaviour {
            kademlia,
            identify,
            ping: ping::Behaviour::default(),
            autonat,
            dcutr,
        };
        
        let swarm_config = libp2p::swarm::Config::with_tokio_executor();
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);
        
        let addr = config.listen_addr.as_deref().unwrap_or("/ip4/0.0.0.0/tcp/0");
        swarm
            .listen_on(addr.parse().map_err(|e| Error::Network(format!("Invalid address: {}", e)))?)
            .map_err(|e| Error::Network(format!("Listen error: {}", e)))?;
        
        let stun_config = StunConfig {
            servers: config.stun_servers.clone(),
            ..Default::default()
        };
        
        Ok(Self {
            swarm,
            local_peer_id,
            known_peers: HashSet::new(),
            nat_traversal: NatTraversal::new(Some(stun_config)),
            config,
            listen_addrs: vec![],
        })
    }
    
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }
    
    pub fn peer_id_string(&self) -> String {
        self.local_peer_id.to_string()
    }
    
    pub fn listen_addrs(&self) -> &[Multiaddr] {
        &self.listen_addrs
    }
    
    pub fn add_peer(&mut self, peer_id: PeerId) {
        self.known_peers.insert(peer_id);
    }
    
    pub fn known_peers(&self) -> &HashSet<PeerId> {
        &self.known_peers
    }
    
    pub async fn detect_nat(&mut self) -> AgoraResult<NatType> {
        self.nat_traversal.detect_nat_type().await
    }
    
    pub async fn dial(&mut self, addr: Multiaddr) -> AgoraResult<()> {
        self.swarm
            .dial(addr)
            .map_err(|e| Error::Network(format!("Dial error: {}", e)))?;
        Ok(())
    }
    
    pub fn add_address(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
    }
    
    pub async fn start_providing(&mut self, room_id: &str) -> AgoraResult<()> {
        let key = RecordKey::new(&room_id);
        self.swarm
            .behaviour_mut()
            .kademlia
            .start_providing(key)
            .map_err(|e| Error::Network(format!("Start providing error: {}", e)))?;
        Ok(())
    }
    
    pub fn get_providers(&mut self, room_id: &str) {
        let key = RecordKey::new(&room_id);
        self.swarm.behaviour_mut().kademlia.get_providers(key);
    }
    
    pub fn bootstrap(&mut self) -> AgoraResult<()> {
        self.swarm
            .behaviour_mut()
            .kademlia
            .bootstrap()
            .map_err(|e| Error::Network(format!("Bootstrap error: {:?}", e)))?;
        Ok(())
    }
    
    pub async fn next_event(&mut self) -> Option<NetworkEvent> {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    self.listen_addrs.push(address.clone());
                    tracing::info!("Listening on {}", address);
                    return Some(NetworkEvent::Listening(address));
                }
                SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                    self.known_peers.insert(peer_id);
                    tracing::info!("Connected to {} at {}", peer_id, endpoint.get_remote_address());
                    return Some(NetworkEvent::PeerConnected {
                        peer_id,
                        addr: endpoint.get_remote_address().clone(),
                    });
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    self.known_peers.remove(&peer_id);
                    tracing::info!("Disconnected from {} ({:?})", peer_id, cause);
                    return Some(NetworkEvent::PeerDisconnected { 
                        peer_id, 
                        cause: cause.map(|e| std::io::Error::new(std::io::ErrorKind::ConnectionReset, e.to_string())),
                    });
                }
                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    tracing::warn!("Failed to connect to {:?}: {}", peer_id, error);
                }
                SwarmEvent::Behaviour(event) => {
                    match event {
                        AgoraBehaviourEvent::Kademlia(KademliaEvent::OutboundQueryProgressed { result, .. }) => {
                            match result {
                                QueryResult::GetProviders(Ok(GetProvidersOk::FoundProviders { key, providers, .. })) => {
                                    return Some(NetworkEvent::ProvidersFound {
                                        room_id: String::from_utf8_lossy(key.as_ref()).to_string(),
                                        providers: providers.into_iter().collect(),
                                    });
                                }
                                QueryResult::Bootstrap(Ok(_)) => {
                                    tracing::info!("Bootstrap complete");
                                    return Some(NetworkEvent::BootstrapComplete);
                                }
                                _ => {}
                            }
                        }
                        AgoraBehaviourEvent::Identify(identify::Event::Received { peer_id, info }) => {
                            tracing::debug!("Identified {} with {} addresses", peer_id, info.listen_addrs.len());
                            for addr in &info.listen_addrs {
                                self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                            }
                            return Some(NetworkEvent::PeerIdentified {
                                peer_id,
                                listen_addrs: info.listen_addrs,
                            });
                        }
                        AgoraBehaviourEvent::Ping(ping::Event { peer, .. }) => {
                            return Some(NetworkEvent::PingResult { peer_id: peer, result: Ok(()) });
                        }
                        AgoraBehaviourEvent::Autonat(autonat::Event::StatusChanged { old, new }) => {
                            tracing::info!("NAT status changed: {:?} -> {:?}", old, new);
                            let is_public = matches!(new, autonat::NatStatus::Public(_));
                            return Some(NetworkEvent::NatStatusChanged { 
                                is_public,
                            });
                        }
                        AgoraBehaviourEvent::Dcutr(event) => {
                            tracing::debug!("DCUtR event: {:?}", event);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
pub enum NetworkEvent {
    Listening(Multiaddr),
    PeerConnected {
        peer_id: PeerId,
        addr: Multiaddr,
    },
    PeerDisconnected {
        peer_id: PeerId,
        cause: Option<std::io::Error>,
    },
    PeerIdentified {
        peer_id: PeerId,
        listen_addrs: Vec<Multiaddr>,
    },
    ProvidersFound {
        room_id: String,
        providers: Vec<PeerId>,
    },
    PingResult {
        peer_id: PeerId,
        result: AgoraResult<()>,
    },
    NatStatusChanged {
        is_public: bool,
    },
    BootstrapComplete,
}

pub fn parse_peer_id(s: &str) -> AgoraResult<PeerId> {
    s.parse()
        .map_err(|e| Error::Network(format!("Invalid peer ID '{}': {}", s, e)))
}

pub fn parse_multiaddr(s: &str) -> AgoraResult<Multiaddr> {
    s.parse()
        .map_err(|e| Error::Network(format!("Invalid multiaddr '{}': {}", s, e)))
}
