#![feature(duration_float)]

use std::time::Duration;

pub mod composition;
pub mod effect;
pub mod envelope;
pub mod filter;
pub mod note;
pub mod oscillator;
pub mod sequence;
pub mod value;

#[derive(Clone, Debug)]
pub struct Env {
    pub sample_rate: u32,
    pub time: Duration,
}
impl Env {
    pub fn new(sample_rate: u32) -> Self {
        Env {
            sample_rate,
            time: Duration::new(0, 0),
        }
    }
}
