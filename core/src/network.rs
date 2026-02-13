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
    Transport,
};
use libp2p_swarm_derive::NetworkBehaviour;
use std::collections::HashSet;
use crate::error::{Error, AgoraResult};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "AgoraBehaviourEvent")]
pub struct AgoraBehaviour {
    kademlia: Kademlia<MemoryStore>,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
}

#[derive(Debug)]
pub enum AgoraBehaviourEvent {
    Kademlia(KademliaEvent),
    Identify(identify::Event),
    Ping(ping::Event),
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

pub struct NetworkNode {
    swarm: Swarm<AgoraBehaviour>,
    local_peer_id: PeerId,
    known_peers: HashSet<PeerId>,
}

impl NetworkNode {
    pub async fn new(listen_addr: Option<&str>) -> AgoraResult<Self> {
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
        .timeout(std::time::Duration::from_secs(20))
        .boxed();
        
        let store = MemoryStore::new(local_peer_id);
        let kademlia = Kademlia::new(local_peer_id, store);
        
        let identify = identify::Behaviour::new(
            identify::Config::new("agora/0.1.0".to_string(), local_keypair.public())
        );
        
        let behaviour = AgoraBehaviour {
            kademlia,
            identify,
            ping: ping::Behaviour::default(),
        };
        
        let swarm_config = libp2p::swarm::Config::with_tokio_executor();
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);
        
        let addr = listen_addr.unwrap_or("/ip4/0.0.0.0/tcp/0");
        swarm
            .listen_on(addr.parse().map_err(|e| Error::Network(format!("Invalid address: {}", e)))?)
            .map_err(|e| Error::Network(format!("Listen error: {}", e)))?;
        
        Ok(Self {
            swarm,
            local_peer_id,
            known_peers: HashSet::new(),
        })
    }
    
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }
    
    pub fn peer_id_string(&self) -> String {
        self.local_peer_id.to_string()
    }
    
    pub fn add_peer(&mut self, peer_id: PeerId) {
        self.known_peers.insert(peer_id);
    }
    
    pub fn known_peers(&self) -> &HashSet<PeerId> {
        &self.known_peers
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
    
    pub async fn next_event(&mut self) -> Option<NetworkEvent> {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    return Some(NetworkEvent::Listening(address));
                }
                SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                    self.known_peers.insert(peer_id);
                    return Some(NetworkEvent::PeerConnected {
                        peer_id,
                        addr: endpoint.get_remote_address().clone(),
                    });
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    self.known_peers.remove(&peer_id);
                    return Some(NetworkEvent::PeerDisconnected { 
                        peer_id, 
                        cause: cause.map(|e| std::io::Error::new(std::io::ErrorKind::ConnectionReset, e.to_string())),
                    });
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
                                _ => {}
                            }
                        }
                        AgoraBehaviourEvent::Identify(identify::Event::Received { peer_id, info }) => {
                            return Some(NetworkEvent::PeerIdentified {
                                peer_id,
                                listen_addrs: info.listen_addrs,
                            });
                        }
                        AgoraBehaviourEvent::Ping(ping::Event { peer, .. }) => {
                            return Some(NetworkEvent::PingResult { peer_id: peer, result: Ok(()) });
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
}

pub fn parse_peer_id(s: &str) -> AgoraResult<PeerId> {
    s.parse()
        .map_err(|e| Error::Network(format!("Invalid peer ID '{}': {}", s, e)))
}

pub fn parse_multiaddr(s: &str) -> AgoraResult<Multiaddr> {
    s.parse()
        .map_err(|e| Error::Network(format!("Invalid multiaddr '{}': {}", s, e)))
}
