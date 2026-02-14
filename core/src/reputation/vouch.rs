use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vouch {
    pub id: String,
    pub voucher_peer_id: String,
    pub vouchee_peer_id: String,
    pub stake: f32,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub active: bool,
    pub signature: Option<Vec<u8>>,
}

impl Vouch {
    pub fn new(
        voucher_peer_id: String,
        vouchee_peer_id: String,
        stake: f32,
        validity_days: Option<u64>,
    ) -> Self {
        let now = current_timestamp();
        let id = format!("{}-{}-{}", voucher_peer_id, vouchee_peer_id, now);

        Self {
            id,
            voucher_peer_id,
            vouchee_peer_id,
            stake,
            created_at: now,
            expires_at: validity_days.map(|days| now + days * 86400),
            active: true,
            signature: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        if !self.active {
            return false;
        }

        if let Some(expires) = self.expires_at {
            return current_timestamp() < expires;
        }

        true
    }

    pub fn revoke(&mut self) {
        self.active = false;
    }

    pub fn serialize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

#[derive(Debug, Clone)]
pub struct VouchLimits {
    pub min_voucher_reputation: f32,
    pub min_voucher_uptime_days: f32,
    pub max_vouches_per_voucher: u32,
    pub max_vouches_per_vouchee: u32,
    pub default_stake: f32,
    pub cooldown_days: u64,
    pub validity_days: Option<u64>,
}

impl Default for VouchLimits {
    fn default() -> Self {
        Self {
            min_voucher_reputation: 0.7,
            min_voucher_uptime_days: 7.0,
            max_vouches_per_voucher: 10,
            max_vouches_per_vouchee: 3,
            default_stake: 0.1,
            cooldown_days: 7,
            validity_days: Some(90),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VouchError {
    InsufficientReputation {
        current: f32,
        required: f32,
    },
    InsufficientUptime {
        current_days: f32,
        required_days: f32,
    },
    TooManyVouchesGiven {
        current: u32,
        max: u32,
    },
    TooManyVouchesReceived {
        current: u32,
        max: u32,
    },
    CooldownActive {
        remaining_days: u64,
    },
    SelfVouch,
    AlreadyVouched,
    InvalidVouch,
}

impl std::fmt::Display for VouchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VouchError::InsufficientReputation { current, required } => {
                write!(f, "Insufficient reputation: {} < {}", current, required)
            }
            VouchError::InsufficientUptime {
                current_days,
                required_days,
            } => {
                write!(
                    f,
                    "Insufficient uptime: {} days < {} days",
                    current_days, required_days
                )
            }
            VouchError::TooManyVouchesGiven { current, max } => {
                write!(f, "Too many vouches given: {} >= {}", current, max)
            }
            VouchError::TooManyVouchesReceived { current, max } => {
                write!(f, "Too many vouches received: {} >= {}", current, max)
            }
            VouchError::CooldownActive { remaining_days } => {
                write!(f, "Cooldown active: {} days remaining", remaining_days)
            }
            VouchError::SelfVouch => write!(f, "Cannot vouch for yourself"),
            VouchError::AlreadyVouched => write!(f, "Already vouched for this peer"),
            VouchError::InvalidVouch => write!(f, "Invalid vouch"),
        }
    }
}

pub struct VouchManager {
    limits: VouchLimits,
    vouches: HashMap<String, Vouch>,
    vouches_by_voucher: HashMap<String, Vec<String>>,
    vouches_by_vouchee: HashMap<String, Vec<String>>,
    last_vouch_time: HashMap<String, u64>,
}

impl VouchManager {
    pub fn new(limits: VouchLimits) -> Self {
        Self {
            limits,
            vouches: HashMap::new(),
            vouches_by_voucher: HashMap::new(),
            vouches_by_vouchee: HashMap::new(),
            last_vouch_time: HashMap::new(),
        }
    }

    pub fn can_vouch(
        &self,
        voucher_peer_id: &str,
        vouchee_peer_id: &str,
        voucher_reputation: f32,
        voucher_uptime_days: f32,
    ) -> Result<(), VouchError> {
        if voucher_peer_id == vouchee_peer_id {
            return Err(VouchError::SelfVouch);
        }

        if voucher_reputation < self.limits.min_voucher_reputation {
            return Err(VouchError::InsufficientReputation {
                current: voucher_reputation,
                required: self.limits.min_voucher_reputation,
            });
        }

        if voucher_uptime_days < self.limits.min_voucher_uptime_days {
            return Err(VouchError::InsufficientUptime {
                current_days: voucher_uptime_days,
                required_days: self.limits.min_voucher_uptime_days,
            });
        }

        let vouches_given = self.count_vouches_by_voucher(voucher_peer_id);
        if vouches_given >= self.limits.max_vouches_per_voucher {
            return Err(VouchError::TooManyVouchesGiven {
                current: vouches_given,
                max: self.limits.max_vouches_per_voucher,
            });
        }

        let vouches_received = self.count_vouches_by_vouchee(vouchee_peer_id);
        if vouches_received >= self.limits.max_vouches_per_vouchee {
            return Err(VouchError::TooManyVouchesReceived {
                current: vouches_received,
                max: self.limits.max_vouches_per_vouchee,
            });
        }

        if self.has_vouched(voucher_peer_id, vouchee_peer_id) {
            return Err(VouchError::AlreadyVouched);
        }

        if let Some(&last_time) = self.last_vouch_time.get(voucher_peer_id) {
            let cooldown_secs = self.limits.cooldown_days * 86400;
            let elapsed = current_timestamp() - last_time;
            if elapsed < cooldown_secs {
                return Err(VouchError::CooldownActive {
                    remaining_days: (cooldown_secs - elapsed) / 86400,
                });
            }
        }

        Ok(())
    }

    pub fn create_vouch(
        &mut self,
        voucher_peer_id: String,
        vouchee_peer_id: String,
        stake: Option<f32>,
    ) -> Result<Vouch, VouchError> {
        let stake = stake.unwrap_or(self.limits.default_stake);

        let vouch = Vouch::new(
            voucher_peer_id.clone(),
            vouchee_peer_id.clone(),
            stake,
            self.limits.validity_days,
        );

        let vouch_id = vouch.id.clone();

        self.vouches.insert(vouch_id.clone(), vouch.clone());

        self.vouches_by_voucher
            .entry(voucher_peer_id.clone())
            .or_default()
            .push(vouch_id.clone());

        self.vouches_by_vouchee
            .entry(vouchee_peer_id)
            .or_default()
            .push(vouch_id);

        self.last_vouch_time
            .insert(voucher_peer_id, current_timestamp());

        Ok(vouch)
    }

    pub fn revoke_vouch(&mut self, vouch_id: &str) -> bool {
        if let Some(vouch) = self.vouches.get_mut(vouch_id) {
            vouch.revoke();
            return true;
        }
        false
    }

    pub fn get_vouch(&self, vouch_id: &str) -> Option<&Vouch> {
        self.vouches.get(vouch_id)
    }

    pub fn get_vouches_for_vouchee(&self, vouchee_peer_id: &str) -> Vec<&Vouch> {
        self.vouches_by_vouchee
            .get(vouchee_peer_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.vouches.get(id))
                    .filter(|v| v.is_valid())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_vouches_by_voucher(&self, voucher_peer_id: &str) -> Vec<&Vouch> {
        self.vouches_by_voucher
            .get(voucher_peer_id)
            .map(|ids| ids.iter().filter_map(|id| self.vouches.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn count_vouches_by_voucher(&self, voucher_peer_id: &str) -> u32 {
        self.get_vouches_by_voucher(voucher_peer_id)
            .iter()
            .filter(|v| v.is_valid())
            .count() as u32
    }

    pub fn count_vouches_by_vouchee(&self, vouchee_peer_id: &str) -> u32 {
        self.get_vouches_for_vouchee(vouchee_peer_id).len() as u32
    }

    pub fn has_vouched(&self, voucher_peer_id: &str, vouchee_peer_id: &str) -> bool {
        self.get_vouches_by_voucher(voucher_peer_id)
            .iter()
            .any(|v| v.vouchee_peer_id == vouchee_peer_id && v.is_valid())
    }

    pub fn calculate_vouch_bonus(&self, vouchee_peer_id: &str) -> f32 {
        let vouches = self.get_vouches_for_vouchee(vouchee_peer_id);

        vouches
            .iter()
            .filter(|v| v.is_valid())
            .map(|v| v.stake)
            .sum()
    }

    pub fn apply_penalty(&mut self, vouchee_peer_id: &str) {
        let vouch_ids: Vec<String> = self
            .vouches_by_vouchee
            .get(vouchee_peer_id)
            .cloned()
            .unwrap_or_default();

        for vouch_id in vouch_ids {
            if let Some(vouch) = self.vouches.get_mut(&vouch_id) {
                vouch.revoke();
            }
        }
    }

    pub fn prune_expired(&mut self) {
        let expired: Vec<String> = self
            .vouches
            .iter()
            .filter(|(_, v)| !v.is_valid())
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired {
            if let Some(vouch) = self.vouches.remove(&id) {
                if let Some(voucher_vouches) =
                    self.vouches_by_voucher.get_mut(&vouch.voucher_peer_id)
                {
                    voucher_vouches.retain(|v| v != &id);
                }
                if let Some(vouchee_vouches) =
                    self.vouches_by_vouchee.get_mut(&vouch.vouchee_peer_id)
                {
                    vouchee_vouches.retain(|v| v != &id);
                }
            }
        }
    }

    pub fn total_vouches(&self) -> usize {
        self.vouches.len()
    }

    pub fn active_vouches(&self) -> usize {
        self.vouches.values().filter(|v| v.is_valid()).count()
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vouch_creation() {
        let vouch = Vouch::new(
            "voucher123".to_string(),
            "vouchee456".to_string(),
            0.1,
            Some(90),
        );

        assert!(vouch.is_valid());
        assert_eq!(vouch.stake, 0.1);
        assert!(vouch.expires_at.is_some());
    }

    #[test]
    fn test_vouch_expiration() {
        let mut vouch = Vouch::new("voucher".to_string(), "vouchee".to_string(), 0.1, Some(0));

        vouch.expires_at = Some(current_timestamp() - 1);

        assert!(!vouch.is_valid());
    }

    #[test]
    fn test_vouch_revocation() {
        let mut vouch = Vouch::new("voucher".to_string(), "vouchee".to_string(), 0.1, None);

        assert!(vouch.is_valid());

        vouch.revoke();

        assert!(!vouch.is_valid());
    }

    #[test]
    fn test_vouch_manager_can_vouch() {
        let manager = VouchManager::new(VouchLimits::default());

        let result = manager.can_vouch("voucher", "vouchee", 0.8, 10.0);

        assert!(result.is_ok());
    }

    #[test]
    fn test_vouch_manager_self_vouch() {
        let manager = VouchManager::new(VouchLimits::default());

        let result = manager.can_vouch("peer", "peer", 0.9, 30.0);

        assert!(matches!(result, Err(VouchError::SelfVouch)));
    }

    #[test]
    fn test_vouch_manager_insufficient_reputation() {
        let manager = VouchManager::new(VouchLimits::default());

        let result = manager.can_vouch("voucher", "vouchee", 0.5, 10.0);

        assert!(matches!(
            result,
            Err(VouchError::InsufficientReputation { .. })
        ));
    }

    #[test]
    fn test_vouch_manager_create_vouch() {
        let mut manager = VouchManager::new(VouchLimits::default());

        let vouch = manager
            .create_vouch("voucher".to_string(), "vouchee".to_string(), Some(0.2))
            .unwrap();

        assert_eq!(vouch.stake, 0.2);
        assert_eq!(manager.total_vouches(), 1);
    }

    #[test]
    fn test_vouch_manager_already_vouched() {
        let mut manager = VouchManager::new(VouchLimits::default());

        manager
            .create_vouch("voucher".to_string(), "vouchee".to_string(), None)
            .unwrap();

        let result = manager.can_vouch("voucher", "vouchee", 0.9, 30.0);

        assert!(matches!(result, Err(VouchError::AlreadyVouched)));
    }

    #[test]
    fn test_vouch_manager_max_vouches() {
        let mut limits = VouchLimits::default();
        limits.max_vouches_per_voucher = 2;

        let mut manager = VouchManager::new(limits);

        manager
            .create_vouch("voucher".to_string(), "vouchee1".to_string(), None)
            .unwrap();
        manager
            .create_vouch("voucher".to_string(), "vouchee2".to_string(), None)
            .unwrap();

        let result = manager.can_vouch("voucher", "vouchee3", 0.9, 30.0);

        assert!(matches!(
            result,
            Err(VouchError::TooManyVouchesGiven { .. })
        ));
    }

    #[test]
    fn test_vouch_bonus_calculation() {
        let mut manager = VouchManager::new(VouchLimits::default());

        manager
            .create_vouch("v1".to_string(), "target".to_string(), Some(0.1))
            .unwrap();
        manager
            .create_vouch("v2".to_string(), "target".to_string(), Some(0.15))
            .unwrap();

        let bonus = manager.calculate_vouch_bonus("target");

        assert!((bonus - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_vouch_penalty() {
        let mut manager = VouchManager::new(VouchLimits::default());

        manager
            .create_vouch("v1".to_string(), "bad-node".to_string(), Some(0.1))
            .unwrap();
        manager
            .create_vouch("v2".to_string(), "bad-node".to_string(), Some(0.1))
            .unwrap();

        assert_eq!(manager.count_vouches_by_vouchee("bad-node"), 2);

        manager.apply_penalty("bad-node");

        assert_eq!(manager.active_vouches(), 0);
    }

    #[test]
    fn test_vouch_serialization() {
        let vouch = Vouch::new("voucher".to_string(), "vouchee".to_string(), 0.15, Some(30));

        let serialized = vouch.serialize().unwrap();
        let deserialized: Vouch = Vouch::deserialize(&serialized).unwrap();

        assert_eq!(vouch.id, deserialized.id);
        assert!((vouch.stake - deserialized.stake).abs() < 0.001);
    }
}
