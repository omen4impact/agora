use serde_json;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SignalingMessage {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PeerInfo {
    peer_id: String,
    display_name: Option<String>,
    joined_at: u64,
}

#[test]
fn test_signaling_message_join_serialization() {
    let msg = SignalingMessage::Join {
        room_id: "test_room".to_string(),
        peer_id: "peer_123".to_string(),
        display_name: Some("Alice".to_string()),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"join\""));
    assert!(json.contains("\"room_id\":\"test_room\""));
    assert!(json.contains("\"peer_id\":\"peer_123\""));
    assert!(json.contains("\"display_name\":\"Alice\""));

    let decoded: SignalingMessage = serde_json::from_str(&json).unwrap();
    match decoded {
        SignalingMessage::Join {
            room_id,
            peer_id,
            display_name,
        } => {
            assert_eq!(room_id, "test_room");
            assert_eq!(peer_id, "peer_123");
            assert_eq!(display_name, Some("Alice".to_string()));
        }
        _ => panic!("Expected Join message"),
    }
}

#[test]
fn test_signaling_message_sdp_offer_serialization() {
    let msg = SignalingMessage::SdpOffer {
        from: "peer1".to_string(),
        to: "peer2".to_string(),
        sdp: "v=0\r\no=- 12345 12345 IN IP4 127.0.0.1".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let decoded: SignalingMessage = serde_json::from_str(&json).unwrap();

    match decoded {
        SignalingMessage::SdpOffer { from, to, sdp } => {
            assert_eq!(from, "peer1");
            assert_eq!(to, "peer2");
            assert!(sdp.contains("v=0"));
        }
        _ => panic!("Expected SdpOffer message"),
    }
}

#[test]
fn test_signaling_message_ice_candidate_serialization() {
    let msg = SignalingMessage::IceCandidate {
        from: "peer1".to_string(),
        to: "peer2".to_string(),
        candidate: "candidate:1 1 UDP 2122260223 192.168.1.1 54321 typ host".to_string(),
        sdp_mid: Some("audio".to_string()),
        sdp_mline_index: Some(0),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let decoded: SignalingMessage = serde_json::from_str(&json).unwrap();

    match decoded {
        SignalingMessage::IceCandidate {
            candidate,
            sdp_mid,
            sdp_mline_index,
            ..
        } => {
            assert!(candidate.contains("candidate:1"));
            assert_eq!(sdp_mid, Some("audio".to_string()));
            assert_eq!(sdp_mline_index, Some(0));
        }
        _ => panic!("Expected IceCandidate message"),
    }
}

#[test]
fn test_signaling_message_peer_list_serialization() {
    let msg = SignalingMessage::PeerList {
        room_id: "test_room".to_string(),
        peers: vec![
            PeerInfo {
                peer_id: "peer1".to_string(),
                display_name: Some("Alice".to_string()),
                joined_at: 1234567890,
            },
            PeerInfo {
                peer_id: "peer2".to_string(),
                display_name: Some("Bob".to_string()),
                joined_at: 1234567891,
            },
        ],
    };

    let json = serde_json::to_string(&msg).unwrap();
    let decoded: SignalingMessage = serde_json::from_str(&json).unwrap();

    match decoded {
        SignalingMessage::PeerList { room_id, peers } => {
            assert_eq!(room_id, "test_room");
            assert_eq!(peers.len(), 2);
            assert_eq!(peers[0].peer_id, "peer1");
            assert_eq!(peers[1].display_name, Some("Bob".to_string()));
        }
        _ => panic!("Expected PeerList message"),
    }
}

#[test]
fn test_signaling_message_error_serialization() {
    let msg = SignalingMessage::Error {
        message: "Room not found".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let decoded: SignalingMessage = serde_json::from_str(&json).unwrap();

    match decoded {
        SignalingMessage::Error { message } => {
            assert_eq!(message, "Room not found");
        }
        _ => panic!("Expected Error message"),
    }
}

#[test]
fn test_signaling_message_roundtrip() {
    let messages = vec![
        SignalingMessage::Join {
            room_id: "room1".to_string(),
            peer_id: "peer1".to_string(),
            display_name: None,
        },
        SignalingMessage::Leave {
            room_id: "room1".to_string(),
            peer_id: "peer1".to_string(),
        },
        SignalingMessage::SdpOffer {
            from: "peer1".to_string(),
            to: "peer2".to_string(),
            sdp: "offer_sdp".to_string(),
        },
        SignalingMessage::SdpAnswer {
            from: "peer2".to_string(),
            to: "peer1".to_string(),
            sdp: "answer_sdp".to_string(),
        },
        SignalingMessage::IceCandidate {
            from: "peer1".to_string(),
            to: "peer2".to_string(),
            candidate: "candidate_data".to_string(),
            sdp_mid: None,
            sdp_mline_index: None,
        },
    ];

    for msg in messages {
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: SignalingMessage = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&decoded).unwrap();
        assert_eq!(json, json2, "Roundtrip failed for {:?}", msg);
    }
}

#[test]
fn test_peer_info_serialization() {
    let peer = PeerInfo {
        peer_id: "12D3KooWABC123".to_string(),
        display_name: Some("Test User".to_string()),
        joined_at: 1700000000,
    };

    let json = serde_json::to_string(&peer).unwrap();
    let decoded: PeerInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.peer_id, peer.peer_id);
    assert_eq!(decoded.display_name, peer.display_name);
    assert_eq!(decoded.joined_at, peer.joined_at);
}
