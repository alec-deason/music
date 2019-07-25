#![feature(duration_float, const_generics)]

use std::time::Duration;

pub mod value;
pub mod oscillator;
pub mod filter;
pub mod envelope;
pub mod effect;
pub mod sequence;
pub mod note;
pub mod composition;

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
