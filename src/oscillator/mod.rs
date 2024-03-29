pub mod sampler;
pub mod string;

use rand::Rng;
use std::f64::consts::PI;

use lazy_static::lazy_static;

use super::{
    value::{Value, ValueNode},
    Env,
};

lazy_static! {
    static ref SINE: Vec<(f64, Vec<f64>)> = {
        let len = 512;
        vec![(
            2200.0,
            (0..len)
                .map(|i| (i as f64 * ((PI * 2.0) / len as f64)).sin())
                .collect(),
        )]
    };
    static ref SQUARE: Vec<(f64, Vec<f64>)> = {
        let len = 512;
        vec![(
            22000.0,
            (0..len)
                .map(|i| {
                    if i as f64 * ((PI * 2.0) / len as f64).sin() > 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                })
                .collect(),
        )]
    };
    static ref SAW_BL: Vec<(f64, Vec<f64>)> = {
        let mut tables = vec![];
        let len = 512;
        let f = 1.0;
        let mut max_f = f * 15.0;
        while max_f <= 22050.0 {
            let mut table = vec![0.0; len];
            let mut partial = f;
            let mut pi = 1;
            while partial < max_f {
                let a = 1.0 / pi as f64;
                for i in 0..len {
                    table[i] += (((2.0 * PI) / len as f64) * partial * i as f64).sin() * a * 0.5;
                }
                partial += f;
                pi += 1;
            }
            let tf = (44100.0 * 44100.0) / (2.0 * max_f * len as f64);
            tables.push((tf, table.iter().rev().cloned().collect()));
            max_f *= 2.0;
        }
        tables.sort_by_key(|t| (t.0 * 1000.0) as u32);
        tables
    };
    static ref SAW: Vec<(f64, Vec<f64>)> = {
        let len = 2048;
        let mut table = Vec::with_capacity(len);
        for i in 0..len {
            let v = -1.0 + 2.0 * (i as f64 / len as f64);
            eprintln!("{}", v);
            table.push(v);
        }
        vec![(22000.0, table)]
    };
}

pub struct WaveTableSynth<'a, T> {
    frequency: Value<'a, T>,
    tables: Vec<(f64, Vec<f64>)>,
    position: f64,
}

impl<'a, T> WaveTableSynth<'a, T> {
    pub fn sin(frequency: impl Into<Value<'a, T>>) -> Self {
        WaveTableSynth {
            frequency: frequency.into(),
            tables: SINE.to_vec(),
            position: 0.0,
        }
    }

    pub fn square(frequency: impl Into<Value<'a, T>>) -> Self {
        WaveTableSynth {
            frequency: frequency.into(),
            tables: SQUARE.to_vec(),
            position: 0.0,
        }
    }

    pub fn saw(frequency: impl Into<Value<'a, T>>) -> Self {
        WaveTableSynth {
            frequency: frequency.into(),
            tables: SAW_BL.to_vec(),
            position: 0.0,
        }
    }
}

impl<'a, T: Default + Clone + Into<f64> + From<f64>> ValueNode for WaveTableSynth<'a, T> {
    type T = T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [T], samples: usize) {
        let mut frequency: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.frequency.fill_buffer(env, &mut frequency, samples);

        for i in 0..samples {
            let mut table = &self.tables[0].1;
            let freq: f64 = frequency[i].clone().into();
            for (cap, t) in &self.tables {
                table = t;
                if cap >= &freq {
                    break;
                }
            }

            let v = table[self.position as usize];
            let len = table.len() as f64;
            self.position += (len / env.sample_rate as f64) * freq;
            while self.position >= len {
                self.position -= len;
            }
            buffer[i] = v.into();
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Impulses {
    freq: f64,
}

impl Impulses {
    pub fn new(freq: f64) -> Self {
        Self { freq }
    }
}
impl ValueNode for Impulses {
    type T = f64;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], samples: usize) {
        for i in 0..samples {
            buffer[i] = if rand::thread_rng().gen::<f64>() < self.freq / env.sample_rate as f64 {
                1.0
            } else {
                0.0
            };
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WhiteNoise;

impl ValueNode for WhiteNoise {
    type T = f64;
    fn fill_buffer(&mut self, _env: &Env, buffer: &mut [Self::T], samples: usize) {
        for i in 0..samples {
            buffer[i] = rand::thread_rng().gen_range(-1.0, 1.0).into();
        }
    }
}

pub struct BrownianNoise<'a, T> {
    current: f64,
    wiggle: Value<'a, T>,
    freq: Value<'a, T>,
}

impl<'a, T> BrownianNoise<'a, T> {
    pub fn new(freq: impl Into<Value<'a, T>>, wiggle: impl Into<Value<'a, T>>) -> Self {
        Self {
            current: 0.0,
            wiggle: wiggle.into(),
            freq: freq.into(),
        }
    }
}

impl<'a, T: Default + Clone + From<f64> + Into<f64>> ValueNode for BrownianNoise<'a, T> {
    type T = T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], samples: usize) {
        let mut freq: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.freq.fill_buffer(env, &mut freq, samples);
        let mut wiggle: Vec<T> = (0..samples).map(|_| Self::T::default()).collect();
        self.wiggle.fill_buffer(env, &mut wiggle, samples);

        for i in 0..samples {
            let wiggle: f64 = wiggle[i].clone().into();
            let freq: f64 = freq[i].clone().into();
            if rand::thread_rng().gen::<f64>() < freq / env.sample_rate as f64 {
                let wiggle = wiggle.max(0.00001);
                let step: f64 = rand::thread_rng().gen_range(-wiggle, wiggle).into();
                self.current = (self.current + step).min(1.0).max(-1.0);
            }
            buffer[i] = self.current.into();
        }
    }
}
