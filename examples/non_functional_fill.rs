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

fn fill_chords(composition: &mut Composition<Message, Voice>, key: &Scale, beats: u32, rng: &mut impl Rng) {
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
        let mut state = 4;
		let octave: i32 = 0;

        composition.add_annotation(0.0, beats as f64, Message::Key(key.clone()));
        for measure in 0..beats/4 {
            let next_states = chord_map[state].1.clone();
            state = *next_states.choose(rng).unwrap();
            let degree = chord_map[state].0;
            let chord = key.triad(degree + octave * 7);
            composition.add_annotation(measure as f64*4.0, 4.0, Message::Chord(chord));
        }
}

fn choose_idea(composition: &mut Composition<Message, Voice>, rng: &mut impl Rng) -> Vec<Option<i32>> {
    let innovation_prob = 0.2;
    let variation_prob = 0.4;
    let density = 0.8;
    let change_direction_prob = 0.2;
    let jumps = [0, 1, 1, 2, 3];

    if composition.ideas.len() == 0 || rng.gen::<f64>() < innovation_prob {
        let mode = rng.gen_range(0, 2);
        let mut direction = *[-1, 1].choose(rng).unwrap();
        let mut current = rng.gen_range(0, 12);
        let idea: Vec<Option<i32>> = (0..8).map(|_| {
            if rng.gen::<f64>() <= density {
                if mode == 0 {
                    if rng.gen::<f64>() < change_direction_prob {
                        direction *= -1;
                    }
                    let jump = *jumps.choose(rng).unwrap();
                    current = (current + jump*direction);
                    if current < -12 {
                        direction *= -1;
                        current += jump*direction*2;
                    }
                    if current > 13 {
                        direction *= -1;
                        current += jump*direction*2;
                    }
                } else {
                    current = rng.gen_range(0, 4) * direction;
                    direction *= -1;
                }
                Some(current)
            } else {
                None
            }
        }).collect();
        composition.ideas.push((1.0, idea.clone()));
        idea
    } else {
        let idxs: Vec<_> = composition.ideas.iter().map(|(t, _)| *t).enumerate().collect();
        let (idx, _) = idxs.choose_weighted(rng, |(_, t)| 1.0 / t).unwrap();
        composition.ideas[*idx].0 += 1.0;
        let fade = 1.0 / composition.ideas.len() as f64;
        for idea in &mut composition.ideas {
            idea.0 = (idea.0 - fade).max(0.01);
        }
        let mut idea = composition.ideas[*idx].1.clone();
        if rng.gen::<f64>() < variation_prob {
            let mut idx = 0;
            let mut jump = 0;
            for i in 0..idea.len() {
                if let Some(j) = idea[i] {
                    if j.abs() > jump {
                        idx = i;
                        jump = j.abs();
                    }
                }
            }
            if let Some(j) = idea[idx] {
                idea[idx].replace(j * -1);
                composition.ideas.push((1.0, idea.clone()));
            }
        }
        idea
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

fn fill_from_chords(composition: &mut Composition<Message, Voice>, beats: u32, rng: &mut impl Rng) -> Option<usize> {
    let density = 1.0;
    let decoration_prob = 0.0;
    let drone_prob = 0.8;
    let passing_note_prob = 0.5;
    let anticipation_prob = 0.5;

    let tone_map: Vec<(Option<i32>, Vec<usize>)> = (0..10).map(|i| (
            if rng.gen::<f64>() < density { Some(rng.gen_range(-5, 6)) } else { None },
            (0..rng.gen_range(1, 10)).chain(vec![(i+1)%10]).collect(),
    )).collect();
    let mut state = 0;

    let mut voice = vec![];
    let pattern = [3, 1, 2, 2];
    let pattern = [2; 4];
    let mut len = 0.0;
    let mut i = 0;
    let mut octave = 0;
    let mut current = 0;
    let mut previous = None;
    while len <= beats as f64 {
        let mut chord = None;
        let mut key = None;
        let beat = pattern[i % pattern.len()] as f64 * (1.0/4.0);
        for annotation in composition.annotations(len as f64, 0.0) {
            match annotation {
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
                if rng.gen::<f64>() < decoration_prob {
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
                    if rng.gen::<f64>() < drone_prob {
                        let amp = accent(len);
                        composition.add_annotation(len, beat / 2.0, Message::Note(tone as u32));
                        voice.push((beat / 2.0, vec![(tone as u32, amp)]));
                        len += beat / 2.0;
                        let amp = accent(len);
                        composition.add_annotation(len, beat / 2.0, Message::Note(67));
                        voice.push((beat / 2.0, vec![(67, amp*0.5)]));
                        len += beat / 2.0;
                    } else {
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

fn rythm_fill(composition: &mut Composition<Message, Voice>, beats: u32, rng: &mut impl Rng) -> Option<usize> {
    let pattern = [3, 1, 2, 2];
    let mut voice = vec![];
    let mut len = 0.0;
    let mut i = 0;
    while len <= beats as f64 {
        let amp = accent(len);
        let beat = pattern[i % pattern.len()] as f64 * (1.0/4.0);
        voice.push((beat, vec![(0, amp)]));
        len += beat;
        i += 1;
    }
    Some(composition.add_voice(voice))
}

fn bass_fill(composition: &mut Composition<Message, Voice>, beats: u32, rng: &mut impl Rng) -> Option<usize> {
    let pattern = [3, 1, 2, 2];
    let mut voice = vec![];
    let mut len = 0.0;
    let mut i = 0;
    let mut old_chord = None;
    while len <= beats as f64 {
        let mut chord = None;
        for annotation in composition.annotations(len as f64, 0.0) {
            match annotation {
                Message::Chord(c) => chord = Some(c.clone()),
                _ => (),
            }
        }
        if chord != old_chord {
            i = 0;
        }
        old_chord = chord.clone();
        let chord = chord.unwrap();
        let amp = accent(len);
        let beat = pattern[i % pattern.len()] as f64 * (1.0/4.0);
        voice.push((beat, vec![(chord[i%chord.len()], amp)]));
        len += beat;
        i += 1;
    }
    Some(composition.add_voice(voice))
}

fn non_functional_fill(composition: &mut Composition<Message, Voice>, beats: u32, pattern_len: usize, rng: &mut impl Rng) -> Option<usize> {
    let mut voice = vec![];
    let octave: i32 = 0;//rng.gen_range(-1, 3);
    let subdivision = 1;//*[1, 2].choose(rng).unwrap();
    let beat = 1.0/2.0;
    let density = 0.3;
    let pattern: Vec<_> = (0..pattern_len).map(|_| {
        if rng.gen::<f64>() < density {
            1.0
        } else {
            0.0
        }
    }).collect();

    let mut current_beat = 0.0;
    for i in 0..(beats as f64 / beat) as usize {
        let amp = pattern[i % pattern.len()];
        if amp != 0.0 {
            let mut existing_notes = vec![];
            for a in composition.annotations(current_beat - 0.5, beat + 0.5) {
                match a {
                    Message::Note(a) => existing_notes.push(a),
                    _ => (),
                }
            }

            let mut note = (60 + rng.gen_range(0, 12) + octave*12) as u32;
            let mut tries = 10;
            while !existing_notes.iter().all(|a| consonance(**a, note)) {
                note = ((60 + rng.gen_range(0, 12)) + octave*12) as u32;
                if tries == 0 {
                    return None;
                }
                tries -= 1;
            }
            voice.push((beat, vec![(note, amp)]));
            composition.add_annotation(current_beat, 1.0, Message::Note(note));
            current_beat += beat;
        } else {
            voice.push((beat, vec![]));
            current_beat += beat;
        }
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

    let mut composition = Composition::new();
    let mut voices = vec![];
    loop {
		fill_chords(&mut composition, &Scale::major(60), target_beats, &mut rng);
        let mut success = true;
        for _ in 0..1 {
            let voice = fill_from_chords(&mut composition, target_beats, &mut rng);
            if let Some(idx) = voice {
                voices.push(idx);
            } else {
                success = false;
                continue;
            }
        }
        if success {
            break;
        } else {
            composition = Composition::new();
            voices.clear();
        }
    }
    let rythm = rythm_fill(&mut composition, target_beats, &mut rng).unwrap();
    let bass = bass_fill(&mut composition, target_beats, &mut rng).unwrap();


    let banjo = SampleSet::from_directory(
        &"samples/banjo",
        &Regex::new(r".*/banjo_(?P<note>[A-G]s?)(?P<octave>[0-9])_very-long_forte_normal_truncated.mp3").unwrap()
    );
    let play_banjo = |note: &Note| {
        banjo.play(note.frequency).unwrap() * note.amplitude
    };
    let trumpet = SampleSet::from_directory(
        &"samples/trumpet",
        &Regex::new(r".*/trumpet_(?P<note>[A-G]s?)(?P<octave>[0-9])_025_forte_normal_truncated.mp3").unwrap()
    );
    let play_trumpet = |note: &Note| {
        trumpet.play(note.frequency * 2.0).unwrap() * note.amplitude
    };
    let double_bass = SampleSet::from_directory(
        &"samples/double_bass",
        &Regex::new(r".*/double-bass_(?P<note>[A-G][s#]?)(?P<octave>[0-9])_025_piano_pizz-normal_truncated.mp3").unwrap()
    );
    let play_double_bass = |note: &Note| {
        double_bass.play(note.frequency/4.0).unwrap() * note.amplitude
    };
    let wood_block = SampleSet::from_file("samples/percussion/woodblock/woodblock__025_mezzo-forte_struck-singly.mp3", 0.0);
    let play_wood_block = |note: &Note| {
        wood_block.play(note.frequency).unwrap() * note.amplitude
    };
    let snare_drum = SampleSet::from_file("samples/snare_drum/snare-drum__025_mezzo-forte_with-snares.mp3", 0.0);
    let play_snare_drum = |note: &Note| {
        snare_drum.play(note.frequency).unwrap() * note.amplitude
    };
    let cymbals = SampleSet::from_file("samples/percussion/clash cymbals/clash-cymbals__025_mezzo-forte_undamped.mp3", 0.0);
    let play_cymbals = |note: &Note| {
        cymbals.play(note.frequency).unwrap() * note.amplitude
    };
    let washboard = SampleSet::from_file("samples/percussion/washboard/washboard__05_forte_scraped.mp3", 0.0);
    let play_washboard = |note: &Note| {
        washboard.play(note.frequency).unwrap() * note.amplitude
    };
    let mut sig: Value<f64> = render_voice(composition.voice(voices[0]).unwrap(), &play_banjo, beat) * 0.5;
    //sig = sig + render_voice(composition.voice(bass).unwrap(), &play_double_bass, beat) * 1.0;
    sig = sig + render_voice(composition.voice(rythm).unwrap(), &play_washboard, beat) * 0.3;
    //sig = sig + render_voice(composition.voice(voices[1]).unwrap(), &play_banjo, beat);
    //sig = sig + render_voice(composition.voice(voices[2]).unwrap(), &play_banjo, beat);
    
    sig = Reverb::new(sig, 0.9, 0.1, 1000.0, 2.0).into();

    
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
