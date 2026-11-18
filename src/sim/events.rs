//! Random events: drawn by weighted chance from condition-gated pools (design doc section 9, appendix A).
//! Every event lists prerequisites, weight modifiers, cooldowns, effects, the NPC response and news.

use super::city::*;
use super::civ::*;
use super::news::*;
use super::planet::*;
use super::*;
use rand::Rng;

pub struct EventDef {
    pub id: u16,
    pub name: &'static str,
    pub cat: &'static str,
    pub pattern: &'static str,
    pub card: bool,
    pub cooldown: i32,
}

macro_rules! ev {
    ($id:expr, $n:expr, $c:expr, $p:expr, $card:expr, $cd:expr) => {
        EventDef { id: $id, name: $n, cat: $c, pattern: $p, card: $card, cooldown: $cd }
    };
}

pub static EVENTS: &[EventDef] = &[
    ev!(1, "Treasury heist", "economic", "A", true, 40),
    ev!(2, "Bank run", "economic", "A", true, 50),
    ev!(3, "Luxury market crash", "economic", "A", false, 60),
    ev!(4, "Gold rush", "economic", "D", true, 80),
    ev!(5, "Boomtown bust", "economic", "A→B", false, 80),
    ev!(6, "Merchant prince's offer", "economic", "D", true, 50),
    ev!(7, "Smuggling ring", "economic", "C", false, 30),
    ev!(8, "Predatory loan", "economic", "C", true, 60),
    ev!(9, "Art forgery scandal", "economic", "A", false, 60),
    ev!(10, "Market fire", "economic", "B", false, 25),
    ev!(11, "Caravan windfall", "economic", "D", false, 25),
    ev!(12, "Exposition bid", "economic", "D", true, 80),
    ev!(13, "Counterfeit currency", "economic", "any", false, 40),
    ev!(14, "Tax revolt", "economic", "C", false, 40),
    ev!(15, "River flood", "ecological", "B", false, 15),
    ev!(16, "Drought", "ecological", "B", false, 30),
    ev!(17, "Wildfire", "ecological", "B", false, 20),
    ev!(18, "Earthquake", "ecological", "B", false, 25),
    ev!(19, "Volcanic ashfall", "ecological", "any", false, 20),
    ev!(20, "Locust swarm", "ecological", "B", false, 25),
    ev!(21, "River shifts course", "ecological", "any", true, 150),
    ev!(22, "Coastal inundation", "ecological", "B", false, 30),
    ev!(23, "Harsh winter", "ecological", "B", false, 15),
    ev!(24, "Fishery collapse", "ecological", "any", false, 50),
    ev!(25, "Plague by trade route", "health", "any", true, 60),
    ev!(26, "Endemic outbreak", "health", "B", false, 30),
    ev!(27, "Medical breakthrough", "health", "D", true, 80),
    ev!(28, "Contaminated water", "health", "B", false, 40),
    ev!(29, "Famine", "health", "B", true, 20),
    ev!(30, "Healer pilgrimage", "health", "any", false, 50),
    ev!(31, "Succession crisis", "political", "any", true, 0),
    ev!(32, "Civil war", "political", "any", true, 0),
    ev!(33, "Corrupt governor exposed", "political", "A mod", false, 40),
    ev!(34, "Reform movement", "political", "any", false, 50),
    ev!(35, "Revolutionary agitation", "political", "C", true, 50),
    ev!(36, "Coup attempt", "political", "any", false, 60),
    ev!(37, "Golden age dawn", "political", "Prosperous+", false, 80),
    ev!(38, "Bureaucratic ossification", "political", "A", false, 100),
    ev!(39, "Relic discovered", "religious", "any", true, 200),
    ev!(40, "Relic theft", "religious", "A", true, 80),
    ev!(41, "Schism", "religious", "any", false, 150),
    ev!(42, "Great awakening", "religious", "any", false, 60),
    ev!(43, "Wonder festival", "cultural", "A", true, 0),
    ev!(44, "Master artisan emerges", "cultural", "D", false, 40),
    ev!(45, "Forbidden idea spreads", "cultural", "any", false, 60),
    ev!(46, "Raiders target the rich road", "military", "A", true, 30),
    ev!(47, "Border incident", "military", "any", false, 15),
    ev!(48, "Spy ring uncovered", "military", "A", true, 0),
    ev!(49, "Mercenaries offer service", "military", "D", false, 30),
    ev!(50, "Arms cache found", "military", "any", false, 80),
    ev!(51, "Refugee column arrives", "military", "C", false, 20),
    ev!(52, "Industrial accident", "technological", "B/A", true, 50),
    ev!(53, "Resource revealed", "technological", "any", false, 0),
    ev!(54, "Observatory omen", "technological", "any", false, 60),
    ev!(55, "Meteor strike", "cosmic", "any", true, 0),
    ev!(56, "Solar anomaly year", "cosmic", "any", false, 0),
    ev!(57, "Ruins expedition", "technological", "D", false, 60),
    ev!(58, "First contact", "political", "any", true, 0),
    ev!(59, "Runaway project", "technological", "A/D", true, 100),
    ev!(60, "Planetary departure project", "technological", "D", true, 0),
];

pub fn event_def(id: u16) -> &'static EventDef {
    &EVENTS[(id - 1) as usize]
}

#[derive(Clone)]
struct Ctx {
    b: usize,
    w: f32,
    era: usize,
    pop: f32,
    fault: bool,
    volcano: bool,
    dry: bool,
    forest: f32,
    temp: f32,
    poor_nb: Option<u32>,
    rich_nb: Option<u32>,
    routes: usize,
    industrial: bool,
    holy: bool,
    market_econ: bool,
    unworked: bool,
    fished: bool,
    low_coast: bool,
    hostile: Option<u16>,
    gap_civ: bool,
}

struct W {
    w: f32,
    why: Vec<String>,
}

impl W {
    fn new(base: f32, gate: impl Into<String>) -> W {
        W { w: base, why: vec![format!("Prerequisite met: {}", gate.into()), format!("Base weight {:.2}", base)] }
    }
    fn x(mut self, cond: bool, f: f32, what: &str) -> W {
        if cond {
            self.w *= f;
            self.why.push(format!("×{} {}", f, what));
        }
        self
    }
    fn done(mut self) -> Option<(f32, Vec<String>)> {
        if self.w <= 0.0 {
            return None;
        }
        self.why.push(format!("Final weight {:.2}", self.w));
        Some((self.w, self.why))
    }
}

impl Sim {
    fn ctx(&self, ci: u32) -> Ctx {
        let c = &self.cities[ci as usize];
        let civ = &self.civs[c.civ as usize];
        let i = self.planet.idx(c.x, c.y).unwrap();
        let near = self.planet.tiles_in_radius(c.x, c.y, 2);
        let fault = near.iter().any(|&t| self.planet.tiles[t].fault);
        let volcano = self.planet.tiles_in_radius(c.x, c.y, 5).iter().any(|&t| self.planet.tiles[t].volcano > 0.0);
        let forest = near.iter().map(|&t| self.planet.tiles[t].forest).sum::<f32>() / near.len() as f32;
        let mut poor_nb = None;
        let mut rich_nb = None;
        for o in self.cities.iter().filter(|o| o.alive && o.id != ci) {
            if self.planet.dist2(c.x, c.y, o.x, o.y) < 100.0 {
                if o.wealth < 250.0 && poor_nb.is_none() {
                    poor_nb = Some(o.id);
                }
                if o.wealth >= 650.0 && rich_nb.is_none() {
                    rich_nb = Some(o.id);
                }
            }
        }
        let routes = self.routes.iter().filter(|r| r.alive && (r.a == ci || r.b == ci)).count();
        let unworked = c.catchment.iter().any(|&t| {
            self.planet.tiles[t].resource.map(|r| matches!(r.kind, ResKind::Gold | ResKind::Gems | ResKind::Copper | ResKind::Iron | ResKind::Coal | ResKind::Oil) && r.kind.revealed_by().map(|k| civ.known_tech[k]).unwrap_or(true) && r.stock > 50.0).unwrap_or(false)
                && self.planet.tiles[t].improvement == Improvement::None
        });
        let fished = c.coastal && c.catchment.iter().filter(|&&t| !self.planet.is_land(t)).any(|&t| self.planet.tiles[t].fish < 0.25);
        let hostile = (0..self.civs.len() as u16).find(|&o| self.r(c.civ, o).met && self.r(c.civ, o).score < -30.0 && !self.r(c.civ, o).at_war);
        let (_, rich, poor) = self.civ_wealth(c.civ);
        let gap_civ = match (rich, poor) {
            (Some(r), Some(p)) => band_index(self.cities[r as usize].wealth) >= band_index(self.cities[p as usize].wealth) + 3,
            _ => false,
        };
        Ctx {
            b: band_index(c.wealth),
            w: c.wealth,
            era: civ.era,
            pop: c.pop,
            fault,
            volcano,
            dry: self.planet.tiles[i].rain < 0.33,
            forest,
            temp: self.planet.tiles[i].temp,
            poor_nb,
            rich_nb,
            routes,
            industrial: c.has("Factory") || c.has("Power Plant"),
            holy: c.religion.map(|r| self.religions[r].holy_city == ci).unwrap_or(false),
            market_econ: civ.gov().market,
            unworked,
            fished,
            low_coast: c.coastal && self.planet.height(i) < 0.05,
            hostile,
            gap_civ,
        }
    }

    fn eligible(&self, ev: u16, ci: u32, x: &Ctx) -> Option<(f32, Vec<String>)> {
        let c = &self.cities[ci as usize];
        let civ = &self.civs[c.civ as usize];
        let band = format!("{} (wealth {:.0})", wealth_band(x.w), x.w);
        let backlog = c.backlog > 30.0;
        let drought = c.chains.drought > 0;
        match ev {
            1 if x.b >= 4 => W::new(1.0, format!("Rich+ city: {}", band)).x(!c.has("Vault"), 2.0, "no vault").x(x.b == 5, 1.5, "Opulent").done(),
            2 if x.b >= 4 && c.has("Bank") && x.market_econ => W::new(0.6, format!("Rich+ with bank: {}", band)).x(c.stability < 40.0, 2.0, "low stability").done(),
            3 if x.b == 5 && c.resources.iter().filter(|r| r.class() == ResClass::Luxury).count() >= 2 && x.market_econ => W::new(0.6, "Opulent, 2+ luxuries").done(),
            4 if x.unworked && c.chains.gold_rush == 0 => W::new(0.8, "unworked deposit near city").done(),
            6 if x.b >= 4 => W::new(0.5, format!("Rich+ city: {}", band)).x(civ.leader.traits[T_GREED] > 0.6, 1.5, "mercantile leader").done(),
            7 if x.b >= 4 && x.poor_nb.is_some() => W::new(0.8, "Rich city beside a Poor city").done(),
            8 if x.b <= 1 && x.rich_nb.is_some() && c.debt <= 0.0 => W::new(0.8, "Poor city with a Rich neighbor lender").done(),
            9 if x.b == 5 && (c.has("Monument") || c.has("Temple")) => W::new(0.5, "Opulent with culture building").done(),
            10 if x.pop > 4.0 => W::new(0.7, "any band (severity by wealth)").x(backlog, 2.5, "maintenance backlog").x(drought, 1.5, "dry season").done(),
            11 if x.b >= 3 && x.routes > 0 => W::new(0.8, format!("Prosperous+ with active route: {}", band)).done(),
            12 if x.b >= 4 && civ.era >= 4 && !self.cities.iter().any(|o| o.on_cooldown(12, self.year)) => W::new(0.4, "Rich+, exploration era").done(),
            13 if x.b >= 2 && c.has("Market") => W::new(0.4, "Modest+ with market").done(),
            14 if x.gap_civ || (x.b <= 1 && c.stability < 45.0) => W::new(0.6, "wide internal wealth gap or poor and unstable").x(x.gap_civ, 2.0, "opulent capital vs poor provinces").done(),
            15 if c.river => W::new(0.4, "riverside city").x(c.has("Flood Defenses"), 0.3, "flood defenses").x(self.planet.tiles[self.planet.idx(c.x, c.y).unwrap()].rain > 0.9, 2.0, "heavy rains").done(),
            16 if x.dry && c.chains.drought == 0 => W::new(0.6, "dry climate cell").x(self.planet.tiles[self.planet.idx(c.x, c.y).unwrap()].rain_nudge < -0.05, 3.0, "drying trend (rain nudged down)").done(),
            17 if x.forest > 0.35 && (x.dry || drought) => W::new(0.7, "forest and dry tiles").done(),
            18 if x.fault => W::new(0.2, "city on a fault line").x(self.planet.tiles_in_radius(c.x, c.y, 3).iter().any(|&t| self.planet.tiles[t].uplift_stress > 0.2), 3.0, "stress from recent uplift").done(),
            19 if x.volcano => W::new(0.6, "downwind of an active volcano").done(),
            20 if x.temp > 16.0 && c.catchment.iter().any(|&t| self.planet.tiles[t].improvement == Improvement::Farm) => W::new(0.5, "warm farmland region").x(drought, 2.0, "drought").done(),
            21 if c.river && c.coastal && !c.stranded && self.planet.height(self.planet.idx(c.x, c.y).unwrap()) < 0.12 => W::new(0.15, "delta lowland city").done(),
            22 if x.low_coast && self.planet.sea_level > 0.004 => W::new(1.0, "low coastal city, sea level rising").x(c.has("Seawall"), 0.2, "seawall").done(),
            23 if x.temp < 6.0 || self.planet.season_variability(c.y as usize) > 0.35 => W::new(0.7, "high latitude or strong seasons").x(civ.ability == "Frost-Hardened", 0.3, "frost-hardened people").done(),
            24 if x.fished => W::new(1.2, "overharvested fishery").done(),
            25 if x.routes >= 2 && c.chains.plague == 0 && (c.coastal || x.routes >= 3) => W::new(0.14, "port or trade hub").x(civ.knows("Antibiotics"), 0.3, "antibiotics known").x(c.pop > 15.0, 1.5, "dense city").done(),
            26 if c.health < 50.0 && x.pop > 8.0 && c.plague_severity == 0.0 => W::new(0.6, "low health, dense city").done(),
            27 if (c.has("University") || c.has("Hospital")) && x.b >= 3 => W::new(0.3, "university and Prosperous+ funding").done(),
            28 if x.industrial && c.river => W::new(0.7, "industrial city on its water source").done(),
            30 if (x.holy || c.has("Library")) && civ.era >= 1 => W::new(0.3, "holy or scholarly city").x(x.b >= 4, 1.5, "rich city (fraud risk)").done(),
            33 if c.governor.corruption > 0.25 => W::new(0.5, "corrupt governor").x(x.b >= 4, 2.0, "rich city (more to steal)").done(),
            34 if civ.era >= 3 && c.has("Library") => W::new(0.25, "literacy threshold").done(),
            35 if x.b <= 1 && x.rich_nb.is_some() && civ.era >= 2 => W::new(0.8, "Poor city beside a Rich exemplar").done(),
            36 if c.is_capital && civ.military > 30.0 && c.stability < 40.0 => W::new(0.5, "strong military, unpopular leader").done(),
            37 if x.b >= 3 && c.stability > 70.0 && c.golden_until < self.year => W::new(0.3, "sustained prosperity and stability").done(),
            38 if x.b >= 4 && civ.cities.len() >= 6 && self.year - civ.founded > 250 => W::new(0.3, "rich, old, large civilization").done(),
            39 if !c.relic && (x.holy || self.planet.tiles_in_radius(c.x, c.y, 3).iter().any(|&t| self.planet.tiles[t].ruin.is_some() || self.planet.tiles[t].natural_wonder.is_some())) => W::new(0.25, "holy site or ruins nearby").done(),
            40 if c.relic && x.b >= 4 => W::new(0.7, "relic in a Rich+ holy city").done(),
            41 if c.religion.is_some() && civ.religion.is_some() && c.religion == civ.religion && self.religions[c.religion.unwrap()].founder != c.civ && self.cities.iter().filter(|o| o.alive && o.religion == c.religion).count() > 8 => W::new(0.15, "large religion, distant converts").done(),
            42 if c.chains.plague >= 2 || c.famine_years > 0 => W::new(0.6, "after disaster or plague").done(),
            44 if x.b >= 3 && (c.has("Library") || c.has("Monument")) => W::new(0.35, "Prosperous+ with culture or science").done(),
            45 if civ.era >= 4 && x.routes > 0 => W::new(0.25, "literacy and trade contact").done(),
            46 if x.b >= 4 && c.garrison < 5.0 && x.routes > 0 => W::new(0.6, "Rich city with low local defense").done(),
            47 if x.hostile.is_some() => W::new(0.8, "hostile neighbor").done(),
            49 if civ.treasury > 50.0 => W::new(0.25, "treasury to pay").x((0..self.civs.len() as u16).any(|o| self.r(c.civ, o).at_war), 3.0, "at war").done(),
            50 if civ.era <= 4 && self.planet.tiles_in_radius(c.x, c.y, 3).iter().any(|&t| self.planet.tiles[t].ruin.is_some() || self.planet.tiles[t].resource.map(|r| r.kind == ResKind::Iron).unwrap_or(false)) => W::new(0.3, "ruins or deposit nearby").done(),
            52 if x.industrial => W::new(0.4, "industrial city").x(backlog, 2.0, "maintenance backlog").x(civ.leader.traits[T_ECOLOGY] < 0.3, 1.5, "exploitive leader").done(),
            54 if (c.wonders.iter().any(|&w| WONDERS[w].name.contains("Observatory")) || x.holy) && civ.era <= 5 => W::new(0.25, "observatory or holy city").done(),
            57 if x.b >= 3 && self.planet.tiles_in_radius(c.x, c.y, 8).iter().any(|&t| self.planet.tiles[t].ruin.is_some()) => W::new(0.3, "Prosperous+ sponsor near ruins").done(),
            59 if civ.era >= 7 && x.b >= 4 => W::new(0.25, "advanced tech, Rich+ funding").done(),
            _ => None,
        }
    }

    pub fn run_events(&mut self) {
        let year = self.year;
        self.advance_chains();
        let sev = self.difficulty.severity();
        // city events: a notable local event roughly every 8-15 years
        let n = self.cities.len();
        for ci in 0..n {
            if !self.cities[ci].alive || !self.rng.gen_bool(0.09) {
                continue;
            }
            let x = self.ctx(ci as u32);
            let mut pool: Vec<(u16, f32, Vec<String>)> = Vec::new();
            for e in EVENTS {
                if self.cities[ci].on_cooldown(e.id, year) {
                    continue;
                }
                if let Some((w, why)) = self.eligible(e.id, ci as u32, &x) {
                    pool.push((e.id, w, why));
                }
            }
            // cap wealth-pattern events to roughly a third of a city's history
            let total: f32 = pool.iter().map(|p| p.1).sum();
            if total <= 0.0 {
                continue;
            }
            let mut r = self.rng.gen_range(0.0..total);
            let mut chosen = None;
            for p in pool {
                if r < p.1 {
                    chosen = Some(p);
                    break;
                }
                r -= p.1;
            }
            if let Some((id, w, mut why)) = chosen {
                why.push(format!("Chosen from pool with total weight {:.2} (roll in city each year: 9%)", total));
                let def = event_def(id);
                self.cities[ci].set_cooldown(id, year + def.cooldown.max(5));
                self.apply_event(id, ci as u32, why, sev, w);
            }
        }
        // geological hazards come from the planet first
        let nt = self.planet.tiles.len();
        for t in 0..nt {
            let v = self.planet.tiles[t].volcano;
            if v > 0.0 && self.rng.gen_bool((v * 0.004 * (1.0 + self.planet.tiles[t].uplift_stress * 5.0) * scenario::volcanism(self)).min(0.5) as f64) {
                self.eruption(t);
            }
        }
        // global events
        if self.rng.gen_bool((self.cosmic * 0.006) as f64) && year > 30 {
            self.meteor(None, false);
        }
        if self.rng.gen_bool(0.012) {
            self.solar_anomaly();
        }
        // planetary departure endgame
        for c in 0..self.civs.len() {
            if self.civs[c].alive && self.civs[c].knows("Starship") && self.ended.is_none() {
                let (w, _, _) = self.civ_wealth(c as u16);
                if w > 600.0 && self.rng.gen_bool(0.05) {
                    self.departure(c as u16);
                }
            }
        }
    }

    fn base_seed(&self, kind: &'static str, ci: u32, scope: Scope, def_id: u16, why: Vec<String>) -> StorySeed {
        let c = &self.cities[ci as usize];
        let civ = &self.civs[c.civ as usize];
        let def = event_def(def_id);
        let mut s = StorySeed::new(kind, scope)
            .slot("City", c.name.clone())
            .slot("Civ", civ.name.clone())
            .slot("Leader", civ.leader.name.clone())
            .slot("Governor", c.governor.name.clone())
            .slot("Band", c.band())
            .slot("Wealth", format!("{:.0}", c.wealth))
            .slot("Spec", c.spec.name())
            .city(ci)
            .civ(c.civ)
            .at(c.x, c.y)
            .cat(def.cat)
            .why(
                std::iter::once(format!("Event #{} {} — wealth pattern {}", def.id, def.name, def.pattern))
                    .chain(why)
                    .collect(),
            );
        if def.card {
            s = s.card();
        }
        s
    }

    /// Pattern B: severity scales inversely with wealth. Returns (severity, counterfactual line).
    fn pattern_b(&self, ci: u32, base: f32, sev: f32) -> (f32, String) {
        let w = self.cities[ci as usize].wealth;
        let s = base * sev * (1.5 - w / 1000.0 * 1.2);
        let alt_w = if w >= 450.0 { 150.0 } else { 750.0 };
        let alt = base * sev * (1.5 - alt_w / 1000.0 * 1.2);
        let line = format!(
            "At {} wealth the same blow would have been {:.0}% {}.",
            wealth_band(alt_w),
            ((alt / s.max(0.01)) - 1.0).abs() * 100.0,
            if alt > s { "worse" } else { "milder" }
        );
        (s.max(0.05), line)
    }

    fn open_window(&mut self, ci: u32, kind: &'static str, title: String, years: i32, cost: f32) {
        if self.cities[ci as usize].pop < 4.0 || matches!(self.mode, scenario::Mode::Chronicle) {
            return;
        }
        let id = self.next_window;
        self.next_window += 1;
        let pop = self.cities[ci as usize].pop;
        self.windows.push(InterventionWindow { id, city: ci, kind, title, expires: self.year + years, cost, resolved: false, opened: self.year, pop_at_open: pop });
    }

    fn apply_event(&mut self, id: u16, ci: u32, why: Vec<String>, sev: f32, _w: f32) {
        let cu = ci as usize;
        let civ_id = self.cities[cu].civ;
        let cv = civ_id as usize;
        let comp = self.cities[cu].governor.competence;
        let treasury = self.civs[cv].treasury;
        let t = self.civs[cv].leader.traits;
        let year = self.year;
        let name = self.cities[cu].name.clone();
        let protected = self.cities[cu].protected_until > year;
        let sev = if protected { sev * 0.3 } else { sev };
        let mut seed: Option<StorySeed> = None;
        match id {
            1 => {
                let amt = (treasury.max(10.0) * self.rng.gen_range(0.10..0.20)).round();
                self.civs[cv].treasury -= amt;
                let resp = if comp > 0.6 { "vault works" } else if self.cities[cu].governor.corruption > 0.25 { "silence" } else { "a citywide inquiry" };
                if resp == "vault works" && !self.cities[cu].has("Vault") && self.civs[cv].knows("Banking") {
                    self.cities[cu].build = Some(Build::Building(bidx("Vault")));
                }
                let partner = self.routes.iter().find(|r| r.alive && (r.a == ci || r.b == ci)).map(|r| if r.a == ci { r.b } else { r.a });
                let mut s = self.base_seed("heist", ci, Scope::Local, id, why)
                    .slot("Amount", format!("{:.0} gold", amt))
                    .slot("Response", resp)
                    .slot("LossDetail", format!("{:.0} gold missing from the vaults", amt))
                    .slot("Bank", if self.cities[cu].has("Bank") { format!("{} Bank", name) } else { format!("{} Treasury", name) })
                    .slot("TradePartner", partner.map(|p| self.city_name(p)).unwrap_or_else(|| "its neighbors".into()))
                    .cause(format!("{} vaults ({}) and {}", self.cities[cu].band(), if self.cities[cu].has("Vault") { "guarded" } else { "no vault" }, if comp < 0.5 { "a lax governor" } else { "a bold crew" }))
                    .tone(Tone::Alarm);
                if self.cities[cu].wealth >= 850.0 {
                    s = s.mag(1.5);
                }
                seed = Some(s);
            }
            2 => {
                let guarantee = treasury > 150.0 && t[T_GREED] < 0.7;
                if guarantee {
                    self.civs[cv].treasury -= 80.0;
                } else {
                    self.cities[cu].debt += 80.0;
                    self.cities[cu].unrest += 15.0;
                }
                seed = Some(self.base_seed("bank_run", ci, Scope::National, id, why)
                    .slot("Response", if guarantee { "guaranteed deposits at great cost" } else { "let the banks fail" })
                    .cause("depositors lost faith after rumors of losses")
                    .cause(format!("{:.0} stability", self.cities[cu].stability))
                    .tone(Tone::Alarm));
            }
            3 => {
                self.cities[cu].debt += 60.0;
                self.cities[cu].pop *= 0.97;
                seed = Some(self.base_seed("luxury_crash", ci, Scope::Local, id, why).cause("an oversupplied luxury market").tone(Tone::Irony));
            }
            4 => {
                self.cities[cu].chains.gold_rush = 1;
                self.cities[cu].chains.stage_year = year;
                self.cities[cu].pop += 2.0;
                self.civs[cv].treasury += 40.0;
                let res = self.cities[cu].catchment.iter().find_map(|&tt| self.planet.tiles[tt].resource.filter(|r| matches!(r.kind, ResKind::Gold | ResKind::Gems | ResKind::Copper | ResKind::Iron | ResKind::Coal | ResKind::Oil)).map(|r| r.kind.name())).unwrap_or("ore");
                for &tt in &self.cities[cu].catchment.clone() {
                    if self.planet.tiles[tt].resource.map(|r| r.kind.name() == res).unwrap_or(false) {
                        self.planet.tiles[tt].improvement = Improvement::Mine;
                    }
                }
                seed = Some(self.base_seed("gold_rush", ci, Scope::Regional, id, why)
                    .slot("Resource", res)
                    .cause(format!("an unworked {} deposit near {}", res.to_lowercase(), name))
                    .thread(format!("Gold rush at {}", name))
                    .tone(Tone::Triumph));
            }
            6 => {
                let accept = t[T_GREED] > 0.4 || treasury < 50.0;
                if accept {
                    self.cities[cu].build_progress += 120.0;
                    self.cities[cu].governor.corruption = (self.cities[cu].governor.corruption + 0.1).min(1.0);
                }
                let prince = self.civs[cv].lang.person(&mut self.rng);
                seed = Some(self.base_seed("merchant_prince", ci, Scope::Local, id, why)
                    .slot("Prince", prince)
                    .slot("Response", if accept { "accepted, trading policy influence for coin" } else { "refused the strings attached" })
                    .cause(format!("a {} city attracts private fortunes", self.cities[cu].band().to_lowercase())));
            }
            7 => {
                let poor = self.ctx(ci).poor_nb.unwrap();
                self.civs[cv].treasury += 15.0;
                self.cities[cu].stability -= 6.0;
                let pc = self.cities[poor as usize].civ;
                self.civs[pc as usize].treasury += 10.0;
                seed = Some(self.base_seed("smuggling", ci, Scope::Local, id, why)
                    .slot("Poor", self.city_name(poor))
                    .cause(format!("the price gap between {} ({}) and {} ({})", name, self.cities[cu].band(), self.city_name(poor), self.cities[poor as usize].band())));
            }
            8 => {
                let rich = self.ctx(ci).rich_nb.unwrap();
                let lender = self.cities[rich as usize].civ;
                let accept = treasury < 40.0 || comp < 0.5;
                if accept {
                    self.civs[cv].treasury += 60.0;
                    self.cities[cu].debt = 90.0;
                    self.cities[cu].debt_lender = Some(lender);
                    self.cities[cu].chains.stage_year = year;
                }
                seed = Some(self.base_seed("predatory_loan", ci, Scope::Local, id, why)
                    .slot("Lender", self.city_name(rich))
                    .slot("LenderCiv", self.civ_name(lender))
                    .slot("Response", if accept { "took the loan" } else { "refused the loan" })
                    .civ2(lender)
                    .cause(format!("{} ({}) has capital; {} ({}) has needs", self.city_name(rich), self.cities[rich as usize].band(), name, self.cities[cu].band())));
            }
            9 => {
                let prosecute = comp > 0.5;
                self.cities[cu].culture -= 20.0;
                seed = Some(self.base_seed("forgery", ci, Scope::Local, id, why)
                    .slot("Response", if prosecute { "prosecuted the forgers" } else { "quietly bought the fakes" })
                    .cause("an opulent art market rewarded fakes").tone(Tone::Irony));
            }
            10 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev * if self.cities[cu].backlog > 30.0 { 1.5 } else { 1.0 });
                let rich = self.cities[cu].wealth >= 450.0;
                if rich {
                    self.civs[cv].treasury -= 20.0 * s;
                } else {
                    self.cities[cu].food_store *= 1.0 - (0.4 * s).min(0.9);
                }
                self.cities[cu].build_progress *= 0.5;
                seed = Some(self.base_seed("market_fire", ci, Scope::Local, id, why)
                    .slot("Loss", if rich { format!("{:.0} gold in goods", 20.0 * s) } else { "much of the food stores".to_string() })
                    .slot("Counterfactual", cf)
                    .cause(if self.cities[cu].backlog > 30.0 { "neglected fire lanes (maintenance backlog)" } else { "a lamp in a crowded market" })
                    .tone(Tone::Alarm));
            }
            11 => {
                let amt = 15.0 + self.cities[cu].wealth * 0.06;
                self.civs[cv].treasury += amt;
                for r in self.routes.iter_mut() {
                    if r.alive && (r.a == ci || r.b == ci) {
                        r.value *= 1.1;
                    }
                }
                seed = Some(self.base_seed("windfall", ci, Scope::Local, id, why).slot("Amount", format!("{:.0}", amt)).cause("a fortunate trading season on its routes").tone(Tone::Triumph));
            }
            12 => {
                let bid = 60.0 + self.cities[cu].wealth * 0.1;
                let win = self.rng.gen_bool(0.5);
                self.civs[cv].treasury -= bid;
                if win {
                    self.cities[cu].culture += 120.0;
                    self.cities[cu].golden_until = year + 10;
                }
                seed = Some(self.base_seed("exposition", ci, Scope::Global, id, why)
                    .slot("Bid", format!("{:.0}", bid))
                    .slot("Result", if win { "won the right to host the great exposition" } else { "lost the bid" })
                    .cause(format!("a {} treasury could afford the bid", self.cities[cu].band().to_lowercase()))
                    .tone(if win { Tone::Triumph } else { Tone::Irony }));
            }
            13 => {
                self.civs[cv].treasury -= 15.0;
                seed = Some(self.base_seed("counterfeit", ci, Scope::Local, id, why).cause("false coins circulating in the market"));
            }
            14 => {
                let resp = if t[T_AGGRESSION] > 0.6 { "repressed the protests" } else if comp > 0.5 { "granted tax reform" } else { "conceded local autonomy" };
                self.cities[cu].unrest += if resp.starts_with("repressed") { 5.0 } else { 15.0 };
                if resp.starts_with("granted") {
                    self.civs[cv].treasury -= 20.0;
                }
                let (_, rich, _) = self.civ_wealth(civ_id);
                seed = Some(self.base_seed("tax_revolt", ci, Scope::National, id, why)
                    .slot("Response", resp)
                    .cause(match rich {
                        Some(r) if r != ci => format!("taxes flowing to {} ({}) while {} stays {}", self.city_name(r), self.cities[r as usize].band(), name, self.cities[cu].band()),
                        _ => "heavy taxes".to_string(),
                    }));
            }
            15 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev);
                let loss = (self.cities[cu].pop * 0.06 * s).min(self.cities[cu].pop * 0.3);
                self.cities[cu].pop -= loss;
                self.cities[cu].backlog += 10.0 * s;
                for &tt in &self.cities[cu].catchment.clone() {
                    if self.planet.tiles[tt].is_river() {
                        self.planet.tiles[tt].soil = (self.planet.tiles[tt].soil + 0.05).min(1.0);
                    }
                }
                self.open_window(ci, "flood", format!("Flood in {}", name), 3, 12.0);
                seed = Some(self.base_seed("flood", ci, Scope::Regional, id, why)
                    .slot("Loss", format!("{:.0}", loss * 1000.0))
                    .slot("Counterfactual", cf)
                    .cause(format!("{:.2} rainfall over the river basin", self.planet.tiles[self.planet.idx(self.cities[cu].x, self.cities[cu].y).unwrap()].rain))
                    .cause(if self.cities[cu].has("Flood Defenses") { "defenses overtopped" } else { "no flood defenses" })
                    .tone(Tone::Alarm));
            }
            16 => {
                self.cities[cu].chains.drought = 1;
                self.cities[cu].chains.stage_year = year;
                let rich = self.cities[cu].wealth >= 450.0;
                if rich {
                    self.civs[cv].treasury -= 25.0;
                    self.cities[cu].food_store += 10.0;
                }
                let i = self.planet.idx(self.cities[cu].x, self.cities[cu].y).unwrap();
                let mut s = self.base_seed("drought", ci, Scope::Regional, id, why)
                    .slot("Response", if rich { "imports grain at high prices" } else { "rations what it has" })
                    .cause(format!("rainfall of only {:.2} in the region", self.planet.tiles[i].rain))
                    .thread(format!("Drought at {}", name))
                    .tone(Tone::Alarm);
                if self.planet.tiles[i].rain_nudge < -0.05 {
                    s = s.cause("a drying trend shaped from above");
                }
                self.open_window(ci, "drought", format!("Drought around {}", name), 6, 18.0);
                seed = Some(s);
            }
            17 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev);
                for &tt in &self.cities[cu].catchment.clone() {
                    let tile = &mut self.planet.tiles[tt];
                    if tile.forest > 0.3 && self.rng.gen_bool(0.5) {
                        tile.forest *= 0.3;
                        tile.ash += 0.4;
                    }
                }
                self.cities[cu].pop *= 1.0 - (0.03 * s).min(0.2);
                self.planet.particulates += 0.05;
                seed = Some(self.base_seed("wildfire", ci, Scope::Local, id, why).slot("Counterfactual", cf).cause("dry forest near the city").tone(Tone::Alarm));
            }
            18 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev * if self.cities[cu].backlog > 30.0 { 1.6 } else { 1.0 });
                let loss = (self.cities[cu].pop * 0.08 * s).min(self.cities[cu].pop * 0.4);
                self.cities[cu].pop -= loss;
                self.cities[cu].backlog += 25.0 * s;
                // a building may collapse
                let built: Vec<usize> = (0..BUILDINGS.len()).filter(|&k| self.cities[cu].buildings[k]).collect();
                let mut lost = String::from("none");
                if !built.is_empty() && self.rng.gen_bool((0.3 * s).min(0.9) as f64) {
                    let k = built[self.rng.gen_range(0..built.len())];
                    self.cities[cu].buildings[k] = false;
                    lost = BUILDINGS[k].name.to_string();
                }
                let stress = self.planet.tiles_in_radius(self.cities[cu].x, self.cities[cu].y, 3).iter().any(|&tt| self.planet.tiles[tt].uplift_stress > 0.2);
                self.open_window(ci, "quake", format!("Earthquake at {}", name), 2, 10.0);
                let mut sd = self.base_seed("earthquake", ci, Scope::Regional, id, why).mag(if loss > 1.0 { 1.6 } else { 0.8 })
                    .slot("Loss", format!("{:.0}", loss * 1000.0))
                    .slot("Lost", lost)
                    .slot("Counterfactual", cf)
                    .cause(format!("{} sits on a fault line", name))
                    .tone(Tone::Tragedy);
                if stress {
                    sd = sd.cause("stress from recent uplift");
                }
                if self.cities[cu].backlog > 30.0 {
                    sd = sd.cause("maintenance backlog worsened the collapse");
                }
                seed = Some(sd);
            }
            19 => {
                self.cities[cu].food_store *= 0.5;
                for &tt in &self.cities[cu].catchment.clone() {
                    self.planet.tiles[tt].ash += 0.5;
                }
                seed = Some(self.base_seed("ashfall", ci, Scope::Local, id, why).cause("ash carried on the wind from a nearby volcano"));
            }
            20 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev);
                self.cities[cu].food_store *= 1.0 - (0.5 * s).min(0.95);
                // neighbors respond
                let nb = self.cities.iter().filter(|o| o.alive && o.civ != civ_id && self.planet.dist2(o.x, o.y, self.cities[cu].x, self.cities[cu].y) < 144.0).map(|o| o.civ).next();
                let resp = nb.map(|n| {
                    let r = self.r(n, civ_id).score;
                    if r > 15.0 {
                        self.ledger(civ_id, n, Reason::PastAid, 10.0, Some(50), "sent grain after the locusts");
                        format!("The {} sent grain.", self.civ_name(n))
                    } else if self.civs[n as usize].leader.traits[T_GREED] > 0.6 {
                        format!("The {} raised grain prices.", self.civ_name(n))
                    } else {
                        format!("The {} closed its markets.", self.civ_name(n))
                    }
                }).unwrap_or_else(|| "No neighbor responded.".into());
                seed = Some(self.base_seed("locusts", ci, Scope::Regional, id, why).slot("Counterfactual", cf).slot("Response", resp).cause("warm weather over farmland").tone(Tone::Alarm));
            }
            21 => {
                self.cities[cu].chains.river_shift = 1;
                self.cities[cu].chains.stage_year = year;
                self.cities[cu].stranded = true;
                self.cities[cu].river = false;
                for r in self.routes.iter_mut() {
                    if r.alive && (r.a == ci || r.b == ci) && !r.sea {
                        r.value *= 0.6;
                    }
                }
                seed = Some(self.base_seed("river_shift", ci, Scope::Regional, id, why)
                    .cause("silt and floods moved the channel away from the docks")
                    .thread(format!("Stranded port of {}", name))
                    .tone(Tone::Irony));
            }
            22 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev);
                self.cities[cu].pop *= 1.0 - (0.05 * s).min(0.3);
                self.cities[cu].backlog += 10.0;
                let resp = if self.civs[cv].knows("Industrialization") && treasury > 100.0 { "commissioned a seawall" } else { "moved families inland" };
                if resp.starts_with("commissioned") && !self.cities[cu].has("Seawall") {
                    self.cities[cu].build = Some(Build::Building(bidx("Seawall")));
                }
                seed = Some(self.base_seed("inundation", ci, Scope::Regional, id, why).slot("Counterfactual", cf).slot("Response", resp).cause(format!("sea level {:+.2} and melting ice", self.planet.sea_level * 10.0)).tone(Tone::Alarm));
            }
            23 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev);
                self.cities[cu].pop *= 1.0 - (0.04 * s).min(0.25);
                self.cities[cu].food_store *= 0.7;
                seed = Some(self.base_seed("cold_snap", ci, Scope::Local, id, why).slot("Counterfactual", cf).cause(format!("a {:.0}°C mean and harsh seasons", self.ctx(ci).temp)).tone(Tone::Tragedy));
            }
            24 => {
                for &tt in &self.cities[cu].catchment.clone() {
                    if !self.planet.is_land(tt) {
                        self.planet.tiles[tt].fish *= 0.2;
                    }
                }
                let resp = if t[T_ECOLOGY] > 0.5 { "imposed fishing quotas" } else { "sent boats to farther grounds" };
                seed = Some(self.base_seed("fishery_collapse", ci, Scope::Local, id, why).slot("Response", resp).cause("years of harvesting above regeneration"));
            }
            25 => {
                self.cities[cu].chains.plague = 1;
                self.cities[cu].chains.stage_year = year;
                self.cities[cu].plague_severity = 0.5 * sev;
                let quarantine = t[T_GREED] < 0.5 || comp > 0.6;
                if quarantine {
                    for r in self.routes.iter_mut() {
                        if r.alive && (r.a == ci || r.b == ci) {
                            r.raided_until = year + 5;
                        }
                    }
                }
                let source = self.routes.iter().find(|r| r.alive && (r.a == ci || r.b == ci)).map(|r| if r.a == ci { r.b } else { r.a });
                self.open_window(ci, "plague", format!("Plague in {}", name), 5, 25.0);
                seed = Some(self.base_seed("plague", ci, Scope::Regional, id, why)
                    .slot("Response", if quarantine { "closed the harbor in quarantine" } else { "kept trade open" })
                    .slot("Source", source.map(|s| self.city_name(s)).unwrap_or_else(|| "abroad".into()))
                    .cause(format!("arrived along trade routes{}", source.map(|s| format!(" from {}", self.city_name(s))).unwrap_or_default()))
                    .thread(format!("Plague at {}", name))
                    .alert(AlertClass::Major)
                    .tone(Tone::Tragedy));
            }
            26 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev);
                self.cities[cu].plague_severity = (0.3 * s).min(0.9);
                self.cities[cu].pop *= 1.0 - (0.05 * s).min(0.3);
                seed = Some(self.base_seed("outbreak", ci, Scope::Local, id, why).slot("Counterfactual", cf).cause(format!("health {:.0} in a crowded city", self.cities[cu].health)).tone(Tone::Tragedy));
            }
            27 => {
                let k = tech_index("Antibiotics");
                self.civs[cv].eureka[k] = true;
                let s_ = tech_index("Sanitation");
                self.civs[cv].eureka[s_] = true;
                for o in self.cities.iter_mut().filter(|o| o.civ == civ_id) {
                    o.plague_severity *= 0.5;
                }
                seed = Some(self.base_seed("breakthrough", ci, Scope::National, id, why).cause("funded physicians and scholars").tone(Tone::Triumph));
            }
            28 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev);
                let cover = self.cities[cu].governor.corruption > 0.3;
                self.cities[cu].health -= 15.0 * s;
                self.cities[cu].plague_severity = (0.2 * s).min(0.8);
                if cover {
                    self.cities[cu].unrest += 10.0;
                }
                seed = Some(self.base_seed("contaminated_water", ci, Scope::Local, id, why).slot("Counterfactual", cf).slot("Response", if cover { "covered it up — a scandal" } else { "built new waterworks" }).cause("factory runoff upstream").tone(Tone::Alarm));
            }
            30 => {
                let fraud = self.cities[cu].wealth >= 650.0 && self.rng.gen_bool(0.4);
                if fraud {
                    self.civs[cv].treasury -= 20.0;
                } else {
                    self.cities[cu].faith_acc += 30.0;
                    self.civs[cv].research_progress += 20.0;
                }
                seed = Some(self.base_seed("healer", ci, Scope::Local, id, why).slot("Result", if fraud { "turned out to be a swindle" } else { "drew pilgrims and scholars" }).cause(if fraud { "rich patrons attract frauds" } else { "a famous healer's reputation" }));
            }
            33 => {
                let resp = if t[T_LOYALTY] > 0.6 { "pardoned" } else if comp > 0.5 { "purged" } else { "scapegoated a clerk" };
                let stolen = self.cities[cu].wealth * 0.05 + 5.0;
                if resp == "purged" {
                    self.cities[cu].governor.corruption = 0.05;
                    self.cities[cu].governor.name = self.civs[cv].lang.person(&mut self.rng);
                }
                seed = Some(self.base_seed("corruption", ci, Scope::Local, id, why).slot("Stolen", format!("{:.0}", stolen)).slot("Response", resp).cause(format!("Governor's corruption of {:.0}%", self.cities[cu].governor.corruption * 100.0)));
            }
            34 => {
                let grant = t[T_CURIOSITY] > 0.5;
                if grant {
                    self.civs[cv].civic_progress += 40.0;
                } else {
                    self.cities[cu].unrest += 10.0;
                }
                seed = Some(self.base_seed("reform", ci, Scope::National, id, why).slot("Response", if grant { "granted reforms" } else { "refused reform" }).cause("a literate public demanding a voice"));
            }
            35 => {
                let rich = self.ctx(ci).rich_nb.unwrap();
                self.cities[cu].unrest += 15.0;
                self.cities[cu].migration += 0.5;
                let rc = self.cities[rich as usize].civ;
                seed = Some(self.base_seed("agitation", ci, Scope::National, id, why)
                    .slot("Rich", self.city_name(rich))
                    .slot("RichCiv", self.civ_name(rc))
                    .cause(format!("pamphlets compare {} ({}) with {} ({})", name, self.cities[cu].band(), self.city_name(rich), self.cities[rich as usize].band()))
                    .tone(Tone::Alarm));
            }
            36 => {
                let success = self.rng.gen_bool(0.4);
                if success {
                    let old = self.civs[cv].leader.clone();
                    let mut lang = std::mem::replace(&mut self.civs[cv].lang, names::Language::generate(&mut self.rng));
                    let mut nl = gen_leader(&mut lang, &mut self.rng, Some(&old.traits), year, self.civs[cv].era);
                    self.civs[cv].lang = lang;
                    nl.traits[T_AGGRESSION] = (nl.traits[T_AGGRESSION] + 0.25).min(1.0);
                    nl.title = "Marshal".into();
                    self.civs[cv].leaders_past.push((format!("{} {}", old.title, old.name), old.reign_start, year, "overthrown in a coup".into()));
                    self.civs[cv].leader = nl;
                }
                seed = Some(self.base_seed("coup", ci, Scope::National, id, why).slot("Result", if success { format!("succeeded; Marshal {} now rules", self.civs[cv].leader.name) } else { "failed; loyalists purged the plotters".into() }).cause("a strong army and an unpopular ruler").alert(AlertClass::Major));
            }
            37 => {
                self.cities[cu].golden_until = year + 10;
                for o in self.cities.iter_mut().filter(|o| o.civ == civ_id) {
                    o.golden_until = o.golden_until.max(year + 6);
                }
                seed = Some(self.base_seed("golden_age", ci, Scope::National, id, why).cause("sustained prosperity and stability").tone(Tone::Triumph).card());
            }
            38 => {
                self.cities[cu].backlog += 30.0;
                seed = Some(self.base_seed("ossification", ci, Scope::National, id, why).cause("centuries of accumulated offices").tone(Tone::Irony));
            }
            39 => {
                self.cities[cu].relic = true;
                self.cities[cu].chains.relic = 1;
                self.cities[cu].chains.stage_year = year;
                let relic = format!("the {} of {}", ["Bone", "Crown", "Lamp", "Tablet", "Mask"][self.rng.gen_range(0..5)], self.civs[cv].lang.word(&mut self.rng, 1, 2));
                seed = Some(self.base_seed("relic", ci, Scope::Regional, id, why).slot("Relic", relic).cause("diggers in old ground near the holy places").thread(format!("Relic of {}", name)));
            }
            40 => {
                self.cities[cu].relic = false;
                let thief = (0..self.civs.len() as u16).filter(|&o| o != civ_id && self.civs[o as usize].alive && self.r(civ_id, o).met).max_by(|a, b| self.civs[*a as usize].leader.traits[T_PIETY].partial_cmp(&self.civs[*b as usize].leader.traits[T_PIETY]).unwrap());
                if let Some(th) = thief {
                    self.ledger(civ_id, th, Reason::WarGrievance, -25.0, Some(80), "stole our relic");
                    if let Some(cap) = self.civs[th as usize].capital {
                        self.cities[cap as usize].relic = true;
                    }
                }
                seed = Some(self.base_seed("relic_theft", ci, Scope::National, id, why)
                    .slot("Thief", thief.map(|t| self.civ_name(t)).unwrap_or_else(|| "unknown hands".into()))
                    .cause(format!("pilgrimage wealth made {} a {} target", name, self.cities[cu].band().to_lowercase()))
                    .thread(format!("Relic of {}", name))
                    .tone(Tone::Alarm));
            }
            41 => {
                let old = self.cities[cu].religion.unwrap();
                let rid = self.religions.len();
                let rname = format!("Reformed {}", self.religions[old].name);
                self.religions.push(Religion { name: rname.clone(), theme: self.religions[old].theme, founder: civ_id, holy_city: ci, founded: year, beliefs: self.religions[old].beliefs.clone(), parent: Some(old), shaper_cult: false });
                for &oc in &self.civs[cv].cities.clone() {
                    if self.cities[oc as usize].religion == Some(old) {
                        self.cities[oc as usize].religion = Some(rid);
                    }
                }
                self.civs[cv].religion = Some(rid);
                seed = Some(self.base_seed("schism", ci, Scope::Global, id, why).slot("Old", self.religions[old].name.clone()).slot("New", rname).cause("distant converts resisted the founders' authority").card());
            }
            42 => {
                self.cities[cu].faith_acc += 60.0;
                for o in self.cities.iter_mut().filter(|o| o.civ == civ_id) {
                    o.faith_acc += 10.0;
                }
                seed = Some(self.base_seed("awakening", ci, Scope::Regional, id, why).cause("suffering turned the people to prayer"));
            }
            44 => {
                let person = self.civs[cv].lang.person(&mut self.rng);
                let field = ["painter", "architect", "philosopher", "engineer", "poet", "astronomer"][self.rng.gen_range(0..6)];
                self.cities[cu].culture += 60.0;
                self.civs[cv].research_progress += 30.0;
                seed = Some(self.base_seed("great_person", ci, Scope::National, id, why).slot("Person", person).slot("Field", field).cause(format!("patronage in a {} city", self.cities[cu].band().to_lowercase())).tone(Tone::Triumph));
            }
            45 => {
                let ban = t[T_CURIOSITY] < 0.4;
                if ban {
                    self.cities[cu].unrest += 8.0;
                } else {
                    self.civs[cv].civic_progress += 50.0;
                }
                let k = tech_index("Printing");
                self.civs[cv].eureka[k] = true;
                seed = Some(self.base_seed("forbidden_idea", ci, Scope::National, id, why).slot("Response", if ban { "banned the book" } else { "adopted its ideas" }).cause("a book carried in by merchants"));
            }
            46 => {
                let loot = self.cities[cu].wealth * 0.06;
                self.civs[cv].treasury -= loot;
                let resp = if treasury > loot * 2.0 && t[T_AGGRESSION] < 0.5 { "paid them off" } else { "sent soldiers to hunt them" };
                for r in self.routes.iter_mut() {
                    if r.alive && (r.a == ci || r.b == ci) {
                        r.raided_until = year + 3;
                    }
                }
                seed = Some(self.base_seed("raid_road", ci, Scope::Local, id, why).slot("Loot", format!("{:.0}", loot)).slot("Response", resp).cause(format!("a {} city with a garrison of {:.0}", self.cities[cu].band().to_lowercase(), self.cities[cu].garrison)).tone(Tone::Alarm));
            }
            47 => {
                let h = self.ctx(ci).hostile.unwrap();
                let escalate = t[T_AGGRESSION] > 0.55;
                self.ledger(civ_id, h, Reason::BorderFriction, if escalate { -15.0 } else { -5.0 }, Some(30), "border incident");
                self.ledger(h, civ_id, Reason::BorderFriction, -10.0, Some(30), "border incident");
                seed = Some(self.base_seed("border_incident", ci, Scope::National, id, why).slot("Other", self.civ_name(h)).slot("Response", if escalate { "escalated" } else { "paid compensation" }).civ2(h).cause("a contested border with hostile neighbors"));
            }
            49 => {
                let price = 30.0 + self.cities[cu].wealth * 0.05;
                let hire = treasury > price * 1.5;
                if hire {
                    self.civs[cv].treasury -= price;
                    let aid = self.next_army;
                    self.next_army += 1;
                    let tier = self.civs[cv].military_tier();
                    self.armies.push(Army { id: aid, owner: Some(civ_id), x: self.cities[cu].x as f32, y: self.cities[cu].y as f32, strength: 8.0 * tier, target: None, enemy: None, alive: true, loot: 0.0, home: Some(ci), sieging: false });
                }
                let company = format!("the {} Company", self.world_lang.word(&mut self.rng, 1, 2));
                seed = Some(self.base_seed("mercenaries", ci, Scope::Local, id, why).slot("Company", company).slot("Price", format!("{:.0}", price)).slot("Response", if hire { "hired them" } else { "turned them away" }).cause("a full treasury draws free companies"));
            }
            50 => {
                self.armies.iter_mut().filter(|a| a.owner == Some(civ_id) && a.alive).for_each(|a| a.strength *= 1.15);
                seed = Some(self.base_seed("arms_cache", ci, Scope::National, id, why).cause("old ruins or a new ore seam near the city"));
            }
            52 => {
                let (s, cf) = self.pattern_b(ci, 1.0, sev);
                let deaths = (self.cities[cu].pop * 0.03 * s).min(3.0);
                self.cities[cu].pop -= deaths;
                let i = self.planet.idx(self.cities[cu].x, self.cities[cu].y).unwrap();
                self.planet.tiles[i].pollution = (self.planet.tiles[i].pollution + 0.3).min(1.0);
                self.cities[cu].chains.accident = 1;
                self.cities[cu].chains.stage_year = year;
                seed = Some(self.base_seed("industrial_accident", ci, Scope::National, id, why)
                    .slot("Deaths", format!("{:.0}", deaths * 1000.0))
                    .slot("Counterfactual", cf)
                    .cause(if self.cities[cu].backlog > 30.0 { "worn machinery (maintenance backlog)" } else { "rushed production" })
                    .thread(format!("Industrial accident at {}", name))
                    .tone(Tone::Tragedy));
            }
            54 => {
                let science = self.civs[cv].leader.traits[T_CURIOSITY] > 0.5 || self.civs[cv].ability == "Star Readers";
                if science {
                    let k = tech_index("Astronomy");
                    self.civs[cv].eureka[k] = true;
                } else {
                    self.cities[cu].faith_acc += 40.0;
                }
                seed = Some(self.base_seed("omen", ci, Scope::National, id, why)
                    .slot("Reading", if science { "astronomers record it as a measurable phenomenon" } else { "priests read it as a sign from the heavens" })
                    .cause("a strange light in the sky"));
            }
            57 => {
                let r = self.rng.gen_range(0..4);
                let result = match r {
                    0 => {
                        let k = (0..TECHS.len()).find(|&k| !self.civs[cv].known_tech[k]);
                        if let Some(k) = k {
                            self.civs[cv].eureka[k] = true;
                        }
                        "returned with forgotten knowledge"
                    }
                    1 => {
                        self.cities[cu].relic = true;
                        "returned with a relic"
                    }
                    2 => {
                        self.civs[cv].treasury += 40.0;
                        "returned with gold and maps"
                    }
                    _ => "returned with nothing but stories",
                };
                self.civs[cv].treasury -= 25.0;
                seed = Some(self.base_seed("expedition", ci, Scope::Local, id, why).slot("Result", result).cause(format!("{} could afford to fund it", name)));
            }
            59 => {
                let i = self.planet.idx(self.cities[cu].x, self.cities[cu].y).unwrap();
                for tt in self.planet.tiles_in_radius(self.cities[cu].x, self.cities[cu].y, 3) {
                    self.planet.tiles[tt].pollution = (self.planet.tiles[tt].pollution + 0.25).min(1.0);
                }
                self.planet.tiles[i].biodiv *= 0.5;
                self.cities[cu].chains.accident = 1;
                self.cities[cu].chains.stage_year = year;
                let proj = ["orbital mirror", "gene-drive program", "machine foundry", "weather engine"][self.rng.gen_range(0..4)];
                seed = Some(self.base_seed("runaway", ci, Scope::Global, id, why).slot("Project", proj).cause("advanced technology outran its safeguards").tone(Tone::Tragedy));
            }
            _ => {}
        }
        if let Some(s) = seed {
            if protected {
                self.seeds.push(s.cause("worst outcome softened by an unseen protection"));
            } else {
                self.seeds.push(s);
            }
        }
    }

    fn advance_chains(&mut self) {
        let year = self.year;
        let sev = self.difficulty.severity();
        let n = self.cities.len();
        for ci in 0..n {
            if !self.cities[ci].alive {
                continue;
            }
            let age = year - self.cities[ci].chains.stage_year;
            let name = self.cities[ci].name.clone();
            let civ_id = self.cities[ci].civ;
            let cu = ci as u32;
            // ---- Drought -> famine -> exodus -> conflict or relief
            match self.cities[ci].chains.drought {
                1 if age >= 3 => {
                    let i = self.planet.idx(self.cities[ci].x, self.cities[ci].y).unwrap();
                    if self.planet.tiles[i].rain > 0.45 || self.rng.gen_bool(0.3) {
                        self.cities[ci].chains.drought = 0;
                        let s = self.base_seed("drought_ends", cu, Scope::Local, 16, vec![]).cause("rains returned").thread(format!("Drought at {}", name));
                        self.seeds.push(s);
                    } else {
                        self.cities[ci].chains.drought = 2;
                        self.cities[ci].chains.stage_year = year;
                        let (s_, cf) = self.pattern_b(cu, 1.0, sev);
                        if self.cities[ci].wealth < 450.0 {
                            self.cities[ci].food_store = -self.cities[ci].pop * 0.5 * s_;
                        }
                        self.civs[civ_id as usize].famine_memory += 1.0;
                        let k = tech_index("Feudal Farming");
                        self.civs[civ_id as usize].eureka[k] = true;
                        let rich = self.cities.iter().filter(|o| o.alive && o.id != cu && o.wealth >= 650.0).min_by_key(|o| self.planet.dist2(o.x, o.y, self.cities[ci].x, self.cities[ci].y) as i32).map(|o| (o.name.clone(), o.wealth));
                        let mut s = self.base_seed("famine", cu, Scope::Regional, 29, vec!["Chain: Drought stage 2".into()])
                            .slot("FailedHarvests", "three")
                            .slot("Counterfactual", cf)
                            .slot("Response", if self.cities[ci].wealth >= 450.0 { "Imports and high prices kept most fed" } else { "Rationing failed and people went hungry" })
                            .cause("three years of failed harvests in the drought")
                            .thread(format!("Drought at {}", name))
                            .tone(Tone::Tragedy)
                            .card();
                        if let Some((rn, rw)) = rich {
                            s = s.slot("RichNeighbor", rn).slot("Gap", format!("{:.0}", rw - self.cities[ci].wealth));
                        }
                        self.seeds.push(s);
                        self.open_window(cu, "famine", format!("Famine in {}", name), 4, 20.0);
                    }
                }
                2 if age >= 3 => {
                    // exodus: migrants to the nearest richer city
                    self.cities[ci].chains.drought = 3;
                    self.cities[ci].chains.stage_year = year;
                    let moved = self.cities[ci].pop * 0.15;
                    self.cities[ci].pop -= moved;
                    let dest = self.cities.iter().filter(|o| o.alive && o.id != cu && o.wealth > self.cities[ci].wealth).min_by_key(|o| self.planet.dist2(o.x, o.y, self.cities[ci].x, self.cities[ci].y) as i32).map(|o| o.id);
                    if let Some(d) = dest {
                        let open = self.civs[self.cities[d as usize].civ as usize].ability == "Open Hands";
                        self.cities[d as usize].pop += moved * 0.7;
                        self.cities[d as usize].migration += moved;
                        if !open {
                            self.cities[d as usize].unrest += 6.0;
                        }
                        let dciv = self.cities[d as usize].civ;
                        let s = self.base_seed("exodus", cu, Scope::Regional, 51, vec!["Chain: Drought stage 3".into()])
                            .slot("Dest", self.city_name(d))
                            .slot("DestCiv", self.civ_name(dciv))
                            .slot("Moved", format!("{:.0}", moved * 1000.0))
                            .civ2(dciv)
                            .cause("famine and a richer city within reach")
                            .thread(format!("Drought at {}", name))
                            .card();
                        self.seeds.push(s);
                        if dciv != civ_id && self.civs[dciv as usize].leader.traits[T_AGGRESSION] > 0.6 {
                            self.ledger(dciv, civ_id, Reason::BorderFriction, -15.0, Some(40), "refugees crossed our border");
                        }
                    }
                }
                3 if age >= 4 => {
                    self.cities[ci].chains.drought = 0;
                    let relief = self.civs[civ_id as usize].treasury > 60.0;
                    if relief {
                        self.civs[civ_id as usize].treasury -= 40.0;
                        self.cities[ci].build = Some(Build::Project("Relief Works"));
                    }
                    let s = self.base_seed(if relief { "drought_relief" } else { "dust_bowl" }, cu, Scope::Local, 16, vec!["Chain: Drought stage 4".into()])
                        .cause(if relief { "a treasury able to fund relief works" } else { "an empty treasury" })
                        .thread(format!("Drought at {}", name));
                    self.seeds.push(s);
                }
                _ => {}
            }
            // ---- Gold rush -> boomtown -> bust or banking center
            match self.cities[ci].chains.gold_rush {
                1 if age >= 8 => {
                    self.cities[ci].chains.gold_rush = 2;
                    self.cities[ci].chains.stage_year = year;
                    self.cities[ci].pop += 2.0;
                    let s = self.base_seed("boomtown", cu, Scope::Local, 4, vec!["Chain: Gold rush stage 2".into()]).cause("miners and merchants flocked to the strike").thread(format!("Gold rush at {}", name));
                    self.seeds.push(s);
                }
                2 if age >= 12 => {
                    self.cities[ci].chains.gold_rush = 0;
                    if self.cities[ci].wealth >= 650.0 || self.cities[ci].has("Bank") {
                        self.cities[ci].buildings[bidx("Market")] = true;
                        let s = self.base_seed("banking_center", cu, Scope::National, 4, vec!["Chain: Gold rush stage 3 (Rich: pivot)".into()]).cause("rush profits reinvested in banking").thread(format!("Gold rush at {}", name)).tone(Tone::Triumph).card();
                        self.seeds.push(s);
                    } else {
                        let (s_, cf) = self.pattern_b(cu, 1.0, sev);
                        self.cities[ci].pop *= 1.0 - (0.15 * s_).min(0.5);
                        self.cities[ci].debt += 40.0;
                        let s = self.base_seed("boomtown_bust", cu, Scope::Local, 5, vec!["Chain: Gold rush stage 3 (bust)".into()]).slot("Counterfactual", cf).cause("the seam ran thin and money left").thread(format!("Gold rush at {}", name)).tone(Tone::Irony);
                        self.seeds.push(s);
                    }
                }
                _ => {}
            }
            // ---- Plague ship -> quarantine politics -> breakthrough or scapegoating
            match self.cities[ci].chains.plague {
                1 if age >= 2 => {
                    self.cities[ci].chains.plague = 2;
                    self.cities[ci].chains.stage_year = year;
                    let (s_, cf) = self.pattern_b(cu, 1.0, sev);
                    let med = if self.civs[civ_id as usize].knows("Sanitation") { 0.5 } else { 1.0 };
                    let loss = (self.cities[ci].pop * 0.12 * s_ * med).min(self.cities[ci].pop * 0.4);
                    self.cities[ci].pop -= loss;
                    self.cities[ci].plague_severity = 0.7;
                    // spread along routes
                    let partners: Vec<u32> = self.routes.iter().filter(|r| r.alive && r.raided_until <= year && (r.a == cu || r.b == cu)).map(|r| if r.a == cu { r.b } else { r.a }).collect();
                    let mut spread = Vec::new();
                    for p in partners {
                        let pc = &mut self.cities[p as usize];
                        if pc.chains.plague == 0 && self.rng.gen_bool(0.3) {
                            pc.chains.plague = 1;
                            pc.chains.stage_year = year;
                            pc.plague_severity = 0.4;
                            spread.push(pc.name.clone());
                        }
                    }
                    let s = self.base_seed("plague_peak", cu, Scope::Regional, 25, vec!["Chain: Plague stage 2".into()])
                        .slot("Loss", format!("{:.0}", loss * 1000.0))
                        .slot("Counterfactual", cf)
                        .slot("Spread", if spread.is_empty() { "no other city".into() } else { spread.join(", ") })
                        .cause("contagion in crowded streets")
                        .thread(format!("Plague at {}", name))
                        .tone(Tone::Tragedy)
                        .card();
                    self.seeds.push(s);
                }
                2 if age >= 3 => {
                    self.cities[ci].chains.plague = 0;
                    let k = tech_index("Sanitation");
                    self.civs[civ_id as usize].eureka[k] = true;
                    let k2 = tech_index("Antibiotics");
                    self.civs[civ_id as usize].eureka[k2] = true;
                    let scape = self.civs[civ_id as usize].leader.traits[T_CURIOSITY] < 0.35;
                    let mut s = self.base_seed(if scape { "plague_scapegoat" } else { "plague_recedes" }, cu, Scope::Local, 25, vec!["Chain: Plague stage 3".into()])
                        .cause("survivors' immunity and quarantine")
                        .thread(format!("Plague at {}", name));
                    if scape {
                        let target = (0..self.civs.len() as u16).find(|&o| o != civ_id && self.civs[o as usize].alive && self.r(civ_id, o).met);
                        if let Some(tg) = target {
                            self.ledger(civ_id, tg, Reason::WarGrievance, -15.0, Some(40), "blamed for the plague");
                            s = s.slot("Scapegoat", self.civ_name(tg)).civ2(tg);
                        }
                    }
                    self.seeds.push(s);
                    self.civs[civ_id as usize].defining_moments.push(format!("survived plague at {}", name));
                }
                _ => {}
            }
            // ---- Relic -> pilgrimage boom
            if self.cities[ci].chains.relic == 1 && age >= 10 {
                self.cities[ci].chains.relic = 2;
                self.cities[ci].faith_acc += 50.0;
                let s = self.base_seed("pilgrimage", cu, Scope::Local, 39, vec!["Chain: Relic stage 2".into()]).cause("pilgrims follow the relic").thread(format!("Relic of {}", name));
                self.seeds.push(s);
            }
            // ---- River shifts -> canal or decline
            if self.cities[ci].chains.river_shift == 1 && age >= 6 {
                self.cities[ci].chains.river_shift = 0;
                let canal = self.civs[civ_id as usize].treasury > 80.0 && self.civs[civ_id as usize].knows("Engineering");
                if canal {
                    self.civs[civ_id as usize].treasury -= 60.0;
                    self.cities[ci].river = true;
                    self.cities[ci].stranded = false;
                } else {
                    self.cities[ci].pop *= 0.9;
                }
                let s = self.base_seed(if canal { "canal_built" } else { "port_decline" }, cu, Scope::Regional, 21, vec!["Chain: River shifts stage 2".into()])
                    .cause(if canal { "engineering and a funded treasury" } else { "no means to dig a canal" })
                    .thread(format!("Stranded port of {}", name))
                    .tone(if canal { Tone::Triumph } else { Tone::Irony });
                self.seeds.push(s);
            }
            // ---- Industrial accident -> reform or cover-up
            if self.cities[ci].chains.accident == 1 && age >= 4 {
                self.cities[ci].chains.accident = 0;
                let reform = self.civs[civ_id as usize].leader.traits[T_ECOLOGY] > 0.45 || self.civs[civ_id as usize].gov().name == "Democracy";
                if reform {
                    for tt in self.planet.tiles_in_radius(self.cities[ci].x, self.cities[ci].y, 2) {
                        self.planet.tiles[tt].pollution *= 0.5;
                    }
                    self.civs[civ_id as usize].leader.traits[T_ECOLOGY] = (self.civs[civ_id as usize].leader.traits[T_ECOLOGY] + 0.1).min(1.0);
                } else {
                    self.cities[ci].unrest += 12.0;
                }
                let s = self.base_seed(if reform { "safety_reform" } else { "coverup" }, cu, Scope::National, 52, vec!["Chain: Industrial accident stage 2".into()])
                    .cause(if reform { "public outrage met a responsive government" } else { "officials buried the reports" })
                    .thread(format!("Industrial accident at {}", name));
                self.seeds.push(s);
            }
            // ---- Succession crisis -> civil war or reunification
            match self.cities[ci].chains.succession {
                1 if age >= 3 => {
                    self.cities[ci].chains.succession = 2;
                    self.cities[ci].chains.stage_year = year;
                    for &oc in &self.civs[civ_id as usize].cities.clone() {
                        self.cities[oc as usize].unrest += 8.0;
                    }
                }
                2 if age >= 8 => {
                    self.cities[ci].chains.succession = 0;
                    let s = self.base_seed("succession_resolved", cu, Scope::National, 31, vec!["Chain: Succession stage 3".into()]).cause("factions reconciled under the new ruler").thread(format!("Succession of the {}", self.civ_name(civ_id)));
                    self.seeds.push(s);
                }
                _ => {}
            }
            // ---- Predatory loan comes due
            if self.cities[ci].debt > 0.0 {
                if let Some(lender) = self.cities[ci].debt_lender {
                    if year - self.cities[ci].chains.stage_year > 15 && self.rng.gen_bool(0.1) {
                        let can_pay = self.civs[civ_id as usize].treasury > self.cities[ci].debt;
                        if can_pay {
                            self.civs[civ_id as usize].treasury -= self.cities[ci].debt;
                            self.civs[lender as usize].treasury += self.cities[ci].debt;
                            self.cities[ci].debt = 0.0;
                            self.cities[ci].debt_lender = None;
                        } else if self.civs[lender as usize].alive && lender != civ_id && !self.cities[ci].is_capital && self.civs[lender as usize].leader.traits[T_GREED] > 0.5 {
                            let s = self.base_seed("debt_takeover", cu, Scope::National, 8, vec!["Chain: Predatory loan stage 2".into()])
                                .slot("Lender", self.civ_name(lender))
                                .civ2(lender)
                                .cause("an unpaid loan from a richer neighbor")
                                .card();
                            self.seeds.push(s);
                            self.cities[ci].debt = 0.0;
                            self.cities[ci].debt_lender = None;
                            self.ledger(civ_id, lender, Reason::CityTaken, -30.0, Some(80), "seized a city for debt");
                            self.transfer_city(cu, lender);
                        } else {
                            self.cities[ci].debt *= 0.5;
                        }
                    }
                }
            }
        }
    }

    pub fn eruption(&mut self, t: usize) {
        let (x, y) = self.planet.xy(t);
        let mag = self.planet.tiles[t].volcano * self.rng.gen_range(0.5..1.5);
        self.planet.particulates += mag * 0.3;
        self.planet.co2 += mag * 0.01;
        for i in self.planet.tiles_in_radius(x, y, 3) {
            let tile = &mut self.planet.tiles[i];
            tile.ash = (tile.ash + 0.8 * mag).min(1.5);
            tile.forest *= 0.7;
        }
        if self.planet.tiles[t].elev < self.planet.sea_level + 0.05 {
            self.planet.tiles[t].elev = self.planet.sea_level + 0.06;
        }
        let mut hit = Vec::new();
        for c in 0..self.cities.len() {
            let city = &self.cities[c];
            if city.alive && self.planet.dist2(x, y, city.x, city.y) < 16.0 {
                let protected = city.protected_until > self.year;
                let (s, _) = self.pattern_b(c as u32, mag, self.difficulty.severity() * if protected { 0.3 } else { 1.0 });
                let city = &mut self.cities[c];
                city.pop *= 1.0 - (0.08 * s).min(0.5);
                city.food_store *= 0.4;
                city.backlog += 15.0 * s;
                hit.push(city.name.clone());
            }
        }
        if !hit.is_empty() || mag > 0.8 {
            let s = StorySeed::new("eruption", if hit.is_empty() { Scope::Regional } else { Scope::Global })
                .slot("Region", self.region_name(x, y))
                .slot("Cities", if hit.is_empty() { "no cities".into() } else { hit.join(", ") })
                .at(x, y)
                .cause(format!("an active volcano (activity {:.1})", self.planet.tiles[t].volcano))
                .cat("ecological")
                .tone(Tone::Tragedy)
                .why(vec!["Geological hazard: volcano activity × uplift stress".into()])
                .card();
            let s = if self.planet.tiles[t].uplift_stress > 0.2 { s.cause("stress from recent uplift") } else { s };
            self.seeds.push(s);
        }
    }

    pub fn meteor(&mut self, at: Option<usize>, player: bool) {
        let t = at.unwrap_or_else(|| self.rng.gen_range(0..self.planet.tiles.len()));
        let (x, y) = self.planet.xy(t);
        let big = player;
        let r = if big { 3 } else { 2 };
        for i in self.planet.tiles_in_radius(x, y, r) {
            let tile = &mut self.planet.tiles[i];
            tile.forest = 0.0;
            tile.ash = 1.5;
            tile.biodiv *= 0.3;
            tile.scorch = 1.0;
        }
        self.planet.tiles[t].elev -= 0.08;
        self.planet.tiles[t].resource = Some(Resource { kind: ResKind::Gems, richness: 1.5, stock: 400.0 });
        self.planet.particulates += if big { 2.0 } else { 0.6 };
        let mut hit = Vec::new();
        for c in 0..self.cities.len() {
            let city = &self.cities[c];
            if city.alive && self.planet.dist2(x, y, city.x, city.y) <= (r * r) as f32 + 1.0 {
                let protected = city.protected_until > self.year;
                let city = &mut self.cities[c];
                if protected {
                    city.pop *= 0.85;
                } else {
                    city.pop *= if big { 0.3 } else { 0.6 };
                }
                city.backlog += 50.0;
                hit.push((c as u32, city.name.clone(), city.civ));
            }
        }
        // every civilization argues about what it meant
        let mut interpretations = Vec::new();
        for c in self.civs.iter().filter(|c| c.alive && !c.minor).take(3) {
            let piety = c.leader.traits[T_PIETY];
            let reading = if piety > 0.65 {
                "a punishment from the heavens"
            } else if piety > 0.4 {
                "an omen of change"
            } else {
                "a stone from the sky, nothing more"
            };
            interpretations.push(format!("the {} call it {}", c.name, reading));
        }
        for i in self.planet.tiles_in_radius(x, y, 8) {
            self.planet.tiles[i].attune += if player { 2.0 } else { 0.5 };
        }
        let mut s = StorySeed::new("meteor", Scope::Global)
            .slot("Region", self.region_name(x, y))
            .slot("Cities", if hit.is_empty() { "no city".into() } else { hit.iter().map(|h| h.1.clone()).collect::<Vec<_>>().join(", ") })
            .slot("Readings", if interpretations.is_empty() { "no one saw it".into() } else { interpretations.join("; ") })
            .at(x, y)
            .cat("cosmic")
            .tone(Tone::Tragedy)
            .why(vec![if player { "Player intervention: meteor strike".into() } else { format!("Cosmic volatility roll ({:.2})", self.cosmic) }])
            .alert(if hit.is_empty() { AlertClass::Major } else { AlertClass::Existential })
            .chronicle();
        s = if player { s.cause("a power beyond the world struck the land") } else { s.cause("a random impact from space") };
        if let Some(h) = hit.first() {
            s = s.city(h.0).civ(h.2);
        }
        self.seeds.push(s);
    }

    fn solar_anomaly(&mut self) {
        let warm = self.rng.gen_bool(0.5);
        let d = if warm { 1.5 } else { -1.5 } * self.difficulty.severity();
        for t in self.planet.tiles.iter_mut() {
            t.temp_nudge += d;
        }
        let s = StorySeed::new("solar_anomaly", Scope::Global)
            .slot("Kind", if warm { "warm" } else { "cold" })
            .slot("Effect", if warm { "bumper harvests in the north, scorched fields in the south" } else { "frosts in high summer and failing grain" })
            .cause(format!("an excursion in the sun's output ({:+.1}°C)", d))
            .cat("cosmic")
            .why(vec!["Climate model excursion (1.2%/year)".into()])
            .card();
        self.seeds.push(s);
    }

    fn departure(&mut self, c: u16) {
        let s = StorySeed::new("departure", Scope::Global)
            .slot("Civ", self.civ_name(c))
            .slot("Leader", self.civs[c as usize].leader.name.clone())
            .civ(c)
            .cause("Starship technology and an opulent treasury")
            .cat("technological")
            .tone(Tone::Triumph)
            .alert(AlertClass::Existential)
            .chronicle();
        self.seeds.push(s);
        self.civs[c as usize].defining_moments.push("left the world for the stars".into());
        self.ended = Some(format!("The {} launched the departure project: a planetary terminus reached on a high note.", self.civ_name(c)));
    }
}
