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

    let freq: Value<f64> = sequence_from_iterator(
        notes.windows(2)
        .map(move |window| {
            let num_beats = window[1].0;
            let tone = window[1].1.frequency_from_midi() as f64;
            let prev_tone = window[0].1.frequency_from_midi() as f64;
            let tone_d = tone - prev_tone;

            let dur = Duration::from_millis((num_beats * beat) as u64);
            let slide = 0.07;

            let slide: Value<f64> = ADSR::new().attack(slide).decay(0.0).duration(dur.as_secs_f64()).release(0.0).into();

            let env: Value<f64> = ADSR::new().attack(0.0).decay(0.0).duration(dur.as_secs_f64()).release(0.0).into();

            let freq: Value<f64> = prev_tone.into();
            let freq = freq + slide * tone_d;
            (dur, (env * freq).into())
        })).into();
    let mut sig: Value<f64> = DrivenString::new(freq).into();

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
