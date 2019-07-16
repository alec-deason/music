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

    fn smooth_progression(key: &Key, start_chord: &Chord, end_chord: &Chord, count: u32) -> ChordProgression {
        let mut rng = rand::thread_rng();
        let mut direction = *[-1, 1].choose(&mut rng).unwrap();
        let mut progression = vec![(4.0, 1.0, start_chord.clone())];
        let (co, current) = key.degree(start_chord[0]).unwrap();
        let mut current = current as i32 + co * 7;
        while progression.len() < count as usize - 1 {
            current += direction * *[1, 1, 1, 1, 2, 3].choose(&mut rng).unwrap();
            if rng.gen::<f64>() > 0.6 {
                direction *= -1;
            }
            progression.push((4.0, 1.0, key.triad(current, key.scale_type())));
        }
        progression.push((4.0, 1.0, end_chord.clone()));
        progression
    }

    pub fn new() -> Parts {
        let mut rng = rand::thread_rng();
        let mut parts = vec![];
        let mut last_end = 0;
        for i in 0..rng.gen_range(3, 5) {
            let pattern = *[Pattern::Major, Pattern::Minor].choose(&mut rng).unwrap();
            let root = 69 + rng.gen_range(0, 12) + rng.gen_range(-1, 2) * 12;
            let key = Scale::new(pattern, root as u32);
            let start = if i == 0 {
                0
            } else {
                last_end + rng.gen_range(-3, 3)
            };
            let end = start + rng.gen_range(-3, 3);
            let progression = smooth_progression(&key, &key.triad(start, pattern), &key.triad(end, pattern), 12);
            last_end = end;
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
        Melody(i32),
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

    fn random_fill_melody(parts: &plan::Parts, octave: i32) -> Vec<(f64, Option<Vec<(u32, f64)>>)> {
        let mut rng = rand::thread_rng();
        let density = 0.7;
        let subdivision = 1.0;
        let mut beat_clock = 0.0;
        let mut notes = vec![];
        for (key, progression) in parts {
            for (dur, inten, chord) in progression {
                for _ in 0..(*dur * subdivision) as u32 {
                    let note = if rng.gen::<f64>() < density {
                        let (accented, amp) = accent(beat_clock as u32);
                        Some(vec![((*chord.choose(&mut rng).unwrap() as i32 + octave*12) as u32, amp)])
                    } else {
                        None
                    };
                    notes.push((1.0/subdivision, note));
                    beat_clock += subdivision;
                }
            }
        }
        notes
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
                Voice::Melody(o) => random_fill_melody(parts, *o),
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
        .release(0.3).into();

    pluck * env * note.amplitude
}

pub fn main() {
    let mut rng = rand::thread_rng();
    let voices = voicing::new(&plan::new(), &[
        voicing::Voice::Melody(0),
        if rng.gen::<f64>() > 0.5 {
            voicing::Voice::Harmony(voicing::HarmonyType::ArpeggiatedChord, -2)
        } else {
            voicing::Voice::Harmony(voicing::HarmonyType::RepeatedRoot, -2)
        },
        voicing::Voice::Harmony(voicing::HarmonyType::Chord, -1),
    ]);

    let bpm = 180.0;
    let beat = 60000.0/bpm;
    let target_len = env::args().into_iter().nth(1).unwrap_or("10".to_string()).parse::<usize>().unwrap();
    let target_beats = (target_len as f64 * 1000.0) / beat;

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
                    let mut pluck: Value<f64> = PluckedString::new(note.frequency).into();
                    let env: Value<f64> = ADSR::new()
                        .attack(0.02)
                        .decay(0.02)
                        .duration(note.duration.as_secs_f64())
                        .release(0.06).into();
                    pluck = pluck * env * note.amplitude;
                    sig = sig + pluck;
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

    let mut sig = melody_voice * 0.8 + (bass * 0.8 + pads * 0.4) * 0.8;
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
