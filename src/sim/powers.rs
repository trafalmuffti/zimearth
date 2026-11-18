//! Player powers and the intervention economy (design doc section 10).

use super::civ::*;
use super::news::*;
use super::planet::*;
use super::*;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Power {
    RaiseLand,
    LowerLand,
    Warm,
    Cool,
    Wetten,
    Dry,
    SeedForest,
    CalmVolcano,
    StirVolcano,
    Bounty,
    Protect,
    Inspire,
    Dream,
    Cure,
    Meteor,
    GreatQuake,
}

pub const ALL_POWERS: [Power; 16] = [
    Power::RaiseLand,
    Power::LowerLand,
    Power::CalmVolcano,
    Power::StirVolcano,
    Power::Warm,
    Power::Cool,
    Power::Wetten,
    Power::Dry,
    Power::SeedForest,
    Power::Bounty,
    Power::Protect,
    Power::Inspire,
    Power::Dream,
    Power::Cure,
    Power::Meteor,
    Power::GreatQuake,
];

impl Power {
    pub fn name(self) -> &'static str {
        match self {
            Power::RaiseLand => "Raise land",
            Power::LowerLand => "Lower land",
            Power::Warm => "Warm region",
            Power::Cool => "Cool region",
            Power::Wetten => "Bring rain",
            Power::Dry => "Dry region",
            Power::SeedForest => "Seed forest",
            Power::CalmVolcano => "Calm volcano",
            Power::StirVolcano => "Stir volcano",
            Power::Bounty => "Bountiful land",
            Power::Protect => "Protect city",
            Power::Inspire => "Inspire eureka",
            Power::Dream => "Grant a dream",
            Power::Cure => "Cure disease",
            Power::Meteor => "Meteor strike",
            Power::GreatQuake => "Great quake",
        }
    }
    pub fn category(self) -> &'static str {
        match self {
            Power::RaiseLand | Power::LowerLand | Power::CalmVolcano | Power::StirVolcano => "Geological",
            Power::Warm | Power::Cool | Power::Wetten | Power::Dry => "Climatic",
            Power::SeedForest | Power::Bounty | Power::Cure => "Biological",
            Power::Protect | Power::Inspire | Power::Dream => "Subtle",
            Power::Meteor | Power::GreatQuake => "Disaster",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            Power::RaiseLand => "Uplift terrain in a small radius. Permanent; builds fault stress (quakes, eruptions).",
            Power::LowerLand => "Sink terrain; can open straits or drown coasts. Permanent.",
            Power::Warm => "Nudge regional temperature up. Decays as the climate pushes back.",
            Power::Cool => "Nudge regional temperature down. Decays over decades.",
            Power::Wetten => "Increase regional rainfall; can end droughts. Temporary.",
            Power::Dry => "Reduce regional rainfall. Temporary; can start droughts.",
            Power::SeedForest => "Plant forest and restore biodiversity. Forests carry rain downwind.",
            Power::CalmVolcano => "Reduce a volcano's activity.",
            Power::StirVolcano => "Raise a volcano's activity. Ash enriches soil — eventually.",
            Power::Bounty => "Restore soil, game and fish. Settlers will perceive the land as blessed.",
            Power::Protect => "Shield one city from the worst outcome of events for 15 years.",
            Power::Inspire => "Set up a eureka for the civilization owning this city.",
            Power::Dream => "A leader dreams: nudges their next strategic decision. Deniable.",
            Power::Cure => "End plague in this city and its trade partners.",
            Power::Meteor => "Devastating impact. Globally attributed; heavy stewardship cost.",
            Power::GreatQuake => "Shake a fault zone. Destructive and attributable.",
        }
    }
    pub fn needs_city(self) -> bool {
        matches!(self, Power::Protect | Power::Inspire | Power::Dream | Power::Cure)
    }
    pub fn destructive(self) -> bool {
        matches!(self, Power::Meteor | Power::GreatQuake | Power::StirVolcano)
    }
    pub fn base_cost(self) -> f32 {
        match self {
            Power::RaiseLand | Power::LowerLand => 30.0,
            Power::Warm | Power::Cool | Power::Wetten | Power::Dry => 8.0,
            Power::SeedForest => 6.0,
            Power::CalmVolcano | Power::StirVolcano => 15.0,
            Power::Bounty => 10.0,
            Power::Protect => 12.0,
            Power::Inspire => 10.0,
            Power::Dream => 5.0,
            Power::Cure => 18.0,
            Power::Meteor => 80.0,
            Power::GreatQuake => 45.0,
        }
    }
}

impl Sim {
    /// Costs scale by age: the world has "set" once peoples rise.
    pub fn power_cost(&self, p: Power) -> f32 {
        let max_era = self.alive_civs().map(|c| c.era).max().unwrap_or(0) as f32;
        let civ_age = !self.civs.is_empty();
        let mult = match p.category() {
            "Geological" => if civ_age { 1.5 + max_era * 0.5 } else { 0.5 },
            "Climatic" => 1.0 + max_era * 0.08,
            "Disaster" => 1.0 + max_era * 0.2,
            _ => 1.0,
        };
        (p.base_cost() * mult).round()
    }

    pub fn power_allowed(&self, p: Power) -> bool {
        if matches!(self.mode, scenario::Mode::Chronicle) {
            return false;
        }
        if p.category() == "Disaster" {
            return matches!(self.mode, scenario::Mode::Sandbox | scenario::Mode::Challenge(_));
        }
        true
    }

    fn attune(&mut self, x: i32, y: i32, amount: f32) -> bool {
        let mut noticed = false;
        for i in self.planet.tiles_in_radius(x, y, 4) {
            self.planet.tiles[i].attune += amount;
            if self.planet.tiles[i].attune > 4.0 {
                noticed = true;
            }
        }
        noticed
    }

    pub fn apply_power(&mut self, p: Power, tile: usize) -> Result<String, String> {
        if !self.power_allowed(p) {
            return Err("That power is disabled in this mode.".into());
        }
        let cost = self.power_cost(p);
        if self.gp < cost {
            return Err(format!("Need {:.0} Gaia Points (have {:.0}).", cost, self.gp));
        }
        let (x, y) = self.planet.xy(tile);
        let city_here = self.planet.tiles_in_radius(x, y, 1).into_iter().find_map(|i| self.planet.tiles[i].city_slot).filter(|&c| self.cities[c as usize].alive);
        if p.needs_city() && city_here.is_none() {
            return Err("Select a city for that power.".into());
        }
        let msg: String;
        match p {
            Power::RaiseLand | Power::LowerLand => {
                let d = if p == Power::RaiseLand { 0.12 } else { -0.12 };
                for i in self.planet.tiles_in_radius(x, y, 2) {
                    let (tx, ty) = self.planet.xy(i);
                    let f = 1.0 - self.planet.dist(x, y, tx, ty) / 3.0;
                    self.planet.tiles[i].elev += d * f.max(0.2);
                    self.planet.tiles[i].uplift_stress += 0.5;
                    if self.planet.tiles[i].elev <= self.planet.sea_level {
                        if let Some(c) = self.planet.tiles[i].city_slot {
                            if self.cities[c as usize].alive {
                                self.planet.tiles[i].elev = self.planet.sea_level + 0.01;
                            }
                        }
                    }
                }
                for i in self.planet.tiles_in_radius(x, y, 4) {
                    if self.planet.tiles[i].uplift_stress > 0.3 && !self.planet.tiles[i].fault && self.rng.gen_bool(0.3) {
                        self.planet.tiles[i].fault = true;
                    }
                }
                self.planet.compute_rivers();
                self.planet.compute_continents();
                self.planet.update_climate();
                self.planet.update_biomes(false);
                msg = format!("The land {} near {}.", if d > 0.0 { "rises" } else { "sinks" }, self.region_name(x, y));
            }
            Power::Warm | Power::Cool => {
                let d = if p == Power::Warm { 4.0 } else { -4.0 };
                for i in self.planet.tiles_in_radius(x, y, 5) {
                    self.planet.tiles[i].temp_nudge += d;
                }
                self.planet.update_climate();
                msg = format!("The air {} over {}.", if d > 0.0 { "warms" } else { "cools" }, self.region_name(x, y));
            }
            Power::Wetten | Power::Dry => {
                let d = if p == Power::Wetten { 0.35 } else { -0.35 };
                for i in self.planet.tiles_in_radius(x, y, 5) {
                    self.planet.tiles[i].rain_nudge += d;
                }
                self.planet.update_climate();
                if p == Power::Wetten {
                    for c in self.cities.iter_mut().filter(|c| c.alive && self.planet.dist2(x, y, c.x, c.y) < 36.0) {
                        if c.chains.drought > 0 {
                            c.chains.drought = 0;
                        }
                    }
                }
                msg = format!("Rains {} over {}.", if d > 0.0 { "gather" } else { "fail" }, self.region_name(x, y));
            }
            Power::SeedForest => {
                for i in self.planet.tiles_in_radius(x, y, 3) {
                    if self.planet.is_land(i) && self.planet.tiles[i].city_slot.is_none() {
                        let t = &mut self.planet.tiles[i];
                        t.forest = (t.forest + 0.6).min(1.0);
                        t.biodiv = (t.biodiv + 0.2).min(1.0);
                        if t.improvement == Improvement::Farm {
                            t.improvement = Improvement::None;
                        }
                    }
                }
                msg = format!("Forest takes root in {}.", self.region_name(x, y));
            }
            Power::CalmVolcano | Power::StirVolcano => {
                let near = self.planet.tiles_in_radius(x, y, 3).into_iter().filter(|&i| self.planet.tiles[i].volcano > 0.0).max_by(|a, b| self.planet.tiles[*a].volcano.partial_cmp(&self.planet.tiles[*b].volcano).unwrap());
                let i = match near {
                    Some(i) => i,
                    None if p == Power::StirVolcano => {
                        self.planet.tiles[tile].volcano = 0.2;
                        tile
                    }
                    None => return Err("No volcano nearby.".into()),
                };
                if p == Power::CalmVolcano {
                    self.planet.tiles[i].volcano *= 0.3;
                    msg = "The mountain falls quiet.".into();
                } else {
                    self.planet.tiles[i].volcano = (self.planet.tiles[i].volcano + 0.6).min(2.0);
                    self.planet.tiles[i].uplift_stress += 0.5;
                    msg = "Deep fires stir beneath the mountain.".into();
                }
            }
            Power::Bounty => {
                for i in self.planet.tiles_in_radius(x, y, 3) {
                    let t = &mut self.planet.tiles[i];
                    t.soil = 1.0;
                    t.game = 1.0;
                    t.fish = 1.0;
                    t.blessed = 40.0;
                }
                self.planet.update_biomes(false);
                msg = format!("{} grows bountiful.", self.region_name(x, y));
            }
            Power::Protect => {
                let c = city_here.unwrap() as usize;
                self.cities[c].protected_until = self.year + 15;
                msg = format!("An unseen hand shelters {}.", self.cities[c].name);
            }
            Power::Inspire => {
                let c = city_here.unwrap() as usize;
                let civ = self.cities[c].civ as usize;
                let k = self.civs[civ].research.or_else(|| (0..TECHS.len()).find(|&k| !self.civs[civ].known_tech[k]));
                if let Some(k) = k {
                    self.civs[civ].eureka[k] = true;
                    msg = format!("Scholars in {} glimpse the secret of {}.", self.cities[c].name, TECHS[k].name);
                } else {
                    return Err("Nothing left to discover.".into());
                }
            }
            Power::Dream => {
                let c = city_here.unwrap() as usize;
                let civ = self.cities[c].civ as usize;
                // nudge toward peace/stewardship if aggressive, curiosity otherwise
                let tr = self.civs[civ].leader.traits;
                let k = if tr[T_AGGRESSION] > 0.6 { T_ECOLOGY } else if tr[T_CURIOSITY] < 0.5 { T_CURIOSITY } else { T_GREED };
                self.civs[civ].leader.dream = Some((k, 0.8));
                self.civs[civ].leader.traits[k] = (tr[k] + 0.05).min(1.0);
                msg = format!("{} {} wakes from a vivid dream.", self.civs[civ].leader.title, self.civs[civ].leader.name);
            }
            Power::Cure => {
                let c = city_here.unwrap();
                let mut cured = vec![c];
                for r in &self.routes {
                    if r.alive && (r.a == c || r.b == c) {
                        cured.push(if r.a == c { r.b } else { r.a });
                    }
                }
                for ci in cured {
                    let city = &mut self.cities[ci as usize];
                    city.plague_severity = 0.0;
                    city.chains.plague = 0;
                }
                msg = format!("The sickness lifts from {}.", self.cities[c as usize].name);
            }
            Power::Meteor => {
                self.gp_spent_destructive += cost;
                self.meteor(Some(tile), true);
                msg = "A star falls.".into();
            }
            Power::GreatQuake => {
                self.gp_spent_destructive += cost;
                for i in self.planet.tiles_in_radius(x, y, 3) {
                    self.planet.tiles[i].fault = true;
                    self.planet.tiles[i].uplift_stress += 1.0;
                }
                let hit: Vec<u32> = self.cities.iter().filter(|c| c.alive && self.planet.dist2(x, y, c.x, c.y) <= 10.0).map(|c| c.id).collect();
                for &c in &hit {
                    let city = &mut self.cities[c as usize];
                    if city.protected_until <= self.year {
                        city.pop *= 0.7;
                    }
                    city.backlog += 40.0;
                }
                let s = StorySeed::new("great_quake", Scope::Global)
                    .slot("Region", self.region_name(x, y))
                    .slot("Cities", hit.iter().map(|&c| self.city_name(c)).collect::<Vec<_>>().join(", "))
                    .at(x, y)
                    .cause("the ground itself was shaken by a power beyond the world")
                    .cat("ecological")
                    .tone(Tone::Tragedy)
                    .chronicle();
                self.seeds.push(s);
                msg = "The earth heaves.".into();
            }
        }
        self.gp -= cost;
        self.gp_spent += cost;
        let witnessed = self.cities.iter().any(|c| c.alive && self.planet.dist2(x, y, c.x, c.y) < 64.0);
        let amount = match p.category() {
            "Subtle" => 0.4,
            "Climatic" | "Biological" => 0.8,
            _ => 1.6,
        };
        let noticed = self.attune(x, y, amount);
        self.intervention_log.push((self.year, format!("{} at ({}, {})", p.name(), x, y)));
        // attribution and belief
        if witnessed && (noticed || p.category() == "Geological") && p != Power::Meteor && p != Power::GreatQuake {
            let city = self.cities.iter().filter(|c| c.alive).min_by_key(|c| self.planet.dist2(x, y, c.x, c.y) as i32).map(|c| c.id);
            if let Some(c) = city {
                let civ = self.cities[c as usize].civ;
                let piety = self.civs[civ as usize].leader.traits[T_PIETY];
                let reading = if noticed {
                    "the work of the Shaper, the hand behind the world"
                } else if piety > 0.6 {
                    "a sign from the gods"
                } else {
                    "a strange but natural turn"
                };
                let s = StorySeed::new("omen_witnessed", Scope::Regional)
                    .slot("City", self.city_name(c))
                    .slot("Civ", self.civ_name(civ))
                    .slot("Power", p.name().to_lowercase())
                    .slot("Reading", reading)
                    .city(c)
                    .civ(civ)
                    .at(x, y)
                    .cause(format!("{} witnessed nearby", p.name().to_lowercase()))
                    .cat("religious");
                let s = if noticed { s.cause("repeated interventions in this region (attunement)") } else { s };
                self.seeds.push(s);
                if noticed {
                    self.cities[c as usize].faith_acc += 20.0;
                }
            }
        }
        self.render_news();
        Ok(msg)
    }

    pub fn mitigate_window(&mut self, id: u32) -> Result<String, String> {
        let Some(k) = self.windows.iter().position(|w| w.id == id && !w.resolved) else { return Err("Window closed.".into()) };
        let w = self.windows[k].clone();
        if self.gp < w.cost {
            return Err(format!("Need {:.0} GP.", w.cost));
        }
        self.gp -= w.cost;
        self.gp_spent += w.cost;
        self.windows[k].resolved = true;
        let c = &mut self.cities[w.city as usize];
        match w.kind {
            "drought" | "famine" => {
                c.chains.drought = 0;
                c.food_store += c.pop * 2.0;
                let (x, y) = (c.x, c.y);
                for i in self.planet.tiles_in_radius(x, y, 4) {
                    self.planet.tiles[i].rain_nudge += 0.25;
                }
            }
            "plague" => {
                c.plague_severity = 0.0;
                c.chains.plague = 0;
            }
            "flood" | "quake" => {
                c.backlog = (c.backlog - 20.0).max(0.0);
                c.pop += 0.5;
            }
            _ => {}
        }
        let (x, y) = (self.cities[w.city as usize].x, self.cities[w.city as usize].y);
        self.attune(x, y, 0.5);
        self.intervention_log.push((self.year, format!("Mitigated: {}", w.title)));
        Ok(format!("Mitigated: {}", w.title))
    }
}
