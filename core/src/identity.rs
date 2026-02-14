use crate::error::{AgoraResult, Error};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use multibase::Base;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct Identity {
    signing_key: SigningKey,
    display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub display_name: Option<String>,
    pub public_key: String,
}

impl Identity {
    pub fn generate() -> AgoraResult<Self> {
        let signing_key = SigningKey::generate(&mut OsRng);
        Ok(Self {
            signing_key,
            display_name: None,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> AgoraResult<Self> {
        let signing_key = SigningKey::from_bytes(
            bytes
                .try_into()
                .map_err(|_| Error::Identity("Invalid key bytes".to_string()))?,
        );
        Ok(Self {
            signing_key,
            display_name: None,
        })
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn peer_id(&self) -> String {
        let verifying_key = self.signing_key.verifying_key();
        let public_key_bytes = verifying_key.as_bytes();
        let mut hasher = Sha256::new();
        hasher.update(public_key_bytes);
        let hash = hasher.finalize();
        format!(
            "12D3KooW{}",
            multibase::encode(Base::Base32Lower, &hash[..20])
        )
    }

    pub fn public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn public_key_base64(&self) -> String {
        multibase::encode(Base::Base64, self.signing_key.verifying_key().as_bytes())
    }

    pub fn set_display_name(&mut self, name: String) {
        self.display_name = Some(name);
    }

    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.signing_key
            .verifying_key()
            .verify(message, signature)
            .is_ok()
    }

    pub fn to_peer_info(&self) -> PeerInfo {
        PeerInfo {
            peer_id: self.peer_id(),
            display_name: self.display_name.clone(),
            public_key: self.public_key_base64(),
        }
    }
}

impl PeerInfo {
    pub fn fingerprint(&self) -> String {
        let bytes = self.public_key.as_bytes();
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        let hex = hex::encode(&hash[..8]);
        hex.chars()
            .collect::<Vec<_>>()
            .chunks(4)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
            .to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = Identity::generate().unwrap();
        assert!(!identity.peer_id().is_empty());
        assert!(identity.peer_id().starts_with("12D3KooW"));
    }

    #[test]
    fn test_identity_serialization() {
        let identity = Identity::generate().unwrap();
        let bytes = identity.to_bytes();
        let restored = Identity::from_bytes(&bytes).unwrap();
        assert_eq!(identity.peer_id(), restored.peer_id());
    }

    #[test]
    fn test_sign_verify() {
        let identity = Identity::generate().unwrap();
        let message = b"Hello, Agora!";
        let signature = identity.sign(message);
        assert!(identity.verify(message, &signature));
    }

    #[test]
    fn test_display_name() {
        let mut identity = Identity::generate().unwrap();
        assert!(identity.display_name().is_none());
        identity.set_display_name("Alice".to_string());
        assert_eq!(identity.display_name(), Some("Alice"));
    }
}
