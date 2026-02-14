use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub challenger_peer_id: String,
    pub target_peer_id: String,
    pub challenge_type: ChallengeType,
    pub data_hash: String,
    pub nonce: u64,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeType {
    Bandwidth { size_bytes: u64, max_time_ms: u32 },
    Latency { target_ms: u32 },
    Compute { difficulty: u8 },
}

impl ChallengeType {
    pub fn bandwidth_1mb() -> Self {
        Self::Bandwidth {
            size_bytes: 1_048_576,
            max_time_ms: 5000,
        }
    }

    pub fn bandwidth_10mb() -> Self {
        Self::Bandwidth {
            size_bytes: 10_485_760,
            max_time_ms: 10000,
        }
    }

    pub fn latency_default() -> Self {
        Self::Latency { target_ms: 100 }
    }
}

impl Challenge {
    pub fn new(
        challenger_peer_id: String,
        target_peer_id: String,
        challenge_type: ChallengeType,
        timeout_secs: u64,
    ) -> Self {
        let now = current_timestamp();
        let nonce = rand_nonce();
        let data_hash = generate_data_hash(&challenger_peer_id, &target_peer_id, nonce);

        let id = format!("{}-{}-{}", challenger_peer_id, target_peer_id, nonce);

        Self {
            id,
            challenger_peer_id,
            target_peer_id,
            challenge_type,
            data_hash,
            nonce,
            created_at: now,
            expires_at: now + timeout_secs,
        }
    }

    pub fn is_expired(&self) -> bool {
        current_timestamp() >= self.expires_at
    }

    pub fn time_remaining_secs(&self) -> u64 {
        let now = current_timestamp();
        if now >= self.expires_at {
            0
        } else {
            self.expires_at - now
        }
    }

    pub fn generate_response_data(&self) -> Vec<u8> {
        let mut data = Vec::new();

        if let ChallengeType::Bandwidth { size_bytes, .. } = self.challenge_type {
            let mut hasher = Sha256::new();
            hasher.update(self.data_hash.as_bytes());

            let seed = hasher.finalize();

            data.reserve(size_bytes as usize);
            for i in 0..size_bytes as usize {
                data.push(seed[i % seed.len()]);
            }
        }

        data
    }

    pub fn serialize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge_id: String,
    pub responder_peer_id: String,
    pub result: ChallengeResult,
    pub completed_at: u64,
    pub signature: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChallengeResult {
    Success {
        bandwidth_mbps: f32,
        latency_ms: u32,
    },
    Failed {
        reason: String,
    },
    Timeout,
}

impl ChallengeResponse {
    pub fn success(
        challenge_id: String,
        responder_peer_id: String,
        bandwidth_mbps: f32,
        latency_ms: u32,
    ) -> Self {
        Self {
            challenge_id,
            responder_peer_id,
            result: ChallengeResult::Success {
                bandwidth_mbps,
                latency_ms,
            },
            completed_at: current_timestamp(),
            signature: None,
        }
    }

    pub fn failed(challenge_id: String, responder_peer_id: String, reason: String) -> Self {
        Self {
            challenge_id,
            responder_peer_id,
            result: ChallengeResult::Failed { reason },
            completed_at: current_timestamp(),
            signature: None,
        }
    }

    pub fn timeout(challenge_id: String, responder_peer_id: String) -> Self {
        Self {
            challenge_id,
            responder_peer_id,
            result: ChallengeResult::Timeout,
            completed_at: current_timestamp(),
            signature: None,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self.result, ChallengeResult::Success { .. })
    }
}

pub struct ChallengeVerifier {
    max_bandwidth_tolerance: f32,
    min_bandwidth_mbps: f32,
}

impl Default for ChallengeVerifier {
    fn default() -> Self {
        Self {
            max_bandwidth_tolerance: 0.2,
            min_bandwidth_mbps: 0.1,
        }
    }
}

impl ChallengeVerifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn verify_response(&self, challenge: &Challenge, response: &ChallengeResponse) -> bool {
        if response.challenge_id != challenge.id {
            return false;
        }

        if response.responder_peer_id != challenge.target_peer_id {
            return false;
        }

        if challenge.is_expired() {
            return false;
        }

        match &response.result {
            ChallengeResult::Success { bandwidth_mbps, .. } => {
                if *bandwidth_mbps < self.min_bandwidth_mbps {
                    return false;
                }

                if let ChallengeType::Bandwidth {
                    size_bytes,
                    max_time_ms,
                } = challenge.challenge_type
                {
                    let expected_mbps =
                        (size_bytes as f32 / 1_048_576.0) / (max_time_ms as f32 / 1000.0);
                    let tolerance = expected_mbps * self.max_bandwidth_tolerance;

                    if *bandwidth_mbps > expected_mbps + tolerance {
                        return false;
                    }
                }

                true
            }
            ChallengeResult::Failed { .. } | ChallengeResult::Timeout => false,
        }
    }

    pub fn measure_bandwidth(data: &[u8], elapsed_ms: u64) -> f32 {
        if elapsed_ms == 0 {
            return 0.0;
        }

        let megabytes = data.len() as f32 / 1_048_576.0;
        let seconds = elapsed_ms as f32 / 1000.0;

        megabytes / seconds
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn rand_nonce() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u64(current_timestamp());
    hasher.finish()
}

fn generate_data_hash(challenger: &str, target: &str, nonce: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(challenger.as_bytes());
    hasher.update(target.as_bytes());
    hasher.update(nonce.to_be_bytes());

    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_creation() {
        let challenge = Challenge::new(
            "challenger123".to_string(),
            "target456".to_string(),
            ChallengeType::bandwidth_1mb(),
            60,
        );

        assert!(challenge.id.contains("challenger123"));
        assert!(challenge.id.contains("target456"));
        assert!(!challenge.is_expired());
    }

    #[test]
    fn test_challenge_expiration() {
        let challenge = Challenge::new(
            "challenger".to_string(),
            "target".to_string(),
            ChallengeType::bandwidth_1mb(),
            0,
        );

        assert!(challenge.is_expired());
    }

    #[test]
    fn test_challenge_response_success() {
        let response = ChallengeResponse::success(
            "challenge-123".to_string(),
            "responder".to_string(),
            5.5,
            50,
        );

        assert!(response.is_success());

        if let ChallengeResult::Success {
            bandwidth_mbps,
            latency_ms,
        } = response.result
        {
            assert!((bandwidth_mbps - 5.5).abs() < 0.01);
            assert_eq!(latency_ms, 50);
        } else {
            panic!("Expected success result");
        }
    }

    #[test]
    fn test_challenge_response_failed() {
        let response = ChallengeResponse::failed(
            "challenge-123".to_string(),
            "responder".to_string(),
            "Connection refused".to_string(),
        );

        assert!(!response.is_success());
    }

    #[test]
    fn test_verifier_valid_response() {
        let verifier = ChallengeVerifier::new();

        let challenge = Challenge::new(
            "challenger".to_string(),
            "target".to_string(),
            ChallengeType::bandwidth_1mb(),
            60,
        );

        let response =
            ChallengeResponse::success(challenge.id.clone(), "target".to_string(), 0.18, 100);

        assert!(verifier.verify_response(&challenge, &response));
    }

    #[test]
    fn test_verifier_wrong_responder() {
        let verifier = ChallengeVerifier::new();

        let challenge = Challenge::new(
            "challenger".to_string(),
            "target".to_string(),
            ChallengeType::bandwidth_1mb(),
            60,
        );

        let response = ChallengeResponse::success(
            challenge.id.clone(),
            "wrong-responder".to_string(),
            10.0,
            100,
        );

        assert!(!verifier.verify_response(&challenge, &response));
    }

    #[test]
    fn test_verifier_failed_response() {
        let verifier = ChallengeVerifier::new();

        let challenge = Challenge::new(
            "challenger".to_string(),
            "target".to_string(),
            ChallengeType::bandwidth_1mb(),
            60,
        );

        let response = ChallengeResponse::failed(
            challenge.id.clone(),
            "target".to_string(),
            "Test failure".to_string(),
        );

        assert!(!verifier.verify_response(&challenge, &response));
    }

    #[test]
    fn test_bandwidth_measurement() {
        let data = vec![0u8; 1_048_576];
        let bandwidth = ChallengeVerifier::measure_bandwidth(&data, 1000);

        assert!((bandwidth - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_challenge_serialization() {
        let challenge = Challenge::new(
            "challenger".to_string(),
            "target".to_string(),
            ChallengeType::bandwidth_1mb(),
            60,
        );

        let serialized = challenge.serialize().unwrap();
        let deserialized: Challenge = Challenge::deserialize(&serialized).unwrap();

        assert_eq!(challenge.id, deserialized.id);
        assert_eq!(
            challenge.challenger_peer_id,
            deserialized.challenger_peer_id
        );
    }
}
