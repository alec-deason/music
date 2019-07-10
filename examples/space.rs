#![feature(duration_float)]

use std::iter;
use std::time::Duration;
use rand::Rng;
use rand::seq::SliceRandom;

use music::{
    Env,
    value::{Value},
    oscillator::*,
    envelope::ADSR,
    filter::*,
    effect::*,
    sequence::*,
    note::*,
};
use byteorder::{WriteBytesExt, LittleEndian};
use std::io::{self};
use lazy_static::lazy_static;

lazy_static! {
    static ref SCALE: Vec<f64> = {
        let mut scale: Vec<u32> = MAJOR.iter().fold(vec![69], |mut acc, x| {acc.push(acc.last().unwrap()+x); acc});
        scale.extend(scale.clone().iter().map(|x| x - 12));
        scale.extend(scale.clone().iter().map(|x| x + 12));
        scale.iter().cloned().map(|x| x as f64).collect()
    };
}

fn bloops(f_mul: f64) -> Value<f64> {
    let sig: Value<f64> = IteratorSequence::new(move |note| {
            let fenv: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(0.25).duration(0.1).release(0.06).curve(0.01).into();
            let env: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(1.0).duration(note.duration.as_secs_f64() - 0.31).release(0.3).curve(0.5).into();
            let sig: Value<f64> = WaveTableSynth::sin(note.frequency * fenv * 4.0 * f_mul).into();
            sig * env * note.amplitude
        }).frequency(iter::repeat_with(|| {
            (*SCALE.choose(&mut rand::thread_rng()).unwrap() as f64).frequency_from_midi()
        })).duration(iter::repeat_with(|| Duration::from_millis(rand::thread_rng().gen_range(610, 800)))).into();
    sig * 0.2
}

fn swish() -> Value<f64> {
    let sig: Value<f64> = IteratorSequence::new(|note| {
            let attack = note.duration.as_secs_f64() / 2.0;
            let release = attack;
            let renv: Value<f64> = ADSR::new().attack(attack).release(0.0).duration(0.0).release(release).into();
            let sig: Value<f64> = BrownianNoise::new(renv).into();
            sig * note.amplitude 
        }).duration(iter::repeat_with(|| Duration::from_millis(rand::thread_rng().gen_range(3000, 15000)))).into();
    sig * 0.01
}

fn main() {
    let mut sig = bloops(1.0);
    sig = sig + bloops(0.5);
    sig = sig + bloops(0.25);
    sig = sig + swish();
    sig = Reverb::new(sig, 0.8, 0.1, 2000.0, 4.8).into();

    let env = Env::new(44100);
    let len = 30;
    let mut buffer = vec![0.0; env.sample_rate as usize*len];
    sig.fill_buffer(&env, &mut buffer, 0, env.sample_rate as usize*len);
    let amp = 0.25;
    for sample in buffer.iter() {
        io::stdout().write_f32::<LittleEndian>(*sample as f32 * amp).unwrap();
        io::stdout().write_f32::<LittleEndian>(*sample as f32 * amp).unwrap();
    }

}
