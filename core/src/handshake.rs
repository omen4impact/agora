use crate::error::{AgoraResult, Error};
use snow::{params::NoiseParams, Builder};

pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
pub const MAX_HANDSHAKE_MESSAGE_SIZE: usize = 65535;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    NotStarted,
    InProgress,
    Complete,
    Failed,
}

#[derive(Debug, Clone)]
pub struct HandshakeMessage {
    pub ephemeral_public_key: Option<[u8; 32]>,
    pub static_public_key: Option<[u8; 32]>,
    pub payload: Vec<u8>,
}

impl HandshakeMessage {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        let flags = {
            let mut f = 0u8;
            if self.ephemeral_public_key.is_some() {
                f |= 0x01;
            }
            if self.static_public_key.is_some() {
                f |= 0x02;
            }
            f
        };

        buf.push(flags);

        if let Some(ref key) = self.ephemeral_public_key {
            buf.extend_from_slice(key);
        }

        if let Some(ref key) = self.static_public_key {
            buf.extend_from_slice(key);
        }

        buf.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.payload);

        buf
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let flags = data[0];
        let mut offset = 1;

        let ephemeral_public_key = if flags & 0x01 != 0 {
            if offset + 32 > data.len() {
                return None;
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&data[offset..offset + 32]);
            offset += 32;
            Some(key)
        } else {
            None
        };

        let static_public_key = if flags & 0x02 != 0 {
            if offset + 32 > data.len() {
                return None;
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&data[offset..offset + 32]);
            offset += 32;
            Some(key)
        } else {
            None
        };

        if offset + 2 > data.len() {
            return None;
        }

        let payload_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;

        if offset + payload_len > data.len() {
            return None;
        }

        let payload = data[offset..offset + payload_len].to_vec();

        Some(Self {
            ephemeral_public_key,
            static_public_key,
            payload,
        })
    }
}

pub struct NoiseSession {
    handshake_state: Option<snow::HandshakeState>,
    transport: Option<snow::TransportState>,
    local_public_key: [u8; 32],
    remote_public_key: Option<[u8; 32]>,
    is_initiator: bool,
}

impl NoiseSession {
    pub fn new_initiator() -> AgoraResult<Self> {
        let params: NoiseParams = NOISE_PATTERN
            .parse()
            .map_err(|e| Error::Crypto(format!("Invalid Noise pattern: {}", e)))?;

        let builder = Builder::new(params);
        let keypair = builder
            .generate_keypair()
            .map_err(|e| Error::Crypto(format!("Failed to generate keypair: {}", e)))?;

        let local_public_key = keypair
            .public
            .as_slice()
            .try_into()
            .map_err(|_| Error::Crypto("Invalid public key length".to_string()))?;

        let handshake = Builder::new(NOISE_PATTERN.parse().unwrap())
            .local_private_key(&keypair.private)
            .build_initiator()
            .map_err(|e| Error::Crypto(format!("Failed to build initiator: {}", e)))?;

        Ok(Self {
            handshake_state: Some(handshake),
            transport: None,
            local_public_key,
            remote_public_key: None,
            is_initiator: true,
        })
    }

    pub fn new_responder() -> AgoraResult<Self> {
        let params: NoiseParams = NOISE_PATTERN
            .parse()
            .map_err(|e| Error::Crypto(format!("Invalid Noise pattern: {}", e)))?;

        let builder = Builder::new(params);
        let keypair = builder
            .generate_keypair()
            .map_err(|e| Error::Crypto(format!("Failed to generate keypair: {}", e)))?;

        let local_public_key = keypair
            .public
            .as_slice()
            .try_into()
            .map_err(|_| Error::Crypto("Invalid public key length".to_string()))?;

        let handshake = Builder::new(NOISE_PATTERN.parse().unwrap())
            .local_private_key(&keypair.private)
            .build_responder()
            .map_err(|e| Error::Crypto(format!("Failed to build responder: {}", e)))?;

        Ok(Self {
            handshake_state: Some(handshake),
            transport: None,
            local_public_key,
            remote_public_key: None,
            is_initiator: false,
        })
    }

    pub fn with_local_key(mut self, private_key: [u8; 32]) -> AgoraResult<Self> {
        let public_key =
            x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(private_key));
        let local_public_key = *public_key.as_bytes();

        let handshake = if self.is_initiator {
            Builder::new(NOISE_PATTERN.parse().unwrap())
                .local_private_key(&private_key)
                .build_initiator()
                .map_err(|e| Error::Crypto(format!("Failed to build initiator: {}", e)))?
        } else {
            Builder::new(NOISE_PATTERN.parse().unwrap())
                .local_private_key(&private_key)
                .build_responder()
                .map_err(|e| Error::Crypto(format!("Failed to build responder: {}", e)))?
        };

        self.handshake_state = Some(handshake);
        self.local_public_key = local_public_key;
        Ok(self)
    }

    pub fn with_remote_public_key(mut self, remote_public_key: [u8; 32]) -> Self {
        self.remote_public_key = Some(remote_public_key);
        self
    }

    pub fn public_key(&self) -> &[u8; 32] {
        &self.local_public_key
    }

    pub fn remote_public_key(&self) -> Option<&[u8; 32]> {
        self.remote_public_key.as_ref()
    }

    pub fn is_handshake_complete(&self) -> bool {
        self.transport.is_some()
    }

    pub fn write_handshake_message(&mut self, payload: &[u8]) -> AgoraResult<Vec<u8>> {
        let handshake = self
            .handshake_state
            .as_mut()
            .ok_or_else(|| Error::Crypto("Handshake already complete".to_string()))?;

        let mut message = vec![0u8; MAX_HANDSHAKE_MESSAGE_SIZE];
        let len = handshake
            .write_message(payload, &mut message)
            .map_err(|e| Error::Crypto(format!("Handshake write failed: {}", e)))?;
        message.truncate(len);

        if handshake.is_handshake_finished() {
            self.finalize_handshake()?;
        }

        Ok(message)
    }

    pub fn read_handshake_message(&mut self, message: &[u8]) -> AgoraResult<Vec<u8>> {
        let handshake = self
            .handshake_state
            .as_mut()
            .ok_or_else(|| Error::Crypto("Handshake already complete".to_string()))?;

        let mut payload = vec![0u8; MAX_HANDSHAKE_MESSAGE_SIZE];
        let len = handshake
            .read_message(message, &mut payload)
            .map_err(|e| Error::Crypto(format!("Handshake read failed: {}", e)))?;
        payload.truncate(len);

        let remote_pub = handshake.get_remote_static();
        if let Some(pub_key) = remote_pub {
            let key: [u8; 32] = pub_key
                .try_into()
                .map_err(|_| Error::Crypto("Invalid remote public key length".to_string()))?;
            self.remote_public_key = Some(key);
        }

        if handshake.is_handshake_finished() {
            self.finalize_handshake()?;
        }

        Ok(payload)
    }

    fn finalize_handshake(&mut self) -> AgoraResult<()> {
        let handshake = self
            .handshake_state
            .take()
            .ok_or_else(|| Error::Crypto("No handshake to finalize".to_string()))?;

        let transport = handshake
            .into_transport_mode()
            .map_err(|e| Error::Crypto(format!("Failed to finalize handshake: {}", e)))?;

        self.transport = Some(transport);

        Ok(())
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> AgoraResult<Vec<u8>> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| Error::Crypto("Handshake not complete".to_string()))?;

        let mut ciphertext = vec![0u8; plaintext.len() + 16];
        let len = transport
            .write_message(plaintext, &mut ciphertext)
            .map_err(|e| Error::Crypto(format!("Encryption failed: {}", e)))?;
        ciphertext.truncate(len);
        Ok(ciphertext)
    }

    pub fn decrypt(&mut self, ciphertext: &[u8]) -> AgoraResult<Vec<u8>> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| Error::Crypto("Handshake not complete".to_string()))?;

        let mut plaintext = vec![0u8; ciphertext.len()];
        let len = transport
            .read_message(ciphertext, &mut plaintext)
            .map_err(|e| Error::Crypto(format!("Decryption failed: {}", e)))?;
        plaintext.truncate(len);
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_session_initiator_responder() {
        let mut initiator = NoiseSession::new_initiator().unwrap();
        let mut responder = NoiseSession::new_responder().unwrap();

        assert!(!initiator.is_handshake_complete());
        assert!(!responder.is_handshake_complete());

        let msg1 = initiator.write_handshake_message(b"hello").unwrap();
        assert!(!initiator.is_handshake_complete());

        let payload1 = responder.read_handshake_message(&msg1).unwrap();
        assert_eq!(payload1, b"hello");

        let msg2 = responder.write_handshake_message(b"world").unwrap();

        let payload2 = initiator.read_handshake_message(&msg2).unwrap();
        assert_eq!(payload2, b"world");

        let msg3 = initiator.write_handshake_message(b"final").unwrap();

        let payload3 = responder.read_handshake_message(&msg3).unwrap();
        assert_eq!(payload3, b"final");

        assert!(initiator.is_handshake_complete());
        assert!(responder.is_handshake_complete());
    }

    #[test]
    fn test_encrypted_communication() {
        let mut initiator = NoiseSession::new_initiator().unwrap();
        let mut responder = NoiseSession::new_responder().unwrap();

        let msg1 = initiator.write_handshake_message(b"").unwrap();
        responder.read_handshake_message(&msg1).unwrap();

        let msg2 = responder.write_handshake_message(b"").unwrap();
        initiator.read_handshake_message(&msg2).unwrap();

        let msg3 = initiator.write_handshake_message(b"").unwrap();
        responder.read_handshake_message(&msg3).unwrap();

        assert!(initiator.is_handshake_complete());
        assert!(responder.is_handshake_complete());

        let plaintext = b"Secret message from Alice";
        let ciphertext = initiator.encrypt(plaintext).unwrap();

        let decrypted = responder.decrypt(&ciphertext).unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);

        let plaintext2 = b"Secret reply from Bob";
        let ciphertext2 = responder.encrypt(plaintext2).unwrap();

        let decrypted2 = initiator.decrypt(&ciphertext2).unwrap();
        assert_eq!(plaintext2.to_vec(), decrypted2);
    }

    #[test]
    fn test_handshake_message_encode_decode() {
        let msg = HandshakeMessage {
            ephemeral_public_key: Some([1u8; 32]),
            static_public_key: Some([2u8; 32]),
            payload: b"test payload".to_vec(),
        };

        let encoded = msg.encode();
        let decoded = HandshakeMessage::decode(&encoded).unwrap();

        assert_eq!(msg.ephemeral_public_key, decoded.ephemeral_public_key);
        assert_eq!(msg.static_public_key, decoded.static_public_key);
        assert_eq!(msg.payload, decoded.payload);
    }

    #[test]
    fn test_handshake_message_no_keys() {
        let msg = HandshakeMessage {
            ephemeral_public_key: None,
            static_public_key: None,
            payload: b"just payload".to_vec(),
        };

        let encoded = msg.encode();
        let decoded = HandshakeMessage::decode(&encoded).unwrap();

        assert!(decoded.ephemeral_public_key.is_none());
        assert!(decoded.static_public_key.is_none());
        assert_eq!(msg.payload, decoded.payload);
    }

    #[test]
    fn test_different_sessions_different_keys() {
        let session1 = NoiseSession::new_initiator().unwrap();
        let session2 = NoiseSession::new_initiator().unwrap();

        assert_ne!(session1.public_key(), session2.public_key());
    }

    #[test]
    fn test_remote_public_key_extraction() {
        let mut initiator = NoiseSession::new_initiator().unwrap();
        let mut responder = NoiseSession::new_responder().unwrap();

        let msg1 = initiator.write_handshake_message(b"").unwrap();
        responder.read_handshake_message(&msg1).unwrap();

        let msg2 = responder.write_handshake_message(b"").unwrap();
        initiator.read_handshake_message(&msg2).unwrap();

        let msg3 = initiator.write_handshake_message(b"").unwrap();
        responder.read_handshake_message(&msg3).unwrap();

        assert!(initiator.remote_public_key().is_some());
        assert!(responder.remote_public_key().is_some());
    }
}
