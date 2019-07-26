use crate::{
    value::{Value, ValueNode},
    Env,
};

pub struct ADSR {
    attack: Option<f64>,
    sustain_level: Option<f64>,
    decay: Option<f64>,
    duration: Option<f64>,
    release: Option<f64>,
    curve: Option<f64>,
}

impl ADSR {
    pub fn new() -> Self {
        Self {
            attack: None,
            sustain_level: None,
            decay: None,
            duration: None,
            release: None,
            curve: None,
        }
    }

    pub fn attack(mut self, attack: f64) -> Self {
        self.attack = Some(attack);
        self
    }

    pub fn decay(mut self, decay: f64) -> Self {
        self.decay = Some(decay);
        self
    }

    pub fn sustain(mut self, sustain: f64) -> Self {
        self.sustain_level = Some(sustain);
        self
    }

    pub fn release(mut self, release: f64) -> Self {
        self.release = Some(release);
        self
    }

    pub fn duration(mut self, duration: f64) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn curve(mut self, curve: f64) -> Self {
        self.curve = Some(curve);
        self
    }
}

impl<'a> From<ADSR> for Value<'a, f64> {
    fn from(adsr: ADSR) -> Value<'a, f64> {
        RunningADSR {
            attack: adsr.attack.unwrap_or(0.1),
            sustain_level: adsr.sustain_level.unwrap_or(1.0),
            decay: adsr.decay.unwrap_or(0.0),
            duration: adsr.duration.unwrap_or(1.0),
            release: adsr.release.unwrap_or(0.1),
            curve: adsr.curve.unwrap_or(1.0),

            active: true,
            clock: 0.0,
        }
        .into()
    }
}

struct RunningADSR {
    attack: f64,
    sustain_level: f64,
    decay: f64,
    duration: f64,
    release: f64,
    curve: f64,

    active: bool,
    clock: f64,
}

impl ValueNode for RunningADSR {
    type T = f64;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], samples: usize) {
        for i in 0..samples {
            buffer[i] = if self.active {
                let v = if self.clock < self.attack {
                    let d = self.attack - self.clock;
                    1.0 - (d / self.attack).powf(self.curve)
                } else if self.clock < self.attack + self.decay {
                    let d = (self.attack + self.decay) - self.clock;
                    1.0 - (d / self.decay).powf(self.clock) * (1.0 - self.sustain_level)
                } else if self.clock < self.attack + self.decay + self.duration {
                    self.sustain_level
                } else {
                    let d = (self.attack + self.decay + self.duration + self.release) - self.clock;
                    (d / self.release).powf(self.curve) * self.sustain_level
                };
                self.clock += 1.0 / env.sample_rate as f64;
                if self.clock > self.attack + self.decay + self.duration + self.release {
                    self.active = false;
                }
                v
            } else {
                0.0
            };
        }
    }
}
