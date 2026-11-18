//! NPC intelligence: tribes, civilization strategy, city governors, research, religion,
//! borders, diplomacy, trade, war, armies, espionage, succession and collapse.

use super::city::*;
use super::civ::*;
use super::news::*;
use super::planet::*;
use super::*;
use rand::Rng;

impl Sim {
    // ------------------------------------------------------------------ tribes & emergence
    pub fn update_tribes(&mut self) {
        let majors = self.civs.iter().filter(|c| !c.minor && c.alive).count();
        let minors = self.civs.iter().filter(|c| c.minor && c.alive).count();
        let n = self.tribes.len();
        for k in 0..n {
            if !self.tribes[k].alive {
                continue;
            }
            let (x, y) = (self.tribes[k].x, self.tribes[k].y);
            let i = self.planet.idx(x, y).unwrap();
            if !self.planet.is_land(i) {
                self.tribes[k].alive = false;
                continue;
            }
            // absorbed by civilizations
            if let Some(owner) = self.planet.tiles[i].owner {
                if let Some(&ci) = self.civs[owner as usize].cities.iter().min_by_key(|&&c| self.planet.dist2(x, y, self.cities[c as usize].x, self.cities[c as usize].y) as i32) {
                    let pop = self.tribes[k].pop;
                    self.cities[ci as usize].pop += pop * 0.05;
                    let s = StorySeed::new("tribe_joins", Scope::Local)
                        .slot("Tribe", self.tribes[k].name.clone())
                        .slot("City", self.cities[ci as usize].name.clone())
                        .slot("Civ", self.civs[owner as usize].name.clone())
                        .city(ci)
                        .cause("expanding borders reached their camps")
                        .cat("cultural");
                    self.seeds.push(s);
                }
                self.tribes[k].alive = false;
                continue;
            }
            let hab = self.habitability(i);
            let mut avg = hab;
            let mut cnt = 1.0;
            for j in self.planet.tiles_in_radius(x, y, 2) {
                avg += self.habitability(j);
                cnt += 1.0;
            }
            avg /= cnt;
            let t = &mut self.tribes[k];
            t.pop += t.pop * 0.035 * (avg - 0.25) * 3.0;
            t.pop = t.pop.clamp(0.0, 120.0);
            if t.pop < 3.0 {
                t.alive = false;
                continue;
            }
            // wander toward better ground
            if self.rng.gen_bool(0.15) {
                let dx = self.rng.gen_range(-2..=2);
                let dy = self.rng.gen_range(-2..=2);
                if let Some(j) = self.planet.idx(x + dx, y + dy) {
                    if self.habitability(j) > hab && self.planet.tiles[j].owner.is_none() {
                        self.tribes[k].x = (x + dx).rem_euclid(self.planet.w as i32);
                        self.tribes[k].y = y + dy;
                    }
                }
            }
            // crystallization: food surplus + good site + density + contact
            let pop = self.tribes[k].pop;
            let contact = self.tribes.iter().enumerate().any(|(o, t)| o != k && t.alive && self.planet.dist2(x, y, t.x, t.y) < 225.0)
                || self.cities.iter().any(|c| c.alive && self.planet.dist2(x, y, c.x, c.y) < 300.0);
            let near_city = self.cities.iter().any(|c| c.alive && self.planet.dist2(x, y, c.x, c.y) < 25.0);
            if near_city {
                continue;
            }
            let minor_next = majors >= self.major_target;
            let ready = (pop > 30.0 || (minor_next && pop > 20.0)) && avg > 0.3 && (contact || pop > 55.0);
            if ready {
                // choose best site within 3 tiles
                let mut best = (i, -1.0f32);
                for j in self.planet.tiles_in_radius(x, y, 3) {
                    if !self.planet.is_land(j) || self.planet.tiles[j].owner.is_some() {
                        continue;
                    }
                    let mut s = self.habitability(j);
                    if self.planet.tiles[j].is_river() {
                        s += 0.4;
                    }
                    if self.planet.is_coastal(j) {
                        s += 0.25;
                    }
                    if self.planet.tiles[j].fault {
                        s -= 0.1;
                    }
                    if s > best.1 {
                        best = (j, s);
                    }
                }
                let minor = majors >= self.major_target && majors < 16;
                if majors >= 16 && minors >= 8 {
                    continue;
                }
                if minor && minors >= 6 + self.major_target / 3 {
                    continue;
                }
                let (bx, by) = self.planet.xy(best.0);
                if let Some(cid) = self.found_civ(bx, by, minor, None, None) {
                    let city = self.found_city(cid, bx, by);
                    self.cities[city as usize].pop = 2.0 + pop * 0.06;
                    self.tribes[k].alive = false;
                    self.announce_founding(cid, city, best.0, pop, contact);
                    // initial contact
                    for o in 0..self.civs.len() as u16 {
                        if o != cid && self.civs[o as usize].alive {
                            let oc = self.civs[o as usize].capital.map(|c| (self.cities[c as usize].x, self.cities[c as usize].y));
                            if let Some((ox, oy)) = oc {
                                if self.planet.dist2(bx, by, ox, oy) < 400.0 {
                                    self.meet(cid, o);
                                }
                            }
                        }
                    }
                }
                return; // at most one founding per year keeps the news readable
            }
        }
        // new tribes appear in empty habitable land occasionally (after collapses, other continents)
        if self.year % 7 == 0 && self.tribes.iter().filter(|t| t.alive).count() < 6 && self.civs.len() < MAX_CIVS - 2 {
            for _ in 0..30 {
                let i = self.rng.gen_range(0..self.planet.tiles.len());
                if self.habitability(i) > 0.45 && self.planet.tiles[i].owner.is_none() {
                    let (x, y) = self.planet.xy(i);
                    if self.cities.iter().any(|c| c.alive && self.planet.dist2(x, y, c.x, c.y) < 64.0) {
                        continue;
                    }
                    let name = self.world_lang.word(&mut self.rng, 1, 2);
                    self.tribes.push(Tribe { x, y, pop: 12.0, name, alive: true });
                    break;
                }
            }
        }
    }

    fn announce_founding(&mut self, cid: u16, city: u32, tile: usize, pop: f32, contact: bool) {
        let civ = &self.civs[cid as usize];
        let c = &self.cities[city as usize];
        let neighbors: Vec<String> = self
            .civs
            .iter()
            .filter(|o| o.id != cid && o.alive)
            .filter(|o| o.capital.map(|oc| self.planet.dist2(c.x, c.y, self.cities[oc as usize].x, self.cities[oc as usize].y) < 900.0).unwrap_or(false))
            .map(|o| o.name.clone())
            .take(3)
            .collect();
        let site = self.site_desc(tile);
        let mut s = StorySeed::new(if civ.minor { "found_citystate" } else { "found_civ" }, Scope::Global)
            .slot("Leader", format!("{} {}", civ.leader.title, civ.leader.name))
            .slot("City", c.name.clone())
            .slot("Civ", civ.name.clone())
            .slot("Demonym", civ.demonym.clone())
            .slot("Site", site.clone())
            .slot("Pop", format!("{}", c.display_pop(civ.era)))
            .slot("Trait", civ.leader.trait_word())
            .slot("Ability", civ.ability)
            .slot("Kind", civ.minor_kind)
            .slot("Neighbors", if neighbors.is_empty() { "none known".to_string() } else { neighbors.join(", ") })
            .city(city)
            .civ(cid)
            .at(c.x, c.y)
            .cause(format!("sustained harvests at {}", site))
            .cause(format!("a population of {:.0} bands", pop))
            .cat("political");
        if contact {
            s = s.cause("contact with neighbors rewarded coordination");
        }
        s = if civ.minor { s.card() } else { s.chronicle() };
        self.seeds.push(s);
    }

    pub fn meet(&mut self, a: u16, b: u16) {
        if a == b || self.r(a, b).met {
            return;
        }
        self.rm(a, b).met = true;
        self.rm(b, a).met = true;
        let ca = self.civs[a as usize].capital.map(|c| self.planet.tiles[self.planet.idx(self.cities[c as usize].x, self.cities[c as usize].y).unwrap()].continent);
        let cb = self.civs[b as usize].capital.map(|c| self.planet.tiles[self.planet.idx(self.cities[c as usize].x, self.cities[c as usize].y).unwrap()].continent);
        let v = self.rng.gen_range(-8.0..10.0);
        self.ledger(a, b, Reason::FirstContact, v, Some(30), "first impressions");
        let v2 = self.rng.gen_range(-8.0..10.0);
        self.ledger(b, a, Reason::FirstContact, v2, Some(30), "first impressions");
        if self.civs[a as usize].minor || self.civs[b as usize].minor {
            return;
        }
        // eureka for writing
        for c in [a, b] {
            let k = tech_index("Writing");
            self.civs[c as usize].eureka[k] = true;
        }
        let cross = ca.is_some() && cb.is_some() && ca != cb;
        let mut s = StorySeed::new(if cross { "first_contact_continents" } else { "first_contact" }, if cross { Scope::Global } else { Scope::National })
            .slot("CivA", self.civ_name(a))
            .slot("CivB", self.civ_name(b))
            .civ(a)
            .civ2(b)
            .cat("political");
        if let Some(c) = self.civs[b as usize].capital {
            s = s.at(self.cities[c as usize].x, self.cities[c as usize].y);
        }
        s = if cross {
            self.contact_made = true;
            s.cause("ships crossed the open sea").chronicle().alert(AlertClass::Major)
        } else {
            s.cause("borders and scouts met").card()
        };
        self.seeds.push(s);
    }

    // ------------------------------------------------------------------ civ yearly
    pub fn update_civs(&mut self) {
        let year = self.year;
        for c in 0..self.civs.len() {
            if !self.civs[c].alive {
                continue;
            }
            let cid = c as u16;
            if self.civs[c].cities.is_empty() {
                self.civ_falls(cid, "the loss of its last city", None);
                continue;
            }
            self.research(cid);
            self.leader_life(cid);
            if year % 10 == c as i32 % 10 {
                self.choose_strategy(cid);
                self.consider_government(cid);
            }
            self.city_builds(cid);
            self.found_religion(cid);
            let at_war = (0..self.civs.len()).any(|o| self.rel[c * MAX_CIVS + o].at_war);
            if !at_war {
                let ww = &mut self.civs[c].war_weariness;
                *ww = (*ww * 0.9 - 0.5).max(0.0);
            }
            let mil: f32 = self.armies.iter().filter(|a| a.alive && a.owner == Some(cid)).map(|a| a.strength).sum();
            self.civs[c].military = mil;
        }
        if year % 5 == 0 {
            self.update_borders();
            self.spread_religion();
            self.loyalty_pressure();
        }
        self.check_civil_wars();
    }

    fn research(&mut self, cid: u16) {
        let c = cid as usize;
        let sci: f32 = self.civs[c].cities.iter().map(|&ci| self.cities[ci as usize].yields.science).sum();
        let cul: f32 = self.civs[c].cities.iter().map(|&ci| self.cities[ci as usize].yields.culture).sum();
        let curiosity = self.civs[c].leader.traits[T_CURIOSITY];
        let coastal = self.civs[c].cities.iter().any(|&ci| self.cities[ci as usize].coastal);
        // pick research
        if self.civs[c].research.is_none() {
            let civ = &self.civs[c];
            let avail: Vec<usize> = (0..TECHS.len())
                .filter(|&k| !civ.known_tech[k])
                .filter(|&k| TECHS[k].prereq.iter().all(|p| civ.known_tech[tech_index(p)]))
                .filter(|&k| !TECHS[k].needs_coast || coastal)
                .collect();
            if let Some(&k) = avail.iter().min_by_key(|&&k| (TECHS[k].era * 10) as i32 - if civ.eureka[k] { 5 } else { 0 } + self.rng.gen_range(0..4)) {
                self.civs[c].research = Some(k);
            }
        }
        if let Some(k) = self.civs[c].research {
            let era = TECHS[k].era as f32;
            let mut cost = 55.0 * (1.0 + era * 1.5).powf(2.35);
            if self.civs[c].eureka[k] {
                cost *= 0.5;
            }
            // diffusion from contacts
            let knowers = (0..self.civs.len())
                .filter(|&o| o != c && self.civs[o].alive && self.civs[o].known_tech[k] && self.rel[c * MAX_CIVS + o].met)
                .count() as f32;
            cost *= 1.0 / (1.0 + knowers * 0.18);
            self.civs[c].research_progress += sci * (0.85 + curiosity * 0.3);
            if self.civs[c].research_progress >= cost {
                self.civs[c].research_progress -= cost;
                self.civs[c].known_tech[k] = true;
                self.civs[c].research = None;
                self.on_tech(cid, k, knowers > 0.0);
            }
        }
        // civics
        if self.civs[c].civic_research.is_none() {
            let civ = &self.civs[c];
            let avail: Vec<usize> = (0..CIVICS.len())
                .filter(|&k| !civ.known_civic[k])
                .filter(|&k| CIVICS[k].prereq.iter().all(|p| civ.known_civic[civic_index(p)]))
                .filter(|&k| CIVICS[k].era <= civ.era + 1)
                .collect();
            if !avail.is_empty() {
                let k = avail[self.rng.gen_range(0..avail.len())];
                self.civs[c].civic_research = Some(k);
            }
        }
        if let Some(k) = self.civs[c].civic_research {
            let cost = 25.0 * (1.0 + CIVICS[k].era as f32 * 1.2).powf(1.7);
            self.civs[c].civic_progress += cul;
            if self.civs[c].civic_progress >= cost {
                self.civs[c].civic_progress -= cost;
                self.civs[c].known_civic[k] = true;
                self.civs[c].civic_research = None;
            }
        }
    }

    fn on_tech(&mut self, cid: u16, k: usize, diffused: bool) {
        let c = cid as usize;
        let t = &TECHS[k];
        // era
        let mut era = 0;
        for e in 0..9 {
            let known = TECHS.iter().filter(|x| x.era == e).filter(|x| self.civs[c].known_tech[tech_index(x.name)]).count();
            if known >= 2 {
                era = e;
            }
        }
        let first_on_planet = !self.civs.iter().enumerate().any(|(o, civ)| o != c && civ.known_tech[k]);
        if era > self.civs[c].era {
            self.civs[c].era = era;
            let first = !self.civs.iter().enumerate().any(|(o, civ)| o != c && civ.era >= era);
            let s = StorySeed::new("era", if first { Scope::Global } else { Scope::National })
                .slot("Civ", self.civ_name(cid))
                .slot("Era", ERA_NAMES[era])
                .slot("Skin", era_skin(era))
                .slot("Tech", t.name)
                .civ(cid)
                .cause(format!("mastery of {}", t.name))
                .cat("technological")
                .tone(Tone::Triumph);
            self.seeds.push(if first { s.chronicle() } else { s.card() });
        }
        // resources revealed
        if ALL_RES.iter().any(|r| r.revealed_by() == Some(k)) {
            let revealed: Vec<(u32, ResKind)> = self.civs[c]
                .cities
                .iter()
                .filter_map(|&ci| {
                    let city = &self.cities[ci as usize];
                    city.catchment.iter().find_map(|&ti| {
                        self.planet.tiles[ti].resource.filter(|r| r.kind.revealed_by() == Some(k)).map(|r| (ci, r.kind))
                    })
                })
                .collect();
            if let Some(&(ci, kind)) = revealed.first() {
                let city = &self.cities[ci as usize];
                let s = StorySeed::new("resource_revealed", Scope::National)
                    .slot("City", city.name.clone())
                    .slot("Civ", self.civ_name(cid))
                    .slot("Resource", kind.name())
                    .slot("Tech", t.name)
                    .city(ci)
                    .civ(cid)
                    .at(city.x, city.y)
                    .cause(format!("{} revealed deposits that had always been there", t.name))
                    .cat("technological")
                    .card();
                self.seeds.push(s);
            }
        }
        if first_on_planet && t.era >= 1 {
            let s = StorySeed::new("tech_first", Scope::Global)
                .slot("Civ", self.civ_name(cid))
                .slot("Tech", t.name)
                .civ(cid)
                .cause(if self.civs[c].eureka[k] { format!("a eureka from {}", t.eureka) } else { "years of patient research".to_string() })
                .cat("technological");
            self.seeds.push(if t.era >= 4 { s.card() } else { s });
        } else if diffused && self.rng.gen_bool(0.2) {
            let s = StorySeed::new("tech_diffused", Scope::National)
                .slot("Civ", self.civ_name(cid))
                .slot("Tech", t.name)
                .civ(cid)
                .cause("knowledge carried by neighbors and traders")
                .cat("technological");
            self.seeds.push(s);
        }
        if t.name == "Starship" {
            self.civs[c].defining_moments.push("began the departure project".into());
        }
    }

    fn leader_life(&mut self, cid: u16) {
        let c = cid as usize;
        self.civs[c].leader.age += 1;
        let l = &self.civs[c].leader;
        if l.age < l.lifespan && !self.rng.gen_bool(0.004) {
            return;
        }
        // succession
        let old = self.civs[c].leader.clone();
        let succ = self.civs[c].gov().succession;
        let era = self.civs[c].era;
        let mut base = old.traits;
        // biography: famine and betrayal shape the next generation
        if self.civs[c].betrayed_memory > 0.0 {
            base[T_VINDICTIVE] = (base[T_VINDICTIVE] + 0.15).min(1.0);
            base[T_LOYALTY] = (base[T_LOYALTY] + 0.05).min(1.0);
        }
        if self.civs[c].famine_memory > 0.0 {
            base[T_ECOLOGY] = (base[T_ECOLOGY] + 0.05).min(1.0);
            base[T_EXPANSION] = (base[T_EXPANSION] - 0.05).max(0.0);
        }
        let drift = match succ {
            Succession::Hereditary => 0.6,
            Succession::Elective => 0.4,
            Succession::Theocratic => 0.7,
            Succession::Military => 0.3,
            Succession::Corporate => 0.5,
            Succession::Machine => 0.9,
        };
        let mut lang = std::mem::replace(&mut self.civs[c].lang, Language::generate(&mut self.rng));
        let mut new = gen_leader(&mut lang, &mut self.rng, Some(&base), self.year, era);
        self.civs[c].lang = lang;
        for k in 0..8 {
            new.traits[k] = (base[k] * drift + new.traits[k] * (1.0 - drift)).clamp(0.02, 0.98);
        }
        if succ == Succession::Theocratic {
            new.traits[T_PIETY] = new.traits[T_PIETY].max(0.7);
        }
        if succ == Succession::Military {
            new.traits[T_AGGRESSION] = (new.traits[T_AGGRESSION] + 0.15).min(1.0);
        }
        if succ == Succession::Hereditary {
            let n = self.civs[c].leaders_past.iter().filter(|l| l.0.starts_with(&old.name)).count() as u32;
            if self.rng.gen_bool(0.3) {
                new.name = format!("{} {}", old.name.split(' ').next().unwrap_or(&old.name), names::roman(n + 2));
            }
        }
        let act = old.defining_act.clone().unwrap_or_default();
        self.civs[c].leaders_past.push((format!("{} {}", old.title, old.name), old.reign_start, self.year, act));
        self.civs[c].leader = new;
        self.civs[c].betrayed_memory *= 0.5;
        self.civs[c].famine_memory *= 0.5;
        // contested succession can trigger a crisis chain
        let contested = matches!(succ, Succession::Hereditary | Succession::Military) && self.rng.gen_bool(0.18) && self.civs[c].cities.len() >= 3;
        let reign = self.year - old.reign_start;
        let mut s = StorySeed::new(if contested { "succession_crisis" } else { "succession" }, Scope::National)
            .slot("Civ", self.civ_name(cid))
            .slot("Old", format!("{} {}", old.title, old.name))
            .slot("New", format!("{} {}", self.civs[c].leader.title, self.civs[c].leader.name))
            .slot("Reign", reign.to_string())
            .slot("Trait", self.civs[c].leader.trait_word())
            .slot("Succession", format!("{:?}", succ).to_lowercase())
            .civ(cid)
            .cause(if old.age >= old.lifespan { format!("{} died at {}", old.name, old.age) } else { format!("{} died suddenly", old.name) })
            .cat("political")
            .alert(AlertClass::Major);
        if let Some(cap) = self.civs[c].capital {
            s = s.at(self.cities[cap as usize].x, self.cities[cap as usize].y).city(cap);
        }
        if contested {
            s = s.card().thread(format!("Succession of the {}", self.civs[c].name));
            if let Some(cap) = self.civs[c].capital {
                self.cities[cap as usize].chains.succession = 1;
                self.cities[cap as usize].chains.stage_year = self.year;
            }
            for &ci in &self.civs[c].cities.clone() {
                self.cities[ci as usize].unrest += 12.0;
            }
        } else if !self.civs[c].minor {
            s = s.card();
        }
        self.seeds.push(s);
        // neighbors re-evaluate
        for o in 0..self.civs.len() as u16 {
            if o != cid && self.r(o, cid).met {
                self.rm(o, cid).ledger.retain(|e| e.reason != Reason::FirstContact);
            }
        }
    }

    fn choose_strategy(&mut self, cid: u16) {
        let c = cid as usize;
        let civ = &self.civs[c];
        let t = civ.leader.traits;
        let dream = civ.leader.dream;
        let n_cities = civ.cities.len() as f32;
        let at_war = (0..self.civs.len()).any(|o| self.rel[c * MAX_CIVS + o].at_war);
        let hostile_neighbors = (0..self.civs.len()).filter(|&o| self.rel[c * MAX_CIVS + o].met && self.rel[c * MAX_CIVS + o].score < -25.0).count() as f32;
        let mut opts = vec![
            (Strategy::Expand, t[T_EXPANSION] * 1.4 + if n_cities < 4.0 { 0.6 } else { 0.0 } - n_cities * 0.05),
            (Strategy::Consolidate, (1.0 - t[T_EXPANSION]) * 0.9 + if civ.treasury < 0.0 { 0.6 } else { 0.0 }),
            (Strategy::Trade, t[T_GREED] * 1.2),
            (Strategy::Convert, t[T_PIETY] * 1.1 * if civ.religion.is_some() { 1.0 } else { 0.4 }),
            (Strategy::Arm, t[T_AGGRESSION] * 1.0 + hostile_neighbors * 0.15 + if at_war { 0.8 } else { 0.0 }),
            (Strategy::Research, t[T_CURIOSITY] * 1.2),
        ];
        if civ.minor {
            opts[0].1 = -10.0;
        }
        if let Some((k, v)) = dream {
            let s = match k {
                T_EXPANSION => 0,
                T_AGGRESSION => 4,
                T_GREED => 2,
                T_PIETY => 3,
                T_CURIOSITY => 5,
                _ => 1,
            };
            opts[s].1 += v;
            self.civs[c].leader.dream = None;
        }
        for o in opts.iter_mut() {
            o.1 += self.rng.gen_range(0.0..0.3);
        }
        let best = opts.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap().0;
        self.civs[c].strategy = best;
    }

    fn consider_government(&mut self, cid: u16) {
        let c = cid as usize;
        let civ = &self.civs[c];
        if self.year - civ.gov_change_year < 40 {
            return;
        }
        let t = civ.leader.traits;
        let mut best = (civ.government, -1.0f32);
        for (k, g) in GOVS.iter().enumerate() {
            if !g.civic.is_empty() && !civ.knows_civic(g.civic) {
                continue;
            }
            let mut s = k as f32 * 0.25 + g.stability * 0.03;
            s += match g.name {
                "Theocracy" => t[T_PIETY] * 1.2,
                "Merchant Republic" | "Corporate State" => t[T_GREED] * 1.1,
                "Despotism" | "Command Economy" => t[T_AGGRESSION] * 0.9,
                "Classical Republic" | "Democracy" => t[T_CURIOSITY] * 0.9 + (1.0 - t[T_AGGRESSION]) * 0.3,
                "Monarchy" => 0.5,
                "Synthetic Commonwealth" => 1.5,
                _ => 0.0,
            };
            if k == civ.government {
                s += 0.4;
            }
            if s > best.1 {
                best = (k, s);
            }
        }
        if best.0 != civ.government {
            let old = GOVS[civ.government].name;
            self.civs[c].government = best.0;
            self.civs[c].gov_change_year = self.year;
            let revolution = matches!(GOVS[best.0].name, "Democracy" | "Command Economy" | "Classical Republic");
            for &ci in &self.civs[c].cities.clone() {
                self.cities[ci as usize].unrest += if revolution { 15.0 } else { 6.0 };
            }
            let s = StorySeed::new("government", Scope::National)
                .slot("Civ", self.civ_name(cid))
                .slot("Old", old)
                .slot("New", GOVS[best.0].name)
                .slot("Leader", self.civs[c].leader.name.clone())
                .civ(cid)
                .cause(format!("new civics and a {} leader", self.civs[c].leader.trait_word()))
                .cat("political")
                .card();
            self.seeds.push(s);
        }
    }

    fn city_builds(&mut self, cid: u16) {
        let c = cid as usize;
        let cities = self.civs[c].cities.clone();
        let strategy = self.civs[c].strategy;
        let minor = self.civs[c].minor;
        for ci in cities {
            let cu = ci as usize;
            if self.cities[cu].build.is_none() {
                let b = self.governor_choose(cid, ci, strategy, minor);
                self.cities[cu].build = b;
            }
            let cost = match &self.cities[cu].build {
                Some(Build::Building(k)) => BUILDINGS[*k].cost * (1.0 + self.civs[c].era as f32 * 0.15),
                Some(Build::Wonder(k)) => WONDERS[*k].cost,
                Some(Build::Army) => 30.0 + self.civs[c].era as f32 * 12.0,
                Some(Build::Settler) => 45.0 + self.civs[c].cities.len() as f32 * 6.0,
                Some(Build::Project(_)) => 40.0,
                None => f32::INFINITY,
            };
            if self.cities[cu].build_progress < cost {
                continue;
            }
            self.cities[cu].build_progress -= cost;
            let b = self.cities[cu].build.take().unwrap();
            match b {
                Build::Building(k) => {
                    self.cities[cu].buildings[k] = true;
                    if BUILDINGS[k].value > 40.0 || self.rng.gen_bool(0.25) {
                        let city = &self.cities[cu];
                        let s = StorySeed::new("building", Scope::Local)
                            .slot("City", city.name.clone())
                            .slot("Building", BUILDINGS[k].name)
                            .slot("Governor", city.governor.name.clone())
                            .city(ci)
                            .civ(cid)
                            .cause(format!("Governor {}'s build queue", city.governor.name))
                            .cat("economic");
                        self.seeds.push(s);
                    }
                }
                Build::Wonder(k) => self.complete_wonder(cid, ci, k),
                Build::Army => {
                    let tier = self.civs[c].military_tier();
                    let disc = if self.civs[c].ability == "Iron Discipline" { 1.15 } else { 1.0 };
                    let id = self.next_army;
                    self.next_army += 1;
                    let city = &self.cities[cu];
                    self.armies.push(Army {
                        id,
                        owner: Some(cid),
                        x: city.x as f32,
                        y: city.y as f32,
                        strength: 5.0 * tier * disc,
                        target: None,
                        enemy: None,
                        alive: true,
                        loot: 0.0,
                        home: Some(ci),
                        sieging: false,
                    });
                    self.cities[cu].garrison += 1.0;
                }
                Build::Settler => {
                    self.cities[cu].pop = (self.cities[cu].pop - 1.0).max(1.0);
                    self.settle(cid, ci);
                }
                Build::Project(p) => {
                    self.complete_project(ci, p);
                }
            }
        }
    }

    fn governor_choose(&mut self, cid: u16, ci: u32, strategy: Strategy, minor: bool) -> Option<Build> {
        let c = cid as usize;
        let civ = &self.civs[c];
        let city = &self.cities[ci as usize];
        let comp = city.governor.competence;
        let my_armies = self.armies.iter().filter(|a| a.alive && a.owner == Some(cid)).count();
        let at_war = (0..self.civs.len()).any(|o| self.rel[c * MAX_CIVS + o].at_war);
        let n_cities = civ.cities.len();
        // project responses
        if city.chains.drought >= 2 && !city.has("Granary") {
            return Some(Build::Building(bidx("Granary")));
        }
        if at_war && my_armies < n_cities * 2 + 2 {
            return Some(Build::Army);
        }
        let want_army = match strategy {
            Strategy::Arm => n_cities * 2 + 1,
            _ => n_cities / 2 + 1,
        };
        if my_armies < want_army && self.rng.gen_bool(0.5) {
            return Some(Build::Army);
        }
        if strategy == Strategy::Expand && !minor && city.pop >= 3.0 && n_cities < 14 && self.rng.gen_bool(0.6) {
            return Some(Build::Settler);
        }
        if !minor && n_cities < 3 && city.pop >= 3.5 && self.rng.gen_bool(0.35) {
            return Some(Build::Settler);
        }
        // wonders: great builders and rich cities chase them
        if !minor && city.pop > 5.0 && self.rng.gen_bool((0.15 + civ.leader.traits[T_PIETY] * 0.1 + if civ.agenda_visible == 2 { 0.15 } else { 0.0 }) as f64) {
            let tile = self.planet.idx(city.x, city.y).unwrap();
            let biome = self.planet.tiles[tile].biome;
            let near_mtn = self.planet.tiles_in_radius(city.x, city.y, 2).iter().any(|&t| self.planet.tiles[t].biome == Biome::Mountain);
            let holy = city.religion.map(|r| self.religions[r].holy_city == ci).unwrap_or(false);
            for (k, w) in WONDERS.iter().enumerate() {
                if self.wonders_built[k].is_some() || !civ.knows(w.tech) {
                    continue;
                }
                let ok = match w.req {
                    "river" => city.river,
                    "coast" => city.coastal,
                    "desert" => matches!(biome, Biome::Desert | Biome::Shrubland) || self.planet.tiles_in_radius(city.x, city.y, 2).iter().any(|&t| self.planet.tiles[t].biome == Biome::Desert),
                    "mountain" => near_mtn,
                    "holy" => holy,
                    "rich" => city.wealth >= 650.0,
                    _ => true,
                };
                if ok {
                    if !self.wonder_race.iter().any(|r| r.0 == k && r.1 == ci) {
                        self.wonder_race.push((k, ci, 0.0));
                        let racers = self.wonder_race.iter().filter(|r| r.0 == k).count();
                        if racers == 2 {
                            let s = StorySeed::new("wonder_race", Scope::Global)
                                .slot("Wonder", w.name)
                                .slot("City", city.name.clone())
                                .slot("Civ", civ.name.clone())
                                .city(ci)
                                .civ(cid)
                                .cause("two cities now compete for the same wonder")
                                .cat("cultural")
                                .thread(format!("Race for the {}", w.name))
                                .card();
                            self.seeds.push(s);
                        }
                    }
                    return Some(Build::Wonder(k));
                }
            }
        }
        // buildings, weighted by strategy and need; low competence misjudges hazards
        let mut best: Option<(usize, f32)> = None;
        for (k, b) in BUILDINGS.iter().enumerate() {
            if city.buildings[k] || !civ.knows(b.tech) {
                continue;
            }
            if b.needs_coast && !city.coastal {
                continue;
            }
            if b.needs_river && !city.river {
                continue;
            }
            let mut s = 1.0 - b.cost / 500.0;
            s += match (strategy, b.name) {
                (Strategy::Trade, "Market" | "Harbor" | "Bank" | "Stock Exchange") => 1.0,
                (Strategy::Research, "Library" | "University" | "Research Lab") => 1.0,
                (Strategy::Convert, "Shrine" | "Temple") => 1.0,
                (Strategy::Arm, "Walls" | "Castle") => 0.8,
                (Strategy::Consolidate, "Granary" | "Aqueduct" | "Hospital") => 0.8,
                _ => 0.0,
            };
            if b.name == "Aqueduct" && city.pop > city.housing() - 2.0 {
                s += 1.0;
            }
            if matches!(b.name, "Flood Defenses" | "Seawall") {
                s += comp * 0.8 - 0.3;
            }
            if b.name == "Vault" && city.wealth > 600.0 {
                s += 0.6;
            }
            if b.name == "Nature Reserve" {
                s += civ.leader.traits[T_ECOLOGY];
            }
            s += self.rng.gen_range(0.0..0.4);
            if best.map(|bb| s > bb.1).unwrap_or(true) {
                best = Some((k, s));
            }
        }
        if let Some((k, _)) = best {
            return Some(Build::Building(k));
        }
        if !minor && city.pop >= 4.0 && n_cities < 16 {
            return Some(Build::Settler);
        }
        Some(Build::Project("Festival"))
    }

    fn complete_project(&mut self, ci: u32, p: &'static str) {
        let city = &mut self.cities[ci as usize];
        match p {
            "Festival" => {
                city.unrest = (city.unrest - 8.0).max(0.0);
                city.culture += 10.0;
            }
            "Relief Works" => {
                city.food_store += city.pop * 2.0;
                city.unrest = (city.unrest - 5.0).max(0.0);
            }
            _ => {}
        }
    }

    fn complete_wonder(&mut self, cid: u16, ci: u32, k: usize) {
        if self.wonders_built[k].is_some() {
            // lost the race: production goes to a festival
            self.cities[ci as usize].culture += 20.0;
            return;
        }
        self.wonders_built[k] = Some((ci, self.year));
        self.cities[ci as usize].wonders.push(k);
        self.civs[cid as usize].wonders.push(WONDERS[k].name.to_string());
        let losers: Vec<u32> = self.wonder_race.iter().filter(|r| r.0 == k && r.1 != ci).map(|r| r.1).collect();
        self.wonder_race.retain(|r| r.0 != k);
        for l in &losers {
            let lc = self.cities[*l as usize].civ;
            if let Some(Build::Wonder(kk)) = self.cities[*l as usize].build {
                if kk == k {
                    self.cities[*l as usize].build = None;
                }
            }
            if lc != cid && self.r(lc, cid).met {
                self.ledger(lc, cid, Reason::WealthEnvy, -8.0, Some(30), "won a wonder race");
            }
        }
        let city = &self.cities[ci as usize];
        let i = self.planet.idx(city.x, city.y).unwrap();
        let mut s = StorySeed::new("wonder", Scope::Global)
            .slot("Wonder", WONDERS[k].name)
            .slot("City", city.name.clone())
            .slot("Civ", self.civ_name(cid))
            .slot("Band", city.band())
            .slot("Leader", self.civs[cid as usize].leader.name.clone())
            .city(ci)
            .civ(cid)
            .at(city.x, city.y)
            .cause(format!("{} output and a {} treasury", city.spec.name(), city.band().to_lowercase()))
            .cat("cultural")
            .tone(Tone::Triumph)
            .thread(format!("Race for the {}", WONDERS[k].name))
            .alert(AlertClass::Major)
            .chronicle();
        if !losers.is_empty() {
            s = s.slot("Rival", self.cities[losers[0] as usize].name.clone());
        }
        self.seeds.push(s);
        if self.civs[cid as usize].leader.defining_act.is_none() {
            self.civs[cid as usize].leader.defining_act = Some(format!("built the {}", WONDERS[k].name));
        }
        let _ = i;
    }

    pub fn settle(&mut self, cid: u16, from: u32) {
        let c = cid as usize;
        let fc = &self.cities[from as usize];
        let (fx, fy) = (fc.x, fc.y);
        let cont = self.planet.tiles[self.planet.idx(fx, fy).unwrap()].continent;
        let sail = self.civs[c].knows("Sailing");
        let nav = self.civs[c].knows("Navigation");
        let range = if nav { 26 } else if sail { 14 } else { 9 };
        let t = self.civs[c].leader.traits;
        let mut best: Option<(usize, f32)> = None;
        for _ in 0..140 {
            let dx = self.rng.gen_range(-range..=range);
            let dy = self.rng.gen_range(-range..=range);
            let Some(i) = self.planet.idx(fx + dx, fy + dy) else { continue };
            if !self.planet.is_land(i) {
                continue;
            }
            let tile = &self.planet.tiles[i];
            if tile.owner.is_some() && tile.owner != Some(cid) {
                continue;
            }
            if tile.continent != cont && !(sail && self.planet.is_coastal(i)) {
                continue;
            }
            let (x, y) = self.planet.xy(i);
            if self.cities.iter().any(|cc| cc.alive && self.planet.dist2(x, y, cc.x, cc.y) < 10.0) {
                continue;
            }
            if self.tribes.iter().any(|tr| tr.alive && self.planet.dist2(x, y, tr.x, tr.y) < 4.0) {
                continue;
            }
            let mut s = self.habitability(i) * 2.0;
            if tile.is_river() {
                s += 0.6;
            }
            if self.planet.is_coastal(i) {
                s += 0.35 + t[T_GREED] * 0.2;
            }
            // resources around
            for j in self.planet.tiles_in_radius(x, y, 2) {
                if self.planet.tiles[j].resource.is_some() {
                    s += 0.15;
                }
                s += self.planet.tiles[j].fert * 0.05;
            }
            if tile.fault {
                s -= 0.25 * self.cities[from as usize].governor.competence;
            }
            if self.planet.height(i) < 0.03 && self.planet.is_coastal(i) {
                s -= 0.2 * self.cities[from as usize].governor.competence;
            }
            if tile.blessed > 0.0 {
                s += 0.8;
            }
            let d = self.planet.dist(fx, fy, x, y);
            s -= d * 0.04;
            if best.map(|b| s > b.1).unwrap_or(true) {
                best = Some((i, s));
            }
        }
        if let Some((i, s)) = best {
            if s < 0.4 {
                return;
            }
            let (x, y) = self.planet.xy(i);
            let nc = self.found_city(cid, x, y);
            let new_cont = self.planet.tiles[i].continent;
            let mut seed = StorySeed::new("new_city", Scope::Local)
                .slot("City", self.cities[nc as usize].name.clone())
                .slot("Civ", self.civ_name(cid))
                .slot("From", self.cities[from as usize].name.clone())
                .slot("Site", self.site_desc(i))
                .city(nc)
                .civ(cid)
                .at(x, y)
                .cause(format!("a {} strategy under {}", self.civs[c].strategy.name().to_lowercase(), self.civs[c].leader.name))
                .cat("political");
            if new_cont != cont {
                seed = seed.cause("settlers crossed the sea").mag(2.0);
                let k = tech_index("Navigation");
                self.civs[c].eureka[k] = true;
            }
            self.seeds.push(seed);
        }
    }

    fn found_religion(&mut self, cid: u16) {
        let c = cid as usize;
        if self.civs[c].religion.is_some() || self.civs[c].minor && self.civs[c].minor_kind != "religious" {
            return;
        }
        if !self.civs[c].knows_civic("Mysticism") {
            return;
        }
        let faith: f32 = self.civs[c].cities.iter().map(|&ci| self.cities[ci as usize].faith_acc).sum();
        let cost = 60.0 + self.religions.len() as f32 * 35.0;
        if faith < cost || self.religions.len() >= 10 {
            return;
        }
        let Some(holy) = self.civs[c].cities.iter().copied().max_by(|a, b| self.cities[*a as usize].faith_acc.partial_cmp(&self.cities[*b as usize].faith_acc).unwrap()) else { return };
        let city = &self.cities[holy as usize];
        let i = self.planet.idx(city.x, city.y).unwrap();
        let attuned = self.planet.tiles[i].attune > 3.0;
        let theme = if attuned {
            "Shaper"
        } else if city.river {
            "River"
        } else if city.coastal && self.planet.moons > 0 {
            "Moon"
        } else if self.civs[c].era >= 5 {
            "Machine"
        } else if self.civs[c].leader.traits[T_ECOLOGY] > 0.65 {
            "Ecology"
        } else {
            ["Sun", "Storm", "Ancestor"][self.rng.gen_range(0..3)]
        };
        let beliefs: Vec<&'static str> = RELIGION_THEMES.iter().find(|t| t.0 == theme).map(|t| t.1.to_vec()).unwrap_or_default();
        let name = format!("{} of {}", ["Way", "Faith", "Covenant", "Path", "Light"][self.rng.gen_range(0..5)], self.civs[c].lang.word(&mut self.rng, 1, 2));
        let rid = self.religions.len();
        self.religions.push(Religion {
            name: name.clone(),
            theme,
            founder: cid,
            holy_city: holy,
            founded: self.year,
            beliefs: beliefs.into_iter().take(3).collect(),
            parent: None,
            shaper_cult: attuned,
        });
        self.civs[c].religion = Some(rid);
        for &ci in &self.civs[c].cities.clone() {
            self.cities[ci as usize].faith_acc = 0.0;
        }
        self.cities[holy as usize].religion = Some(rid);
        let city = &self.cities[holy as usize];
        let mut s = StorySeed::new("religion_founded", Scope::Global)
            .slot("Religion", name)
            .slot("City", city.name.clone())
            .slot("Civ", self.civ_name(cid))
            .slot("Theme", theme.to_lowercase())
            .city(holy)
            .civ(cid)
            .at(city.x, city.y)
            .cause(format!("faith gathered in {}", city.name))
            .cat("religious")
            .chronicle();
        if attuned {
            s = s.cause("repeated signs from the sky over this land");
        }
        self.seeds.push(s);
    }

    pub fn spread_religion(&mut self) {
        let n = self.cities.len();
        let mut pressure: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
        for a in 0..n {
            let ca = &self.cities[a];
            if !ca.alive {
                continue;
            }
            let Some(r) = ca.religion else { continue };
            let mut strength = 1.0 + ca.yields.faith * 0.3;
            if self.religions[r].holy_city == ca.id {
                strength *= 2.0;
            }
            let piety = self.civs[ca.civ as usize].leader.traits[T_PIETY];
            if self.civs[ca.civ as usize].strategy == Strategy::Convert {
                strength *= 1.6;
            }
            strength *= 0.6 + piety * 0.8;
            for b in 0..n {
                if a == b || !self.cities[b].alive {
                    continue;
                }
                let cb = &self.cities[b];
                let d = self.planet.dist(ca.x, ca.y, cb.x, cb.y);
                if d < 10.0 {
                    pressure[b].push((r, strength * (1.0 - d / 10.0)));
                }
            }
        }
        for r in &self.routes {
            if !r.alive {
                continue;
            }
            for (x, y) in [(r.a, r.b), (r.b, r.a)] {
                if let Some(rel) = self.cities[x as usize].religion {
                    pressure[y as usize].push((rel, 1.5));
                }
            }
        }
        for b in 0..n {
            if !self.cities[b].alive || pressure[b].is_empty() {
                continue;
            }
            let mut tally: Vec<(usize, f32)> = Vec::new();
            for &(r, p) in &pressure[b] {
                if let Some(e) = tally.iter_mut().find(|e| e.0 == r) {
                    e.1 += p;
                } else {
                    tally.push((r, p));
                }
            }
            let (top, p) = tally.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
            let city = &mut self.cities[b];
            if city.religion == Some(top) {
                city.conv_pressure = 0.0;
                continue;
            }
            let resist = 2.0 + city.yields.faith * 0.5;
            city.conv_pressure += p - resist;
            if city.conv_pressure > 20.0 {
                city.conv_pressure = 0.0;
                let old = city.religion;
                city.religion = Some(top);
                let id = city.id;
                if self.rng.gen_bool(0.35) {
                    let s = StorySeed::new("conversion", Scope::Local)
                        .slot("City", self.cities[b].name.clone())
                        .slot("Religion", self.religions[top].name.clone())
                        .city(id)
                        .civ(self.cities[b].civ)
                        .cause(if old.is_some() { "missionaries and pilgrims outnumbered the old priests" } else { "missionaries and trade brought the faith" })
                        .cat("religious");
                    self.seeds.push(s);
                }
            }
        }
    }

    pub fn update_borders(&mut self) {
        let n = self.planet.tiles.len();
        let mut best: Vec<(f32, Option<u16>)> = vec![(0.0, None); n];
        for c in self.cities.iter().filter(|c| c.alive) {
            let r = c.radius + 1;
            let strength = 1.0 + c.culture.sqrt() * 0.15 + c.pop * 0.1;
            for t in self.planet.tiles_in_radius(c.x, c.y, r) {
                let (tx, ty) = self.planet.xy(t);
                let d2 = self.planet.dist2(c.x, c.y, tx, ty);
                if d2 > (c.radius * c.radius + c.radius) as f32 {
                    continue;
                }
                let inf = strength / (1.0 + d2);
                if inf > best[t].0 {
                    best[t] = (inf, Some(c.civ));
                }
            }
        }
        for t in 0..n {
            let tile = &mut self.planet.tiles[t];
            if tile.city_slot.is_some() {
                let ci = tile.city_slot.unwrap() as usize;
                if self.cities[ci].alive {
                    tile.owner = Some(self.cities[ci].civ);
                    continue;
                } else {
                    tile.city_slot = None;
                }
            }
            // incumbency: owner keeps the tile unless clearly out-pressured
            tile.owner = match (tile.owner, best[t].1) {
                (Some(o), Some(nw)) if o != nw => {
                    if self.civs[o as usize].alive && self.rng.gen_bool(0.7) {
                        Some(o)
                    } else {
                        Some(nw)
                    }
                }
                (Some(o), None) => {
                    if self.civs[o as usize].alive {
                        None
                    } else {
                        None
                    }
                }
                (_, nw) => nw,
            };
        }
        // border friction between neighbors
        let mut friction: Vec<(u16, u16)> = Vec::new();
        for t in (0..n).step_by(3) {
            if let Some(o) = self.planet.tiles[t].owner {
                for j in self.planet.neighbors8(t) {
                    if let Some(p) = self.planet.tiles[j].owner {
                        if p != o && !friction.contains(&(o, p)) {
                            friction.push((o, p));
                        }
                    }
                }
            }
        }
        for (a, b) in friction {
            if !self.r(a, b).met {
                self.meet(a, b);
            }
            let expansion = self.civs[b as usize].leader.traits[T_EXPANSION];
            self.set_ledger(a, b, Reason::BorderFriction, -4.0 - expansion * 8.0, "shared border");
        }
    }

    fn loyalty_pressure(&mut self) {
        let n = self.cities.len();
        for a in 0..n {
            if !self.cities[a].alive || self.cities[a].is_capital {
                continue;
            }
            let ca = self.cities[a].clone();
            let mut foreign: Option<(u16, f32)> = None;
            let mut own = 0.0;
            for b in 0..n {
                if a == b || !self.cities[b].alive {
                    continue;
                }
                let cb = &self.cities[b];
                let d = self.planet.dist(ca.x, ca.y, cb.x, cb.y);
                if d > 9.0 {
                    continue;
                }
                let p = (cb.pop + cb.culture.sqrt() * 0.2) * (1.0 - d / 9.0);
                if cb.civ == ca.civ {
                    own += p;
                } else if foreign.map(|f| p > f.1).unwrap_or(true) && !self.civs[cb.civ as usize].minor {
                    foreign = Some((cb.civ, p));
                }
            }
            own += ca.pop * 1.5 + ca.stability * 0.1;
            let city = &mut self.cities[a];
            if let Some((fc, fp)) = foreign {
                if fp > own * 1.2 && city.stability < 35.0 {
                    city.loyalty -= (fp - own) * 0.8;
                } else {
                    city.loyalty = (city.loyalty + 5.0).min(100.0);
                }
                if city.loyalty <= 0.0 && !self.r(ca.civ, fc).at_war {
                    let old = ca.civ;
                    self.transfer_city(a as u32, fc);
                    self.cities[a].loyalty = 60.0;
                    self.ledger(old, fc, Reason::CityTaken, -25.0, Some(80), format!("{} defected", ca.name));
                    let s = StorySeed::new("city_flips", Scope::National)
                        .slot("City", ca.name.clone())
                        .slot("Old", self.civ_name(old))
                        .slot("New", self.civ_name(fc))
                        .city(a as u32)
                        .civ(fc)
                        .civ2(old)
                        .at(ca.x, ca.y)
                        .cause(format!("cultural pull of the {}", self.civ_name(fc)))
                        .cause(format!("low stability ({:.0})", ca.stability))
                        .cat("political")
                        .card();
                    self.seeds.push(s);
                }
            } else {
                city.loyalty = (city.loyalty + 5.0).min(100.0);
            }
        }
    }

    pub fn transfer_city(&mut self, ci: u32, to: u16) {
        let from = self.cities[ci as usize].civ;
        if from == to {
            return;
        }
        self.civs[from as usize].cities.retain(|&c| c != ci);
        self.civs[to as usize].cities.push(ci);
        let was_capital = self.cities[ci as usize].is_capital;
        self.cities[ci as usize].civ = to;
        self.cities[ci as usize].is_capital = false;
        self.cities[ci as usize].build = None;
        self.cities[ci as usize].build_progress = 0.0;
        if was_capital {
            let next = self.civs[from as usize].cities.iter().copied().max_by(|a, b| self.cities[*a as usize].pop.partial_cmp(&self.cities[*b as usize].pop).unwrap());
            self.civs[from as usize].capital = next;
            if let Some(n) = next {
                self.cities[n as usize].is_capital = true;
            }
        }
        let (x, y, r) = (self.cities[ci as usize].x, self.cities[ci as usize].y, self.cities[ci as usize].radius);
        for t in self.planet.tiles_in_radius(x, y, r) {
            if self.planet.tiles[t].owner == Some(from) {
                self.planet.tiles[t].owner = Some(to);
            }
        }
        // armies homed there are lost
        for a in self.armies.iter_mut() {
            if a.home == Some(ci) && a.owner == Some(from) {
                a.home = self.civs[from as usize].capital;
            }
        }
        self.update_catchment(ci);
        if self.civs[from as usize].cities.is_empty() {
            self.civ_falls(from, "conquest", Some(to));
        }
    }

    pub fn civ_falls(&mut self, cid: u16, cause: &str, by: Option<u16>) {
        let c = cid as usize;
        if !self.civs[c].alive {
            return;
        }
        self.civs[c].alive = false;
        self.civs[c].fell = Some(self.year);
        self.extinct_civs += 1;
        for o in 0..self.civs.len() {
            self.rel[c * MAX_CIVS + o].at_war = false;
            self.rel[o * MAX_CIVS + c].at_war = false;
        }
        for a in self.armies.iter_mut() {
            if a.owner == Some(cid) {
                a.alive = false;
            }
            if a.enemy == Some(cid) {
                a.enemy = None;
                a.target = None;
            }
        }
        // ruins mark the homeland
        if let Some(&ci) = self.civs[c].cities.first().or(self.civs[c].capital.as_ref()) {
            let (x, y) = (self.cities[ci as usize].x, self.cities[ci as usize].y);
            let i = self.planet.idx(x, y).unwrap();
            self.planet.tiles[i].ruin = Some(format!("Ruins of the {}", self.civs[c].name));
        }
        if let Some(b) = by {
            for o in 0..self.civs.len() as u16 {
                if o != b && o != cid && self.civs[o as usize].alive && self.r(o, b).met {
                    self.ledger(o, b, Reason::Atrocity, -10.0, Some(60), format!("destroyed the {}", self.civs[c].name));
                }
            }
        }
        let mut s = StorySeed::new(if self.civs[c].minor { "citystate_falls" } else { "civ_falls" }, Scope::Global)
            .slot("Civ", self.civs[c].name.clone())
            .slot("Years", (self.year - self.civs[c].founded).to_string())
            .slot("Leader", format!("{} {}", self.civs[c].leader.title, self.civs[c].leader.name))
            .civ(cid)
            .cause(cause.to_string())
            .cat("political")
            .tone(Tone::Tragedy)
            .alert(if self.civs[c].minor { AlertClass::Major } else { AlertClass::Existential });
        if let Some(b) = by {
            s = s.slot("By", self.civ_name(b)).civ2(b);
        }
        s = if self.civs[c].minor { s.card() } else { s.chronicle() };
        self.seeds.push(s);
    }

    fn check_civil_wars(&mut self) {
        if self.year % 3 != 0 {
            return;
        }
        for c in 0..self.civs.len() {
            let civ = &self.civs[c];
            if !civ.alive || civ.minor || civ.cities.len() < 4 || self.civs.len() >= MAX_CIVS {
                continue;
            }
            if self.civs.iter().filter(|c| c.alive).count() >= 16 {
                return;
            }
            let unstable: Vec<u32> = civ.cities.iter().copied().filter(|&ci| self.cities[ci as usize].stability < 18.0 && !self.cities[ci as usize].is_capital).collect();
            let crisis = self.cities.iter().any(|ct| ct.civ == c as u16 && ct.chains.succession >= 2);
            if unstable.len() * 2 >= civ.cities.len() || (crisis && unstable.len() >= 2) {
                if !self.rng.gen_bool(0.2) {
                    continue;
                }
                let cid = c as u16;
                let seat = unstable[0];
                let (x, y) = (self.cities[seat as usize].x, self.cities[seat as usize].y);
                let traits = self.civs[c].leader.traits;
                let Some(nid) = self.found_civ(x, y, false, Some(cid), Some(traits)) else { return };
                // successor gets the unstable cities
                let cnt = unstable.len();
                for &u in &unstable {
                    self.transfer_city(u, nid);
                }
                if let Some(&cap) = self.civs[nid as usize].cities.first() {
                    self.civs[nid as usize].capital = Some(cap);
                    self.cities[cap as usize].is_capital = true;
                }
                for &u in &unstable {
                    self.cities[u as usize].stability = 50.0;
                    self.cities[u as usize].unrest = 0.0;
                    self.cities[u as usize].orig_civ = nid;
                }
                self.meet(cid, nid);
                self.civs[nid as usize].ability = self.civs[c].ability;
                self.civs[nid as usize].religion = self.civs[c].religion;
                self.civs[nid as usize].government = self.civs[c].government;
                self.ledger(cid, nid, Reason::WarGrievance, -40.0, Some(60), "rebels");
                self.ledger(nid, cid, Reason::WarGrievance, -30.0, Some(60), "oppressors");
                let s = StorySeed::new("civil_war", Scope::Global)
                    .slot("Civ", self.civ_name(cid))
                    .slot("New", self.civ_name(nid))
                    .slot("City", self.city_name(seat))
                    .slot("Count", cnt.to_string())
                    .civ(cid)
                    .civ2(nid)
                    .at(x, y)
                    .cause(format!("{} cities with collapsing stability", cnt))
                    .cat("political")
                    .tone(Tone::Tragedy)
                    .alert(AlertClass::Major)
                    .chronicle();
                let s = if crisis { s.cause("a contested succession") } else { s };
                self.seeds.push(s);
                if self.rng.gen_bool(0.6) {
                    self.declare_war(cid, nid, "reunification".to_string(), vec!["rebellion".into()]);
                }
                return;
            }
        }
    }

    // ------------------------------------------------------------------ diplomacy
    pub fn update_diplomacy(&mut self) {
        let year = self.year;
        let n = self.civs.len();
        // ledger upkeep and scores
        for a in 0..n {
            for b in 0..n {
                if a == b || !self.civs[a].alive || !self.civs[b].alive {
                    continue;
                }
                let r = &mut self.rel[a * MAX_CIVS + b];
                if !r.met {
                    continue;
                }
                r.ledger.retain(|e| e.expires.map(|x| x > year).unwrap_or(true));
                let mut s = 0.0;
                for e in &r.ledger {
                    s += e.value;
                }
                r.score = s;
            }
        }
        if year % 2 != 0 {
            return;
        }
        // situational reasons
        for a in 0..n {
            if !self.civs[a].alive {
                continue;
            }
            for b in 0..n {
                if a == b || !self.civs[b].alive || !self.rel[a * MAX_CIVS + b].met {
                    continue;
                }
                let (ra, rb) = (self.civs[a].religion, self.civs[b].religion);
                if ra.is_some() && ra == rb {
                    self.set_ledger(a as u16, b as u16, Reason::SharedReligion, 10.0 + self.civs[a].leader.traits[T_PIETY] * 10.0, "shared faith");
                } else if ra.is_some() && rb.is_some() {
                    self.set_ledger(a as u16, b as u16, Reason::RivalReligion, -5.0 - self.civs[a].leader.traits[T_PIETY] * 10.0, "rival holy cities");
                }
                let ga = GOVS[self.civs[a].government].market;
                let gb = GOVS[self.civs[b].government].market;
                self.set_ledger(a as u16, b as u16, Reason::Ideology, if ga != gb { -10.0 } else { 0.0 }, "ideology");
                // agendas
                let ag = self.civs[a].agenda_visible;
                let agh = self.civs[a].agenda_hidden;
                let mut av = 0.0;
                for (k, w) in [(ag, 1.0), (agh, 0.8)] {
                    av += w * match AGENDAS[k].0 {
                        "Iron Fist" => (self.civs[b].military - self.civs[a].military).signum() * 6.0,
                        "Green Mantle" => (self.civs[b].leader.traits[T_ECOLOGY] - 0.5) * 20.0,
                        "Gold Hoarder" => if self.civs[b].treasury > self.civs[a].treasury * 1.5 { -8.0 } else { 0.0 },
                        "Scholar's Lamp" => (self.civs[b].known_tech.iter().filter(|x| **x).count() as f32 - self.civs[a].known_tech.iter().filter(|x| **x).count() as f32).clamp(-3.0, 3.0) * 2.0,
                        "Great Builder" => self.civs[b].wonders.len() as f32 * -3.0,
                        "Frontier Hunger" => if self.civs[b].cities.len() < 3 { -8.0 } else { 0.0 },
                        "Kin Loyalty" => if self.rel[a * MAX_CIVS + b].alliance { 10.0 } else { 0.0 },
                        _ => 0.0,
                    };
                }
                self.set_ledger(a as u16, b as u16, Reason::Agenda, av, "agenda");
                // wealth envy (greedy leaders)
                let (wa, _, _) = self.civ_wealth(a as u16);
                let (wb, _, _) = self.civ_wealth(b as u16);
                let env = if wb > wa + 150.0 { -(wb - wa) / 30.0 * self.civs[a].leader.traits[T_GREED] } else { 0.0 };
                self.set_ledger(a as u16, b as u16, Reason::WealthEnvy, env, "wealth envy");
            }
        }
        // deals: trade agreements, open borders, alliances, routes
        for a in 0..n {
            for b in (a + 1)..n {
                if !self.civs[a].alive || !self.civs[b].alive {
                    continue;
                }
                let (au, bu) = (a as u16, b as u16);
                if !self.r(au, bu).met || self.r(au, bu).at_war {
                    continue;
                }
                let s = (self.r(au, bu).score + self.r(bu, au).score) / 2.0;
                let greed = (self.civs[a].leader.traits[T_GREED] + self.civs[b].leader.traits[T_GREED]) / 2.0;
                if !self.r(au, bu).trade_agreement && s > -5.0 && self.rng.gen_bool((0.15 + greed * 0.3) as f64) {
                    self.rm(au, bu).trade_agreement = true;
                    self.rm(bu, au).trade_agreement = true;
                    self.deal_news("trade_agreement", au, bu, "both treasuries stand to gain");
                }
                if !self.r(au, bu).open_borders && s > 10.0 && self.rng.gen_bool(0.2) {
                    self.rm(au, bu).open_borders = true;
                    self.rm(bu, au).open_borders = true;
                }
                if !self.r(au, bu).research_agreement && s > 40.0 && self.rng.gen_bool(0.15) {
                    self.rm(au, bu).research_agreement = true;
                    self.rm(bu, au).research_agreement = true;
                    self.deal_news("research_agreement", au, bu, "declared friendship");
                }
                if self.r(au, bu).research_agreement && year % 10 == 0 {
                    // share a known technology
                    for (x, y) in [(a, b), (b, a)] {
                        if let Some(k) = (0..TECHS.len()).find(|&k| self.civs[x].known_tech[k] && !self.civs[y].known_tech[k] && TECHS[k].prereq.iter().all(|p| self.civs[y].known_tech[tech_index(p)])) {
                            self.civs[y].eureka[k] = true;
                        }
                    }
                }
                if !self.r(au, bu).alliance && s > 28.0 && !self.civs[a].minor && !self.civs[b].minor && self.rng.gen_bool(0.1) {
                    self.rm(au, bu).alliance = true;
                    self.rm(bu, au).alliance = true;
                    self.rm(au, bu).defensive_pact = true;
                    self.rm(bu, au).defensive_pact = true;
                    self.ledger(au, bu, Reason::Alliance, 15.0, None, "alliance");
                    self.ledger(bu, au, Reason::Alliance, 15.0, None, "alliance");
                    self.deal_news("alliance", au, bu, "long friendship and shared rivals");
                }
                if self.r(au, bu).alliance && s < 0.0 {
                    // alliance breaks; opportunists may backstab
                    self.rm(au, bu).alliance = false;
                    self.rm(bu, au).alliance = false;
                    self.deal_news("alliance_broken", au, bu, "relations soured");
                }
                if !self.r(au, bu).trade_agreement && self.r(au, bu).nonaggression_until < year && s > 0.0 && self.rng.gen_bool(0.05) {
                    self.rm(au, bu).nonaggression_until = year + 30;
                    self.rm(bu, au).nonaggression_until = year + 30;
                }
            }
        }
        // city-state suzerainty
        for m in 0..n {
            if !self.civs[m].alive || !self.civs[m].minor {
                continue;
            }
            let mut best: Option<(u16, f32)> = None;
            for o in 0..n {
                if o == m || !self.civs[o].alive || self.civs[o].minor || !self.rel[m * MAX_CIVS + o].met {
                    continue;
                }
                let infl = self.rel[m * MAX_CIVS + o].score + self.civs[o].military * 0.1 + self.civs[o].treasury.max(0.0) * 0.02;
                if best.map(|b| infl > b.1).unwrap_or(true) {
                    best = Some((o as u16, infl));
                }
            }
            let new = best.filter(|b| b.1 > 15.0).map(|b| b.0);
            if new != self.civs[m].suzerain {
                self.civs[m].suzerain = new;
                if let Some(s) = new {
                    let seed = StorySeed::new("suzerain", Scope::Regional)
                        .slot("CityState", self.civ_name(m as u16))
                        .slot("Civ", self.civ_name(s))
                        .slot("Kind", self.civs[m].minor_kind)
                        .civ(s)
                        .civ2(m as u16)
                        .cause("envoys, trade and intimidation")
                        .cat("political");
                    self.seeds.push(seed);
                }
            }
        }
        self.create_routes();
        self.war_decisions();
        self.peace_decisions();
    }

    fn deal_news(&mut self, kind: &'static str, a: u16, b: u16, cause: &str) {
        if self.civs[a as usize].minor && self.civs[b as usize].minor {
            return;
        }
        let s = StorySeed::new(kind, Scope::National)
            .slot("CivA", self.civ_name(a))
            .slot("CivB", self.civ_name(b))
            .slot("LeaderA", self.civs[a as usize].leader.name.clone())
            .slot("LeaderB", self.civs[b as usize].leader.name.clone())
            .civ(a)
            .civ2(b)
            .cause(cause.to_string())
            .cat("political");
        let s = if kind == "alliance" || kind == "alliance_broken" { s.card() } else { s };
        self.seeds.push(s);
    }

    fn create_routes(&mut self) {
        let n = self.cities.len();
        let year = self.year;
        for a in 0..n {
            if !self.cities[a].alive || self.rng.gen_bool(0.75) {
                continue;
            }
            let ca = &self.cities[a];
            let civ = &self.civs[ca.civ as usize];
            if !civ.knows("Pottery") {
                continue;
            }
            let cap = 1 + ca.has("Market") as usize + ca.has("Harbor") as usize + ca.has("Bank") as usize + civ.era / 3 + if civ.strategy == Strategy::Trade { 1 } else { 0 };
            let existing = self.routes.iter().filter(|r| r.alive && (r.a as usize == a || r.b as usize == a)).count();
            if existing >= cap {
                continue;
            }
            let sail = civ.knows("Sailing");
            let nav = civ.knows("Navigation");
            let land_range = 9.0 + civ.era as f32 * 2.5;
            let sea_range = if nav { 60.0 } else if sail { 20.0 } else { 0.0 };
            let conta = self.planet.tiles[self.planet.idx(ca.x, ca.y).unwrap()].continent;
            let mut best: Option<(usize, f32, bool)> = None;
            for b in 0..n {
                if a == b || !self.cities[b].alive {
                    continue;
                }
                let cb = &self.cities[b];
                if self.routes.iter().any(|r| r.alive && ((r.a as usize == a && r.b as usize == b) || (r.b as usize == a && r.a as usize == b))) {
                    continue;
                }
                if ca.civ != cb.civ {
                    let r = self.r(ca.civ, cb.civ);
                    if !r.met || r.at_war || (!r.trade_agreement && r.score < 10.0) {
                        continue;
                    }
                }
                let d = self.planet.dist(ca.x, ca.y, cb.x, cb.y);
                let contb = self.planet.tiles[self.planet.idx(cb.x, cb.y).unwrap()].continent;
                let same = conta == contb;
                let sea = !same || (ca.coastal && cb.coastal && d > land_range);
                if sea && !(ca.coastal && cb.coastal && d <= sea_range) {
                    continue;
                }
                if !sea && d > land_range {
                    continue;
                }
                let comp = cb.resources.iter().filter(|r| !ca.resources.contains(r)).count() as f32 + ca.resources.iter().filter(|r| !cb.resources.contains(r)).count() as f32;
                let gap = (ca.wealth - cb.wealth).abs() / 250.0;
                let foreign = if ca.civ != cb.civ { 1.0 } else { 0.0 };
                let v = 2.0 + comp * 0.8 + gap + foreign + (ca.pop + cb.pop) * 0.06 - d * 0.05 + if sea { 0.8 } else { 0.0 };
                if best.map(|bb| v > bb.1).unwrap_or(true) {
                    best = Some((b, v, sea));
                }
            }
            if let Some((b, v, sea)) = best {
                if v < 1.5 {
                    continue;
                }
                let era_mult = 1.0 + self.civs[self.cities[a].civ as usize].era as f32 * 0.5;
                self.routes.push(TradeRoute { a: a as u32, b: b as u32, value: v * era_mult, sea, created: year, alive: true, raided_until: 0 });
                let (ca, cb) = (self.cities[a].civ, self.cities[b].civ);
                if ca != cb {
                    self.meet(ca, cb);
                    let k = tech_index("Currency");
                    self.civs[ca as usize].eureka[k] = true;
                    let s = StorySeed::new("route_opened", Scope::Regional)
                        .slot("CityA", self.city_name(a as u32))
                        .slot("CityB", self.city_name(b as u32))
                        .slot("CivA", self.civ_name(ca))
                        .slot("CivB", self.civ_name(cb))
                        .slot("Mode", if sea { "sea" } else { "road" })
                        .city(a as u32)
                        .civ(ca)
                        .civ2(cb)
                        .at(self.cities[a].x, self.cities[a].y)
                        .cause("complementary goods and a profitable distance")
                        .cat("economic");
                    self.seeds.push(s);
                }
            }
        }
    }

    fn reachable(&self, a: u16, target: u32) -> bool {
        let civ = &self.civs[a as usize];
        let t = &self.cities[target as usize];
        let tc = self.planet.tiles[self.planet.idx(t.x, t.y).unwrap()].continent;
        let near = civ.cities.iter().any(|&ci| {
            let c = &self.cities[ci as usize];
            let cc = self.planet.tiles[self.planet.idx(c.x, c.y).unwrap()].continent;
            let d = self.planet.dist(c.x, c.y, t.x, t.y);
            (cc == tc && d < 22.0) || (civ.knows("Sailing") && c.coastal && t.coastal && d < 16.0) || (civ.knows("Navigation") && c.coastal && t.coastal && d < 45.0)
        });
        near
    }

    fn war_decisions(&mut self) {
        let n = self.civs.len();
        let year = self.year;
        for a in 0..n {
            let ca = &self.civs[a];
            if !ca.alive || ca.minor || ca.cities.is_empty() {
                continue;
            }
            let at_war_count = (0..n).filter(|&o| self.rel[a * MAX_CIVS + o].at_war).count();
            if at_war_count > 0 || ca.war_weariness > 20.0 {
                continue;
            }
            let t = ca.leader.traits;
            let mut best: Option<(u16, f32, Vec<String>, u32)> = None;
            for b in 0..n {
                if a == b || !self.civs[b].alive {
                    continue;
                }
                let r = &self.rel[a * MAX_CIVS + b];
                if !r.met || r.at_war || r.alliance || year - r.last_war_end < 25 {
                    continue;
                }
                // casus belli from grievances
                let grievance: f32 = r.ledger.iter().filter(|e| e.value < 0.0).map(|e| -e.value).sum();
                let needed = 20.0 + (1.0 - t[T_AGGRESSION]) * 70.0;
                let promise = r.nonaggression_until > year;
                let mut cb_case: Vec<String> = r.ledger.iter().filter(|e| e.value < -4.0).map(|e| format!("{} ({})", e.reason.text(), e.note)).collect();
                // resource need
                let need_iron = !self.civs[a].cities.iter().any(|&ci| self.cities[ci as usize].resources.contains(&ResKind::Iron));
                let mut target: Option<(u32, f32)> = None;
                for &ci in &self.civs[b].cities {
                    if !self.reachable(a as u16, ci) {
                        continue;
                    }
                    let c = &self.cities[ci as usize];
                    let mut v = c.wealth / 200.0 + c.pop * 0.1 + c.wonders.len() as f32;
                    if need_iron && c.resources.contains(&ResKind::Iron) {
                        v += 2.0;
                    }
                    let d = self.civs[a].capital.map(|cap| self.planet.dist(self.cities[cap as usize].x, self.cities[cap as usize].y, c.x, c.y)).unwrap_or(30.0);
                    v -= d * 0.08;
                    v -= c.defense(self.civs[b].military_tier()) * 0.05;
                    if target.map(|tt| v > tt.1).unwrap_or(true) {
                        target = Some((ci, v));
                    }
                }
                let Some((tc, tv)) = target else { continue };
                let my = self.civs[a].military + 1.0;
                let their = self.civs[b].military + self.civs[b].cities.iter().map(|&ci| self.cities[ci as usize].defense(self.civs[b].military_tier())).sum::<f32>() * 0.3 + 1.0;
                let ratio = my / their;
                let thresh = 1.5 - t[T_AGGRESSION] * 0.6;
                if ratio < thresh {
                    continue;
                }
                if grievance < needed && !(self.civs[b].minor && t[T_AGGRESSION] > 0.7) {
                    continue;
                }
                if promise && t[T_LOYALTY] > 0.4 {
                    continue;
                }
                if promise {
                    cb_case.push("broke a non-aggression promise".into());
                }
                if need_iron {
                    cb_case.push("need for iron".into());
                }
                let score = tv + ratio + grievance * 0.02 + self.rng.gen_range(0.0..1.0);
                if best.as_ref().map(|bb| score > bb.1).unwrap_or(true) {
                    best = Some((b as u16, score, cb_case, tc));
                }
            }
            if let Some((b, _, case, tc)) = best {
                if self.rng.gen_bool(0.08 + self.civs[a].leader.traits[T_AGGRESSION] as f64 * 0.17) {
                    let aim = format!("conquest of {}", self.city_name(tc));
                    let broke = self.r(a as u16, b).nonaggression_until > year;
                    self.declare_war(a as u16, b, aim, case);
                    if broke {
                        self.promise_broken(a as u16, b);
                    }
                }
            }
        }
    }

    fn promise_broken(&mut self, a: u16, victim: u16) {
        for o in 0..self.civs.len() as u16 {
            if o != a && self.civs[o as usize].alive && self.r(o, a).met {
                let v = if o == victim { -35.0 } else { -12.0 };
                self.ledger(o, a, Reason::BrokenPromise, v, Some(100), format!("attacked the {} despite a promise", self.civ_name(victim)));
            }
        }
        self.civs[victim as usize].betrayed_memory += 1.0;
        if self.civs[a as usize].leader.defining_act.is_none() {
            self.civs[a as usize].leader.defining_act = Some("broke a sworn promise".into());
        }
    }

    pub fn declare_war(&mut self, a: u16, b: u16, aim: String, case: Vec<String>) {
        if self.r(a, b).at_war || !self.civs[a as usize].alive || !self.civs[b as usize].alive {
            return;
        }
        let year = self.year;
        for (x, y) in [(a, b), (b, a)] {
            let r = self.rm(x, y);
            r.at_war = true;
            r.war_start = year;
            r.war_aim = aim.clone();
            r.alliance = false;
            r.trade_agreement = false;
            r.open_borders = false;
        }
        let k = tech_index("Iron Working");
        self.civs[a as usize].eureka[k] = true;
        self.civs[b as usize].eureka[k] = true;
        if case.is_empty() {
            for o in 0..self.civs.len() as u16 {
                if o != a && self.r(o, a).met {
                    self.ledger(o, a, Reason::WarGrievance, -10.0, Some(50), "surprise war without cause");
                }
            }
        }
        self.ledger(b, a, Reason::WarGrievance, -30.0, Some(80), "declared war on us");
        let grievance_age = self.r(a, b).ledger.iter().filter_map(|e| e.expires).min().map(|e| (e - self.year).max(5)).unwrap_or(10);
        let claim = aim.trim_start_matches("conquest of ").to_string();
        let pos = self.civs[b as usize].capital.map(|c| (self.cities[c as usize].x, self.cities[c as usize].y));
        let causes = if case.is_empty() { vec![] } else { case.iter().take(3).cloned().collect::<Vec<_>>() };
        let defense = ["raises the walls", "calls every garrison home", "orders levies to the border", "seeks allies abroad"][self.rng.gen_range(0..4)];
        let motive = ["plunder", "land", "pride", "gold"][self.rng.gen_range(0..4)];
        let thread = format!("War: {} vs {}", self.civ_name(a), self.civ_name(b));
        for persp in ["W", "A", "B"] {
            let mut s = StorySeed::new("war_declared", if persp == "W" { Scope::Global } else { Scope::National })
                .slot("CivA", self.civ_name(a))
                .slot("CivB", self.civ_name(b))
                .slot("LeaderA", self.civs[a as usize].leader.name.clone())
                .slot("LeaderB", self.civs[b as usize].leader.name.clone())
                .slot("WarAim", aim.clone())
                .slot("Claim", claim.clone())
                .slot("GrievanceAge", grievance_age.to_string())
                .slot("Motive", motive)
                .slot("Defense", defense)
                .civ(if persp == "B" { b } else { a })
                .civ2(if persp == "B" { a } else { b })
                .cat("military")
                .tone(Tone::Alarm)
                .persp(persp)
                .thread(thread.clone());
            for c in &causes {
                s = s.cause(c.clone());
            }
            if let Some((x, y)) = pos {
                s = s.at(x, y);
            }
            if persp == "W" {
                s = s.alert(AlertClass::Major).card();
                if !self.civs[b as usize].minor {
                    s = s.chronicle();
                }
            }
            self.seeds.push(s);
        }
        // allies join
        for o in 0..self.civs.len() as u16 {
            if o != a && o != b && self.civs[o as usize].alive && self.r(o, b).defensive_pact && !self.r(o, a).at_war {
                if self.civs[o as usize].leader.traits[T_LOYALTY] > 0.35 {
                    let year = self.year;
                    let r = self.rm(o, a);
                    r.at_war = true;
                    r.war_start = year;
                    let r2 = self.rm(a, o);
                    r2.at_war = true;
                    r2.war_start = year;
                    let s = StorySeed::new("ally_joins", Scope::National)
                        .slot("Ally", self.civ_name(o))
                        .slot("CivA", self.civ_name(a))
                        .slot("CivB", self.civ_name(b))
                        .civ(o)
                        .civ2(a)
                        .cause(format!("defensive pact with the {}", self.civ_name(b)))
                        .cat("military")
                        .card();
                    self.seeds.push(s);
                } else {
                    self.promise_broken(o, b);
                    self.rm(o, b).alliance = false;
                    self.rm(b, o).alliance = false;
                    self.rm(o, b).defensive_pact = false;
                    self.rm(b, o).defensive_pact = false;
                }
            }
        }
        // dispatch armies
        self.assign_war_targets(a);
        self.assign_war_targets(b);
    }

    fn assign_war_targets(&mut self, c: u16) {
        let enemies: Vec<u16> = (0..self.civs.len() as u16).filter(|&o| self.r(c, o).at_war).collect();
        if enemies.is_empty() {
            return;
        }
        let ids: Vec<usize> = (0..self.armies.len()).filter(|&k| self.armies[k].alive && self.armies[k].owner == Some(c)).collect();
        let total = ids.len();
        for (n, k) in ids.into_iter().enumerate() {
            // keep a third home as defenders
            if n * 3 >= total * 2 && total > 1 {
                self.armies[k].target = self.armies[k].home;
                self.armies[k].enemy = None;
                continue;
            }
            let (ax, ay) = (self.armies[k].x, self.armies[k].y);
            let mut best: Option<(u32, u16, f32)> = None;
            for &e in &enemies {
                for &ci in &self.civs[e as usize].cities {
                    if !self.reachable(c, ci) {
                        continue;
                    }
                    let city = &self.cities[ci as usize];
                    let d = self.planet.dist(ax as i32, ay as i32, city.x, city.y);
                    let v = city.wealth / 300.0 - d * 0.1 - city.defense(self.civs[e as usize].military_tier()) * 0.03;
                    if best.map(|b| v > b.2).unwrap_or(true) {
                        best = Some((ci, e, v));
                    }
                }
            }
            if let Some((ci, e, _)) = best {
                self.armies[k].target = Some(ci);
                self.armies[k].enemy = Some(e);
            }
        }
    }

    fn peace_decisions(&mut self) {
        let n = self.civs.len();
        for a in 0..n {
            for b in (a + 1)..n {
                if !self.rel[a * MAX_CIVS + b].at_war {
                    continue;
                }
                let (au, bu) = (a as u16, b as u16);
                let dur = self.year - self.r(au, bu).war_start;
                let wa = self.civs[a].war_weariness;
                let wb = self.civs[b].war_weariness;
                let ma = self.civs[a].military + 1.0;
                let mb = self.civs[b].military + 1.0;
                let want = dur > 6 && (wa > 35.0 || wb > 35.0 || (ma / mb > 4.0 || mb / ma > 4.0) && dur > 10) || dur > 30;
                if want && self.rng.gen_bool(0.35) {
                    self.make_peace(au, bu);
                }
            }
        }
    }

    pub fn make_peace(&mut self, a: u16, b: u16) {
        let dur = self.year - self.r(a, b).war_start;
        let year = self.year;
        for (x, y) in [(a, b), (b, a)] {
            let r = self.rm(x, y);
            r.at_war = false;
            r.last_war_end = year;
            r.nonaggression_until = year + 20;
        }
        // reparations from the wearier side
        let (loser, winner) = if self.civs[a as usize].war_weariness > self.civs[b as usize].war_weariness { (a, b) } else { (b, a) };
        let rep = (self.civs[loser as usize].treasury.max(0.0) * 0.3).round();
        self.civs[loser as usize].treasury -= rep;
        self.civs[winner as usize].treasury += rep;
        self.ledger(loser, winner, Reason::WarGrievance, -20.0, Some(80), "humiliating peace");
        for ar in self.armies.iter_mut() {
            if (ar.owner == Some(a) && ar.enemy == Some(b)) || (ar.owner == Some(b) && ar.enemy == Some(a)) {
                ar.enemy = None;
                ar.target = ar.home;
                ar.sieging = false;
            }
        }
        let thread = format!("War: {} vs {}", self.civ_name(a), self.civ_name(b));
        let thread2 = format!("War: {} vs {}", self.civ_name(b), self.civ_name(a));
        let s = StorySeed::new("peace", Scope::Global)
            .slot("CivA", self.civ_name(winner))
            .slot("CivB", self.civ_name(loser))
            .slot("Years", dur.to_string())
            .slot("Reparations", format!("{:.0}", rep))
            .civ(winner)
            .civ2(loser)
            .cause(format!("war weariness in the {}", self.civ_name(loser)))
            .cat("military")
            .thread(if self.news.followed_threads.contains(&thread2) { thread2 } else { thread })
            .card();
        self.seeds.push(s);
    }

    // ------------------------------------------------------------------ armies & raiders
    pub fn update_armies(&mut self) {
        let year = self.year;
        // spawn raiders near rich, weakly defended cities in wild regions
        if year % 4 == 0 && year > 60 {
            let n = self.cities.len();
            for _ in 0..2 {
                if n == 0 {
                    break;
                }
                let ci = self.rng.gen_range(0..n);
                let c = &self.cities[ci];
                if !c.alive || c.wealth < 350.0 || self.rng.gen_bool(0.6) {
                    continue;
                }
                // look for unclaimed land nearby
                let (cx, cy) = (c.x, c.y);
                let mut spot = None;
                for _ in 0..20 {
                    let dx = self.rng.gen_range(-8..=8);
                    let dy = self.rng.gen_range(-8..=8);
                    if let Some(i) = self.planet.idx(cx + dx, cy + dy) {
                        if self.planet.is_land(i) && self.planet.tiles[i].owner.is_none() {
                            spot = Some((cx + dx, cy + dy));
                            break;
                        }
                    }
                }
                if let Some((x, y)) = spot {
                    let era_avg = self.alive_civs().map(|c| c.era as f32).sum::<f32>() / self.alive_civs().count().max(1) as f32;
                    let id = self.next_army;
                    self.next_army += 1;
                    self.armies.push(Army {
                        id,
                        owner: None,
                        x: x as f32,
                        y: y as f32,
                        strength: 4.0 + era_avg * 3.0 + self.rng.gen_range(0.0..4.0),
                        target: Some(ci as u32),
                        enemy: None,
                        alive: true,
                        loot: 0.0,
                        home: None,
                        sieging: false,
                    });
                }
            }
        }
        let n = self.armies.len();
        for k in 0..n {
            if !self.armies[k].alive {
                continue;
            }
            let Some(t) = self.armies[k].target else {
                if let Some(h) = self.armies[k].home {
                    if self.cities[h as usize].alive {
                        let (hx, hy) = (self.cities[h as usize].x as f32, self.cities[h as usize].y as f32);
                        self.armies[k].x = hx;
                        self.armies[k].y = hy;
                    }
                }
                continue;
            };
            let tc = &self.cities[t as usize];
            if !tc.alive {
                self.armies[k].target = self.armies[k].home;
                continue;
            }
            let (tx, ty) = (tc.x as f32, tc.y as f32);
            let a = &self.armies[k];
            let dx = self.planet.wrap_dx(a.x, tx);
            let dy = ty - a.y;
            let d = (dx * dx + dy * dy).sqrt();
            let owner = a.owner;
            let speed = 3.0 + owner.map(|o| if self.civs[o as usize].ability == "Steppe Riders" { 1.5 } else { 0.0 } + self.civs[o as usize].era as f32 * 0.5).unwrap_or(1.0);
            let hostile = match owner {
                None => true,
                Some(o) => tc.civ != o && self.r(o, tc.civ).at_war,
            };
            if d > 1.5 {
                let step = speed.min(d - 1.0);
                let a = &mut self.armies[k];
                a.x = (a.x + dx / d * step).rem_euclid(self.planet.w as f32);
                a.y += dy / d * step;
                a.sieging = false;
                continue;
            }
            if !hostile {
                // arrived home or friendly: become garrison
                self.armies[k].target = None;
                continue;
            }
            // siege / assault
            self.armies[k].sieging = true;
            let tcivid = tc.civ;
            let tier = self.civs[tcivid as usize].military_tier();
            let mut def = self.cities[t as usize].defense(tier);
            if self.cities[t as usize].protected_until > year {
                def *= 2.5;
            }
            // defenders: armies at home in that city
            let defenders: Vec<usize> = (0..n).filter(|&j| self.armies[j].alive && self.armies[j].owner == Some(tcivid) && self.armies[j].target.is_none() && self.armies[j].home == Some(t)).collect();
            let def_str: f32 = defenders.iter().map(|&j| self.armies[j].strength).sum();
            let total_def = def + def_str;
            let att = self.armies[k].strength;
            let roll = self.rng.gen_range(0.7..1.3);
            let ratio = att * roll / total_def.max(1.0);
            self.cities[t as usize].siege += 2.0;
            let (sx, sy) = (self.cities[t as usize].x, self.cities[t as usize].y);
            for i in self.planet.tiles_in_radius(sx, sy, 1) {
                self.planet.tiles[i].scorch = (self.planet.tiles[i].scorch + 0.15).min(1.0);
                if self.planet.tiles[i].improvement == Improvement::Farm && self.rng.gen_bool(0.2) {
                    self.planet.tiles[i].improvement = Improvement::None;
                }
            }
            if let Some(i) = self.planet.idx(sx, sy) {
                self.planet.tiles[i].battle = true;
            }
            // losses
            self.armies[k].strength -= total_def * 0.12 * self.rng.gen_range(0.5..1.2);
            for &j in &defenders {
                self.armies[j].strength -= att * 0.08 * self.rng.gen_range(0.5..1.2);
                if self.armies[j].strength <= 0.5 {
                    self.armies[j].alive = false;
                }
            }
            self.cities[t as usize].garrison = (self.cities[t as usize].garrison - att * 0.02).max(0.0);
            if let Some(o) = owner {
                self.civs[o as usize].war_weariness += 1.5;
            }
            self.civs[tcivid as usize].war_weariness += 1.0;
            if self.armies[k].strength <= 0.5 {
                self.armies[k].alive = false;
                if owner.is_none() && self.rng.gen_bool(0.5) {
                    let c = &self.cities[t as usize];
                    let s = StorySeed::new("raiders_repelled", Scope::Local)
                        .slot("City", c.name.clone())
                        .city(t)
                        .civ(c.civ)
                        .at(c.x, c.y)
                        .cause("walls and garrison held")
                        .cat("military")
                        .tone(Tone::Triumph);
                    self.seeds.push(s);
                }
                continue;
            }
            if ratio > 1.15 {
                match owner {
                    None => {
                        // raiders loot and leave (wealth is a risk factor)
                        let c = &mut self.cities[t as usize];
                        let loot = c.wealth * 0.05 + 5.0;
                        let civid = c.civ;
                        c.pop = (c.pop * 0.95).max(1.0);
                        c.unrest += 6.0;
                        self.civs[civid as usize].treasury -= loot;
                        self.armies[k].alive = false;
                        let c = &self.cities[t as usize];
                        let s = StorySeed::new("raiders_loot", Scope::Local)
                            .slot("City", c.name.clone())
                            .slot("Amount", format!("{:.0}", loot))
                            .slot("Band", c.band())
                            .city(t)
                            .civ(c.civ)
                            .at(c.x, c.y)
                            .cause(format!("a {} city with thin defenses", c.band().to_lowercase()))
                            .cat("military")
                            .tone(Tone::Alarm);
                        self.seeds.push(s);
                    }
                    Some(o) => {
                        self.capture_city(o, t);
                        self.armies[k].target = None;
                        self.armies[k].home = Some(t);
                        self.armies[k].sieging = false;
                        self.assign_war_targets(o);
                    }
                }
            }
        }
        // field battles between warring armies close to each other
        for a in 0..n {
            if !self.armies[a].alive {
                continue;
            }
            for b in (a + 1)..n {
                if !self.armies[b].alive {
                    continue;
                }
                let (oa, ob) = (self.armies[a].owner, self.armies[b].owner);
                let hostile = match (oa, ob) {
                    (Some(x), Some(y)) => x != y && self.r(x, y).at_war,
                    (None, Some(_)) | (Some(_), None) => true,
                    _ => false,
                };
                if !hostile {
                    continue;
                }
                let dx = self.planet.wrap_dx(self.armies[a].x, self.armies[b].x);
                let dy = self.armies[b].y - self.armies[a].y;
                if dx * dx + dy * dy > 2.5 {
                    continue;
                }
                let sa = self.armies[a].strength * self.rng.gen_range(0.7..1.3);
                let sb = self.armies[b].strength * self.rng.gen_range(0.7..1.3);
                self.armies[a].strength -= sb * 0.35;
                self.armies[b].strength -= sa * 0.35;
                let (bx, by) = (self.armies[a].x as i32, self.armies[a].y as i32);
                if let Some(i) = self.planet.idx(bx, by) {
                    self.planet.tiles[i].battle = true;
                    self.planet.tiles[i].scorch = (self.planet.tiles[i].scorch + 0.3).min(1.0);
                }
                for k in [a, b] {
                    if self.armies[k].strength <= 0.5 {
                        self.armies[k].alive = false;
                    }
                }
                if let (Some(x), Some(y)) = (oa, ob) {
                    if (sa + sb) > 25.0 && self.rng.gen_bool(0.5) {
                        let (win, lose) = if sa > sb { (x, y) } else { (y, x) };
                        let s = StorySeed::new("battle", Scope::National)
                            .slot("Winner", self.civ_name(win))
                            .slot("Loser", self.civ_name(lose))
                            .slot("Region", self.region_name(bx, by))
                            .civ(win)
                            .civ2(lose)
                            .at(bx, by)
                            .cause("armies met in the field")
                            .cat("military");
                        self.seeds.push(s);
                    }
                }
            }
        }
        self.armies.retain(|a| a.alive || false);
        // periodically re-target armies at war that have no target
        if year % 3 == 0 {
            for c in 0..self.civs.len() as u16 {
                if self.civs[c as usize].alive && (0..self.civs.len() as u16).any(|o| self.r(c, o).at_war) {
                    let idle = self.armies.iter().any(|a| a.owner == Some(c) && a.target.is_none());
                    if idle {
                        self.assign_war_targets(c);
                    }
                }
            }
        }
    }

    fn capture_city(&mut self, by: u16, ci: u32) {
        let old = self.cities[ci as usize].civ;
        let c = &self.cities[ci as usize];
        let name = c.name.clone();
        let (x, y) = (c.x, c.y);
        let loot = c.wealth * 0.08;
        let raze = c.pop < 3.0 && self.civs[by as usize].leader.traits[T_VINDICTIVE] > 0.75 && self.civs[old as usize].cities.len() > 1;
        self.civs[by as usize].treasury += loot;
        self.civs[old as usize].treasury -= loot * 0.5;
        // refugees flow to the old civ's other cities
        let refugees = self.cities[ci as usize].pop * 0.2;
        self.cities[ci as usize].pop *= 0.75;
        if let Some(&dest) = self.civs[old as usize].cities.iter().find(|&&d| d != ci) {
            self.cities[dest as usize].pop += refugees * 0.6;
            self.cities[dest as usize].migration += refugees;
        }
        self.cities[ci as usize].backlog += 20.0;
        self.cities[ci as usize].unrest += 15.0;
        self.ledger(old, by, Reason::CityTaken, -30.0, Some(120), format!("took {}", name));
        let is_cap = self.cities[ci as usize].is_capital;
        if raze {
            self.cities[ci as usize].alive = false;
            self.civs[old as usize].cities.retain(|&c| c != ci);
            let i = self.planet.idx(x, y).unwrap();
            self.planet.tiles[i].ruin = Some(format!("Ruins of {}", name));
            self.planet.tiles[i].city_slot = None;
            for t in self.planet.tiles_in_radius(x, y, 2) {
                if self.planet.tiles[t].owner == Some(old) {
                    self.planet.tiles[t].owner = None;
                }
            }
            if is_cap {
                let next = self.civs[old as usize].cities.first().copied();
                self.civs[old as usize].capital = next;
                if let Some(n) = next {
                    self.cities[n as usize].is_capital = true;
                }
            }
            for o in 0..self.civs.len() as u16 {
                if o != by && self.r(o, by).met {
                    self.ledger(o, by, Reason::Atrocity, -15.0, Some(60), format!("razed {}", name));
                }
            }
            let s = StorySeed::new("city_razed", Scope::Global)
                .slot("City", name.clone())
                .slot("Civ", self.civ_name(by))
                .slot("Old", self.civ_name(old))
                .city(ci)
                .civ(by)
                .civ2(old)
                .at(x, y)
                .cause(format!("a grudging conqueror, {}", self.civs[by as usize].leader.name))
                .cat("military")
                .tone(Tone::Tragedy)
                .alert(AlertClass::Existential)
                .chronicle();
            self.seeds.push(s);
            if self.civs[old as usize].cities.is_empty() {
                self.civ_falls(old, "conquest", Some(by));
            }
            return;
        }
        let thread = format!("War: {} vs {}", self.civ_name(by), self.civ_name(old));
        let s = StorySeed::new("city_captured", Scope::National)
            .slot("City", name)
            .slot("Civ", self.civ_name(by))
            .slot("Old", self.civ_name(old))
            .slot("Loot", format!("{:.0}", loot))
            .city(ci)
            .civ(by)
            .civ2(old)
            .at(x, y)
            .cause(format!("{} fell after a siege", self.city_name(ci)))
            .cat("military")
            .tone(Tone::Alarm)
            .thread(thread)
            .card();
        let s = if self.cities[ci as usize].backlog > 40.0 { s.cause("neglected walls (maintenance backlog)") } else { s };
        self.seeds.push(s);
        self.transfer_city(ci, by);
        if self.civs[by as usize].leader.defining_act.is_none() {
            self.civs[by as usize].leader.defining_act = Some(format!("conquered {}", self.city_name(ci)));
        }
    }

    pub fn update_espionage(&mut self) {
        if self.year % 2 != 0 {
            return;
        }
        let n = self.civs.len();
        for a in 0..n {
            if !self.civs[a].alive || self.civs[a].minor || self.civs[a].era < 3 {
                continue;
            }
            let greed = self.civs[a].leader.traits[T_GREED];
            self.civs[a].spy_progress += 0.5 + greed;
            if self.civs[a].spy_progress < 12.0 {
                continue;
            }
            self.civs[a].spy_progress = 0.0;
            // target: richest city of a met non-ally
            let mut best: Option<(u32, f32)> = None;
            for c in self.cities.iter().filter(|c| c.alive && c.civ as usize != a) {
                let r = &self.rel[a * MAX_CIVS + c.civ as usize];
                if !r.met || r.alliance {
                    continue;
                }
                let v = c.wealth - r.score * 3.0;
                if best.map(|b| v > b.1).unwrap_or(true) {
                    best = Some((c.id, v));
                }
            }
            let Some((t, _)) = best else { continue };
            let tciv = self.cities[t as usize].civ;
            let wealth = self.cities[t as usize].wealth;
            // tech theft or treasury siphon
            let stolen = (0..TECHS.len()).find(|&k| self.civs[tciv as usize].known_tech[k] && !self.civs[a].known_tech[k]);
            if let Some(k) = stolen {
                self.civs[a].eureka[k] = true;
            } else {
                let amt = wealth * 0.05;
                self.civs[tciv as usize].treasury -= amt;
                self.civs[a].treasury += amt;
            }
            self.cities[t as usize].suspected_spy = Some(a as u16);
            if self.rng.gen_bool(0.35) {
                self.ledger(tciv, a as u16, Reason::Espionage, -15.0, Some(40), "spy ring uncovered");
                let c = &self.cities[t as usize];
                let s = StorySeed::new("spy_caught", Scope::National)
                    .slot("City", c.name.clone())
                    .slot("Civ", self.civ_name(a as u16))
                    .slot("Victim", self.civ_name(tciv))
                    .slot("Band", c.band())
                    .slot("Stole", if stolen.is_some() { TECHS[stolen.unwrap()].name.to_string() } else { "treasury gold".into() })
                    .city(t)
                    .civ(tciv)
                    .civ2(a as u16)
                    .at(c.x, c.y)
                    .cause(format!("a {} city is a tempting target", c.band().to_lowercase()))
                    .cat("military")
                    .card();
                self.seeds.push(s);
            }
        }
    }
}
