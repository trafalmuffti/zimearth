//! Civilizations, leaders, technology and civics trees, governments, religions and the diplomacy ledger.

use super::names::Language;
use rand::Rng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

pub const T_EXPANSION: usize = 0;
pub const T_AGGRESSION: usize = 1;
pub const T_GREED: usize = 2;
pub const T_PIETY: usize = 3;
pub const T_CURIOSITY: usize = 4;
pub const T_LOYALTY: usize = 5;
pub const T_VINDICTIVE: usize = 6;
pub const T_ECOLOGY: usize = 7;

pub const TRAIT_NAMES: [(&str, &str, &str); 8] = [
    ("Expansion", "Tall", "Settler"),
    ("Aggression", "Peaceful", "Conqueror"),
    ("Greed", "Austere", "Mercantile"),
    ("Piety", "Secular", "Zealous"),
    ("Curiosity", "Traditional", "Innovative"),
    ("Loyalty", "Opportunist", "Faithful"),
    ("Vindictiveness", "Forgiving", "Grudging"),
    ("Ecology", "Exploitive", "Steward"),
];

pub const ERA_NAMES: [&str; 9] = [
    "Stone", "River", "Classical", "Medieval", "Exploration", "Industrial", "Modern", "Information",
    "Futures",
];

pub struct TechDef {
    pub name: &'static str,
    pub era: usize,
    pub prereq: &'static [&'static str],
    pub needs_coast: bool,
    pub eureka: &'static str,
}

macro_rules! tech {
    ($n:expr, $e:expr, [$($p:expr),*], $c:expr, $eu:expr) => {
        TechDef { name: $n, era: $e, prereq: &[$($p),*], needs_coast: $c, eureka: $eu }
    };
}

pub static TECHS: &[TechDef] = &[
    tech!("Foraging", 0, [], false, ""),
    tech!("Pottery", 0, ["Foraging"], false, "a harvest surplus"),
    tech!("Animal Husbandry", 0, ["Foraging"], false, "pasture land"),
    tech!("Mining", 0, ["Foraging"], false, "a mountain in reach"),
    tech!("Irrigation", 1, ["Pottery"], false, "a river city"),
    tech!("Writing", 1, ["Pottery"], false, "meeting another people"),
    tech!("Bronze Working", 1, ["Mining"], false, "copper in reach"),
    tech!("Sailing", 1, ["Pottery"], true, "a coastal city"),
    tech!("Masonry", 1, ["Mining"], false, "a quarry"),
    tech!("Mathematics", 2, ["Writing"], false, ""),
    tech!("Iron Working", 2, ["Bronze Working"], false, "a war"),
    tech!("Currency", 2, ["Writing", "Bronze Working"], false, "a trade route"),
    tech!("Horseback Riding", 2, ["Animal Husbandry"], false, "horses"),
    tech!("Engineering", 3, ["Mathematics", "Masonry"], false, "a wonder"),
    tech!("Astronomy", 3, ["Mathematics"], false, "an omen in the sky"),
    tech!("Steel", 3, ["Iron Working"], false, "iron in reach"),
    tech!("Feudal Farming", 3, ["Irrigation", "Currency"], false, "surviving famine"),
    tech!("Navigation", 4, ["Sailing", "Astronomy"], true, "a new continent sighted"),
    tech!("Gunpowder", 4, ["Steel"], false, "a long war"),
    tech!("Printing", 4, ["Engineering"], false, "a forbidden book"),
    tech!("Banking", 4, ["Currency", "Engineering"], false, "a rich city"),
    tech!("Steam Power", 5, ["Printing", "Steel"], false, ""),
    tech!("Railroads", 5, ["Steam Power"], false, "coal in reach"),
    tech!("Industrialization", 5, ["Steam Power", "Banking"], false, ""),
    tech!("Sanitation", 5, ["Printing"], false, "surviving plague"),
    tech!("Electricity", 6, ["Industrialization"], false, ""),
    tech!("Combustion", 6, ["Railroads"], false, ""),
    tech!("Flight", 6, ["Combustion", "Electricity"], false, ""),
    tech!("Antibiotics", 6, ["Sanitation", "Electricity"], false, "surviving plague"),
    tech!("Computers", 7, ["Electricity", "Flight"], false, ""),
    tech!("Nuclear Fission", 7, ["Electricity"], false, ""),
    tech!("Genetics", 7, ["Antibiotics", "Computers"], false, ""),
    tech!("Satellites", 7, ["Flight", "Computers"], false, ""),
    tech!("Fusion", 8, ["Nuclear Fission", "Computers"], false, ""),
    tech!("Ecological Engineering", 8, ["Genetics"], false, "ecological collapse nearby"),
    tech!("Arcologies", 8, ["Ecological Engineering", "Fusion"], false, ""),
    tech!("Starship", 8, ["Fusion", "Satellites"], false, ""),
];

pub fn tech_index(name: &str) -> usize {
    TECHS
        .iter()
        .position(|t| t.name == name)
        .unwrap_or_else(|| panic!("unknown tech {name}"))
}

pub struct CivicDef {
    pub name: &'static str,
    pub era: usize,
    pub prereq: &'static [&'static str],
}

pub static CIVICS: &[CivicDef] = &[
    CivicDef { name: "Code of Laws", era: 0, prereq: &[] },
    CivicDef { name: "Mysticism", era: 0, prereq: &["Code of Laws"] },
    CivicDef { name: "Early Empire", era: 1, prereq: &["Code of Laws"] },
    CivicDef { name: "Monarchy", era: 1, prereq: &["Early Empire"] },
    CivicDef { name: "Philosophy", era: 2, prereq: &["Mysticism"] },
    CivicDef { name: "Theology", era: 2, prereq: &["Philosophy"] },
    CivicDef { name: "Guilds", era: 3, prereq: &["Monarchy"] },
    CivicDef { name: "Civil Service", era: 3, prereq: &["Philosophy"] },
    CivicDef { name: "Mercantilism", era: 4, prereq: &["Guilds"] },
    CivicDef { name: "Humanism", era: 4, prereq: &["Civil Service"] },
    CivicDef { name: "Nationalism", era: 5, prereq: &["Mercantilism"] },
    CivicDef { name: "Ideology", era: 5, prereq: &["Humanism"] },
    CivicDef { name: "Mass Media", era: 6, prereq: &["Ideology"] },
    CivicDef { name: "Conservation", era: 6, prereq: &["Humanism"] },
    CivicDef { name: "Globalization", era: 7, prereq: &["Mass Media"] },
    CivicDef { name: "Networks", era: 7, prereq: &["Mass Media"] },
    CivicDef { name: "Synthesis", era: 8, prereq: &["Networks", "Conservation"] },
];

pub fn civic_index(name: &str) -> usize {
    CIVICS.iter().position(|t| t.name == name).unwrap_or(0)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Succession {
    Hereditary,
    Elective,
    Theocratic,
    Military,
    Corporate,
    Machine,
}

pub struct GovDef {
    pub name: &'static str,
    pub civic: &'static str,
    pub succession: Succession,
    pub stability: f32,
    pub corruption: f32,
    pub tax: f32,
    pub market: bool,
    pub science: f32,
    pub faith: f32,
}

pub static GOVS: &[GovDef] = &[
    GovDef { name: "Chiefdom", civic: "", succession: Succession::Hereditary, stability: 0.0, corruption: 0.1, tax: 0.6, market: true, science: 1.0, faith: 1.0 },
    GovDef { name: "Despotism", civic: "Early Empire", succession: Succession::Military, stability: -4.0, corruption: 0.2, tax: 0.8, market: true, science: 0.95, faith: 1.0 },
    GovDef { name: "Monarchy", civic: "Monarchy", succession: Succession::Hereditary, stability: 6.0, corruption: 0.12, tax: 0.85, market: true, science: 1.0, faith: 1.05 },
    GovDef { name: "Classical Republic", civic: "Philosophy", succession: Succession::Elective, stability: 3.0, corruption: 0.08, tax: 0.9, market: true, science: 1.1, faith: 0.95 },
    GovDef { name: "Theocracy", civic: "Theology", succession: Succession::Theocratic, stability: 5.0, corruption: 0.1, tax: 0.8, market: true, science: 0.9, faith: 1.4 },
    GovDef { name: "Merchant Republic", civic: "Mercantilism", succession: Succession::Elective, stability: 2.0, corruption: 0.1, tax: 1.1, market: true, science: 1.05, faith: 0.95 },
    GovDef { name: "Democracy", civic: "Ideology", succession: Succession::Elective, stability: 4.0, corruption: 0.05, tax: 1.0, market: true, science: 1.2, faith: 0.9 },
    GovDef { name: "Command Economy", civic: "Nationalism", succession: Succession::Military, stability: 0.0, corruption: 0.15, tax: 1.0, market: false, science: 1.05, faith: 0.8 },
    GovDef { name: "Corporate State", civic: "Globalization", succession: Succession::Corporate, stability: 1.0, corruption: 0.12, tax: 1.25, market: true, science: 1.15, faith: 0.85 },
    GovDef { name: "Synthetic Commonwealth", civic: "Synthesis", succession: Succession::Machine, stability: 8.0, corruption: 0.02, tax: 1.1, market: true, science: 1.35, faith: 0.9 },
];

#[derive(Clone, Debug)]
pub struct Leader {
    pub name: String,
    pub title: String,
    pub age: i32,
    pub lifespan: i32,
    pub traits: [f32; 8],
    pub reign_start: i32,
    pub defining_act: Option<String>,
    pub dream: Option<(usize, f32)>,
}

impl Leader {
    pub fn trait_word(&self) -> String {
        let mut best = (0usize, 0f32);
        for (k, v) in self.traits.iter().enumerate() {
            let d = (v - 0.5).abs();
            if d > best.1 {
                best = (k, d);
            }
        }
        let (_, lo, hi) = TRAIT_NAMES[best.0];
        let word = if self.traits[best.0] > 0.5 { hi } else { lo };
        word.to_lowercase()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Strategy {
    Expand,
    Consolidate,
    Trade,
    Convert,
    Arm,
    Research,
}

impl Strategy {
    pub fn name(self) -> &'static str {
        match self {
            Strategy::Expand => "Expand",
            Strategy::Consolidate => "Consolidate",
            Strategy::Trade => "Trade",
            Strategy::Convert => "Convert",
            Strategy::Arm => "Arm",
            Strategy::Research => "Research",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Agenda {
    pub name: &'static str,
    pub desc: &'static str,
    pub key: usize,
}

pub static AGENDAS: &[(&str, &str)] = &[
    ("Sea Lanes", "values coastal trade and dislikes those who blockade it"),
    ("Old Gods", "respects shared faith and despises rival holy cities"),
    ("Great Builder", "admires wonder builders and envies them"),
    ("Iron Fist", "respects strong armies and scorns the weak"),
    ("Green Mantle", "admires stewards of forest and hates polluters"),
    ("Gold Hoarder", "envies the richest civilizations"),
    ("Scholar's Lamp", "befriends the curious and shares knowledge"),
    ("Frontier Hunger", "covets the land of neighbors with few cities"),
    ("Kin Loyalty", "keeps promises and never forgets a betrayal"),
    ("River Mother", "loves fertile rivers and anyone who protects them"),
];

#[derive(Clone, Debug)]
pub struct Civ {
    pub id: u16,
    pub name: String,
    pub demonym: String,
    pub color: [u8; 3],
    pub lang: Language,
    pub leader: Leader,
    pub leaders_past: Vec<(String, i32, i32, String)>,
    pub alive: bool,
    pub minor: bool,
    pub minor_kind: &'static str,
    pub suzerain: Option<u16>,
    pub founded: i32,
    pub fell: Option<i32>,
    pub capital: Option<u32>,
    pub cities: Vec<u32>,
    pub treasury: f32,
    pub gold_income: f32,
    pub known_tech: Vec<bool>,
    pub known_civic: Vec<bool>,
    pub research: Option<usize>,
    pub research_progress: f32,
    pub civic_research: Option<usize>,
    pub civic_progress: f32,
    pub eureka: Vec<bool>,
    pub era: usize,
    pub government: usize,
    pub gov_change_year: i32,
    pub religion: Option<usize>,
    pub strategy: Strategy,
    pub agenda_visible: usize,
    pub agenda_hidden: usize,
    pub agenda_revealed: bool,
    pub ability: &'static str,
    pub ability_desc: &'static str,
    pub war_weariness: f32,
    pub stability: f32,
    pub military: f32,
    pub score: f32,
    pub score_parts: [f32; 8],
    pub settler_progress: f32,
    pub homeland_biome: &'static str,
    pub famine_memory: f32,
    pub betrayed_memory: f32,
    pub peak_pop: f32,
    pub pop_history: Vec<f32>,
    pub contacted_continents: Vec<u16>,
    pub spy_progress: f32,
    pub defining_moments: Vec<String>,
    pub parent: Option<u16>,
    pub wonders: Vec<String>,
}

impl Civ {
    pub fn knows(&self, tech: &str) -> bool {
        self.known_tech[tech_index(tech)]
    }
    pub fn knows_civic(&self, c: &str) -> bool {
        self.known_civic[civic_index(c)]
    }
    pub fn gov(&self) -> &'static GovDef {
        &GOVS[self.government]
    }
    pub fn military_tier(&self) -> f32 {
        let mut t = 1.0;
        for (n, v) in [
            ("Bronze Working", 0.3),
            ("Iron Working", 0.4),
            ("Horseback Riding", 0.25),
            ("Steel", 0.5),
            ("Gunpowder", 0.9),
            ("Railroads", 0.6),
            ("Combustion", 1.2),
            ("Flight", 1.4),
            ("Computers", 1.0),
            ("Nuclear Fission", 1.5),
            ("Fusion", 1.5),
        ] {
            if self.knows(n) {
                t += v;
            }
        }
        t
    }
    pub fn full_name(&self) -> String {
        if self.minor {
            format!("city-state of {}", self.name)
        } else {
            self.name.clone()
        }
    }
}

pub fn gen_leader(lang: &mut Language, rng: &mut ChaCha8Rng, base: Option<&[f32; 8]>, year: i32, era: usize) -> Leader {
    let mut traits = [0.5f32; 8];
    for k in 0..8 {
        let b = base.map(|b| b[k]).unwrap_or(0.5);
        traits[k] = (b + rng.gen_range(-0.35..0.35)).clamp(0.02, 0.98);
    }
    let titles = match era {
        0 => ["Chief", "Elder", "Matriarch", "Speaker"],
        1 | 2 => ["King", "Queen", "Archon", "High Priest"],
        3 | 4 => ["Sovereign", "Doge", "Emperor", "Empress"],
        5 | 6 => ["President", "Chancellor", "Premier", "Marshal"],
        _ => ["Director", "First Speaker", "Coordinator", "Consensus"],
    };
    Leader {
        name: lang.person(rng),
        title: titles.choose(rng).unwrap().to_string(),
        age: rng.gen_range(24..48),
        lifespan: rng.gen_range(52..80) + era as i32 * 3,
        traits,
        reign_start: year,
        defining_act: None,
        dream: None,
    }
}

pub static ABILITIES: &[(&str, &str, &str)] = &[
    ("Floodplain Masters", "river", "Extra food from river tiles; drought hits them softer."),
    ("Caravan Kings", "desert", "Trade routes yield 30% more gold."),
    ("Tidecallers", "coast", "Harbors and fisheries yield more; sail earlier."),
    ("Mountain Halls", "mountain", "Cities near mountains gain production and stronger walls."),
    ("Forest Wardens", "forest", "Forests near cities give culture; slower to deforest."),
    ("Steppe Riders", "grass", "Armies move faster and fight better on open ground."),
    ("Frost-Hardened", "cold", "Cold snaps and tundra hurt far less."),
    ("Spice Lords", "jungle", "Luxury resources add extra wealth."),
    ("Star Readers", "any", "Science bonus; observatory omens become eurekas."),
    ("Ancestor Keepers", "any", "Faith bonus; stability recovers faster after disasters."),
    ("Iron Discipline", "any", "Garrisons stronger; war weariness grows slower."),
    ("Open Hands", "any", "Refugees and migrants boost growth instead of straining it."),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    SharedReligion,
    RivalReligion,
    Trade,
    BorderFriction,
    PastAid,
    BrokenPromise,
    TerritorialEnvy,
    WealthEnvy,
    Ideology,
    WarGrievance,
    Agenda,
    FirstContact,
    Alliance,
    CityTaken,
    Espionage,
    Atrocity,
}

impl Reason {
    pub fn text(self) -> &'static str {
        match self {
            Reason::SharedReligion => "Shared religion",
            Reason::RivalReligion => "Competing faiths",
            Reason::Trade => "Trade volume",
            Reason::BorderFriction => "Border friction",
            Reason::PastAid => "Past aid",
            Reason::BrokenPromise => "Broken promise",
            Reason::TerritorialEnvy => "Territorial envy",
            Reason::WealthEnvy => "Wealth envy",
            Reason::Ideology => "Ideological distance",
            Reason::WarGrievance => "Grievance from war",
            Reason::Agenda => "Agenda",
            Reason::FirstContact => "First impressions",
            Reason::Alliance => "Alliance",
            Reason::CityTaken => "City taken",
            Reason::Espionage => "Caught spying",
            Reason::Atrocity => "Atrocity",
        }
    }
}

#[derive(Clone, Debug)]
pub struct LedgerEntry {
    pub reason: Reason,
    pub value: f32,
    pub expires: Option<i32>,
    pub note: String,
}

#[derive(Clone, Debug, Default)]
pub struct Relation {
    pub met: bool,
    pub ledger: Vec<LedgerEntry>,
    pub score: f32,
    pub at_war: bool,
    pub war_start: i32,
    pub war_aim: String,
    pub open_borders: bool,
    pub trade_agreement: bool,
    pub defensive_pact: bool,
    pub alliance: bool,
    pub research_agreement: bool,
    pub nonaggression_until: i32,
    pub denounced: bool,
    pub last_war_end: i32,
}

pub fn rel_state(score: f32, r: &Relation) -> &'static str {
    if r.at_war {
        "At war"
    } else if r.alliance {
        "Allied"
    } else if r.denounced || score < -60.0 {
        "Denounced"
    } else if score < -30.0 {
        "Hostile"
    } else if score < -10.0 {
        "Unfriendly"
    } else if score < 15.0 {
        "Neutral"
    } else if score < 40.0 {
        "Friendly"
    } else {
        "Declared Friends"
    }
}

#[derive(Clone, Debug)]
pub struct Religion {
    pub name: String,
    pub theme: &'static str,
    pub founder: u16,
    pub holy_city: u32,
    pub founded: i32,
    pub beliefs: Vec<&'static str>,
    pub parent: Option<usize>,
    pub shaper_cult: bool,
}

pub static RELIGION_THEMES: &[(&str, &[&str])] = &[
    ("River", &["Floodwater Blessing", "Ferry of Souls", "Delta Festivals", "Water Courts"]),
    ("Sun", &["Solar Calendar", "Golden Temples", "Noon Rites", "Pilgrim Roads"]),
    ("Storm", &["Thunder Oaths", "Warrior Monks", "Rain Dances", "Lightning Omens"]),
    ("Ancestor", &["Ancestor Halls", "Tomb Pilgrimage", "Lineage Law", "Elders' Council"]),
    ("Machine", &["Sacred Engines", "Code Litanies", "Workshop Monasteries", "Perfect Order"]),
    ("Ecology", &["Sacred Groves", "Animal Kinship", "Seed Vaults", "Green Tithes"]),
    ("Moon", &["Tide Mysteries", "Night Vigils", "Lunar Healers", "Silver Shrines"]),
    ("Shaper", &["The Watching Sky", "Omen Readers", "Earth's Hand", "Offerings to the Shaper"]),
];
