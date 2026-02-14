use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SALT_LENGTH: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: Option<String>,
    pub creator_peer_id: String,
    pub password_hash: Option<String>,
    pub max_participants: usize,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub name: Option<String>,
    pub password: Option<String>,
    pub max_participants: Option<usize>,
}

impl Room {
    pub fn new(creator_peer_id: String, config: RoomConfig) -> Self {
        let id = generate_room_id();
        let password_hash = config.password.map(|p| hash_password(&p));

        Self {
            id,
            name: config.name,
            creator_peer_id,
            password_hash,
            max_participants: config.max_participants.unwrap_or(20),
            created_at: current_timestamp(),
        }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        match &self.password_hash {
            Some(hash) => verify_password_hash(password, hash),
            None => true,
        }
    }

    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }

    pub fn share_link(&self) -> String {
        format!("agora://room/{}", self.id)
    }

    pub fn share_link_with_password(&self, password: &str) -> String {
        if self.has_password() {
            format!("agora://room/{}?p={}", self.id, password)
        } else {
            self.share_link()
        }
    }
}

impl RoomConfig {
    pub fn default_public() -> Self {
        Self {
            name: None,
            password: None,
            max_participants: Some(20),
        }
    }

    pub fn named(name: String) -> Self {
        Self {
            name: Some(name),
            ..Self::default_public()
        }
    }

    pub fn private(password: String) -> Self {
        Self {
            password: Some(password),
            ..Self::default_public()
        }
    }
}

pub fn generate_room_id() -> String {
    let random_bytes: [u8; 16] = rand::thread_rng().gen();
    let mut hasher = Sha256::new();
    hasher.update(random_bytes);
    hasher.update(current_timestamp().to_le_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..8])
}

fn hash_password(password: &str) -> String {
    let salt: [u8; SALT_LENGTH] = rand::thread_rng().gen();
    let mut hash = [0u8; 32];

    hkdf::Hkdf::<Sha256>::new(Some(&salt), password.as_bytes())
        .expand(&[0u8; 32], &mut hash)
        .expect("HKDF expand should never fail");

    format!(
        "{}:{}",
        base64::engine::general_purpose::STANDARD.encode(&salt),
        base64::engine::general_purpose::STANDARD.encode(&hash)
    )
}

fn verify_password_hash(password: &str, stored_hash: &str) -> bool {
    let parts: Vec<&str> = stored_hash.split(':').collect();
    if parts.len() != 2 {
        return false;
    }

    let salt = match base64::engine::general_purpose::STANDARD.decode(parts[0]) {
        Ok(s) if s.len() == SALT_LENGTH => s,
        _ => return false,
    };

    let expected_hash = match base64::engine::general_purpose::STANDARD.decode(parts[1]) {
        Ok(h) if h.len() == 32 => h,
        _ => return false,
    };

    let mut computed_hash = [0u8; 32];
    hkdf::Hkdf::<Sha256>::new(Some(&salt), password.as_bytes())
        .expand(&[0u8; 32], &mut computed_hash)
        .expect("HKDF expand should never fail");

    computed_hash.as_slice() == expected_hash.as_slice()
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn parse_room_link(link: &str) -> Option<(String, Option<String>)> {
    let link = link.strip_prefix("agora://room/")?;

    if let Some((id, rest)) = link.split_once('?') {
        let password = rest.strip_prefix("p=").map(urlencoding_decode);
        Some((id.to_string(), password))
    } else {
        Some((link.to_string(), None))
    }
}

fn urlencoding_decode(s: &str) -> String {
    s.replace("%20", " ")
        .replace("%21", "!")
        .replace("%40", "@")
        .replace("%23", "#")
        .replace("%24", "$")
        .replace("%25", "%")
        .replace("%26", "&")
        .replace("%2B", "+")
        .replace("%3D", "=")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_creation() {
        let room = Room::new("peer123".to_string(), RoomConfig::default_public());
        assert!(!room.id.is_empty());
        assert!(room.id.len() == 16);
        assert!(!room.has_password());
    }

    #[test]
    fn test_room_with_password() {
        let room = Room::new(
            "peer123".to_string(),
            RoomConfig::private("secret".to_string()),
        );
        assert!(room.has_password());
        assert!(room.verify_password("secret"));
        assert!(!room.verify_password("wrong"));
        assert!(!room.verify_password(""));
    }

    #[test]
    fn test_password_hash_uniqueness() {
        let hash1 = hash_password("same_password");
        let hash2 = hash_password("same_password");
        assert_ne!(
            hash1, hash2,
            "Same password should produce different hashes due to random salt"
        );

        assert!(verify_password_hash("same_password", &hash1));
        assert!(verify_password_hash("same_password", &hash2));
    }

    #[test]
    fn test_share_link() {
        let room = Room::new("peer123".to_string(), RoomConfig::default_public());
        let link = room.share_link();
        assert!(link.starts_with("agora://room/"));
    }

    #[test]
    fn test_parse_room_link() {
        let (id, pass) = parse_room_link("agora://room/abc123").unwrap();
        assert_eq!(id, "abc123");
        assert!(pass.is_none());

        let (id, pass) = parse_room_link("agora://room/abc123?p=secret").unwrap();
        assert_eq!(id, "abc123");
        assert_eq!(pass, Some("secret".to_string()));
    }
}
