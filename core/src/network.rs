use crate::error::{AgoraResult, Error};
use crate::ice::{Candidate, ConnectionState as IceConnectionState, IceAgent, IceConfig};
use crate::nat::{NatTraversal, NatType, StunConfig};
use crate::protocol::{
    AudioPacket, ControlMessage, ControlMessageType, MAX_FRAME_SIZE, PROTOCOL_CONTROL,
    PROTOCOL_NAME,
};
use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, StreamExt};
use libp2p::{
    autonat, dcutr, dns, identify,
    kad::{
        store::MemoryStore, Behaviour as Kademlia, Event as KademliaEvent, GetProvidersOk,
        QueryResult, RecordKey,
    },
    noise, ping,
    request_response::{self, Behaviour as RequestResponse, Codec, ProtocolSupport},
    swarm::{Swarm, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Transport,
};
use libp2p_swarm_derive::NetworkBehaviour;
use std::collections::{HashMap, HashSet};
use std::io;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "AgoraBehaviourEvent")]
pub struct AgoraBehaviour {
    kademlia: Kademlia<MemoryStore>,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    autonat: autonat::Behaviour,
    dcutr: dcutr::Behaviour,
    audio_stream: RequestResponse<AudioCodec>,
    control: RequestResponse<ControlCodec>,
}

#[derive(Debug)]
pub enum AgoraBehaviourEvent {
    Kademlia(KademliaEvent),
    Identify(identify::Event),
    Ping(ping::Event),
    Autonat(autonat::Event),
    Dcutr(dcutr::Event),
    AudioStream(request_response::Event<AudioPacket, AudioPacket>),
    Control(request_response::Event<ControlMessage, ControlMessage>),
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

impl From<request_response::Event<AudioPacket, AudioPacket>> for AgoraBehaviourEvent {
    fn from(event: request_response::Event<AudioPacket, AudioPacket>) -> Self {
        AgoraBehaviourEvent::AudioStream(event)
    }
}

impl From<request_response::Event<ControlMessage, ControlMessage>> for AgoraBehaviourEvent {
    fn from(event: request_response::Event<ControlMessage, ControlMessage>) -> Self {
        AgoraBehaviourEvent::Control(event)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AudioCodec;

#[async_trait]
impl Codec for AudioCodec {
    type Protocol = &'static str;
    type Request = AudioPacket;
    type Response = AudioPacket;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_bytes = [0u8; 4];
        io.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Frame too large",
            ));
        }

        let mut data = vec![0u8; len];
        io.read_exact(&mut data).await?;

        AudioPacket::decode(&data)
    }

    async fn read_response<T>(
        &mut self,
        proto: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read_request(proto, io).await
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = req.encode()?;
        write_frame(io, &data).await
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = res.encode()?;
        write_frame(io, &data).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct ControlCodec;

#[async_trait]
impl Codec for ControlCodec {
    type Protocol = &'static str;
    type Request = ControlMessage;
    type Response = ControlMessage;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_bytes = [0u8; 4];
        io.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > 65536 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Message too large",
            ));
        }

        let mut data = vec![0u8; len];
        io.read_exact(&mut data).await?;

        ControlMessage::decode(&data)
    }

    async fn read_response<T>(
        &mut self,
        proto: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read_request(proto, io).await
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = req.encode()?;
        write_frame(io, &data).await
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = res.encode()?;
        write_frame(io, &data).await
    }
}

async fn write_frame<T: AsyncWrite + Unpin + Send>(io: &mut T, data: &[u8]) -> io::Result<()> {
    io.write_all(&(data.len() as u32).to_be_bytes()).await?;
    io.write_all(data).await?;
    io.flush().await?;
    Ok(())
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
            stun_servers: vec!["stun:stun.l.google.com:19302".to_string()],
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
    ice_agent: Option<IceAgent>,
    listen_addrs: Vec<Multiaddr>,
    room_peers: HashMap<String, HashSet<PeerId>>,
    peer_names: HashMap<PeerId, String>,
    event_tx: broadcast::Sender<NetworkEvent>,
    command_tx: mpsc::Sender<NetworkCommand>,
    command_rx: Option<mpsc::Receiver<NetworkCommand>>,
}

#[derive(Debug, Clone)]
pub enum NetworkCommand {
    SendAudio {
        peer_id: PeerId,
        packet: AudioPacket,
    },
    BroadcastAudio {
        room_id: String,
        packet: AudioPacket,
    },
    SendControl {
        peer_id: PeerId,
        message: ControlMessage,
    },
    JoinRoom {
        room_id: String,
    },
    LeaveRoom {
        room_id: String,
    },
    ConnectToPeer {
        addr: Multiaddr,
    },
    Stop,
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Listening(Multiaddr),
    PeerConnected {
        peer_id: PeerId,
        addr: Multiaddr,
    },
    PeerDisconnected {
        peer_id: PeerId,
    },
    PeerIdentified {
        peer_id: PeerId,
        listen_addrs: Vec<Multiaddr>,
    },
    AudioReceived {
        peer_id: PeerId,
        packet: AudioPacket,
    },
    ControlReceived {
        peer_id: PeerId,
        message: ControlMessage,
    },
    ProvidersFound {
        room_id: String,
        providers: Vec<PeerId>,
    },
    RoomJoined {
        room_id: String,
        peer_id: PeerId,
    },
    RoomLeft {
        room_id: String,
        peer_id: PeerId,
    },
    NatStatusChanged {
        is_public: bool,
    },
    BootstrapComplete,
    IceCandidatesGathered {
        candidates: Vec<String>,
    },
    IceConnectionStateChanged {
        state: String,
    },
    Error(String),
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

        let transport = dns::tokio::Transport::system(tcp::tokio::Transport::new(
            tcp::Config::new().nodelay(true),
        ))
        .map_err(|e| Error::Network(format!("DNS transport error: {}", e)))?
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(
            noise::Config::new(&local_keypair)
                .map_err(|e| Error::Network(format!("Noise config error: {}", e)))?,
        )
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed();

        let store = MemoryStore::new(local_peer_id);
        let kademlia = Kademlia::new(local_peer_id, store);

        let identify = identify::Behaviour::new(
            identify::Config::new("agora/0.1.0".to_string(), local_keypair.public())
                .with_agent_version(format!("agora/0.1.0 rust/{}", env!("CARGO_PKG_VERSION"))),
        );

        let autonat = autonat::Behaviour::new(
            local_peer_id,
            autonat::Config {
                only_global_ips: false,
                ..Default::default()
            },
        );

        let dcutr = dcutr::Behaviour::new(local_peer_id);

        let audio_protocols = std::iter::once((PROTOCOL_NAME, ProtocolSupport::Full));
        let audio_stream = RequestResponse::new(
            audio_protocols,
            request_response::Config::default().with_request_timeout(Duration::from_secs(5)),
        );

        let control_protocols = std::iter::once((PROTOCOL_CONTROL, ProtocolSupport::Full));
        let control = RequestResponse::new(
            control_protocols,
            request_response::Config::default().with_request_timeout(Duration::from_secs(10)),
        );

        let behaviour = AgoraBehaviour {
            kademlia,
            identify,
            ping: ping::Behaviour::default(),
            autonat,
            dcutr,
            audio_stream,
            control,
        };

        let swarm_config = libp2p::swarm::Config::with_tokio_executor();
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);

        let addr = config
            .listen_addr
            .as_deref()
            .unwrap_or("/ip4/0.0.0.0/tcp/0");
        swarm
            .listen_on(
                addr.parse()
                    .map_err(|e| Error::Network(format!("Invalid address: {}", e)))?,
            )
            .map_err(|e| Error::Network(format!("Listen error: {}", e)))?;

        let stun_config = StunConfig {
            servers: config.stun_servers.clone(),
            ..Default::default()
        };

        let (event_tx, _) = broadcast::channel(256);
        let (command_tx, command_rx) = mpsc::channel(256);

        Ok(Self {
            swarm,
            local_peer_id,
            known_peers: HashSet::new(),
            nat_traversal: NatTraversal::new(Some(stun_config)),
            ice_agent: None,
            listen_addrs: vec![],
            room_peers: HashMap::new(),
            peer_names: HashMap::new(),
            event_tx,
            command_tx,
            command_rx: Some(command_rx),
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

    pub fn known_peers(&self) -> &HashSet<PeerId> {
        &self.known_peers
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<NetworkEvent> {
        self.event_tx.subscribe()
    }

    pub fn command_sender(&self) -> mpsc::Sender<NetworkCommand> {
        self.command_tx.clone()
    }

    pub async fn detect_nat(&mut self) -> AgoraResult<NatType> {
        self.nat_traversal.detect_nat_type().await
    }

    pub async fn gather_ice_candidates(&mut self) -> AgoraResult<Vec<String>> {
        let ice_config = IceConfig::default();
        let mut agent = IceAgent::new(Some(ice_config));

        agent.gather_candidates().await?;

        let candidates: Vec<String> = agent
            .local_candidates()
            .iter()
            .map(|c| c.to_sdp())
            .collect();

        let _ = self.event_tx.send(NetworkEvent::IceCandidatesGathered {
            candidates: candidates.clone(),
        });

        self.ice_agent = Some(agent);

        tracing::info!("Gathered {} ICE candidates", candidates.len());
        Ok(candidates)
    }

    pub fn ice_candidates(&self) -> Option<Vec<&Candidate>> {
        self.ice_agent
            .as_ref()
            .map(|a| a.local_candidates().iter().collect())
    }

    pub fn ice_connection_state(&self) -> Option<String> {
        self.ice_agent.as_ref().map(|a| match a.state() {
            IceConnectionState::New => "new".to_string(),
            IceConnectionState::Checking => "checking".to_string(),
            IceConnectionState::Connected => "connected".to_string(),
            IceConnectionState::Completed => "completed".to_string(),
            IceConnectionState::Failed => "failed".to_string(),
            IceConnectionState::Disconnected => "disconnected".to_string(),
            IceConnectionState::Closed => "closed".to_string(),
        })
    }

    pub fn add_remote_ice_candidate(&mut self, candidate_sdp: &str) -> AgoraResult<()> {
        let candidate = crate::ice::parse_candidate_from_sdp(candidate_sdp)
            .ok_or_else(|| Error::Network("Invalid ICE candidate SDP".to_string()))?;

        if let Some(ref mut agent) = self.ice_agent {
            let addr = candidate.connection_addr;
            agent.add_remote_candidate(candidate);
            tracing::debug!("Added remote ICE candidate: {}", addr);
        }

        Ok(())
    }

    pub async fn perform_ice_connectivity_checks(&mut self) -> AgoraResult<()> {
        if let Some(ref mut agent) = self.ice_agent {
            let state_before = agent.state();

            agent.perform_connectivity_checks().await?;

            let state_after = agent.state();
            if state_before != state_after {
                let _ = self.event_tx.send(NetworkEvent::IceConnectionStateChanged {
                    state: match state_after {
                        IceConnectionState::New => "new".to_string(),
                        IceConnectionState::Checking => "checking".to_string(),
                        IceConnectionState::Connected => "connected".to_string(),
                        IceConnectionState::Completed => "completed".to_string(),
                        IceConnectionState::Failed => "failed".to_string(),
                        IceConnectionState::Disconnected => "disconnected".to_string(),
                        IceConnectionState::Closed => "closed".to_string(),
                    },
                });
            }
        }

        Ok(())
    }

    pub fn get_selected_ice_connection(
        &self,
    ) -> Option<(std::net::SocketAddr, std::net::SocketAddr)> {
        self.ice_agent.as_ref()?.get_selected_connection()
    }

    pub fn get_ice_candidates_as_multiaddrs(&self) -> Vec<Multiaddr> {
        let mut addrs = Vec::new();

        if let Some(agent) = &self.ice_agent {
            for candidate in agent.local_candidates() {
                if let Ok(addr) = candidate_to_multiaddr(candidate) {
                    addrs.push(addr);
                }
            }
        }

        addrs
    }

    pub async fn dial(&mut self, addr: Multiaddr) -> AgoraResult<()> {
        self.swarm
            .dial(addr)
            .map_err(|e| Error::Network(format!("Dial error: {}", e)))?;
        Ok(())
    }

    pub fn add_address(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, addr);
    }

    pub async fn start_providing(&mut self, room_id: &str) -> AgoraResult<()> {
        let key = RecordKey::new(&room_id);
        self.swarm
            .behaviour_mut()
            .kademlia
            .start_providing(key)
            .map_err(|e| Error::Network(format!("Start providing error: {}", e)))?;
        tracing::info!("Started providing room: {}", room_id);
        Ok(())
    }

    pub fn get_providers(&mut self, room_id: &str) {
        let key = RecordKey::new(&room_id);
        self.swarm.behaviour_mut().kademlia.get_providers(key);
        tracing::debug!("Looking for providers of room: {}", room_id);
    }

    pub fn bootstrap(&mut self) -> AgoraResult<()> {
        self.swarm
            .behaviour_mut()
            .kademlia
            .bootstrap()
            .map_err(|e| Error::Network(format!("Bootstrap error: {:?}", e)))?;
        Ok(())
    }

    pub async fn run(&mut self) {
        let mut command_rx = match self.command_rx.take() {
            Some(rx) => rx,
            None => {
                tracing::error!("Command receiver already taken - run() called twice?");
                return;
            }
        };

        loop {
            tokio::select! {
                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        NetworkCommand::Stop => {
                            tracing::info!("Network node stopping");
                            break;
                        }
                        NetworkCommand::SendAudio { peer_id, packet } => {
                            self.send_audio_packet(peer_id, packet).await;
                        }
                        NetworkCommand::BroadcastAudio { room_id, packet } => {
                            self.broadcast_audio(&room_id, packet).await;
                        }
                        NetworkCommand::SendControl { peer_id, message } => {
                            self.send_control_message(peer_id, message).await;
                        }
                        NetworkCommand::JoinRoom { room_id } => {
                            if let Err(e) = self.join_room(&room_id).await {
                                tracing::error!("Failed to join room: {}", e);
                            }
                        }
                        NetworkCommand::LeaveRoom { room_id } => {
                            self.leave_room(&room_id);
                        }
                        NetworkCommand::ConnectToPeer { addr } => {
                            if let Err(e) = self.dial(addr).await {
                                tracing::error!("Failed to connect: {}", e);
                            }
                        }
                    }
                }

                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                }
            }
        }
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<AgoraBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                self.listen_addrs.push(address.clone());
                tracing::info!("Listening on {}", address);
                let _ = self.event_tx.send(NetworkEvent::Listening(address));
            }

            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                self.known_peers.insert(peer_id);
                tracing::info!("Connected to {}", peer_id);
                let _ = self.event_tx.send(NetworkEvent::PeerConnected {
                    peer_id,
                    addr: endpoint.get_remote_address().clone(),
                });
            }

            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                self.known_peers.remove(&peer_id);
                tracing::info!("Disconnected from {}", peer_id);
                let _ = self
                    .event_tx
                    .send(NetworkEvent::PeerDisconnected { peer_id });
            }

            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                tracing::warn!("Failed to connect to {:?}: {}", peer_id, error);
            }

            SwarmEvent::Behaviour(event) => {
                self.handle_behaviour_event(event).await;
            }

            _ => {}
        }
    }

    async fn handle_behaviour_event(&mut self, event: AgoraBehaviourEvent) {
        match event {
            AgoraBehaviourEvent::Kademlia(KademliaEvent::OutboundQueryProgressed {
                result,
                ..
            }) => match result {
                QueryResult::GetProviders(Ok(GetProvidersOk::FoundProviders {
                    key,
                    providers,
                    ..
                })) => {
                    let room_id = String::from_utf8_lossy(key.as_ref()).to_string();
                    let providers: Vec<PeerId> = providers.into_iter().collect();
                    tracing::info!("Found {} providers for room {}", providers.len(), room_id);
                    let _ = self
                        .event_tx
                        .send(NetworkEvent::ProvidersFound { room_id, providers });
                }
                QueryResult::StartProviding(Ok(_)) => {
                    tracing::debug!("Successfully started providing");
                }
                QueryResult::Bootstrap(Ok(_)) => {
                    tracing::info!("Bootstrap complete");
                    let _ = self.event_tx.send(NetworkEvent::BootstrapComplete);
                }
                _ => {}
            },

            AgoraBehaviourEvent::Identify(identify::Event::Received { peer_id, info }) => {
                tracing::debug!(
                    "Identified {} with {} addresses",
                    peer_id,
                    info.listen_addrs.len()
                );
                for addr in &info.listen_addrs {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr.clone());
                }
                let _ = self.event_tx.send(NetworkEvent::PeerIdentified {
                    peer_id,
                    listen_addrs: info.listen_addrs,
                });
            }

            AgoraBehaviourEvent::AudioStream(request_response::Event::Message {
                peer,
                message: request_response::Message::Request { request, .. },
                ..
            }) => {
                let _ = self.event_tx.send(NetworkEvent::AudioReceived {
                    peer_id: peer,
                    packet: request,
                });
            }

            AgoraBehaviourEvent::Control(request_response::Event::Message {
                peer,
                message: request_response::Message::Request { request, .. },
                ..
            }) => {
                self.handle_control_message(peer, &request).await;
                let _ = self.event_tx.send(NetworkEvent::ControlReceived {
                    peer_id: peer,
                    message: request,
                });
            }

            AgoraBehaviourEvent::Autonat(autonat::Event::StatusChanged { new, .. }) => {
                let is_public = matches!(new, autonat::NatStatus::Public(_));
                tracing::info!(
                    "NAT status: {}",
                    if is_public { "Public" } else { "Private" }
                );
                let _ = self
                    .event_tx
                    .send(NetworkEvent::NatStatusChanged { is_public });
            }

            AgoraBehaviourEvent::Ping(ping::Event { peer, result, .. }) => match result {
                Ok(duration) => {
                    tracing::trace!("Ping to {}: {:?}", peer, duration);
                }
                Err(e) => {
                    tracing::warn!("Ping failed to {}: {}", peer, e);
                }
            },

            AgoraBehaviourEvent::Dcutr(event) => {
                tracing::debug!("DCUtR event: {:?}", event);
            }

            _ => {}
        }
    }

    async fn handle_control_message(&mut self, peer_id: PeerId, message: &ControlMessage) {
        match &message.message_type {
            ControlMessageType::JoinRoom { room_id } => {
                self.room_peers
                    .entry(room_id.clone())
                    .or_default()
                    .insert(peer_id);
                if let Some(name) = &message.display_name {
                    self.peer_names.insert(peer_id, name.clone());
                }
                tracing::info!("Peer {} joined room {}", peer_id, room_id);
                let _ = self.event_tx.send(NetworkEvent::RoomJoined {
                    room_id: room_id.clone(),
                    peer_id,
                });
            }

            ControlMessageType::LeaveRoom { room_id } => {
                if let Some(peers) = self.room_peers.get_mut(room_id) {
                    peers.remove(&peer_id);
                }
                tracing::info!("Peer {} left room {}", peer_id, room_id);
                let _ = self.event_tx.send(NetworkEvent::RoomLeft {
                    room_id: room_id.clone(),
                    peer_id,
                });
            }

            ControlMessageType::UpdateInfo { display_name } => {
                self.peer_names.insert(peer_id, display_name.clone());
            }

            ControlMessageType::MuteChanged { is_muted } => {
                tracing::info!("Peer {} muted: {}", peer_id, is_muted);
            }

            _ => {}
        }
    }

    async fn send_audio_packet(&mut self, peer_id: PeerId, packet: AudioPacket) {
        let request_id = self
            .swarm
            .behaviour_mut()
            .audio_stream
            .send_request(&peer_id, packet);
        tracing::trace!(
            "Sent audio packet to {}, request_id: {:?}",
            peer_id,
            request_id
        );
    }

    async fn broadcast_audio(&mut self, room_id: &str, packet: AudioPacket) {
        let peers: Vec<PeerId> = self
            .room_peers
            .get(room_id)
            .map(|p| p.iter().cloned().collect())
            .unwrap_or_default();

        for peer_id in peers {
            let packet = packet.clone();
            self.send_audio_packet(peer_id, packet).await;
        }
    }

    async fn send_control_message(&mut self, peer_id: PeerId, message: ControlMessage) {
        let request_id = self
            .swarm
            .behaviour_mut()
            .control
            .send_request(&peer_id, message);
        tracing::debug!(
            "Sent control message to {}, request_id: {:?}",
            peer_id,
            request_id
        );
    }

    async fn join_room(&mut self, room_id: &str) -> AgoraResult<()> {
        self.start_providing(room_id).await?;
        self.get_providers(room_id);
        Ok(())
    }

    fn leave_room(&mut self, room_id: &str) {
        self.room_peers.remove(room_id);
        tracing::info!("Left room: {}", room_id);
    }
}

pub fn parse_peer_id(s: &str) -> AgoraResult<PeerId> {
    s.parse()
        .map_err(|e| Error::Network(format!("Invalid peer ID '{}': {}", s, e)))
}

pub fn parse_multiaddr(s: &str) -> AgoraResult<Multiaddr> {
    s.parse()
        .map_err(|e| Error::Network(format!("Invalid multiaddr '{}': {}", s, e)))
}

fn candidate_to_multiaddr(candidate: &Candidate) -> AgoraResult<Multiaddr> {
    let ip = candidate.connection_addr.ip();
    let port = candidate.connection_addr.port();

    let addr_str = match ip {
        std::net::IpAddr::V4(ipv4) => format!("/ip4/{}/tcp/{}", ipv4, port),
        std::net::IpAddr::V6(ipv6) => format!("/ip6/{}/tcp/{}", ipv6, port),
    };

    addr_str
        .parse()
        .map_err(|e| Error::Network(format!("Failed to create multiaddr: {}", e)))
}
