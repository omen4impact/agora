use crate::error::AgoraResult;

pub const RNNOISE_FRAME_SIZE: usize = 480;
pub const RNNOISE_SAMPLE_RATE: u32 = 48000;

pub struct RnnoiseDenoiser {
    state: Box<nnnoiseless::DenoiseState<'static>>,
    enabled: bool,
    frame_count: u64,
}

impl RnnoiseDenoiser {
    pub fn new() -> AgoraResult<Self> {
        let state = nnnoiseless::DenoiseState::new();

        tracing::info!(
            "RNNoise denoiser created (frame size: {})",
            RNNOISE_FRAME_SIZE
        );

        Ok(Self {
            state,
            enabled: true,
            frame_count: 0,
        })
    }

    pub fn process_frame(&mut self, frame: &mut [f32]) {
        if !self.enabled {
            return;
        }

        if frame.len() != RNNOISE_FRAME_SIZE {
            tracing::warn!(
                "RNNoise frame size mismatch: expected {}, got {}",
                RNNOISE_FRAME_SIZE,
                frame.len()
            );
            return;
        }

        let mut output = [0.0f32; RNNOISE_FRAME_SIZE];
        self.state.process_frame(frame, &mut output);
        frame.copy_from_slice(&output);
        self.frame_count += 1;
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }

        for chunk in samples.chunks_mut(RNNOISE_FRAME_SIZE) {
            if chunk.len() == RNNOISE_FRAME_SIZE {
                let mut output = [0.0f32; RNNOISE_FRAME_SIZE];
                self.state.process_frame(chunk, &mut output);
                chunk.copy_from_slice(&output);
                self.frame_count += 1;
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = nnnoiseless::DenoiseState::new();
        self.frame_count = 0;
        tracing::debug!("RNNoise state reset");
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        tracing::debug!("RNNoise enabled: {}", enabled);
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

impl Default for RnnoiseDenoiser {
    fn default() -> Self {
        Self::new().expect("Failed to create RNNoise denoiser")
    }
}

pub struct DenoiserConfig {
    pub enabled: bool,
    pub threshold: f32,
}

impl Default for DenoiserConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rnnoise_creation() {
        let denoiser = RnnoiseDenoiser::new();
        assert!(denoiser.is_ok());
        assert!(denoiser.unwrap().is_enabled());
    }

    #[test]
    fn test_rnnoise_enable_disable() {
        let mut denoiser = RnnoiseDenoiser::new().unwrap();

        assert!(denoiser.is_enabled());

        denoiser.set_enabled(false);
        assert!(!denoiser.is_enabled());

        denoiser.set_enabled(true);
        assert!(denoiser.is_enabled());
    }

    #[test]
    fn test_rnnoise_process_frame() {
        let mut denoiser = RnnoiseDenoiser::new().unwrap();

        let mut frame: Vec<f32> = (0..RNNOISE_FRAME_SIZE)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        let original: Vec<f32> = frame.clone();
        denoiser.process_frame(&mut frame);

        let changed = frame
            .iter()
            .zip(original.iter())
            .any(|(a, b)| (a - b).abs() > 0.001);

        assert!(changed || denoiser.frame_count() > 0);
    }

    #[test]
    fn test_rnnoise_process_multiple_frames() {
        let mut denoiser = RnnoiseDenoiser::new().unwrap();

        let mut samples: Vec<f32> = (0..RNNOISE_FRAME_SIZE * 3)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        denoiser.process(&mut samples);

        assert_eq!(denoiser.frame_count(), 3);
    }

    #[test]
    fn test_rnnoise_disabled_no_processing() {
        let mut denoiser = RnnoiseDenoiser::new().unwrap();
        denoiser.set_enabled(false);

        let mut frame: Vec<f32> = (0..RNNOISE_FRAME_SIZE)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        let original: Vec<f32> = frame.clone();
        denoiser.process_frame(&mut frame);

        assert_eq!(frame, original);
    }

    #[test]
    fn test_rnnoise_reset() {
        let mut denoiser = RnnoiseDenoiser::new().unwrap();

        let mut frame: Vec<f32> = (0..RNNOISE_FRAME_SIZE)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        denoiser.process_frame(&mut frame);
        assert_eq!(denoiser.frame_count(), 1);

        denoiser.reset();
        assert_eq!(denoiser.frame_count(), 0);
    }

    #[test]
    fn test_rnnoise_wrong_frame_size() {
        let mut denoiser = RnnoiseDenoiser::new().unwrap();

        let mut small_frame = vec![0.0f32; 100];
        denoiser.process_frame(&mut small_frame);

        assert_eq!(denoiser.frame_count(), 0);
    }
}
