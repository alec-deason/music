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

fn bass(duration: Duration, frequency: f64, amp: f64) -> Value<f64> {
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
    Value(Box::new(RLPF::low_pass(osc1+osc2+osc3, ffreq*ffreq_env + 80.0.into(), fq))) * env * amp.into()
}

fn vangelis(duration: Duration, frequency: f64, amp: f64) -> Value<f64> {
    let osc1: Value<f64> = WaveTableSynth::saw(frequency.into()).to_value();
    let lfo: Value<f64> = WaveTableSynth::sin(0.12.into()).to_value();
    let osc2: Value<f64> = WaveTableSynth::saw(Value::<f64>::from(frequency) / (Value::<f64>::from(1.0) -lfo * 0.1.into())).to_value();
    let mut sig = osc1 + osc2;
    let env = Linear::new(0.41, 1.0, 0.0, duration.as_secs_f64(), 0.41).to_value();
    let fenv = Linear::new(1.6, 1.0, 0.0, duration.as_secs_f64(), 4.2).to_value();
    sig = RLPF::low_pass(sig, Value::<f64>::from(7000.0)*fenv+100.0.into(), 1.0.into()).to_value();
    //sig = Reverb::new(sig, 0.5, 0.1, 4000.0, 4.0).to_value();
    sig * env * amp.into()
}

fn kick_drum(_duration: Duration, _frequency: f64, amp: f64) -> Value<f64> {
    let click: Value<f64> = WhiteNoise.to_value() * 0.025.into();
    let cenv = Linear::new(0.001, 1.0, 0.01, 0.0, 0.0).to_value();
    let oenv = Linear::new(0.001, 1.0, 0.09, 0.0, 0.06).to_value();
    let osc = WaveTableSynth::sin(Value::<f64>::from(58.0) * oenv).to_value();
    let env = Linear::new(0.001, 1.0, 0.0, 0.02, 0.005).to_value();
    ((click * cenv) + osc) * env * amp.into()
}

fn snare_drum(_duration: Duration, _frequency: f64, amp: f64) -> Value<f64> {
    let click: Value<f64> = WhiteNoise.to_value() * 0.025.into();
    let cenv = Linear::new(0.001, 1.0, 0.01, 0.0, 0.0).to_value();
    let oenv = Linear::new(0.001, 1.0, 0.09, 0.0, 0.06).to_value();
    let osc = WaveTableSynth::sin(Value::<f64>::from(200.0) * oenv).to_value() * 0.5.into();
    let renv = Linear::new(0.001, 1.0, 0.09, 0.0, 0.12).to_value();
    let rattle: Value<f64> = WhiteNoise.to_value();
    let rfilter = RLPF::low_pass(rattle, Value::<f64>::from(1000.0)*renv, 1.0.into()).to_value();
    let env = Linear::new(0.001, 1.0, 0.0, 0.02, 0.005).to_value();
    ((click * cenv) + osc + rfilter) * env * amp.into()
}

fn bewww(duration: Duration, frequency: f64, amp: f64) -> Value<f64> {
    let fenv = Linear::new(0.05, 1.0, 0.09, 0.0, 0.12).to_value();
    let mut sig: Value<f64> = WaveTableSynth::sin(Value::<f64>::from(frequency) * fenv).to_value();

    let env = Linear::new(0.05, 1.0, 0.01, 0.2, 0.05).to_value();
    sig * env * amp.into()
}

fn swish(duration: Duration, frequency: f64, amp: f64) -> Value<f64> {
    //let mut sig: Value<f64> = BrownianNoise::new(0.03).to_value();
    let attack = duration.as_secs_f64() / 2.0;
    let decay = attack;
    let renv = Linear::new(attack, 1.0, 0.0, 0.0, decay).to_value();
    let mut sig: Value<f64> = BrownianNoise::new(renv).to_value();
    //sig = TrapezoidSVF::band(sig, renv, 0.9.into()).to_value();
    sig * amp.into()
}

fn main() {
    //let notes: Vec<(Duration, f64)> = (0..16).map(|_| (Duration::from_millis(rand::thread_rng().gen_range(80, 400)), rand::thread_rng().gen_range(120.0, 420.0))).collect();
    //let notes = vec![(Duration::new(1, 0), 440.0); 10];
    let bpm = 100;
    let beat = Duration::from_millis((1000*60) / bpm);

    let notes: Vec<_> = (0..100).map(|i| (beat / rand::thread_rng().gen_range(2, 8), 240.0 * rand::thread_rng().gen_range(1, 8) as f64, if (i%4==1) | (i%4==3) {0.0} else {1.0})).collect();
    let mut sig = SimpleSequence::new(Box::new(bewww), &notes, 3).to_value();

    //let notes: Vec<_> = (0..100).map(|i| beat * if i%, 240.0 * rand::thread_rng().gen_range(1, 8) as f64, if (i%4==1) | (i%4==3) {0.0} else {1.0})).collect();

    //sig = sig + SimpleSequence::new(Box::new(snare_drum), &notes, 3).to_value();
    
    let notes: Vec<_> = (0..100).map(|i| (Duration::new(rand::thread_rng().gen_range(2, 8), 0), 0.0, 0.01)).collect();
    sig = sig + SimpleSequence::new(Box::new(swish), &notes, 3).to_value();
    //let mut sig:Value<f64> = Sine::new(440.0.into()).to_value();
    //let mut sig:Value<f64> = WaveTableSynth::saw(440.0.into()).to_value();
    let notes: Vec<_> = (0..100).map(|i| (beat / 4, 0.0, if (i%4==2) | (i%4==4) {0.0} else {0.5})).collect();

    sig = sig + SimpleSequence::new(Box::new(bewww), &notes, 3).to_value();
    sig = Reverb::new(sig.to_value(), 0.8, 0.1, 1000.0, 3.8).to_value();

    sig = RingModulator::new(WaveTableSynth::sin(440.0.into()).to_value(), WaveTableSynth::sin(40.0.into()).to_value(), (WaveTableSynth::sin(WaveTableSynth::sin(1.3.into()).to_value() * 4.0.into() + 8.0.into()).to_value() + 1.0.into()) / 2.0).to_value();
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
