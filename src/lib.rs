pub mod value;
pub mod oscillator;
pub mod filter;
pub mod envelope;

pub struct Env {
    pub sample_rate: u32,
}
impl Env {
    pub fn new(sample_rate: u32) -> Self {
        Env {
            sample_rate,
        }
    }
}
