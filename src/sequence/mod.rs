use std::iter;
use std::ops::Add;
use std::time::Duration;
use std::collections::VecDeque;
use crate::{
    value::{ValueNode, Value},
    Env,
};

pub struct SimpleSequence<T> {
    notes: Vec<(Duration, f64, f64)>,
    instrument: Box<Fn(Duration, f64, f64) -> Value<T>>,

    current_notes: VecDeque<Option<Value<T>>>,
    trigger: Duration,
}

impl<T> SimpleSequence<T> {
    pub fn new(instrument: Box<Fn(Duration, f64, f64) -> Value<T>>, notes: &[(Duration, f64, f64)], voices: usize) -> Self {
        SimpleSequence {
            notes: notes.iter().rev().cloned().collect(),
            instrument: instrument,

            current_notes: (0..voices).map(|_| None).collect(),
            trigger: Duration::new(0, 0),
        }
    }
}

impl<T: From<f64> + Add<Output=T>> ValueNode for SimpleSequence<T> {
    type T = T;
    fn next(&mut self, env: &Env) -> Self::T {
        if (env.time > self.trigger) & (!self.notes.is_empty()) {
            let (duration, frequency, amplitude) = self.notes.pop().unwrap();
            self.trigger = env.time + duration;
            self.current_notes[0].replace((self.instrument)(duration, frequency, amplitude));
            self.current_notes.rotate_left(1);
        }

        let mut out: T = 0.0.into();
        for note in &mut self.current_notes {
            if let Some(note) = note {
                out = out + note.next(env);
            }
        }
        out
    }
}

pub struct Note {
    pub duration: Duration,
    pub amplitude: f64,
    pub frequency: f64,
}

pub struct IteratorSequence<T> {
    pub instrument: Box<Fn(Note) -> Value<T>>,
    pub duration: Box<dyn Iterator<Item = Duration>>,
    pub amplitude: Box<dyn Iterator<Item = f64>>,
    pub frequency: Box<dyn Iterator<Item = f64>>,

    pub current_notes: VecDeque<Option<Value<T>>>,
    pub trigger: Duration,
}

impl Default for IteratorSequence<f64> {
    fn default() -> Self {
        IteratorSequence {
            instrument: Box::new(|_| 0.0.into()),
            duration: Box::new(iter::repeat(Duration::new(1, 0))),
            frequency: Box::new(iter::repeat(440.0)),
            amplitude: Box::new(iter::repeat(1.0)),

            current_notes: (0..3).map(|_| None).collect(),
            trigger: Duration::new(0, 0),
        }
    }
}

impl<T: From<f64> + Add<Output=T>> ValueNode for IteratorSequence<T> {
    type T = T;
    fn next(&mut self, env: &Env) -> Self::T {
        if (env.time > self.trigger) {
            let duration = self.duration.next();
            let frequency = self.frequency.next();
            let amplitude = self.amplitude.next();
            if duration.is_some() & frequency.is_some() & amplitude.is_some() {
                let duration = duration.unwrap();
                self.trigger = env.time + duration;
                let note = Note {
                    duration: duration,
                    amplitude: amplitude.unwrap(),
                    frequency: frequency.unwrap(),
                };
                self.current_notes[0].replace((self.instrument)(note));
                self.current_notes.rotate_left(1);
            }
        }

        let mut out: T = 0.0.into();
        for note in &mut self.current_notes {
            if let Some(note) = note {
                out = out + note.next(env);
            }
        }
        out
    }
}
