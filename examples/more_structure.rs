#![feature(duration_float)]

use std::collections::HashMap;
use std::time::Duration;
use std::env;
use rand::Rng;
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use byteorder::{WriteBytesExt, LittleEndian};
use std::io::{self};


use music::{
    Env,
    value::*,
    oscillator::*,
    oscillator::string::*,
    envelope::*,
    filter::*,
    effect::*,
    sequence::*,
    note::*,  
};

mod plan {
    use super::*;

    pub type Parts = Vec<SubPart>;
    pub type Key = Scale;
    pub type Tone = u32;
    pub type Chord = Vec<Tone>;
    pub type Intensity = f64;
    pub type Density = f64;
    pub type Duration = f64;
    pub type ChordProgression = Vec<(Duration, Intensity, Density, Chord)>;
    pub type SubPart = (Key, ChordProgression);

    fn smooth_progression(key: &Key, leadin_chord: &Chord, count: u32, augmentation_prob: f64, local_density: f64, rng: &mut impl Rng) -> ChordProgression {
        let chord_map: Vec<(i32, Vec<usize>)> = vec![
            (2, vec![1, 5]),
            (3, vec![2, 6]),
            (4, vec![3, 4, 7]),
            (5, vec![4]),
            (1, vec![0, 1, 2, 3, 4, 5, 6, 7]),
            (5, vec![1, 6]),
            (6, vec![2, 7]),
            (2, vec![3]),
        ];

        let change_direction_prob = rng.gen_range(0.2, 0.8);
        let octave_jump_prob = rng.gen_range(0.0, 0.3);

        let mut direction = *[-1, 1].choose(rng).unwrap();
        let mut progression = vec![];
        let (mut octave, current) = key.degree(leadin_chord[0]).unwrap();
        let mut state = chord_map.iter().enumerate().filter(|(_, s)| s.0 == current as i32).nth(0).unwrap_or((4, &(6, vec![2, 7]))).0;
        while progression.len() < count as usize  {
            let mut next_states = chord_map[state].1.clone();
            next_states.sort();
            if direction < 0 {
                next_states.reverse();
            }
            if rng.gen::<f64>() < change_direction_prob {
                direction *= -1;
            }
            let next_states: Vec<_> = next_states.iter().enumerate().collect();

            state = *next_states.choose_weighted(rng, |(i, _)| *i+1).unwrap().1;

            let mut degree = chord_map[state].0;
            if rng.gen::<f64>() < octave_jump_prob {
                octave = (octave + direction).min(1).max(-2);
            }

            let mut chord = key.triad(degree + octave * 7);
            if state != 3 && rng.gen::<f64>() < augmentation_prob {
                let (octave, degree) = key.degree(chord[rng.gen_range(0, chord.len())]).unwrap();
                let degree = degree as i32 + *[-1, 1].choose(rng).unwrap();
                chord.push((key.pitch(degree) as i32 + octave as i32*12) as u32);
                chord.sort();
            }
            progression.push((4.0, 1.0, local_density, chord));
        }
        progression
    }

    pub fn new(rng: &mut impl Rng) -> Parts {
        let mut parts: Parts = vec![];
        let pattern = *[Pattern::Major, Pattern::Minor].choose(rng).unwrap();
        let root = 69 + rng.gen_range(-12, 12);
        let key = Scale::new(pattern, root as u32);
        let augmentation_prob = rng.gen_range(0.0, 0.9);
        let mut last_end = key.triad(0);
        for _ in 0..10 {
            if parts.len() > 0 && rng.gen::<f64>() < 0.6 {
                let part = parts.choose(rng).unwrap().clone();
                last_end = part.1[part.1.len()-1].3.clone();
                parts.push(part);
            } else {
                let density = if rng.gen::<f64>() > 0.5 { 1.0 } else { 2.0 };
                let progression = smooth_progression(&key, &last_end, 8, augmentation_prob, density, rng);
                last_end = progression[progression.len()-1].3.clone();
                parts.push((key.clone(), progression));
            }
        }
        parts
    }
}

mod voicing {
    use super::*;

    #[derive(Copy, Clone, Debug)]
    pub enum Voice {
        Harmony(HarmonyType, i32),
        Melody(i32, f64),
    }

    #[derive(Copy, Clone, Debug)]
    pub enum HarmonyType {
        Chord,
        ArpeggiatedChord(u32),
        RepeatedRoot,
    }

    fn fill_harmony(parts: &plan::Parts, t: HarmonyType, octave: i32, rng: &mut impl Rng) -> Vec<(f64, Option<Vec<(u32, f64)>>)> {
        let mut notes = vec![];
        let mut beat_clock = 0.0;
        for (key, progression) in parts {
            for (dur, inten, _, chord) in progression {
                match t {
                    HarmonyType::Chord => {
                        let (accented, amp) = accent(beat_clock as u32);
                        notes.push((*dur, Some(
                            chord.iter().map(|t| ((*t as i32 + octave*12) as u32, amp)).collect()
                        )));
                        beat_clock += dur;
                    },
                    HarmonyType::ArpeggiatedChord(direction) => {
                        for i in 0..*dur as usize {
                            let i = if direction == 0 {
                                i
                            } else if direction == 1 {
                                *dur as usize - i
                            } else {
                                rng.gen_range(0, *dur as usize)
                            };
                            let (accented, amp) = accent(beat_clock as u32);
                            notes.push((1.0, Some(vec![((chord[i % chord.len()] as i32 + octave*12) as u32, amp)])));
                            beat_clock += 1.0;
                        }
                    },
                    HarmonyType::RepeatedRoot => {
                        for _ in 0..*dur as u32 {
                            let (accented, amp) = accent(beat_clock as u32);
                            notes.push((1.0, Some(vec![((chord[0] as i32 + octave*12) as u32, amp)])));
                            beat_clock += 1.0;
                        }
                    },
                }
            }
        }
        notes
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

    #[derive(Copy, Clone, Debug)]
    enum Tone {
        ChordTone(usize),
        NonChordTone(i32),
        StepFromChordTone(usize, i32),
        AdjacentChordTone(usize, i32),
    }

    fn random_fill_melody(parts: &plan::Parts, octave: i32, density: f64, rng: &mut impl Rng) -> Vec<(f64, Option<Vec<(u32, f64)>>)> {
        let passing_note_prob = rng.gen_range(0.0, 0.8);
        let break_direction_prob = rng.gen_range(0.1, 0.6);
        let switch_direction_prob = rng.gen_range(0.1, 0.6);
        let repeat_measure_prob = rng.gen_range(0.4, 0.8);
        let repeat_distance_set = vec![1, 2, 3];
        let double_prob = rng.gen_range(0.2, 0.8);

        let mut doubled = false;
        let mut direction: i32 = *[-1, 1].choose(rng).unwrap();
        let mut last_chord_position = 0;
        let subdivision = 1.0;
        let mut beat_clock = 0.0;
        let mut all_notes: Vec<Vec<(f64, Option<Vec<(Tone, f64)>>)>> = vec![];
        let mut local_subdivision = subdivision;
        for (_key, progression) in parts {
            for (dur, _inten, local_density, chord) in progression {
                if all_notes.len() > 0 && rng.gen::<f64>() < repeat_measure_prob {
                    let idx = *repeat_distance_set.choose(rng).unwrap();
                    let idx = (all_notes.len() as i32 - idx).max(0);
                    all_notes.push(all_notes[idx as usize].clone());
                    continue
                }
                let mut notes = vec![];
                let mut beats_remaining = *dur * subdivision;
                while beats_remaining > 0.0 {
                    if doubled && rng.gen::<f64>() < 1.0 - double_prob {
                        doubled = false;
                        local_subdivision = subdivision / local_density;
                    } else if rng.gen::<f64>() < double_prob {
                        doubled = true;
                        local_subdivision = (subdivision * 2.0) / local_density;
                    }
                    if rng.gen::<f64>() < switch_direction_prob {
                        direction *= -1;
                    }
                    if rng.gen::<f64>() < density {
                        if beats_remaining < 3.0/local_subdivision || rng.gen::<f64>() > passing_note_prob {
                            let (accented, amp) = accent(beat_clock as u32);
                            let tone = if rng.gen::<f64>() < break_direction_prob {
                                last_chord_position = (last_chord_position + direction).min(chord.len() as i32 -1).max(0);
                                Tone::ChordTone(last_chord_position as usize)
                            } else {
                                last_chord_position = rng.gen_range(0, chord.len() as i32);
                                Tone::ChordTone(last_chord_position as usize)
                            };
                            notes.push((1.0 / local_subdivision, Some(vec![(tone, amp)])));
                            beat_clock += 1.0/local_subdivision;
                            beats_remaining -= 1.0/local_subdivision;
                        } else  {
                            //Passing tone
                            let direction = *[-1, 1].choose(rng).unwrap();
                            let mut tone = if direction > 0 {
                                rng.gen_range(0, chord.len() as i32 -1)
                            } else {
                                rng.gen_range(1, chord.len() as i32)
                            };
                            let (accented, amp) = accent(beat_clock as u32);
                            notes.push((1.0 / local_subdivision, Some(vec![(Tone::ChordTone(tone as usize), amp)])));
                            beat_clock += 1.0/local_subdivision;
                            beats_remaining -= 1.0/local_subdivision;

                            let (accented, amp) = accent(beat_clock as u32);
                            notes.push((1.0 / local_subdivision, Some(vec![(Tone::StepFromChordTone(tone as usize, direction), amp)])));
                            beat_clock += 1.0/local_subdivision;
                            beats_remaining -= 1.0/local_subdivision;

                            let (accented, amp) = accent(beat_clock as u32);
                            notes.push((1.0 / local_subdivision, Some(vec![(Tone::AdjacentChordTone(tone as usize, direction), amp)])));
                            beat_clock += 1.0/local_subdivision;
                            beats_remaining -= 1.0/local_subdivision;

                        }
                    } else {
                        notes.push((1.0 / local_subdivision, None));
                        beat_clock += 1.0/local_subdivision;
                        beats_remaining -= 1.0/local_subdivision;
                    }
                }
                all_notes.push(notes);
            }
        }
        let mut final_tones = vec![];
        for (key, progression) in parts {
            for (_, _, _, chord) in progression {
                let measure = all_notes.remove(0);
                for (dur, note) in measure {
                    match note {
                        None => final_tones.push((dur, None)),
                        Some(tone_templates) => {
                            let mut tones = vec![];
                            for (tone, amp) in tone_templates {
                                let tone = match tone {
                                    Tone::ChordTone(idx) => {
                                        chord[(idx).min(chord.len()-1)] as i32 + octave*12
                                    },
                                    Tone::NonChordTone(degree) => {
                                        key.pitch(degree) as i32 + octave*12
                                    },
                                    Tone::StepFromChordTone(idx, direction) => {
                                        let mut tone = chord[idx.min(chord.len()-1)] as i32 + direction;
                                        while key.degree(tone as u32).is_none() { tone += direction; }
                                        tone
                                    },
                                    Tone::AdjacentChordTone(idx, direction) => {
                                        let idx = (idx).min(chord.len()-1);
                                        let mut tone = chord[idx] as i32 + direction;
                                        if direction < 0 || idx < chord.len() - 1 {
                                            while !chord.contains(&(tone as u32)) { tone += direction; }
                                        }
                                        tone
                                    },
                                };
                                tones.push((tone as u32, amp));
                            }
                            final_tones.push((dur, Some(tones)));
                        }
                    }
                }
            }
        }
        final_tones
    }

    pub type Voicing = Vec<Vec<(f64, Option<Vec<(u32, f64)>>)>>;

    pub fn new(parts: &plan::Parts, voice_plan: &[Voice], rng: &mut impl Rng) -> Voicing {
        let mut voices = vec![];

        for vp in voice_plan {
            voices.push(match vp {
                Voice::Harmony(t, o) => fill_harmony(parts, *t, *o, rng),
                Voice::Melody(o, d) => random_fill_melody(parts, *o, *d, rng),
            });
        }

        voices
    }
}

fn chirp<'a>(note: Note, buzzyness: f64) -> Value<'a, f64> {
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
    sig = RLPF::new(sig, 1800.0, buzzyness).into();

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

fn chorus<'a>(note: Note) -> Value<'a, f64> {
    let mut rng = rand::thread_rng();
    let detune = 0.015;
    let count = 5;

    let mut sig: Value<f64> = WaveTableSynth::sin(note.frequency).into();
    for _ in 0..count {
        sig = sig + WaveTableSynth::saw(note.frequency * rng.gen_range(1.0 - detune, 1.0 + detune));
    }
    let mut cutoff: Value<f64> = WaveTableSynth::sin(5.0).into();
    cutoff = 1500.0 + ((cutoff + 1.0) / 2.0) * 500.0;
    sig = RLPF::new(sig, cutoff, 0.5).into();
    let env: Value<f64> = ADSR::new().attack(0.1).sustain(1.0).duration(note.duration.as_secs_f64() - 0.1).release(0.1).curve(1.0).into();
    (sig / count as f64) * env * note.amplitude
}

pub fn main() {
    let seed = env::args().into_iter().nth(2).unwrap_or("42".to_string()).parse::<u64>().unwrap();
    let mut rng = ChaChaRng::seed_from_u64(seed);
    let voices = voicing::new(&plan::new(&mut rng), &[
        voicing::Voice::Melody(0, 0.9),
        if rng.gen::<f64>() > 0.5 {
            voicing::Voice::Harmony(voicing::HarmonyType::ArpeggiatedChord(rng.gen_range(0, 3)), -2)
        } else {
            voicing::Voice::Harmony(voicing::HarmonyType::RepeatedRoot, -2)
        },
        voicing::Voice::Harmony(voicing::HarmonyType::Chord, rng.gen_range(-1, 1)),
    ], &mut rng);

    let bpm = rng.gen_range(130.0, 195.0);
    let beat = 60000.0/bpm;
    let target_len = env::args().into_iter().nth(1).unwrap_or("10".to_string()).parse::<usize>().unwrap();

    let swing = 0.0;
    let mut beat_clock = 0.0;
    let melody_string = rng.gen_range(0, 3);
    let buzzyness = rng.gen_range(0.05, 2.0);
    let melody_voice: Value<f64> = sequence_from_iterator(
        (&voices[0]).into_iter()
        .map(move |(num_beats, note)| { 
            if let Some(notes) = note {
                let mut sig: Value<f64> = 0.0.into();
                let beat = if beat_clock % 0.5 == 0.0 {
                    beat * (1.0+swing)
                } else {
                    beat * swing
                };
                beat_clock += num_beats;
                for (tone, amp) in notes {
                    let note = Note {
                        duration: Duration::from_millis((num_beats * beat) as u64),
                        frequency: (tone ).frequency_from_midi() as f64,
                        amplitude: *amp,
                    };
                    if melody_string == 0 {
                        let mut pluck: Value<f64> = PluckedString::new(note.frequency / 2.0, 0.6).into();
                        pluck = RLPF::new(pluck, 2000.0, 0.15).into();
                        sig = sig + pluck;
                    } else if melody_string == 1 {
                        sig = sig + chorus(note);
                    } else if melody_string == 2 {
                        sig = sig + chirp(note, buzzyness);
                    }
                }
                (Duration::from_millis((num_beats * beat) as u64), sig)
            } else {
                (Duration::from_millis((num_beats * beat) as u64), 0.0.into())
            }
        })).into();

    let mut beat_clock = 0.0;
    let buzzyness = rng.gen_range(0.05, 2.0);
    let bass_string = rng.gen_range(0, 3);
    let bass: Value<f64> = sequence_from_iterator(
        (&voices[1]).into_iter()
        .map(move |(num_beats, note)| { 
            if let Some(notes) = note {
                let mut sig: Value<f64> = 0.0.into();
                let beat = if beat_clock % 0.5 == 0.0 {
                    beat * (1.0+swing)
                } else {
                    beat * swing
                };
                beat_clock += num_beats;
                for (tone, amp) in notes {
                    let note = Note {
                        duration: Duration::from_millis((num_beats * beat) as u64),
                        frequency: (tone ).frequency_from_midi() as f64,
                        amplitude: *amp,
                    };
                    if bass_string == 0 {
                        let mut pluck: Value<f64> = PluckedString::new(note.frequency, 0.15).into();
                        pluck = RLPF::new(pluck, 2000.0, 0.25).into();
                        sig = sig + pluck;
                    } else if bass_string == 1 {
                        sig = sig + chirp(note, buzzyness);
                    } else if bass_string == 2 {
                        sig = sig + chorus(note);
                    }
                }
                (Duration::from_millis((num_beats * beat) as u64), sig)
            } else {
                (Duration::from_millis((num_beats * beat) as u64), 0.0.into())
            }
        })).into();

    let pads: Value<f64> = sequence_from_iterator(
        (&voices[2]).into_iter()
        .map(move |(num_beats, note)| { 
            if let Some(notes) = note {
                let mut sig: Value<f64> = 0.0.into();
                for (tone, amp) in notes {
                    let note = Note {
                        duration: Duration::from_millis((num_beats * beat) as u64),
                        frequency: (tone ).frequency_from_midi() as f64,
                        amplitude: *amp,
                    };
                    sig = sig + pad(note);
                }
                (Duration::from_millis((num_beats * beat) as u64), sig)
            } else {
                (Duration::from_millis((num_beats * beat) as u64), 0.0.into())
            }
        })).into();

    let pad_mix = rng.gen_range(0.0, 0.8);
    let mut sig = ((melody_voice ) * 1.0 + (bass * 1.0 + pads * pad_mix) * 0.4) * 1.0;

    sig = Reverb::new(sig, 0.8, 0.1, 1000.0, 3.8).into();

    //sig = old_timeify(sig, 1.5);

    
    let mut env = Env::new(44100);
    let chunk_size = 1024;
    let total_samples = env.sample_rate as usize*target_len;
    for _ in 0..total_samples / chunk_size {
        let mut buffer_left = vec![0.0; chunk_size];
        sig.fill_buffer(&mut env, &mut buffer_left, 0, chunk_size);
        env.time += Duration::from_millis((chunk_size * 1000) as u64 / env.sample_rate as u64);
        let amp = 0.25;
        for left in buffer_left {
            io::stdout().write_f32::<LittleEndian>(left as f32 * amp).unwrap();    
            io::stdout().write_f32::<LittleEndian>(left as f32 * amp).unwrap();
        }
    }
}
