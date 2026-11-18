//! The news system: story seeds -> template family -> slot fill -> tone -> significance -> placement.
//! Templates are data (assets/news_templates.txt, overridable by data/news_templates.txt next to the exe).
//! A story may only assert causes contained in its seed; with no causes the copy says so.

use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Scope {
    Local,
    National,
    Regional,
    Global,
}

impl Scope {
    pub fn name(self) -> &'static str {
        match self {
            Scope::Local => "Local",
            Scope::National => "National",
            Scope::Regional => "Regional",
            Scope::Global => "Global",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Triumph,
    Routine,
    Alarm,
    Tragedy,
    Irony,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlertClass {
    None,
    Existential,
    Major,
}

#[derive(Clone, Debug)]
pub struct StorySeed {
    pub kind: &'static str,
    pub scope: Scope,
    pub tone: Tone,
    pub slots: Vec<(&'static str, String)>,
    pub causes: Vec<String>,
    pub magnitude: f32,
    pub city: Option<u32>,
    pub civ: Option<u16>,
    pub civ2: Option<u16>,
    pub pos: Option<(i32, i32)>,
    pub category: &'static str,
    pub why: Vec<String>,
    pub card: bool,
    pub chronicle: bool,
    pub alert: AlertClass,
    pub perspective: Option<&'static str>,
    pub thread: Option<String>,
}

impl StorySeed {
    pub fn new(kind: &'static str, scope: Scope) -> Self {
        StorySeed {
            kind,
            scope,
            tone: Tone::Routine,
            slots: Vec::new(),
            causes: Vec::new(),
            magnitude: 1.0,
            city: None,
            civ: None,
            civ2: None,
            pos: None,
            category: "general",
            why: Vec::new(),
            card: false,
            chronicle: false,
            alert: AlertClass::None,
            perspective: None,
            thread: None,
        }
    }
    pub fn slot(mut self, k: &'static str, v: impl Into<String>) -> Self {
        self.slots.push((k, v.into()));
        self
    }
    pub fn cause(mut self, c: impl Into<String>) -> Self {
        self.causes.push(c.into());
        self
    }
    pub fn tone(mut self, t: Tone) -> Self {
        self.tone = t;
        self
    }
    pub fn at(mut self, x: i32, y: i32) -> Self {
        self.pos = Some((x, y));
        self
    }
    pub fn city(mut self, c: u32) -> Self {
        self.city = Some(c);
        self
    }
    pub fn civ(mut self, c: u16) -> Self {
        self.civ = Some(c);
        self
    }
    pub fn civ2(mut self, c: u16) -> Self {
        self.civ2 = Some(c);
        self
    }
    pub fn card(mut self) -> Self {
        self.card = true;
        self
    }
    pub fn chronicle(mut self) -> Self {
        self.chronicle = true;
        self.card = true;
        self
    }
    pub fn mag(mut self, m: f32) -> Self {
        self.magnitude = m;
        self
    }
    pub fn cat(mut self, c: &'static str) -> Self {
        self.category = c;
        self
    }
    pub fn why(mut self, w: Vec<String>) -> Self {
        self.why = w;
        self
    }
    pub fn alert(mut self, a: AlertClass) -> Self {
        self.alert = a;
        self
    }
    pub fn persp(mut self, p: &'static str) -> Self {
        self.perspective = Some(p);
        self
    }
    pub fn thread(mut self, t: impl Into<String>) -> Self {
        self.thread = Some(t.into());
        self
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        self.slots.iter().find(|(s, _)| *s == k).map(|(_, v)| v.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct NewsItem {
    pub id: usize,
    pub year: i32,
    pub headline: String,
    pub body: String,
    pub scope: Scope,
    pub category: &'static str,
    pub significance: f32,
    pub placement: Placement,
    pub city: Option<u32>,
    pub civ: Option<u16>,
    pub civ2: Option<u16>,
    pub pos: Option<(i32, i32)>,
    pub causes: Vec<String>,
    pub why: Vec<String>,
    pub alert: AlertClass,
    pub thread: Option<String>,
    pub kind: &'static str,
    pub era_skin: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Placement {
    Gazette,
    Ticker,
    Card,
    Chronicle,
}

#[derive(Clone, Debug)]
pub struct Template {
    pub kind: String,
    pub cond: String,
    pub headline: String,
    pub body: String,
}

pub struct Templates {
    pub list: Vec<Template>,
}

pub const DEFAULT_TEMPLATES: &str = include_str!("../../assets/news_templates.txt");

impl Templates {
    pub fn load() -> Self {
        let mut text = DEFAULT_TEMPLATES.to_string();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                if let Ok(extra) = std::fs::read_to_string(dir.join("data").join("news_templates.txt")) {
                    text.push('\n');
                    text.push_str(&extra);
                }
            }
        }
        let mut list = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() < 4 {
                continue;
            }
            list.push(Template {
                kind: parts[0].trim().to_string(),
                cond: parts[1].trim().to_string(),
                headline: parts[2].trim().to_string(),
                body: parts[3].trim().to_string(),
            });
        }
        Templates { list }
    }

    fn cond_ok(cond: &str, seed: &StorySeed, era: usize) -> bool {
        if cond == "*" || cond.is_empty() {
            return true;
        }
        cond.split(',').all(|c| {
            let c = c.trim();
            if let Some(v) = c.strip_prefix("band=") {
                seed.get("Band").map(|b| b == v).unwrap_or(false)
            } else if let Some(v) = c.strip_prefix("era>=") {
                era >= v.parse::<usize>().unwrap_or(0)
            } else if let Some(v) = c.strip_prefix("era<") {
                era < v.parse::<usize>().unwrap_or(99)
            } else if let Some(v) = c.strip_prefix("persp=") {
                seed.perspective == Some(match v {
                    "A" => "A",
                    "B" => "B",
                    _ => "W",
                })
            } else if let Some(v) = c.strip_prefix("tone=") {
                format!("{:?}", seed.tone).eq_ignore_ascii_case(v)
            } else if let Some(v) = c.strip_prefix("has=") {
                seed.get(v).is_some()
            } else {
                true
            }
        })
    }

    pub fn render(&self, seed: &StorySeed, era: usize, rng: &mut ChaCha8Rng) -> (String, String) {
        let cands: Vec<&Template> = self
            .list
            .iter()
            .filter(|t| t.kind == seed.kind && Self::cond_ok(&t.cond, seed, era))
            .collect();
        // prefer the most specific matching templates (those with a condition)
        let specific: Vec<&&Template> = cands.iter().filter(|t| t.cond != "*").collect();
        let chosen: Option<&Template> = if !specific.is_empty() {
            specific.choose(rng).map(|t| **t)
        } else {
            cands.choose(rng).copied()
        };
        let (h, b) = match chosen {
            Some(t) => (t.headline.clone(), t.body.clone()),
            None => (format!("{{Kind}} reported"), "{Detail} {CauseLine}".to_string()),
        };
        (fill(&h, seed, era), fill(&b, seed, era))
    }
}

fn cause_line(seed: &StorySeed) -> String {
    if seed.causes.is_empty() {
        "The cause is unknown or disputed.".to_string()
    } else {
        format!("Cause: {}.", seed.causes.join("; "))
    }
}

fn fill(t: &str, seed: &StorySeed, _era: usize) -> String {
    let mut out = String::with_capacity(t.len() + 32);
    let mut rest = t;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        if let Some(end) = after.find('}') {
            let key = &after[..end];
            let val = match key {
                "CauseLine" => cause_line(seed),
                "Causes" => {
                    if seed.causes.is_empty() {
                        "causes unknown or disputed".to_string()
                    } else {
                        seed.causes.join("; ")
                    }
                }
                "Kind" => seed.kind.replace('_', " "),
                _ => seed.get(key).map(|s| s.to_string()).unwrap_or_else(|| "—".to_string()),
            };
            out.push_str(&val);
            rest = &after[end + 1..];
        } else {
            out.push_str(&rest[start..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

pub fn era_skin(era: usize) -> &'static str {
    match era {
        0 | 1 => "Clay Bulletin",
        2 | 3 => "Herald's Scroll",
        4 | 5 => "Broadsheet",
        6 => "Broadcast",
        _ => "Feed",
    }
}

#[derive(Default)]
pub struct NewsLog {
    pub items: Vec<NewsItem>,
    pub unread_cards: Vec<usize>,
    pub muted: Vec<&'static str>,
    pub watch_civs: Vec<u16>,
    pub watch_cities: Vec<u32>,
    pub followed_threads: Vec<String>,
    pub threshold_ticker: f32,
    pub threshold_card: f32,
}
