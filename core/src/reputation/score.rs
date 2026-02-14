use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    pub overall: f32,
    pub components: ScoreComponents,

    pub uptime_seconds: u64,
    pub total_sessions: u64,
    pub successful_sessions: u64,
    pub failed_sessions: u64,

    pub latency_samples: Vec<u32>,
    pub avg_latency_ms: u32,

    pub challenges_passed: u64,
    pub challenges_total: u64,

    pub first_seen: u64,
    pub last_updated: u64,

    pub vouches_received: u32,
    pub vouches_given: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreComponents {
    pub uptime: f32,
    pub performance: f32,
    pub reliability: f32,
    pub challenge: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    pub uptime_weight: f32,
    pub performance_weight: f32,
    pub reliability_weight: f32,
    pub challenge_weight: f32,

    pub uptime_days_max: f32,
    pub latency_excellent_ms: u32,
    pub latency_good_ms: u32,
    pub latency_acceptable_ms: u32,

    pub max_latency_samples: usize,

    pub initial_score: f32,
    pub min_score: f32,
    pub max_score: f32,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            uptime_weight: 0.40,
            performance_weight: 0.30,
            reliability_weight: 0.20,
            challenge_weight: 0.10,

            uptime_days_max: 30.0,
            latency_excellent_ms: 50,
            latency_good_ms: 100,
            latency_acceptable_ms: 200,

            max_latency_samples: 100,

            initial_score: 0.5,
            min_score: 0.0,
            max_score: 1.0,
        }
    }
}

impl ReputationScore {
    pub fn new(config: &ReputationConfig) -> Self {
        let now = current_timestamp();

        Self {
            overall: config.initial_score,
            components: ScoreComponents::default(),
            uptime_seconds: 0,
            total_sessions: 0,
            successful_sessions: 0,
            failed_sessions: 0,
            latency_samples: Vec::new(),
            avg_latency_ms: 0,
            challenges_passed: 0,
            challenges_total: 0,
            first_seen: now,
            last_updated: now,
            vouches_received: 0,
            vouches_given: 0,
        }
    }

    pub fn recalculate(&mut self, config: &ReputationConfig) {
        self.components.uptime = self.calculate_uptime_score(config);
        self.components.performance = self.calculate_performance_score(config);
        self.components.reliability = self.calculate_reliability_score(config);
        self.components.challenge = self.calculate_challenge_score();

        let raw_score = self.components.uptime * config.uptime_weight
            + self.components.performance * config.performance_weight
            + self.components.reliability * config.reliability_weight
            + self.components.challenge * config.challenge_weight;

        self.overall = raw_score.clamp(config.min_score, config.max_score);
        self.last_updated = current_timestamp();
    }

    fn calculate_uptime_score(&self, config: &ReputationConfig) -> f32 {
        let days = self.uptime_seconds as f32 / 86400.0;
        let normalized = days / config.uptime_days_max;

        let quadratic = normalized * normalized;

        quadratic.clamp(0.0, 1.0)
    }

    fn calculate_performance_score(&self, config: &ReputationConfig) -> f32 {
        if self.latency_samples.is_empty() {
            return 0.5;
        }

        let avg = self.avg_latency_ms as f32;

        if avg <= config.latency_excellent_ms as f32 {
            1.0
        } else if avg <= config.latency_good_ms as f32 {
            0.8
        } else if avg <= config.latency_acceptable_ms as f32 {
            0.5
        } else {
            0.2
        }
    }

    fn calculate_reliability_score(&self, _config: &ReputationConfig) -> f32 {
        if self.total_sessions == 0 {
            return 0.5;
        }

        let success_rate = self.successful_sessions as f32 / self.total_sessions as f32;
        success_rate
    }

    fn calculate_challenge_score(&self) -> f32 {
        if self.challenges_total == 0 {
            return 0.5;
        }

        self.challenges_passed as f32 / self.challenges_total as f32
    }

    pub fn record_uptime(&mut self, seconds: u64) {
        self.uptime_seconds += seconds;
    }

    pub fn record_session(&mut self, success: bool) {
        self.total_sessions += 1;
        if success {
            self.successful_sessions += 1;
        } else {
            self.failed_sessions += 1;
        }
    }

    pub fn record_latency(&mut self, latency_ms: u32, config: &ReputationConfig) {
        self.latency_samples.push(latency_ms);

        if self.latency_samples.len() > config.max_latency_samples {
            self.latency_samples.remove(0);
        }

        self.avg_latency_ms = if self.latency_samples.is_empty() {
            0
        } else {
            self.latency_samples.iter().sum::<u32>() / self.latency_samples.len() as u32
        };
    }

    pub fn record_challenge(&mut self, passed: bool) {
        self.challenges_total += 1;
        if passed {
            self.challenges_passed += 1;
        }
    }

    pub fn record_vouch_received(&mut self) {
        self.vouches_received += 1;
    }

    pub fn record_vouch_given(&mut self) {
        self.vouches_given += 1;
    }

    pub fn uptime_days(&self) -> f32 {
        self.uptime_seconds as f32 / 86400.0
    }

    pub fn success_rate(&self) -> f32 {
        if self.total_sessions == 0 {
            return 0.0;
        }
        self.successful_sessions as f32 / self.total_sessions as f32
    }

    pub fn challenge_pass_rate(&self) -> f32 {
        if self.challenges_total == 0 {
            return 0.0;
        }
        self.challenges_passed as f32 / self.challenges_total as f32
    }

    pub fn age_days(&self) -> f32 {
        let now = current_timestamp();
        (now - self.first_seen) as f32 / 86400.0
    }

    pub fn is_trustworthy(&self, threshold: f32) -> bool {
        self.overall >= threshold
    }

    pub fn serialize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
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
    fn test_new_reputation_score() {
        let config = ReputationConfig::default();
        let score = ReputationScore::new(&config);

        assert_eq!(score.overall, config.initial_score);
        assert_eq!(score.uptime_seconds, 0);
        assert_eq!(score.total_sessions, 0);
    }

    #[test]
    fn test_uptime_score_calculation() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        score.uptime_seconds = 86400 * 7;
        score.recalculate(&config);

        let days = 7.0_f32;
        let expected = (days / config.uptime_days_max).powi(2);
        assert!((score.components.uptime - expected).abs() < 0.01);
    }

    #[test]
    fn test_uptime_score_max() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        score.uptime_seconds = 86400 * 60;
        score.recalculate(&config);

        assert!((score.components.uptime - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_performance_score_excellent() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        for _ in 0..10 {
            score.record_latency(30, &config);
        }
        score.recalculate(&config);

        assert!((score.components.performance - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_performance_score_good() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        for _ in 0..10 {
            score.record_latency(75, &config);
        }
        score.recalculate(&config);

        assert!((score.components.performance - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_reliability_score() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        for _ in 0..8 {
            score.record_session(true);
        }
        for _ in 0..2 {
            score.record_session(false);
        }
        score.recalculate(&config);

        assert!((score.components.reliability - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_challenge_score() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        for _ in 0..8 {
            score.record_challenge(true);
        }
        for _ in 0..2 {
            score.record_challenge(false);
        }
        score.recalculate(&config);

        assert!((score.components.challenge - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_overall_score_calculation() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        score.uptime_seconds = 86400 * 30;

        for _ in 0..10 {
            score.record_latency(40, &config);
        }

        for _ in 0..9 {
            score.record_session(true);
        }
        score.record_session(false);

        for _ in 0..10 {
            score.record_challenge(true);
        }

        score.recalculate(&config);

        assert!(score.overall > 0.9);
    }

    #[test]
    fn test_serialization() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        score.uptime_seconds = 86400;
        score.record_session(true);
        score.record_latency(50, &config);
        score.recalculate(&config);

        let serialized = score.serialize().unwrap();
        let deserialized: ReputationScore = ReputationScore::deserialize(&serialized).unwrap();

        assert!((score.overall - deserialized.overall).abs() < 0.001);
        assert_eq!(score.uptime_seconds, deserialized.uptime_seconds);
    }

    #[test]
    fn test_latency_rolling_average() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        for _ in 0..50 {
            score.record_latency(100, &config);
        }

        assert_eq!(score.avg_latency_ms, 100);

        for _ in 0..50 {
            score.record_latency(200, &config);
        }

        assert_eq!(score.avg_latency_ms, 150);
    }

    #[test]
    fn test_is_trustworthy() {
        let config = ReputationConfig::default();
        let mut score = ReputationScore::new(&config);

        score.overall = 0.8;
        assert!(score.is_trustworthy(0.7));
        assert!(!score.is_trustworthy(0.9));
    }
}
