use agora_core::{
    mixer::{ParticipantStats, ScoreWeights, TopologyMode},
    protocol::{AudioPacket, ControlMessage, ControlMessageType, JitterBuffer},
    storage::IdentityStorage,
    AudioConfig, AudioPipeline, Identity, MixerConfig, MixerManager, MixerRole, NetworkNode,
    Participant, Room, RoomConfig,
};
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn test_identity_generation_and_persistence() {
    let identity1 = Identity::generate().expect("Failed to generate identity");
    let peer_id1 = identity1.peer_id();

    assert!(!peer_id1.is_empty());
    assert!(peer_id1.starts_with("12D3KooW"));

    let identity2 = Identity::generate().expect("Failed to generate identity");
    assert_ne!(peer_id1, identity2.peer_id());

    let dir = tempdir().expect("Failed to create temp dir");
    let storage =
        IdentityStorage::with_path(dir.path().to_path_buf()).expect("Failed to create storage");

    let mut identity = Identity::generate().expect("Failed to generate identity");
    identity.set_display_name("Test User".to_string());

    storage.save(&identity).expect("Failed to save identity");
    assert!(storage.has_stored_identity());

    let loaded = storage.load().expect("Failed to load identity");
    assert_eq!(identity.peer_id(), loaded.peer_id());
    assert_eq!(identity.display_name(), loaded.display_name());

    let bytes = identity.to_bytes();
    let restored = Identity::from_bytes(&bytes).expect("Failed to restore identity");
    assert_eq!(identity.peer_id(), restored.peer_id());

    let message = b"Test message";
    let signature = identity.sign(message);
    assert!(identity.verify(message, &signature));
}

#[tokio::test]
async fn test_room_creation_and_link_sharing() {
    let identity = Identity::generate().expect("Failed to generate identity");

    let config = RoomConfig {
        name: Some("Test Room".to_string()),
        password: Some("secret123".to_string()),
        max_participants: Some(10),
    };

    let room = Room::new(identity.peer_id(), config);

    assert!(!room.id.is_empty());
    assert_eq!(room.id.len(), 16);
    assert_eq!(room.name, Some("Test Room".to_string()));
    assert!(room.has_password());
    assert!(room.verify_password("secret123"));
    assert!(!room.verify_password("wrong"));

    let link = room.share_link();
    assert!(link.starts_with("agora://room/"));
    assert!(link.ends_with(&room.id));

    let link_with_password = room.share_link_with_password("secret123");
    assert!(link_with_password.contains("?p="));
}

#[tokio::test]
async fn test_room_link_parsing() {
    let room_id = "a1b2c3d4e5f67890";
    let link = format!("agora://room/{}", room_id);

    let parsed = agora_core::room::parse_room_link(&link);
    assert!(parsed.is_some());

    let (parsed_id, password) = parsed.unwrap();
    assert_eq!(parsed_id, room_id);
    assert!(password.is_none());

    let link_with_password = format!("agora://room/{}?p=mypassword", room_id);
    let parsed_with_pwd = agora_core::room::parse_room_link(&link_with_password);
    assert!(parsed_with_pwd.is_some());

    let (parsed_id2, password2) = parsed_with_pwd.unwrap();
    assert_eq!(parsed_id2, room_id);
    assert_eq!(password2, Some("mypassword".to_string()));

    let invalid_link = "invalid://notagora/abc";
    let parsed_invalid = agora_core::room::parse_room_link(invalid_link);
    assert!(parsed_invalid.is_none());
}

#[tokio::test]
async fn test_mixer_topology_switching() {
    let mut manager = MixerManager::new("local_peer".to_string(), None);

    assert_eq!(manager.get_topology_mode(), TopologyMode::FullMesh);
    assert_eq!(manager.get_participant_count(), 1);

    for i in 0..4 {
        manager.add_participant(format!("peer_{}", i));
    }
    assert_eq!(manager.get_participant_count(), 5);
    assert_eq!(manager.get_topology_mode(), TopologyMode::FullMesh);

    manager.add_participant("peer_5".to_string());
    assert_eq!(manager.get_participant_count(), 6);
    assert_eq!(manager.get_topology_mode(), TopologyMode::SFU);

    manager.add_participant("peer_6".to_string());
    manager.add_participant("peer_7".to_string());
    assert_eq!(manager.get_topology_mode(), TopologyMode::SFU);

    manager.remove_participant("peer_5");
    manager.remove_participant("peer_6");
    manager.remove_participant("peer_7");
    assert_eq!(manager.get_participant_count(), 5);
    assert_eq!(manager.get_topology_mode(), TopologyMode::FullMesh);
}

#[tokio::test]
async fn test_mixer_score_calculation() {
    let mut manager = MixerManager::new("local_peer".to_string(), None);

    for i in 0..6 {
        manager.add_participant(format!("peer_{}", i));
    }

    assert_eq!(manager.get_topology_mode(), TopologyMode::SFU);

    let stats1 = ParticipantStats {
        bandwidth_bps: 10_000_000,
        latency_ms: 20,
        latency_variance: 0.0,
        cpu_usage_percent: 10.0,
        memory_usage_percent: 20.0,
        session_duration: Duration::from_secs(3600),
        packet_loss_percent: 0.0,
        last_updated: std::time::Instant::now(),
    };
    manager.update_participant_stats("peer_0", stats1);

    let stats2 = ParticipantStats {
        bandwidth_bps: 1_000_000,
        latency_ms: 200,
        latency_variance: 100.0,
        cpu_usage_percent: 80.0,
        memory_usage_percent: 90.0,
        session_duration: Duration::from_secs(60),
        packet_loss_percent: 5.0,
        last_updated: std::time::Instant::now(),
    };
    manager.update_participant_stats("peer_1", stats2);

    let mixer = manager.select_mixer();
    assert!(mixer.is_some());

    let participants = manager.get_participants();
    let p1 = participants.get("peer_0").expect("peer_0 should exist");
    let p2 = participants.get("peer_1").expect("peer_1 should exist");

    let weights = ScoreWeights::default();
    let mut test_p1 = Participant::new("peer_0".to_string());
    test_p1.stats = p1.stats.clone();
    let score1 = test_p1.calculate_score(&weights);

    let mut test_p2 = Participant::new("peer_1".to_string());
    test_p2.stats = p2.stats.clone();
    let score2 = test_p2.calculate_score(&weights);

    assert!(
        score1 > score2,
        "Higher bandwidth and lower CPU should yield higher score"
    );
    assert!(score1 > 0.0 && score1 <= 1.0);
    assert!(score2 > 0.0 && score2 <= 1.0);
}

#[tokio::test]
async fn test_mixer_selection_with_tie() {
    let mut manager = MixerManager::new("local_peer".to_string(), None);

    for i in 0..6 {
        manager.add_participant(format!("peer_{}", i));
    }

    let mixer1 = manager.select_mixer();
    let mixer2 = manager.select_mixer();

    assert_eq!(
        mixer1, mixer2,
        "Same scores should produce same mixer selection"
    );
}

#[tokio::test]
async fn test_audio_packet_serialization() {
    let frame: Vec<f32> = (0..960).map(|i| (i as f32 / 960.0) * 0.5).collect();
    let packet = AudioPacket::new(42, "peer_123".to_string(), frame.clone());

    assert_eq!(packet.sequence, 42);
    assert_eq!(packet.peer_id, "peer_123");
    assert_eq!(packet.frame.len(), 960);
    assert_eq!(packet.sample_rate, 48000);
    assert_eq!(packet.channels, 1);

    let encoded = packet.encode().expect("Failed to encode");
    assert!(!encoded.is_empty());

    let decoded = AudioPacket::decode(&encoded).expect("Failed to decode");

    assert_eq!(decoded.sequence, packet.sequence);
    assert_eq!(decoded.peer_id, packet.peer_id);
    assert_eq!(decoded.frame.len(), packet.frame.len());
    for (a, b) in decoded.frame.iter().zip(packet.frame.iter()) {
        assert!((a - b).abs() < f32::EPSILON);
    }
}

#[tokio::test]
async fn test_control_message_serialization() {
    let join_msg = ControlMessage::join_room("room_abc123".to_string(), "peer_xyz".to_string());

    let encoded = join_msg.encode().expect("Failed to encode");
    assert!(!encoded.is_empty());

    let decoded = ControlMessage::decode(&encoded).expect("Failed to decode");

    match decoded.message_type {
        ControlMessageType::JoinRoom { room_id } => {
            assert_eq!(room_id, "room_abc123");
        }
        _ => panic!("Expected JoinRoom message type"),
    }
    assert_eq!(decoded.peer_id, "peer_xyz");

    let leave_msg = ControlMessage::leave_room("room_abc123".to_string(), "peer_xyz".to_string());
    let encoded_leave = leave_msg.encode().expect("Failed to encode leave");
    let decoded_leave = ControlMessage::decode(&encoded_leave).expect("Failed to decode leave");

    match decoded_leave.message_type {
        ControlMessageType::LeaveRoom { room_id } => {
            assert_eq!(room_id, "room_abc123");
        }
        _ => panic!("Expected LeaveRoom message type"),
    }

    let mute_msg = ControlMessage::mute_changed("peer_xyz".to_string(), true);
    let encoded_mute = mute_msg.encode().expect("Failed to encode mute");
    let decoded_mute = ControlMessage::decode(&encoded_mute).expect("Failed to decode mute");

    match decoded_mute.message_type {
        ControlMessageType::MuteChanged { is_muted } => {
            assert!(is_muted);
        }
        _ => panic!("Expected MuteChanged message type"),
    }

    let participant_list_msg = ControlMessage::new(
        ControlMessageType::ParticipantList {
            participants: vec![
                agora_core::ProtocolParticipantInfo {
                    peer_id: "peer1".to_string(),
                    display_name: Some("Alice".to_string()),
                    is_mixer: true,
                    is_muted: false,
                    latency_ms: 50,
                },
                agora_core::ProtocolParticipantInfo {
                    peer_id: "peer2".to_string(),
                    display_name: Some("Bob".to_string()),
                    is_mixer: false,
                    is_muted: true,
                    latency_ms: 100,
                },
            ],
        },
        "peer_xyz".to_string(),
    );

    let encoded_list = participant_list_msg
        .encode()
        .expect("Failed to encode list");
    let decoded_list = ControlMessage::decode(&encoded_list).expect("Failed to decode list");

    match decoded_list.message_type {
        ControlMessageType::ParticipantList { participants } => {
            assert_eq!(participants.len(), 2);
        }
        _ => panic!("Expected ParticipantList message type"),
    }
}

#[tokio::test]
async fn test_jitter_buffer() {
    let mut buffer = JitterBuffer::new(100, 48000);

    assert_eq!(buffer.buffer_depth(), 0);

    for seq in 0..5 {
        let packet = AudioPacket::new(seq, "peer1".to_string(), vec![0.5; 960]);
        buffer.push(packet);
    }

    assert!(buffer.buffer_depth() > 0);

    let first = buffer.pop();
    assert!(first.is_some());
    assert_eq!(first.unwrap().sequence, 0);

    for i in 1..5 {
        let packet = buffer.pop();
        assert!(packet.is_some());
        assert_eq!(packet.unwrap().sequence, i);
    }

    assert_eq!(buffer.buffer_depth(), 0);
    assert!(buffer.pop().is_none());

    for seq in 0..3 {
        let packet = AudioPacket::new(seq, "peer1".to_string(), vec![0.3; 960]);
        buffer.push(packet);
    }

    buffer.clear();
    assert_eq!(buffer.buffer_depth(), 0);
}

#[tokio::test]
async fn test_network_node_creation() {
    let node = NetworkNode::new(None)
        .await
        .expect("Failed to create network node");

    let _peer_id = node.local_peer_id();
    let peer_id_str = node.peer_id_string();

    assert!(!peer_id_str.is_empty());
    assert!(peer_id_str.starts_with("12D3KooW"));

    let known_peers = node.known_peers();
    assert!(known_peers.is_empty());
}

#[tokio::test]
async fn test_network_event_subscription() {
    let node = NetworkNode::new(None)
        .await
        .expect("Failed to create network node");

    let event_rx = node.subscribe_events();

    let peer_id = node.local_peer_id();
    assert!(!peer_id.to_string().is_empty());

    drop(event_rx);
}

#[tokio::test]
async fn test_e2e_room_flow() {
    let identity1 = Identity::generate().expect("Failed to generate identity 1");
    let identity2 = Identity::generate().expect("Failed to generate identity 2");

    let config = RoomConfig {
        name: Some("E2E Test Room".to_string()),
        password: None,
        max_participants: Some(10),
    };

    let room = Room::new(identity1.peer_id(), config.clone());
    let room_id = room.id.clone();

    assert!(!room_id.is_empty());
    assert_eq!(room.name, Some("E2E Test Room".to_string()));
    assert!(!room.has_password());

    let link = room.share_link();
    let parsed = agora_core::room::parse_room_link(&link);
    assert!(parsed.is_some());

    let (parsed_id, _) = parsed.unwrap();
    assert_eq!(parsed_id, room_id);

    let mut mixer1 = MixerManager::new(identity1.peer_id(), None);
    let mut mixer2 = MixerManager::new(identity2.peer_id(), None);

    mixer1.add_participant(identity2.peer_id());
    mixer2.add_participant(identity1.peer_id());

    assert_eq!(mixer1.get_participant_count(), 2);
    assert_eq!(mixer2.get_participant_count(), 2);
    assert_eq!(mixer1.get_topology_mode(), TopologyMode::FullMesh);
}

#[tokio::test]
async fn test_audio_pipeline_lifecycle() {
    let config = AudioConfig::default();
    let mut pipeline = AudioPipeline::new(config);

    assert!(!pipeline.is_running());

    let result = pipeline.start();
    if result.is_ok() {
        assert!(pipeline.is_running());

        let stats = pipeline.get_stats();
        assert_eq!(stats.frames_processed, 0);

        pipeline.stop();
        assert!(!pipeline.is_running());
    }
}

#[tokio::test]
async fn test_multiple_room_participants() {
    let mut identities: Vec<Identity> = Vec::new();
    for _ in 0..10 {
        identities.push(Identity::generate().expect("Failed to generate identity"));
    }

    let host_identity = &identities[0];
    let config = RoomConfig {
        name: Some("Multi-Participant Room".to_string()),
        password: Some("secret".to_string()),
        max_participants: Some(20),
    };

    let _room = Room::new(host_identity.peer_id(), config);

    let mut mixer_manager = MixerManager::new(host_identity.peer_id(), None);

    for identity in identities.iter().skip(1) {
        mixer_manager.add_participant(identity.peer_id());
    }

    assert_eq!(mixer_manager.get_participant_count(), 10);
    assert_eq!(mixer_manager.get_topology_mode(), TopologyMode::SFU);

    let mixer = mixer_manager.select_mixer();
    assert!(mixer.is_some());

    for identity in identities.iter().skip(1) {
        let mut local_mixer = MixerManager::new(identity.peer_id(), None);
        for other in identities
            .iter()
            .filter(|i| i.peer_id() != identity.peer_id())
        {
            local_mixer.add_participant(other.peer_id());
        }
        assert_eq!(local_mixer.get_participant_count(), 10);
    }
}

#[allow(clippy::field_reassign_with_default)]
#[tokio::test]
async fn test_mixer_rotation() {
    let mut config = MixerConfig::default();
    config.rotation_interval = Duration::from_millis(50);

    let mut manager = MixerManager::new("local".to_string(), Some(config));

    for i in 0..6 {
        manager.add_participant(format!("peer_{}", i));
    }

    assert_eq!(manager.get_topology_mode(), TopologyMode::SFU);

    let first_mixer = manager.select_mixer();
    assert!(first_mixer.is_some());

    assert!(!manager.check_rotation());

    tokio::time::sleep(Duration::from_millis(60)).await;

    assert!(manager.check_rotation());

    let new_mixer = manager.rotate_mixer();
    assert!(new_mixer.is_some());

    assert!(!manager.check_rotation());
}

#[tokio::test]
async fn test_connection_targets() {
    let mut manager = MixerManager::new("local".to_string(), None);

    manager.add_participant("peer1".to_string());
    manager.add_participant("peer2".to_string());
    manager.add_participant("peer3".to_string());

    assert_eq!(manager.get_topology_mode(), TopologyMode::FullMesh);

    let targets = manager.get_connection_targets();
    assert_eq!(targets.len(), 3);
    assert!(targets.contains(&"peer1".to_string()));
    assert!(targets.contains(&"peer2".to_string()));
    assert!(targets.contains(&"peer3".to_string()));

    for i in 4..7 {
        manager.add_participant(format!("peer{}", i));
    }

    assert_eq!(manager.get_topology_mode(), TopologyMode::SFU);

    manager.select_mixer();

    let sfu_targets = manager.get_connection_targets();
    assert!(
        sfu_targets.len() <= 1,
        "In SFU mode, should only connect to mixer"
    );
}

#[tokio::test]
async fn test_participant_role_changes() {
    let mut manager = MixerManager::new("local".to_string(), None);

    for i in 0..6 {
        manager.add_participant(format!("peer_{}", i));
    }

    manager.select_mixer();

    let current_mixer = manager.get_current_mixer();
    assert!(current_mixer.is_some());

    if let Some(mixer_id) = current_mixer {
        if mixer_id == "local" {
            assert!(manager.is_mixer());
        } else if let Some(participant) = manager.get_participant_info(mixer_id) {
            assert_eq!(participant.role, MixerRole::Mixer);
        }
    }

    manager.rotate_mixer();

    let new_mixer = manager.get_current_mixer();
    assert!(new_mixer.is_some());
}

mod stress_tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_high_volume_audio_packets() {
        let mut packets = Vec::new();

        for seq in 0..1000 {
            let frame: Vec<f32> = vec![0.5; 960];
            let packet = AudioPacket::new(seq, "stress_test_peer".to_string(), frame);
            packets.push(packet);
        }

        let mut encoded_packets = Vec::new();
        for packet in &packets {
            let encoded = packet.encode().expect("Failed to encode");
            encoded_packets.push(encoded);
        }

        assert_eq!(encoded_packets.len(), 1000);

        let mut jitter_buffer = JitterBuffer::new(200, 48000);

        for packet in packets {
            jitter_buffer.push(packet);
        }

        let mut decoded_count = 0;
        while jitter_buffer.pop().is_some() {
            decoded_count += 1;
        }

        assert!(decoded_count > 0);
    }

    #[tokio::test]
    async fn test_rapid_participant_changes() {
        let mut manager = MixerManager::new("local".to_string(), None);

        for round in 0..10 {
            for i in 0..20 {
                manager.add_participant(format!("peer_{}_{}", round, i));
            }

            let count_after_add = manager.get_participant_count();
            assert_eq!(count_after_add, 1 + 20);

            if manager.get_topology_mode() == TopologyMode::SFU {
                manager.select_mixer();
            }

            for i in 0..20 {
                manager.remove_participant(&format!("peer_{}_{}", round, i));
            }

            let count_after_remove = manager.get_participant_count();
            assert_eq!(count_after_remove, 1);
        }
    }

    #[tokio::test]
    async fn test_concurrent_mixer_operations() {
        let manager = Arc::new(Mutex::new(MixerManager::new("local".to_string(), None)));

        let mut handles = vec![];

        for i in 0..5 {
            let mgr = manager.clone();
            let handle = tokio::spawn(async move {
                let mut m = mgr.lock().await;
                for j in 0..3 {
                    m.add_participant(format!("concurrent_peer_{}_{}", i, j));
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Task failed");
        }

        let final_manager = manager.lock().await;
        assert_eq!(final_manager.get_participant_count(), 16);
    }

    #[tokio::test]
    async fn test_stress_packet_encoding_decoding() {
        let mut success_count = 0;
        let total = 500;

        for i in 0..total {
            let size = (i % 960) + 1;
            let frame: Vec<f32> = (0..size)
                .map(|j| (j as f32 / size as f32) * 2.0 - 1.0)
                .collect();
            let packet = AudioPacket::new(i as u64, format!("peer_{}", i % 10), frame);

            if let Ok(encoded) = packet.encode() {
                if let Ok(decoded) = AudioPacket::decode(&encoded) {
                    if decoded.sequence == packet.sequence && decoded.peer_id == packet.peer_id {
                        success_count += 1;
                    }
                }
            }
        }

        assert_eq!(
            success_count, total,
            "All packets should encode/decode successfully"
        );
    }

    #[tokio::test]
    async fn test_stress_control_messages() {
        let message_types = [
            ControlMessageType::JoinRoom {
                room_id: "room1".to_string(),
            },
            ControlMessageType::LeaveRoom {
                room_id: "room1".to_string(),
            },
            ControlMessageType::MuteChanged { is_muted: true },
            ControlMessageType::MuteChanged { is_muted: false },
            ControlMessageType::Ping,
            ControlMessageType::Pong,
            ControlMessageType::UpdateInfo {
                display_name: "Test".to_string(),
            },
        ];

        let mut success_count = 0;

        for (i, msg_type) in message_types.iter().cycle().take(100).enumerate() {
            let msg = ControlMessage::new(msg_type.clone(), format!("peer_{}", i % 20));

            if let Ok(encoded) = msg.encode() {
                if let Ok(decoded) = ControlMessage::decode(&encoded) {
                    if decoded.peer_id == msg.peer_id {
                        success_count += 1;
                    }
                }
            }
        }

        assert_eq!(success_count, 100);
    }

    #[tokio::test]
    async fn test_identity_generation_stress() {
        let mut peer_ids = std::collections::HashSet::new();

        for _ in 0..100 {
            let identity = Identity::generate().expect("Failed to generate identity");
            let peer_id = identity.peer_id();

            assert!(peer_id.starts_with("12D3KooW"));
            assert!(peer_ids.insert(peer_id), "Generated duplicate peer ID");
        }

        assert_eq!(peer_ids.len(), 100);
    }

    #[tokio::test]
    async fn test_room_creation_stress() {
        let mut room_ids = std::collections::HashSet::new();

        for i in 0..100 {
            let config = RoomConfig {
                name: Some(format!("Room {}", i)),
                password: if i % 2 == 0 {
                    Some("password".to_string())
                } else {
                    None
                },
                max_participants: Some((i % 20) + 5),
            };

            let room = Room::new(format!("creator_{}", i), config);

            assert!(
                room_ids.insert(room.id.clone()),
                "Generated duplicate room ID"
            );
            assert_eq!(room.id.len(), 16);
        }

        assert_eq!(room_ids.len(), 100);
    }
}
