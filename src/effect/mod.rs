use num::Num;
use std::ops::{Mul, Sub, Add};
use std::collections::VecDeque;
use std::clone::Clone;

use crate::{
    value::{ValueNode, CacheValue, Value},
    filter::{AllPass, RLPF, TrapezoidSVF},
    oscillator::BrownianNoise,
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


impl<'a, T: Default + Clone> ValueNode for Delay<'a, T> {
    type T = T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], offset: usize, samples: usize) {
        let mut delay: Vec<f64> = vec![0.0; samples];
        self.delay.fill_buffer(env, &mut delay, 0, samples);
        let mut input: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.input.fill_buffer(env, &mut input, 0, samples);
        for i in 0..samples {
            let idx = (delay[i] * env.sample_rate as f64) as usize + i;
            if idx >= self.buffer.len() {
                self.buffer.resize_with(idx + 1, || T::default());
            }
            self.buffer[idx] = input[i].clone();
            buffer[i+offset] = self.buffer.pop_front().unwrap_or_else(|| T::default());
        }
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
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], offset: usize, samples: usize) {
        self.output.fill_buffer(env, buffer, offset, samples);
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


impl<'a, T: Copy + Num + Default> ValueNode for RingModulator<'a, T> {
    type T = T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [T], offset: usize, samples: usize) {
        let mut input: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.input.fill_buffer(env, &mut input, 0, samples);
        let mut modulator: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.modulator.fill_buffer(env, &mut modulator, 0, samples);
        let mut mix: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.mix.fill_buffer(env, &mut mix, 0, samples);

        buffer[offset..offset+samples].iter_mut().zip(input).zip(modulator).zip(mix).for_each(|(((b, v), modulator), mix)| {
            *b = (T::one() - mix)*v + mix*modulator*v;
        });
    }
}


pub struct SoftClip<'a, T> {
    input: Value<'a, T>,
}

impl<'a, T> SoftClip<'a, T> {
    pub fn new(input: impl  Into<Value<'a, T>>) -> Self {
        Self {
            input: input.into(),
        }
    }
}

impl<'a, T: Default + Into<f64> + From<f64>> ValueNode for SoftClip<'a, T> {
    type T = T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [T], offset: usize, samples: usize) {
        let mut input: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.input.fill_buffer(env, &mut input, 0, samples);
        buffer[offset..offset+samples].iter_mut().zip(input).for_each(|(b, v)| {
            let v: f64 = v.into();
            *b = (v - v.powf(3.0)/3.0).into();
        });
    }
}

pub fn old_timeify<'a>(sig: impl Into<Value<'a, f64>>, overdrive: f64) -> Value<'a, f64> {
    let mut sig: Value<f64> = sig.into();
    sig = TrapezoidSVF::low_pass(sig, 800.0, 0.8).into();
    sig = TrapezoidSVF::high(sig, 100.0, 0.8).into();
    let impulses: Value<f64> = BrownianNoise::new(0.8, 0.5).into();
    let low_crackle: Value<f64> = BrownianNoise::new(20.0, 0.1).into();
    let low_crackle2: Value<f64> = BrownianNoise::new(30.0, 0.1).into();
    sig = SoftClip::new(sig * overdrive).into();
    sig * 1.0 + (impulses * 0.6 + low_crackle * 0.5 + low_crackle2 * 0.5) * 0.6
}
