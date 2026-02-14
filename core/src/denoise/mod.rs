mod rnnoise;

pub use rnnoise::{RnnoiseDenoiser, RNNOISE_FRAME_SIZE};

pub trait Denoiser: Send {
    fn process(&mut self, frame: &mut [f32]);
    fn reset(&mut self);
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
}
