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

pub struct IteratorSequence<'a, T> {
    instrument: Box<Fn(Note) -> Value<'a, T> + 'a>,
    duration: Option<Box<dyn Iterator<Item = Duration> + 'a>>,
    amplitude: Option<Box<dyn Iterator<Item = f64> + 'a>>,
    frequency: Option<Box<dyn Iterator<Item = f64> + 'a>>,
}

struct RunningIteratorSequence<'a, T> {
    instrument: Box<Fn(Note) -> Value<'a, T> + 'a>,
    duration: Box<dyn Iterator<Item = Duration> + 'a>,
    amplitude: Box<dyn Iterator<Item = f64> + 'a>,
    frequency: Box<dyn Iterator<Item = f64> + 'a>,

    current_notes: VecDeque<Option<Value<'a, T>>>,
    trigger: Duration,
}

impl<'a, T> IteratorSequence<'a, T> {
    pub fn new<F: Fn(Note) -> Value<'a, T> + 'a>(instrument: F) -> Self {
        Self {
            instrument: Box::new(instrument),
            duration: None,
            amplitude: None,
            frequency: None,
        }
    }

    pub fn duration<I: IntoIterator<Item = Duration> + 'a>(mut self, duration: I) -> Self {
        self.duration = Some(Box::new(duration.into_iter()));
        self
    }

    pub fn amplitude<I: IntoIterator<Item = f64> + 'a>(mut self, amplitude: I) -> Self {
        self.amplitude = Some(Box::new(amplitude.into_iter()));
        self
    }

    pub fn frequency<I: IntoIterator<Item = f64> + 'a>(mut self, frequency: I) -> Self {
        self.frequency = Some(Box::new(frequency.into_iter()));
        self
    }
}

impl<'a, T: From<f64> + Add<Output = T> + 'a> From<IteratorSequence<'a, T>> for Value<'a, T> {
    fn from(iterator: IteratorSequence<'a, T>) -> Value<'a, T> {
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

impl<'a, T: From<f64> + Add<Output=T>> ValueNode for RunningIteratorSequence<'a, T> {
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


pub struct FancySequence<'a, S, T> {
    state: S,
    generator: Box<Fn(&mut S) -> Option<(Duration, Value<'a, T>)> + 'a>,

    current_notes: VecDeque<Option<Value<'a, T>>>,
    trigger: Duration,
}

impl<'a, S, T> FancySequence<'a, S, T> {
    pub fn new<F: Fn(&mut S) -> Option<(Duration, Value<'a, T>)> + 'a>(initial_state: S, generator: F) -> Self {
        Self {
            state: initial_state,
            generator: Box::new(generator),

            current_notes: (0..5).map(|_| None).collect(),
            trigger: Duration::new(0, 0),
        }
    }


}
pub fn sequence_from_iterator<'a, T, I: IntoIterator<Item = (Duration, Value<'a, T>)> + 'a>(iter: I) -> FancySequence<'a, Box<Iterator<Item = (Duration, Value<'a, T>)> + 'a>, T> {
    let iterator = Box::new(iter.into_iter());
    FancySequence::new(iterator, |iterator| iterator.next())
}

impl<'a, S, T: From<f64> + Add<Output=T>> ValueNode for FancySequence<'a, S, T> {
    type T = T;
    fn next(&mut self, env: &Env) -> Self::T {
        if env.time > self.trigger {
            let note = (self.generator)(&mut self.state);
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
