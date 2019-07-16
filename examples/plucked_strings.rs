#![feature(duration_float)]

use std::f64::consts::PI;
use std::rc::Rc;
use std::cell::RefCell;
use std::env;
use std::iter;
use std::time::Duration;   
use rand::Rng;
use rand::seq::SliceRandom;
       
use music::{
    Env,
    value::{Value, CacheValue},
    oscillator::*,
    oscillator::string::*,
    envelope::ADSR,
    filter::*,
    effect::*,
    sequence::*,
    note::*,  
};
use byteorder::{WriteBytesExt, LittleEndian};
use std::io::{self};    


fn main() {
    let bpm = 180.0;
    let beat = 60000.0/bpm;
    let target_len = env::args().into_iter().nth(1).unwrap_or("10".to_string()).parse::<usize>().unwrap();

    let scale = Scale::major(69);
    let notes: Vec<_> = (-12..-11).chain((-12..12).chain((-11..11).rev()).cycle()).map(|degree| (1.0, scale.pitch(degree), 1.0)).take(300).collect();

    let mut sig: Value<f64> = sequence_from_iterator(
        notes.into_iter()
        .map(move |(num_beats, tone, amp)| {
            let note = Note {
                        duration: Duration::from_millis((num_beats * beat) as u64),
                        frequency: (tone ).frequency_from_midi() as f64,
                        amplitude: amp,
                    };
            let mut pluck: Value<f64> = PluckedString::new(note.frequency).into();
            let env: Value<f64> = ADSR::new()
                .attack(0.02)
                .decay(0.02)
                .duration(note.duration.as_secs_f64())
                .release(0.06).into();
            pluck = pluck * env * note.amplitude;
        (Duration::from_millis((num_beats * beat) as u64), pluck)

        })).into();

    let mut env = Env::new(44100);
    let chunk_size = 2048;
    let total_samples = env.sample_rate as usize*target_len;
    for _ in 0..total_samples / chunk_size {
        let mut buffer_left = vec![0.0; chunk_size];
        sig.fill_buffer(&mut env, &mut buffer_left, 0, chunk_size);
        let amp = 0.25;
        for left in buffer_left {
            io::stdout().write_f32::<LittleEndian>(left as f32 * amp).unwrap();
            io::stdout().write_f32::<LittleEndian>(left as f32 * amp).unwrap();
        }
    }
}
