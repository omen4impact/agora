use crate::audio::FRAME_SIZE;

pub const AEC_FILTER_LENGTH: usize = 1024;
pub const AEC_FRAME_SIZE: usize = FRAME_SIZE;
pub const AEC_STEP_SIZE: f32 = 0.5;
pub const AEC_REGULARIZATION: f32 = 1e-6;

#[derive(Debug, Clone)]
pub struct EchoCancellerConfig {
    pub filter_length: usize,
    pub frame_size: usize,
    pub step_size: f32,
    pub regularization: f32,
    pub enable_nonlinear_processing: bool,
    pub double_talk_threshold: f32,
}

impl Default for EchoCancellerConfig {
    fn default() -> Self {
        Self {
            filter_length: AEC_FILTER_LENGTH,
            frame_size: AEC_FRAME_SIZE,
            step_size: AEC_STEP_SIZE,
            regularization: AEC_REGULARIZATION,
            enable_nonlinear_processing: true,
            double_talk_threshold: 0.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EchoStats {
    pub erle: f32,
    pub echo_return: f32,
    pub double_talk_detected: bool,
    pub frames_processed: u64,
}

pub struct EchoCanceller {
    config: EchoCancellerConfig,
    filter: Vec<f32>,
    far_end_buffer: Vec<f32>,
    near_end_buffer: Vec<f32>,
    far_end_history: Vec<f32>,
    power_far: f32,
    power_near: f32,
    power_error: f32,
    stats: EchoStats,
    #[allow(dead_code)]
    sample_rate: u32,
}

impl EchoCanceller {
    pub fn new(config: EchoCancellerConfig, sample_rate: u32) -> Self {
        let filter_length = config.filter_length;

        Self {
            config,
            filter: vec![0.0; filter_length],
            far_end_buffer: Vec::with_capacity(filter_length * 2),
            near_end_buffer: Vec::with_capacity(filter_length),
            far_end_history: vec![0.0; filter_length],
            power_far: 0.0,
            power_near: 0.0,
            power_error: 0.0,
            stats: EchoStats {
                erle: 0.0,
                echo_return: 0.0,
                double_talk_detected: false,
                frames_processed: 0,
            },
            sample_rate,
        }
    }

    pub fn with_sample_rate(sample_rate: u32) -> Self {
        Self::new(EchoCancellerConfig::default(), sample_rate)
    }

    pub fn push_far_end(&mut self, samples: &[f32]) {
        self.far_end_buffer.extend(samples);

        let max_buffer = self.config.filter_length * 4;
        if self.far_end_buffer.len() > max_buffer {
            let excess = self.far_end_buffer.len() - max_buffer;
            self.far_end_buffer.drain(0..excess);
        }
    }

    pub fn push_near_end(&mut self, samples: &[f32]) {
        self.near_end_buffer.extend(samples);

        let max_buffer = self.config.filter_length * 2;
        if self.near_end_buffer.len() > max_buffer {
            let excess = self.near_end_buffer.len() - max_buffer;
            self.near_end_buffer.drain(0..excess);
        }
    }

    pub fn process(&mut self) -> Option<Vec<f32>> {
        let frame_size = self.config.frame_size;
        let filter_length = self.config.filter_length;

        if self.far_end_buffer.len() < frame_size + filter_length {
            return None;
        }

        if self.near_end_buffer.len() < frame_size {
            return None;
        }

        let near_frame: Vec<f32> = self.near_end_buffer.drain(0..frame_size).collect();
        let far_offset = self
            .far_end_buffer
            .len()
            .saturating_sub(frame_size + filter_length);

        let mut output = vec![0.0f32; frame_size];

        for i in 0..frame_size {
            let far_idx = far_offset + i;
            if far_idx >= self.far_end_buffer.len() {
                break;
            }

            let near_sample = near_frame[i];
            let mut echo_estimate = 0.0f32;

            for j in 0..filter_length {
                let history_idx = self.far_end_history.len().saturating_sub(filter_length - j);
                if history_idx < self.far_end_history.len() {
                    echo_estimate += self.filter[j] * self.far_end_history[history_idx];
                } else if far_idx >= j {
                    echo_estimate += self.filter[j] * self.far_end_buffer[far_idx - j];
                }
            }

            let error = near_sample - echo_estimate;
            output[i] = error;

            if far_idx < self.far_end_buffer.len() {
                self.far_end_history.push(self.far_end_buffer[far_idx]);
                if self.far_end_history.len() > filter_length * 2 {
                    self.far_end_history.remove(0);
                }
            }

            self.update_filter(error, far_idx);
        }

        self.far_end_buffer
            .drain(0..frame_size.min(self.far_end_buffer.len()));

        if self.config.enable_nonlinear_processing {
            self.apply_nonlinear_processing(&mut output);
        }

        self.update_stats(&near_frame, &output);

        Some(output)
    }

    fn update_filter(&mut self, error: f32, far_idx: usize) {
        if self.is_double_talk() {
            return;
        }

        let step = self.config.step_size;
        let reg = self.config.regularization;

        let far_power: f32 = if far_idx >= self.config.filter_length {
            self.far_end_buffer[far_idx - self.config.filter_length..far_idx]
                .iter()
                .map(|x| x * x)
                .sum()
        } else {
            1.0
        };

        let normalization = step / (far_power + reg);

        for j in 0..self.config.filter_length {
            let history_idx = self
                .far_end_history
                .len()
                .saturating_sub(self.config.filter_length - j);
            let far_sample = if history_idx < self.far_end_history.len() {
                self.far_end_history[history_idx]
            } else if far_idx >= j {
                self.far_end_buffer[far_idx - j]
            } else {
                0.0
            };

            self.filter[j] += normalization * error * far_sample;
        }

        let max_coeff = self
            .filter
            .iter()
            .map(|x| x.abs())
            .fold(0.0f32, |a, b| a.max(b));
        if max_coeff > 1.0 {
            for coeff in &mut self.filter {
                *coeff /= max_coeff;
            }
        }
    }

    fn is_double_talk(&self) -> bool {
        let threshold = self.config.double_talk_threshold;

        if self.power_far < 1e-10 {
            return false;
        }

        let ratio = self.power_near / self.power_far;
        ratio > threshold
    }

    fn apply_nonlinear_processing(&self, output: &mut [f32]) {
        let threshold = 0.02;

        for sample in output.iter_mut() {
            if sample.abs() < threshold {
                *sample *= 0.5;
            }
        }
    }

    fn update_stats(&mut self, near_frame: &[f32], output: &[f32]) {
        let near_power: f32 =
            near_frame.iter().map(|x| x * x).sum::<f32>() / near_frame.len() as f32;
        let error_power: f32 = output.iter().map(|x| x * x).sum::<f32>() / output.len() as f32;

        self.power_near = self.power_near * 0.9 + near_power * 0.1;
        self.power_error = self.power_error * 0.9 + error_power * 0.1;

        if self.power_far > 1e-10 {
            self.stats.erle = 10.0 * (near_power / (error_power + 1e-10)).log10();
        }

        if near_power > 1e-10 {
            self.stats.echo_return = (near_power - error_power) / near_power;
        }

        self.stats.double_talk_detected = self.is_double_talk();
        self.stats.frames_processed += 1;
    }

    pub fn reset(&mut self) {
        self.filter.fill(0.0);
        self.far_end_buffer.clear();
        self.near_end_buffer.clear();
        self.far_end_history.clear();
        self.power_far = 0.0;
        self.power_near = 0.0;
        self.power_error = 0.0;
        self.stats = EchoStats {
            erle: 0.0,
            echo_return: 0.0,
            double_talk_detected: false,
            frames_processed: 0,
        };
    }

    pub fn stats(&self) -> &EchoStats {
        &self.stats
    }

    pub fn set_step_size(&mut self, step_size: f32) {
        self.config.step_size = step_size.clamp(0.0, 1.0);
    }

    pub fn set_double_talk_threshold(&mut self, threshold: f32) {
        self.config.double_talk_threshold = threshold.clamp(0.0, 1.0);
    }
}

pub struct AcousticEchoCanceller {
    canceller: EchoCanceller,
    enabled: bool,
    suppressor: ResidualEchoSuppressor,
}

impl AcousticEchoCanceller {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            canceller: EchoCanceller::with_sample_rate(sample_rate),
            enabled: true,
            suppressor: ResidualEchoSuppressor::new(),
        }
    }

    pub fn process_frame(&mut self, far_end: &[f32], near_end: &[f32]) -> Vec<f32> {
        if !self.enabled {
            return near_end.to_vec();
        }

        self.canceller.push_far_end(far_end);
        self.canceller.push_near_end(near_end);

        if let Some(output) = self.canceller.process() {
            self.suppressor.process(&output)
        } else {
            near_end.to_vec()
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.canceller.reset();
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn stats(&self) -> &EchoStats {
        self.canceller.stats()
    }

    pub fn reset(&mut self) {
        self.canceller.reset();
        self.suppressor.reset();
    }
}

struct ResidualEchoSuppressor {
    gain: f32,
    attack_rate: f32,
    release_rate: f32,
    min_gain: f32,
}

impl ResidualEchoSuppressor {
    fn new() -> Self {
        Self {
            gain: 1.0,
            attack_rate: 0.1,
            release_rate: 0.001,
            min_gain: 0.1,
        }
    }

    fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        let frame_energy: f32 = samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32;

        let target_gain = if frame_energy > 0.01 {
            self.min_gain
        } else {
            1.0
        };

        if target_gain < self.gain {
            self.gain -= self.attack_rate * (self.gain - target_gain);
        } else {
            self.gain += self.release_rate * (target_gain - self.gain);
        }

        samples.iter().map(|&x| x * self.gain).collect()
    }

    fn reset(&mut self) {
        self.gain = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_canceller_creation() {
        let aec = EchoCanceller::with_sample_rate(48000);
        assert_eq!(aec.filter.len(), AEC_FILTER_LENGTH);
    }

    #[test]
    fn test_echo_canceller_config_default() {
        let config = EchoCancellerConfig::default();
        assert_eq!(config.filter_length, AEC_FILTER_LENGTH);
        assert_eq!(config.frame_size, AEC_FRAME_SIZE);
        assert!(config.step_size > 0.0);
    }

    #[test]
    fn test_push_samples() {
        let mut aec = EchoCanceller::with_sample_rate(48000);

        let far_end = vec![0.5; AEC_FRAME_SIZE];
        let near_end = vec![0.3; AEC_FRAME_SIZE];

        aec.push_far_end(&far_end);
        aec.push_near_end(&near_end);

        assert!(!aec.far_end_buffer.is_empty());
        assert!(!aec.near_end_buffer.is_empty());
    }

    #[test]
    fn test_process_insufficient_data() {
        let mut aec = EchoCanceller::with_sample_rate(48000);

        let result = aec.process();
        assert!(result.is_none());
    }

    #[test]
    fn test_reset() {
        let mut aec = EchoCanceller::with_sample_rate(48000);

        aec.push_far_end(&vec![0.5; 1000]);
        aec.push_near_end(&vec![0.3; 500]);

        aec.reset();

        assert!(aec.far_end_buffer.is_empty());
        assert!(aec.near_end_buffer.is_empty());
        assert_eq!(aec.stats.frames_processed, 0);
    }

    #[test]
    fn test_acoustic_echo_canceller_disabled() {
        let mut aec = AcousticEchoCanceller::new(48000);
        aec.set_enabled(false);

        let far = vec![0.5; AEC_FRAME_SIZE];
        let near = vec![0.3; AEC_FRAME_SIZE];

        let output = aec.process_frame(&far, &near);

        assert_eq!(output, near);
    }

    #[test]
    fn test_echo_cancellation_basic() {
        let mut aec = AcousticEchoCanceller::new(48000);

        let far_signal: Vec<f32> = (0..AEC_FRAME_SIZE)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        let near_signal: Vec<f32> = far_signal
            .iter()
            .enumerate()
            .map(|(i, &s)| s * 0.8 + (i as f32 * 0.005).sin() * 0.2)
            .collect();

        let output = aec.process_frame(&far_signal, &near_signal);

        assert_eq!(output.len(), AEC_FRAME_SIZE);
    }

    #[test]
    fn test_double_talk_detection() {
        let mut aec = EchoCanceller::with_sample_rate(48000);

        let far_quiet = vec![0.01; 500];
        let near_loud = vec![0.9; 500];

        aec.push_far_end(&far_quiet);
        aec.push_near_end(&near_loud);

        aec.power_far = 0.0001;
        aec.power_near = 0.81;

        assert!(aec.is_double_talk());
    }

    #[test]
    fn test_filter_normalization() {
        let mut aec = EchoCanceller::with_sample_rate(48000);

        for i in 0..aec.filter.len() {
            aec.filter[i] = 2.0;
        }

        let far_end = vec![0.5; AEC_FRAME_SIZE + AEC_FILTER_LENGTH];
        let near_end = vec![0.3; AEC_FRAME_SIZE];

        aec.push_far_end(&far_end);
        aec.push_near_end(&near_end);

        let _ = aec.process();

        let max_coeff = aec
            .filter
            .iter()
            .map(|x| x.abs())
            .fold(0.0f32, |a, b| a.max(b));
        assert!(max_coeff <= 1.0);
    }

    #[test]
    fn test_stats_update() {
        let mut aec = AcousticEchoCanceller::new(48000);

        let far = vec![0.5; 500];
        let near = vec![0.3; 160];

        aec.process_frame(&far, &near);

        let stats = aec.stats();
        assert!(stats.frames_processed >= 1);
    }

    #[test]
    fn test_residual_suppressor() {
        let mut suppressor = ResidualEchoSuppressor::new();

        let loud = vec![0.5; 160];
        let output = suppressor.process(&loud);

        assert_eq!(output.len(), 160);
    }
}
