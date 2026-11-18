#![allow(dead_code)]
//! Simulation root. Pure Rust; the Bevy layer only wraps it in a resource.
//! The yearly tick order is contractual (design doc 13.2).

pub mod ai;
pub mod city;
pub mod civ;
pub mod events;
pub mod names;
pub mod news;
pub mod planet;
pub mod powers;
pub mod scenario;

use city::*;
use civ::*;
use names::Language;
use news::*;
use planet::*;
use rand::Rng;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

pub const MAX_CIVS: usize = 28;

#[derive(Clone, Debug)]
pub struct Tribe {
    pub x: i32,
    pub y: i32,
    pub pop: f32,
    pub name: String,
    pub alive: bool,
}

#[derive(Clone, Debug)]
pub struct Army {
    pub id: u32,
    pub owner: Option<u16>,
    pub x: f32,
    pub y: f32,
    pub strength: f32,
    pub target: Option<u32>,
    pub enemy: Option<u16>,
    pub alive: bool,
    pub loot: f32,
    pub home: Option<u32>,
    pub sieging: bool,
}

#[derive(Clone, Debug)]
pub struct TradeRoute {
    pub a: u32,
    pub b: u32,
    pub value: f32,
    pub sea: bool,
    pub created: i32,
    pub alive: bool,
    pub raided_until: i32,
}

#[derive(Clone, Debug)]
pub struct Species {
    pub name: String,
    pub pref_temp: f32,
    pub aquatic: bool,
    pub nocturnal: bool,
    pub diet: &'static str,
    pub desc: String,
    pub extinct_lineages: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct InterventionWindow {
    pub id: u32,
    pub city: u32,
    pub kind: &'static str,
    pub title: String,
    pub expires: i32,
    pub cost: f32,
    pub resolved: bool,
    pub opened: i32,
    pub pop_at_open: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub years: Vec<i32>,
    pub pop: Vec<f32>,
    pub temp: Vec<f32>,
    pub biodiv: Vec<f32>,
    pub wealth: Vec<f32>,
    pub co2: Vec<f32>,
    pub sea: Vec<f32>,
    pub civ_scores: Vec<Vec<(i32, f32)>>,
    pub markers: Vec<(i32, usize)>,
}

#[derive(Clone, Debug)]
pub struct Review {
    pub year: i32,
    pub lines: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Difficulty {
    Storyteller,
    Steward,
    Challenger,
    HarshPlanet,
}

impl Difficulty {
    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Storyteller => "Storyteller",
            Difficulty::Steward => "Steward",
            Difficulty::Challenger => "Challenger",
            Difficulty::HarshPlanet => "Harsh Planet",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            Difficulty::Storyteller => "Events 30% milder, +50% Gaia Points, calm climate.",
            Difficulty::Steward => "Baseline severity and income.",
            Difficulty::Challenger => "Events 25% harsher, -25% Gaia Points, wider spread of NPC competence.",
            Difficulty::HarshPlanet => "Events 50% harsher, -40% Gaia Points, volatile climate.",
        }
    }
    pub fn severity(self) -> f32 {
        match self {
            Difficulty::Storyteller => 0.7,
            Difficulty::Steward => 1.0,
            Difficulty::Challenger => 1.25,
            Difficulty::HarshPlanet => 1.5,
        }
    }
    pub fn gp(self) -> f32 {
        match self {
            Difficulty::Storyteller => 1.5,
            Difficulty::Steward => 1.0,
            Difficulty::Challenger => 0.75,
            Difficulty::HarshPlanet => 0.6,
        }
    }
}

pub struct Sim {
    pub params: WorldParams,
    pub rng: ChaCha8Rng,
    pub planet: Planet,
    pub year: i32,
    pub civs: Vec<Civ>,
    pub cities: Vec<City>,
    pub tribes: Vec<Tribe>,
    pub armies: Vec<Army>,
    pub routes: Vec<TradeRoute>,
    pub religions: Vec<Religion>,
    pub rel: Vec<Relation>,
    pub seeds: Vec<StorySeed>,
    pub news: NewsLog,
    pub templates: Templates,
    pub gp: f32,
    pub gp_spent: f32,
    pub gp_spent_destructive: f32,
    pub stewardship: f32,
    pub stats: Stats,
    pub species: Species,
    pub windows: Vec<InterventionWindow>,
    pub reviews: Vec<Review>,
    pub alerts: Vec<usize>,
    pub auto_pause_major: bool,
    pub auto_pause_watch: bool,
    pub auto_pause_existential: bool,
    pub difficulty: Difficulty,
    pub mode: scenario::Mode,
    pub scenario: Option<scenario::ScenarioState>,
    pub wonders_built: Vec<Option<(u32, i32)>>,
    pub wonder_race: Vec<(usize, u32, f32)>,
    pub flashes: Vec<(i32, i32, i32, [u8; 3])>,
    pub major_target: usize,
    pub next_army: u32,
    pub next_window: u32,
    pub world_lang: Language,
    pub intervention_log: Vec<(i32, String)>,
    pub extinct_civs: usize,
    pub ever_civs: usize,
    pub last_richest: Option<u32>,
    pub rankings: Vec<(i32, Vec<(u16, f32)>)>,
    pub ended: Option<String>,
    pub cosmic: f32,
    pub fog_of_rumor: bool,
    pub contact_made: bool,
}

pub fn sat(x: f32) -> f32 {
    1000.0 * (1.0 - (-x.max(0.0)).exp())
}

impl Sim {
    pub fn new(params: WorldParams, mode: scenario::Mode, difficulty: Difficulty) -> Sim {
        let mut params = params;
        scenario::adjust_params(&mode, &mut params);
        let mut rng = ChaCha8Rng::seed_from_u64(params.seed);
        let planet = Planet::generate(&params, &mut rng);
        let mut world_lang = Language::generate(&mut rng);
        let species = Self::gen_species(&mut rng, &mut world_lang, &planet);
        let major_target = match params.w {
            0..=70 => 5,
            71..=110 => 8,
            111..=145 => 10,
            _ => 12,
        };
        let mut sim = Sim {
            cosmic: params.cosmic,
            params,
            rng,
            planet,
            year: 0,
            civs: Vec::new(),
            cities: Vec::new(),
            tribes: Vec::new(),
            armies: Vec::new(),
            routes: Vec::new(),
            religions: Vec::new(),
            rel: vec![Relation::default(); MAX_CIVS * MAX_CIVS],
            seeds: Vec::new(),
            news: NewsLog {
                threshold_ticker: 1.0,
                threshold_card: 3.0,
                ..Default::default()
            },
            templates: Templates::load(),
            gp: 40.0,
            gp_spent: 0.0,
            gp_spent_destructive: 0.0,
            stewardship: 70.0,
            stats: Stats::default(),
            species,
            windows: Vec::new(),
            reviews: Vec::new(),
            alerts: Vec::new(),
            auto_pause_major: false,
            auto_pause_watch: false,
            auto_pause_existential: true,
            difficulty,
            mode,
            scenario: None,
            wonders_built: vec![None; WONDERS.len()],
            wonder_race: Vec::new(),
            flashes: Vec::new(),
            major_target,
            next_army: 0,
            next_window: 0,
            world_lang,
            intervention_log: Vec::new(),
            extinct_civs: 0,
            ever_civs: 0,
            last_richest: None,
            rankings: Vec::new(),
            ended: None,
            fog_of_rumor: false,
            contact_made: false,
        };
        sim.spawn_initial_tribes();
        scenario::setup(&mut sim);
        let s = StorySeed::new("dawn", Scope::Global)
            .slot("Species", sim.species.name.clone())
            .slot("SpeciesDesc", sim.species.desc.clone())
            .slot("Tribes", sim.tribes.len().to_string())
            .cause("a stable climate and abundant food let a thinking species spread")
            .cat("cultural")
            .chronicle();
        sim.seeds.push(s);
        sim.render_news();
        sim
    }

    fn gen_species(rng: &mut ChaCha8Rng, lang: &mut Language, planet: &Planet) -> Species {
        let name = lang.word(rng, 2, 3);
        let pref_temp = 14.0 + rng.gen_range(-8.0..8.0) + (planet.global_temp - 14.0) * 0.5;
        let aquatic = rng.gen_bool(0.25);
        let nocturnal = rng.gen_bool(0.3);
        let diet = ["omnivorous", "herbivorous", "fish-eating", "hunting"]
            .choose(rng)
            .copied()
            .unwrap();
        let body = ["six-limbed", "feathered", "furred", "scaled", "long-necked", "tusked"]
            .choose(rng)
            .unwrap();
        let desc = format!(
            "{} {} {}{}{}, most at home near {:.0}°C",
            if aquatic { "amphibious" } else { "upright" },
            body,
            diet,
            if nocturnal { ", night-active" } else { "" },
            " foragers",
            pref_temp
        );
        let mut extinct = Vec::new();
        for _ in 0..rng.gen_range(3..6) {
            extinct.push(format!(
                "{} ({})",
                lang.word(rng, 2, 3),
                ["giant grazer", "armored swimmer", "glider", "burrower", "apex hunter", "reef builder"]
                    .choose(rng)
                    .unwrap()
            ));
        }
        Species { name, pref_temp, aquatic, nocturnal, diet, desc, extinct_lineages: extinct }
    }

    pub fn habitability(&self, i: usize) -> f32 {
        let t = &self.planet.tiles[i];
        if !self.planet.is_land(i) || matches!(t.biome, Biome::Ice | Biome::Mountain) {
            return 0.0;
        }
        let dt = (t.temp - self.species.pref_temp).abs();
        let temp_ok = (1.0 - dt / 22.0).clamp(0.0, 1.0);
        let mut h = t.fert * 0.7 + temp_ok * 0.3;
        if t.is_river() {
            h += 0.2;
        }
        if self.species.aquatic && self.planet.is_coastal(i) {
            h += 0.15;
        }
        h * temp_ok.sqrt()
    }

    fn spawn_initial_tribes(&mut self) {
        let n = self.planet.tiles.len();
        let mut cands: Vec<(usize, f32)> = (0..n)
            .map(|i| (i, self.habitability(i)))
            .filter(|&(_, h)| h > 0.35)
            .collect();
        cands.shuffle(&mut self.rng);
        cands.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let want = self.major_target * 2 + 4;
        let mut placed: Vec<(i32, i32)> = Vec::new();
        for (i, _) in cands {
            if self.tribes.len() >= want {
                break;
            }
            let (x, y) = self.planet.xy(i);
            if placed.iter().any(|&(px, py)| self.planet.dist2(x, y, px, py) < 36.0) {
                continue;
            }
            placed.push((x, y));
            let name = self.world_lang.word(&mut self.rng, 1, 2);
            self.tribes.push(Tribe { x, y, pop: self.rng.gen_range(8.0..22.0), name, alive: true });
        }
    }

    // ------------------------------------------------------------------ helpers
    pub fn r(&self, a: u16, b: u16) -> &Relation {
        &self.rel[a as usize * MAX_CIVS + b as usize]
    }
    pub fn rm(&mut self, a: u16, b: u16) -> &mut Relation {
        &mut self.rel[a as usize * MAX_CIVS + b as usize]
    }
    /// Opinion of `a` toward `b` gets a ledger line.
    pub fn ledger(&mut self, a: u16, b: u16, reason: Reason, value: f32, years: Option<i32>, note: impl Into<String>) {
        if a == b {
            return;
        }
        let year = self.year;
        let vind = self.civs[a as usize].leader.traits[T_VINDICTIVE];
        let years = years.map(|y| if value < 0.0 { (y as f32 * (0.5 + vind)) as i32 } else { y });
        let r = self.rm(a, b);
        let note = note.into();
        if let Some(e) = r.ledger.iter_mut().find(|e| e.reason == reason && e.note == note) {
            e.value = (e.value + value * 0.5).clamp(-80.0, 80.0);
            e.expires = years.map(|y| year + y);
        } else {
            r.ledger.push(LedgerEntry { reason, value, expires: years.map(|y| year + y), note });
        }
    }
    pub fn set_ledger(&mut self, a: u16, b: u16, reason: Reason, value: f32, note: &str) {
        let r = self.rm(a, b);
        r.ledger.retain(|e| e.reason != reason);
        if value.abs() > 0.5 {
            r.ledger.push(LedgerEntry { reason, value, expires: None, note: note.to_string() });
        }
    }
    pub fn city_era(&self, c: u32) -> usize {
        self.civs[self.cities[c as usize].civ as usize].era
    }
    pub fn civ_pop(&self, c: u16) -> f32 {
        self.civs[c as usize]
            .cities
            .iter()
            .map(|&ci| self.cities[ci as usize].pop)
            .sum()
    }
    pub fn civ_wealth(&self, c: u16) -> (f32, Option<u32>, Option<u32>) {
        let civ = &self.civs[c as usize];
        let mut tot = 0.0;
        let mut wsum = 0.0;
        let mut rich: Option<u32> = None;
        let mut poor: Option<u32> = None;
        for &ci in &civ.cities {
            let city = &self.cities[ci as usize];
            tot += city.pop;
            wsum += city.wealth * city.pop;
            if rich.map(|r| self.cities[r as usize].wealth < city.wealth).unwrap_or(true) {
                rich = Some(ci);
            }
            if poor.map(|r| self.cities[r as usize].wealth > city.wealth).unwrap_or(true) {
                poor = Some(ci);
            }
        }
        (if tot > 0.0 { wsum / tot } else { 0.0 }, rich, poor)
    }
    pub fn alive_civs(&self) -> impl Iterator<Item = &Civ> {
        self.civs.iter().filter(|c| c.alive)
    }
    pub fn flash(&mut self, x: i32, y: i32, color: [u8; 3]) {
        self.flashes.push((x, y, self.year, color));
    }
    pub fn civ_name(&self, c: u16) -> String {
        self.civs[c as usize].name.clone()
    }
    pub fn city_name(&self, c: u32) -> String {
        self.cities[c as usize].name.clone()
    }
    pub fn site_desc(&self, i: usize) -> String {
        let t = &self.planet.tiles[i];
        let mut parts = Vec::new();
        if t.is_river() {
            parts.push("a river bend".to_string());
        }
        if self.planet.is_coastal(i) {
            parts.push("a sheltered coast".to_string());
        }
        if parts.is_empty() {
            parts.push(format!("the {}", t.biome.name().to_lowercase()));
        }
        parts.join(" on ")
    }
    pub fn region_name(&self, x: i32, y: i32) -> String {
        let l = lat(y as usize, self.planet.h);
        let ns = if l < -0.33 {
            "northern "
        } else if l > 0.33 {
            "southern "
        } else {
            ""
        };
        let i = self.planet.idx(x, y).unwrap();
        format!("the {}{}", ns, self.planet.tiles[i].biome.name().to_lowercase())
    }

    pub fn found_civ(&mut self, x: i32, y: i32, minor: bool, parent: Option<u16>, inherited: Option<[f32; 8]>) -> Option<u16> {
        if self.civs.len() >= MAX_CIVS {
            return None;
        }
        let id = self.civs.len() as u16;
        let mut lang = Language::generate(&mut self.rng);
        let name = lang.word(&mut self.rng, 2, 3);
        let demonym = lang.demonym(&name);
        let hue = (id as f32 * 0.618034 * 360.0 + self.rng.gen_range(0.0..20.0)) % 360.0;
        let color = hsv(hue, if minor { 0.35 } else { 0.75 }, if minor { 0.85 } else { 0.95 });
        let i = self.planet.idx(x, y).unwrap();
        let t = &self.planet.tiles[i];
        let homeland = if t.is_river() {
            "river"
        } else if self.planet.is_coastal(i) {
            "coast"
        } else {
            match t.biome {
                Biome::Desert | Biome::Shrubland => "desert",
                Biome::Rainforest => "jungle",
                Biome::TemperateForest | Biome::Taiga => "forest",
                Biome::Grassland | Biome::Savanna => "grass",
                Biome::Tundra => "cold",
                _ => "any",
            }
        };
        let mut base = inherited.unwrap_or([0.5; 8]);
        if inherited.is_none() {
            // homeland shapes temperament
            match homeland {
                "coast" => base[T_GREED] += 0.15,
                "desert" => base[T_PIETY] += 0.15,
                "grass" => base[T_AGGRESSION] += 0.15,
                "forest" => base[T_ECOLOGY] += 0.2,
                "river" => base[T_EXPANSION] += 0.1,
                "cold" => base[T_VINDICTIVE] += 0.1,
                _ => {}
            }
        }
        let leader = gen_leader(&mut lang, &mut self.rng, Some(&base), self.year, 0);
        let pool: Vec<&(&str, &str, &str)> = ABILITIES
            .iter()
            .filter(|a| a.1 == homeland || a.1 == "any")
            .collect();
        let ab = pool.choose(&mut self.rng).copied().unwrap_or(&ABILITIES[8]);
        let mut known_tech = vec![false; TECHS.len()];
        known_tech[0] = true;
        let mut era = 0;
        if let Some(p) = parent {
            let pc = &self.civs[p as usize];
            for k in 0..TECHS.len() {
                known_tech[k] = pc.known_tech[k];
            }
            era = pc.era;
        }
        let av = self.rng.gen_range(0..AGENDAS.len());
        let mut ah = self.rng.gen_range(0..AGENDAS.len());
        if ah == av {
            ah = (ah + 3) % AGENDAS.len();
        }
        let minor_kind = if minor {
            ["cultural", "trade", "scientific", "religious", "militaristic", "ecological"]
                .choose(&mut self.rng)
                .copied()
                .unwrap()
        } else {
            ""
        };
        let civ = Civ {
            id,
            name,
            demonym,
            color,
            lang,
            leader,
            leaders_past: Vec::new(),
            alive: true,
            minor,
            minor_kind,
            suzerain: None,
            founded: self.year,
            fell: None,
            capital: None,
            cities: Vec::new(),
            treasury: 20.0,
            gold_income: 0.0,
            known_tech,
            known_civic: vec![false; CIVICS.len()],
            research: None,
            research_progress: 0.0,
            civic_research: None,
            civic_progress: 0.0,
            eureka: vec![false; TECHS.len()],
            era,
            government: 0,
            gov_change_year: self.year,
            religion: None,
            strategy: Strategy::Expand,
            agenda_visible: av,
            agenda_hidden: ah,
            agenda_revealed: false,
            ability: ab.0,
            ability_desc: ab.2,
            war_weariness: 0.0,
            stability: 60.0,
            military: 0.0,
            score: 0.0,
            score_parts: [0.0; 8],
            settler_progress: 0.0,
            homeland_biome: homeland,
            famine_memory: 0.0,
            betrayed_memory: 0.0,
            peak_pop: 0.0,
            pop_history: Vec::new(),
            contacted_continents: Vec::new(),
            spy_progress: 0.0,
            defining_moments: Vec::new(),
            parent,
            wonders: Vec::new(),
        };
        self.civs.push(civ);
        self.ever_civs += 1;
        self.stats.civ_scores.push(Vec::new());
        Some(id)
    }

    pub fn found_city(&mut self, civ: u16, x: i32, y: i32) -> u32 {
        let id = self.cities.len() as u32;
        let name = {
            let c = &mut self.civs[civ as usize];
            c.lang.word(&mut self.rng, 1, 3)
        };
        let i = self.planet.idx(x, y).unwrap();
        let coastal = self.planet.is_coastal(i);
        let river = self.planet.tiles[i].is_river()
            || self.planet.neighbors8(i).any(|n| self.planet.tiles[n].is_river());
        let gov_name = self.civs[civ as usize].lang.person(&mut self.rng);
        let comp_spread: f32 = match self.difficulty {
            Difficulty::Challenger | Difficulty::HarshPlanet => 0.45,
            _ => 0.3,
        };
        let governor = Governor {
            name: gov_name,
            competence: (0.55 + self.rng.gen_range(-comp_spread..comp_spread)).clamp(0.05, 1.0),
            corruption: self.rng.gen_range(0.0..0.35),
        };
        let capital = self.civs[civ as usize].capital.is_none();
        let city = City {
            id,
            civ,
            orig_civ: civ,
            name,
            x,
            y,
            founded: self.year,
            alive: true,
            pop: 2.0,
            food_store: 5.0,
            buildings: vec![false; BUILDINGS.len()],
            wonders: Vec::new(),
            build: None,
            build_progress: 0.0,
            spec: Spec::None,
            governor,
            backlog: 0.0,
            stability: 65.0,
            health: 70.0,
            wealth: 200.0,
            wealth_parts: [0.0; 6],
            wealth_hist: Vec::new(),
            yields: Yields::default(),
            culture: 0.0,
            faith_acc: 0.0,
            religion: self.civs[civ as usize].religion,
            conv_pressure: 0.0,
            relic: false,
            is_capital: capital,
            coastal,
            river,
            radius: 1,
            trade_income: 0.0,
            cooldowns: Vec::new(),
            chains: Chains::default(),
            gazette: Vec::new(),
            famine_years: 0,
            plague_severity: 0.0,
            protected_until: 0,
            siege: 0.0,
            garrison: 2.0,
            loyalty: 100.0,
            golden_until: 0,
            catchment: Vec::new(),
            resources: Vec::new(),
            unrest: 0.0,
            migration: 0.0,
            suspected_spy: None,
            debt: 0.0,
            debt_lender: None,
            pop_history: Vec::new(),
            stranded: false,
        };
        self.cities.push(city);
        self.planet.tiles[i].city_slot = Some(id);
        let c = &mut self.civs[civ as usize];
        c.cities.push(id);
        if capital {
            c.capital = Some(id);
        }
        // claim immediate tiles
        for t in self.planet.tiles_in_radius(x, y, 1) {
            if self.planet.tiles[t].owner.is_none() {
                self.planet.tiles[t].owner = Some(civ);
            }
        }
        self.planet.tiles[i].owner = Some(civ);
        self.update_catchment(id);
        id
    }

    pub fn update_catchment(&mut self, ci: u32) {
        let c = &self.cities[ci as usize];
        let tiles: Vec<usize> = self
            .planet
            .tiles_in_radius(c.x, c.y, c.radius.max(1))
            .into_iter()
            .filter(|&t| self.planet.tiles[t].owner == Some(c.civ))
            .collect();
        let mut res = Vec::new();
        let civ = &self.civs[c.civ as usize];
        for &t in &tiles {
            if let Some(r) = self.planet.tiles[t].resource {
                let visible = r.kind.revealed_by().map(|k| civ.known_tech[k]).unwrap_or(true);
                if visible && r.stock > 0.0 && !res.contains(&r.kind) {
                    res.push(r.kind);
                }
            }
        }
        let c = &mut self.cities[ci as usize];
        c.catchment = tiles;
        c.resources = res;
    }

    // ------------------------------------------------------------------ the year
    pub fn tick(&mut self) {
        if self.ended.is_some() && !matches!(self.mode, scenario::Mode::Sandbox | scenario::Mode::Chronicle) {
            return;
        }
        self.year += 1;
        // 1. climate and geology
        scenario::physics(self);
        self.planet.year_update(self.year);
        // 2. harvests, regeneration, trade route movement and profits
        self.update_trade_income();
        // 3. city yields, wealth, growth, health, stability
        self.update_cities();
        // 4. civilization research, civics, government, strategy; tribes and emergence
        self.update_tribes();
        self.update_civs();
        // 5. diplomacy
        self.update_diplomacy();
        // 6. units, combat, espionage
        self.update_armies();
        self.update_espionage();
        // 7. events, chains, NPC responses
        self.run_events();
        // 8. news from all seeds emitted this year
        self.render_news();
        // 9. rankings, decadal review, chronicle, stats
        self.end_of_year();
    }

    fn update_trade_income(&mut self) {
        for c in self.cities.iter_mut() {
            c.trade_income = 0.0;
        }
        let year = self.year;
        for r in 0..self.routes.len() {
            let rt = self.routes[r].clone();
            if !rt.alive {
                continue;
            }
            let (a, b) = (rt.a as usize, rt.b as usize);
            if !self.cities[a].alive || !self.cities[b].alive {
                self.routes[r].alive = false;
                continue;
            }
            let (ca, cb) = (self.cities[a].civ, self.cities[b].civ);
            if ca != cb && self.r(ca, cb).at_war {
                self.routes[r].alive = false;
                continue;
            }
            if rt.raided_until > year {
                continue;
            }
            let mut v = rt.value;
            for (ci, civ) in [(a, ca), (b, cb)] {
                let bonus = if self.civs[civ as usize].ability == "Caravan Kings" { 1.3 } else { 1.0 };
                self.cities[ci].trade_income += v * bonus;
            }
            // trade drags culture and technology contact
            if ca != cb {
                self.rm(ca, cb).met = true;
                self.rm(cb, ca).met = true;
                v *= 0.15;
                self.set_ledger(ca, cb, Reason::Trade, v.min(20.0), "trade partners");
                self.set_ledger(cb, ca, Reason::Trade, v.min(20.0), "trade partners");
            }
        }
    }

    fn city_tile_yield(&self, civ: &Civ, city: &City, t: usize) -> (f32, f32, f32) {
        let tile = &self.planet.tiles[t];
        let land = self.planet.is_land(t);
        let hgt = self.planet.height(t);
        let (mut f, mut p, mut g) = (0.0, 0.0, 0.0);
        if land {
            f += tile.fert * 2.3 + tile.game * tile.forest * 0.35;
            p += tile.forest * 0.6 + if hgt > 0.32 { 0.9 } else { 0.1 };
            if tile.is_river() {
                g += 0.35;
                if civ.ability == "Floodplain Masters" {
                    f += 0.6;
                }
            }
            match tile.improvement {
                Improvement::Farm => {
                    f += 0.6 + tile.fert * if civ.knows("Irrigation") { 1.0 } else { 0.5 };
                    if civ.knows("Feudal Farming") {
                        f += 0.4;
                    }
                    if civ.knows("Industrialization") {
                        f += 0.8;
                    }
                }
                Improvement::Mine | Improvement::Quarry => p += 1.2,
                Improvement::Plantation | Improvement::Pasture => {
                    g += 0.6;
                    f += 0.3
                }
                Improvement::Park => g += 0.2,
                _ => {}
            }
            if let Some(r) = tile.resource {
                let visible = r.kind.revealed_by().map(|k| civ.known_tech[k]).unwrap_or(true);
                if visible && r.stock > 0.0 {
                    match r.kind.class() {
                        ResClass::Luxury => g += 1.4 * r.richness,
                        ResClass::Strategic => p += 1.2 * r.richness,
                        _ => p += 0.6 * r.richness,
                    }
                }
            }
            if civ.ability == "Mountain Halls" && hgt > 0.45 {
                p += 0.5;
            }
            if civ.ability == "Frost-Hardened" && tile.temp < 3.0 {
                f += 0.4;
            }
        } else if tile.biome != Biome::SeaIce {
            let shallow = tile.biome == Biome::Shallow;
            f += tile.fish * if shallow { 1.3 } else { 0.6 } * if self.planet.moons > 0 { 1.1 } else { 1.0 };
            g += 0.2;
            if city.has("Harbor") {
                f += 0.6;
                g += 0.3;
            }
            if civ.ability == "Tidecallers" {
                f += 0.3;
            }
        }
        (f.max(0.0), p.max(0.0), g)
    }

    fn update_cities(&mut self) {
        let year = self.year;
        let n = self.cities.len();
        for ci in 0..n {
            if !self.cities[ci].alive {
                continue;
            }
            if year % 3 == ci as i32 % 3 {
                self.update_catchment(ci as u32);
            }
            let civ_id = self.cities[ci].civ;
            let civ = &self.civs[civ_id as usize];
            let city = &self.cities[ci];
            // pick the best worked tiles
            let mut scored: Vec<(usize, f32, f32, f32)> = city
                .catchment
                .iter()
                .map(|&t| {
                    let (f, p, g) = self.city_tile_yield(civ, city, t);
                    (t, f, p, g)
                })
                .collect();
            scored.sort_by(|a, b| (b.1 * 1.2 + b.2 + b.3).partial_cmp(&(a.1 * 1.2 + a.2 + a.3)).unwrap());
            let worked = (city.pop.ceil() as usize + 1).min(scored.len());
            let (mut food, mut prod, mut gold) = (2.0f32, 1.0f32, 1.0f32);
            let mut worked_tiles = Vec::new();
            for s in scored.iter().take(worked) {
                food += s.1;
                prod += s.2;
                gold += s.3;
                worked_tiles.push(s.0);
            }
            let (ty, _) = (city.y as usize, 0);
            // harvest variability from seasons (tilt) and drought chains
            let var = self.planet.season_variability(ty);
            let harvest = (1.0 + self.rng.gen_range(-var..var * 0.6)) * if city.chains.drought > 0 { 0.6 } else { 1.0 };
            food *= harvest;
            let mut sci = 1.0 + city.pop * 0.45;
            let mut cul = 1.0;
            let mut fai = 0.0;
            if city.has("Granary") {
                food += 1.5;
            }
            if city.has("Monument") {
                cul += 2.0;
            }
            if city.has("Shrine") {
                fai += 1.5;
            }
            if city.has("Temple") {
                fai += 3.0;
                cul += 1.0;
            }
            if city.has("Library") {
                sci = sci * 1.25 + 2.0;
            }
            if city.has("University") {
                sci *= 1.4;
            }
            if city.has("Research Lab") {
                sci *= 1.5;
            }
            if city.has("Workshop") {
                prod *= 1.25;
            }
            if city.has("Factory") {
                prod *= 1.5;
            }
            if city.has("Power Plant") {
                prod *= 1.3;
            }
            if city.has("Market") {
                gold *= 1.25;
            }
            if city.has("Bank") {
                gold *= 1.3;
            }
            if city.has("Stock Exchange") {
                gold *= 1.4;
            }
            if city.has("Nature Reserve") {
                cul += 3.0;
            }
            if city.relic {
                fai += 4.0;
                gold += 2.0;
            }
            if civ.ability == "Star Readers" {
                sci *= 1.15;
            }
            if civ.ability == "Ancestor Keepers" {
                fai += 1.0;
            }
            if civ.ability == "Forest Wardens" {
                cul += worked_tiles.iter().map(|&t| self.planet.tiles[t].forest).sum::<f32>() * 0.3;
            }
            if civ.ability == "Spice Lords" {
                gold += city.resources.iter().filter(|r| r.class() == ResClass::Luxury).count() as f32;
            }
            if let Some(rid) = city.religion {
                if self.religions[rid].holy_city == city.id {
                    fai += 3.0;
                    gold += 2.0; // pilgrimage
                }
            }
            let wonder_count = city.wonders.len() as f32;
            cul += wonder_count * 3.0;
            gold += wonder_count * 1.5;
            if city.golden_until > year {
                food *= 1.1;
                prod *= 1.15;
                gold *= 1.2;
                cul *= 1.3;
            }
            let gov = civ.gov();
            gold += city.trade_income;
            gold *= gov.tax;
            sci *= gov.science;
            fai *= gov.faith;
            let corrupt = city.governor.corruption * (gov.corruption + 0.05) * 2.0;
            let diverted = gold * corrupt.min(0.4);
            gold -= diverted;
            if city.plague_severity > 0.0 {
                prod *= 1.0 - city.plague_severity * 0.4;
            }
            if city.siege > 0.0 {
                food *= 0.5;
                gold *= 0.5;
            }
            let yields = Yields { food, prod, gold, science: sci, culture: cul, faith: fai };
            // improvements and environmental pressure on worked tiles
            let eco = civ.leader.traits[T_ECOLOGY];
            let industrial = city.has("Factory") || city.has("Power Plant");
            let knows_mining = civ.knows("Mining");
            let warden = civ.ability == "Forest Wardens";
            for &t in &worked_tiles {
                let tile = &mut self.planet.tiles[t];
                let land = tile.elev > self.planet.sea_level;
                if land && tile.improvement == Improvement::None && self.rng.gen_bool(0.08) {
                    let hgt = tile.elev - self.planet.sea_level;
                    if let Some(r) = tile.resource {
                        tile.improvement = match r.kind.class() {
                            ResClass::Luxury if !matches!(r.kind, ResKind::Gold | ResKind::Gems) => Improvement::Plantation,
                            ResClass::Biological => Improvement::Pasture,
                            _ if knows_mining => Improvement::Mine,
                            _ => Improvement::None,
                        };
                    } else if hgt > 0.32 && knows_mining {
                        tile.improvement = Improvement::Mine;
                    } else if tile.fert > 0.3 {
                        let cut_chance = (1.0 - eco) * if warden { 0.4 } else { 1.0 };
                        if tile.forest < 0.3 || self.rng.gen_bool(cut_chance.clamp(0.05, 1.0) as f64) {
                            tile.improvement = Improvement::Farm;
                        }
                    }
                }
                match tile.improvement {
                    Improvement::Farm => {
                        tile.forest = tile.forest.min(0.1);
                        tile.soil = (tile.soil - 0.006).max(0.4);
                    }
                    Improvement::Mine => {
                        if let Some(r) = tile.resource.as_mut() {
                            r.stock -= 1.0;
                        }
                    }
                    _ => {}
                }
                if land {
                    tile.forest = (tile.forest - 0.004 * (1.2 - eco)).max(0.0);
                    tile.game = (tile.game - 0.03).max(0.1);
                } else {
                    tile.fish = (tile.fish - 0.035 * (1.0 + city_pop_factor(self.cities[ci].pop))).max(0.0);
                }
                if industrial {
                    tile.pollution = (tile.pollution + 0.01 * (1.3 - eco)).min(1.0);
                }
            }
            if industrial {
                let i = self.planet.idx(self.cities[ci].x, self.cities[ci].y).unwrap();
                self.planet.tiles[i].pollution = (self.planet.tiles[i].pollution + 0.03).min(1.0);
                self.planet.co2 += 0.0004 * (1.3 - eco) * self.cities[ci].pop / 10.0;
            }
            // growth
            let city = &mut self.cities[ci];
            city.yields = yields;
            let consumption = city.pop * 1.0;
            let surplus = food - consumption;
            let granary_cap = city.pop * 2.0 + if city.has("Granary") { 15.0 } else { 5.0 };
            city.food_store = (city.food_store + surplus).min(granary_cap);
            let housing = city.housing();
            if city.food_store < 0.0 {
                let loss = (-city.food_store * 0.25).min(city.pop * 0.15);
                city.pop = (city.pop - loss).max(1.0);
                city.food_store = 0.0;
                city.famine_years = city.famine_years.saturating_add(1);
            } else {
                city.famine_years = 0;
                let room = if city.pop < housing { 1.0 } else { 0.15 };
                let g = (surplus.max(0.0) * 0.055).min(city.pop * 0.05) * room;
                city.pop += g;
            }
            city.pop = city.pop.min(80.0);
            // health
            let mut health = 68.0;
            if city.has("Aqueduct") {
                health += 10.0;
            }
            if city.has("Hospital") {
                health += 15.0;
            }
            if city.river || city.coastal {
                health += 4.0;
            }
            health -= ((city.pop - housing) * 2.0).max(0.0);
            health -= city.plague_severity * 40.0;
            let i = (city.y as usize) * self.planet.w + city.x as usize;
            health -= self.planet.tiles[i].pollution * 30.0;
            city.health = health.clamp(0.0, 100.0);
            // culture, faith and border radius
            city.culture += cul;
            city.faith_acc += fai;
            let max_r = 2 + (civ_era_radius(self.civs[civ_id as usize].era)) as i32;
            city.radius = (1.0 + (city.culture / 70.0).sqrt()).min(max_r as f32) as i32;
            // build queue progress
            city.build_progress += prod;
            city.siege = (city.siege - 0.5).max(0.0);
            city.plague_severity = (city.plague_severity - 0.08).max(0.0);
            city.unrest = (city.unrest * 0.9).max(0.0);
            if year % 1 == 0 {
                city.pop_history.push(city.pop);
                if city.pop_history.len() > 400 {
                    city.pop_history.remove(0);
                }
            }
        }
        // treasury and upkeep at civ level, then stability and wealth
        for c in 0..self.civs.len() {
            if !self.civs[c].alive {
                continue;
            }
            let mut income = 0.0;
            let mut upkeep = 0.0;
            for &ci in &self.civs[c].cities {
                let city = &self.cities[ci as usize];
                income += city.yields.gold;
                for (k, b) in BUILDINGS.iter().enumerate() {
                    if city.buildings[k] {
                        upkeep += b.upkeep;
                    }
                }
            }
            let army_upkeep: f32 = self
                .armies
                .iter()
                .filter(|a| a.alive && a.owner == Some(c as u16))
                .map(|a| a.strength * 0.08)
                .sum();
            let net = income - upkeep - army_upkeep;
            let civ = &mut self.civs[c];
            civ.gold_income = net;
            civ.treasury += net;
            let short = civ.treasury < 0.0;
            if short {
                civ.treasury = civ.treasury.max(-200.0);
            }
            let cities = civ.cities.clone();
            for ci in cities {
                let city = &mut self.cities[ci as usize];
                let competence = city.governor.competence;
                if short {
                    city.backlog += 3.0 + upkeep * 0.1;
                } else {
                    city.backlog = (city.backlog - 2.0 * competence - 0.5).max(0.0);
                }
                city.backlog += (1.0 - competence) * 0.4;
            }
        }
        self.compute_stability_and_wealth();
    }

    pub fn compute_stability_and_wealth(&mut self) {
        let year = self.year;
        let n = self.cities.len();
        // civ population and treasury shares
        let mut civ_pop = vec![0.0f32; self.civs.len()];
        for c in &self.cities {
            if c.alive {
                civ_pop[c.civ as usize] += c.pop;
            }
        }
        // traded resources per civ (via routes)
        let mut traded: Vec<Vec<ResKind>> = vec![Vec::new(); self.civs.len()];
        let mut route_value = vec![0.0f32; n];
        for r in &self.routes {
            if !r.alive {
                continue;
            }
            route_value[r.a as usize] += r.value;
            route_value[r.b as usize] += r.value;
            let (ca, cb) = (self.cities[r.a as usize].civ as usize, self.cities[r.b as usize].civ as usize);
            for k in self.cities[r.b as usize].resources.clone() {
                if !traded[ca].contains(&k) {
                    traded[ca].push(k);
                }
            }
            for k in self.cities[r.a as usize].resources.clone() {
                if !traded[cb].contains(&k) {
                    traded[cb].push(k);
                }
            }
        }
        for ci in 0..n {
            if !self.cities[ci].alive {
                continue;
            }
            let civ_id = self.cities[ci].civ as usize;
            let civ = &self.civs[civ_id];
            let era = civ.era;
            let gov_stab = civ.gov().stability;
            let war = civ.war_weariness;
            let treasury = civ.treasury;
            let civ_rel = civ.religion;
            let ef = 1.0 + era as f32 * 0.9;
            let city = &self.cities[ci];
            // ---- stability
            let mut st = 68.0 + gov_stab - (war * 0.5).min(20.0) - city.unrest.min(30.0);
            if city.golden_until > year {
                st += 12.0;
            }
            if city.famine_years > 0 {
                st -= 10.0 + city.famine_years as f32 * 4.0;
            }
            if city.religion.is_some() && city.religion != civ_rel && civ_rel.is_some() {
                st -= 6.0;
            }
            if city.civ != city.orig_civ {
                st -= 8.0;
            }
            st -= city.plague_severity * 20.0;
            st -= (city.backlog / 10.0).min(15.0);
            st += (city.health - 60.0) * 0.15;
            if treasury < 0.0 {
                st -= 8.0;
            }
            if civ_id < self.civs.len() && civ.ability == "Ancestor Keepers" {
                st += 3.0;
            }
            let stability = st.clamp(0.0, 100.0);
            // ---- wealth (design doc 5.3)
            let share = treasury.max(0.0) * city.pop / civ_pop[civ_id].max(1.0);
            let per_cap = (city.yields.gold + share.sqrt() * 0.5) / city.pop.max(1.0);
            let c0 = sat(per_cap / (5.5 * ef));
            let mult = 1.0 + if city.has("Harbor") { 0.25 } else { 0.0 } + if city.has("Market") { 0.25 } else { 0.0 };
            let c1 = sat(route_value[ci] * mult / (45.0 * ef));
            let mut nres = city
                .resources
                .iter()
                .filter(|r| matches!(r.class(), ResClass::Luxury | ResClass::Strategic))
                .count() as f32;
            nres += traded[civ_id]
                .iter()
                .filter(|r| !city.resources.contains(r) && matches!(r.class(), ResClass::Luxury | ResClass::Strategic))
                .count() as f32
                * 0.4;
            let c2 = sat(nres / 3.0);
            let mut bval: f32 = BUILDINGS
                .iter()
                .enumerate()
                .filter(|(k, _)| city.buildings[*k])
                .map(|(_, b)| b.value)
                .sum();
            bval += city.wonders.len() as f32 * 60.0;
            let c3 = sat(bval / (55.0 * ef));
            let housing = city.housing();
            let food_sec = (city.food_store / (city.pop * 2.0 + 1.0)).clamp(0.0, 1.0);
            let headroom = ((housing - city.pop) / housing).clamp(0.0, 1.0);
            let employ = (city.yields.prod / city.pop.max(1.0) / 1.2).clamp(0.0, 1.0);
            let c4 = (food_sec * 0.4 + headroom * 0.3 + employ * 0.3) * 1000.0;
            let c5 = stability * 10.0;
            let parts = [c0 * 0.30, c1 * 0.25, c2 * 0.15, c3 * 0.15, c4 * 0.10, c5 * 0.05];
            let mut w: f32 = parts.iter().sum();
            w -= city.debt.min(200.0) * 0.5;
            let city = &mut self.cities[ci];
            city.stability = stability;
            city.wealth_parts = parts;
            city.wealth = w.clamp(0.0, 1000.0);
            city.wealth_hist.push(city.wealth);
            if city.wealth_hist.len() > 400 {
                city.wealth_hist.remove(0);
            }
            // specialization emerges from tiles + buildings
            let y = city.yields;
            city.spec = if city.has("Castle") || (city.has("Walls") && city.garrison > 8.0) {
                Spec::Fortress
            } else if city.has("Bank") || city.has("Stock Exchange") {
                Spec::Financial
            } else if city.religion.map(|r| self.religions[r].holy_city == city.id).unwrap_or(false) || y.faith > y.science.max(4.0) {
                Spec::Holy
            } else if city.has("Factory") || y.prod > y.food * 1.1 {
                Spec::Industrial
            } else if city.coastal && city.has("Harbor") {
                Spec::Port
            } else if y.science > y.food * 0.8 && city.has("Library") {
                Spec::Scholarly
            } else if city.wonders.len() > 1 || city.has("Nature Reserve") {
                Spec::Resort
            } else if y.food > city.pop * 1.3 {
                Spec::Breadbasket
            } else {
                Spec::None
            };
        }
    }

    pub fn end_of_year(&mut self) {
        let year = self.year;
        // GP income
        let alive = self.alive_civs().filter(|c| !c.minor).count() as f32;
        let inc = (1.0 + self.planet.biodiv_index * 2.5 + alive * 0.08) * self.difficulty.gp();
        self.gp = (self.gp + inc * scenario::gp_scale(self)).min(999.0);
        // intervention windows: restraint reward
        let mut rewards = Vec::new();
        for w in self.windows.iter_mut() {
            if !w.resolved && w.expires <= year {
                w.resolved = true;
                rewards.push((w.city, w.pop_at_open, w.title.clone()));
            }
        }
        for (c, p0, title) in rewards {
            let city = &self.cities[c as usize];
            if city.alive && city.pop > p0 * 0.75 {
                self.gp += 12.0;
                let s = StorySeed::new("restraint", Scope::Local)
                    .slot("City", city.name.clone())
                    .slot("Crisis", title)
                    .cause(format!("{} weathered the crisis by its own means", city.name))
                    .city(c)
                    .cat("stewardship");
                self.seeds.push(s);
                self.render_news();
            }
        }
        // stewardship
        let survival = if self.ever_civs > 0 {
            1.0 - self.extinct_civs as f32 / self.ever_civs as f32
        } else {
            1.0
        };
        let proportion = 1.0 - (self.gp_spent_destructive / (self.gp_spent + 50.0)).min(1.0);
        let target = self.planet.biodiv_index * 100.0 * 0.4 + survival * 100.0 * 0.35 + proportion * 100.0 * 0.25;
        self.stewardship += (target - self.stewardship) * 0.05;
        // scores
        for c in 0..self.civs.len() {
            if !self.civs[c].alive {
                continue;
            }
            let pop = self.civ_pop(c as u16);
            let (w, _, _) = self.civ_wealth(c as u16);
            let territory = self.civs[c].cities.iter().map(|&ci| self.cities[ci as usize].catchment.len()).sum::<usize>() as f32;
            let tech = self.civs[c].known_tech.iter().filter(|k| **k).count() as f32;
            let culture: f32 = self.civs[c].cities.iter().map(|&ci| self.cities[ci as usize].culture).sum();
            let mil = self.civs[c].military;
            let parts = [
                territory * 0.5,
                self.civs[c].cities.len() as f32 * 6.0,
                pop * 1.5,
                tech * 5.0,
                (culture / 50.0).sqrt() * 4.0,
                w / 20.0,
                mil * 0.4,
                self.civs[c].wonders.len() as f32 * 15.0,
            ];
            let civ = &mut self.civs[c];
            civ.score_parts = parts;
            civ.score = parts.iter().sum();
            civ.peak_pop = civ.peak_pop.max(pop);
            civ.pop_history.push(pop);
            if civ.pop_history.len() > 400 {
                civ.pop_history.remove(0);
            }
            let sc = civ.score;
            if year % 5 == 0 {
                self.stats.civ_scores[c].push((year, sc));
            }
        }
        // stats
        let tot_pop: f32 = self.cities.iter().filter(|c| c.alive).map(|c| c.pop).sum();
        let (mut ws, mut wn) = (0.0, 0.0);
        for c in self.cities.iter().filter(|c| c.alive) {
            ws += c.wealth * c.pop;
            wn += c.pop;
        }
        self.stats.years.push(year);
        self.stats.pop.push(tot_pop);
        self.stats.temp.push(self.planet.global_temp);
        self.stats.biodiv.push(self.planet.biodiv_index);
        self.stats.wealth.push(if wn > 0.0 { ws / wn } else { 0.0 });
        self.stats.co2.push(self.planet.co2);
        self.stats.sea.push(self.planet.sea_level);
        // rankings each 50 years
        if year % 50 == 0 && self.alive_civs().filter(|c| !c.minor).count() > 1 {
            self.publish_rankings();
        }
        if year % 10 == 0 {
            self.decadal_review();
        }
        self.flashes.retain(|f| year - f.2 < 4);
        scenario::check(self);
        self.render_news();
    }

    fn publish_rankings(&mut self) {
        let mut v: Vec<(u16, f32)> = self.alive_civs().filter(|c| !c.minor).map(|c| (c.id, c.score)).collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let lines: Vec<String> = v
            .iter()
            .take(5)
            .enumerate()
            .map(|(k, (c, s))| format!("{}. {} ({:.0})", k + 1, self.civs[*c as usize].name, s))
            .collect();
        let leader = v[0].0;
        // envy and underdog sympathy
        for &(c, _) in v.iter().skip(1) {
            if self.r(c, leader).met {
                self.ledger(c, leader, Reason::WealthEnvy, -6.0, Some(40), "leads the era rankings");
            }
        }
        let s = StorySeed::new("rankings", Scope::Global)
            .slot("Leader", self.civs[leader as usize].name.clone())
            .slot("List", lines.join(", "))
            .slot("Year", year_str(self.year))
            .civ(leader)
            .cause("territory, population, technology, culture, wealth, military and wonders tallied")
            .cat("political")
            .card();
        self.seeds.push(s);
        self.rankings.push((self.year, v));
    }

    fn decadal_review(&mut self) {
        let from = self.year - 10;
        let mut lines = Vec::new();
        // richest city
        let richest = self
            .cities
            .iter()
            .filter(|c| c.alive)
            .max_by(|a, b| a.wealth.partial_cmp(&b.wealth).unwrap())
            .map(|c| c.id);
        if let Some(r) = richest {
            let c = &self.cities[r as usize];
            let again = self.last_richest == Some(r);
            let top = {
                let mut idx: Vec<usize> = (0..5).collect();
                idx.sort_by(|a, b| c.wealth_parts[*b].partial_cmp(&c.wealth_parts[*a]).unwrap());
                format!(
                    "{} and {}",
                    WEALTH_WEIGHTS[idx[0]].0.to_lowercase(),
                    WEALTH_WEIGHTS[idx[1]].0.to_lowercase()
                )
            };
            // widest gap
            let poorest = self
                .cities
                .iter()
                .filter(|p| p.alive)
                .min_by(|a, b| a.wealth.partial_cmp(&b.wealth).unwrap())
                .map(|p| p.id)
                .unwrap();
            let pc = &self.cities[poorest as usize];
            let gap = format!(
                "{} ({}) against {} ({})",
                c.name,
                c.band(),
                pc.name,
                pc.band()
            );
            lines.push(format!(
                "Richest city: {} of the {} — wealth {:.0} ({}){}",
                c.name,
                self.civs[c.civ as usize].name,
                c.wealth,
                c.band(),
                if again { ", again" } else { "" }
            ));
            let s = StorySeed::new("richest_city", Scope::Global)
                .slot("City", c.name.clone())
                .slot("Civ", self.civs[c.civ as usize].name.clone())
                .slot("Spec", c.spec.name())
                .slot("Wealth", format!("{:.0}", c.wealth))
                .slot("Band", c.band())
                .slot("Again", if again { "again" } else { "for the first time" })
                .slot("Top", top.clone())
                .slot("Gap", gap)
                .city(r)
                .civ(c.civ)
                .at(c.x, c.y)
                .cause(format!("led by {}", top))
                .cat("economic")
                .thread("Richest city of the decade");
            self.seeds.push(s);
            self.last_richest = Some(r);
        }
        // fastest growing civ
        let mut best: Option<(u16, f32)> = None;
        for c in self.alive_civs().filter(|c| !c.minor) {
            let h = &c.pop_history;
            if h.len() > 10 {
                let g = h[h.len() - 1] / h[h.len() - 11].max(1.0);
                if best.map(|b| g > b.1).unwrap_or(true) {
                    best = Some((c.id, g));
                }
            }
        }
        if let Some((c, g)) = best {
            lines.push(format!("Fastest growing: the {} (+{:.0}%)", self.civs[c as usize].name, (g - 1.0) * 100.0));
        }
        let count = |cat: &str| self.news.items.iter().filter(|n| n.year > from && n.category == cat && n.placement >= Placement::Ticker).count();
        let wars = count("military");
        let disasters = count("ecological");
        let health = count("health");
        let discoveries = count("technological");
        lines.push(format!("Wars & clashes reported: {}  ·  Disasters: {}  ·  Health crises: {}  ·  Discoveries: {}", wars, disasters, health, discoveries));
        let ext: Vec<String> = self.civs.iter().filter(|c| c.fell.map(|f| f > from).unwrap_or(false)).map(|c| c.name.clone()).collect();
        if !ext.is_empty() {
            lines.push(format!("Fallen: {}", ext.join(", ")));
        }
        lines.push(format!(
            "Planet: {:.1}°C mean, CO₂ ×{:.2}, biodiversity {:.0}%, sea level {:+.2}",
            self.planet.global_temp,
            self.planet.co2,
            self.planet.biodiv_index * 100.0,
            self.planet.sea_level * 10.0
        ));
        // terminus warnings
        if self.planet.biodiv_index < 0.25 {
            lines.push("WARNING: biodiversity approaching collapse floor.".into());
        }
        let living = self.alive_civs().count();
        if living <= 2 && self.year > 100 {
            lines.push("WARNING: few peoples remain; sapient extinction is possible.".into());
        }
        let s = StorySeed::new("decadal", Scope::Global)
            .slot("From", year_str(from))
            .slot("To", year_str(self.year))
            .slot("Lines", lines.join(" · "))
            .cause("the decade's records")
            .cat("review");
        self.seeds.push(s);
        self.reviews.push(Review { year: self.year, lines });
    }

    // ------------------------------------------------------------------ news
    pub fn render_news(&mut self) {
        let seeds = std::mem::take(&mut self.seeds);
        for seed in seeds {
            let era = seed.civ.map(|c| self.civs[c as usize].era).or_else(|| seed.city.map(|c| self.city_era(c))).unwrap_or_else(|| self.alive_civs().map(|c| c.era).max().unwrap_or(0));
            let (headline, body) = self.templates.render(&seed, era, &mut self.rng);
            // significance
            let mut sig = match seed.scope {
                Scope::Local => 0.5,
                Scope::National => 1.5,
                Scope::Regional => 2.0,
                Scope::Global => 3.5,
            } * seed.magnitude;
            if let Some(c) = seed.city {
                sig += (self.cities[c as usize].pop / 20.0).min(1.0);
                if self.news.watch_cities.contains(&c) {
                    sig += 3.0;
                }
            }
            if let Some(c) = seed.civ {
                if self.news.watch_civs.contains(&c) {
                    sig += 3.0;
                }
            }
            let followed = seed.thread.as_ref().map(|t| self.news.followed_threads.contains(t)).unwrap_or(false);
            if followed {
                sig += 3.0;
            }
            if seed.card {
                sig = sig.max(self.news.threshold_card);
            }
            let muted = self.news.muted.contains(&seed.category);
            let mut placement = if seed.chronicle {
                Placement::Chronicle
            } else if sig >= self.news.threshold_card {
                Placement::Card
            } else if sig >= self.news.threshold_ticker {
                Placement::Ticker
            } else {
                Placement::Gazette
            };
            if muted && placement < Placement::Chronicle && !followed {
                placement = Placement::Gazette;
            }
            let id = self.news.items.len();
            let mut why = seed.why.clone();
            if !seed.causes.is_empty() {
                why.push(format!("Causes cited: {}", seed.causes.join("; ")));
            }
            why.push(format!("Significance {:.1} → {:?}", sig, placement));
            let item = NewsItem {
                id,
                year: self.year,
                headline,
                body,
                scope: seed.scope,
                category: seed.category,
                significance: sig,
                placement,
                city: seed.city,
                civ: seed.civ,
                civ2: seed.civ2,
                pos: seed.pos,
                causes: seed.causes.clone(),
                why,
                alert: seed.alert,
                thread: seed.thread.clone(),
                kind: seed.kind,
                era_skin: era,
            };
            if let Some(c) = seed.city {
                self.cities[c as usize].gazette.push(id);
            }
            if placement >= Placement::Card {
                self.news.unread_cards.push(id);
                if self.news.unread_cards.len() > 60 {
                    self.news.unread_cards.remove(0);
                }
            }
            if placement >= Placement::Card && seed.scope >= Scope::National {
                self.stats.markers.push((self.year, id));
            }
            let watch_hit = seed.city.map(|c| self.news.watch_cities.contains(&c)).unwrap_or(false)
                || seed.civ.map(|c| self.news.watch_civs.contains(&c)).unwrap_or(false)
                || followed;
            if (seed.alert == AlertClass::Existential && self.auto_pause_existential)
                || (seed.alert == AlertClass::Major && self.auto_pause_major)
                || (watch_hit && self.auto_pause_watch && placement >= Placement::Ticker)
            {
                self.alerts.push(id);
            }
            if let Some((x, y)) = seed.pos {
                let col = match seed.tone {
                    Tone::Triumph => [120, 230, 120],
                    Tone::Alarm => [255, 170, 40],
                    Tone::Tragedy => [255, 60, 60],
                    _ => [220, 220, 255],
                };
                if placement >= Placement::Ticker {
                    self.flash(x, y, col);
                }
            }
            self.news.items.push(item);
        }
    }

    pub fn chronicle_text(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("THE CHRONICLE OF A LIVING WORLD (seed {})\n", self.params.seed));
        s.push_str(&format!("Sapient species: {} — {}\n", self.species.name, self.species.desc));
        s.push_str(&format!("Lineages lost before the dawn: {}\n\n", self.species.extinct_lineages.join(", ")));
        for n in self.news.items.iter().filter(|n| n.placement == Placement::Chronicle || (n.placement == Placement::Card && n.scope >= Scope::National)) {
            s.push_str(&format!("[{}] {}\n    {}\n", year_str(n.year), n.headline, n.body));
        }
        s.push_str("\nCIVILIZATIONS\n");
        for c in &self.civs {
            s.push_str(&format!(
                "{} ({}), founded {}{}; leaders: {}\n",
                c.name,
                if c.minor { "city-state" } else { "civilization" },
                year_str(c.founded),
                c.fell.map(|f| format!(", fell {}", year_str(f))).unwrap_or_default(),
                c.leaders_past
                    .iter()
                    .map(|l| format!("{} ({}–{}){}", l.0, year_str(l.1), year_str(l.2), if l.3.is_empty() { String::new() } else { format!(" — {}", l.3) }))
                    .chain(std::iter::once(format!("{} {} (reigning)", c.leader.title, c.leader.name)))
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        s.push_str("\nDECADAL REVIEWS\n");
        for r in &self.reviews {
            s.push_str(&format!("{}: {}\n", year_str(r.year), r.lines.join(" | ")));
        }
        s
    }
}

pub fn city_pop_factor(p: f32) -> f32 {
    (p / 20.0).min(2.0)
}

pub fn civ_era_radius(era: usize) -> usize {
    match era {
        0..=1 => 0,
        2..=4 => 1,
        _ => 2,
    }
}

pub fn year_str(y: i32) -> String {
    format!("Year {}", y)
}

pub fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8]
}
