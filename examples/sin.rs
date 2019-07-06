use music::{
    Env,
    value::{Value, ValueNode,},
    ugen::WaveTableSynth,
    envelope::Linear,
    filter::*,
};
use byteorder::{WriteBytesExt, LittleEndian};
use std::io::{self};

fn main() {
    let freq: Value<f64> = 260.6.into();
    let osc1 = WaveTableSynth::saw(freq).to_value();
    let freq: Value<f64> = 262.4.into();
    let osc2 = WaveTableSynth::saw(freq).to_value();
    let freq: Value<f64> = 130.3.into();
    let osc3 = WaveTableSynth::sin(freq).to_value();
    let ffreq: Value<f64> = 1800.0.into();
    //let ffreq = ffreq - WaveTableSynth::sin(15.5.into()).to_value() * 1050.0.into();
    let ffreq_env = Linear::new(0.01, 1.0, 0.01, 1.0, 0.07).to_value();
    let env = Linear::new(0.01, 1.0, 0.01, 1.0, 0.07).to_value();
    let fq: Value<f64> = 0.5.into();
    let mut sig = Value(Box::new(RLPF::low_pass(osc1+osc2+osc3, ffreq*ffreq_env + 80.0.into(), fq))) * env;
    //let mut sig = BiQuad::low_pass(Value(Box::new(BiQuad::low_pass(osc1+osc2+osc3, 1800.0, 1.34119610))), 1800.0, 0.5065630);
    let env = Env::new(44100);
    let len = 10;
    let mut buffer = vec![0.0; env.sample_rate as usize*len];
    sig.fill_buffer(&env, &mut buffer, 0, env.sample_rate as usize*len);
    for sample in buffer.iter() {
        io::stdout().write_f32::<LittleEndian>(*sample as f32 / 1.0).unwrap();
    }
}
