use crate::error::{AgoraResult, Error};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub const SAMPLE_RATE: u32 = 48000;
pub const CHANNELS: u16 = 1;
pub const FRAME_SIZE: usize = 960;
pub const BITRATE: i32 = 32000;

#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_size: usize,
    pub bitrate: i32,
    pub enable_noise_suppression: bool,
    pub enable_echo_cancellation: bool,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: SAMPLE_RATE,
            channels: CHANNELS,
            frame_size: FRAME_SIZE,
            bitrate: BITRATE,
            enable_noise_suppression: true,
            enable_echo_cancellation: true,
            input_device: None,
            output_device: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_input: bool,
    pub is_default: bool,
    pub channels: u16,
    pub sample_rate: u32,
}

pub struct AudioDevice {
    device: Device,
    info: AudioDeviceInfo,
}

impl AudioDevice {
    pub fn input_devices() -> AgoraResult<Vec<AudioDeviceInfo>> {
        let host = cpal::default_host();
        let default_input = host.default_input_device().and_then(|d| d.name().ok());

        let mut devices = Vec::new();

        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    let is_default = default_input.as_ref().map(|d| d == &name).unwrap_or(false);

                    let (channels, sample_rate) = device
                        .default_input_config()
                        .map(|c| (c.channels(), c.sample_rate().0))
                        .unwrap_or((1, 48000));

                    devices.push(AudioDeviceInfo {
                        name,
                        is_input: true,
                        is_default,
                        channels,
                        sample_rate,
                    });
                }
            }
        }

        Ok(devices)
    }

    pub fn output_devices() -> AgoraResult<Vec<AudioDeviceInfo>> {
        let host = cpal::default_host();
        let default_output = host.default_output_device().and_then(|d| d.name().ok());

        let mut devices = Vec::new();

        if let Ok(output_devices) = host.output_devices() {
            for device in output_devices {
                if let Ok(name) = device.name() {
                    let is_default = default_output.as_ref().map(|d| d == &name).unwrap_or(false);

                    let (channels, sample_rate) = device
                        .default_output_config()
                        .map(|c| (c.channels(), c.sample_rate().0))
                        .unwrap_or((2, 48000));

                    devices.push(AudioDeviceInfo {
                        name,
                        is_input: false,
                        is_default,
                        channels,
                        sample_rate,
                    });
                }
            }
        }

        Ok(devices)
    }

    pub fn default_input() -> AgoraResult<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| Error::Audio("No default input device".to_string()))?;

        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let config = device
            .default_input_config()
            .map_err(|e| Error::Audio(format!("Failed to get input config: {}", e)))?;

        Ok(Self {
            device,
            info: AudioDeviceInfo {
                name,
                is_input: true,
                is_default: true,
                channels: config.channels(),
                sample_rate: config.sample_rate().0,
            },
        })
    }

    pub fn default_output() -> AgoraResult<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| Error::Audio("No default output device".to_string()))?;

        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let config = device
            .default_output_config()
            .map_err(|e| Error::Audio(format!("Failed to get output config: {}", e)))?;

        Ok(Self {
            device,
            info: AudioDeviceInfo {
                name,
                is_input: false,
                is_default: true,
                channels: config.channels(),
                sample_rate: config.sample_rate().0,
            },
        })
    }

    pub fn info(&self) -> &AudioDeviceInfo {
        &self.info
    }
}

pub type AudioFrame = Vec<f32>;

#[derive(Debug, Clone)]
pub struct AudioStats {
    pub frames_processed: u64,
    pub frames_dropped: u64,
    pub average_latency_ms: f64,
    pub peak_latency_ms: f64,
}

impl AudioStats {
    fn new() -> Self {
        Self {
            frames_processed: 0,
            frames_dropped: 0,
            average_latency_ms: 0.0,
            peak_latency_ms: 0.0,
        }
    }
}

enum AudioCommand {
    Stop,
    SetNoiseGate(f32),
}

struct AudioBackend {
    input_stream: Option<Stream>,
    output_stream: Option<Stream>,
    input_buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    output_buffer: Arc<std::sync::Mutex<Vec<f32>>>,
}

impl AudioBackend {
    fn new() -> Self {
        Self {
            input_stream: None,
            output_stream: None,
            input_buffer: Arc::new(std::sync::Mutex::new(Vec::new())),
            output_buffer: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn start(&mut self, config: &AudioConfig) -> AgoraResult<()> {
        self.start_input_stream(config)?;
        self.start_output_stream()?;
        tracing::info!("Audio backend started");
        Ok(())
    }

    fn start_input_stream(&mut self, config: &AudioConfig) -> AgoraResult<()> {
        let device = AudioDevice::default_input()?;
        let supported_config = device
            .device
            .default_input_config()
            .map_err(|e| Error::Audio(format!("Input config error: {}", e)))?;

        let sample_format = supported_config.sample_format();
        let stream_config: StreamConfig = supported_config.into();

        let buffer = self.input_buffer.clone();
        let noise_gate = 0.01;
        let enable_noise_suppression = config.enable_noise_suppression;

        let stream = match sample_format {
            SampleFormat::F32 => device.device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buf = match buffer.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            tracing::error!("Audio input mutex poisoned: {}", e);
                            return;
                        }
                    };
                    for &sample in data {
                        let processed = if enable_noise_suppression {
                            apply_noise_gate(sample, noise_gate)
                        } else {
                            sample
                        };
                        buf.push(processed);
                    }

                    if buf.len() > FRAME_SIZE * 10 {
                        buf.drain(0..FRAME_SIZE);
                    }
                },
                |err| tracing::error!("Input stream error: {}", err),
                None,
            ),
            SampleFormat::I16 => device.device.build_input_stream(
                &stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mut buf = match buffer.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            tracing::error!("Audio input mutex poisoned: {}", e);
                            return;
                        }
                    };
                    for &sample in data {
                        let sample_f32 = sample as f32 / i16::MAX as f32;
                        let processed = if enable_noise_suppression {
                            apply_noise_gate(sample_f32, noise_gate)
                        } else {
                            sample_f32
                        };
                        buf.push(processed);
                    }

                    if buf.len() > FRAME_SIZE * 10 {
                        buf.drain(0..FRAME_SIZE);
                    }
                },
                |err| tracing::error!("Input stream error: {}", err),
                None,
            ),
            _ => return Err(Error::Audio("Unsupported sample format".to_string())),
        }
        .map_err(|e| Error::Audio(format!("Failed to build input stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| Error::Audio(format!("Failed to play input stream: {}", e)))?;
        self.input_stream = Some(stream);

        tracing::info!("Input stream started on device: {}", device.info.name);
        Ok(())
    }

    fn start_output_stream(&mut self) -> AgoraResult<()> {
        let device = AudioDevice::default_output()?;
        let supported_config = device
            .device
            .default_output_config()
            .map_err(|e| Error::Audio(format!("Output config error: {}", e)))?;

        let sample_format = supported_config.sample_format();
        let stream_config: StreamConfig = supported_config.into();

        let buffer = self.output_buffer.clone();

        let stream = match sample_format {
            SampleFormat::F32 => device.device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut buf = match buffer.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            tracing::error!("Audio output mutex poisoned: {}", e);
                            data.fill(0.0);
                            return;
                        }
                    };
                    let len = data.len().min(buf.len());
                    data[..len].copy_from_slice(&buf[..len]);
                    buf.drain(0..len);
                },
                |err| tracing::error!("Output stream error: {}", err),
                None,
            ),
            SampleFormat::I16 => device.device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let mut buf = match buffer.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            tracing::error!("Audio output mutex poisoned: {}", e);
                            data.fill(0);
                            return;
                        }
                    };
                    let to_drain = data.len().min(buf.len());
                    for (i, sample) in data.iter_mut().enumerate() {
                        *sample = if i < buf.len() {
                            let clamped = buf[i].clamp(-1.0, 1.0);
                            (clamped * i16::MAX as f32) as i16
                        } else {
                            0
                        };
                    }
                    if to_drain > 0 {
                        buf.drain(0..to_drain);
                    }
                },
                |err| tracing::error!("Output stream error: {}", err),
                None,
            ),
            _ => return Err(Error::Audio("Unsupported sample format".to_string())),
        }
        .map_err(|e| Error::Audio(format!("Failed to build output stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| Error::Audio(format!("Failed to play output stream: {}", e)))?;
        self.output_stream = Some(stream);

        tracing::info!("Output stream started on device: {}", device.info.name);
        Ok(())
    }

    fn stop(&mut self) {
        self.input_stream = None;
        self.output_stream = None;
        tracing::info!("Audio backend stopped");
    }
}

pub struct AudioPipeline {
    config: AudioConfig,
    input_buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    output_buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    stats: AudioStats,
    noise_gate_threshold: f32,
    is_running: bool,
    command_tx: Option<Sender<AudioCommand>>,
    _thread_handle: Option<JoinHandle<()>>,
}

impl AudioPipeline {
    pub fn new(config: AudioConfig) -> Self {
        Self {
            config,
            input_buffer: Arc::new(std::sync::Mutex::new(Vec::new())),
            output_buffer: Arc::new(std::sync::Mutex::new(Vec::new())),
            stats: AudioStats::new(),
            noise_gate_threshold: 0.01,
            is_running: false,
            command_tx: None,
            _thread_handle: None,
        }
    }

    pub fn start(&mut self) -> AgoraResult<()> {
        if self.is_running {
            return Ok(());
        }

        let config = self.config.clone();
        let input_buffer = self.input_buffer.clone();
        let output_buffer = self.output_buffer.clone();
        let (tx, rx): (Sender<AudioCommand>, Receiver<AudioCommand>) = mpsc::channel();
        let (ready_tx, ready_rx): (Sender<AgoraResult<()>>, Receiver<AgoraResult<()>>) =
            mpsc::channel();

        let handle = thread::spawn(move || {
            let mut backend = AudioBackend::new();
            backend.input_buffer = input_buffer;
            backend.output_buffer = output_buffer;

            if let Err(e) = backend.start(&config) {
                tracing::error!("Failed to start audio backend: {}", e);
                let _ = ready_tx.send(Err(e));
                return;
            }

            let _ = ready_tx.send(Ok(()));

            loop {
                match rx.try_recv() {
                    Ok(AudioCommand::Stop) | Err(mpsc::TryRecvError::Disconnected) => {
                        backend.stop();
                        break;
                    }
                    Ok(AudioCommand::SetNoiseGate(threshold)) => {
                        tracing::info!("Noise gate threshold set to {}", threshold);
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        });

        match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => {
                self.command_tx = Some(tx);
                self._thread_handle = Some(handle);
                self.is_running = true;
                tracing::info!("Audio pipeline started");
                Ok(())
            }
            Ok(Err(e)) => {
                handle.join().ok();
                Err(e)
            }
            Err(_) => {
                tracing::error!("Audio pipeline start timeout");
                Err(Error::Audio("Audio start timeout".to_string()))
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.command_tx.take() {
            let _ = tx.send(AudioCommand::Stop);
        }
        self.is_running = false;
        tracing::info!("Audio pipeline stopped");
    }

    pub fn capture_frame(&mut self) -> Option<AudioFrame> {
        let mut buffer = match self.input_buffer.lock() {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to lock input buffer: {}", e);
                return None;
            }
        };

        if buffer.len() >= self.config.frame_size {
            let frame: Vec<f32> = buffer.drain(0..self.config.frame_size).collect();
            self.stats.frames_processed += 1;
            Some(frame)
        } else {
            None
        }
    }

    pub fn play_frame(&mut self, frame: AudioFrame) {
        let mut buffer = match self.output_buffer.lock() {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to lock output buffer: {}", e);
                return;
            }
        };
        buffer.extend(frame);
    }

    pub fn get_stats(&self) -> &AudioStats {
        &self.stats
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn set_noise_gate_threshold(&mut self, threshold: f32) {
        self.noise_gate_threshold = threshold;
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(AudioCommand::SetNoiseGate(threshold));
        }
    }
}

fn apply_noise_gate(sample: f32, threshold: f32) -> f32 {
    let threshold = threshold.clamp(0.0, 0.99);
    if sample.abs() < threshold {
        0.0
    } else {
        let ratio = (sample.abs() - threshold) / (1.0 - threshold);
        sample * ratio
    }
}

pub fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

pub fn calculate_db(rms: f32) -> f32 {
    if rms <= 0.0 {
        return -100.0;
    }
    20.0 * rms.log10()
}

pub fn normalize_audio(samples: &mut [f32], target_peak: f32) {
    if samples.is_empty() {
        return;
    }

    let max = samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, |a, b| a.max(b));

    if max > 0.0 {
        let scale = target_peak / max;
        for sample in samples.iter_mut() {
            *sample *= scale;
        }
    }
}

pub fn mix_audio(inputs: &[&[f32]], weights: &[f32]) -> Vec<f32> {
    if inputs.is_empty() {
        return Vec::new();
    }

    let len = inputs[0].len();
    let mut output = vec![0.0f32; len];

    for (input, &weight) in inputs.iter().zip(weights.iter()) {
        for (i, &sample) in input.iter().enumerate() {
            if i < len {
                output[i] += sample * weight;
            }
        }
    }

    for sample in output.iter_mut() {
        *sample = sample.clamp(-1.0, 1.0);
    }

    output
}

pub fn resample_nearest(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let new_len = (samples.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_idx = (i as f64 / ratio) as usize;
        output.push(samples[src_idx.min(samples.len() - 1)]);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, SAMPLE_RATE);
        assert_eq!(config.channels, CHANNELS);
        assert_eq!(config.frame_size, FRAME_SIZE);
    }

    #[test]
    fn test_calculate_rms() {
        let silence = vec![0.0; 100];
        assert_eq!(calculate_rms(&silence), 0.0);

        let max_signal = vec![1.0; 100];
        assert!((calculate_rms(&max_signal) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_db() {
        assert_eq!(calculate_db(0.0), -100.0);
        assert!((calculate_db(1.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_normalize_audio() {
        let mut samples = vec![0.5, -0.5, 0.25];
        normalize_audio(&mut samples, 1.0);

        let max = samples
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, |a, b| a.max(b));
        assert!((max - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_mix_audio() {
        let input1 = vec![1.0, 1.0, 1.0];
        let input2 = vec![1.0, 1.0, 1.0];

        let mixed = mix_audio(&[&input1, &input2], &[0.5, 0.5]);

        assert_eq!(mixed.len(), 3);
        assert!((mixed[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_noise_gate() {
        assert_eq!(apply_noise_gate(0.001, 0.01), 0.0);
        assert!(apply_noise_gate(0.5, 0.01).abs() > 0.0);
    }

    #[test]
    fn test_resample_nearest() {
        let samples = vec![1.0, 2.0, 3.0, 4.0];
        let resampled = resample_nearest(&samples, 48000, 24000);

        assert!(resampled.len() < samples.len());
    }
}
