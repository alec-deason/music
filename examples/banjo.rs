#![feature(duration_float)]

use std::collections::HashSet;
use std::time::Duration;
use std::env;
use rand::Rng;
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use byteorder::{WriteBytesExt, LittleEndian};
use std::io::{self};
use regex::Regex;


use music::{
    Env,
    value::*,
    oscillator::*,
    oscillator::string::*,
    oscillator::sampler::*,
    envelope::*,
    filter::*,
    effect::*,
    sequence::*,
    note::*,  
    composition::*,
};


fn chirp<'a>(note: &Note) -> Value<'a, f64> {
    let buzzyness = 0.5;
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
             
fn pad<'a>(note: &Note) -> Value<'a, f64> {
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

fn chorus<'a>(note: &Note) -> Value<'a, f64> {
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

fn beep<'a>(note: &Note) -> Value<'a, f64> {
    let mut sig: Value<f64> = WaveTableSynth::sin(note.frequency).into();
    let env: Value<f64> = ADSR::new().attack(0.1).sustain(1.0).duration(note.duration.as_secs_f64() - 0.1).release(0.1).curve(1.0).into();
    sig * env * note.amplitude
}

fn consonance(a: u32, b: u32) -> bool {
    let a = (a as f64).frequency_from_midi();
    let b = (b as f64).frequency_from_midi();

    let small_number_limit = 9;
    let acceptable_ratios: Vec<f64> = (1..small_number_limit).map(|a| (1..small_number_limit).map(|b| a as f64 / b as f64).collect::<Vec<f64>>()).flatten().collect();
    
    let fudge = 1.0/40.0;

    let ratio = a / b;

    acceptable_ratios.iter().any(|r| (r - ratio).abs() < fudge)
}

fn fill_chords(composition: &mut Composition<Message>, key: &Scale, beats: u32, rng: &mut impl Rng) {
    let chord_map: Vec<(i32, Vec<(usize, f64)>)> = vec![
            (2, vec![1, 5]),
            (3, vec![2, 6]),
            (4, vec![3, 4, 7]),
            (5, vec![4]),
            (1, vec![0, 1, 2, 3, 4, 5, 6, 7]),
            (5, vec![1, 6]),
            (6, vec![2, 7]),
            (2, vec![3]),
        ].iter().map(|(s, ns)| (*s, ns.iter().map(|n| (*n, rng.gen_range(0.0001, 1.0))).collect::<Vec<(usize, f64)>>())).collect();
        let mut state = 4;
		let octave: i32 = 0;

        composition.add_annotation(0.0, beats as f64, Message::Key(key.clone()));
        for measure in 0..beats/4 {
            let next_states = chord_map[state].1.clone();
            state = next_states.choose_weighted(rng, |(_, w)| *w).unwrap().0;
            let degree = chord_map[state].0;
            let chord = key.triad(degree + octave * 7);
            composition.add_annotation(measure as f64*4.0, 4.0, Message::Chord(chord));
        }
}

fn accent(beat_clock: f64) -> f64 {
    let beats_per_measure = 2;
    let accented_beats = [0, 1];
    let beat = beat_clock.floor() as u32 % beats_per_measure;
    for b in &accented_beats {
        if beat == *b {
            return 1.0;
        }
    }
    0.8
}

fn roll(chord: &Vec<u32>, rng: &mut impl Rng) -> Vec<(u32, f64)> {
    [
            // Mixed
            vec![
                (chord[0], 1.0),
                (chord[1], 0.8),
                (chord[2] + 3, 0.8),
                (chord[2], 1.0),
                (chord[0] - 3, 0.8),
                (chord[1], 0.8),
                (chord[2] + 3, 1.0),
                (chord[2], 0.8),
            ],
            // Forward-Backward
            vec![
                (chord[0], 1.0),
                (chord[1], 0.8),
                (chord[2], 0.8),
                (chord[2] + 3, 1.0),
                (chord[2], 0.8),
                (chord[1], 0.8),
                (chord[0], 1.0),
                (chord[2], 0.8),
            ],
            // Backward
            vec![
                (chord[2], 1.0),
                (chord[1], 0.8),
                (chord[0], 0.8),
                (chord[2], 1.0),
                (chord[1], 0.8),
                (chord[0], 0.8),
                (chord[2], 1.0),
                (chord[1], 0.8),
            ],
            // Forward
            vec![
                (chord[0], 1.0),
                (chord[1], 0.8),
                (chord[2], 0.8),
                (chord[0], 1.0),
                (chord[1], 0.8),
                (chord[2], 0.8),
                (chord[0], 1.0),
                (chord[1], 0.8),
            ],
    ].choose(rng).unwrap().clone()
}

fn brownian_jiggle(state: f64, min: f64, max: f64, rng: &mut impl Rng) -> f64 {
    let mut state = state + rng.gen_range(-0.1, 0.1);
    state = state.max(min).min(max);
    state
}

fn fill_from_chords(composition: &mut Composition<Message>, beats: u32, rng: &mut impl Rng) -> Option<usize> {
    let density = 1.0;
    let decoration_prob = 0.6;
    let mut decoration_prob_jiggle = rng.gen_range(0.0, 1.0);
    let long_note_prob = 0.0;
    let drone_prob = 0.4;
    let mut drone_prob_jiggle = rng.gen_range(0.0, 1.0);
    let roll_prob = 0.4;
    let mut roll_prob_jiggle = rng.gen_range(0.0, 1.0);
    let passing_note_prob = 0.5;
    let anticipation_prob = 0.3;

    let tone_map: Vec<(Option<i32>, Vec<usize>)> = (0..10).map(|i| (
            if rng.gen::<f64>() < density { Some(rng.gen_range(-5, 6)) } else { None },
            (0..rng.gen_range(1, 10)).chain(vec![(i+1)%10]).collect(),
    )).collect();
    let mut state = 0;

    let mut voice = vec![];
    let pattern = [2; 4];
    let mut len = 0.0;
    let mut i = 0;
    let mut octave = 0;
    let mut current = 0;
    let mut previous = None;
    while len <= beats as f64 {
        decoration_prob_jiggle = brownian_jiggle(decoration_prob_jiggle, 0.0, 2.0, rng);
        roll_prob_jiggle = brownian_jiggle(roll_prob_jiggle, 0.6, 2.0, rng);
        drone_prob_jiggle = brownian_jiggle(drone_prob_jiggle, 0.6, 2.0, rng);
        let mut chord = None;
        let mut key = None;
        let beat = pattern[i % pattern.len()] as f64 * (1.0/4.0);
        for annotation in composition.annotations(len as f64, 0.0) {
            match &annotation.message {
                Message::Chord(c) => chord = Some(c.clone()),
                Message::Key(k) => key = Some(k.clone()),
                _ => (),
            }
        }
        let key = key.unwrap();
        if chord.is_some() {
            let chord = chord.unwrap();
            state = *tone_map[state].1.choose(rng).unwrap();
            let jump = tone_map[state].0;
            if let Some(mut jump) = jump {
                current += jump;
                while current < 0 {
                    current += chord.len() as i32;
                    octave -= 1;
                }
                while current >= chord.len() as i32 {
                    current -= chord.len() as i32;
                    octave += 1;
                }
                octave = 0;
                let tone = (chord[current as usize] as i32 + octave*12) as u32;
                let mut did_decorate = false;
                if rng.gen::<f64>() < decoration_prob * decoration_prob_jiggle {
                    if previous.is_some() && rng.gen::<f64>() < passing_note_prob {
                        let direction = if previous.unwrap() < tone { 1 } else { -1 };
                        let mut passing = previous.unwrap() as i32 + direction;
                        while key.degree(passing as u32).is_none() { passing += direction; }
                        
                        let amp = accent(len);
                        composition.add_annotation(len, beat / 2.0, Message::Note(passing as u32));
                        voice.push((beat / 2.0, vec![(passing as u32, amp)]));
                        len += beat / 2.0;
                        let amp = accent(len);
                        composition.add_annotation(len, beat / 2.0, Message::Note(tone as u32));
                        voice.push((beat / 2.0, vec![(tone as u32, amp)]));
                        len += beat / 2.0;
                        did_decorate = true;
                    } else if rng.gen::<f64>() < anticipation_prob {
                        let amp = accent(len);
                        composition.add_annotation(len, beat / 2.0, Message::Note(tone as u32));
                        voice.push((beat / 2.0, vec![(tone as u32, amp)]));
                        len += beat / 2.0;
                        let amp = accent(len);
                        composition.add_annotation(len, beat / 2.0, Message::Note(tone as u32));
                        voice.push((beat / 2.0, vec![(tone as u32, amp)]));
                        len += beat / 2.0;
                        did_decorate = true;
                    }
                }
                if !did_decorate {
                    if (len * beat) % 2.0 == 0.0 && rng.gen::<f64>() < roll_prob * roll_prob_jiggle {
                        let roll = roll(&chord, rng);
                        for (tone, amp) in roll {
                            composition.add_annotation(len, beat / 2.0, Message::Note(tone as u32));
                            voice.push((beat / 2.0, vec![(tone as u32, amp)]));
                            len += beat / 2.0;
                        }
                    } else if rng.gen::<f64>() < drone_prob * drone_prob_jiggle {
                        let amp = accent(len);
                        composition.add_annotation(len, beat, Message::Note(tone as u32));
                        voice.push((beat, vec![(tone as u32, amp)]));
                        len += beat;

                        let amp = accent(len);
                        composition.add_annotation(len, beat / 2.0, Message::Note(67));
                        voice.push((beat / 2.0, chord.iter().map(|t| (*t, amp)).collect()));
                        len += beat / 2.0;

                        let amp = accent(len);
                        composition.add_annotation(len, beat / 2.0, Message::Note(67));
                        voice.push((beat / 2.0, vec![(67, amp*0.5)]));
                        len += beat / 2.0;
                    } else {
                        let beat = if (len * beat) % 1.0 == 0.0 && rng.gen::<f64>() < long_note_prob {
                            beat * 2.0
                        } else {
                            beat
                        };
                        let amp = accent(len);
                        composition.add_annotation(len, beat, Message::Note(tone as u32));
                        voice.push((beat, vec![(tone as u32, amp)]));
                        len += beat;
                    }
                }
                previous = Some(tone as u32);
            } else {
                voice.push((beat, vec![]));
                previous = None;
                len += beat;
            }
        } else {
            voice.push((beat, vec![]));
            previous = None;
            len += beat;
        }
        i += 1;
    }
    Some(composition.add_voice(voice))
}

fn fill_from_accompanyment(composition: &mut Composition<Message>, beats: u32, rng: &mut impl Rng) -> Option<usize> {
    let density = 0.8;
    let double_prob = 0.8;

    let mut voice = vec![];
    let pattern = [2; 4];
    let mut len = 0.0;
    let mut i = 0;
    while len <= beats as f64 {
        let mut chord = None;
        let mut key = None;
        let mut notes = vec![];
        let beat = pattern[i % pattern.len()] as f64 * (1.0/ if rng.gen::<f64>() < double_prob { 8.0 } else { 4.0 });
        for annotation in composition.annotations(len as f64, 0.0) {
            match &annotation.message {
                Message::Chord(c) => chord = Some(c.clone()),
                Message::Note(c) => notes.push(c.clone()),
                Message::Key(k) => key = Some(k.clone()),
                _ => (),
            }
        }
        let key = key.unwrap();
        let mut degrees: Vec<_> = (0..7).collect();
        degrees.shuffle(rng);
        let amp = accent(len);
        let mut success = false;
        if rng.gen::<f64>() < density {
            if notes.len() > 0 {
                for degree in degrees {
                    if consonance(key.pitch(degree), notes[0]) {
                        voice.push((beat, vec![(key.pitch(degree), amp)]));
                        success = true;
                        break;
                    }
                }
            }
        }
        if ! success {
            voice.push((beat, vec![]));
        }
        len += beat;
        i += 1;
    }
    Some(composition.add_voice(voice))
}

fn silence_fill(composition: &mut Composition<Message>, beats: u32) -> Option<usize> {
    Some(composition.add_voice(vec![(beats as f64, vec![])]))
}

fn bass_fill(composition: &mut Composition<Message>, beats: u32, rng: &mut impl Rng) -> Option<usize> {
    let pattern = [2; 4];
    let mut voice = vec![];
    let mut len = 0.0;
    let mut i = 0i32;
    let mut dir = 1;
    let mut mode = false;
    let mode_switch_prob = 0.05;
    let mut note = 0;
    while len <= beats as f64 {
        let mut chord = None;
        for annotation in composition.annotations(len as f64, 0.0) {
            match &annotation.message {
                Message::Chord(c) => chord = Some(c.clone()),
                _ => (),
            }
        }
        let chord = chord.unwrap();
        let amp = accent(len);
        let beat = pattern[i as usize % pattern.len()] as f64 * (1.0/4.0);

        if mode {
            voice.push((beat, vec![(chord[i as usize %chord.len()], amp)]));
        } else {
            let idx = if note % 2 == 0 {
                0
            } else {
                chord.len() - 1
            };
            voice.push((beat, vec![(chord[idx], amp)]));
        }
        if rng.gen::<f64>() < mode_switch_prob {
            mode = !mode;
        }
        len += beat;
        if i >= chord.len() as i32 {
            dir = -1;
        } else if i <= 0 {
            dir = 1;
        }
        i += dir;
        note += 1;
    }
    Some(composition.add_voice(voice))
}

fn render_voice<'a>(voice: &Voice, instrument: &'a Fn(&Note) -> Value<'a, f64>, beat: f64) -> Value<'a, f64> {
    sequence_from_iterator(
        voice.clone().into_iter()
        .map(move |(num_beats, chord)| { 
            let mut sig: Value<f64> = 0.0.into();
            for (tone, amp) in chord {
                let note = Note {
                    duration: Duration::from_millis((num_beats * beat) as u64),
                    frequency: (tone ).frequency_from_midi() as f64,
                    amplitude: amp,
                };
                sig = sig + instrument(&note);
            }
            (Duration::from_millis((num_beats * beat) as u64), sig)
        })).into()
}

pub fn main() {
    let seed = env::args().into_iter().nth(2).unwrap_or("42".to_string()).parse::<u64>().unwrap();
    let mut rng = ChaChaRng::seed_from_u64(seed);

    let bpm = 110.0;
    let beat = 60000.0/bpm;
    let target_len = env::args().into_iter().nth(1).unwrap_or("10".to_string()).parse::<usize>().unwrap();
    let target_beats = (target_len as u32 * bpm as u32) / 60;

    let key = Scale::new(Pattern::Major, 65);

    let target_measures = target_beats / 4;
    let target_measures = target_measures.min(100);

    let mut intro = Composition::new();
    let section_beats = (target_measures / 10) * 4;
    fill_chords(&mut intro, &key, section_beats, &mut rng);
    let melody = fill_from_chords(&mut intro, section_beats, &mut rng).unwrap();
    let accompanyment = fill_from_accompanyment(&mut intro, section_beats, &mut rng).unwrap();
    let bass = bass_fill(&mut intro, section_beats, &mut rng).unwrap();

    let mut section_a = Composition::new();
    let section_beats = (target_measures / 5) * 4;
    fill_chords(&mut section_a, &key, section_beats, &mut rng);
    fill_from_chords(&mut section_a, section_beats, &mut rng).unwrap();
    fill_from_accompanyment(&mut section_a, section_beats, &mut rng).unwrap();
    bass_fill(&mut section_a, section_beats, &mut rng).unwrap();

    let mut section_b = Composition::new();
    let section_beats = (target_measures / 5) * 4;
    fill_chords(&mut section_b, &key, section_beats, &mut rng);
    silence_fill(&mut section_b, section_beats);
    fill_from_accompanyment(&mut section_b, section_beats, &mut rng).unwrap();
    fill_from_chords(&mut section_b, section_beats, &mut rng).unwrap();

    let mut section_c = Composition::new();
    let section_beats = (target_measures / 5) * 4;
    fill_chords(&mut section_c, &key, section_beats, &mut rng);
    fill_from_chords(&mut section_c, section_beats, &mut rng).unwrap();
    fill_from_accompanyment(&mut section_c, section_beats, &mut rng).unwrap();
    bass_fill(&mut section_c, section_beats, &mut rng).unwrap();

    let mut outro = Composition::new();
    let section_beats = (target_measures / 10) * 4;
    fill_chords(&mut outro, &key, section_beats, &mut rng);
    fill_from_chords(&mut outro, section_beats, &mut rng).unwrap();
    fill_from_accompanyment(&mut outro, section_beats, &mut rng).unwrap();
    bass_fill(&mut outro, section_beats, &mut rng).unwrap();

    let mut composition = intro.clone();
    composition.extend(&section_a);
    composition.extend(&section_b);
    composition.extend(&section_c);
    composition.extend(&section_a);
    composition.extend(&outro);


    let banjo = SampleSet::from_directory(
        &"samples/banjo",
        &Regex::new(r".*/banjo_(?P<note>[A-G]s?)(?P<octave>[0-9])_very-long_forte_normal_truncated.mp3").unwrap()
    );
    let play_banjo = |note: &Note| {
        banjo.play(note.frequency).unwrap() * note.amplitude
    };
    let trumpet = SampleSet::from_directory(
        &"samples/trumpet",
        &Regex::new(r".*/trumpet_(?P<note>[A-G]s?)(?P<octave>[0-9])_025_pianissimo_normal_truncated.mp3").unwrap()
    );
    let play_trumpet = |note: &Note| {
        trumpet.play(note.frequency ).unwrap() * note.amplitude
    };
    let double_bass = SampleSet::from_directory(
        &"samples/double_bass",
        &Regex::new(r".*/double-bass_(?P<note>[A-G][s#]?)(?P<octave>[0-9])_025_piano_pizz-normal_truncated.mp3").unwrap()
    );
    let play_double_bass = |note: &Note| {
        //double_bass.play(note.frequency/4.0).unwrap() * note.amplitude
        let mut pluck: Value<f64> = PluckedString::new(note.frequency / 8.0, 0.09).into();
        RLPF::new(pluck, 1000.0, 0.1).into()
    };
    let mut sig: Value<MultiSample<f64>> = hass_shift(
        render_voice(composition.voice(melody).unwrap(), &play_banjo, beat) * 1.0,
        0.3/1000.0,
    );
    //sig = sig + render_voice(composition.voice(accompanyment).unwrap(), &play_trumpet, beat) * 1.5;
    sig = sig + hass_shift(
          render_voice(composition.voice(bass).unwrap(), &play_double_bass, beat) * 0.7,
          -0.1/1000.0,
    );
    
    sig = Reverb::new(sig, 0.9, 0.1, 1000.0, 2.0).into();

    
    let mut env = Env::new(44100);
    let chunk_size = 1024;
    let total_samples = env.sample_rate as usize*target_len;
    for _ in 0..total_samples / chunk_size {
        let mut buffer_left = vec![MultiSample(0.0, 0.0); chunk_size];
        sig.fill_buffer(&mut env, &mut buffer_left, chunk_size);
        env.time += Duration::from_millis((chunk_size * 1000) as u64 / env.sample_rate as u64);
        let amp = 0.25;
        for sample in buffer_left {
            io::stdout().write_f32::<LittleEndian>(sample.0 as f32 * amp).unwrap();    
            io::stdout().write_f32::<LittleEndian>(sample.1 as f32 * amp).unwrap();
        }
    }
}
