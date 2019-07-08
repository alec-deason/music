#![feature(duration_float)]

use std::time::Duration;
use rand::Rng;

use music::{
    Env,
    value::{Value, ValueNode},
    oscillator::*,
    envelope::Linear,
    filter::*,
    effect::*,
    sequence::*,
};
use byteorder::{WriteBytesExt, LittleEndian};
use std::io::{self};

fn bass(duration: Duration, frequency: f64) -> Value<f64> {
    let freq: Value<f64> = frequency.into();
    let osc1: Value<f64> = WaveTableSynth::saw(freq).to_value();
    let freq: Value<f64> = (frequency / 0.9931).into();
    let osc2 = WaveTableSynth::saw(freq).to_value();
    let freq: Value<f64> = (frequency / 2.0).into();
    let osc3 = WaveTableSynth::sin(freq).to_value();
    let ffreq: Value<f64> = 1800.0.into();
    let ffreq_env = Linear::new(0.01, 1.0, 0.01, duration.as_secs_f64(), 0.1).to_value();
    let env = Linear::new(0.01, 1.0, 0.01, 1.0, 0.07).to_value();
    let fq: Value<f64> = 0.5.into();
    Value(Box::new(RLPF::low_pass(osc1+osc2+osc3, ffreq*ffreq_env + 80.0.into(), fq))) * env
}

fn main() {
    let notes: Vec<(Duration, f64)> = (0..16).map(|_| (Duration::from_millis(rand::thread_rng().gen_range(200, 1000)), rand::thread_rng().gen_range(120.0, 420.0))).collect();
    //let notes = vec![(Duration::new(1, 0), 440.0); 10];
    let mut sig = SimpleSequence::new(Box::new(bass), &notes, 3).to_value();
    //let mut sig:Value<f64> = Sine::new(440.0.into()).to_value();
    //let mut sig:Value<f64> = WaveTableSynth::saw(440.0.into()).to_value();

    //sig = Reverb::new(sig.to_value(), 0.5, 0.1, 4000.0, 3.8).to_value();
    let env = Env::new(44100);
    let len = 10;
    let mut buffer = vec![0.0; env.sample_rate as usize*len];
    sig.fill_buffer(&env, &mut buffer, 0, env.sample_rate as usize*len);
    let amp = 0.25;
    for sample in buffer.iter() {
        io::stdout().write_f32::<LittleEndian>(*sample as f32 * amp).unwrap();
        io::stdout().write_f32::<LittleEndian>(*sample as f32 * amp).unwrap();
    }
}
