//! Game modes, scenarios, challenge planets, objectives and scoring (design doc section 11).

use super::news::*;
use super::planet::WorldParams;
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scn {
    FirstRivers,
    IceAtTheDoor,
    AshenCentury,
    TwoContinents,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Challenge {
    DyingSun,
    ExtremeTilt,
    Supercontinent,
    ScheduledImpact,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Sandbox,
    Chronicle,
    Scenario(Scn),
    Challenge(Challenge),
}

impl Mode {
    pub fn name(&self) -> String {
        match self {
            Mode::Sandbox => "Sandbox Stewardship".into(),
            Mode::Chronicle => "Chronicle Mode (hands-off)".into(),
            Mode::Scenario(s) => format!("Scenario: {}", scn_name(*s)),
            Mode::Challenge(c) => format!("Challenge: {}", ch_name(*c)),
        }
    }
}

pub fn scn_name(s: Scn) -> &'static str {
    match s {
        Scn::FirstRivers => "First Rivers",
        Scn::IceAtTheDoor => "Ice at the Door",
        Scn::AshenCentury => "Ashen Century",
        Scn::TwoContinents => "Two Continents, One Sky",
    }
}

pub fn scn_brief(s: Scn) -> &'static str {
    match s {
        Scn::FirstRivers => "A fertile continent is slowly drying. Objective: at least four civilizations reach the Classical era before Year 300, when the great rivers fail. Recommended: bring rain to breadbaskets, inspire eurekas.",
        Scn::IceAtTheDoor => "The sun dims on schedule and an ice age advances, peaking near Year 250. Objective: at Year 400, keep 60% of the Year-120 population and at least three civilizations. Warming is possible but expensive.",
        Scn::AshenCentury => "Volcanism runs four times normal from Year 50 to 150. Objective: no civilization extinctions before Year 200. Watch the hazard map; protect cities near volcanoes.",
        Scn::TwoContinents => "Two isolated continents develop apart. Objective: achieve contact between them, then avoid any war across the sea for 50 years after first contact. Must make contact by Year 600.",
    }
}

pub fn ch_name(c: Challenge) -> &'static str {
    match c {
        Challenge::DyingSun => "Dying Sun",
        Challenge::ExtremeTilt => "Extreme Tilt",
        Challenge::Supercontinent => "Single Supercontinent",
        Challenge::ScheduledImpact => "Scheduled Impact",
    }
}

#[derive(Clone, Debug)]
pub struct ScenarioState {
    pub name: String,
    pub objectives: Vec<(String, Option<bool>)>,
    pub baseline_pop: f32,
    pub contact_year: Option<i32>,
    pub majors_at_start: usize,
    pub score: Option<f32>,
}

pub fn adjust_params(mode: &Mode, p: &mut WorldParams) {
    match mode {
        Mode::Scenario(Scn::FirstRivers) => {
            p.land_frac = 0.32;
            p.tilt = 20.0;
        }
        Mode::Scenario(Scn::IceAtTheDoor) => {
            p.tilt = 28.0;
        }
        Mode::Scenario(Scn::AshenCentury) => {
            p.tectonics = 0.9;
        }
        Mode::Scenario(Scn::TwoContinents) => {
            p.land_frac = 0.36;
        }
        Mode::Challenge(Challenge::ExtremeTilt) => p.tilt = 40.0,
        Mode::Challenge(Challenge::Supercontinent) => {
            p.supercontinent = true;
            p.land_frac = 0.3;
        }
        _ => {}
    }
}

pub fn setup(sim: &mut Sim) {
    let st = |name: &str, objs: &[&str]| ScenarioState {
        name: name.into(),
        objectives: objs.iter().map(|o| (o.to_string(), None)).collect(),
        baseline_pop: 0.0,
        contact_year: None,
        majors_at_start: 0,
        score: None,
    };
    match sim.mode {
        Mode::Scenario(Scn::FirstRivers) => sim.scenario = Some(st("First Rivers", &["Four civilizations reach the Classical era before Year 300"])),
        Mode::Scenario(Scn::IceAtTheDoor) => sim.scenario = Some(st("Ice at the Door", &["Hold 60% of Year-120 population at Year 400", "At least three civilizations alive at Year 400"])),
        Mode::Scenario(Scn::AshenCentury) => sim.scenario = Some(st("Ashen Century", &["No civilization extinctions before Year 200"])),
        Mode::Scenario(Scn::TwoContinents) => {
            // carve two ocean belts so the land splits into two isolated halves
            let w = sim.planet.w as i32;
            for y in 0..sim.planet.h as i32 {
                for cx in [w / 4, 3 * w / 4] {
                    for dx in -3..=3 {
                        let i = sim.planet.idx(cx + dx, y).unwrap();
                        sim.planet.tiles[i].elev = sim.planet.tiles[i].elev.min(-0.4);
                    }
                }
            }
            sim.planet.compute_continents();
            sim.planet.update_climate();
            sim.planet.compute_rivers();
            sim.planet.update_biomes(true);
            sim.tribes.retain(|t| {
                let i = sim.planet.idx(t.x, t.y).unwrap();
                sim.planet.tiles[i].elev > sim.planet.sea_level
            });
            sim.scenario = Some(st("Two Continents, One Sky", &["Make contact between the two continents by Year 600", "No war across the sea for 50 years after contact"]));
        }
        Mode::Challenge(c) => {
            sim.scenario = Some(st(ch_name(c), &["Keep at least one civilization alive to Year 500"]));
        }
        _ => {}
    }
}

pub fn physics(sim: &mut Sim) {
    let y = sim.year;
    match sim.mode {
        Mode::Scenario(Scn::FirstRivers) => {
            // the great rivers slowly fail across the most populated continent
            if y % 5 == 0 && y < 320 {
                let cont = sim.civs.iter().filter(|c| c.alive).filter_map(|c| c.capital).map(|c| {
                    let city = &sim.cities[c as usize];
                    sim.planet.tiles[sim.planet.idx(city.x, city.y).unwrap()].continent
                }).next();
                if let Some(cont) = cont {
                    let dry = -0.02 * (y as f32 / 300.0);
                    for t in sim.planet.tiles.iter_mut() {
                        if t.continent == cont {
                            t.rain_nudge += dry;
                        }
                    }
                }
            }
        }
        Mode::Scenario(Scn::IceAtTheDoor) => {
            let target = if y < 250 { -1.3 * (y as f32 / 250.0) } else { -1.3 + 1.3 * ((y - 250) as f32 / 200.0).min(1.0) };
            sim.planet.solar = target;
        }
        Mode::Challenge(Challenge::DyingSun) => {
            sim.planet.solar -= 0.0025;
        }
        Mode::Challenge(Challenge::ScheduledImpact) => {
            if y == 250 {
                sim.meteor(None, false);
                sim.meteor(None, false);
            } else if y == 240 {
                let s = StorySeed::new("impact_warning", Scope::Global).cause("astronomers tracked an approaching body").cat("cosmic").chronicle();
                sim.seeds.push(s);
            }
        }
        _ => {}
    }
}

pub fn volcanism(sim: &Sim) -> f32 {
    match sim.mode {
        Mode::Scenario(Scn::AshenCentury) if (50..150).contains(&sim.year) => 4.0,
        _ => 1.0,
    }
}

pub fn gp_scale(_sim: &Sim) -> f32 {
    1.0
}

fn finish(sim: &mut Sim, success: bool, text: String) {
    let Some(st) = sim.scenario.as_mut() else { return };
    if st.score.is_some() {
        return;
    }
    let alive = sim.civs.iter().filter(|c| c.alive && !c.minor).count() as f32;
    let obj = st.objectives.iter().filter(|o| o.1 == Some(true)).count() as f32 * 300.0;
    let score = obj + alive * 40.0 + sim.planet.biodiv_index * 300.0 + sim.stewardship * 3.0 + (sim.news.items.iter().filter(|n| n.placement == Placement::Chronicle).count() as f32).min(60.0) * 3.0;
    st.score = Some(score);
    let name = st.name.clone();
    sim.ended = Some(format!("{} — {} {} Final score {:.0}.", name, if success { "Objectives complete." } else { "Objective failed." }, text, score));
    let s = StorySeed::new("scenario_end", Scope::Global)
        .slot("Name", name)
        .slot("Result", if success { "succeeded" } else { "failed" })
        .slot("Score", format!("{:.0}", score))
        .cause(text)
        .cat("stewardship")
        .alert(AlertClass::Existential)
        .chronicle();
    sim.seeds.push(s);
}

pub fn check(sim: &mut Sim) {
    let year = sim.year;
    // planetary terminus applies to every mode
    if sim.ended.is_none() && year > 150 && sim.civs.iter().all(|c| !c.alive) && sim.tribes.iter().all(|t| !t.alive) {
        sim.ended = Some("All sapient life has vanished. The planet turns on, unobserved.".into());
        let s = StorySeed::new("terminus", Scope::Global).cause("the extinction of every people").cat("stewardship").alert(AlertClass::Existential).chronicle();
        sim.seeds.push(s);
        return;
    }
    let Some(mut st) = sim.scenario.clone() else { return };
    if st.score.is_some() {
        return;
    }
    let majors = sim.civs.iter().filter(|c| c.alive && !c.minor).count();
    match sim.mode {
        Mode::Scenario(Scn::FirstRivers) => {
            let classical = sim.civs.iter().filter(|c| c.alive && !c.minor && c.era >= 2).count();
            st.objectives[0].0 = format!("Four civilizations reach the Classical era before Year 300 ({}/4)", classical);
            sim.scenario = Some(st.clone());
            if classical >= 4 {
                sim.scenario.as_mut().unwrap().objectives[0].1 = Some(true);
                finish(sim, true, format!("{} peoples reached the Classical era by Year {}.", classical, year));
            } else if year >= 300 {
                sim.scenario.as_mut().unwrap().objectives[0].1 = Some(false);
                finish(sim, false, "The rivers failed first.".into());
            }
        }
        Mode::Scenario(Scn::IceAtTheDoor) => {
            let pop: f32 = sim.cities.iter().filter(|c| c.alive).map(|c| c.pop).sum();
            if year == 120 {
                st.baseline_pop = pop.max(1.0);
            }
            if st.baseline_pop > 0.0 {
                st.objectives[0].0 = format!("Hold 60% of Year-120 population at Year 400 (now {:.0}%)", pop / st.baseline_pop * 100.0);
            }
            st.objectives[1].0 = format!("At least three civilizations alive at Year 400 ({} alive)", majors);
            sim.scenario = Some(st.clone());
            if year >= 400 {
                let a = st.baseline_pop > 0.0 && pop >= st.baseline_pop * 0.6;
                let b = majors >= 3;
                let s = sim.scenario.as_mut().unwrap();
                s.objectives[0].1 = Some(a);
                s.objectives[1].1 = Some(b);
                finish(sim, a && b, "The glacial maximum has passed.".into());
            }
        }
        Mode::Scenario(Scn::AshenCentury) => {
            if year == 50 {
                st.majors_at_start = sim.extinct_civs;
            }
            sim.scenario = Some(st.clone());
            if year > 50 && sim.extinct_civs > st.majors_at_start {
                sim.scenario.as_mut().unwrap().objectives[0].1 = Some(false);
                finish(sim, false, "A people was lost under the ash.".into());
            } else if year >= 200 {
                sim.scenario.as_mut().unwrap().objectives[0].1 = Some(true);
                finish(sim, true, "Every people survived the ashen century.".into());
            }
        }
        Mode::Scenario(Scn::TwoContinents) => {
            if st.contact_year.is_none() && sim.contact_made {
                st.contact_year = Some(year);
                st.objectives[0].1 = Some(true);
            }
            sim.scenario = Some(st.clone());
            if let Some(cy) = st.contact_year {
                // any war between civs on different continents?
                let mut cross_war = false;
                for a in 0..sim.civs.len() {
                    for b in 0..sim.civs.len() {
                        if a != b && sim.rel[a * MAX_CIVS + b].at_war {
                            let ca = sim.civs[a].capital.map(|c| sim.planet.tiles[sim.planet.idx(sim.cities[c as usize].x, sim.cities[c as usize].y).unwrap()].continent);
                            let cb = sim.civs[b].capital.map(|c| sim.planet.tiles[sim.planet.idx(sim.cities[c as usize].x, sim.cities[c as usize].y).unwrap()].continent);
                            if ca.is_some() && cb.is_some() && ca != cb {
                                cross_war = true;
                            }
                        }
                    }
                }
                if cross_war && year - cy < 50 {
                    sim.scenario.as_mut().unwrap().objectives[1].1 = Some(false);
                    finish(sim, false, "War crossed the sea too soon.".into());
                } else if year - cy >= 50 {
                    sim.scenario.as_mut().unwrap().objectives[1].1 = Some(true);
                    finish(sim, true, "Two worlds met in peace.".into());
                }
            } else if year >= 600 {
                sim.scenario.as_mut().unwrap().objectives[0].1 = Some(false);
                finish(sim, false, "The seas were never crossed.".into());
            }
        }
        Mode::Challenge(_) => {
            if majors == 0 && year > 100 {
                sim.scenario.as_mut().unwrap().objectives[0].1 = Some(false);
                finish(sim, false, "No civilization endured.".into());
            } else if year >= 500 {
                sim.scenario.as_mut().unwrap().objectives[0].1 = Some(true);
                finish(sim, true, "Civilization endured the harsh planet.".into());
            }
        }
        _ => {}
    }
}
