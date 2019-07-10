pub const MINOR: [u32; 7] = [2, 1, 2, 2, 1, 2, 2];
pub const MAJOR: [u32; 7] = [2, 2, 1, 2, 2, 1, 2];

pub trait Pitch {
    fn midi_from_frequency(&self) -> f64;
    fn frequency_from_midi(&self) -> f64;
}

impl Pitch for f64 {
    fn midi_from_frequency(&self) -> f64 {
        69.0 + 12.0 * (self / 440.0).log2()
    }

    fn frequency_from_midi(&self) -> f64 {
        27.5*2.0f64.powf((self - 21.0) / 12.0)
    }
}
