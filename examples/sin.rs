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
    let osc1: Value<f64> = WaveTableSynth::saw(freq).into();
    let freq: Value<f64> = (frequency / 0.9931).into();
    let osc2 = WaveTableSynth::saw(freq).into();
    let freq: Value<f64> = (frequency / 2.0).into();
    let osc3 = WaveTableSynth::sin(freq).into();
    let ffreq: Value<f64> = 1800.0.into();
    let ffreq_env = Linear::new(0.01, 1.0, 0.01, duration.as_secs_f64(), 0.1).into();
    let env: Value<f64> = Linear::new(0.01, 1.0, 0.01, 1.0, 0.07).into();
    let fq: Value<f64> = 0.5.into();
    let sig: Value<f64> = RLPF::new(osc1+osc2+osc3, ffreq*ffreq_env + 80.0.into(), fq).into();
    sig * env * amp.into()
}

fn vangelis(duration: Duration, frequency: f64, amp: f64) -> Value<f64> {
    let osc1: Value<f64> = WaveTableSynth::saw(frequency.into()).into();
    let lfo: Value<f64> = WaveTableSynth::sin(0.12.into()).into();
    let osc2: Value<f64> = WaveTableSynth::saw(Value::<f64>::from(frequency) / (Value::<f64>::from(1.0) -lfo * 0.1.into())).into();
    let mut sig = osc1 + osc2;
    let env = Linear::new(0.41, 1.0, 0.0, duration.as_secs_f64(), 0.41).into();
    let fenv = Linear::new(1.6, 1.0, 0.0, duration.as_secs_f64(), 4.2).into();
    sig = RLPF::new(sig, Value::<f64>::from(7000.0)*fenv+100.0.into(), 1.0.into()).into();
    //sig = Reverb::new(sig, 0.5, 0.1, 4000.0, 4.0).into();
    sig * env * amp.into()
}

fn kick_drum(_duration: Duration, _frequency: f64, amp: f64) -> Value<f64> {
    let click: Value<f64> = WhiteNoise.into();
    let click: Value<f64> = click * 0.025.into();
    let cenv = Linear::new(0.001, 1.0, 0.01, 0.0, 0.0).into();
    let oenv = Linear::new(0.001, 1.0, 0.09, 0.0, 0.06).into();
    let osc = WaveTableSynth::sin(Value::<f64>::from(58.0) * oenv).into();
    let env = Linear::new(0.001, 1.0, 0.0, 0.02, 0.005).into();
    ((click * cenv) + osc) * env * amp.into()
}

fn snare_drum(_duration: Duration, _frequency: f64, amp: f64) -> Value<f64> {
    let click: Value<f64> = WhiteNoise.into();
    let click: Value<f64> = click * 0.025.into();
    let cenv = Linear::new(0.001, 1.0, 0.01, 0.0, 0.0).into();
    let oenv = Linear::new(0.001, 1.0, 0.09, 0.0, 0.06).into();
    let osc: Value<f64> = WaveTableSynth::sin(Value::<f64>::from(200.0) * oenv).into();
    let osc = osc * 0.5.into();
    let renv = Linear::new(0.001, 1.0, 0.09, 0.0, 0.12).into();
    let rattle: Value<f64> = WhiteNoise.into();
    let rfilter = RLPF::new(rattle, Value::<f64>::from(1000.0)*renv, 1.0.into()).into();
    let env = Linear::new(0.001, 1.0, 0.0, 0.02, 0.005).into();
    ((click * cenv) + osc + rfilter) * env * amp.into()
}

fn bewww(duration: Duration, frequency: f64, amp: f64) -> Value<f64> {
    let fenv = Linear::new(0.05, 1.0, 0.09, 0.0, 0.12).into();
    let mut sig: Value<f64> = WaveTableSynth::sin(Value::<f64>::from(frequency) * fenv).into();

    let env = Linear::new(0.05, 1.0, 0.01, 0.2, 0.05).into();
    sig * env * amp.into()
}

fn swish(duration: Duration, frequency: f64, amp: f64) -> Value<f64> {
    //let mut sig: Value<f64> = BrownianNoise::new(0.03).into();
    let attack = duration.as_secs_f64() / 2.0;
    let decay = attack;
    let renv: Value<f64> = Linear::new(attack, 1.0, 0.0, 0.0, decay).into();
    let mut sig: Value<f64> = BrownianNoise::new(renv).into();
    //sig = TrapezoidSVF::band(sig, renv, 0.9.into()).into();
    sig * amp.into()
}

fn main() {
    //let notes: Vec<(Duration, f64)> = (0..16).map(|_| (Duration::from_millis(rand::thread_rng().gen_range(80, 400)), rand::thread_rng().gen_range(120.0, 420.0))).collect();
    //let notes = vec![(Duration::new(1, 0), 440.0); 10];
    let bpm = 100;
    let beat = Duration::from_millis((1000*60) / bpm);

    let notes: Vec<_> = (0..100).map(|i| (beat / rand::thread_rng().gen_range(2, 8), 240.0 * rand::thread_rng().gen_range(1, 8) as f64, if (i%4==1) | (i%4==3) {0.0} else {1.0})).collect();
    let mut sig: Value<f64> = SimpleSequence::new(Box::new(bewww), &notes, 3).into();

    //let notes: Vec<_> = (0..100).map(|i| beat * if i%, 240.0 * rand::thread_rng().gen_range(1, 8) as f64, if (i%4==1) | (i%4==3) {0.0} else {1.0})).collect();

    //sig = sig + SimpleSequence::new(Box::new(snare_drum), &notes, 3).into();
    
    let notes: Vec<_> = (0..100).map(|i| (Duration::new(rand::thread_rng().gen_range(2, 8), 0), 0.0, 0.01)).collect();
    sig = sig + SimpleSequence::new(Box::new(swish), &notes, 3).into();
    //let mut sig:Value<f64> = Sine::new(440.0.into()).into();
    //let mut sig:Value<f64> = WaveTableSynth::saw(440.0.into()).into();
    let notes: Vec<_> = (0..100).map(|i| (beat / 4, 0.0, if (i%4==2) | (i%4==4) {0.0} else {0.5})).collect();

    sig = sig + SimpleSequence::new(Box::new(bewww), &notes, 3).into();
    sig = Reverb::new(sig.into(), 0.8, 0.1, 1000.0, 3.8).into();

    let sig_in: Value<f64> = WaveTableSynth::sin(440.0.into()).into();
    let sig_modulator: Value<f64> = WaveTableSynth::sin(40.0.into()).into();
    let sig_mix_modulator: Value<f64> = WaveTableSynth::sin(1.3.into()).into();
    let sig_mix: Value<f64> = WaveTableSynth::sin(sig_mix_modulator * 4.0.into() + 8.0.into()).into();
    let sig_mix = (sig_mix + 1.0.into()) / 2.0.into();
    sig = RingModulator::new(sig_in, sig_modulator, sig_mix).into();
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
