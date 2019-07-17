#![feature(duration_float)]

use std::time::Duration;
use std::env;
use rand::Rng;
use rand::seq::SliceRandom;
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
    pub type Duration = f64;
    pub type ChordProgression = Vec<(Duration, Intensity, Chord)>;
    pub type SubPart = (Key, ChordProgression);

    fn smooth_progression(key: &Key, leadin_chord: &Chord, count: u32) -> ChordProgression {
        let mut rng = rand::thread_rng();
        let mut direction = *[-1, 1].choose(&mut rng).unwrap();
        let mut progression = vec![];
        let (co, current) = key.degree(leadin_chord[0]).unwrap();
        let mut current = current as i32 + co * 7;
        let mut since_resolution = 0.0;
        while progression.len() < count as usize  {
            if rng.gen::<f64>() < since_resolution / 8.0 {
                while current.abs() % 7 != 5 {
                    current += direction;
                }
                progression.push((4.0, 1.0, key.triad(current)));
                if direction > 0 {
                    current -= 4;
                } else {
                    current += 3;
                }
                direction *= -1;
                let mut chord = key.triad(current);
                if rng.gen::<f64>() > 0.6 {
                    chord.push(key.pitch(current+6));
                }
                progression.push((4.0, 1.0, chord));
                since_resolution = 0.0;
            } else {
                current = current + direction * *[1, 1, 1, 2, 2].choose(&mut rng).unwrap();
                if rng.gen::<f64>() > 0.8 {
                    direction *= -1;
                }
                progression.push((4.0, 1.0, key.triad(current)));
                since_resolution += 1.0;
            }
        }
        progression
    }

    pub fn new() -> Parts {
        let mut rng = rand::thread_rng();
        let mut parts = vec![];
        let pattern = *[Pattern::Major, Pattern::Minor].choose(&mut rng).unwrap();
        let root = 69 + 12 + rng.gen_range(-1, 1) * 12;
        let key = Scale::new(pattern, root as u32);
        let mut last_end = key.triad(0);
        for i in 0..10 {
            let progression = smooth_progression(&key, &last_end, 12);
            last_end = progression[progression.len()-1].2.clone();
            parts.push((key.clone(), progression));
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
        ArpeggiatedChord,
        RepeatedRoot,
    }

    fn fill_harmony(parts: &plan::Parts, t: HarmonyType, octave: i32) -> Vec<(f64, Option<Vec<(u32, f64)>>)> {
        let mut notes = vec![];
        let mut beat_clock = 0.0;
        for (key, progression) in parts {
            for (dur, inten, chord) in progression {
                match t {
                    HarmonyType::Chord => {
                        let (accented, amp) = accent(beat_clock as u32);
                        notes.push((*dur, Some(
                            chord.iter().map(|t| ((*t as i32 + octave*12) as u32, amp)).collect()
                        )));
                        beat_clock += dur;
                    },
                    HarmonyType::ArpeggiatedChord => {
                        for i in 0..*dur as usize {
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

    fn random_fill_melody(parts: &plan::Parts, octave: i32, density: f64) -> Vec<(f64, Option<Vec<(u32, f64)>>)> {
        let mut rng = rand::thread_rng();

        let passing_note_prob = 0.4;
        let keep_direction_prob = 0.8;
        let switch_direction_prob = 0.4;
        let repeat_measure_prob = 0.5;
        let repeat_distance_set = vec![1, 2, 3, 4];

        let mut direction: i32 = *[-1, 1].choose(&mut rng).unwrap();
        let mut last_chord_position = 0;
        let subdivision = 1.0;
        let mut beat_clock = 0.0;
        let mut all_notes: Vec<Vec<(f64, Option<Vec<(Tone, f64)>>)>> = vec![];
        for (key, progression) in parts {
            for (dur, inten, chord) in progression {
                if all_notes.len() > 0 && rng.gen::<f64>() < repeat_measure_prob {
                    let idx = *repeat_distance_set.choose(&mut rng).unwrap();
                    let idx = (all_notes.len() as i32 - idx).max(0);
                    all_notes.push(all_notes[idx as usize].clone());
                    continue
                }
                let mut notes = vec![];
                let mut beats_remaining = *dur * subdivision;
                while beats_remaining > 0.0 {
                    if rng.gen::<f64>() < switch_direction_prob {
                        direction *= -1;
                    }
                    if rng.gen::<f64>() < density {
                        if beats_remaining < 3.0/subdivision || rng.gen::<f64>() > passing_note_prob {
                            let (accented, amp) = accent(beat_clock as u32);
                            let tone = if rng.gen::<f64>() < keep_direction_prob {
                                last_chord_position = (last_chord_position + direction).min(chord.len() as i32 -1).max(0);
                                Tone::ChordTone(last_chord_position as usize)
                            } else {
                                last_chord_position = rng.gen_range(0, chord.len() as i32);
                                Tone::ChordTone(last_chord_position as usize)
                            };
                            notes.push((1.0 / subdivision, Some(vec![(tone, amp)])));
                            beat_clock += 1.0/subdivision;
                            beats_remaining -= 1.0/subdivision;
                        } else  {
                            //Passing tone
                            let direction = *[-1, 1].choose(&mut rng).unwrap();
                            let mut tone = if direction > 0 {
                                rng.gen_range(0, chord.len() as i32 -1)
                            } else {
                                rng.gen_range(1, chord.len() as i32)
                            };
                            let (accented, amp) = accent(beat_clock as u32);
                            notes.push((1.0 / subdivision, Some(vec![(Tone::ChordTone(tone as usize), amp)])));
                            beat_clock += 1.0/subdivision;
                            beats_remaining -= 1.0/subdivision;

                            let (accented, amp) = accent(beat_clock as u32);
                            notes.push((1.0 / subdivision, Some(vec![(Tone::StepFromChordTone(tone as usize, direction), amp)])));
                            beat_clock += 1.0/subdivision;
                            beats_remaining -= 1.0/subdivision;

                            let (accented, amp) = accent(beat_clock as u32);
                            notes.push((1.0 / subdivision, Some(vec![(Tone::AdjacentChordTone(tone as usize, direction), amp)])));
                            beat_clock += 1.0/subdivision;
                            beats_remaining -= 1.0/subdivision;

                        }
                    } else {
                        notes.push((1.0 / subdivision, None));
                        beat_clock += 1.0/subdivision;
                        beats_remaining -= 1.0/subdivision;
                    }
                }
                all_notes.push(notes);
            }
        }
        let mut final_tones = vec![];
        for (key, progression) in parts {
            for (_, _, chord) in progression {
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

    fn new_idea() {
    }
    fn structured_fill_melody(parts: &plan::Parts, octave: i32) -> Vec<(f64, Option<Vec<(u32, f64)>>)> {
        let mut rng = rand::thread_rng();
        let ideas: Vec<_> = (0..rng.gen_range(1, 3)).map(|_| (0..rng.gen_range(3, 6)).map(|_| (rng.gen_range(1, 8) as f64 / 4.0, rng.gen_range(-4, 4))).collect::<Vec<(f64, i32)>>()).collect();
        let mut current_idea = rng.gen_range(0, ideas.len());
        let mut idea_idx = 0;
        let mut last_note = parts[0].1[0].2[0];
        let mut notes = vec![];
        for (key, progression) in parts {
            for (dur, inten, chord) in progression {
                for _ in 0..*dur as u32 {
                    if idea_idx == ideas[current_idea].len() {
                        if rng.gen::<f64>() > 0.8 {
                            current_idea = rng.gen_range(0, ideas.len());
                        }
                        idea_idx = 0;
                        last_note = (*chord.choose(&mut rng).unwrap() as i32 + octave*12) as u32;
                        notes.push((1.0, Some(vec![(last_note, 1.0)])));
                    } else {
                        notes.push((1.0, Some(vec![((last_note as i32 + ideas[current_idea][idea_idx].1) as u32, 1.0)])));
                    }
                    idea_idx += 1;
                }
            }
        }
        notes
    }

    pub type Voicing = Vec<Vec<(f64, Option<Vec<(u32, f64)>>)>>;

    pub fn new(parts: &plan::Parts, voice_plan: &[Voice]) -> Voicing {
        let mut voices = vec![];

        for vp in voice_plan {
            voices.push(match vp {
                Voice::Harmony(t, o) => fill_harmony(parts, *t, *o),
                Voice::Melody(o, d) => random_fill_melody(parts, *o, *d),
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
    let mut rng = rand::thread_rng();
    let pluck: Value<f64> = DrivenString::new(note.frequency.into()).into();
    let env: Value<f64> = ADSR::new()
        .attack(0.4)
        .duration(note.duration.as_secs_f64() - 0.4)
        .release(1.5).into();

    pluck * env * note.amplitude
}

pub fn main() {
    let mut rng = rand::thread_rng();
    let voices = voicing::new(&plan::new(), &[
        voicing::Voice::Melody(0, 0.8),
        if rng.gen::<f64>() > 0.5 {
            voicing::Voice::Harmony(voicing::HarmonyType::ArpeggiatedChord, -2)
        } else {
            voicing::Voice::Harmony(voicing::HarmonyType::RepeatedRoot, -2)
        },
        voicing::Voice::Harmony(voicing::HarmonyType::Chord, -1),
        voicing::Voice::Melody(-1, 0.6),
    ]);

    let bpm = 180.0;
    let beat = 60000.0/bpm;
    let target_len = env::args().into_iter().nth(1).unwrap_or("10".to_string()).parse::<usize>().unwrap();

    eprintln!("{} {}", voices[0].iter().map(|(d, _)| d).sum::<f64>(), voices[1].iter().map(|(d, _)| d).sum::<f64>());
    let buzzyness = rng.gen_range(0.05, 2.0);
    let melody_voice: Value<f64> = sequence_from_iterator(
        (&voices[0]).into_iter()
        .map(move |(num_beats, note)| { 
            if let Some(notes) = note {
                let mut sig: Value<f64> = 0.0.into();
                for (tone, amp) in notes {
                    let note = Note {
                        duration: Duration::from_millis((num_beats * beat) as u64),
                        frequency: (tone ).frequency_from_midi() as f64,
                        amplitude: *amp,
                    };
                    sig = sig + chirp(note, buzzyness);
                }
                (Duration::from_millis((num_beats * beat) as u64), sig)
            } else {
                (Duration::from_millis((num_beats * beat) as u64), 0.0.into())
            }
        })).into();
    let buzzyness = rng.gen_range(0.05, 2.0);
    let melody_voice2: Value<f64> = sequence_from_iterator(
        (&voices[3]).into_iter()
        .map(move |(num_beats, note)| { 
            if let Some(notes) = note {
                let mut sig: Value<f64> = 0.0.into();
                for (tone, amp) in notes {
                    let note = Note {
                        duration: Duration::from_millis((num_beats * beat) as u64),
                        frequency: (tone ).frequency_from_midi() as f64,
                        amplitude: *amp,
                    };
                    sig = sig + chirp(note, buzzyness);
                }
                (Duration::from_millis((num_beats * beat) as u64), sig)
            } else {
                (Duration::from_millis((num_beats * beat) as u64), 0.0.into())
            }
        })).into();

    let buzzyness = rng.gen_range(0.05, 2.0);
    let bass: Value<f64> = sequence_from_iterator(
        (&voices[1]).into_iter()
        .map(move |(num_beats, note)| { 
            if let Some(notes) = note {
                let mut sig: Value<f64> = 0.0.into();
                for (tone, amp) in notes {
                    let note = Note {
                        duration: Duration::from_millis((num_beats * beat) as u64),
                        frequency: (tone ).frequency_from_midi() as f64,
                        amplitude: *amp,
                    };
                    sig = sig + chirp(note, buzzyness);
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

    let mut sig = ((melody_voice + melody_voice2 * 0.0) * 0.8 + (bass * 1.0 + pads * 1.0) * 0.0) * 1.0;
    sig = Reverb::new(sig, 0.8, 0.1, 1000.0, 4.8).into();
    
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
