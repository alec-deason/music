use std::path::Path;
use std::fs;
use std::fs::File;

use minimp3::{Decoder, Frame, Error};
use regex::Regex;

use crate::{
    Env,
    value::{Value, ValueNode},
    note::Pitch,
};

pub struct SampleSet {
    samples: Vec<(f64, Vec<f64>)>,
}

impl SampleSet {
    pub fn from_directory(path: impl AsRef<Path>, pattern: &Regex) -> Self {
        let mut samples = vec![];
        for entry in fs::read_dir(path.as_ref()).unwrap() {
            let entry = entry.unwrap();
            if let Some(captures) = pattern.captures(entry.path().to_str().unwrap()) {
                let note = match captures.name("note").unwrap().as_str().to_uppercase().as_str() {
                    "C" => 0,
                    "CS" => 1,
                    "C#" => 1,
                    "D" => 2,
                    "DS" => 3,
                    "D#" => 3,
                    "E" => 4,
                    "F" => 5,
                    "FS" => 6,
                    "F#" => 6,
                    "G" => 7,
                    "GS" => 8,
                    "G#" => 8,
                    "A" => 9,
                    "AS" => 10,
                    "A#" => 10,
                    "B" => 11,
                    _ => panic!(),
                };
                let octave = captures.name("octave").unwrap().as_str().parse::<u32>().unwrap();
                let adjusted_note = note + (octave+1)*12;

                samples.push(((adjusted_note as f64).frequency_from_midi(), Self::samples_from_file(entry.path())));
            }
        }

        Self {
            samples,
        }
    }

    fn samples_from_file(path: impl AsRef<Path>) -> Vec<f64> {
        let mut decoder = Decoder::new(File::open(path).unwrap());
        let mut sample = vec![];
        loop {
            match decoder.next_frame() {
                Ok(Frame { data, sample_rate, channels, .. }) => {
                    if channels != 1 { panic!() }
                    if sample_rate != 44100 { panic!() }
                    sample.extend(data.iter().cloned().map(|s| s as f64 / 65536.0));
                },
                Err(Error::Eof) => break,
                Err(e) => panic!("{:?}", e),
            }
        }
        sample
    }

    pub fn from_file(path: impl AsRef<Path>, freq: f64) -> Self {
        Self {
            samples: vec![(freq, Self::samples_from_file(path))],
        }
    }

    pub fn play<'a>(&'a self, freq: f64) -> Option<Value<'a, f64>> {
        let mut idxs: Vec<_> = (0..self.samples.len()).collect();
        idxs.sort_by_key(|i| ((self.samples[*i].0 - freq).abs() * 10000.0) as i32);

        let (chosen_freq, samples) = &self.samples[idxs[0]];

        let rate = if self.samples.len() > 1 {
            freq / chosen_freq
        } else {
            1.0
        };


        Some(Sampler::new(samples, rate).into())
    }
}

struct Sampler<'a> {
    samples: &'a[f64],
    pos: f64,
    rate: f64,
}

impl<'a> Sampler<'a> {
    fn new(samples: &'a [f64], rate: f64) -> Self {
        Self {
            samples,
            pos: 0.0,
            rate,
        }
    }
}


impl<'a> ValueNode for Sampler<'a> {
    type T = f64;
    fn fill_buffer(&mut self, _env: &Env, buffer: &mut [Self::T], samples: usize) {
        for i in 0..samples {
            buffer[i] = if self.pos as usize >= self.samples.len() {
                0.0
            } else {
                let s = self.samples[self.pos as usize];
                self.pos += self.rate;
                s
            };
        }
    }
}
