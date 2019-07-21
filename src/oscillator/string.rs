use std::collections::VecDeque;
use rand::Rng;
use rand::seq::SliceRandom;

use crate::{
    value::{ValueNode, Value, CacheValue},
    oscillator::{BrownianNoise, WaveTableSynth,},
    filter::TrapezoidSVF,
    Env,
};

pub struct PluckedString {
    buffer: Vec<f64>,
    smoothing: f64,

    amp: f64,
    previous: f64,
    position: usize,
}

impl PluckedString {
    pub fn new(freq: f64, smoothing: f64) -> Self {
        let mut rng = rand::thread_rng();
        let buffer_length = 44100 / freq as usize;
        let amp = 1.0 + ((freq.min(1000.0) - 300.0).max(0.0) / 700.0) * 3.0;
        Self {
            buffer: (0..buffer_length).map(|_| *[1.0, -1.0].choose(&mut rng).unwrap()).collect(),
            smoothing,

            amp,
            previous: 0.0,
            position: 0,
        }
    }
}


impl ValueNode for PluckedString {
    type T = f64;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], offset: usize, samples: usize) {
        for i in 0..samples {
            let sample = self.smoothing * self.buffer[self.position] + (1.0 - self.smoothing) * self.previous;
            self.buffer[self.position] = sample;
            self.previous = sample;
            self.position = (self.position + 1) % self.buffer.len();
            buffer[i] = sample * self.amp;
        }
    }
}

pub struct DrivenString<'a> {
    output: Value<'a, f64>,
}

impl<'a> DrivenString<'a> {
    pub fn new(freq: Value<'a, f64>) -> Self {
        let vibrato: Value<f64> = WaveTableSynth::sin(8.0).into();
        let tremalo: Value<f64> = WaveTableSynth::sin(2.1).into();
        let mut bow: Value<f64> = Value::<f64>::from(WaveTableSynth::saw(freq + vibrato * 3.0)) * (1.0 - (tremalo + 1.0) / 20.0);
        bow = TrapezoidSVF::high(bow, 200.0, 1.0).into();
        bow = TrapezoidSVF::low_pass(bow, 2000.0, 5.0).into();
        let bow = CacheValue::new(bow);
        let f1: Value<f64> = TrapezoidSVF::band(bow.clone(), 400.0, 0.5).into();
        let f2: Value<f64> = TrapezoidSVF::band(bow.clone(), 700.0, 0.5).into();
        let f3: Value<f64> = TrapezoidSVF::band(bow, 4000.0, 0.4).into();
        let sig = (f1 + f2 + f3) / 3.0;
        Self {
            output: sig,
        }
    }
}


impl<'a> ValueNode for DrivenString<'a> {
    type T = f64;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], offset: usize, samples: usize) {
        let mut output: Vec<f64> = Vec::with_capacity(samples);
        self.output.fill_buffer(env, &mut output, 0, samples);
        buffer[offset..offset+samples].copy_from_slice(&output);
    }
}
