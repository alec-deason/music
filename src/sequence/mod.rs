use crate::{
    value::{Value, ValueNode},
    Env,
};
use std::collections::VecDeque;
use std::ops::Add;
use std::time::Duration;

pub struct Note {
    pub duration: Duration,
    pub amplitude: f64,
    pub frequency: f64,
}

pub struct FancySequence<'a, S, T> {
    state: S,
    generator: Box<dyn Fn(&mut S) -> Option<(Duration, Value<'a, T>)> + 'a>,

    current_notes: VecDeque<Option<Value<'a, T>>>,
    samples_remaining: usize,
}

impl<'a, S, T> FancySequence<'a, S, T> {
    pub fn new<F: Fn(&mut S) -> Option<(Duration, Value<'a, T>)> + 'a>(
        initial_state: S,
        generator: F,
    ) -> Self {
        Self {
            state: initial_state,
            generator: Box::new(generator),

            current_notes: (0..10).map(|_| None).collect(),
            samples_remaining: 0,
        }
    }
}
pub fn sequence_from_iterator<'a, T, I: IntoIterator<Item = (Duration, Value<'a, T>)> + 'a>(
    iter: I,
) -> FancySequence<'a, Box<dyn Iterator<Item = (Duration, Value<'a, T>)> + 'a>, T> {
    let iterator = Box::new(iter.into_iter());
    FancySequence::new(iterator, |iterator| iterator.next())
}

impl<'a, S, T: Default + Add<Output = T> + Clone> ValueNode for FancySequence<'a, S, T> {
    type T = T;
    fn fill_buffer(&mut self, env: &Env, buffer: &mut [Self::T], samples: usize) {
        if self.samples_remaining == 0 {
            let note = (self.generator)(&mut self.state);
            if note.is_some() {
                let (duration, note) = note.unwrap();
                self.current_notes[0].replace(note);
                self.current_notes.rotate_left(1);
                self.samples_remaining = (duration.as_secs_f64() * env.sample_rate as f64) as usize;
            } else {
                self.samples_remaining = std::usize::MAX;
            }
        }
        let remaining = self.samples_remaining.min(samples);
        let mut result: Vec<Self::T> = (0..remaining).map(|_| Self::T::default()).collect();
        let mut is_first = true;
        for note in &mut self.current_notes {
            if let Some(note) = note {
                note.fill_buffer(env, &mut result, remaining);
                if is_first {
                    buffer[0..remaining].clone_from_slice(&result);
                    is_first = false;
                } else {
                    for i in 0..remaining {
                        let cur = buffer[i].clone();
                        buffer[i] = cur + result[i].clone();
                    }
                }
            }
        }
        self.samples_remaining -= remaining;
        if remaining < samples {
            self.fill_buffer(env, &mut buffer[remaining..], samples - remaining);
        }
    }
}
