use std::ops::{Add, Mul, Neg};
use std::collections::VecDeque;
use std::f64::consts::PI;

use crate::{
    value::{ValueNode, Value},
    Env,
};



pub struct RLPF {
    input: Value<f64>,
    cutoff: Value<f64>,
    q: Value<f64>,
    cached_cutoff: f64,
    cached_q: f64,

    a0: f64,
    b1: f64,
    b2: f64,

    y0: f64,
    y1: f64,
    y2: f64,
}

impl RLPF {
    pub fn low_pass(input: Value<f64>, cutoff: Value<f64>, q: Value<f64>) -> Self {
        RLPF {
            input,
            cutoff,
            q,
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

    fn parameters(&mut self, env: &Env) -> (f64, f64, f64) {
        let cutoff = self.cutoff.next(env);
        let q = self.q.next(env);

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
        (self.a0, self.b1, self.b2)
    }
}


impl ValueNode<f64> for RLPF {
    fn next(&mut self, env: &Env) -> f64 {
        let (a0, b1, b2) = self.parameters(env);
        let v0 = self.input.next(env);
        self.y0 = a0 * v0 + b1 * self.y1 + b2 * self.y2;
        let out = self.y0 + 2.0 * self.y1 + self.y2;
        self.y2 = self.y1;
        self.y1 = self.y0;
        out
    }

    fn to_value(self) -> Value<f64> {
        Value(Box::new(self))
    }
}

pub struct AllPass<T> {
    input: Value<T>,
    k: T,

    buff: VecDeque<T>,
}

impl<T> AllPass<T> where T: From<f64> {
    pub fn new(input: Value<T>, delay: f64, decay: f64) -> Self {
        let k = 0.001f64.powf(delay / decay.abs()) * decay.signum();
        AllPass {
            input,
            k: k.into(),

            buff: (0..(44100.0*delay) as usize).map(|_| 0.0.into()).collect(),
        }
    }
}

impl<T> ValueNode<T> for AllPass<T> where T: Add<Output = T> + Mul<Output = T> + Neg<Output = T> + From<f64> + Copy + 'static {
    fn next(&mut self, env: &Env) -> T {
        let x = self.input.next(env);
        let s_d = self.buff.pop_front().unwrap_or_else(|| 0.0.into());
        let s:T = x + self.k * s_d;
        let y:T = -self.k * self.buff[0] + s_d;
        self.buff.push_back(s);
        y
    }

    fn to_value(self) -> Value<T> {
        Value(Box::new(self))
    }
}

/*
pub struct BiQuad {
    input: Value<f64>,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,

    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiQuad {
    pub fn low_pass(input: Value<f64>, center_frequency: f64, q: f64) -> Self {
        let sample_rate = 44100.0;
        let omega = 2.0 * PI * center_frequency / sample_rate;
        let cs = omega.sin();
        let alpha = cs / (2.0 * q);
        let a0 = 1.0 + alpha;
        BiQuad {
            input,
            a1: (-2.0 * cs) / a0,
            a2: (1.0 - alpha) / a0,
            b0: ((1.0 + cs) / 2.0) / a0,
            b1: (-(1.0 + cs)) / a0,
            b2: ((1.0 + cs) / 2.0) / a0,

            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

impl ValueNode<f64> for BiQuad {
    fn next(&mut self, env: &Env) -> f64 {
        let x = self.input.next(env);
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

pub struct TrapezoidSVF {
    input: Value<f64>,
    a1: f64,
    a2: f64,
    a3: f64,

    ic1eq: f64,
    ic2eq: f64,
}

//From: http://www.cytomic.com/files/dsp/SvfLinearTrapOptimised2.pdf
impl TrapezoidSVF {
    pub fn low_pass(input: Value<f64>, cutoff: f64, q: f64) -> Self {
        let g = (PI * cutoff/44100.0).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        TrapezoidSVF {
            input,
            a1,
            a2,
            a3,

            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }
}

impl ValueNode<f64> for TrapezoidSVF {
    fn next(&mut self, env: &Env) -> f64 {
        let v0 = self.input.next(env);
        let v3 = v0 - self.ic2eq;
        let v1 = self.a1*self.ic1eq + self.a2*v3;
        let v2 = self.ic2eq + self.a2*self.ic1eq + self.a3*v3;
        self.ic1eq = 2.0*v1 - self.ic1eq;
        self.ic2eq = 2.0*v2 - self.ic2eq;

        v2
    }
}
*/
