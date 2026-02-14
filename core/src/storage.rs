use crate::error::{AgoraResult, Error};
use crate::identity::Identity;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const IDENTITY_FILE: &str = "identity.bin";

#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    key_bytes: [u8; 32],
    display_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportedIdentity {
    pub peer_id: String,
    pub key_bytes: String,
    pub display_name: Option<String>,
}

pub struct IdentityStorage {
    config_dir: PathBuf,
}

impl IdentityStorage {
    pub fn new() -> AgoraResult<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| Error::Storage("Cannot determine config directory".to_string()))?
            .join("agora");

        Self::with_path(config_dir)
    }

    pub fn with_path(config_dir: PathBuf) -> AgoraResult<Self> {
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| Error::Storage(format!("Failed to create config directory: {}", e)))?;

        Ok(Self { config_dir })
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    fn identity_path(&self) -> PathBuf {
        self.config_dir.join(IDENTITY_FILE)
    }

    pub fn save(&self, identity: &Identity) -> AgoraResult<()> {
        let path = self.identity_path();

        let stored = StoredIdentity {
            key_bytes: identity.to_bytes(),
            display_name: identity.display_name().map(|s| s.to_string()),
        };

        let bytes = postcard::to_allocvec(&stored)
            .map_err(|e| Error::Storage(format!("Failed to serialize identity: {}", e)))?;

        std::fs::write(&path, &bytes)
            .map_err(|e| Error::Storage(format!("Failed to write identity: {}", e)))?;

        Ok(())
    }

    pub fn load(&self) -> AgoraResult<Identity> {
        let path = self.identity_path();

        if !path.exists() {
            return Err(Error::Storage("No stored identity found".to_string()));
        }

        let bytes = std::fs::read(&path)
            .map_err(|e| Error::Storage(format!("Failed to read identity: {}", e)))?;

        let stored: StoredIdentity = postcard::from_bytes(&bytes)
            .map_err(|e| Error::Storage(format!("Failed to deserialize identity: {}", e)))?;

        let mut identity = Identity::from_bytes(&stored.key_bytes)?;
        if let Some(name) = stored.display_name {
            identity.set_display_name(name);
        }

        Ok(identity)
    }

    pub fn has_stored_identity(&self) -> bool {
        self.identity_path().exists()
    }

    pub fn load_or_create(&self) -> AgoraResult<Identity> {
        if self.has_stored_identity() {
            self.load()
        } else {
            let identity = Identity::generate()?;
            self.save(&identity)?;
            Ok(identity)
        }
    }

    pub fn delete(&self) -> AgoraResult<()> {
        let path = self.identity_path();

        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| Error::Storage(format!("Failed to delete identity: {}", e)))?;
        }

        Ok(())
    }

    pub fn export_to_file(&self, identity: &Identity, path: &Path) -> AgoraResult<()> {
        let exported = ExportedIdentity {
            peer_id: identity.peer_id(),
            key_bytes: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                identity.to_bytes(),
            ),
            display_name: identity.display_name().map(|s| s.to_string()),
        };

        let json = serde_json::to_string_pretty(&exported)
            .map_err(|e| Error::Storage(format!("Failed to serialize: {}", e)))?;

        std::fs::write(path, json)
            .map_err(|e| Error::Storage(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    pub fn import_from_file(&self, path: &Path) -> AgoraResult<Identity> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| Error::Storage(format!("Failed to read file: {}", e)))?;

        let exported: ExportedIdentity = serde_json::from_str(&json)
            .map_err(|e| Error::Storage(format!("Failed to parse: {}", e)))?;

        let key_bytes: [u8; 32] = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &exported.key_bytes,
        )
        .map_err(|e| Error::Storage(format!("Invalid key: {}", e)))?
        .try_into()
        .map_err(|_| Error::Storage("Invalid key length".to_string()))?;

        let mut identity = Identity::from_bytes(&key_bytes)?;
        if let Some(name) = exported.display_name {
            identity.set_display_name(name);
        }

        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load() {
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
    }

    #[test]
    fn test_load_or_create() {
        let dir = tempdir().expect("Failed to create temp dir");
        let storage =
            IdentityStorage::with_path(dir.path().to_path_buf()).expect("Failed to create storage");

        assert!(!storage.has_stored_identity());

        let identity1 = storage.load_or_create().expect("Failed to create identity");
        assert!(storage.has_stored_identity());

        let identity2 = storage.load_or_create().expect("Failed to load identity");
        assert_eq!(identity1.peer_id(), identity2.peer_id());
    }

    #[test]
    fn test_delete() {
        let dir = tempdir().expect("Failed to create temp dir");
        let storage =
            IdentityStorage::with_path(dir.path().to_path_buf()).expect("Failed to create storage");

        let identity = Identity::generate().expect("Failed to generate identity");
        storage.save(&identity).expect("Failed to save identity");
        assert!(storage.has_stored_identity());

        storage.delete().expect("Failed to delete identity");
        assert!(!storage.has_stored_identity());
    }

    #[test]
    fn test_export_and_import() {
        let dir = tempdir().expect("Failed to create temp dir");
        let storage =
            IdentityStorage::with_path(dir.path().to_path_buf()).expect("Failed to create storage");

        let mut identity = Identity::generate().expect("Failed to generate identity");
        identity.set_display_name("Export Test".to_string());

        let export_path = dir.path().join("exported.json");
        storage
            .export_to_file(&identity, &export_path)
            .expect("Failed to export");

        assert!(export_path.exists());

        let imported = storage
            .import_from_file(&export_path)
            .expect("Failed to import");

        assert_eq!(identity.peer_id(), imported.peer_id());
        assert_eq!(identity.display_name(), imported.display_name());
    }
}
