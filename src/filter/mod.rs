use std::ops::{Add, Mul, Neg};
use std::collections::VecDeque;
use std::f64::consts::PI;

use crate::{
    value::{ValueNode, Value},
    Env,
};


pub struct RLPF<'a> {
    input: Value<'a, f64>,
    cutoff: Value<'a, f64>,
    q: Value<'a, f64>,
    cached_cutoff: f64,
    cached_q: f64,

    a0: f64,
    b1: f64,
    b2: f64,

    y0: f64,
    y1: f64,
    y2: f64,
}

impl<'a> RLPF<'a> {
    pub fn new(input: impl Into<Value<'a, f64>>, cutoff: impl Into<Value<'a, f64>>, q: impl Into<Value<'a, f64>>) -> Self {
        RLPF {
            input: input.into(),
            cutoff: cutoff.into(),
            q: q.into(),
            cached_cutoff: std::f64::NAN,
            cached_q: std::f64::NAN,
            a0: std::f64::NAN,
            b1: std::f64::NAN,
            b2: std::f64::NAN,

            y0: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn parameters(&mut self, env: &Env, samples: usize) -> Vec<(f64, f64, f64)> {
        let mut cutoff: Vec<f64> = vec![0.0; samples];
        self.cutoff.fill_buffer(env, &mut cutoff, samples);
        let mut q: Vec<f64> = vec![0.0; samples];
        self.q.fill_buffer(env, &mut q, samples);

        let mut result = vec![(0.0, 0.0, 0.0); samples];
        for i in 0..samples {
            let q = q[i];
            let cutoff = cutoff[i];
            if (cutoff != self.cached_cutoff) | (q != self.cached_q) {
                self.cached_cutoff = cutoff;
                self.cached_q = q;
                let pfreq = PI * cutoff/44100.0;
                let d = pfreq.tan();
                let c = (1.0 -d) / (1.0 + d);
                let cosf = pfreq.cos();

                self.b1 = (1.0 + c) * cosf;
                self.b2 = -c;
                self.a0 = (1.0 + c - self.b1) * 0.25;
            }
            result[i] = (self.a0, self.b1, self.b2);
        }
        result
    }
}


impl<'a> ValueNode for RLPF<'a> {
    type T = f64;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], samples: usize) {
        let parameters = self.parameters(env, samples);
        let mut input: Vec<f64> = (0..samples).map(|_| Self::T::default()).collect();
        self.input.fill_buffer(env, &mut input, samples);

        for i in 0..samples {
            let v0 = input[i];
            let (a0, b1, b2) = parameters[i];

            self.y0 = a0 * v0 + b1 * self.y1 + b2 * self.y2;
            let out = self.y0 + 2.0 * self.y1 + self.y2;
            self.y2 = self.y1;
            self.y1 = self.y0;
            buffer[i] = out;
        }
    }
}

pub struct AllPass<'a, T> {
    input: Value<'a, T>,
    k: T,

    buff: VecDeque<T>,
}

impl<'a, T: From<f64>> AllPass<'a, T> {
    pub fn new(input: impl Into<Value<'a, T>>, delay: f64, decay: f64) -> Self {
        let k = 0.001f64.powf(delay / decay.abs()) * decay.signum();
        AllPass {
            input: input.into(),
            k: k.into(),

            buff: (0..(44100.0*delay) as usize).map(|_| 0.0.into()).collect(),
        }
    }
}

impl<'a, T: Add<Output = T> + Mul<Output = T> + Neg<Output = T> + Into<f64> + Default + Copy> ValueNode for AllPass<'a, T> {
    type T = T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [T], samples: usize) {
        let mut input: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.input.fill_buffer(env, &mut input, samples);

        for i in 0..samples {
            let s_d = self.buff.pop_front().unwrap_or_else(|| T::default());
            let s:T = input[i] + self.k * s_d;
            let y:T = -self.k * self.buff[0] + s_d;
            self.buff.push_back(s);
            buffer[i] = y;
        }
    }
}

enum FilterType {
    Low,
    Band,
    High,
    Notch,
    Peak,
    All,
}

pub struct TrapezoidSVF<'a> {
    input: Value<'a, f64>,
    frequency: Value<'a, f64>,
    cached_frequency: f64, 
    q: Value<'a, f64>,
    cached_q: f64,
    filter_type: FilterType,
    k: f64,
    a1: f64,
    a2: f64,
    a3: f64,

    ic1eq: f64,
    ic2eq: f64,
}

//From: http://www.cytomic.com/files/dsp/SvfLinearTrapOptimised2.pdf
impl<'a> TrapezoidSVF<'a> {
    fn new(filter_type: FilterType, input: impl Into<Value<'a, f64>>, frequency: impl Into<Value<'a, f64>>, q: impl Into<Value<'a, f64>>) -> Self {
        TrapezoidSVF {
            input: input.into(),
            frequency: frequency.into(),
            q: q.into(),
            cached_q: std::f64::NAN,
            cached_frequency: std::f64::NAN,
            filter_type: filter_type,
            k: std::f64::NAN,
            a1: std::f64::NAN,
            a2: std::f64::NAN,
            a3: std::f64::NAN,

            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    fn parameters(&mut self, env: &Env, samples: usize) -> Vec<(f64, f64, f64, f64)> {
        let mut frequency: Vec<f64> = vec![0.0; samples];
        self.frequency.fill_buffer(env, &mut frequency, samples);
        let mut q: Vec<f64> = vec![0.0; samples];
        self.q.fill_buffer(env, &mut q, samples);

        let mut result = vec![(0.0, 0.0, 0.0, 0.0); samples];
        for i in 0..samples {
            let frequency = frequency[i];
            let q = q[i];

            if (frequency != self.cached_frequency) | (q != self.cached_q) {
                self.cached_frequency = frequency;
                self.cached_q = q;

                let g = (PI * frequency/env.sample_rate as f64).tan();
                self.k = 1.0 / q;
                self.a1 = 1.0 / (1.0 + g * (g + self.k));
                self.a2 = g * self.a1;
                self.a3 = g * self.a2;
            }

            result[i] = (self.k, self.a1, self.a2, self.a3);
        }
        result
    }

    pub fn low_pass(input: impl Into<Value<'a, f64>>, cutoff: impl Into<Value<'a, f64>>, q: impl Into<Value<'a, f64>>) -> Self {
        Self::new(FilterType::Low, input, cutoff, q)
    }

    pub fn band(input: impl Into<Value<'a, f64>>, frequency: impl Into<Value<'a, f64>>, q: impl Into<Value<'a, f64>>) -> Self {
        Self::new(FilterType::Band, input, frequency, q)
    }

    pub fn high(input: impl Into<Value<'a, f64>>, frequency: impl Into<Value<'a, f64>>, q: impl Into<Value<'a, f64>>) -> Self {
        Self::new(FilterType::High, input, frequency, q)
    }

    pub fn notch(input: impl Into<Value<'a, f64>>, frequency: impl Into<Value<'a, f64>>, q: impl Into<Value<'a, f64>>) -> Self {
        Self::new(FilterType::Notch, input, frequency, q)
    }

    pub fn peak(input: impl Into<Value<'a, f64>>, frequency: impl Into<Value<'a, f64>>, q: impl Into<Value<'a, f64>>) -> Self {
        Self::new(FilterType::Peak, input, frequency, q)
    }

    pub fn all(input: impl Into<Value<'a, f64>>, frequency: impl Into<Value<'a, f64>>, q: impl Into<Value<'a, f64>>) -> Self {
        Self::new(FilterType::All, input, frequency, q)
    }
}

impl<'a> ValueNode for TrapezoidSVF<'a> {
    type T = f64;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], samples: usize) {
        let mut input: Vec<f64> = (0..samples).map(|_| Self::T::default()).collect();
        self.input.fill_buffer(env, &mut input, samples);
        let parameters = self.parameters(env, samples);

        for i in 0..samples {
            let (k, a1, a2, a3) = parameters[i];
            let v0 = input[i];

            let v3 = v0 - self.ic2eq;
            let v1 = a1*self.ic1eq + a2*v3;
            let v2 = self.ic2eq + a2*self.ic1eq + a3*v3;
            self.ic1eq = 2.0*v1 - self.ic1eq;
            self.ic2eq = 2.0*v2 - self.ic2eq;

            buffer[i] = match self.filter_type {
                FilterType::Low => v2,
                FilterType::Band => v1,
                FilterType::High => v0 - k*v1 - v2,
                FilterType::Notch => v0 - k*v1,
                FilterType::Peak => 2.0*v2 - v0 + k*v1,
                FilterType::All => v0 - 2.0*k*v1,
            };
        }
    }
}

pub struct Comb<D> where D: ValueNode {
    input: D,
    buffer: VecDeque<D::T>
}

impl<T: Default, D: ValueNode<T=T>> Comb<D> {
    pub fn new(input: D, delay: f64) -> Self {
        let len = delay * 44100.0;
        Self {
            input,
            buffer: (0..len as usize).map(|_| T::default()).collect(),
        }
    }
}

impl<T: Add<Output = T> + Copy + Default, D: ValueNode<T=T>> ValueNode for Comb<D> {
    type T = D::T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [T], samples: usize) {
        let mut input: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.input.fill_buffer(env, &mut input, samples);

        for (i, v0) in input.iter().enumerate() {
            self.buffer.push_back(*v0);
            buffer[i] = *v0 + self.buffer.pop_front().unwrap();
        }
    }
}
