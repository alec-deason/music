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
    envelope::ADSR,
    filter::*,
    effect::*,
    sequence::*,
    note::*,  
};
use byteorder::{WriteBytesExt, LittleEndian};
use std::io::{self};    

fn chirp<'a>(note: Note) -> Value<'a, f64> {
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

fn pad<'a>(note: Note) -> Value<'a, f64> {
    let chirp_amount = 1.5;
    let freq1: Value<f64> = note.frequency.into();
    let freq2: Value<f64> = (note.frequency.midi_from_frequency() + 0.05).frequency_from_midi().into();
    let freq3: Value<f64> = (note.frequency/2.0).into();
    let osc1: Value<f64> = WaveTableSynth::saw(freq1 * chirp_amount).into();
    let osc2: Value<f64> = WaveTableSynth::saw(freq2 * chirp_amount).into();
    let osc3: Value<f64> = WaveTableSynth::sin(freq3 * chirp_amount).into();

    let amps = vec![1.0, 1.0, 2.0];
    let amp_sum: f64 = amps.iter().sum();
    let mut sig = (osc1*amps[0] + osc2*amps[1] + osc3*amps[2]) / amp_sum;
    sig = RLPF::new(sig, 1800.0, 0.5).into();

    let env: Value<f64> = ADSR::new().attack(0.4).sustain(1.0).duration(note.duration.as_secs_f64() - 0.4).release(0.3).curve(1.0).into();
    sig * env * note.amplitude
}

type HarmonicStructure<'a> = Vec<(f64, &'a Scale, Vec<u32>)>;
fn harmonic_structure<'a>(chord_durations: &[(f64, f64)], scale: &'a Scale, len: f64) -> HarmonicStructure<'a> {
    let mut rng = rand::thread_rng();
    let mut progression: Vec<(f64, i32, &str)> = vec![];
    let mut current_length = 0.0;
    let mut progression_pattern = "I-I-I-I-IV-IV-I-V-V-I-I".split("-").cycle();
    let terminal_chord = "I";

    while current_length < len || (progression.is_empty() || (progression.last().unwrap().2 != terminal_chord)) {
        let chord_duration = chord_durations.choose_weighted(&mut rng, |d| d.1).unwrap().0;
        current_length += chord_duration;
        progression.push((chord_duration, 0, progression_pattern.next().unwrap()));
    }
    
    progression.iter().map(|(d, octave, chord)| {
        let (base_degree, semitones) = parse_roman_numeral_notation(chord);
        let base = scale.pitch(base_degree as i32);
        let tones: Vec<_> = semitones.iter().map(|t| {
            let tone = (base as i32 + (octave * 12) + *t as i32) as u32;
            tone
        }).collect();
        (
            *d,
            scale,
            tones,
    )
    }).collect()
}

fn intensity_map(len: u32) -> Vec<f64> {
    let mut densities = vec![];
    let mut knots = vec![
        (0.0, 0.2),
        (0.1, 0.6),
        (0.4, 0.8),
        (0.6, 0.2),
        (0.8, 1.0),
        (1.0, 0.1),
    ];

    let (mut prev_time, mut prev_level) = knots.remove(0);
    let (mut next_time, mut next_level) = knots.remove(0);

    for i in 0..len {
        let t = i as f64 / len as f64;
        if t > next_time {
            prev_time = next_time;
            prev_level = next_level;
            let n = knots.remove(0);
            next_time = n.0;
            next_level = n.1;
        }
        let t = (t - prev_time) / (next_time - prev_time);
        let d = next_level - prev_level;
        densities.push(prev_level + d * t);
    }
    densities
}

fn accent(beat: u32) -> (bool, f64) {
    match beat % 4 {
        0 => (true, 1.0),
        1 => (false, 0.8),
        2 => (false, 0.8),
        3 => (true, 1.0),
        _ => (false, 0.8),
    }
}

fn melody<'a>(structure: &HarmonicStructure<'a>, subdivision: f64, threshold: f64, density: f64) -> Vec<(f64, u32, f64)> {
    let mut rng = rand::thread_rng();
    let intensity = intensity_map(structure.len() as u32);
    let mut result = vec![];
    let mut current_beat = 0.0;
    let mut previous_note = structure[0].1.pitch(0) as i32;

    for (intensity, (beats, scale, chord)) in intensity.iter().zip(structure) {
        if *intensity > threshold {
            let mut notes = (beats / subdivision) as u32;
            while notes > 0 {
                if rng.gen::<f64>() > density {
                    result.push((subdivision, 0u32, 0.0));
                    current_beat += subdivision;
                    notes -= 1;
                } else {
                    let (accented, amp) = accent(current_beat as u32);
                    if !accented && notes > 1 && !result.is_empty() && rng.gen::<f64>() > 0.6 {
                        //decorate
                        /*
                        let chord_max = *chord.iter().max().unwrap();
                        let chord_min = *chord.iter().min().unwrap();
                        let direction: i32 = if chord_max > previous_note && chord_min < previous_note {
                            *[-1, 1].choose(&mut rng).unwrap()
                        } else if chord_max > previous_note {
                            1
                        } else {
                            -1
                        };
                        */
                        let direction: i32 = *[-1, 1].choose(&mut rng).unwrap();

                        match rng.gen_range(0, 1) {
                            0 => {
                                // Passing note
                                let (octave, degree) = scale.degree(previous_note as u32).unwrap();
                                let degree = degree as i32 + direction  + octave*12;
                                let tone = scale.pitch(degree);
                                result.push((subdivision, tone as u32, amp));
                                current_beat += subdivision;
                                notes -= 1;
                                let degree = degree + direction;
                                let tone = scale.pitch(degree);
                                previous_note = tone as i32;
                                result.push((subdivision, tone as u32, amp));
                                current_beat += subdivision;
                                notes -= 1;
                            },
                            1 => {
                                // Escape Tone or anticipation
                                let (octave, degree) = scale.degree(previous_note as u32).unwrap();
                                let degree = degree as i32 + direction  + octave*12;
                                let tone = scale.pitch(degree);
                                result.push((subdivision, tone as u32, amp));
                                current_beat += subdivision;
                                notes -= 1;
                                let direction = -direction;
                                let tone = if direction > 0 {
                                    *chord.iter().filter(|x| **x > tone).min().unwrap_or(&tone)
                                } else {
                                    *chord.iter().filter(|x| **x < tone).max().unwrap_or(&tone)
                                };
                                previous_note = tone as i32;
                                result.push((subdivision, tone as u32, amp));
                                current_beat += subdivision;
                                notes -= 1;
                            },
                            _ => panic!(),
                        }
                    } else {
                        //regular chord note
                        let tone = *chord.choose(&mut rng).unwrap();
                        previous_note = tone as i32;
                        result.push((subdivision, tone, amp));
                        current_beat += subdivision;
                        notes -= 1;
                    }
                }
            }
        } else {
            result.push((*beats, scale.pitch(0), 0.0));
            current_beat += *beats;
        }
    }

    result
}

fn main() {
    let bpm = 180.0;
    let beat = 60000.0/bpm;
    let target_len = env::args().into_iter().nth(1).unwrap_or("10".to_string()).parse::<usize>().unwrap();
    let target_beats = (target_len as f64 * 1000.0) / beat;

    let scale = Scale::major(69);
    let structure = harmonic_structure(&[(4.0, 0.9), (2.0, 0.1)], &scale, target_beats);
    let voice_1_melody = melody(&structure, 1.0, 0.6, 0.8);
    let voice_2_melody = melody(&structure, 1.0, 0.2, 0.7);

    let pad: Value<f64> = sequence_from_iterator(
        structure.clone().into_iter()
        .map(move |(num_beats, _, chord)| {
            let mut sig: Value<f64> = 0.0.into();
            for tone in chord {
                let note = Note {
                    duration: Duration::from_millis((num_beats * beat) as u64),
                    frequency: (tone - 12).frequency_from_midi() as f64,
                    amplitude: 1.0,
                };
                sig = sig + pad(note);
            }
            (Duration::from_millis((num_beats * beat) as u64), sig)
        })).into();
    let mut beat_clock = 0;
    let bass: Value<f64> = sequence_from_iterator(
        structure.into_iter()
        .map(move |(num_beats, _, chord)| {
            let num_beats = num_beats;
            let chord = chord;
            (Duration::from_millis((num_beats * beat) as u64), sequence_from_iterator((0..num_beats as usize).map(move |_| {
                let note = Note {
                    duration: Duration::from_millis((beat * 0.8) as u64),
                    frequency: (chord[0] - 36).frequency_from_midi() as f64,
                    amplitude: accent(beat_clock).1,
                };
                beat_clock += 1;
                (Duration::from_millis(beat as u64), chirp(note))
            })).into())
        })).into();
    let voice_1: Value<f64> = sequence_from_iterator(
        voice_1_melody.into_iter()
        .map(move |(num_beats, tone, amp)| {
            let note = Note {
                duration: Duration::from_millis((num_beats * beat * 1.35) as u64),
                frequency: (tone).frequency_from_midi() as f64,
                amplitude: amp,
            };
            (Duration::from_millis((num_beats * beat) as u64), chirp(note))
        })).into();
    let voice_2: Value<f64> = sequence_from_iterator(
        voice_2_melody.into_iter()
        .map(move |(num_beats, tone, amp)| {
            let note = Note {
                duration: Duration::from_millis((num_beats * beat) as u64),
                frequency: (tone ).frequency_from_midi() as f64,
                amplitude: amp,
            };
            (Duration::from_millis((num_beats * beat) as u64), chirp(note))
        })).into();

    let mut sig = pad * 0.4 + bass * 0.8 + voice_1 * 0.0 + voice_2 * 1.0;
    sig = Reverb::new(sig, 0.8, 0.1, 1000.0, 6.8).into();

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
