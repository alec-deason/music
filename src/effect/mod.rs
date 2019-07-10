use std::ops::{Mul, Sub, Add};
use std::collections::VecDeque;
use std::clone::Clone;

use crate::{
    value::{ValueNode, CacheValue, Value},
    filter::{AllPass, RLPF},
    Env,
};

pub struct Delay<T> {
    input: Value<T>,
    buffer: VecDeque<T>,
    delay: Value<f64>,
}

impl<T: Into<f64>> Delay<T> {
    pub fn new(input: Value<T>, delay: Value<f64>) -> Self {
        Delay {
            input: input,
            buffer: VecDeque::new(),
            delay: delay,
        }
    }
}


impl<T: From<f64>> ValueNode for Delay<T> {
    type T = T;
    fn next(&mut self, env: &Env) -> Self::T {
        let delay: f64 = self.delay.next(env).into();
        let idx = (delay * env.sample_rate as f64) as usize;
        if idx >= self.buffer.len() {
            self.buffer.resize_with(idx + 1, || 0.0.into());
        }
        self.buffer[idx] = self.input.next(env);
        self.buffer.pop_front().unwrap_or_else(|| 0.0.into())
    }
}

pub struct Reverb {
    output: Value<f64>,
}

impl Reverb {
    pub fn new(input: Value<f64>, mix: f64, predelay: f64, lpf: f64, revtime: f64) -> Self {
        let dry = CacheValue::new(input);
        let mut temp: Value<_> = Delay::new(dry.clone().into(), predelay.into()).into();
        let mut wet: Value<f64> = 0.0.into();
        let things = vec![0.038045169615104804, 0.02999076016847762, 0.04963873923379772, 0.04368894979626656, 0.007460425959828037, 0.02817080130412364, 0.00657126832222354, 0.04779429369666802, 0.004010513054838128, 0.01541601071664956, 0.011602441530870984, 0.0012122872292874213, 0.025404225677194647, 0.0017341472693168261, 0.01003645759720834, 0.04604357296027947];

        for r in things {
            let ltemp = AllPass::new(temp, r, revtime);
            let cache = CacheValue::new(
                RLPF::new(ltemp.into(), lpf.into(), 50.0.into()).into()
            );
            wet = wet + Value::<f64>::from(cache.clone());
            temp = cache.into();
        }

        let output: Value<_> = Value::<f64>::from(dry) * Value::<f64>::from(mix) + wet * (Value::<f64>::from(1.0) - Value::<f64>::from(mix));


        Reverb {
            output: output,
        }
    }
}


impl ValueNode for Reverb {
    type T = f64;
    fn next(&mut self, env: &Env) -> Self::T {
        self.output.next(env)
    }
}

pub struct RingModulator<T> {
    input: Value<T>,
    modulator: Value<T>,
    mix: Value<T>,
}

impl<T> RingModulator<T> {
    pub fn new(input: Value<T>, modulator: Value<T>, mix: Value<T>) -> Self {
        Self {
            input: input,
            modulator: modulator,
            mix: mix,
        }
    }
}


impl<T: Copy + Sub<Output=T> + Mul<Output=T> + Add<Output=T> + From<f64>> ValueNode for RingModulator<T> {
    type T = T;
    fn next(&mut self, env: &Env) -> Self::T {
        let v = self.input.next(env);
        let m = self.modulator.next(env);
        let mix = self.mix.next(env);
        (T::from(1.0)-mix)*v + mix*m*v
    }
}
