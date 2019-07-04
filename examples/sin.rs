use music::{
    Env,
    value::{Value, ValueNode,},
    ugen::WaveTableSynth,
};
use byteorder::{WriteBytesExt, LittleEndian};
use std::io::{self};

fn main() {
    let lfo = (WaveTableSynth::sin(4.0.into()).to_value() + 1.0.into()) / 2.0.into();
    let mut synth: Value<f64> = WaveTableSynth::saw(Into::<Value<f64>>::into(440.0) * lfo).to_value();
    for i in 2..18 {
        let lfo = (WaveTableSynth::sin(4.0.into()).to_value() + 1.0.into()) / 2.0.into();
        let new_synth: Value<f64> = WaveTableSynth::square(Into::<Value<f64>>::into(440.0*i as f64 + i as f64) * lfo).to_value();
        let divisor: Value<f64> = Value(Box::new(i as f64));
        synth = synth + (new_synth / divisor);
    }
    let env = Env::new(44100);
    let len = 10;
    let mut buffer = vec![0.0; env.sample_rate as usize*len];
    synth.fill_buffer(&env, &mut buffer, 0, env.sample_rate as usize*len);
    for sample in buffer.iter() {
        io::stdout().write_f32::<LittleEndian>(*sample as f32 / 40.0).unwrap();
    }
}
