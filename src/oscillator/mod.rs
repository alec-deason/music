use std::f64::consts::PI;

use lazy_static::lazy_static;

use super::{
    value::{ValueNode, Value},
    Env,
};

lazy_static! {
    static ref SINE: Vec<(f64, Vec<f64>)> = {
        let len = 44100;
        vec![(2200.0, 
        (0..len).map(|i| (i as f64 * ((PI*2.0)/len as f64)).sin()).collect()
        )]
    };
    static ref SQUARE: Vec<(f64, Vec<f64>)> = {
        let len = 2560;
        vec![(22000.0,
        (0..len).map(|i| if i as f64 * ((PI*2.0)/len as f64).sin() > 0.0 { 1.0 } else { -1.0 }).collect()
        )]
    };
    static ref SAW_BL: Vec<(f64, Vec<f64>)> = {
        let mut tables = vec![];
        let len = 256;
        let f = 1.0;
        let mut max_f = f*15.0;
        while max_f <= 22050.0 {
            let mut table = vec![0.0; len];
            let mut partial = f;
            let mut pi = 1;
            while partial < max_f {
                let a = 1.0 / pi as f64;
                for i in 0..len {
                    table[i] += (((2.0*PI) / len as f64) * partial * i as f64).sin() * a * 0.5;
                }
                partial += f;
                pi += 1;
            }
            let tf = (44100.0*44100.0)/(2.0*max_f*len as f64);
            eprintln!("{}", tf);
            tables.push((tf, table.iter().rev().cloned().collect()));
            max_f *= 2.0;
        }
        tables.sort_by_key(|t| (t.0*1000.0) as u32);
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

pub struct Sine {
    frequency: f64,
    clock: u32,
}
impl Sine {
    pub fn new(frequency: f64) -> Self {
        Sine {
            frequency,
            clock: 0,
        }
    }
}

impl ValueNode<f64> for Sine {
    fn next(&mut self, env: &Env) -> f64 {
        let v = (2.0*PI*self.clock as f64*(self.frequency/env.sample_rate as f64)).sin();
        self.clock += 1;
        v
    }

    fn to_value(self) -> Value<f64> {
        Value(Box::new(self))
    }
}

pub struct WaveTableSynth {
    frequency: Value<f64>,
    tables: Vec<(f64, Vec<f64>)>,
    position: f64,
}

impl WaveTableSynth {
    pub fn sin(frequency: Value<f64>) -> Self {
        WaveTableSynth {
            frequency,
            tables: SINE.to_vec(),
            position: 0.0,
        }
    }

    pub fn square(frequency: Value<f64>) -> Self {
        WaveTableSynth {
            frequency,
            tables: SQUARE.to_vec(),
            position: 0.0,
        }
    }

    pub fn saw(frequency: Value<f64>) -> Self {
        WaveTableSynth {
            frequency,
            tables: SAW_BL.to_vec(),
            position: 0.0,
        }
    }
}

impl ValueNode<f64> for WaveTableSynth {
    fn next(&mut self, env: &Env) -> f64 {
        let freq = self.frequency.next(env);
        let mut table = &self.tables[0].1;
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
        v
    }

    fn to_value(self) -> Value<f64> {
        Value(Box::new(self))
    }
}
