use crate::error::{AgoraResult, Error};
use opus::{Application, Decoder, Encoder};

pub const OPUS_SAMPLE_RATE: u32 = 48000;
pub const OPUS_CHANNELS: u8 = 1;
pub const OPUS_FRAME_SIZE: usize = 960;
pub const OPUS_MIN_BITRATE: i32 = 6000;
pub const OPUS_MAX_BITRATE: i32 = 510000;
pub const OPUS_DEFAULT_BITRATE: i32 = 32000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OpusMode {
    #[default]
    Voip,
    Audio,
    LowDelay,
}

impl From<OpusMode> for Application {
    fn from(mode: OpusMode) -> Self {
        match mode {
            OpusMode::Voip => Application::Voip,
            OpusMode::Audio => Application::Audio,
            OpusMode::LowDelay => Application::LowDelay,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpusConfig {
    pub sample_rate: u32,
    pub channels: u8,
    pub bitrate: i32,
    pub mode: OpusMode,
    pub complexity: u8,
    pub enable_fec: bool,
    pub enable_dtx: bool,
    pub packet_loss_perc: u8,
}

impl Default for OpusConfig {
    fn default() -> Self {
        Self {
            sample_rate: OPUS_SAMPLE_RATE,
            channels: OPUS_CHANNELS,
            bitrate: OPUS_DEFAULT_BITRATE,
            mode: OpusMode::Voip,
            complexity: 5,
            enable_fec: true,
            enable_dtx: true,
            packet_loss_perc: 10,
        }
    }
}

impl OpusConfig {
    pub fn with_bitrate(mut self, bitrate: i32) -> Self {
        self.bitrate = bitrate.clamp(OPUS_MIN_BITRATE, OPUS_MAX_BITRATE);
        self
    }

    pub fn with_mode(mut self, mode: OpusMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_complexity(mut self, complexity: u8) -> Self {
        self.complexity = complexity.clamp(0, 10);
        self
    }

    pub fn with_fec(mut self, enable: bool) -> Self {
        self.enable_fec = enable;
        self
    }

    pub fn with_dtx(mut self, enable: bool) -> Self {
        self.enable_dtx = enable;
        self
    }
}

#[derive(Debug, Clone)]
pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub sequence: u64,
    pub timestamp: u64,
    pub bitrate: i32,
}

pub struct OpusEncoder {
    encoder: Encoder,
    config: OpusConfig,
    frame_count: u64,
}

impl OpusEncoder {
    pub fn new(config: OpusConfig) -> AgoraResult<Self> {
        let mut encoder =
            Encoder::new(config.sample_rate, opus::Channels::Mono, config.mode.into())
                .map_err(|e| Error::Audio(format!("Failed to create Opus encoder: {}", e)))?;

        encoder
            .set_bitrate(opus::Bitrate::Bits(config.bitrate))
            .map_err(|e| Error::Audio(format!("Failed to set bitrate: {}", e)))?;

        encoder
            .set_complexity(config.complexity as i32)
            .map_err(|e| Error::Audio(format!("Failed to set complexity: {}", e)))?;

        if config.enable_fec {
            encoder
                .set_inband_fec(true)
                .map_err(|e| Error::Audio(format!("Failed to enable FEC: {}", e)))?;

            encoder
                .set_packet_loss_perc(config.packet_loss_perc as i32)
                .map_err(|e| {
                    Error::Audio(format!("Failed to set packet loss percentage: {}", e))
                })?;
        }

        if config.enable_dtx {
            encoder
                .set_dtx(true)
                .map_err(|e| Error::Audio(format!("Failed to enable DTX: {}", e)))?;
        }

        tracing::info!(
            "Opus encoder created: {} Hz, {} channels, {} bps, mode {:?}",
            config.sample_rate,
            config.channels,
            config.bitrate,
            config.mode
        );

        Ok(Self {
            encoder,
            config,
            frame_count: 0,
        })
    }

    pub fn encode(&mut self, input: &[f32]) -> AgoraResult<Vec<u8>> {
        if input.len() != OPUS_FRAME_SIZE {
            return Err(Error::Audio(format!(
                "Invalid frame size: expected {}, got {}",
                OPUS_FRAME_SIZE,
                input.len()
            )));
        }

        let mut output = vec![0u8; 4000];
        let len = self
            .encoder
            .encode_float(input, &mut output)
            .map_err(|e| Error::Audio(format!("Opus encoding failed: {}", e)))?;

        output.truncate(len);
        self.frame_count += 1;

        Ok(output)
    }

    pub fn encode_frame(&mut self, input: &[f32]) -> AgoraResult<EncodedFrame> {
        let data = self.encode(input)?;

        Ok(EncodedFrame {
            data,
            sequence: self.frame_count,
            timestamp: self.frame_count * (OPUS_FRAME_SIZE as u64),
            bitrate: self.config.bitrate,
        })
    }

    pub fn set_bitrate(&mut self, bitrate: i32) -> AgoraResult<()> {
        let bitrate = bitrate.clamp(OPUS_MIN_BITRATE, OPUS_MAX_BITRATE);

        self.encoder
            .set_bitrate(opus::Bitrate::Bits(bitrate))
            .map_err(|e| Error::Audio(format!("Failed to set bitrate: {}", e)))?;

        self.config.bitrate = bitrate;
        tracing::debug!("Opus bitrate changed to {} bps", bitrate);
        Ok(())
    }

    pub fn set_complexity(&mut self, complexity: u8) -> AgoraResult<()> {
        let complexity = complexity.clamp(0, 10);

        self.encoder
            .set_complexity(complexity as i32)
            .map_err(|e| Error::Audio(format!("Failed to set complexity: {}", e)))?;

        self.config.complexity = complexity;
        Ok(())
    }

    pub fn bitrate(&self) -> i32 {
        self.config.bitrate
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn config(&self) -> &OpusConfig {
        &self.config
    }
}

pub struct OpusDecoder {
    decoder: Decoder,
    sample_rate: u32,
    channels: u8,
    frame_count: u64,
}

impl OpusDecoder {
    pub fn new(sample_rate: u32, channels: u8) -> AgoraResult<Self> {
        let opus_channels = if channels == 1 {
            opus::Channels::Mono
        } else {
            opus::Channels::Stereo
        };

        let decoder = Decoder::new(sample_rate, opus_channels)
            .map_err(|e| Error::Audio(format!("Failed to create Opus decoder: {}", e)))?;

        tracing::info!(
            "Opus decoder created: {} Hz, {} channels",
            sample_rate,
            channels
        );

        Ok(Self {
            decoder,
            sample_rate,
            channels,
            frame_count: 0,
        })
    }

    pub fn decode(&mut self, input: &[u8]) -> AgoraResult<Vec<f32>> {
        let frame_size = self.sample_rate as usize / 100 * 2;
        let mut output = vec![0.0f32; frame_size * self.channels as usize];

        let samples = self
            .decoder
            .decode_float(input, &mut output, false)
            .map_err(|e| Error::Audio(format!("Opus decoding failed: {}", e)))?;

        output.truncate(samples * self.channels as usize);
        self.frame_count += 1;

        Ok(output)
    }

    pub fn decode_with_fec(&mut self, input: &[u8], decode_fec: bool) -> AgoraResult<Vec<f32>> {
        let frame_size = self.sample_rate as usize / 100 * 2;
        let mut output = vec![0.0f32; frame_size * self.channels as usize];

        let samples = self
            .decoder
            .decode_float(input, &mut output, decode_fec)
            .map_err(|e| Error::Audio(format!("Opus decoding failed: {}", e)))?;

        output.truncate(samples * self.channels as usize);

        if !input.is_empty() {
            self.frame_count += 1;
        }

        Ok(output)
    }

    pub fn decode_packet_loss(&mut self) -> AgoraResult<Vec<f32>> {
        let frame_size = self.sample_rate as usize / 100 * 2;
        let mut output = vec![0.0f32; frame_size * self.channels as usize];

        let samples = self
            .decoder
            .decode_float(&[], &mut output, true)
            .map_err(|e| Error::Audio(format!("Opus PLC failed: {}", e)))?;

        output.truncate(samples * self.channels as usize);
        self.frame_count += 1;

        Ok(output)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u8 {
        self.channels
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

impl super::AudioEncoder for OpusEncoder {
    fn encode(&mut self, input: &[f32]) -> AgoraResult<Vec<u8>> {
        self.encode(input)
    }

    fn set_bitrate(&mut self, bitrate: i32) -> AgoraResult<()> {
        self.set_bitrate(bitrate)
    }

    fn bitrate(&self) -> i32 {
        self.bitrate()
    }
}

impl super::AudioDecoder for OpusDecoder {
    fn decode(&mut self, input: &[u8]) -> AgoraResult<Vec<f32>> {
        self.decode(input)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate()
    }

    fn channels(&self) -> u8 {
        self.channels()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opus_config_default() {
        let config = OpusConfig::default();
        assert_eq!(config.sample_rate, OPUS_SAMPLE_RATE);
        assert_eq!(config.channels, OPUS_CHANNELS);
        assert_eq!(config.bitrate, OPUS_DEFAULT_BITRATE);
        assert_eq!(config.mode, OpusMode::Voip);
    }

    #[test]
    fn test_opus_config_with_bitrate() {
        let config = OpusConfig::default().with_bitrate(64000);
        assert_eq!(config.bitrate, 64000);
    }

    #[test]
    fn test_opus_config_bitrate_clamp() {
        let config = OpusConfig::default().with_bitrate(0);
        assert_eq!(config.bitrate, OPUS_MIN_BITRATE);

        let config = OpusConfig::default().with_bitrate(1000000);
        assert_eq!(config.bitrate, OPUS_MAX_BITRATE);
    }

    #[test]
    fn test_opus_encoder_creation() {
        let config = OpusConfig::default();
        let encoder = OpusEncoder::new(config);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_opus_decoder_creation() {
        let decoder = OpusDecoder::new(48000, 1);
        assert!(decoder.is_ok());
    }

    #[test]
    fn test_opus_encode_decode_roundtrip() {
        let mut encoder = OpusEncoder::new(OpusConfig::default()).unwrap();
        let mut decoder = OpusDecoder::new(48000, 1).unwrap();

        let input: Vec<f32> = (0..OPUS_FRAME_SIZE)
            .map(|i| (i as f32 * 0.001).sin() * 0.5)
            .collect();

        let encoded = encoder.encode(&input).unwrap();
        assert!(!encoded.is_empty());
        assert!(encoded.len() < input.len() * 4);

        let decoded = decoder.decode(&encoded).unwrap();
        assert!(!decoded.is_empty());

        let correlation: f32 = input.iter().zip(decoded.iter()).map(|(a, b)| a * b).sum();
        assert!(correlation > 0.0);
    }

    #[test]
    fn test_opus_encode_invalid_frame_size() {
        let mut encoder = OpusEncoder::new(OpusConfig::default()).unwrap();

        let small_frame = vec![0.0f32; 100];
        let result = encoder.encode(&small_frame);
        assert!(result.is_err());
    }

    #[test]
    fn test_opus_bitrate_change() {
        let mut encoder = OpusEncoder::new(OpusConfig::default()).unwrap();

        encoder.set_bitrate(64000).unwrap();
        assert_eq!(encoder.bitrate(), 64000);

        encoder.set_bitrate(24000).unwrap();
        assert_eq!(encoder.bitrate(), 24000);
    }

    #[test]
    fn test_opus_complexity_change() {
        let mut encoder = OpusEncoder::new(OpusConfig::default()).unwrap();

        encoder.set_complexity(10).unwrap();
        assert_eq!(encoder.config().complexity, 10);
    }

    #[test]
    fn test_opus_different_modes() {
        for mode in [OpusMode::Voip, OpusMode::Audio, OpusMode::LowDelay] {
            let config = OpusConfig::default().with_mode(mode);
            let encoder = OpusEncoder::new(config);
            assert!(encoder.is_ok());
        }
    }

    #[test]
    fn test_opus_fec_dtx_config() {
        let config = OpusConfig::default().with_fec(true).with_dtx(true);

        let encoder = OpusEncoder::new(config);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_opus_packet_loss_concealment() {
        let mut decoder = OpusDecoder::new(48000, 1).unwrap();

        let output = decoder.decode_packet_loss().unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_encoded_frame() {
        let mut encoder = OpusEncoder::new(OpusConfig::default()).unwrap();

        let input: Vec<f32> = (0..OPUS_FRAME_SIZE)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        let frame = encoder.encode_frame(&input).unwrap();
        assert!(!frame.data.is_empty());
        assert_eq!(frame.sequence, 1);
        assert_eq!(frame.bitrate, OPUS_DEFAULT_BITRATE);
    }
}
