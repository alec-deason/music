use std::ops::Add;
use std::time::Duration;
use std::collections::VecDeque;
use crate::{
    value::{ValueNode, Value},
    Env,
};

pub struct SimpleSequence<T> {
    notes: Vec<(Duration, f64)>,
    instrument: Box<Fn(Duration, f64) -> Value<T>>,

    current_notes: VecDeque<Option<Value<T>>>,
    trigger: Duration,
}

impl<T> SimpleSequence<T> {
    pub fn new(instrument: Box<Fn(Duration, f64) -> Value<T>>, notes: &[(Duration, f64)], voices: usize) -> Self {
        SimpleSequence {
            notes: notes.iter().rev().cloned().collect(),
            instrument: instrument,

            current_notes: (0..voices).map(|_| None).collect(),
            trigger: Duration::new(0, 0),
        }
    }
}

impl<T> ValueNode<T> for SimpleSequence<T> where T: From<f64> + Add<Output=T> + 'static {
    fn next(&mut self, env: &Env) -> T {
        if (env.time > self.trigger) & (!self.notes.is_empty()) {
            let (duration, frequency) = self.notes.pop().unwrap();
            self.trigger = env.time + duration;
            self.current_notes[0].replace((self.instrument)(duration, frequency));
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

    fn to_value(self) -> Value<T> {
        Value(Box::new(self))
    }
}