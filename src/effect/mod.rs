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
        let things = vec![0.038045169615104804, 0.02999076016847762, 0.04963873923379772, 0.04368894979626656, 0.007460425959828037, 0.02817080130412364, 0.00657126832222354, 0.04779429369666802, 0.004010513054838128, 0.01541601071664956, 0.011602441530870984, 0.0012122872292874213, 0.025404225677194647, 0.0017341472693168261, 0.01003645759720834, 0.04604357296027947];

        for r in things {
            let ltemp = AllPass::new(temp, r, revtime).to_value();
            let cache = CacheValue::new(RLPF::low_pass(ltemp, lpf.into(), 50.0.into()).to_value());
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
