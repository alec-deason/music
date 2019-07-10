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

fn bloops(f_mul: f64) -> Value<f64> {
    let scale: Vec<f64> = {
        let mut scale: Vec<u32> = MAJOR.iter().fold(vec![69], |mut acc, x| {acc.push(acc.last().unwrap()+x); acc});
        scale.extend(scale.clone().iter().map(|x| x - 12));
        scale.extend(scale.clone().iter().map(|x| x + 12));
        scale.iter().cloned().map(|x| x as f64).collect()
    };
    let sig: Value<f64> = IteratorSequence::new(move |note| {
            let fenv: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(0.25).duration(0.1).release(0.06).curve(0.01).into();
            let env: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(1.0).duration(note.duration.as_secs_f64() - 0.31).release(0.3).curve(0.5).into();
            let sig: Value<f64> = WaveTableSynth::sin(note.frequency * fenv * 4.0 * f_mul).into();
            sig * env * note.amplitude
        }).frequency(iter::repeat_with(move || {
            (*scale.choose(&mut rand::thread_rng()).unwrap() as f64).frequency_from_midi()
        })).duration(iter::repeat_with(|| Duration::from_millis(rand::thread_rng().gen_range(610, 800)))).into();
    sig * 0.2
}

fn arpeggiator(chord: &[f64], duration: Duration) -> Value<f64> {
    let note_duration = duration / chord.len() as u32;
    let chord: Vec<f64> = chord.iter().cloned().collect();
    IteratorSequence::new(move |note| {  
            let fenv: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(0.25).duration(0.1).release(0.06).curve(0.01).into();
            let env: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(1.0).duration(note.duration.as_secs_f64() - 0.31).release(0.3).curve(0.5).into();
            let sig: Value<f64> = WaveTableSynth::sin(note.frequency * fenv * 4.0).into();
            sig * env * note.amplitude
    }).frequency(chord.into_iter().map(|x| x.frequency_from_midi()))
    .duration(iter::repeat(note_duration))
    .into()
}

fn basic_chord(chord: &[f64], duration: Duration) -> Value<f64> {
    let mut sig: Value<f64> = 0.0.into();
    for note in chord {
        let fenv: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(0.25).duration(duration.as_secs_f64() - 0.09).release(0.06).curve(0.01).into();
        let env: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(1.0).duration(duration.as_secs_f64() - 0.31).release(0.3).curve(0.5).into();
        let sig2: Value<f64> = WaveTableSynth::sin(note.frequency_from_midi() * fenv * 4.0).into();
        sig = sig + sig2 * env
    }
    sig / chord.len() as f64
}

fn circle(f_mul: f64) -> Value<f64> {
    let scale: Vec<f64> = {
        let scale: Vec<f64> = MAJOR.iter().fold(vec![69.0], |mut acc, x| {
            acc.push(
                acc.last().unwrap()+(*x as f64)
            );
            acc
        });
        scale.iter().map(|x| x.frequency_from_midi()).collect()
    };
    let mut riff_notes =     vec![0usize, 2, 3, 3, 6, 7, 0];
    let mut riff_durations = vec![1usize, 1, 2, 1, 2, 2, 1];
    riff_notes.extend(riff_notes.clone().iter().rev());
    riff_durations.extend(riff_durations.clone().iter().rev());
    let beat = 200;
    let riff_notes: Vec<f64> = riff_notes.iter().map(|n| scale[*n]).collect();
    let riff_durations: Vec<Duration> = riff_durations.iter().map(|n| Duration::from_millis((n * beat) as u64)).collect();
    let sig: Value<f64> = IteratorSequence::new(move |note| {
            let fenv: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(0.25).duration(0.1).release(0.06).curve(0.01).into();
            let env: Value<f64> = ADSR::new().attack(0.01).release(0.01).sustain(1.0).duration(note.duration.as_secs_f64() - 0.31).release(0.3).curve(0.5).into();
            let sig: Value<f64> = WaveTableSynth::sin(note.frequency * fenv * 4.0 * f_mul).into();
            sig * env * note.amplitude
        }).frequency(riff_notes.into_iter().cycle())
        .duration(riff_durations.into_iter().cycle()).into();
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
    let scale: Vec<f64> = {
        let mut scale: Vec<f64> = MAJOR.iter().fold(vec![69.0], |mut acc, x| {
            acc.push(
                acc.last().unwrap()+(*x as f64)
            );
            acc
        });
        scale.remove(0);
        scale.extend(scale.clone().iter().map(|x| x+12.0));
        scale
    };

    let chord_class = iter::repeat(vec![0, 4, 7]); // Major triads
    let chord_root = [0, 3, 4, 4, 0, 3, 4, 0].iter().cycle(); // I-IV-V-V - I-IV-V-I progression
    let arpeggio_duration = Duration::from_millis(600);
    let mut sig: Value<f64> = SimpleSequence::new(
        chord_root.zip(chord_class)
        .map(move |(r, cs)| cs.iter().map(|c| scale[(c+r) as usize] - 12.0).collect::<Vec<f64>>())
        .map(move |chord| {
            if rand::thread_rng().gen_range(0.0, 1.0) > 0.5 {
             (arpeggio_duration, basic_chord(&chord, arpeggio_duration))
            } else {
             (arpeggio_duration, arpeggiator(&chord, arpeggio_duration))
            }
        })).into();
    //let mut sig = bloops(1.0);
    //sig = sig + bloops(0.5);
    //sig = sig + bloops(0.25);
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
