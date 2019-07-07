use std::collections::VecDeque;
use std::clone::Clone;
use rand::Rng;

use crate::{
    value::{ValueNode, Value, CacheValue},
    filter::{AllPass, RLPF},
    Env,
};

pub struct Delay<T> {
    input: Value<T>,
    buffer: VecDeque<T>,
    delay: Value<f64>,
}

impl<T> Delay<T> {
    pub fn new(input: Value<T>, delay: Value<f64>) -> Self {
        Delay {
            input,
            buffer: VecDeque::new(),
            delay,
        }
    }
}


impl<T> ValueNode<T> for Delay<T> where T: From<f64> + 'static {
    fn next(&mut self, env: &Env) -> T {
        let delay = self.delay.next(env);
        let idx = (delay * env.sample_rate as f64) as usize;
        if idx >= self.buffer.len() {
            self.buffer.resize_with(idx + 1, || 0.0.into());
        }
        self.buffer[idx] = self.input.next(env);
        self.buffer.pop_front().unwrap_or_else(|| 0.0.into())
    }

    fn to_value(self) -> Value<T> {
        Value(Box::new(self))
    }
}

pub struct Reverb {
    output: Value<f64>,
}

impl Reverb {
    pub fn new(input: Value<f64>, mix: f64, predelay: f64, lpf: f64, revtime: f64) -> Self {
        let dry = CacheValue::new(input);
        let mut temp = Delay::new(dry.clone().to_value(), predelay.into()).to_value();
        let mut wet: Value<f64> = 0.0.into();

        for _ in 0..16 {
            let ltemp = AllPass::new(temp, rand::thread_rng().gen_range(0.001, 0.05), revtime).to_value();
            let cache = CacheValue::new(RLPF::low_pass(ltemp, lpf.into(), 5.0.into()).to_value());
            wet = wet + cache.clone().to_value();
            temp = cache.to_value();
        }

        let output = dry.to_value() * mix.to_value() + wet * (1.0 - mix).to_value();


        Reverb {
            output,
        }
    }
}


impl ValueNode<f64> for Reverb {
    fn next(&mut self, env: &Env) -> f64 {
        self.output.next(env)
    }

    fn to_value(self) -> Value<f64> {
        Value(Box::new(self))
    }
}
