use std::ops::{Mul, Sub, Add};
use std::collections::VecDeque;
use std::clone::Clone;

use crate::{
    value::{ValueNode, CacheValue, Value},
    filter::{AllPass, RLPF},
    Env,
};

pub struct Delay<'a, T> {
    input: Value<'a, T>,
    buffer: VecDeque<T>,
    delay: Value<'a, f64>,
}

impl<'a, T: Into<f64>> Delay<'a, T> {
    pub fn new(input: impl Into<Value<'a, T>>, delay: impl Into<Value<'a, f64>>) -> Self {
        Delay {
            input: input.into(),
            buffer: VecDeque::new(),
            delay: delay.into(),
        }
    }
}


impl<'a, T: From<f64>> ValueNode for Delay<'a, T> {
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

pub struct Reverb<'a> {
    output: Value<'a, f64>,
}

impl<'a> Reverb<'a> {
    pub fn new(input: impl Into<Value<'a, f64>>, mix: f64, predelay: f64, lpf: f64, revtime: f64) -> Self {
        let dry = CacheValue::new(input);
        let mut temp: Value<f64> = Delay::new(dry.clone(), predelay).into();
        let mut wet: Value<f64> = 0.0.into();
        let things = vec![0.038045169615104804, 0.02999076016847762, 0.04963873923379772, 0.04368894979626656, 0.007460425959828037, 0.02817080130412364, 0.00657126832222354, 0.04779429369666802, 0.004010513054838128, 0.01541601071664956, 0.011602441530870984, 0.0012122872292874213, 0.025404225677194647, 0.0017341472693168261, 0.01003645759720834, 0.04604357296027947];

        for r in things {
            let ltemp = AllPass::new(temp, r, revtime);
            let cache = CacheValue::new(
                RLPF::new(ltemp, lpf, 50.0)
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


impl<'a> ValueNode for Reverb<'a> {
    type T = f64;
    fn next(&mut self, env: &Env) -> Self::T {
        self.output.next(env)
    }
}

pub struct RingModulator<'a, T> {
    input: Value<'a, T>,
    modulator: Value<'a, T>,
    mix: Value<'a, T>,
}

impl<'a, T> RingModulator<'a, T> {
    pub fn new(input: impl Into<Value<'a, T>>, modulator: impl Into<Value<'a, T>>, mix: impl Into<Value<'a, T>>) -> Self {
        Self {
            input: input.into(),
            modulator: modulator.into(),
            mix: mix.into(),
        }
    }
}


impl<'a, T: Copy + Sub<Output=T> + Mul<Output=T> + Add<Output=T> + From<f64>> ValueNode for RingModulator<'a, T> {
    type T = T;
    fn next(&mut self, env: &Env) -> Self::T {
        let v = self.input.next(env);
        let m = self.modulator.next(env);
        let mix = self.mix.next(env);
        (T::from(1.0)-mix)*v + mix*m*v
    }
}
