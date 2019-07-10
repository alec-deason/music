use std::iter;
use std::ops::Add;
use std::time::Duration;
use std::collections::VecDeque;
use crate::{
    value::{ValueNode, Value},
    Env,
};

pub struct Note {
    pub duration: Duration,
    pub amplitude: f64,
    pub frequency: f64,
}

pub struct IteratorSequence<T> {
    instrument: Box<Fn(Note) -> Value<T>>,
    duration: Option<Box<dyn Iterator<Item = Duration>>>,
    amplitude: Option<Box<dyn Iterator<Item = f64>>>,
    frequency: Option<Box<dyn Iterator<Item = f64>>>,
}

struct RunningIteratorSequence<T> {
    instrument: Box<Fn(Note) -> Value<T>>,
    duration: Box<dyn Iterator<Item = Duration>>,
    amplitude: Box<dyn Iterator<Item = f64>>,
    frequency: Box<dyn Iterator<Item = f64>>,

    current_notes: VecDeque<Option<Value<T>>>,
    trigger: Duration,
}

impl<T> IteratorSequence<T> {
    pub fn new<F: Fn(Note) -> Value<T> + 'static>(instrument: F) -> Self {
        Self {
            instrument: Box::new(instrument),
            duration: None,
            amplitude: None,
            frequency: None,
        }
    }

    pub fn duration<I: Iterator<Item = Duration> + 'static>(mut self, duration: I) -> Self {
        self.duration = Some(Box::new(duration));
        self
    }

    pub fn amplitude<I: Iterator<Item = f64> + 'static>(mut self, amplitude: I) -> Self {
        self.amplitude = Some(Box::new(amplitude));
        self
    }

    pub fn frequency<I: Iterator<Item = f64> + 'static>(mut self, frequency: I) -> Self {
        self.frequency = Some(Box::new(frequency));
        self
    }
}

impl<T: From<f64> + Add<Output = T> + 'static> From<IteratorSequence<T>> for Value<T> {
    fn from(iterator: IteratorSequence<T>) -> Value<T> {
        let running = RunningIteratorSequence {
            instrument: iterator.instrument,
            duration: iterator.duration.unwrap_or_else(|| Box::new(iter::repeat(Duration::new(1, 0)))),
            amplitude: iterator.amplitude.unwrap_or_else(|| Box::new(iter::repeat(1.0))),
            frequency: iterator.frequency.unwrap_or_else(|| Box::new(iter::repeat(440.0))),

            current_notes: (0..3).map(|_| None).collect(),
            trigger: Duration::new(0, 0),
        };
        running.into()
    }
}

impl<T: From<f64> + Add<Output=T>> ValueNode for RunningIteratorSequence<T> {
    type T = T;
    fn next(&mut self, env: &Env) -> Self::T {
        if env.time > self.trigger {
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

pub struct SimpleSequence<T> {
    iter: Box<dyn Iterator<Item = (Duration, Value<T>)>>,

    current_notes: VecDeque<Option<Value<T>>>,
    trigger: Duration,
}

impl<T> SimpleSequence<T> {
    pub fn new<I: IntoIterator<Item = (Duration, Value<T>)> + 'static>(iter: I) -> Self {
        Self {
            iter: Box::new(iter.into_iter()),

            current_notes: (0..3).map(|_| None).collect(),
            trigger: Duration::new(0, 0),
        }
    }
}

impl<T: From<f64> + Add<Output=T>> ValueNode for SimpleSequence<T> {
    type T = T;
    fn next(&mut self, env: &Env) -> Self::T {
        if env.time > self.trigger {
            let note = self.iter.next();
            if note.is_some() {
                let (duration, note) = note.unwrap();
                self.trigger = env.time + duration;
                self.current_notes[0].replace(note);
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
