use std::f64::consts::PI;

use lazy_static::lazy_static;

use super::{
    value::{ValueNode, Value},
    Env,
};

lazy_static! {
    static ref SINE: Vec<f64> = {
        (0..2048).map(|i| (i as f64 * ((PI*2.0)/2048.0)).sin()).collect()
    };
    static ref SQUARE: Vec<f64> = {
        (0..2048).map(|i| if i as f64 * ((PI*2.0)/2048.0).sin() > 0.0 { 1.0 } else { -1.0 }).collect()
    };
    static ref SAW: Vec<f64> = {
        let mut table = Vec::with_capacity(2048);
        let mut phase = 0.0;
        for _ in 0..2048 {
            table.push(1.0 - (1.0 / PI * phase));

            phase = phase + ((2.0 * PI * 1.0) / 2048.0);

            if phase > (2.0 * PI) {
                phase = phase - (2.0 * PI)
            }
        }
        table
    };
}

pub struct WaveTableSynth {
    frequency: Value<'static, f64>,
    table: Vec<f64>,
    position: f64,
}

impl WaveTableSynth {
    pub fn sin(frequency: Value<'static, f64>) -> Self {
        WaveTableSynth {
            frequency,
            table: SINE.to_vec(),
            position: 0.0,
        }
    }

    pub fn square(frequency: Value<'static, f64>) -> Self {
        WaveTableSynth {
            frequency,
            table: SQUARE.to_vec(),
            position: 0.0,
        }
    }

    pub fn saw(frequency: Value<'static, f64>) -> Self {
        WaveTableSynth {
            frequency,
            table: SAW.to_vec(),
            position: 0.0,
        }
    }

    pub fn to_value(self) -> Value<'static, f64> {
        Value(Box::new(self))
    }
}

impl ValueNode<f64> for WaveTableSynth {
    fn next(&mut self, env: &Env) -> f64 {
        let v = self.table[self.position as usize];
        let len = self.table.len() as f64;
        self.position += (len / env.sample_rate as f64) * self.frequency.next(env);
        while self.position >= len {
            self.position -= len;
        }
        v
    }
}
