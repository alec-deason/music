#![feature(duration_float)]

use std::env;
use std::iter;
use std::time::Duration;
use rand::Rng;
use rand::seq::SliceRandom;

use music::{
    Env,
    value::{Value, CacheValue},
    oscillator::*,
    envelope::ADSR,
    filter::*,
    effect::*,
    sequence::*,
    note::*,
};
use byteorder::{WriteBytesExt, LittleEndian};
use std::io::{self};

fn chirp(note: Note) -> Value<f64> {
    let chirp_amount = 1.5;
    let fenv = CacheValue::new(ADSR::new().attack(0.02).decay(0.02).sustain(1.0/chirp_amount).duration(note.duration.as_secs_f64()*2.0).release(0.06).curve(0.01));
    let freq1: Value<f64> = note.frequency * Value::<f64>::from(fenv.clone()) * chirp_amount;
    let freq2: Value<f64> = (note.frequency.midi_from_frequency() + 0.05).frequency_from_midi() * Value::<f64>::from(fenv.clone()) * chirp_amount;
    let freq3: Value<f64> = (note.frequency/2.0) * Value::<f64>::from(fenv.clone()) * chirp_amount;
    let osc1: Value<f64> = WaveTableSynth::saw(freq1 * chirp_amount).into();
    let osc2: Value<f64> = WaveTableSynth::saw(freq2 * chirp_amount).into();
    let osc3: Value<f64> = WaveTableSynth::sin(freq3 * chirp_amount).into();

    let amps = vec![1.0, 1.0, 2.0];
    let amp_sum: f64 = amps.iter().sum();
    let mut sig = (osc1*amps[0] + osc2*amps[1] + osc3*amps[2]) / amp_sum;
    sig = RLPF::new(sig, 1800.0, 0.5).into();

    let env: Value<f64> = ADSR::new().attack(0.03).sustain(1.0).duration(note.duration.as_secs_f64()).release(0.3).curve(1.0).into();
    sig * env * note.amplitude
}

fn arpeggiator(chord: &[f64], duration: Duration) -> Value<f64> {
    let note_duration = duration / chord.len() as u32;
    let chord: Vec<f64> = chord.iter().cloned().collect();
    IteratorSequence::new(chirp)
        .frequency(chord.into_iter().map(|x| x.frequency_from_midi()))
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

fn outline_chord_structure(chord_len: usize, length: Duration, beat: Duration) -> (Duration, Vec<(Duration, usize)>) {
    let beat = beat / 2;
    let mut current_length = Duration::new(0, 0);
    let mut notes = vec![];
    let durations = vec![1, 2, 2, 3, 3, 3, 4, 4, 4];
    while current_length < length {
        let idx = rand::thread_rng().gen_range(0, chord_len);
        let duration = beat * *durations.choose(&mut rand::thread_rng()).unwrap();
        notes.push((duration, idx));
        current_length += duration;
    }
    (current_length, notes)
}

fn outline_chord(chord: &[f64], length: Duration, beat: Duration) -> (Duration, Vec<(Duration, f64)>) {
    let (total_dur, notes) = outline_chord_structure(chord.len(), length, beat);
    (
        total_dur,
        notes.iter().map(|(d, i)| (*d, chord[*i])).collect()
    )
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
    //let chord_root = [0, 3, 4, 4, 0, 3, 4, 0].iter().cycle(); // I-IV-V-V - I-IV-V-I progression

    let chord_root = [0, 0, 0, 0, 3, 3, 0, 0, 4, 3].iter().cycle();
    let arpeggio_duration = Duration::from_millis(2500);
    let scale1 = scale.clone();
    let mut sig: Value<f64> = SimpleSequence::new(
        chord_root.clone().zip(chord_class.clone())
        .map(move |(r, cs)| cs.iter().map(|c| scale1[(c+r) as usize] - 12.0).collect::<Vec<f64>>())
        .map(move |chord| {
            let (dur, notes) = outline_chord(&chord, arpeggio_duration, arpeggio_duration / 8);
            let (durations, frequencies): (Vec<_>, Vec<_>) = notes.iter().cloned().unzip();
            (dur, IteratorSequence::new(chirp).duration(durations).frequency(frequencies.into_iter().map(|x| x.frequency_from_midi())).into())
        })).into();

    let scale1 = scale.clone();
    let (theme_dur, theme_notes) = outline_chord_structure(3, arpeggio_duration, arpeggio_duration / 8);
    let bass: Value<f64> = SimpleSequence::new(
        chord_root.clone().zip(chord_class.clone())
        .map(move |(r, cs)| cs.iter().map(|c| scale1[(c+r) as usize]).collect::<Vec<f64>>())
        .map(move |chord| {
            let (durations, frequencies): (Vec<_>, Vec<_>) = theme_notes.iter().map(|(d, i)| (*d, chord[*i])).unzip();
            (theme_dur, IteratorSequence::new(chirp).duration(durations).frequency(frequencies.into_iter().map(|x| x.frequency_from_midi())).into())
        })).into();

    let scale1 = scale.clone();
    let fill: Value<f64> = SimpleSequence::new(
        chord_root.clone().zip(chord_class.clone())
        .map(move |(r, cs)| cs.iter().map(|c| scale1[(c+r) as usize] - 36.0).collect::<Vec<f64>>())
        .map(move |chord| {
            let (dur, notes) = outline_chord(&chord, arpeggio_duration, arpeggio_duration / 4);
            let (durations, frequencies): (Vec<_>, Vec<_>) = notes.iter().cloned().unzip();
            (dur, IteratorSequence::new(chirp).duration(durations).frequency(frequencies.into_iter().map(|x| x.frequency_from_midi())).into())
        })).into();

    sig = fill + sig + bass;
    sig = Reverb::new(sig, 0.8, 0.1, 2000.0, 4.8).into();

    let env = Env::new(44100);
    let len = env::args().into_iter().nth(1).unwrap_or("10".to_string()).parse::<usize>().unwrap();
    let mut buffer_left = vec![0.0; env.sample_rate as usize*len];
    sig.fill_buffer(&env, &mut buffer_left, 0, env.sample_rate as usize*len);
    let mut buffer_right = vec![0.0; (env.sample_rate as f64 * 0.05) as usize];
    buffer_right.extend(buffer_left.iter());
    let amp = 0.25;
    for (left, right) in buffer_left.iter().zip(buffer_right) {
        io::stdout().write_f32::<LittleEndian>(*left as f32 * amp).unwrap();
        io::stdout().write_f32::<LittleEndian>(right as f32 * amp).unwrap();
    }

}
