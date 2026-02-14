use crate::error::{AgoraResult, Error};
use base64::Engine;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

pub type Cipher = ChaCha20Poly1305;

#[derive(Debug, Clone)]
pub struct SessionKey {
    key: [u8; 32],
    created_at: Instant,
    expires_after: Duration,
}

impl SessionKey {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key,
            created_at: Instant::now(),
            expires_after: Duration::from_secs(3600),
        }
    }

    pub fn with_expiry(key: [u8; 32], expires_after: Duration) -> Self {
        Self {
            key,
            created_at: Instant::now(),
            expires_after,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.expires_after
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }

    pub fn derive_for_peer(&self, peer_id: &[u8]) -> [u8; 32] {
        let hkdf = Hkdf::<Sha256>::new(None, &self.key);
        let mut derived = [0u8; 32];
        hkdf.expand(peer_id, &mut derived)
            .expect("HKDF expand should never fail with 32-byte output");
        derived
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedMessage {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

impl EncryptedMessage {
    pub fn new(nonce: [u8; 12], ciphertext: Vec<u8>) -> Self {
        Self { nonce, ciphertext }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12 + self.ciphertext.len());
        bytes.extend_from_slice(&self.nonce);
        bytes.extend_from_slice(&self.ciphertext);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 + 16 {
            return None;
        }

        let nonce: [u8; 12] = bytes[0..12].try_into().ok()?;
        let ciphertext = bytes[12..].to_vec();

        Some(Self { nonce, ciphertext })
    }
}

pub struct EncryptedChannel {
    cipher: Cipher,
    session_key: SessionKey,
    peer_public_key: Option<Vec<u8>>,
    send_counter: u64,
    recv_counters: HashMap<u64, Instant>,
}

impl EncryptedChannel {
    pub fn new(session_key: SessionKey) -> Self {
        let cipher = Cipher::new_from_slice(session_key.as_bytes()).expect("Key must be 32 bytes");
        Self {
            cipher,
            session_key,
            peer_public_key: None,
            send_counter: 0,
            recv_counters: HashMap::new(),
        }
    }

    pub fn with_peer_key(session_key: SessionKey, peer_public_key: Vec<u8>) -> Self {
        let cipher = Cipher::new_from_slice(session_key.as_bytes()).expect("Key must be 32 bytes");
        Self {
            cipher,
            session_key,
            peer_public_key: Some(peer_public_key),
            send_counter: 0,
            recv_counters: HashMap::new(),
        }
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> AgoraResult<EncryptedMessage> {
        if self.session_key.is_expired() {
            return Err(Error::Crypto("Session key expired".to_string()));
        }

        let nonce = self.derive_nonce(self.send_counter);
        self.send_counter = self.send_counter.wrapping_add(1);

        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| Error::Crypto(format!("Encryption failed: {}", e)))?;

        Ok(EncryptedMessage::new(nonce.into(), ciphertext))
    }

    pub fn decrypt(&mut self, message: &EncryptedMessage) -> AgoraResult<Vec<u8>> {
        if self.session_key.is_expired() {
            return Err(Error::Crypto("Session key expired".to_string()));
        }

        let counter = self.extract_counter(&message.nonce);

        if self.recv_counters.contains_key(&counter) {
            return Err(Error::Crypto("Replay attack detected".to_string()));
        }

        let nonce = Nonce::from_slice(&message.nonce);
        let plaintext = self
            .cipher
            .decrypt(nonce, message.ciphertext.as_slice())
            .map_err(|e| Error::Crypto(format!("Decryption failed: {}", e)))?;

        self.recv_counters.insert(counter, Instant::now());
        self.cleanup_old_counters();

        Ok(plaintext)
    }

    fn derive_nonce(&self, counter: u64) -> Nonce {
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&counter.to_le_bytes());
        Nonce::from(nonce_bytes)
    }

    fn extract_counter(&self, nonce: &[u8; 12]) -> u64 {
        let mut counter_bytes = [0u8; 8];
        counter_bytes.copy_from_slice(&nonce[4..12]);
        u64::from_le_bytes(counter_bytes)
    }

    fn cleanup_old_counters(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(300);
        self.recv_counters.retain(|_, &mut time| time > cutoff);
    }

    pub fn rotate_key(&mut self, new_key: SessionKey) {
        self.cipher = Cipher::new_from_slice(new_key.as_bytes()).expect("Key must be 32 bytes");
        self.session_key = new_key;
        self.send_counter = 0;
        self.recv_counters.clear();
    }

    pub fn is_key_expired(&self) -> bool {
        self.session_key.is_expired()
    }

    pub fn peer_public_key(&self) -> Option<&[u8]> {
        self.peer_public_key.as_deref()
    }
}

pub struct KeyExchange {
    secret: Option<EphemeralSecret>,
    public_key: PublicKey,
}

impl KeyExchange {
    pub fn new() -> Self {
        use rand::rngs::OsRng;
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public_key = PublicKey::from(&secret);
        Self {
            secret: Some(secret),
            public_key,
        }
    }

    pub fn public_key(&self) -> &[u8; 32] {
        self.public_key.as_bytes()
    }

    pub fn public_key_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.public_key.as_bytes())
    }

    pub fn compute_shared_secret(&mut self, peer_public: &[u8; 32]) -> AgoraResult<SharedSecret> {
        let secret = self
            .secret
            .take()
            .ok_or_else(|| Error::Crypto("Key exchange already completed".to_string()))?;

        let peer_public = PublicKey::from(*peer_public);
        Ok(secret.diffie_hellman(&peer_public))
    }
}

impl Default for KeyExchange {
    fn default() -> Self {
        Self::new()
    }
}

pub fn derive_session_key_from_shared_secret(
    shared_secret: &SharedSecret,
    room_id: &str,
) -> SessionKey {
    let hkdf = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
    let mut key = [0u8; 32];
    hkdf.expand(room_id.as_bytes(), &mut key)
        .expect("HKDF expand should never fail");
    SessionKey::new(key)
}

pub fn generate_ephemeral_key() -> [u8; 32] {
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

pub fn derive_session_key(
    local_private: &[u8; 32],
    remote_public: &[u8; 32],
    room_id: &str,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(local_private);
    hasher.update(remote_public);
    hasher.update(room_id.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

pub fn compute_fingerprint(public_key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    let hash = hasher.finalize();
    let hex = hex::encode(&hash[..8]);
    format_fingerprint(&hex)
}

fn format_fingerprint(hex: &str) -> String {
    hex.chars()
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(":")
        .to_uppercase()
}

pub const DEFAULT_KEY_ROTATION_INTERVAL: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone)]
pub struct KeyRotationEvent {
    pub room_id: String,
    pub new_key_id: u64,
    pub previous_key_id: Option<u64>,
}

pub struct SessionKeyManager {
    rooms: HashMap<String, RoomKeys>,
    rotation_interval: Duration,
}

struct RoomKeys {
    current_key: SessionKeyInfo,
    previous_key: Option<SessionKeyInfo>,
    next_rotation: Instant,
}

struct SessionKeyInfo {
    id: u64,
    key: SessionKey,
    cipher: Cipher,
    send_counter: u64,
    recv_counters: HashMap<u64, Instant>,
}

impl SessionKeyInfo {
    fn new(id: u64, key: SessionKey) -> Self {
        let cipher = Cipher::new_from_slice(key.as_bytes()).expect("Key must be 32 bytes");
        Self {
            id,
            key,
            cipher,
            send_counter: 0,
            recv_counters: HashMap::new(),
        }
    }

    fn encrypt(&mut self, plaintext: &[u8]) -> AgoraResult<EncryptedMessage> {
        if self.key.is_expired() {
            return Err(Error::Crypto("Session key expired".to_string()));
        }

        let nonce = self.derive_nonce(self.send_counter);
        self.send_counter = self.send_counter.wrapping_add(1);

        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| Error::Crypto(format!("Encryption failed: {}", e)))?;

        Ok(EncryptedMessage::new(nonce.into(), ciphertext))
    }

    fn decrypt(&mut self, message: &EncryptedMessage) -> AgoraResult<Vec<u8>> {
        if self.key.is_expired() {
            return Err(Error::Crypto("Session key expired".to_string()));
        }

        let counter = self.extract_counter(&message.nonce);

        if self.recv_counters.contains_key(&counter) {
            return Err(Error::Crypto("Replay attack detected".to_string()));
        }

        let nonce = Nonce::from_slice(&message.nonce);
        let plaintext = self
            .cipher
            .decrypt(nonce, message.ciphertext.as_slice())
            .map_err(|e| Error::Crypto(format!("Decryption failed: {}", e)))?;

        self.recv_counters.insert(counter, Instant::now());
        self.cleanup_old_counters();

        Ok(plaintext)
    }

    fn derive_nonce(&self, counter: u64) -> Nonce {
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&counter.to_le_bytes());
        Nonce::from(nonce_bytes)
    }

    fn extract_counter(&self, nonce: &[u8; 12]) -> u64 {
        let mut counter_bytes = [0u8; 8];
        counter_bytes.copy_from_slice(&nonce[4..12]);
        u64::from_le_bytes(counter_bytes)
    }

    fn cleanup_old_counters(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(300);
        self.recv_counters.retain(|_, &mut time| time > cutoff);
    }

    fn is_expired(&self) -> bool {
        self.key.is_expired()
    }
}

impl SessionKeyManager {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            rotation_interval: DEFAULT_KEY_ROTATION_INTERVAL,
        }
    }

    pub fn with_rotation_interval(mut self, interval: Duration) -> Self {
        self.rotation_interval = interval;
        self
    }

    pub fn create_room(&mut self, room_id: &str, initial_key: SessionKey) -> u64 {
        let key_info = SessionKeyInfo::new(1, initial_key);
        let next_rotation = Instant::now() + self.rotation_interval;

        self.rooms.insert(
            room_id.to_string(),
            RoomKeys {
                current_key: key_info,
                previous_key: None,
                next_rotation,
            },
        );

        tracing::info!("Created session for room {} with key ID 1", room_id);
        1
    }

    pub fn remove_room(&mut self, room_id: &str) {
        self.rooms.remove(room_id);
        tracing::debug!("Removed session for room {}", room_id);
    }

    pub fn encrypt(&mut self, room_id: &str, plaintext: &[u8]) -> AgoraResult<EncryptedMessage> {
        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| Error::Crypto(format!("Room {} not found", room_id)))?;

        room.current_key.encrypt(plaintext)
    }

    pub fn decrypt(&mut self, room_id: &str, message: &EncryptedMessage) -> AgoraResult<Vec<u8>> {
        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| Error::Crypto(format!("Room {} not found", room_id)))?;

        if let Ok(plaintext) = room.current_key.decrypt(message) {
            return Ok(plaintext);
        }

        if let Some(ref mut prev_key) = room.previous_key {
            if !prev_key.is_expired() {
                return prev_key.decrypt(message);
            }
        }

        Err(Error::Crypto("Decryption failed with all keys".to_string()))
    }

    pub fn check_rotation(&mut self) -> Vec<KeyRotationEvent> {
        let mut events = Vec::new();
        let now = Instant::now();
        let rotation_interval = self.rotation_interval;

        let rooms_to_rotate: Vec<String> = self
            .rooms
            .iter()
            .filter(|(_, room)| now >= room.next_rotation)
            .map(|(room_id, _)| room_id.clone())
            .collect();

        for room_id in rooms_to_rotate {
            if let Some(room) = self.rooms.get_mut(&room_id) {
                if let Some(event) = Self::rotate_room_key(&room_id, room, rotation_interval) {
                    events.push(event);
                }
            }
        }

        events
    }

    fn rotate_room_key(
        room_id: &str,
        room: &mut RoomKeys,
        rotation_interval: Duration,
    ) -> Option<KeyRotationEvent> {
        let new_key_id = room.current_key.id + 1;
        let new_key = Self::derive_new_key(&room.current_key.key, new_key_id, rotation_interval);
        let new_key_info = SessionKeyInfo::new(new_key_id, new_key);

        let previous_id = room.current_key.id;

        let old_key = std::mem::replace(&mut room.current_key, new_key_info);
        room.previous_key = Some(old_key);
        room.next_rotation = Instant::now() + rotation_interval;

        tracing::info!(
            "Rotated key for room {}: {} -> {}",
            room_id,
            previous_id,
            new_key_id
        );

        Some(KeyRotationEvent {
            room_id: room_id.to_string(),
            new_key_id,
            previous_key_id: Some(previous_id),
        })
    }

    fn derive_new_key(
        current_key: &SessionKey,
        new_id: u64,
        rotation_interval: Duration,
    ) -> SessionKey {
        let hkdf = Hkdf::<Sha256>::new(None, current_key.as_bytes());
        let mut new_key_bytes = [0u8; 32];
        hkdf.expand(&new_id.to_le_bytes(), &mut new_key_bytes)
            .expect("HKDF expand should never fail");
        SessionKey::with_expiry(new_key_bytes, rotation_interval)
    }

    pub fn rotate_key_now(&mut self, room_id: &str) -> AgoraResult<KeyRotationEvent> {
        let rotation_interval = self.rotation_interval;
        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| Error::Crypto(format!("Room {} not found", room_id)))?;

        Self::rotate_room_key(room_id, room, rotation_interval)
            .ok_or_else(|| Error::Crypto("Failed to rotate key".to_string()))
    }

    pub fn get_current_key_id(&self, room_id: &str) -> Option<u64> {
        self.rooms.get(room_id).map(|r| r.current_key.id)
    }

    pub fn time_until_rotation(&self, room_id: &str) -> Option<Duration> {
        self.rooms.get(room_id).map(|r| {
            let remaining = r.next_rotation.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                Duration::ZERO
            } else {
                remaining
            }
        })
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    pub fn has_room(&self, room_id: &str) -> bool {
        self.rooms.contains_key(room_id)
    }
}

impl Default for SessionKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SecureAudioChannel {
    key_manager: SessionKeyManager,
}

impl SecureAudioChannel {
    pub fn new() -> Self {
        Self {
            key_manager: SessionKeyManager::new(),
        }
    }

    pub fn with_key_manager(key_manager: SessionKeyManager) -> Self {
        Self { key_manager }
    }

    pub fn create_room(&mut self, room_id: &str, initial_key: SessionKey) -> u64 {
        self.key_manager.create_room(room_id, initial_key)
    }

    pub fn remove_room(&mut self, room_id: &str) {
        self.key_manager.remove_room(room_id);
    }

    pub fn encrypt_packet(
        &mut self,
        room_id: &str,
        packet: &crate::protocol::AudioPacket,
    ) -> AgoraResult<crate::protocol::EncryptedAudioPacket> {
        let plaintext = packet
            .encode()
            .map_err(|e| Error::Crypto(format!("Failed to encode packet: {}", e)))?;

        let encrypted = self.key_manager.encrypt(room_id, &plaintext)?;
        let key_id = self
            .key_manager
            .get_current_key_id(room_id)
            .ok_or_else(|| Error::Crypto("Room not found".to_string()))?;

        Ok(
            crate::protocol::EncryptedAudioPacket::from_encrypted_message(
                packet.sequence,
                packet.peer_id.clone(),
                encrypted,
                key_id,
            ),
        )
    }

    pub fn decrypt_packet(
        &mut self,
        room_id: &str,
        encrypted_packet: &crate::protocol::EncryptedAudioPacket,
    ) -> AgoraResult<crate::protocol::AudioPacket> {
        let encrypted_msg = encrypted_packet.to_encrypted_message();
        let plaintext = self.key_manager.decrypt(room_id, &encrypted_msg)?;

        crate::protocol::AudioPacket::decode(&plaintext)
            .map_err(|e| Error::Crypto(format!("Failed to decode packet: {}", e)))
    }

    pub fn check_rotation(&mut self) -> Vec<KeyRotationEvent> {
        self.key_manager.check_rotation()
    }

    pub fn rotate_key_now(&mut self, room_id: &str) -> AgoraResult<KeyRotationEvent> {
        self.key_manager.rotate_key_now(room_id)
    }

    pub fn get_current_key_id(&self, room_id: &str) -> Option<u64> {
        self.key_manager.get_current_key_id(room_id)
    }

    pub fn has_room(&self, room_id: &str) -> bool {
        self.key_manager.has_room(room_id)
    }

    pub fn room_count(&self) -> usize {
        self.key_manager.room_count()
    }
}

impl Default for SecureAudioChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_key_expiry() {
        let key = SessionKey::with_expiry([1u8; 32], Duration::from_millis(10));

        assert!(!key.is_expired());
        std::thread::sleep(Duration::from_millis(20));
        assert!(key.is_expired());
    }

    #[test]
    fn test_encrypted_message_roundtrip() {
        let nonce = [1u8; 12];
        let ciphertext = vec![2u8; 32];
        let msg = EncryptedMessage::new(nonce, ciphertext.clone());
        let bytes = msg.to_bytes();
        let restored = EncryptedMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg.nonce, restored.nonce);
        assert_eq!(msg.ciphertext, restored.ciphertext);
    }

    #[test]
    fn test_encrypt_decrypt_chacha20() {
        let key = SessionKey::new([42u8; 32]);
        let mut channel = EncryptedChannel::new(key);

        let plaintext = b"Hello, Agora!";
        let encrypted = channel.encrypt(plaintext).unwrap();
        let decrypted = channel.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_different_ciphertexts() {
        let key = SessionKey::new([42u8; 32]);
        let mut channel = EncryptedChannel::new(key);

        let plaintext = b"Same message";
        let enc1 = channel.encrypt(plaintext).unwrap();
        let enc2 = channel.encrypt(plaintext).unwrap();

        assert_ne!(enc1.ciphertext, enc2.ciphertext);
    }

    #[test]
    fn test_replay_attack_prevention() {
        let key = SessionKey::new([42u8; 32]);
        let mut channel = EncryptedChannel::new(key);

        let plaintext = b"Test message";
        let encrypted = channel.encrypt(plaintext).unwrap();

        channel.decrypt(&encrypted).unwrap();

        assert!(channel.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_key_exchange() {
        let mut alice = KeyExchange::new();
        let mut bob = KeyExchange::new();

        let alice_public = *alice.public_key();
        let bob_public = *bob.public_key();

        let alice_shared = alice.compute_shared_secret(&bob_public).unwrap();
        let bob_shared = bob.compute_shared_secret(&alice_public).unwrap();

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_session_key_from_shared_secret() {
        let mut alice = KeyExchange::new();
        let mut bob = KeyExchange::new();

        let alice_public = *alice.public_key();
        let bob_public = *bob.public_key();

        let alice_shared = alice.compute_shared_secret(&bob_public).unwrap();
        let bob_shared = bob.compute_shared_secret(&alice_public).unwrap();

        let alice_key = derive_session_key_from_shared_secret(&alice_shared, "test-room");
        let bob_key = derive_session_key_from_shared_secret(&bob_shared, "test-room");

        assert_eq!(alice_key.as_bytes(), bob_key.as_bytes());
    }

    #[test]
    fn test_different_rooms_different_keys() {
        let mut alice = KeyExchange::new();
        let bob = KeyExchange::new();

        let bob_public = *bob.public_key();

        let alice_shared = alice.compute_shared_secret(&bob_public).unwrap();

        let key1 = derive_session_key_from_shared_secret(&alice_shared, "room1");
        let key2 = derive_session_key_from_shared_secret(&alice_shared, "room2");

        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_fingerprint() {
        let fp = compute_fingerprint(&[1, 2, 3, 4, 5]);
        assert!(fp.contains(':'));
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit() || c == ':'));
    }

    #[test]
    fn test_key_rotation() {
        let key1 = SessionKey::new([1u8; 32]);
        let key2 = SessionKey::new([2u8; 32]);

        let mut channel = EncryptedChannel::new(key1);

        let plaintext = b"Before rotation";
        let enc1 = channel.encrypt(plaintext).unwrap();
        channel.decrypt(&enc1).unwrap();

        channel.rotate_key(key2);

        let plaintext2 = b"After rotation";
        let enc2 = channel.encrypt(plaintext2).unwrap();
        let dec2 = channel.decrypt(&enc2).unwrap();
        assert_eq!(plaintext2.to_vec(), dec2);
    }

    #[test]
    fn test_large_message() {
        let key = SessionKey::new([42u8; 32]);
        let mut channel = EncryptedChannel::new(key);

        let plaintext = vec![0u8; 65536];
        let encrypted = channel.encrypt(&plaintext).unwrap();
        let decrypted = channel.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_session_key_manager_create_room() {
        let mut manager = SessionKeyManager::new();
        let key = SessionKey::new([1u8; 32]);

        let key_id = manager.create_room("test-room", key);
        assert_eq!(key_id, 1);
        assert!(manager.has_room("test-room"));
        assert_eq!(manager.room_count(), 1);
    }

    #[test]
    fn test_session_key_manager_encrypt_decrypt() {
        let mut manager = SessionKeyManager::new();
        let key = SessionKey::new([1u8; 32]);
        manager.create_room("test-room", key);

        let plaintext = b"Hello, secure world!";
        let encrypted = manager.encrypt("test-room", plaintext).unwrap();
        let decrypted = manager.decrypt("test-room", &encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_session_key_manager_key_rotation() {
        let mut manager =
            SessionKeyManager::new().with_rotation_interval(Duration::from_millis(50));

        let key = SessionKey::with_expiry([1u8; 32], Duration::from_secs(10));
        manager.create_room("test-room", key);

        let plaintext = b"Before rotation";
        let encrypted = manager.encrypt("test-room", plaintext).unwrap();

        std::thread::sleep(Duration::from_millis(60));

        let events = manager.check_rotation();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].room_id, "test-room");
        assert_eq!(events[0].new_key_id, 2);

        let decrypted = manager.decrypt("test-room", &encrypted).unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_session_key_manager_manual_rotation() {
        let mut manager = SessionKeyManager::new();
        let key = SessionKey::new([1u8; 32]);
        manager.create_room("test-room", key);

        assert_eq!(manager.get_current_key_id("test-room"), Some(1));

        let event = manager.rotate_key_now("test-room").unwrap();
        assert_eq!(event.new_key_id, 2);
        assert_eq!(event.previous_key_id, Some(1));

        assert_eq!(manager.get_current_key_id("test-room"), Some(2));
    }

    #[test]
    fn test_session_key_manager_multiple_rooms() {
        let mut manager = SessionKeyManager::new();

        manager.create_room("room1", SessionKey::new([1u8; 32]));
        manager.create_room("room2", SessionKey::new([2u8; 32]));
        manager.create_room("room3", SessionKey::new([3u8; 32]));

        assert_eq!(manager.room_count(), 3);

        let enc1 = manager.encrypt("room1", b"msg1").unwrap();
        let enc2 = manager.encrypt("room2", b"msg2").unwrap();
        let enc3 = manager.encrypt("room3", b"msg3").unwrap();

        assert_ne!(enc1.ciphertext, enc2.ciphertext);
        assert_ne!(enc2.ciphertext, enc3.ciphertext);

        manager.remove_room("room2");
        assert_eq!(manager.room_count(), 2);
        assert!(!manager.has_room("room2"));
    }

    #[test]
    fn test_session_key_manager_replay_protection() {
        let mut manager = SessionKeyManager::new();
        let key = SessionKey::new([1u8; 32]);
        manager.create_room("test-room", key);

        let plaintext = b"Secret message";
        let encrypted = manager.encrypt("test-room", plaintext).unwrap();

        manager.decrypt("test-room", &encrypted).unwrap();

        assert!(manager.decrypt("test-room", &encrypted).is_err());
    }

    #[test]
    fn test_session_key_manager_previous_key_decryption() {
        let mut manager = SessionKeyManager::new();
        let key = SessionKey::new([1u8; 32]);
        manager.create_room("test-room", key);

        let plaintext = b"Old key message";
        let encrypted = manager.encrypt("test-room", plaintext).unwrap();

        manager.rotate_key_now("test-room").unwrap();

        let decrypted = manager.decrypt("test-room", &encrypted).unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_time_until_rotation() {
        let mut manager = SessionKeyManager::new().with_rotation_interval(Duration::from_secs(60));

        let key = SessionKey::new([1u8; 32]);
        manager.create_room("test-room", key);

        let time = manager.time_until_rotation("test-room").unwrap();
        assert!(time <= Duration::from_secs(60));
        assert!(time > Duration::from_secs(58));
    }

    #[test]
    fn test_secure_audio_channel_encrypt_decrypt() {
        use crate::protocol::AudioPacket;

        let mut channel = SecureAudioChannel::new();
        let key = SessionKey::new([1u8; 32]);
        channel.create_room("test-room", key);

        let packet = AudioPacket::new(1, "peer123".to_string(), vec![0.5; 960]);
        let encrypted = channel.encrypt_packet("test-room", &packet).unwrap();

        assert_eq!(encrypted.sequence, packet.sequence);
        assert_eq!(encrypted.peer_id, packet.peer_id);
        assert!(!encrypted.encrypted_frame.is_empty());

        let decrypted = channel.decrypt_packet("test-room", &encrypted).unwrap();
        assert_eq!(decrypted.sequence, packet.sequence);
        assert_eq!(decrypted.peer_id, packet.peer_id);
        assert_eq!(decrypted.frame, packet.frame);
    }

    #[test]
    fn test_secure_audio_channel_key_rotation() {
        use crate::protocol::AudioPacket;

        let mut channel = SecureAudioChannel::new();
        let key = SessionKey::new([1u8; 32]);
        channel.create_room("test-room", key);

        let packet = AudioPacket::new(1, "peer123".to_string(), vec![0.5; 960]);
        let encrypted = channel.encrypt_packet("test-room", &packet).unwrap();
        assert_eq!(encrypted.key_id, 1);

        channel.rotate_key_now("test-room").unwrap();

        let packet2 = AudioPacket::new(2, "peer123".to_string(), vec![0.3; 960]);
        let encrypted2 = channel.encrypt_packet("test-room", &packet2).unwrap();
        assert_eq!(encrypted2.key_id, 2);

        let decrypted = channel.decrypt_packet("test-room", &encrypted).unwrap();
        assert_eq!(decrypted.frame, packet.frame);

        let decrypted2 = channel.decrypt_packet("test-room", &encrypted2).unwrap();
        assert_eq!(decrypted2.frame, packet2.frame);
    }

    #[test]
    fn test_secure_audio_channel_multiple_rooms() {
        use crate::protocol::AudioPacket;

        let mut channel = SecureAudioChannel::new();

        channel.create_room("room1", SessionKey::new([1u8; 32]));
        channel.create_room("room2", SessionKey::new([2u8; 32]));

        let packet1 = AudioPacket::new(1, "peer1".to_string(), vec![0.5; 960]);
        let packet2 = AudioPacket::new(1, "peer2".to_string(), vec![0.3; 960]);

        let enc1 = channel.encrypt_packet("room1", &packet1).unwrap();
        let enc2 = channel.encrypt_packet("room2", &packet2).unwrap();

        assert_ne!(enc1.encrypted_frame, enc2.encrypted_frame);

        let dec1 = channel.decrypt_packet("room1", &enc1).unwrap();
        let dec2 = channel.decrypt_packet("room2", &enc2).unwrap();

        assert_eq!(dec1.peer_id, "peer1");
        assert_eq!(dec2.peer_id, "peer2");
    }

    #[test]
    fn test_secure_audio_channel_serialization() {
        use crate::protocol::{AudioPacket, EncryptedAudioPacket};

        let mut channel = SecureAudioChannel::new();
        let key = SessionKey::new([1u8; 32]);
        channel.create_room("test-room", key);

        let packet = AudioPacket::new(42, "peer456".to_string(), vec![0.7; 480]);
        let encrypted = channel.encrypt_packet("test-room", &packet).unwrap();

        let encoded = encrypted.encode().unwrap();
        let decoded = EncryptedAudioPacket::decode(&encoded).unwrap();

        assert_eq!(encrypted.sequence, decoded.sequence);
        assert_eq!(encrypted.encrypted_frame, decoded.encrypted_frame);
        assert_eq!(encrypted.nonce, decoded.nonce);
        assert_eq!(encrypted.key_id, decoded.key_id);

        let decrypted = channel.decrypt_packet("test-room", &decoded).unwrap();
        assert_eq!(decrypted.frame, packet.frame);
    }
}
