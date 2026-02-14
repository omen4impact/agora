use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    Join {
        room_id: String,
        peer_id: String,
        display_name: Option<String>,
    },
    Leave {
        room_id: String,
        peer_id: String,
    },
    SdpOffer {
        from: String,
        to: String,
        sdp: String,
    },
    SdpAnswer {
        from: String,
        to: String,
        sdp: String,
    },
    IceCandidate {
        from: String,
        to: String,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u32>,
    },
    PeerList {
        room_id: String,
        peers: Vec<PeerInfo>,
    },
    PeerJoined {
        room_id: String,
        peer: PeerInfo,
    },
    PeerLeft {
        room_id: String,
        peer_id: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub display_name: Option<String>,
    pub joined_at: u64,
}

#[derive(Debug, Clone)]
struct Room {
    peers: HashMap<String, PeerInfo>,
}

#[allow(dead_code)]
impl Room {
    fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    fn add_peer(&mut self, peer_id: String, display_name: Option<String>) -> PeerInfo {
        let info = PeerInfo {
            peer_id: peer_id.clone(),
            display_name,
            joined_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        self.peers.insert(peer_id, info.clone());
        info
    }

    fn remove_peer(&mut self, peer_id: &str) -> Option<PeerInfo> {
        self.peers.remove(peer_id)
    }

    fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers.values().cloned().collect()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ConnectedPeer {
    peer_id: String,
    room_id: Option<String>,
    tx: broadcast::Sender<SignalingMessage>,
}

pub struct SignalingState {
    rooms: RwLock<HashMap<String, Room>>,
    peers: RwLock<HashMap<String, ConnectedPeer>>,
}

impl SignalingState {
    pub fn new() -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
            peers: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for SignalingState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn signaling_router() -> Router<Arc<SignalingState>> {
    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/rooms/{room_id}/peers", get(get_room_peers))
}

#[allow(dead_code)]
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<SignalingState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let peer_id = params
        .get("peer_id")
        .cloned()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    ws.on_upgrade(move |socket| handle_websocket(socket, state, peer_id))
}

#[allow(dead_code)]
async fn handle_websocket(socket: WebSocket, state: Arc<SignalingState>, peer_id: String) {
    let (mut tx, mut rx) = socket.split();
    let (msg_tx, mut msg_rx) = broadcast::channel::<SignalingMessage>(100);

    {
        let mut peers = state.peers.write().await;
        peers.insert(
            peer_id.clone(),
            ConnectedPeer {
                peer_id: peer_id.clone(),
                room_id: None,
                tx: msg_tx.clone(),
            },
        );
    }

    info!("WebSocket connected: {}", peer_id);

    let send_task = async move {
        while let Ok(msg) = msg_rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap_or_default();
            if tx.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    };

    let peer_id_clone = peer_id.clone();
    let state_clone = state.clone();

    let recv_task = async move {
        while let Some(msg) = rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(signaling_msg) = serde_json::from_str::<SignalingMessage>(&text) {
                        handle_signaling_message(&state_clone, &peer_id_clone, signaling_msg).await;
                    } else {
                        warn!("Failed to parse signaling message: {}", text);
                    }
                }
                Ok(Message::Close(_)) => {
                    debug!("WebSocket close received from {}", peer_id_clone);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error from {}: {}", peer_id_clone, e);
                    break;
                }
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    cleanup_peer(&state, &peer_id).await;
    info!("WebSocket disconnected: {}", peer_id);
}

#[allow(dead_code)]
async fn handle_signaling_message(
    state: &Arc<SignalingState>,
    peer_id: &str,
    msg: SignalingMessage,
) {
    match msg {
        SignalingMessage::Join {
            room_id,
            peer_id: joining_peer_id,
            display_name,
        } => {
            let peer_info;
            {
                let mut rooms = state.rooms.write().await;
                let room = rooms.entry(room_id.clone()).or_insert_with(Room::new);
                peer_info = room.add_peer(joining_peer_id.clone(), display_name);
            }

            {
                let mut peers = state.peers.write().await;
                if let Some(peer) = peers.get_mut(peer_id) {
                    peer.room_id = Some(room_id.clone());
                }
            }

            let peers_list = {
                let rooms = state.rooms.read().await;
                rooms
                    .get(&room_id)
                    .map(|r| r.get_peers())
                    .unwrap_or_default()
            };

            {
                let peers = state.peers.read().await;
                for (pid, peer) in peers.iter() {
                    if pid != peer_id {
                        let _ = peer.tx.send(SignalingMessage::PeerJoined {
                            room_id: room_id.clone(),
                            peer: peer_info.clone(),
                        });
                    }
                }
            }

            if let Some(peer) = state.peers.read().await.get(peer_id) {
                let _ = peer.tx.send(SignalingMessage::PeerList {
                    room_id,
                    peers: peers_list,
                });
            }
        }

        SignalingMessage::Leave {
            room_id,
            peer_id: leaving_peer_id,
        } => {
            let mut rooms = state.rooms.write().await;
            if let Some(room) = rooms.get_mut(&room_id) {
                room.remove_peer(&leaving_peer_id);
            }
        }

        SignalingMessage::SdpOffer { from, to, sdp } => {
            let peers = state.peers.read().await;
            if let Some(peer) = peers.get(&to) {
                let _ = peer.tx.send(SignalingMessage::SdpOffer { from, to, sdp });
            }
        }

        SignalingMessage::SdpAnswer { from, to, sdp } => {
            let peers = state.peers.read().await;
            if let Some(peer) = peers.get(&to) {
                let _ = peer.tx.send(SignalingMessage::SdpAnswer { from, to, sdp });
            }
        }

        SignalingMessage::IceCandidate {
            from,
            to,
            candidate,
            sdp_mid,
            sdp_mline_index,
        } => {
            let peers = state.peers.read().await;
            if let Some(peer) = peers.get(&to) {
                let _ = peer.tx.send(SignalingMessage::IceCandidate {
                    from,
                    to,
                    candidate,
                    sdp_mid,
                    sdp_mline_index,
                });
            }
        }

        _ => {
            warn!("Unhandled signaling message type from {}", peer_id);
        }
    }
}

#[allow(dead_code)]
async fn cleanup_peer(state: &Arc<SignalingState>, peer_id: &str) {
    let room_id = {
        let peers = state.peers.read().await;
        peers.get(peer_id).and_then(|p| p.room_id.clone())
    };

    if let Some(ref rid) = room_id {
        let mut rooms = state.rooms.write().await;
        if let Some(room) = rooms.get_mut(rid) {
            room.remove_peer(peer_id);
        }
    }

    {
        let mut peers = state.peers.write().await;
        peers.remove(peer_id);
    }
}

#[allow(dead_code)]
async fn get_room_peers(
    Path(room_id): Path<String>,
    State(state): State<Arc<SignalingState>>,
) -> axum::Json<Vec<PeerInfo>> {
    let rooms = state.rooms.read().await;
    let peers = rooms
        .get(&room_id)
        .map(|r| r.get_peers())
        .unwrap_or_default();
    axum::Json(peers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signaling_message_serialization() {
        let msg = SignalingMessage::Join {
            room_id: "room123".to_string(),
            peer_id: "peer456".to_string(),
            display_name: Some("Alice".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("join"));
        assert!(json.contains("room123"));

        let decoded: SignalingMessage = serde_json::from_str(&json).unwrap();
        match decoded {
            SignalingMessage::Join { room_id, .. } => {
                assert_eq!(room_id, "room123");
            }
            _ => panic!("Expected Join message"),
        }
    }

    #[test]
    fn test_room_peer_management() {
        let mut room = Room::new();

        room.add_peer("peer1".to_string(), Some("Alice".to_string()));
        room.add_peer("peer2".to_string(), Some("Bob".to_string()));

        assert_eq!(room.peers.len(), 2);

        let peers = room.get_peers();
        assert_eq!(peers.len(), 2);

        room.remove_peer("peer1");
        assert_eq!(room.peers.len(), 1);
    }

    #[test]
    fn test_sdp_offer_serialization() {
        let msg = SignalingMessage::SdpOffer {
            from: "peer1".to_string(),
            to: "peer2".to_string(),
            sdp: "v=0...".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let decoded: SignalingMessage = serde_json::from_str(&json).unwrap();

        match decoded {
            SignalingMessage::SdpOffer { from, to, sdp } => {
                assert_eq!(from, "peer1");
                assert_eq!(to, "peer2");
                assert_eq!(sdp, "v=0...");
            }
            _ => panic!("Expected SdpOffer"),
        }
    }

    #[test]
    fn test_ice_candidate_serialization() {
        let msg = SignalingMessage::IceCandidate {
            from: "peer1".to_string(),
            to: "peer2".to_string(),
            candidate: "candidate:...".to_string(),
            sdp_mid: Some("audio".to_string()),
            sdp_mline_index: Some(0),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let decoded: SignalingMessage = serde_json::from_str(&json).unwrap();

        match decoded {
            SignalingMessage::IceCandidate {
                candidate, sdp_mid, ..
            } => {
                assert_eq!(candidate, "candidate:...");
                assert_eq!(sdp_mid, Some("audio".to_string()));
            }
            _ => panic!("Expected IceCandidate"),
        }
    }
}
