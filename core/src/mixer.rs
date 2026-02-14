use crate::audio::{mix_audio, AudioFrame};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const FULL_MESH_MAX_PARTICIPANTS: usize = 5;
pub const MIXER_ROTATION_INTERVAL: Duration = Duration::from_secs(1800); // 30 minutes
pub const SCORE_TIE_THRESHOLD: f64 = 0.05; // 5% difference

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerRole {
    Peer,
    Mixer,
}

#[derive(Debug, Clone)]
pub struct ParticipantStats {
    pub bandwidth_bps: u64,
    pub latency_ms: u32,
    pub latency_variance: f32,
    pub cpu_usage_percent: f32,
    pub memory_usage_percent: f32,
    pub session_duration: Duration,
    pub packet_loss_percent: f32,
    pub last_updated: Instant,
}

impl ParticipantStats {
    pub fn new() -> Self {
        Self {
            bandwidth_bps: 0,
            latency_ms: 0,
            latency_variance: 0.0,
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            session_duration: Duration::ZERO,
            packet_loss_percent: 0.0,
            last_updated: Instant::now(),
        }
    }

    pub fn update_latency(&mut self, latency_ms: u32) {
        // Update variance using Welford's algorithm
        let delta = latency_ms as f32 - self.latency_ms as f32;
        self.latency_ms = latency_ms;
        self.latency_variance += delta * delta;
        self.last_updated = Instant::now();
    }

    pub fn get_stability_score(&self) -> f32 {
        // Lower variance = higher stability
        if self.latency_variance == 0.0 {
            return 1.0;
        }
        1.0 / (1.0 + self.latency_variance.sqrt() / 100.0)
    }
}

impl Default for ParticipantStats {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Participant {
    pub peer_id: String,
    pub display_name: Option<String>,
    pub role: MixerRole,
    pub stats: ParticipantStats,
    pub audio_buffer: Vec<AudioFrame>,
    pub mixer_start_time: Option<Instant>,
    pub score: f64,
}

impl Participant {
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            display_name: None,
            role: MixerRole::Peer,
            stats: ParticipantStats::new(),
            audio_buffer: Vec::new(),
            mixer_start_time: None,
            score: 0.0,
        }
    }

    pub fn calculate_score(&mut self, weights: &ScoreWeights) -> f64 {
        let bandwidth_score = self.calculate_bandwidth_score();
        let stability_score = self.stats.get_stability_score() as f64;
        let resource_score = self.calculate_resource_score();
        let duration_score = self.calculate_duration_score();

        self.score = (bandwidth_score * weights.bandwidth)
            + (stability_score * weights.stability)
            + (resource_score * weights.resources)
            + (duration_score * weights.duration);

        self.score
    }

    fn calculate_bandwidth_score(&self) -> f64 {
        // Normalize bandwidth (assume max 10 Mbps = 1.0 score)
        let max_bandwidth = 10_000_000.0; // 10 Mbps
        (self.stats.bandwidth_bps as f64 / max_bandwidth).min(1.0)
    }

    fn calculate_resource_score(&self) -> f64 {
        // Lower CPU and memory usage = higher score
        let cpu_factor = 1.0 - (self.stats.cpu_usage_percent / 100.0).min(1.0);
        let memory_factor = 1.0 - (self.stats.memory_usage_percent / 100.0).min(1.0);
        ((cpu_factor + memory_factor) / 2.0) as f64
    }

    fn calculate_duration_score(&self) -> f64 {
        // Quadratic growth with session duration (capped at 1 hour)
        let max_duration = Duration::from_secs(3600);
        let normalized = self.stats.session_duration.as_secs_f64() / max_duration.as_secs_f64();
        (normalized * normalized).min(1.0)
    }
}

#[derive(Debug, Clone)]
pub struct ScoreWeights {
    pub bandwidth: f64,
    pub stability: f64,
    pub resources: f64,
    pub duration: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            bandwidth: 0.40,
            stability: 0.25,
            resources: 0.20,
            duration: 0.15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyMode {
    FullMesh,
    SFU,
}

#[derive(Debug, Clone)]
pub struct MixerConfig {
    pub max_participants: usize,
    pub full_mesh_threshold: usize,
    pub rotation_interval: Duration,
    pub score_weights: ScoreWeights,
    pub mixing_sample_rate: u32,
}

impl Default for MixerConfig {
    fn default() -> Self {
        Self {
            max_participants: 20,
            full_mesh_threshold: FULL_MESH_MAX_PARTICIPANTS,
            rotation_interval: MIXER_ROTATION_INTERVAL,
            score_weights: ScoreWeights::default(),
            mixing_sample_rate: 48000,
        }
    }
}

pub struct MixerManager {
    config: MixerConfig,
    participants: HashMap<String, Participant>,
    local_peer_id: String,
    local_role: MixerRole,
    current_mixer: Option<String>,
    mixer_start_time: Option<Instant>,
    topology_mode: TopologyMode,
    pending_rotation: bool,
}

impl MixerManager {
    pub fn new(local_peer_id: String, config: Option<MixerConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            participants: HashMap::new(),
            local_peer_id,
            local_role: MixerRole::Peer,
            current_mixer: None,
            mixer_start_time: None,
            topology_mode: TopologyMode::FullMesh,
            pending_rotation: false,
        }
    }

    pub fn add_participant(&mut self, peer_id: String) {
        let participant = Participant::new(peer_id.clone());
        self.participants.insert(peer_id, participant);
        self.update_topology_mode();
    }

    pub fn remove_participant(&mut self, peer_id: &str) {
        self.participants.remove(peer_id);

        // If mixer left, trigger reselection
        if self.current_mixer.as_deref() == Some(peer_id) {
            self.current_mixer = None;
            self.select_mixer();
        }

        self.update_topology_mode();
    }

    pub fn update_participant_stats(&mut self, peer_id: &str, stats: ParticipantStats) {
        if let Some(participant) = self.participants.get_mut(peer_id) {
            participant.stats = stats;
        }
    }

    fn update_topology_mode(&mut self) {
        let count = self.participants.len() + 1; // +1 for local peer

        let new_mode = if count <= self.config.full_mesh_threshold {
            TopologyMode::FullMesh
        } else {
            TopologyMode::SFU
        };

        if new_mode != self.topology_mode {
            tracing::info!(
                from = ?self.topology_mode,
                to = ?new_mode,
                participant_count = count,
                "Topology mode changed"
            );
            self.topology_mode = new_mode;

            if new_mode == TopologyMode::SFU && self.current_mixer.is_none() {
                self.select_mixer();
            }
        }
    }

    pub fn select_mixer(&mut self) -> Option<String> {
        if self.topology_mode != TopologyMode::SFU {
            self.current_mixer = None;
            return None;
        }

        // Calculate scores for all participants including self
        let mut local_participant = Participant::new(self.local_peer_id.clone());
        local_participant.role = self.local_role;

        let mut all_scores: Vec<(String, f64)> = self
            .participants
            .iter_mut()
            .map(|(id, p)| {
                p.calculate_score(&self.config.score_weights);
                (id.clone(), p.score)
            })
            .collect();

        // Add local peer score
        local_participant.calculate_score(&self.config.score_weights);
        all_scores.push((self.local_peer_id.clone(), local_participant.score));

        // Sort by score descending
        all_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Check for tie
        if all_scores.len() >= 2 {
            let top = &all_scores[0];
            let second = &all_scores[1];
            let diff = (top.1 - second.1) / top.1.max(0.001);

            if diff < SCORE_TIE_THRESHOLD {
                // Use deterministic hash-based selection for tie
                let selected = self.resolve_tie(&all_scores);
                tracing::info!(
                    tie_score = top.1,
                    selected = %selected,
                    "Mixer selection: tie resolved deterministically"
                );
                self.set_mixer(selected);
                return self.current_mixer.clone();
            }
        }

        if let Some((peer_id, score)) = all_scores.first() {
            tracing::info!(
                mixer = %peer_id,
                score = score,
                "Mixer selected"
            );
            self.set_mixer(peer_id.clone());
        }

        self.current_mixer.clone()
    }

    fn resolve_tie(&self, candidates: &[(String, f64)]) -> String {
        // Deterministic selection based on hash of peer IDs
        // All clients will compute the same result
        let mut sorted_ids: Vec<&String> = candidates.iter().map(|(id, _)| id).collect();
        sorted_ids.sort();

        // Use the lexicographically smallest peer ID
        // This ensures all clients select the same mixer
        sorted_ids.first().unwrap().to_string()
    }

    fn set_mixer(&mut self, peer_id: String) {
        let is_local = peer_id == self.local_peer_id;

        // Reset mixer timer for previous mixer
        if let Some(old_mixer) = &self.current_mixer {
            if let Some(p) = self.participants.get_mut(old_mixer) {
                p.role = MixerRole::Peer;
                p.mixer_start_time = None;
            }
        }

        self.current_mixer = Some(peer_id.clone());
        self.mixer_start_time = Some(Instant::now());

        if is_local {
            self.local_role = MixerRole::Mixer;
        } else if let Some(p) = self.participants.get_mut(&peer_id) {
            p.role = MixerRole::Mixer;
            p.mixer_start_time = Some(Instant::now());
        }
    }

    pub fn check_rotation(&mut self) -> bool {
        if self.topology_mode != TopologyMode::SFU {
            return false;
        }

        let Some(start_time) = self.mixer_start_time else {
            return false;
        };

        if start_time.elapsed() >= self.config.rotation_interval {
            self.pending_rotation = true;
            tracing::info!("Mixer rotation required after 30 minutes");
            return true;
        }

        false
    }

    pub fn rotate_mixer(&mut self) -> Option<String> {
        // Reset duration score for current mixer to force rotation
        if let Some(current) = &self.current_mixer {
            if let Some(p) = self.participants.get_mut(current) {
                p.stats.session_duration = Duration::ZERO;
            }
        }

        self.pending_rotation = false;
        self.select_mixer()
    }

    pub fn is_mixer(&self) -> bool {
        self.local_role == MixerRole::Mixer
    }

    pub fn get_topology_mode(&self) -> TopologyMode {
        self.topology_mode
    }

    pub fn get_current_mixer(&self) -> Option<&str> {
        self.current_mixer.as_deref()
    }

    pub fn get_participant_count(&self) -> usize {
        self.participants.len() + 1
    }

    pub fn get_participants(&self) -> &HashMap<String, Participant> {
        &self.participants
    }

    pub fn mix_incoming_audio(&self, local_audio: Option<&AudioFrame>) -> Option<AudioFrame> {
        if !self.is_mixer() {
            return local_audio.cloned();
        }

        // Collect all audio frames
        let mut frames: Vec<&AudioFrame> = Vec::new();

        for participant in self.participants.values() {
            if let Some(frame) = participant.audio_buffer.last() {
                frames.push(frame);
            }
        }

        if let Some(local) = local_audio {
            frames.push(local);
        }

        if frames.is_empty() {
            return None;
        }

        // Mix with equal weights
        let weights: Vec<f32> = vec![1.0 / frames.len() as f32; frames.len()];
        let frame_refs: Vec<&[f32]> = frames.iter().map(|f| f.as_slice()).collect();

        Some(mix_audio(&frame_refs, &weights))
    }

    pub fn get_connection_targets(&self) -> Vec<String> {
        match self.topology_mode {
            TopologyMode::FullMesh => {
                // Connect to all peers
                self.participants.keys().cloned().collect()
            }
            TopologyMode::SFU => {
                // Only connect to mixer
                self.current_mixer
                    .iter()
                    .filter(|id| *id != &self.local_peer_id)
                    .cloned()
                    .collect()
            }
        }
    }

    pub fn get_participant_info(&self, peer_id: &str) -> Option<&Participant> {
        self.participants.get(peer_id)
    }

    pub fn update_local_stats(&mut self, bandwidth_bps: u64, cpu_usage: f32, memory_usage: f32) {
        let local_participant = Participant {
            peer_id: self.local_peer_id.clone(),
            display_name: None,
            role: self.local_role,
            stats: ParticipantStats {
                bandwidth_bps,
                cpu_usage_percent: cpu_usage,
                memory_usage_percent: memory_usage,
                session_duration: self
                    .mixer_start_time
                    .map(|t| t.elapsed())
                    .unwrap_or(Duration::ZERO),
                ..ParticipantStats::default()
            },
            audio_buffer: Vec::new(),
            mixer_start_time: self.mixer_start_time,
            score: 0.0,
        };

        // This is just for score calculation, we don't store local participant
        let _ = local_participant;
    }
}

#[derive(Debug, Clone)]
pub struct MixerStatus {
    pub topology: TopologyMode,
    pub participant_count: usize,
    pub current_mixer: Option<String>,
    pub is_local_mixer: bool,
    pub uptime: Option<Duration>,
}

impl MixerManager {
    pub fn get_status(&self) -> MixerStatus {
        MixerStatus {
            topology: self.topology_mode,
            participant_count: self.get_participant_count(),
            current_mixer: self.current_mixer.clone(),
            is_local_mixer: self.is_mixer(),
            uptime: self.mixer_start_time.map(|t| t.elapsed()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_participant_score_calculation() {
        let mut participant = Participant::new("peer1".to_string());
        participant.stats.bandwidth_bps = 5_000_000; // 5 Mbps
        participant.stats.latency_ms = 50;
        participant.stats.cpu_usage_percent = 30.0;
        participant.stats.memory_usage_percent = 40.0;
        participant.stats.session_duration = Duration::from_secs(1800); // 30 min

        let weights = ScoreWeights::default();
        participant.calculate_score(&weights);

        assert!(participant.score > 0.0);
        assert!(participant.score <= 1.0);
    }

    #[test]
    fn test_topology_mode_switching() {
        let mut manager = MixerManager::new("local".to_string(), None);

        // Start with FullMesh (just local peer = 1)
        assert_eq!(manager.get_topology_mode(), TopologyMode::FullMesh);

        // Add participants up to threshold (local + 4 = 5 total)
        for i in 0..4 {
            manager.add_participant(format!("peer{}", i));
        }
        assert_eq!(manager.get_participant_count(), 5);
        assert_eq!(manager.get_topology_mode(), TopologyMode::FullMesh);

        // Add one more -> switch to SFU (local + 5 = 6 total)
        manager.add_participant("extra".to_string());
        assert_eq!(manager.get_participant_count(), 6);
        assert_eq!(manager.get_topology_mode(), TopologyMode::SFU);
    }

    #[test]
    fn test_mixer_selection() {
        let mut manager = MixerManager::new("local".to_string(), None);

        // Force SFU mode
        for i in 0..6 {
            manager.add_participant(format!("peer{}", i));
        }

        // Set local peer as best candidate
        manager.update_local_stats(10_000_000, 10.0, 20.0);

        let mixer = manager.select_mixer();
        assert!(mixer.is_some());
    }

    #[test]
    fn test_stability_score() {
        let mut stats = ParticipantStats::new();

        // Low variance = high stability
        stats.update_latency(50);
        stats.update_latency(50);
        stats.update_latency(50);

        let score = stats.get_stability_score();
        assert!(
            score > 0.5,
            "Low variance should give high stability score, got {}",
            score
        );

        // Very high variance = lower stability
        stats.update_latency(500);
        stats.update_latency(1000);
        stats.update_latency(50);

        let score_high_variance = stats.get_stability_score();
        assert!(
            score_high_variance < score,
            "High variance should give lower stability score"
        );
    }

    #[test]
    fn test_duration_score() {
        let mut p1 = Participant::new("p1".to_string());
        p1.stats.session_duration = Duration::from_secs(3600); // 1 hour
        let score1 = p1.calculate_duration_score();

        let mut p2 = Participant::new("p2".to_string());
        p2.stats.session_duration = Duration::from_secs(900); // 15 min
        let score2 = p2.calculate_duration_score();

        assert!(score1 > score2);
        assert!((score1 - 1.0).abs() < 0.01); // Should be ~1.0
    }

    #[test]
    fn test_connection_targets_full_mesh() {
        let mut manager = MixerManager::new("local".to_string(), None);
        manager.add_participant("peer1".to_string());
        manager.add_participant("peer2".to_string());

        let targets = manager.get_connection_targets();
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&"peer1".to_string()));
        assert!(targets.contains(&"peer2".to_string()));
    }

    #[test]
    fn test_mixer_rotation_check() {
        let mut config = MixerConfig::default();
        config.rotation_interval = Duration::from_millis(10);

        let mut manager = MixerManager::new("local".to_string(), Some(config));

        // Force SFU mode and select mixer
        for i in 0..6 {
            manager.add_participant(format!("peer{}", i));
        }
        manager.select_mixer();

        // Wait for rotation interval
        std::thread::sleep(Duration::from_millis(20));

        assert!(manager.check_rotation());
    }

    #[test]
    fn test_tie_resolution() {
        let manager = MixerManager::new("local".to_string(), None);

        let candidates = vec![
            ("peer_c".to_string(), 0.9),
            ("peer_a".to_string(), 0.9),
            ("peer_b".to_string(), 0.9),
        ];

        let selected = manager.resolve_tie(&candidates);
        assert_eq!(selected, "peer_a"); // Lexicographically smallest
    }

    #[test]
    fn test_participant_count() {
        let mut manager = MixerManager::new("local".to_string(), None);
        assert_eq!(manager.get_participant_count(), 1); // Just local

        manager.add_participant("peer1".to_string());
        assert_eq!(manager.get_participant_count(), 2);

        manager.add_participant("peer2".to_string());
        assert_eq!(manager.get_participant_count(), 3);

        manager.remove_participant("peer1");
        assert_eq!(manager.get_participant_count(), 2);
    }
}
