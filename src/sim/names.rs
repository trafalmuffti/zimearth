//! Generated language profiles so every people's names feel internally consistent.

use rand::Rng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Debug)]
pub struct Language {
    pub onsets: Vec<&'static str>,
    pub vowels: Vec<&'static str>,
    pub codas: Vec<&'static str>,
    pub endings: Vec<&'static str>,
    pub demonym_suffix: &'static str,
    pub used: Vec<String>,
}

const ONSETS: &[&str] = &[
    "b", "d", "g", "k", "t", "p", "m", "n", "s", "r", "l", "v", "z", "h", "th", "sh", "kh", "dr",
    "tr", "br", "gr", "kr", "st", "f", "j", "y", "w", "ch", "ph", "q", "x", "zh", "sk", "nd",
];
const VOWELS: &[&str] = &[
    "a", "e", "i", "o", "u", "aa", "ei", "ou", "ia", "ae", "y", "oa", "ai", "io", "u", "a", "e",
];
const CODAS: &[&str] = &[
    "", "", "", "n", "r", "l", "s", "m", "th", "k", "sh", "nd", "rn", "x", "t", "lk", "st",
];
const ENDINGS: &[&str] = &[
    "a", "on", "ar", "eth", "is", "um", "or", "ia", "en", "ash", "ir", "ae", "ul", "os", "at",
    "ek", "ine", "and", "oth", "ura",
];
const DEMONYMS: &[&str] = &["ans", "i", "ese", "ites", "ari", "ics", "ens", "oth", "ani", "ish"];

impl Language {
    pub fn generate(rng: &mut ChaCha8Rng) -> Self {
        let pick = |rng: &mut ChaCha8Rng, src: &[&'static str], n: usize| {
            let mut v: Vec<&'static str> = src.to_vec();
            v.shuffle(rng);
            v.truncate(n);
            v
        };
        let n_on = rng.gen_range(7..13);
        let n_v = rng.gen_range(3..7);
        Language {
            onsets: pick(rng, ONSETS, n_on),
            vowels: pick(rng, VOWELS, n_v),
            codas: {
                let k = rng.gen_range(3..7);
                let mut c = pick(rng, CODAS, k);
                c.push("");
                c
            },
            endings: {
                let k = rng.gen_range(3..6);
                pick(rng, ENDINGS, k)
            },
            demonym_suffix: DEMONYMS.choose(rng).copied().unwrap_or("ans"),
            used: Vec::new(),
        }
    }

    fn syllable(&self, rng: &mut ChaCha8Rng) -> String {
        let mut s = String::new();
        if rng.gen_bool(0.85) {
            s.push_str(self.onsets.choose(rng).unwrap());
        }
        s.push_str(self.vowels.choose(rng).unwrap());
        if rng.gen_bool(0.4) {
            s.push_str(self.codas.choose(rng).unwrap());
        }
        s
    }

    pub fn word(&mut self, rng: &mut ChaCha8Rng, min: usize, max: usize) -> String {
        for _ in 0..40 {
            let n = rng.gen_range(min..=max);
            let mut w = String::new();
            for _ in 0..n {
                w.push_str(&self.syllable(rng));
            }
            if rng.gen_bool(0.45) {
                w.push_str(self.endings.choose(rng).unwrap());
            }
            if w.len() < 3 || w.len() > 11 {
                continue;
            }
            let w = capitalize(&w);
            if !self.used.contains(&w) {
                self.used.push(w.clone());
                return w;
            }
        }
        let w = capitalize(&format!("{}{}", self.syllable(rng), self.used.len()));
        self.used.push(w.clone());
        w
    }

    pub fn demonym(&self, name: &str) -> String {
        let base = name.trim_end_matches(|c| "aeiouy".contains(c));
        let base = if base.len() < 3 { name } else { base };
        format!("{}{}", base, self.demonym_suffix)
    }

    pub fn person(&mut self, rng: &mut ChaCha8Rng) -> String {
        let first = self.word(rng, 1, 2);
        if rng.gen_bool(0.35) {
            let second = self.word(rng, 1, 2);
            format!("{} {}", first, second)
        } else {
            first
        }
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn roman(n: u32) -> String {
    let table = [
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut n = n.min(39);
    let mut s = String::new();
    for &(v, r) in &table {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}
