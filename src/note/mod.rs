use regex::Regex;

pub const MINOR: [u32; 7] = [2, 1, 2, 2, 1, 2, 2];
pub const MAJOR: [u32; 7] = [2, 2, 1, 2, 2, 2, 1];

pub trait Pitch<T> {
    fn midi_from_frequency(&self) -> T;
    fn frequency_from_midi(&self) -> T;
}

macro_rules! pitch_impl {
    ( $( $type:ident ),* ) => {
        $(
        impl Pitch<$type> for $type {
            fn midi_from_frequency(&self) -> Self {
                (69.0 + 12.0 * (*self as f64 / 440.0).log2()) as Self
            }

            fn frequency_from_midi(&self) -> Self {
                (27.5*2.0f64.powf((*self as f64 - 21.0) / 12.0)) as Self
            }
        }
        )*
    }
}
pitch_impl!(usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64);

pub fn parse_roman_numeral_notation(src: &str) -> (usize, Vec<usize>) {
    let re = Regex::new(r"(?P<minor>[iv]{1,3})|(?P<major>[IV]{1,3})(?P<augmentation>[ᐤᐩ])?(?P<added_note>[⁰-⁹])?").unwrap();
    let caps = re.captures(src).unwrap();
    if caps.name("added_note").is_some() { unimplemented!() }
    let semitones = if caps.name("augmentation").is_some() {
        match caps.name("augmentation").unwrap().as_str() {
            "ᐤ"  => vec![0, 3, 6], //Diminished minor
            "+"  => vec![0, 4, 8], //Augmented major
            _ => panic!(),
        }
    } else if caps.name("major").is_some() {
        vec![0, 4, 7]
    } else if caps.name("minor").is_some() {
        vec![0, 3, 7]
    } else { panic!() };

    let degree = match caps.name("major").unwrap_or_else(|| caps.name("minor").unwrap()).as_str().to_uppercase().as_str() {
        "I" => 0,
        "II" => 1,
        "III" => 2,
        "IV" => 3,
        "V" => 4,
        "VI" => 5,
        "VII" => 6,
        _ => panic!(),
    };

    (degree, semitones)
}

pub struct Scale {
    pattern: Vec<u32>,
    root: u32,
}

impl Scale {
    pub fn new(pattern: &[u32], root: u32) -> Self {
        Self {
            pattern: pattern.iter().cloned().collect(),
            root,
        }
    }

    pub fn degree(&self, mut semitone: u32) -> Option<(i32, u32)> {
        let mut octave = 0;
        while semitone < self.root {
            semitone += 12;
            octave -= 1;
        }
        while semitone >= self.root + 12 {
            semitone -= 12;
            octave += 1;
        }
        let mut current = self.root;
        for (step_i, step) in self.pattern.iter().enumerate() {
            if current == semitone {
                return Some((octave, step_i as u32));
            }
            current += step;
        }
        if current == semitone {
            Some((octave, self.pattern.len() as u32))
        } else {
            None
        }
    }

    pub fn pitch(&self, mut degree: i32) -> u32 {
        let mut octave = 0;
        while degree < 0 {
            degree += self.pattern.len() as i32;
            octave -= 1;
        }
        while degree >= self.pattern.len() as i32 {
            degree -= self.pattern.len() as i32;
            octave += 1;
        }
        let mut semitone = self.root as i32;
        for i in 0..degree as usize {
            semitone += self.pattern[i] as i32;
        }
        (semitone + octave*12) as u32
    }
}
