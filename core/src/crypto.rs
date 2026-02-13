use std::collections::HashMap;
use std::time::{Duration, Instant};
use sha2::{Sha256, Digest};
use crate::error::{Error, AgoraResult};

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
            expires_after: Duration::from_secs(3600), // 1 hour default
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
    
    pub fn derive_for_encryption(&self, nonce: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.key);
        hasher.update(nonce.to_le_bytes());
        let result = hasher.finalize();
        let mut derived = [0u8; 32];
        derived.copy_from_slice(&result);
        derived
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedMessage {
    pub nonce: u64,
    pub ciphertext: Vec<u8>,
    pub tag: [u8; 16],
}

impl EncryptedMessage {
    pub fn new(nonce: u64, ciphertext: Vec<u8>, tag: [u8; 16]) -> Self {
        Self { nonce, ciphertext, tag }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.ciphertext.len() + 16);
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        bytes.extend_from_slice(&self.ciphertext);
        bytes.extend_from_slice(&self.tag);
        bytes
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 + 16 {
            return None;
        }
        
        let nonce = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let ciphertext = bytes[8..bytes.len()-16].to_vec();
        let mut tag = [0u8; 16];
        tag.copy_from_slice(&bytes[bytes.len()-16..]);
        
        Some(Self { nonce, ciphertext, tag })
    }
}

pub struct EncryptedChannel {
    session_key: SessionKey,
    peer_public_key: Option<Vec<u8>>,
    send_nonce: u64,
    recv_nonces: HashMap<u64, Instant>,
}

impl EncryptedChannel {
    pub fn new(session_key: SessionKey) -> Self {
        Self {
            session_key,
            peer_public_key: None,
            send_nonce: 0,
            recv_nonces: HashMap::new(),
        }
    }
    
    pub fn with_peer_key(session_key: SessionKey, peer_public_key: Vec<u8>) -> Self {
        Self {
            session_key,
            peer_public_key: Some(peer_public_key),
            send_nonce: 0,
            recv_nonces: HashMap::new(),
        }
    }
    
    pub fn encrypt(&mut self, plaintext: &[u8]) -> AgoraResult<EncryptedMessage> {
        if self.session_key.is_expired() {
            return Err(Error::Crypto("Session key expired".to_string()));
        }
        
        let nonce = self.send_nonce;
        self.send_nonce = self.send_nonce.wrapping_add(1);
        
        // Simplified encryption - in production use proper AEAD (ChaCha20-Poly1305 or AES-GCM)
        let key = self.session_key.derive_for_encryption(nonce);
        let ciphertext = self.xor_encrypt(plaintext, &key);
        
        // Generate authentication tag
        let tag = self.compute_tag(&ciphertext, nonce, &key);
        
        Ok(EncryptedMessage::new(nonce, ciphertext, tag))
    }
    
    pub fn decrypt(&mut self, message: &EncryptedMessage) -> AgoraResult<Vec<u8>> {
        if self.session_key.is_expired() {
            return Err(Error::Crypto("Session key expired".to_string()));
        }
        
        // Check for replay attacks
        if self.recv_nonces.contains_key(&message.nonce) {
            return Err(Error::Crypto("Replay attack detected".to_string()));
        }
        
        // Verify tag
        let key = self.session_key.derive_for_encryption(message.nonce);
        let expected_tag = self.compute_tag(&message.ciphertext, message.nonce, &key);
        
        if message.tag != expected_tag {
            return Err(Error::Crypto("Authentication tag mismatch".to_string()));
        }
        
        // Decrypt
        let plaintext = self.xor_encrypt(&message.ciphertext, &key);
        
        // Record nonce to prevent replay
        self.recv_nonces.insert(message.nonce, Instant::now());
        
        // Clean up old nonces
        self.cleanup_old_nonces();
        
        Ok(plaintext)
    }
    
    fn xor_encrypt(&self, data: &[u8], key: &[u8; 32]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, &byte)| byte ^ key[i % 32])
            .collect()
    }
    
    fn compute_tag(&self, ciphertext: &[u8], nonce: u64, key: &[u8; 32]) -> [u8; 16] {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(ciphertext);
        hasher.update(nonce.to_le_bytes());
        let result = hasher.finalize();
        let mut tag = [0u8; 16];
        tag.copy_from_slice(&result[..16]);
        tag
    }
    
    fn cleanup_old_nonces(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes
        self.recv_nonces.retain(|_, &mut time| time > cutoff);
    }
    
    pub fn rotate_key(&mut self, new_key: SessionKey) {
        self.session_key = new_key;
        self.send_nonce = 0;
        self.recv_nonces.clear();
    }
    
    pub fn is_key_expired(&self) -> bool {
        self.session_key.is_expired()
    }
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
    hex::encode(&hash[..8]).to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_key_expiry() {
        let key = SessionKey::with_expiry(
            [1u8; 32],
            Duration::from_millis(10),
        );
        
        assert!(!key.is_expired());
        std::thread::sleep(Duration::from_millis(20));
        assert!(key.is_expired());
    }
    
    #[test]
    fn test_encrypted_message_roundtrip() {
        let msg = EncryptedMessage::new(42, vec![1, 2, 3, 4, 5], [7u8; 16]);
        let bytes = msg.to_bytes();
        let restored = EncryptedMessage::from_bytes(&bytes).unwrap();
        
        assert_eq!(msg.nonce, restored.nonce);
        assert_eq!(msg.ciphertext, restored.ciphertext);
        assert_eq!(msg.tag, restored.tag);
    }
    
    #[test]
    fn test_encrypt_decrypt() {
        let key = SessionKey::new([42u8; 32]);
        let mut channel = EncryptedChannel::new(key);
        
        let plaintext = b"Hello, Agora!";
        let encrypted = channel.encrypt(plaintext).unwrap();
        let decrypted = channel.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }
    
    #[test]
    fn test_replay_attack_prevention() {
        let key = SessionKey::new([42u8; 32]);
        let mut channel = EncryptedChannel::new(key);
        
        let plaintext = b"Test message";
        let encrypted = channel.encrypt(plaintext).unwrap();
        
        // First decrypt should succeed
        channel.decrypt(&encrypted).unwrap();
        
        // Second decrypt with same nonce should fail
        assert!(channel.decrypt(&encrypted).is_err());
    }
    
    #[test]
    fn test_fingerprint() {
        let fp = compute_fingerprint(&[1, 2, 3, 4, 5]);
        assert_eq!(fp.len(), 16);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
