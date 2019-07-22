use crate::{
    note::Scale,
};

pub enum Message {
    Key(Scale),
    Note(u32),
    Chord(Vec<u32>),
}
pub struct Annotation<Message> {
    start: f64,
    duration: f64,
    message: Message,
}

pub type Voice = Vec<(f64, Vec<(u32, f64)>)>;

pub struct Composition<Message, Voice> {
    pub ideas: Vec<(f64, Vec<Option<i32>>)>,
    annotations: Vec<Annotation<Message>>,
    voices: Vec<Voice>,
}

impl<Message, Voice> Composition<Message, Voice> {
    pub fn new() -> Self {
        Self {
            ideas: vec![],
            annotations: vec![],
            voices: vec![],
        }
    }

    pub fn voice(&self, idx: usize) -> Option<&Voice> {
        self.voices.get(idx)
    }
    pub fn voice_mut(&mut self, idx: usize) -> Option<&mut Voice> {
        self.voices.get_mut(idx)
    }
    pub fn add_voice(&mut self, voice: Voice) -> usize {
        self.voices.push(voice);
        self.voices.len() - 1
    }


    pub fn add_annotation(&mut self, start: f64, duration: f64, message: Message) {
        self.annotations.push(Annotation {
            start,
            duration,
            message,
        });
    }
    pub fn annotations(&self, start: f64, duration: f64) -> Vec<&Message> {
        let mut messages = vec![];
        for annotation in &self.annotations {
            if (annotation.start <= start+duration && annotation.start >= start) ||
               (annotation.start + annotation.duration <= start+duration && annotation.start+annotation.duration >= start) ||
               (start <= annotation.start+annotation.duration && start >= annotation.start) ||
               (start + duration <= annotation.start+annotation.duration && start+duration >= annotation.start) {
                messages.push(&annotation.message);
            }
        }
        messages
    }
}
