mod opus;

pub use opus::{EncodedFrame, OpusConfig, OpusDecoder, OpusEncoder, OpusMode, OPUS_FRAME_SIZE};

pub trait AudioEncoder: Send {
    fn encode(&mut self, input: &[f32]) -> crate::error::AgoraResult<Vec<u8>>;
    fn set_bitrate(&mut self, bitrate: i32) -> crate::error::AgoraResult<()>;
    fn bitrate(&self) -> i32;
}

pub trait AudioDecoder: Send {
    fn decode(&mut self, input: &[u8]) -> crate::error::AgoraResult<Vec<f32>>;
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u8;
}
