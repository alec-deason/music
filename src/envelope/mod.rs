use crate::{
    value::{ValueNode},
    Env,
};

pub struct ADSR {
    attack: f64,
    sustain_level: f64,
    decay: f64,
    duration: f64,
    release: f64,
    curve: f64,

    active: bool,
    clock: f64,
}

impl ADSR {
    pub fn new(attack: f64, decay: f64, sustain_level: f64, duration: f64, release: f64, curve: f64) -> Self {
        Self {
            attack,
            sustain_level,
            decay,
            duration,
            release,
            curve,

            active: true,
            clock: 0.0,
        }
    }
}


impl ValueNode for ADSR {
    type T = f64;
    fn next(&mut self, env: &Env) -> Self::T {
        if self.active {
            let v = if self.clock < self.attack {
                let d = self.attack - self.clock;
                1.0 - (d / self.attack).powf(self.curve)
            } else if self.clock < self.attack+self.decay {
                let d = (self.attack+self.decay) - self.clock;
                1.0 - (d / self.decay).powf(self.clock) * (1.0 - self.sustain_level)
            } else if self.clock < self.attack+self.decay+self.duration {
                self.sustain_level
            } else {
                let d = (self.attack+self.decay+self.duration+self.release) - self.clock;
                (d / self.release).powf(self.curve) * self.sustain_level
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
}
