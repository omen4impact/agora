use serde::{Deserialize, Serialize};
use std::io;

pub const PROTOCOL_NAME: &str = "/agora/audio/1.0.0";
pub const PROTOCOL_CONTROL: &str = "/agora/control/1.0.0";

pub const MAX_FRAME_SIZE: usize = 4096;
pub const AUDIO_FRAME_SIZE: usize = 960;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPacket {
    pub sequence: u64,
    pub timestamp: u64,
    pub peer_id: String,
    pub frame: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioPacket {
    pub fn new(sequence: u64, peer_id: String, frame: Vec<f32>) -> Self {
        Self {
            sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            peer_id,
            frame,
            sample_rate: 48000,
            channels: 1,
        }
    }

    pub fn encode(&self) -> io::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn decode(data: &[u8]) -> io::Result<Self> {
        bincode::deserialize(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedAudioPacket {
    pub sequence: u64,
    pub timestamp: u64,
    pub peer_id: String,
    pub encrypted_frame: Vec<u8>,
    pub nonce: [u8; 12],
    pub key_id: u64,
}

impl EncryptedAudioPacket {
    pub fn new(
        sequence: u64,
        peer_id: String,
        encrypted_frame: Vec<u8>,
        nonce: [u8; 12],
        key_id: u64,
    ) -> Self {
        Self {
            sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            peer_id,
            encrypted_frame,
            nonce,
            key_id,
        }
    }

    pub fn from_encrypted_message(
        sequence: u64,
        peer_id: String,
        encrypted_msg: crate::crypto::EncryptedMessage,
        key_id: u64,
    ) -> Self {
        Self {
            sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            peer_id,
            encrypted_frame: encrypted_msg.ciphertext,
            nonce: encrypted_msg.nonce,
            key_id,
        }
    }

    pub fn to_encrypted_message(&self) -> crate::crypto::EncryptedMessage {
        crate::crypto::EncryptedMessage::new(self.nonce, self.encrypted_frame.clone())
    }

    pub fn encode(&self) -> io::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn decode(data: &[u8]) -> io::Result<Self> {
        bincode::deserialize(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMessage {
    pub message_type: ControlMessageType,
    pub peer_id: String,
    pub room_id: Option<String>,
    pub display_name: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessageType {
    JoinRoom { room_id: String },
    LeaveRoom { room_id: String },
    UpdateInfo { display_name: String },
    MuteChanged { is_muted: bool },
    ParticipantList { participants: Vec<ParticipantInfo> },
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub peer_id: String,
    pub display_name: Option<String>,
    pub is_mixer: bool,
    pub is_muted: bool,
    pub latency_ms: u32,
}

impl ControlMessage {
    pub fn new(message_type: ControlMessageType, peer_id: String) -> Self {
        Self {
            message_type,
            peer_id,
            room_id: None,
            display_name: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn join_room(room_id: String, peer_id: String) -> Self {
        Self::new(ControlMessageType::JoinRoom { room_id }, peer_id)
    }

    pub fn leave_room(room_id: String, peer_id: String) -> Self {
        Self::new(ControlMessageType::LeaveRoom { room_id }, peer_id)
    }

    pub fn mute_changed(peer_id: String, is_muted: bool) -> Self {
        Self::new(ControlMessageType::MuteChanged { is_muted }, peer_id)
    }

    pub fn encode(&self) -> io::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn decode(data: &[u8]) -> io::Result<Self> {
        bincode::deserialize(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionQuality {
    Excellent,
    Good,
    Fair,
    Poor,
}

impl ConnectionQuality {
    pub fn from_latency(latency_ms: u32) -> Self {
        match latency_ms {
            0..=50 => ConnectionQuality::Excellent,
            51..=100 => ConnectionQuality::Good,
            101..=200 => ConnectionQuality::Fair,
            _ => ConnectionQuality::Poor,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionQuality::Excellent => "excellent",
            ConnectionQuality::Good => "good",
            ConnectionQuality::Fair => "fair",
            ConnectionQuality::Poor => "poor",
        }
    }
}

pub struct JitterBuffer {
    frames: Vec<Option<AudioPacket>>,
    write_idx: usize,
    read_idx: usize,
}

impl JitterBuffer {
    pub fn new(target_delay_ms: u32, sample_rate: u32) -> Self {
        let buffer_size = (target_delay_ms as f64 / 1000.0 * sample_rate as f64
            / AUDIO_FRAME_SIZE as f64)
            .ceil() as usize;
        let buffer_size = buffer_size.clamp(5, 50);

        Self {
            frames: vec![None; buffer_size],
            write_idx: 0,
            read_idx: 0,
        }
    }

    pub fn push(&mut self, packet: AudioPacket) {
        let len = self.frames.len();
        if len == 0 {
            return;
        }
        let idx = self.write_idx % len;
        self.frames[idx] = Some(packet);
        self.write_idx += 1;
    }

    pub fn pop(&mut self) -> Option<AudioPacket> {
        if self.write_idx <= self.read_idx {
            return None;
        }

        let len = self.frames.len();
        let idx = self.read_idx % len;
        let frame = self.frames[idx].take();
        self.read_idx += 1;
        frame
    }

    pub fn buffer_depth(&self) -> usize {
        self.write_idx.saturating_sub(self.read_idx)
    }

    pub fn clear(&mut self) {
        for frame in &mut self.frames {
            *frame = None;
        }
        self.write_idx = 0;
        self.read_idx = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_packet_encode_decode() {
        let packet = AudioPacket::new(1, "peer123".to_string(), vec![0.5; 960]);
        let encoded = packet.encode().unwrap();
        let decoded = AudioPacket::decode(&encoded).unwrap();

        assert_eq!(packet.sequence, decoded.sequence);
        assert_eq!(packet.peer_id, decoded.peer_id);
        assert_eq!(packet.frame.len(), decoded.frame.len());
    }

    #[test]
    fn test_control_message_encode_decode() {
        let msg = ControlMessage::join_room("room456".to_string(), "peer123".to_string());
        let encoded = msg.encode().unwrap();
        let decoded = ControlMessage::decode(&encoded).unwrap();

        match decoded.message_type {
            ControlMessageType::JoinRoom { room_id } => {
                assert_eq!(room_id, "room456");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_connection_quality() {
        assert_eq!(
            ConnectionQuality::from_latency(30),
            ConnectionQuality::Excellent
        );
        assert_eq!(ConnectionQuality::from_latency(75), ConnectionQuality::Good);
        assert_eq!(
            ConnectionQuality::from_latency(150),
            ConnectionQuality::Fair
        );
        assert_eq!(
            ConnectionQuality::from_latency(300),
            ConnectionQuality::Poor
        );
    }

    #[test]
    fn test_jitter_buffer() {
        let mut buffer = JitterBuffer::new(100, 48000);

        let packet = AudioPacket::new(1, "peer1".to_string(), vec![0.5; 960]);
        buffer.push(packet);

        assert!(buffer.buffer_depth() > 0);

        let retrieved = buffer.pop();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_encrypted_audio_packet_encode_decode() {
        let encrypted =
            EncryptedAudioPacket::new(42, "peer789".to_string(), vec![1, 2, 3, 4, 5], [0u8; 12], 1);

        let encoded = encrypted.encode().unwrap();
        let decoded = EncryptedAudioPacket::decode(&encoded).unwrap();

        assert_eq!(encrypted.sequence, decoded.sequence);
        assert_eq!(encrypted.peer_id, decoded.peer_id);
        assert_eq!(encrypted.encrypted_frame, decoded.encrypted_frame);
        assert_eq!(encrypted.nonce, decoded.nonce);
        assert_eq!(encrypted.key_id, decoded.key_id);
    }

    #[test]
    fn test_encrypted_audio_packet_to_encrypted_message() {
        let encrypted = EncryptedAudioPacket::new(
            1,
            "peer".to_string(),
            vec![10, 20, 30],
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            5,
        );

        let msg = encrypted.to_encrypted_message();

        assert_eq!(msg.nonce, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        assert_eq!(msg.ciphertext, vec![10, 20, 30]);
    }
}
