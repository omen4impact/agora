use crate::aec::AcousticEchoCanceller;
use crate::audio::{AudioConfig, AudioFrame, FRAME_SIZE, SAMPLE_RATE};
use crate::codec::{EncodedFrame, OpusConfig, OpusDecoder, OpusEncoder, OpusMode};
use crate::denoise::RnnoiseDenoiser;
use crate::error::{AgoraResult, Error};

pub struct AudioProcessorConfig {
    pub audio: AudioConfig,
    pub opus: OpusConfig,
    pub enable_denoising: bool,
    pub enable_echo_cancellation: bool,
}

impl Default for AudioProcessorConfig {
    fn default() -> Self {
        Self {
            audio: AudioConfig::default(),
            opus: OpusConfig::default(),
            enable_denoising: true,
            enable_echo_cancellation: true,
        }
    }
}

impl AudioProcessorConfig {
    pub fn with_bitrate(mut self, bitrate: i32) -> Self {
        self.opus = self.opus.with_bitrate(bitrate);
        self.audio.bitrate = bitrate;
        self
    }

    pub fn with_mode(mut self, mode: OpusMode) -> Self {
        self.opus = self.opus.with_mode(mode);
        self
    }

    pub fn with_denoising(mut self, enable: bool) -> Self {
        self.enable_denoising = enable;
        self.audio.enable_noise_suppression = enable;
        self
    }

    pub fn with_fec(mut self, enable: bool) -> Self {
        self.opus = self.opus.with_fec(enable);
        self
    }

    pub fn with_echo_cancellation(mut self, enable: bool) -> Self {
        self.enable_echo_cancellation = enable;
        self.audio.enable_echo_cancellation = enable;
        self
    }
}

pub struct AudioProcessor {
    encoder: OpusEncoder,
    decoder: OpusDecoder,
    denoiser: Option<RnnoiseDenoiser>,
    echo_canceller: Option<AcousticEchoCanceller>,
    config: AudioProcessorConfig,
    frames_processed: u64,
    bytes_encoded: u64,
    far_end_buffer: Vec<f32>,
}

impl AudioProcessor {
    pub fn new(config: AudioProcessorConfig) -> AgoraResult<Self> {
        let encoder = OpusEncoder::new(config.opus.clone())?;
        let decoder = OpusDecoder::new(config.audio.sample_rate, config.audio.channels as u8)?;

        let denoiser = if config.enable_denoising {
            Some(RnnoiseDenoiser::new()?)
        } else {
            None
        };

        let echo_canceller = if config.enable_echo_cancellation {
            Some(AcousticEchoCanceller::new(config.audio.sample_rate))
        } else {
            None
        };

        tracing::info!(
            "AudioProcessor created: {} Hz, {} channels, {} bps, denoising: {}, aec: {}",
            config.audio.sample_rate,
            config.audio.channels,
            config.opus.bitrate,
            config.enable_denoising,
            config.enable_echo_cancellation
        );

        Ok(Self {
            encoder,
            decoder,
            denoiser,
            echo_canceller,
            config,
            frames_processed: 0,
            bytes_encoded: 0,
            far_end_buffer: Vec::new(),
        })
    }

    pub fn process_and_encode(&mut self, frame: &mut AudioFrame) -> AgoraResult<EncodedFrame> {
        if frame.len() != FRAME_SIZE {
            return Err(Error::Audio(format!(
                "Invalid frame size: expected {}, got {}",
                FRAME_SIZE,
                frame.len()
            )));
        }

        if let Some(ref mut denoiser) = self.denoiser {
            denoiser.process(frame);
        }

        let encoded = self.encoder.encode_frame(frame)?;

        self.frames_processed += 1;
        self.bytes_encoded += encoded.data.len() as u64;

        Ok(encoded)
    }

    pub fn decode_and_process(&mut self, encoded: &[u8]) -> AgoraResult<AudioFrame> {
        let mut frame = self.decoder.decode(encoded)?;

        if let Some(ref mut denoiser) = self.denoiser {
            denoiser.process(&mut frame);
        }

        Ok(frame)
    }

    pub fn decode_with_plc(&mut self) -> AgoraResult<AudioFrame> {
        self.decoder.decode_packet_loss()
    }

    pub fn set_bitrate(&mut self, bitrate: i32) -> AgoraResult<()> {
        self.encoder.set_bitrate(bitrate)
    }

    pub fn set_denoising(&mut self, enabled: bool) {
        if enabled && self.denoiser.is_none() {
            self.denoiser = RnnoiseDenoiser::new().ok();
        } else if !enabled {
            self.denoiser = None;
        }
        self.config.enable_denoising = enabled;
    }

    pub fn set_echo_cancellation(&mut self, enabled: bool) {
        if enabled && self.echo_canceller.is_none() {
            self.echo_canceller = Some(AcousticEchoCanceller::new(self.config.audio.sample_rate));
        } else if !enabled {
            self.echo_canceller = None;
        }
        self.config.enable_echo_cancellation = enabled;
    }

    pub fn push_far_end(&mut self, frame: &AudioFrame) {
        self.far_end_buffer.extend(frame);
        if self.far_end_buffer.len() > FRAME_SIZE * 4 {
            self.far_end_buffer.drain(0..FRAME_SIZE);
        }
    }

    pub fn process_with_aec(&mut self, frame: &mut AudioFrame) -> AgoraResult<EncodedFrame> {
        if frame.len() != FRAME_SIZE {
            return Err(Error::Audio(format!(
                "Invalid frame size: expected {}, got {}",
                FRAME_SIZE,
                frame.len()
            )));
        }

        if let Some(ref mut aec) = self.echo_canceller {
            let far_end: Vec<f32> = self
                .far_end_buffer
                .drain(0..FRAME_SIZE.min(self.far_end_buffer.len()))
                .collect();
            if far_end.len() == FRAME_SIZE {
                let processed = aec.process_frame(&far_end, frame);
                *frame = processed;
            }
        }

        if let Some(ref mut denoiser) = self.denoiser {
            denoiser.process(frame);
        }

        let encoded = self.encoder.encode_frame(frame)?;

        self.frames_processed += 1;
        self.bytes_encoded += encoded.data.len() as u64;

        Ok(encoded)
    }

    pub fn echo_stats(&self) -> Option<&crate::aec::EchoStats> {
        self.echo_canceller
            .as_ref()
            .map(|aec: &AcousticEchoCanceller| aec.stats())
    }

    pub fn bitrate(&self) -> i32 {
        self.encoder.bitrate()
    }

    pub fn stats(&self) -> ProcessorStats {
        let avg_frame_size = if self.frames_processed > 0 {
            self.bytes_encoded as f64 / self.frames_processed as f64
        } else {
            0.0
        };

        let effective_bitrate = if self.frames_processed > 0 {
            let frame_duration_ms = FRAME_SIZE as f64 / SAMPLE_RATE as f64 * 1000.0;
            let frames_per_second = 1000.0 / frame_duration_ms;
            avg_frame_size * 8.0 * frames_per_second
        } else {
            0.0
        };

        ProcessorStats {
            frames_processed: self.frames_processed,
            bytes_encoded: self.bytes_encoded,
            avg_frame_size,
            effective_bitrate,
            denoising_enabled: self.denoiser.is_some(),
            echo_cancellation_enabled: self.echo_canceller.is_some(),
        }
    }

    pub fn config(&self) -> &AudioProcessorConfig {
        &self.config
    }
}

#[derive(Debug, Clone)]
pub struct ProcessorStats {
    pub frames_processed: u64,
    pub bytes_encoded: u64,
    pub avg_frame_size: f64,
    pub effective_bitrate: f64,
    pub denoising_enabled: bool,
    pub echo_cancellation_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitrateLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

impl BitrateLevel {
    pub fn bitrate(&self) -> i32 {
        match self {
            BitrateLevel::Low => 16000,
            BitrateLevel::Medium => 32000,
            BitrateLevel::High => 64000,
            BitrateLevel::VeryHigh => 96000,
        }
    }

    pub fn from_network_quality(packet_loss: f32, rtt_ms: u64) -> Self {
        if packet_loss > 0.15 || rtt_ms > 200 {
            BitrateLevel::Low
        } else if packet_loss > 0.08 || rtt_ms > 100 {
            BitrateLevel::Medium
        } else if packet_loss > 0.03 || rtt_ms > 50 {
            BitrateLevel::High
        } else {
            BitrateLevel::VeryHigh
        }
    }
}

pub struct AdaptiveBitrateController {
    current_level: BitrateLevel,
    packet_loss_history: Vec<f32>,
    rtt_history: Vec<u64>,
    max_history: usize,
}

impl AdaptiveBitrateController {
    pub fn new() -> Self {
        Self {
            current_level: BitrateLevel::Medium,
            packet_loss_history: Vec::new(),
            rtt_history: Vec::new(),
            max_history: 10,
        }
    }

    pub fn update(&mut self, packet_loss: f32, rtt_ms: u64) {
        self.packet_loss_history.push(packet_loss);
        self.rtt_history.push(rtt_ms);

        if self.packet_loss_history.len() > self.max_history {
            self.packet_loss_history.remove(0);
        }
        if self.rtt_history.len() > self.max_history {
            self.rtt_history.remove(0);
        }
    }

    pub fn suggest_bitrate(&self) -> i32 {
        let avg_loss = if self.packet_loss_history.is_empty() {
            0.0
        } else {
            self.packet_loss_history.iter().sum::<f32>() / self.packet_loss_history.len() as f32
        };

        let avg_rtt = if self.rtt_history.is_empty() {
            0
        } else {
            self.rtt_history.iter().sum::<u64>() / self.rtt_history.len() as u64
        };

        BitrateLevel::from_network_quality(avg_loss, avg_rtt).bitrate()
    }

    pub fn should_adjust(&self) -> bool {
        let suggested = self.suggest_bitrate();
        suggested != self.current_level.bitrate()
    }

    pub fn current_level(&self) -> BitrateLevel {
        self.current_level
    }

    pub fn set_level(&mut self, level: BitrateLevel) {
        self.current_level = level;
    }
}

impl Default for AdaptiveBitrateController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_processor_config_default() {
        let config = AudioProcessorConfig::default();
        assert!(config.enable_denoising);
        assert_eq!(config.opus.bitrate, 32000);
    }

    #[test]
    fn test_audio_processor_creation() {
        let processor = AudioProcessor::new(AudioProcessorConfig::default());
        assert!(processor.is_ok());
    }

    #[test]
    fn test_audio_processor_encode_decode() {
        let mut processor = AudioProcessor::new(AudioProcessorConfig::default()).unwrap();

        let mut frame: Vec<f32> = (0..FRAME_SIZE)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        let encoded = processor.process_and_encode(&mut frame).unwrap();
        assert!(!encoded.data.is_empty());
        assert_eq!(encoded.sequence, 1);

        let decoded = processor.decode_and_process(&encoded.data).unwrap();
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_audio_processor_denoising_toggle() {
        let mut processor = AudioProcessor::new(AudioProcessorConfig::default()).unwrap();

        assert!(processor.denoiser.is_some());

        processor.set_denoising(false);
        assert!(processor.denoiser.is_none());

        processor.set_denoising(true);
        assert!(processor.denoiser.is_some());
    }

    #[test]
    fn test_audio_processor_bitrate_change() {
        let mut processor = AudioProcessor::new(AudioProcessorConfig::default()).unwrap();

        processor.set_bitrate(64000).unwrap();
        assert_eq!(processor.bitrate(), 64000);
    }

    #[test]
    fn test_audio_processor_stats() {
        let mut processor = AudioProcessor::new(AudioProcessorConfig::default()).unwrap();

        let mut frame: Vec<f32> = (0..FRAME_SIZE)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        processor.process_and_encode(&mut frame).unwrap();
        processor.process_and_encode(&mut frame).unwrap();

        let stats = processor.stats();
        assert_eq!(stats.frames_processed, 2);
        assert!(stats.bytes_encoded > 0);
        assert!(stats.denoising_enabled);
    }

    #[test]
    fn test_bitrate_level_bitrate() {
        assert_eq!(BitrateLevel::Low.bitrate(), 16000);
        assert_eq!(BitrateLevel::Medium.bitrate(), 32000);
        assert_eq!(BitrateLevel::High.bitrate(), 64000);
        assert_eq!(BitrateLevel::VeryHigh.bitrate(), 96000);
    }

    #[test]
    fn test_bitrate_level_from_network_quality() {
        assert_eq!(
            BitrateLevel::from_network_quality(0.2, 250),
            BitrateLevel::Low
        );
        assert_eq!(
            BitrateLevel::from_network_quality(0.1, 150),
            BitrateLevel::Medium
        );
        assert_eq!(
            BitrateLevel::from_network_quality(0.05, 75),
            BitrateLevel::High
        );
        assert_eq!(
            BitrateLevel::from_network_quality(0.01, 30),
            BitrateLevel::VeryHigh
        );
    }

    #[test]
    fn test_adaptive_bitrate_controller() {
        let mut controller = AdaptiveBitrateController::new();

        controller.update(0.05, 80);
        controller.update(0.04, 75);

        let bitrate = controller.suggest_bitrate();
        assert!(bitrate > 0);
    }

    #[test]
    fn test_adaptive_bitrate_should_adjust() {
        let mut controller = AdaptiveBitrateController::new();

        controller.update(0.01, 30);
        controller.update(0.01, 25);

        assert!(controller.should_adjust());
    }

    #[test]
    fn test_audio_processor_plc() {
        let mut processor = AudioProcessor::new(AudioProcessorConfig::default()).unwrap();

        let frame = processor.decode_with_plc().unwrap();
        assert!(!frame.is_empty());
    }

    #[test]
    fn test_config_with_bitrate() {
        let config = AudioProcessorConfig::default().with_bitrate(64000);
        assert_eq!(config.opus.bitrate, 64000);
        assert_eq!(config.audio.bitrate, 64000);
    }

    #[test]
    fn test_config_with_mode() {
        let config = AudioProcessorConfig::default().with_mode(OpusMode::Audio);
        assert_eq!(config.opus.mode, OpusMode::Audio);
    }

    #[test]
    fn test_audio_processor_aec_default() {
        let processor = AudioProcessor::new(AudioProcessorConfig::default()).unwrap();
        assert!(processor.echo_canceller.is_some());
        let stats = processor.stats();
        assert!(stats.echo_cancellation_enabled);
    }

    #[test]
    fn test_audio_processor_aec_disabled() {
        let config = AudioProcessorConfig::default().with_echo_cancellation(false);
        let processor = AudioProcessor::new(config).unwrap();
        assert!(processor.echo_canceller.is_none());
    }

    #[test]
    fn test_audio_processor_aec_toggle() {
        let mut processor = AudioProcessor::new(AudioProcessorConfig::default()).unwrap();
        assert!(processor.echo_canceller.is_some());

        processor.set_echo_cancellation(false);
        assert!(processor.echo_canceller.is_none());

        processor.set_echo_cancellation(true);
        assert!(processor.echo_canceller.is_some());
    }

    #[test]
    fn test_audio_processor_with_aec() {
        let mut processor = AudioProcessor::new(AudioProcessorConfig::default()).unwrap();

        let far_frame: Vec<f32> = (0..FRAME_SIZE)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        let mut near_frame: Vec<f32> = far_frame
            .iter()
            .enumerate()
            .map(|(i, &s)| s * 0.8 + (i as f32 * 0.005).sin() * 0.2)
            .collect();

        processor.push_far_end(&far_frame);
        let encoded = processor.process_with_aec(&mut near_frame).unwrap();

        assert!(!encoded.data.is_empty());
        assert!(processor.echo_stats().is_some());
    }
}
