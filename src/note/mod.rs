use regex::Regex;

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

pub fn parse_roman_numeral_notation(src: &str) -> Vec<usize> {
    let re = Regex::new(r"(?P<minor>[iv]{1,3})|(?P<major>[IV]{1,3})(?P<augmentation>[ᐤᐩ])?(?P<added_note>[⁰-⁹])?").unwrap();
    let caps = re.captures(src).unwrap();
    if caps.name("added_note").is_some() { unimplemented!() }
    let base = if caps.name("augmentation").is_some() {
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

    let degrees: Vec<usize> = match caps.name("major").unwrap_or_else(|| caps.name("minor").unwrap()).as_str().to_uppercase().as_str() {
        "I" => base,
        "II" => base.iter().map(|x| x+1).collect(),
        "III" => base.iter().map(|x| x+2).collect(),
        "IV" => base.iter().map(|x| x+3).collect(),
        "V" => base.iter().map(|x| x+4).collect(),
        "VI" => base.iter().map(|x| x+5).collect(),
        "VII" => base.iter().map(|x| x+6).collect(),
        _ => panic!(),
    };

    degrees
}
