use crate::{
    note::Scale,
};

#[derive(Copy, Clone, Debug)]
pub enum SectionType {
    Intro,
    Outro,
    Normal,
}

#[derive(Clone)]
pub enum Message {
    Key(Scale),
    Note(u32),
    Chord(Vec<u32>),
}
#[derive(Clone)]
pub struct Annotation<Message> where Message: Clone {
    pub start: f64,
    pub duration: f64,
    pub message: Message,
}

pub type Voice = Vec<(f64, Vec<(u32, f64)>)>;

#[derive(Clone)]
pub struct Composition<Message> where Message: Clone {
    pub ideas: Vec<(f64, Vec<Option<i32>>)>,
    annotations: Vec<Annotation<Message>>,
    voices: Vec<Voice>,
}

impl<Message: Clone> Composition<Message> {
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
    pub fn annotations(&self, start: f64, duration: f64) -> Vec<&Annotation<Message>> {
        let mut messages = vec![];
        for annotation in &self.annotations {
            if (annotation.start <= start+duration && annotation.start >= start) ||
               (annotation.start + annotation.duration <= start+duration && annotation.start+annotation.duration >= start) ||
               (start <= annotation.start+annotation.duration && start >= annotation.start) ||
               (start + duration <= annotation.start+annotation.duration && start+duration >= annotation.start) {
                messages.push(annotation);
            }
        }
        messages
    }

    pub fn total_beats(&self) -> f64 {
        let mul = 1000.0;
        self.voices.iter().map(|voice| voice.iter().map(|(dur, _)| (*dur * mul) as i32).sum::<i32>()).max().unwrap_or(0) as f64 / mul
    }

    pub fn extend(&mut self, other: &Composition<Message>) {
        for i in &other.ideas {
            self.ideas.push(i.clone());
        }
        let dur = self.total_beats();
        self.annotations.extend(other.annotations.iter().map(|a| Annotation {
            start: a.start + dur,
            duration: a.duration,
            message: a.message.clone(),
        }));
        for (i, v) in self.voices.iter_mut().enumerate() {
            v.extend(other.voices[i].clone());
        }
    }
}
