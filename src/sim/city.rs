//! Cities: buildings, governors, yields and the canonical wealth score (design doc 5.3).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Spec {
    None,
    Breadbasket,
    Industrial,
    Port,
    Holy,
    Scholarly,
    Fortress,
    Resort,
    Financial,
}

impl Spec {
    pub fn name(self) -> &'static str {
        match self {
            Spec::None => "town",
            Spec::Breadbasket => "breadbasket",
            Spec::Industrial => "industrial city",
            Spec::Port => "port city",
            Spec::Holy => "holy city",
            Spec::Scholarly => "scholarly city",
            Spec::Fortress => "fortress city",
            Spec::Resort => "resort city",
            Spec::Financial => "financial city",
        }
    }
}

pub struct BuildingDef {
    pub name: &'static str,
    pub tech: &'static str,
    pub cost: f32,
    pub upkeep: f32,
    pub value: f32,
    pub needs_coast: bool,
    pub needs_river: bool,
}

macro_rules! b {
    ($n:expr, $t:expr, $c:expr, $u:expr, $v:expr) => {
        BuildingDef { name: $n, tech: $t, cost: $c, upkeep: $u, value: $v, needs_coast: false, needs_river: false }
    };
    ($n:expr, $t:expr, $c:expr, $u:expr, $v:expr, coast) => {
        BuildingDef { name: $n, tech: $t, cost: $c, upkeep: $u, value: $v, needs_coast: true, needs_river: false }
    };
    ($n:expr, $t:expr, $c:expr, $u:expr, $v:expr, river) => {
        BuildingDef { name: $n, tech: $t, cost: $c, upkeep: $u, value: $v, needs_coast: false, needs_river: true }
    };
}

pub static BUILDINGS: &[BuildingDef] = &[
    b!("Granary", "Pottery", 30.0, 0.3, 10.0),
    b!("Monument", "Foraging", 25.0, 0.2, 8.0),
    b!("Shrine", "Foraging", 28.0, 0.3, 8.0),
    b!("Walls", "Masonry", 45.0, 0.5, 12.0),
    b!("Market", "Currency", 60.0, 0.6, 25.0),
    b!("Library", "Writing", 55.0, 0.6, 18.0),
    b!("Harbor", "Sailing", 60.0, 0.6, 22.0, coast),
    b!("Temple", "Mathematics", 65.0, 0.8, 18.0),
    b!("Aqueduct", "Engineering", 80.0, 0.8, 20.0),
    b!("Workshop", "Bronze Working", 55.0, 0.6, 15.0),
    b!("Flood Defenses", "Engineering", 70.0, 0.6, 12.0, river),
    b!("Bank", "Banking", 110.0, 1.2, 45.0),
    b!("University", "Printing", 120.0, 1.4, 40.0),
    b!("Castle", "Steel", 100.0, 1.0, 20.0),
    b!("Vault", "Banking", 90.0, 0.8, 15.0),
    b!("Factory", "Industrialization", 180.0, 2.0, 55.0),
    b!("Hospital", "Sanitation", 150.0, 1.6, 35.0),
    b!("Seawall", "Industrialization", 160.0, 1.4, 20.0, coast),
    b!("Stock Exchange", "Electricity", 220.0, 2.4, 80.0),
    b!("Power Plant", "Electricity", 220.0, 2.5, 50.0),
    b!("Research Lab", "Computers", 260.0, 3.0, 70.0),
    b!("Nature Reserve", "Ecological Engineering", 200.0, 1.5, 40.0),
    b!("Arcology", "Arcologies", 420.0, 4.0, 120.0),
];

pub fn bidx(name: &str) -> usize {
    BUILDINGS.iter().position(|b| b.name == name).unwrap_or_else(|| panic!("building {name}"))
}

pub struct WonderDef {
    pub name: &'static str,
    pub tech: &'static str,
    pub cost: f32,
    pub req: &'static str,
}

pub static WONDERS: &[WonderDef] = &[
    WonderDef { name: "Great Granary of the Plains", tech: "Irrigation", cost: 180.0, req: "river" },
    WonderDef { name: "Stone Circle", tech: "Masonry", cost: 150.0, req: "any" },
    WonderDef { name: "Lighthouse of the Strait", tech: "Sailing", cost: 170.0, req: "coast" },
    WonderDef { name: "Hanging Terraces", tech: "Irrigation", cost: 200.0, req: "desert" },
    WonderDef { name: "Grand Library", tech: "Writing", cost: 230.0, req: "any" },
    WonderDef { name: "Colossal Harbor", tech: "Currency", cost: 260.0, req: "coast" },
    WonderDef { name: "Sky Observatory", tech: "Astronomy", cost: 300.0, req: "mountain" },
    WonderDef { name: "Cathedral of Echoes", tech: "Engineering", cost: 320.0, req: "holy" },
    WonderDef { name: "Great Wall of Dust", tech: "Engineering", cost: 340.0, req: "any" },
    WonderDef { name: "Merchant's Exchange", tech: "Banking", cost: 380.0, req: "rich" },
    WonderDef { name: "Leaning Tower of Ages", tech: "Printing", cost: 360.0, req: "any" },
    WonderDef { name: "Grand Canal", tech: "Navigation", cost: 420.0, req: "river" },
    WonderDef { name: "Iron Bridge", tech: "Railroads", cost: 480.0, req: "river" },
    WonderDef { name: "Great Dam", tech: "Electricity", cost: 600.0, req: "river" },
    WonderDef { name: "World Fairground", tech: "Industrialization", cost: 520.0, req: "rich" },
    WonderDef { name: "Orbital Ring", tech: "Satellites", cost: 900.0, req: "any" },
    WonderDef { name: "Seed Ark", tech: "Genetics", cost: 760.0, req: "any" },
    WonderDef { name: "Sky Arcology", tech: "Arcologies", cost: 1100.0, req: "any" },
];

#[derive(Clone, Debug)]
pub struct Governor {
    pub name: String,
    pub competence: f32,
    pub corruption: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Yields {
    pub food: f32,
    pub prod: f32,
    pub gold: f32,
    pub science: f32,
    pub culture: f32,
    pub faith: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Chains {
    pub drought: u8,
    pub gold_rush: u8,
    pub plague: u8,
    pub relic: u8,
    pub river_shift: u8,
    pub accident: u8,
    pub succession: u8,
    pub stage_year: i32,
}

#[derive(Clone, Debug)]
pub enum Build {
    Building(usize),
    Wonder(usize),
    Army,
    Settler,
    Project(&'static str),
}

#[derive(Clone, Debug)]
pub struct City {
    pub id: u32,
    pub civ: u16,
    pub orig_civ: u16,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub founded: i32,
    pub alive: bool,
    pub pop: f32,
    pub food_store: f32,
    pub buildings: Vec<bool>,
    pub wonders: Vec<usize>,
    pub build: Option<Build>,
    pub build_progress: f32,
    pub spec: Spec,
    pub governor: Governor,
    pub backlog: f32,
    pub stability: f32,
    pub health: f32,
    pub wealth: f32,
    pub wealth_parts: [f32; 6],
    pub wealth_hist: Vec<f32>,
    pub yields: Yields,
    pub culture: f32,
    pub faith_acc: f32,
    pub religion: Option<usize>,
    pub conv_pressure: f32,
    pub relic: bool,
    pub is_capital: bool,
    pub coastal: bool,
    pub river: bool,
    pub radius: i32,
    pub trade_income: f32,
    pub cooldowns: Vec<(u16, i32)>,
    pub chains: Chains,
    pub gazette: Vec<usize>,
    pub famine_years: u8,
    pub plague_severity: f32,
    pub protected_until: i32,
    pub siege: f32,
    pub garrison: f32,
    pub loyalty: f32,
    pub golden_until: i32,
    pub catchment: Vec<usize>,
    pub resources: Vec<super::planet::ResKind>,
    pub unrest: f32,
    pub migration: f32,
    pub suspected_spy: Option<u16>,
    pub debt: f32,
    pub debt_lender: Option<u16>,
    pub pop_history: Vec<f32>,
    pub stranded: bool,
}

pub const WEALTH_WEIGHTS: [(&str, f32); 6] = [
    ("Treasury & income per capita", 0.30),
    ("Trade throughput", 0.25),
    ("Resource access", 0.15),
    ("Buildings & infrastructure", 0.15),
    ("Population prosperity", 0.10),
    ("Stability modifier", 0.05),
];

pub fn wealth_band(w: f32) -> &'static str {
    match w as i32 {
        i32::MIN..=99 => "Destitute",
        100..=249 => "Poor",
        250..=449 => "Modest",
        450..=649 => "Prosperous",
        650..=849 => "Rich",
        _ => "Opulent",
    }
}

pub fn band_index(w: f32) -> usize {
    match w as i32 {
        i32::MIN..=99 => 0,
        100..=249 => 1,
        250..=449 => 2,
        450..=649 => 3,
        650..=849 => 4,
        _ => 5,
    }
}

pub const BAND_COLORS: [[u8; 3]; 6] = [
    [120, 40, 40],
    [200, 90, 60],
    [210, 180, 90],
    [120, 190, 100],
    [60, 160, 210],
    [190, 110, 230],
];

impl City {
    pub fn has(&self, name: &str) -> bool {
        self.buildings[bidx(name)]
    }
    pub fn display_pop(&self, era: usize) -> u64 {
        let m = [1.0, 1.6, 2.6, 4.0, 6.5, 11.0, 20.0, 34.0, 50.0][era.min(8)];
        (self.pop * 1000.0 * m) as u64
    }
    pub fn band(&self) -> &'static str {
        wealth_band(self.wealth)
    }
    pub fn on_cooldown(&self, ev: u16, year: i32) -> bool {
        self.cooldowns.iter().any(|&(e, y)| e == ev && y > year)
    }
    pub fn set_cooldown(&mut self, ev: u16, until: i32) {
        self.cooldowns.retain(|&(e, _)| e != ev);
        self.cooldowns.push((ev, until));
    }
    pub fn housing(&self) -> f32 {
        let mut h = 6.0;
        if self.river {
            h += 3.0;
        }
        if self.coastal {
            h += 2.0;
        }
        if self.has("Aqueduct") {
            h += 8.0;
        }
        if self.has("Granary") {
            h += 2.0;
        }
        if self.has("Hospital") {
            h += 6.0;
        }
        if self.has("Arcology") {
            h += 20.0;
        }
        h
    }
    pub fn defense(&self, civ_tier: f32) -> f32 {
        let mut d = 4.0 + self.pop * 0.8;
        if self.has("Walls") {
            d += 8.0;
        }
        if self.has("Castle") {
            d += 12.0;
        }
        d *= civ_tier.sqrt();
        d += self.garrison;
        // maintenance matters: a backlog weakens defenses
        d * (1.0 - (self.backlog / 200.0).clamp(0.0, 0.5))
    }
}
