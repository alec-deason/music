use crate::{
    value::{ValueNode, Value},
    Env,
};

pub struct Linear {
    attack: f64,
    sustain_level: f64,
    decay: f64,
    duration: f64,
    release: f64,

    active: bool,
    clock: f64,
}

impl Linear {
    pub fn new(attack: f64, sustain_level: f64, decay: f64, duration: f64, release: f64) -> Self {
        Linear {
            attack,
            sustain_level,
            decay,
            duration,
            release,

            active: true,
            clock: 0.0,
        }
    }
}


impl ValueNode<f64> for Linear {
    fn next(&mut self, env: &Env) -> f64 {
        if self.active {
            let v = if self.clock < self.attack {
                let d = self.attack - self.clock;
                1.0 - d / self.attack
            } else if self.clock < self.attack+self.decay {
                let d = (self.attack+self.decay) - self.clock;
                1.0 - (d / self.decay) * (1.0 - self.sustain_level)
            } else if self.clock < self.attack+self.decay+self.duration {
                self.sustain_level
            } else {
                let d = (self.attack+self.decay+self.duration+self.release) - self.clock;
                (d / self.release) * self.sustain_level
            };
            self.clock += 1.0 / env.sample_rate as f64;
            if self.clock > self.attack + self.decay + self.duration + self.release {
                self.active = false;
            }
            v
        } else {
            0.0
        }
    }

    fn to_value(self) -> Value<f64> {
        Value(Box::new(self))
    }
}
