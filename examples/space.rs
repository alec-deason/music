#![feature(duration_float)]

use rand::seq::SliceRandom;
use rand::Rng;
use std::cell::RefCell;
use std::env;
use std::iter;
use std::rc::Rc;
use std::time::Duration;

use byteorder::{LittleEndian, WriteBytesExt};
use music::{
    effect::*,
    envelope::ADSR,
    filter::*,
    note::*,
    oscillator::*,
    sequence::*,
    value::{CacheValue, Value},
    Env,
};
use std::io::{self};

fn chirp(note: Note) -> Value<f64> {
    let chirp_amount = 1.5;
    let fenv = CacheValue::new(
        ADSR::new()
            .attack(0.02)
            .decay(0.02)
            .sustain(1.0 / chirp_amount)
            .duration(note.duration.as_secs_f64() * 2.0)
            .release(0.06)
            .curve(0.01),
    );
    let freq1: Value<f64> = note.frequency * Value::<f64>::from(fenv.clone()) * chirp_amount;
    let freq2: Value<f64> = (note.frequency.midi_from_frequency() + 0.05).frequency_from_midi()
        * Value::<f64>::from(fenv.clone())
        * chirp_amount;
    let freq3: Value<f64> =
        (note.frequency / 2.0) * Value::<f64>::from(fenv.clone()) * chirp_amount;
    let osc1: Value<f64> = WaveTableSynth::saw(freq1 * chirp_amount).into();
    let osc2: Value<f64> = WaveTableSynth::saw(freq2 * chirp_amount).into();
    let osc3: Value<f64> = WaveTableSynth::sin(freq3 * chirp_amount).into();

    let amps = vec![1.0, 1.0, 2.0];
    let amp_sum: f64 = amps.iter().sum();
    let mut sig = (osc1 * amps[0] + osc2 * amps[1] + osc3 * amps[2]) / amp_sum;
    sig = RLPF::new(sig, 1800.0, 0.5).into();

    let env: Value<f64> = ADSR::new()
        .attack(0.03)
        .sustain(1.0)
        .duration(note.duration.as_secs_f64())
        .release(0.3)
        .curve(1.0)
        .into();
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
        let fenv: Value<f64> = ADSR::new()
            .attack(0.01)
            .release(0.01)
            .sustain(0.25)
            .duration(duration.as_secs_f64() - 0.09)
            .release(0.06)
            .curve(0.01)
            .into();
        let env: Value<f64> = ADSR::new()
            .attack(0.01)
            .release(0.01)
            .sustain(1.0)
            .duration(duration.as_secs_f64() - 0.31)
            .release(0.3)
            .curve(0.5)
            .into();
        let sig2: Value<f64> = WaveTableSynth::sin(note.frequency_from_midi() * fenv * 4.0).into();
        sig = sig + sig2 * env
    }
    sig / chord.len() as f64
}

fn outline_chord_structure(
    chord_len: usize,
    length: Duration,
    beat: Duration,
    root_first: bool,
    density: f64,
) -> (Duration, Vec<(Duration, usize, f64)>) {
    let beat = beat / 2;
    let mut current_length = Duration::new(0, 0);
    let mut notes = vec![];
    let durations = vec![1, 2, 2, 3, 3, 3, 4, 4, 4];
    while current_length < length {
        let idx = if root_first & (notes.len() == 0) {
            0
        } else {
            rand::thread_rng().gen_range(0, chord_len)
        };
        let duration = beat * *durations.choose(&mut rand::thread_rng()).unwrap();
        let amp = if rand::thread_rng().gen::<f64>() < density {
            1.0
        } else {
            0.0
        };
        notes.push((duration, idx, amp));
        current_length += duration;
    }
    (current_length, notes)
}

fn outline_chord(
    chord: &[f64],
    length: Duration,
    beat: Duration,
    root_first: bool,
    density: f64,
) -> (Duration, Vec<(Duration, f64, f64)>) {
    let (total_dur, notes) =
        outline_chord_structure(chord.len(), length, beat, root_first, density);
    (
        total_dur,
        notes.iter().map(|(d, i, a)| (*d, chord[*i], *a)).collect(),
    )
}

fn unzip3<A, B, C, FromA, FromB, FromC>(
    src: impl Iterator<Item = (A, B, C)>,
) -> (FromA, FromB, FromC)
where
    FromA: Default + Extend<A>,
    FromB: Default + Extend<B>,
    FromC: Default + Extend<C>,
{
    let mut ts: FromA = Default::default();
    let mut us: FromB = Default::default();
    let mut vs: FromC = Default::default();

    src.for_each(|(t, u, v)| {
        ts.extend(Some(t));
        us.extend(Some(u));
        vs.extend(Some(v));
    });

    (ts, us, vs)
}
fn main() {
    let scale: Vec<f64> = {
        let mut scale: Vec<f64> = MAJOR.iter().fold(vec![69.0], |mut acc, x| {
            acc.push(acc.last().unwrap() + (*x as f64));
            acc
        });
        scale.remove(0);
        scale.extend(scale.clone().iter().map(|x| x));
        scale
    };

    //let chords = "I-ii-iii-IV-V-IV-iii-ii-I".split("-").map(|n| parse_roman_numeral_notation(n)).cycle();
    let chords = "iii-VI-V-II-I"
        .split("-")
        .map(|n| parse_roman_numeral_notation(n))
        .cycle();

    let arpeggio_duration = Duration::from_millis(2500);
    let density = Rc::new(RefCell::new(-0.1f64));
    let mut density_delta = 0.1;
    let mut measure_clock = 0;

    let scale1 = scale.clone();
    let density1 = density.clone();
    let top_notes: Value<f64> = sequence_from_iterator(
        chords
            .clone()
            .map(move |cs| cs.iter().map(|c| scale1[*c]).collect::<Vec<f64>>())
            .map(move |chord| {
                let (dur, notes) = outline_chord(
                    &chord,
                    arpeggio_duration,
                    arpeggio_duration / 8,
                    false,
                    if *density1.borrow() > 0.0 { 1.0 } else { 0.0 },
                );
                let nd = (*density1.borrow() + density_delta).min(1.5);
                *density1.borrow_mut() = nd;
                measure_clock += 1;
                if measure_clock == 17 {
                    density_delta *= -1.0;
                }
                let (durations, frequencies, amps): (Vec<_>, Vec<_>, Vec<_>) =
                    unzip3(notes.iter().cloned());
                (
                    dur,
                    IteratorSequence::new(chirp)
                        .duration(durations)
                        .frequency(frequencies.into_iter().map(|x| x.frequency_from_midi()))
                        .amplitude(amps)
                        .into(),
                )
            }),
    )
    .into();

    let scale1 = scale.clone();
    let density1 = density.clone();
    let (mut theme_dur, mut theme_notes) = outline_chord_structure(
        3,
        arpeggio_duration,
        arpeggio_duration / 8,
        false,
        *density1.borrow() - 0.1,
    );
    let structure: Value<f64> = sequence_from_iterator(
        chords
            .clone()
            .map(move |cs| cs.iter().map(|c| scale1[*c] + 12.0).collect::<Vec<f64>>())
            .map(move |chord| {
                if rand::thread_rng().gen::<f64>() > 0.7 {
                    let (ndur, nnotes) = outline_chord_structure(
                        3,
                        arpeggio_duration,
                        arpeggio_duration / 8,
                        false,
                        *density1.borrow() - 0.1,
                    );
                    theme_dur = ndur;
                    theme_notes = nnotes;
                }
                let (durations, frequencies, amp): (Vec<_>, Vec<_>, Vec<_>) =
                    unzip3(theme_notes.iter().map(|(d, i, a)| (*d, chord[*i], *a)));
                (
                    theme_dur,
                    IteratorSequence::new(chirp)
                        .duration(durations)
                        .frequency(frequencies.into_iter().map(|x| x.frequency_from_midi()))
                        .amplitude(amp)
                        .into(),
                )
            }),
    )
    .into();

    let scale1 = scale.clone();
    let density1 = density.clone();
    let pad: Value<f64> = sequence_from_iterator(
        chords
            .clone()
            .map(move |cs| cs.iter().map(|c| scale1[*c] - 24.0).collect::<Vec<f64>>())
            .map(move |chord| {
                let note = Note {
                    duration: arpeggio_duration,
                    frequency: chord[0].frequency_from_midi(),
                    amplitude: if *density1.borrow() >= -0.2 { 1.0 } else { 0.0 },
                };
                (arpeggio_duration, chirp(note))
            }),
    )
    .into();

    let scale1 = scale.clone();
    let density1 = density.clone();
    let bass_line: Value<f64> = sequence_from_iterator(
        chords
            .clone()
            .map(move |cs| cs.iter().map(|c| scale1[*c] - 36.0).collect::<Vec<f64>>())
            .map(move |chord| {
                let root = chord[0].frequency_from_midi();
                let mut notes = vec![];
                for _ in 0..8 {
                    let note = Note {
                        duration: arpeggio_duration / 8,
                        frequency: root,
                        amplitude: if *density1.borrow() > 0.0 { 1.0 } else { 0.0 },
                    };
                    notes.push((arpeggio_duration / 8, chirp(note)));
                }
                (arpeggio_duration, sequence_from_iterator(notes).into())
            }),
    )
    .into();

    let mut sig = pad + bass_line + top_notes + structure;
    sig = Reverb::new(sig, 0.8, 0.1, 2000.0, 4.8).into();

    let mut env = Env::new(44100);
    let len = env::args()
        .into_iter()
        .nth(1)
        .unwrap_or("10".to_string())
        .parse::<usize>()
        .unwrap();
    let chunk_size = 2048;
    let total_samples = env.sample_rate as usize * len;
    for _ in 0..total_samples / chunk_size {
        let mut buffer_left = vec![0.0; chunk_size];
        sig.fill_buffer(&mut env, &mut buffer_left, 0, chunk_size);
        let amp = 0.25;
        for left in buffer_left {
            io::stdout()
                .write_f32::<LittleEndian>(left as f32 * amp)
                .unwrap();
            io::stdout()
                .write_f32::<LittleEndian>(left as f32 * amp)
                .unwrap();
        }
    }
}
