//! Panels and windows: top bar, layer rail and powers, contextual inspector, ticker, news cards,
//! chronicle, diplomacy matrix, planet graphs, decadal reviews, "Why did this happen?", objectives.

use super::map::*;
use super::*;
use crate::sim::city::*;
use crate::sim::civ::*;
use crate::sim::events::EVENTS;
use crate::sim::news::*;
use crate::sim::powers::ALL_POWERS;
use crate::sim::{MAX_CIVS, year_str};

fn civ_label(ui: &mut Ui, sim: &Sim, c: u16) -> egui::Response {
    let civ = &sim.civs[c as usize];
    let txt = RichText::new(format!("■ {}", civ.name)).color(c32(civ.color));
    ui.add(egui::Label::new(txt).sense(Sense::click())).on_hover_text("Click to inspect")
}

fn bar(ui: &mut Ui, frac: f32, col: Color32, text: String) {
    ui.add(egui::ProgressBar::new(frac.clamp(0.0, 1.0)).fill(col).text(text).desired_height(14.0));
}

pub fn draw_game(root: &mut Ui, ctx: &egui::Context, sim: &mut Sim, v: &mut View) -> bool {
    let mut back = false;
    // keyboard shortcuts
    if !ctx.egui_wants_keyboard_input() {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                if v.speed == 0 {
                    v.speed = v.prev_speed.max(1);
                } else {
                    v.prev_speed = v.speed;
                    v.speed = 0;
                }
            }
            for (k, key) in [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4].iter().enumerate() {
                if i.key_pressed(*key) {
                    v.speed = k + 1;
                }
            }
            if i.key_pressed(egui::Key::Escape) {
                v.tool = None;
            }
            if i.key_pressed(egui::Key::N) {
                v.win_cards = !v.win_cards;
            }
            if i.key_pressed(egui::Key::C) {
                v.win_chronicle = !v.win_chronicle;
            }
        });
    }
    top_bar(root, sim, v, &mut back);
    bottom_ticker(root, sim, v);
    left_rail(root, sim, v);
    right_panel(root, sim, v);
    egui::CentralPanel::default().frame(egui::Frame::NONE).show(root, |ui| {
        v.map_rect = ui.available_rect_before_wrap();
        if let Some((x, y)) = v.center_request.take() {
            center_on(v, sim, x, y, v.map_rect.width(), v.map_rect.height());
        }
        draw_map(ui, sim, v);
    });
    windows(ctx, sim, v, &mut back);
    // toasts
    if !v.toasts.is_empty() {
        egui::Area::new("toasts".into()).anchor(Align2::CENTER_TOP, Vec2::new(0.0, 60.0)).show(ctx, |ui| {
            for (t, _) in &v.toasts {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(t).size(15.0));
                });
            }
        });
    }
    back
}

fn top_bar(root: &mut Ui, sim: &mut Sim, v: &mut View, back: &mut bool) {
    egui::Panel::top("top").show(root, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("LIVING WORLDS").strong().color(Color32::from_rgb(140, 210, 160)));
            ui.separator();
            let max_era = sim.alive_civs().map(|c| c.era).max().unwrap_or(0);
            ui.label(RichText::new(year_str(sim.year)).size(18.0).strong());
            ui.label(RichText::new(format!("({})", ERA_NAMES[max_era])).weak());
            ui.separator();
            for (k, (_, lbl)) in SPEEDS.iter().enumerate() {
                if ui.selectable_label(v.speed == k, *lbl).on_hover_text("Space pauses; 1–4 set speed").clicked() {
                    v.speed = k;
                }
            }
            if ui.button("+1 yr").on_hover_text("Advance a single year").clicked() {
                sim.tick();
                v.tex_dirty = true;
            }
            ui.separator();
            ui.label(RichText::new(format!("🌿 {:.0} GP", sim.gp)).color(Color32::from_rgb(150, 230, 140)).strong())
                .on_hover_text("Gaia Points: earned faster on a biodiverse, stable planet and for restraint during crises.");
            ui.label(format!("Stewardship {:.0}", sim.stewardship)).on_hover_text("Ecological health 40%, civilization survival 35%, intervention proportionality 25%.");
            ui.label(format!("{:.1}°C", sim.planet.global_temp)).on_hover_text("Global mean temperature");
            ui.label(format!("Biodiv {:.0}%", sim.planet.biodiv_index * 100.0));
            ui.separator();
            let unread = sim.news.unread_cards.len();
            if ui.selectable_label(v.win_cards, format!("📰 Cards ({})", unread)).on_hover_text("News cards (N)").clicked() {
                v.win_cards = !v.win_cards;
            }
            if ui.selectable_label(v.win_chronicle, "📜 Chronicle").on_hover_text("C").clicked() {
                v.win_chronicle = !v.win_chronicle;
            }
            if ui.selectable_label(v.win_civs, "👑 Civilizations").clicked() {
                v.win_civs = !v.win_civs;
            }
            if ui.selectable_label(v.win_diplo, "⚖ Diplomacy").clicked() {
                v.win_diplo = !v.win_diplo;
            }
            if ui.selectable_label(v.win_planet, "🌍 Planet").clicked() {
                v.win_planet = !v.win_planet;
            }
            if ui.selectable_label(v.win_reviews, "📅 Reviews").clicked() {
                v.win_reviews = !v.win_reviews;
            }
            if sim.scenario.is_some() && ui.selectable_label(v.win_objectives, "🎯 Objectives").clicked() {
                v.win_objectives = !v.win_objectives;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Menu").clicked() {
                    *back = true;
                }
                if ui.button("?").clicked() {
                    v.win_help = !v.win_help;
                }
                if ui.button("⚙").clicked() {
                    v.win_settings = !v.win_settings;
                }
                ui.label(RichText::new(sim.mode.name()).weak());
            });
        });
    });
}

fn bottom_ticker(root: &mut Ui, sim: &mut Sim, v: &mut View) {
    egui::Panel::bottom("ticker").show(root, |ui| {
        ui.horizontal(|ui| {
            let skin = era_skin(sim.alive_civs().map(|c| c.era).max().unwrap_or(0));
            ui.label(RichText::new(format!("{}:", skin)).strong().color(Color32::from_rgb(240, 200, 100)));
            egui::ScrollArea::horizontal().id_salt("tick").show(ui, |ui| {
                let items: Vec<usize> = sim.news.items.iter().rev().filter(|n| n.placement >= Placement::Ticker && n.kind != "decadal").take(8).map(|n| n.id).collect();
                for id in items {
                    let n = &sim.news.items[id];
                    let col = match n.scope {
                        Scope::Global => Color32::from_rgb(255, 230, 150),
                        Scope::Regional => Color32::from_rgb(200, 220, 255),
                        Scope::National => Color32::from_rgb(230, 230, 230),
                        Scope::Local => Color32::from_rgb(170, 170, 170),
                    };
                    if ui.add(egui::Label::new(RichText::new(format!("[{}] {}", n.year, n.headline)).color(col)).sense(Sense::click())).on_hover_text("Click: why did this happen?").clicked() {
                        v.win_why = Some(id);
                    }
                    ui.label(RichText::new("•").weak());
                }
            });
        });
    });
}

fn left_rail(root: &mut Ui, sim: &mut Sim, v: &mut View) {
    egui::Panel::left("rail").resizable(false).exact_size(190.0).show(root, |ui| {
        egui::ScrollArea::vertical().id_salt("rail_s").show(ui, |ui| {
            ui.heading("Map layers");
            for (l, name, tip) in LAYERS {
                if ui.selectable_label(v.layer == l, name).on_hover_text(tip).clicked() {
                    v.layer = l;
                }
            }
            ui.add_space(2.0);
            ui.checkbox(&mut v.show_borders, "Borders");
            ui.checkbox(&mut v.show_routes, "Trade routes");
            ui.checkbox(&mut v.show_armies, "Armies");
            ui.checkbox(&mut v.show_labels, "City names");
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Pairs:").weak());
                if ui.small_button("Wealth+Trade").clicked() {
                    v.layer = Layer::Wealth;
                    v.show_routes = true;
                }
                if ui.small_button("Hazards+Political").clicked() {
                    v.layer = Layer::Geology;
                    v.show_borders = true;
                }
            });
            ui.separator();
            ui.heading("Powers");
            if matches!(sim.mode, Mode::Chronicle) {
                ui.label(RichText::new("Chronicle mode: interventions disabled.").weak());
            } else {
                let mut cat = "";
                for p in ALL_POWERS {
                    if p.category() != cat {
                        cat = p.category();
                        ui.label(RichText::new(cat).small().strong().color(Color32::from_rgb(170, 200, 170)));
                    }
                    let cost = sim.power_cost(p);
                    let allowed = sim.power_allowed(p);
                    let afford = sim.gp >= cost;
                    let txt = RichText::new(format!("{}  ({:.0})", p.name(), cost)).color(if !allowed { Color32::DARK_GRAY } else if afford { Color32::WHITE } else { Color32::GRAY });
                    let r = ui.add_enabled(allowed, egui::Button::selectable(v.tool == Some(p), txt));
                    if r.on_hover_text(format!("{}{}", p.desc(), if p.needs_city() { "\nClick a city." } else { "\nClick a tile." })).clicked() {
                        v.tool = if v.tool == Some(p) { None } else { Some(p) };
                    }
                }
                if v.tool.is_some() && ui.button("Cancel tool (Esc)").clicked() {
                    v.tool = None;
                }
            }
            ui.separator();
            let open: Vec<_> = sim.windows.iter().filter(|w| !w.resolved).cloned().collect();
            ui.heading(format!("Intervention windows ({})", open.len()));
            if open.is_empty() {
                ui.label(RichText::new("No crises awaiting a decision.").weak());
            }
            for w in open {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    if ui.add(egui::Label::new(RichText::new(&w.title).strong()).sense(Sense::click())).clicked() {
                        let c = &sim.cities[w.city as usize];
                        v.center_request = Some((c.x, c.y));
                        v.sel_city = Some(w.city);
                    }
                    ui.label(RichText::new(format!("Closes {} · refrain to earn GP if the city holds", year_str(w.expires))).small().weak());
                    let afford = sim.gp >= w.cost;
                    if ui.add_enabled(afford && !matches!(sim.mode, Mode::Chronicle), egui::Button::new(format!("Mitigate ({:.0} GP)", w.cost))).clicked() {
                        match sim.mitigate_window(w.id) {
                            Ok(m) => v.toast(m),
                            Err(e) => v.toast(e),
                        }
                        v.tex_dirty = true;
                    }
                });
            }
        });
    });
}

fn right_panel(root: &mut Ui, sim: &mut Sim, v: &mut View) {
    egui::Panel::right("inspector").resizable(true).default_size(330.0).show(root, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut v.right_tab, 0, "City");
            ui.selectable_value(&mut v.right_tab, 1, "Civilization");
            ui.selectable_value(&mut v.right_tab, 2, "Tile");
        });
        ui.separator();
        egui::ScrollArea::vertical().id_salt("insp").show(ui, |ui| match v.right_tab {
            0 => city_panel(ui, sim, v),
            1 => civ_panel(ui, sim, v),
            _ => tile_panel(ui, sim, v),
        });
    });
}

fn tile_panel(ui: &mut Ui, sim: &mut Sim, v: &mut View) {
    let Some(i) = v.sel_tile else {
        ui.label("Click the map to inspect a tile.");
        return;
    };
    let p = &sim.planet;
    let t = &p.tiles[i];
    let (x, y) = p.xy(i);
    ui.heading(format!("{} ({}, {})", t.biome.name(), x, y));
    egui::Grid::new("tile").num_columns(2).show(ui, |ui| {
        let mut row = |a: &str, b: String| {
            ui.label(a);
            ui.label(b);
            ui.end_row();
        };
        row("Elevation", format!("{:+.2}", p.height(i)));
        row("Temperature", format!("{:.1}°C", t.temp));
        row("Rainfall", format!("{:.2}{}", t.rain, if t.rain_nudge.abs() > 0.02 { format!(" (nudged {:+.2})", t.rain_nudge) } else { String::new() }));
        row("Fertility", format!("{:.2} (soil {:.0}%)", t.fert, t.soil * 100.0));
        row("Forest", format!("{:.0}%", t.forest * 100.0));
        row("Biodiversity", format!("{:.0}%", t.biodiv * 100.0));
        if t.is_river() {
            row("River", format!("flow {:.0}", t.river));
        }
        if let Some(r) = t.resource {
            let known = r.kind.revealed_by().map(|k| sim.civs.iter().any(|c| c.known_tech[k])).unwrap_or(true);
            row("Resource", format!("{} ({:?}){}{}", r.kind.name(), r.kind.class(), if known { "" } else { " — hidden until revealed" }, if r.stock.is_finite() { format!(", stock {:.0}", r.stock) } else { String::new() }));
        }
        if t.fault {
            row("Fault line", "yes — earthquake risk".into());
        }
        if t.volcano > 0.0 {
            row("Volcano", format!("activity {:.2}", t.volcano));
        }
        if t.uplift_stress > 0.05 {
            row("Uplift stress", format!("{:.2}", t.uplift_stress));
        }
        if t.pollution > 0.02 {
            row("Pollution", format!("{:.0}%", t.pollution * 100.0));
        }
        if t.attune > 0.1 {
            row("Attunement", format!("{:.1}{}", t.attune, if t.attune > 4.0 { " — peoples here sense the Shaper" } else { "" }));
        }
        row("Improvement", format!("{:?}", t.improvement));
        if let Some(o) = t.owner {
            row("Owner", sim.civs[o as usize].name.clone());
        }
        if let Some(n) = &t.natural_wonder {
            row("Natural wonder", n.clone());
        }
        if let Some(n) = &t.ruin {
            row("Ruins", n.clone());
        }
        if t.battle {
            row("Battlefield", "armies fought here".into());
        }
    });
}

fn city_panel(ui: &mut Ui, sim: &mut Sim, v: &mut View) {
    let Some(ci) = v.sel_city else {
        ui.label("Click a city on the map.");
        return;
    };
    let c = sim.cities[ci as usize].clone();
    let civ = &sim.civs[c.civ as usize];
    let era = civ.era;
    ui.horizontal(|ui| {
        ui.heading(if c.is_capital { format!("★ {}", c.name) } else { c.name.clone() });
        if !c.alive {
            ui.label(RichText::new("(ruined)").color(Color32::RED));
        }
    });
    ui.horizontal(|ui| {
        if civ_label(ui, sim, c.civ).clicked() {
            v.sel_civ = Some(c.civ);
            v.right_tab = 1;
        }
        ui.label(format!("· the {} · founded {}", c.spec.name(), year_str(c.founded)));
    });
    ui.horizontal(|ui| {
        let watching = sim.news.watch_cities.contains(&ci);
        if ui.selectable_label(watching, "👁 Watch").on_hover_text("Watched subjects break through mutes and can auto-pause").clicked() {
            if watching {
                sim.news.watch_cities.retain(|&x| x != ci);
            } else {
                sim.news.watch_cities.push(ci);
            }
        }
        if ui.button("Center").clicked() {
            v.center_request = Some((c.x, c.y));
        }
    });
    ui.separator();
    // wealth
    let bi = band_index(c.wealth);
    ui.label(RichText::new(format!("Wealth {:.0} — {}", c.wealth, c.band())).size(16.0).strong().color(c32(BAND_COLORS[bi])));
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::from_gray(30));
    let cols = [Color32::from_rgb(230, 200, 80), Color32::from_rgb(90, 170, 240), Color32::from_rgb(200, 120, 220), Color32::from_rgb(160, 160, 160), Color32::from_rgb(120, 200, 110), Color32::from_rgb(240, 130, 90)];
    let mut x0 = rect.min.x;
    for k in 0..6 {
        let wpx = c.wealth_parts[k] / 1000.0 * rect.width();
        painter.rect_filled(Rect::from_min_size(Pos2::new(x0, rect.min.y), Vec2::new(wpx, rect.height())), 0.0, cols[k]);
        x0 += wpx;
    }
    for (k, (name, wgt)) in WEALTH_WEIGHTS.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("■").color(cols[k]));
            ui.label(RichText::new(format!("{} ({:.0}%): {:.0} / {:.0}", name, wgt * 100.0, c.wealth_parts[k], wgt * 1000.0)).small());
        });
    }
    sparkline(ui, &c.wealth_hist, 1000.0, Color32::from_rgb(230, 200, 80), "wealth history");
    ui.separator();
    egui::Grid::new("city").num_columns(2).show(ui, |ui| {
        let mut row = |a: &str, b: String| {
            ui.label(a);
            ui.label(b);
            ui.end_row();
        };
        row("Population", format!("{} ({:.1} pts, housing {:.0})", fmt_pop(c.display_pop(era)), c.pop, c.housing()));
        let y = c.yields;
        row("Yields", format!("Food {:.1} · Prod {:.1} · Gold {:.1}", y.food, y.prod, y.gold));
        row("", format!("Sci {:.1} · Cult {:.1} · Faith {:.1}", y.science, y.culture, y.faith));
        row("Food stores", format!("{:.0}", c.food_store));
        row("Health", format!("{:.0}", c.health));
        row("Stability", format!("{:.0}", c.stability));
        row("Maintenance backlog", format!("{:.0}{}", c.backlog, if c.backlog > 30.0 { " ⚠" } else { "" }));
        row("Governor", format!("{} (competence {:.0}%, corruption {:.0}%)", c.governor.name, c.governor.competence * 100.0, c.governor.corruption * 100.0));
        row("Garrison", format!("{:.0}  defense {:.0}", c.garrison, c.defense(civ.military_tier())));
        row("Religion", c.religion.map(|r| format!("{}{}", sim.religions[r].name, if sim.religions[r].holy_city == ci { " (holy city)" } else { "" })).unwrap_or("none".into()));
        row("Trade income", format!("{:.1}", c.trade_income));
        if !c.resources.is_empty() {
            row("Resources", c.resources.iter().map(|r| r.name()).collect::<Vec<_>>().join(", "));
        }
        let build = match &c.build {
            Some(Build::Building(k)) => BUILDINGS[*k].name.to_string(),
            Some(Build::Wonder(k)) => format!("Wonder: {}", WONDERS[*k].name),
            Some(Build::Army) => "Army".into(),
            Some(Build::Settler) => "Settlers".into(),
            Some(Build::Project(p)) => format!("Project: {}", p),
            None => "—".into(),
        };
        row("Building", format!("{} ({:.0} done)", build, c.build_progress));
        if c.debt > 0.0 {
            row("Debt", format!("{:.0}{}", c.debt, c.debt_lender.map(|l| format!(" to the {}", sim.civs[l as usize].name)).unwrap_or_default()));
        }
        if c.protected_until > sim.year {
            row("Protected", format!("until {}", year_str(c.protected_until)));
        }
        if let Some(s) = c.suspected_spy {
            row("Suspected actor", format!("evidence points to the {}", sim.civs[s as usize].name));
        }
        let ch = &c.chains;
        let mut chains = Vec::new();
        if ch.drought > 0 { chains.push(format!("Drought stage {}", ch.drought)); }
        if ch.gold_rush > 0 { chains.push(format!("Gold rush stage {}", ch.gold_rush)); }
        if ch.plague > 0 { chains.push(format!("Plague stage {}", ch.plague)); }
        if ch.relic > 0 { chains.push(format!("Relic stage {}", ch.relic)); }
        if ch.river_shift > 0 { chains.push("Stranded port".into()); }
        if ch.accident > 0 { chains.push("Industrial accident".into()); }
        if ch.succession > 0 { chains.push(format!("Succession crisis stage {}", ch.succession)); }
        if !chains.is_empty() {
            row("Active chains", chains.join(", "));
        }
    });
    let built: Vec<&str> = BUILDINGS.iter().enumerate().filter(|(k, _)| c.buildings[*k]).map(|(_, b)| b.name).collect();
    ui.label(RichText::new("Buildings").strong());
    ui.label(if built.is_empty() { "none".to_string() } else { built.join(", ") });
    if !c.wonders.is_empty() {
        ui.label(RichText::new(format!("Wonders: {}", c.wonders.iter().map(|&w| WONDERS[w].name).collect::<Vec<_>>().join(", "))).color(Color32::from_rgb(255, 220, 120)));
    }
    sparkline(ui, &c.pop_history, 0.0, Color32::from_rgb(120, 200, 120), "population history");
    ui.separator();
    ui.label(RichText::new("City gazette").strong());
    for &id in c.gazette.iter().rev().take(25) {
        let n = &sim.news.items[id];
        if ui.add(egui::Label::new(RichText::new(format!("[{}] {}", n.year, n.headline)).small()).sense(Sense::click())).clicked() {
            v.win_why = Some(id);
        }
    }
}

fn sparkline(ui: &mut Ui, data: &[f32], max: f32, col: Color32, label: &str) {
    if data.len() < 2 {
        return;
    }
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::from_gray(22));
    let mx = if max > 0.0 { max } else { data.iter().cloned().fold(0.01, f32::max) };
    let n = data.len();
    let pts: Vec<Pos2> = data.iter().enumerate().map(|(k, &d)| Pos2::new(rect.min.x + k as f32 / (n - 1) as f32 * rect.width(), rect.max.y - (d / mx).clamp(0.0, 1.0) * rect.height())).collect();
    painter.add(egui::Shape::line(pts, Stroke::new(1.5, col)));
    resp.on_hover_text(format!("{} (last {} years)", label, n));
}

fn civ_panel(ui: &mut Ui, sim: &mut Sim, v: &mut View) {
    let Some(cid) = v.sel_civ else {
        ui.label("Select a city or civilization.");
        return;
    };
    let civ = sim.civs[cid as usize].clone();
    ui.heading(RichText::new(&civ.name).color(c32(civ.color)));
    ui.label(format!(
        "{} · {} era · {}{}",
        if civ.minor { format!("{} city-state", civ.minor_kind) } else { "civilization".into() },
        ERA_NAMES[civ.era],
        civ.gov().name,
        if civ.alive { String::new() } else { format!(" · fell {}", year_str(civ.fell.unwrap_or(0))) }
    ));
    ui.horizontal(|ui| {
        let watching = sim.news.watch_civs.contains(&cid);
        if ui.selectable_label(watching, "👁 Watch").clicked() {
            if watching {
                sim.news.watch_civs.retain(|&x| x != cid);
            } else {
                sim.news.watch_civs.push(cid);
            }
        }
        if let Some(cap) = civ.capital {
            if ui.button("Capital").clicked() {
                v.sel_city = Some(cap);
                let c = &sim.cities[cap as usize];
                v.center_request = Some((c.x, c.y));
            }
        }
    });
    ui.separator();
    ui.label(RichText::new(format!("{} {}, age {}", civ.leader.title, civ.leader.name, civ.leader.age)).strong());
    ui.label(RichText::new(format!("Reigning since {} · {}", year_str(civ.leader.reign_start), civ.leader.defining_act.clone().unwrap_or_else(|| "no defining act yet".into()))).small().weak());
    for (k, (name, lo, hi)) in TRAIT_NAMES.iter().enumerate() {
        let t = civ.leader.traits[k];
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{:<9}", lo)).small());
            bar(ui, t, Color32::from_rgb(100, 140, 200), format!("{}: {:.0}", name, t * 100.0));
            ui.label(RichText::new(*hi).small());
        });
    }
    ui.separator();
    egui::Grid::new("civ").num_columns(2).show(ui, |ui| {
        let mut row = |a: &str, b: String| {
            ui.label(a);
            ui.label(b);
            ui.end_row();
        };
        row("Unique ability", format!("{} — {}", civ.ability, civ.ability_desc));
        row("Visible agenda", format!("{}: {}", AGENDAS[civ.agenda_visible].0, AGENDAS[civ.agenda_visible].1));
        let revealed = sim.year - civ.founded > 80 || civ.agenda_revealed;
        row("Hidden agenda", if revealed { format!("{}: {}", AGENDAS[civ.agenda_hidden].0, AGENDAS[civ.agenda_hidden].1) } else { "not yet discovered".into() });
        row("Homeland", civ.homeland_biome.to_string());
        row("Strategy", civ.strategy.name().into());
        row("Researching", civ.research.map(|k| format!("{} ({:.0}){}", TECHS[k].name, civ.research_progress, if civ.eureka[k] { " ★eureka" } else { "" })).unwrap_or("—".into()));
        row("Civics", civ.civic_research.map(|k| CIVICS[k].name.to_string()).unwrap_or("—".into()));
        row("Religion", civ.religion.map(|r| sim.religions[r].name.clone()).unwrap_or("none".into()));
        row("Treasury", format!("{:.0} ({:+.1}/yr)", civ.treasury, civ.gold_income));
        let (w, rich, poor) = sim.civ_wealth(cid);
        row("Wealth", format!("{:.0} ({})", w, wealth_band(w)));
        if let (Some(r), Some(p)) = (rich, poor) {
            row("Richest / poorest", format!("{} ({:.0}) / {} ({:.0})", sim.cities[r as usize].name, sim.cities[r as usize].wealth, sim.cities[p as usize].name, sim.cities[p as usize].wealth));
        }
        row("Military", format!("{:.0} (tier {:.1})", civ.military, civ.military_tier()));
        row("War weariness", format!("{:.0}", civ.war_weariness));
        row("Score", format!("{:.0}", civ.score));
        if let Some(s) = civ.suzerain {
            row("Suzerain", sim.civs[s as usize].name.clone());
        }
        if let Some(p) = civ.parent {
            row("Successor of", sim.civs[p as usize].name.clone());
        }
    });
    let techs: Vec<&str> = TECHS.iter().enumerate().filter(|(k, _)| civ.known_tech[*k]).map(|(_, t)| t.name).collect();
    ui.collapsing(format!("Technologies ({})", techs.len()), |ui| {
        ui.label(techs.join(", "));
    });
    ui.collapsing(format!("Cities ({})", civ.cities.len()), |ui| {
        for &ci in &civ.cities {
            let c = &sim.cities[ci as usize];
            if ui.add(egui::Label::new(format!("{} — {} — {} ({:.0})", c.name, fmt_pop(c.display_pop(civ.era)), c.band(), c.wealth)).sense(Sense::click())).clicked() {
                v.sel_city = Some(ci);
                v.center_request = Some((c.x, c.y));
                v.right_tab = 0;
            }
        }
    });
    ui.collapsing("Relations", |ui| {
        for o in 0..sim.civs.len() as u16 {
            if o == cid || !sim.civs[o as usize].alive || !sim.r(cid, o).met {
                continue;
            }
            let r = sim.r(cid, o);
            let st = rel_state(r.score, r);
            ui.horizontal(|ui| {
                if civ_label(ui, sim, o).clicked() {
                    v.diplo_pair = Some((cid, o));
                    v.win_diplo = true;
                }
                ui.label(format!("{} ({:+.0})", st, r.score));
            });
        }
    });
    ui.collapsing("Score breakdown", |ui| {
        let names = ["Territory", "Cities", "Population", "Technology", "Culture", "Wealth", "Military", "Wonders"];
        for (k, n) in names.iter().enumerate() {
            ui.label(format!("{}: {:.0}", n, civ.score_parts[k]));
        }
    });
    ui.collapsing("Past leaders", |ui| {
        for l in civ.leaders_past.iter().rev() {
            ui.label(format!("{} ({}–{}){}", l.0, l.1, l.2, if l.3.is_empty() { String::new() } else { format!(" — {}", l.3) }));
        }
    });
    sparkline(ui, &civ.pop_history, 0.0, Color32::from_rgb(120, 200, 120), "population");
}

fn windows(ctx: &egui::Context, sim: &mut Sim, v: &mut View, back: &mut bool) {
    // ---- news cards
    let mut open = v.win_cards;
    egui::Window::new("📰 News cards").open(&mut open).default_size([460.0, 520.0]).show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Mark all read").clicked() {
                sim.news.unread_cards.clear();
            }
            ui.label(RichText::new(format!("{} unread", sim.news.unread_cards.len())).weak());
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            let ids: Vec<usize> = sim.news.unread_cards.iter().rev().copied().collect();
            for id in ids {
                let n = sim.news.items[id].clone();
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(format!("{} · {} · {} · {}", era_skin(n.era_skin), year_str(n.year), n.scope.name(), n.category)).small().weak());
                    ui.label(RichText::new(&n.headline).strong().size(15.0));
                    ui.label(&n.body);
                    ui.horizontal(|ui| {
                        if let Some((x, y)) = n.pos {
                            if ui.small_button("Show on map").clicked() {
                                v.center_request = Some((x, y));
                                if let Some(c) = n.city {
                                    v.sel_city = Some(c);
                                }
                            }
                        }
                        if ui.small_button("Why?").clicked() {
                            v.win_why = Some(id);
                        }
                        if let Some(t) = &n.thread {
                            let f = sim.news.followed_threads.contains(t);
                            if ui.small_button(if f { "Unfollow thread" } else { "Follow thread" }).on_hover_text(t.as_str()).clicked() {
                                if f {
                                    sim.news.followed_threads.retain(|x| x != t);
                                } else {
                                    sim.news.followed_threads.push(t.clone());
                                }
                            }
                        }
                        if ui.small_button("Dismiss").clicked() {
                            sim.news.unread_cards.retain(|&x| x != id);
                        }
                    });
                });
            }
        });
    });
    v.win_cards = open;
    // ---- why panel
    if let Some(id) = v.win_why {
        let mut open = true;
        let n = sim.news.items[id].clone();
        egui::Window::new("Why did this happen?").open(&mut open).default_size([420.0, 300.0]).show(ctx, |ui| {
            ui.label(RichText::new(&n.headline).strong().size(15.0));
            ui.label(RichText::new(format!("{} · {} · {} · {:?}", year_str(n.year), n.scope.name(), n.category, n.placement)).small().weak());
            ui.label(&n.body);
            ui.separator();
            ui.label(RichText::new("Causes cited by the story seed").strong());
            if n.causes.is_empty() {
                ui.label("None recorded — the cause is unknown or disputed.");
            }
            for c in &n.causes {
                ui.label(format!("• {}", c));
            }
            ui.label(RichText::new("Prerequisites and weight calculation").strong());
            for w in &n.why {
                ui.label(RichText::new(format!("• {}", w)).small());
            }
            ui.horizontal(|ui| {
                if let Some(c) = n.city {
                    if ui.button(format!("City: {}", sim.cities[c as usize].name)).clicked() {
                        v.sel_city = Some(c);
                        v.right_tab = 0;
                        let cc = &sim.cities[c as usize];
                        v.center_request = Some((cc.x, cc.y));
                    }
                }
                if let Some(c) = n.civ {
                    if ui.button(format!("Civ: {}", sim.civs[c as usize].name)).clicked() {
                        v.sel_civ = Some(c);
                        v.right_tab = 1;
                    }
                }
                if let Some(c) = n.civ2 {
                    if ui.button(format!("Civ: {}", sim.civs[c as usize].name)).clicked() {
                        v.sel_civ = Some(c);
                        v.right_tab = 1;
                    }
                }
                if let Some(t) = &n.thread {
                    let f = sim.news.followed_threads.contains(t);
                    if ui.button(if f { "Unfollow thread" } else { "Follow thread" }).clicked() {
                        if f {
                            sim.news.followed_threads.retain(|x| x != t);
                        } else {
                            sim.news.followed_threads.push(t.clone());
                        }
                    }
                }
            });
        });
        if !open {
            v.win_why = None;
        }
    }
    // ---- chronicle
    let mut open = v.win_chronicle;
    egui::Window::new("📜 Chronicle").open(&mut open).default_size([560.0, 560.0]).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Search");
            ui.add(egui::TextEdit::singleline(&mut v.chron_filter).desired_width(140.0));
            egui::ComboBox::from_id_salt("cat").selected_text(if v.chron_cat.is_empty() { "all categories" } else { v.chron_cat.as_str() }).show_ui(ui, |ui| {
                ui.selectable_value(&mut v.chron_cat, String::new(), "all categories");
                for c in ["political", "military", "economic", "ecological", "health", "religious", "cultural", "technological", "cosmic", "stewardship"] {
                    ui.selectable_value(&mut v.chron_cat, c.to_string(), c);
                }
            });
            if ui.button("Export").on_hover_text("Write the chronicle to a text file next to the game").clicked() {
                let name = format!("chronicle_seed{}_year{}.txt", sim.params.seed, sim.year);
                let path = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join(&name))).unwrap_or_else(|| name.clone().into());
                match std::fs::write(&path, sim.chronicle_text()) {
                    Ok(_) => v.toast(format!("Chronicle written to {}", path.display())),
                    Err(e) => v.toast(format!("Export failed: {}", e)),
                }
            }
        });
        ui.label(RichText::new(format!("The species: {} — {}. Lineages lost before the dawn: {}.", sim.species.name, sim.species.desc, sim.species.extinct_lineages.join(", "))).small().weak());
        ui.separator();
        let f = v.chron_filter.to_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            let ids: Vec<usize> = sim
                .news
                .items
                .iter()
                .rev()
                .filter(|n| n.placement == Placement::Chronicle || (n.placement == Placement::Card && n.scope >= Scope::National))
                .filter(|n| v.chron_cat.is_empty() || n.category == v.chron_cat)
                .filter(|n| f.is_empty() || n.headline.to_lowercase().contains(&f) || n.body.to_lowercase().contains(&f))
                .take(400)
                .map(|n| n.id)
                .collect();
            for id in ids {
                let n = &sim.news.items[id];
                let big = n.placement == Placement::Chronicle;
                if ui.add(egui::Label::new(RichText::new(format!("{} — {}", year_str(n.year), n.headline)).strong().color(if big { Color32::from_rgb(255, 225, 160) } else { Color32::LIGHT_GRAY })).sense(Sense::click())).clicked() {
                    v.win_why = Some(id);
                }
                if big {
                    ui.label(RichText::new(&n.body).small());
                }
            }
        });
    });
    v.win_chronicle = open;
    // ---- civilizations list
    let mut open = v.win_civs;
    egui::Window::new("👑 Civilizations").open(&mut open).default_size([560.0, 420.0]).show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("civs").striped(true).num_columns(8).show(ui, |ui| {
                for h in ["Name", "Leader", "Era", "Cities", "Pop", "Wealth", "Score", "Status"] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                let mut order: Vec<u16> = (0..sim.civs.len() as u16).collect();
                order.sort_by(|a, b| {
                    let (ca, cb) = (&sim.civs[*a as usize], &sim.civs[*b as usize]);
                    cb.alive.cmp(&ca.alive).then(ca.minor.cmp(&cb.minor)).then(cb.score.partial_cmp(&ca.score).unwrap())
                });
                for c in order {
                    let civ = &sim.civs[c as usize];
                    if civ_label(ui, sim, c).clicked() {
                        v.sel_civ = Some(c);
                        v.right_tab = 1;
                    }
                    ui.label(format!("{} {}", civ.leader.title, civ.leader.name));
                    ui.label(ERA_NAMES[civ.era]);
                    ui.label(civ.cities.len().to_string());
                    ui.label(format!("{:.0}", sim.civ_pop(c)));
                    let (w, _, _) = sim.civ_wealth(c);
                    ui.label(format!("{:.0}", w));
                    ui.label(format!("{:.0}", civ.score));
                    ui.label(if !civ.alive { "fallen".to_string() } else if civ.minor { format!("{} city-state", civ.minor_kind) } else { "alive".into() });
                    ui.end_row();
                }
            });
        });
    });
    v.win_civs = open;
    // ---- diplomacy matrix
    let mut open = v.win_diplo;
    egui::Window::new("⚖ Diplomacy matrix").open(&mut open).default_size([620.0, 520.0]).show(ctx, |ui| {
        let alive: Vec<u16> = (0..sim.civs.len() as u16).filter(|&c| sim.civs[c as usize].alive).collect();
        ui.label(RichText::new("Row's opinion of column. Click a cell to read the ledger.").small().weak());
        egui::ScrollArea::both().max_height(300.0).show(ui, |ui| {
            egui::Grid::new("diplo").spacing([2.0, 2.0]).show(ui, |ui| {
                ui.label("");
                for &c in &alive {
                    ui.label(RichText::new(sim.civs[c as usize].name.chars().take(5).collect::<String>()).color(c32(sim.civs[c as usize].color)).small());
                }
                ui.end_row();
                for &a in &alive {
                    ui.label(RichText::new(&sim.civs[a as usize].name).color(c32(sim.civs[a as usize].color)).small());
                    for &b in &alive {
                        if a == b {
                            ui.label("—");
                            continue;
                        }
                        let r = sim.r(a, b);
                        if !r.met {
                            ui.label(RichText::new("?").weak());
                            continue;
                        }
                        let st = rel_state(r.score, r);
                        let col = match st {
                            "At war" => Color32::from_rgb(220, 40, 40),
                            "Denounced" => Color32::from_rgb(170, 60, 60),
                            "Hostile" => Color32::from_rgb(210, 110, 60),
                            "Unfriendly" => Color32::from_rgb(200, 170, 80),
                            "Neutral" => Color32::GRAY,
                            "Friendly" => Color32::from_rgb(120, 200, 120),
                            "Allied" => Color32::from_rgb(80, 160, 255),
                            _ => Color32::from_rgb(60, 220, 160),
                        };
                        let abbrev: String = st.chars().take(3).collect();
                        if ui.add(egui::Button::new(RichText::new(abbrev).small().color(Color32::BLACK)).fill(col).min_size(Vec2::new(34.0, 16.0))).on_hover_text(format!("{} → {}: {} ({:+.0})", sim.civs[a as usize].name, sim.civs[b as usize].name, st, r.score)).clicked() {
                            v.diplo_pair = Some((a, b));
                        }
                    }
                    ui.end_row();
                }
            });
        });
        ui.separator();
        if let Some((a, b)) = v.diplo_pair {
            if (a as usize) < MAX_CIVS && (b as usize) < MAX_CIVS && (a as usize) < sim.civs.len() && (b as usize) < sim.civs.len() {
                let r = sim.r(a, b).clone();
                ui.label(RichText::new(format!("{} on {}: {} ({:+.0})", sim.civs[a as usize].name, sim.civs[b as usize].name, rel_state(r.score, &r), r.score)).strong());
                let mut treaties = Vec::new();
                if r.at_war { treaties.push(format!("AT WAR since {} — aim: {}", year_str(r.war_start), r.war_aim)); }
                if r.alliance { treaties.push("Alliance".into()); }
                if r.defensive_pact { treaties.push("Defensive pact".into()); }
                if r.trade_agreement { treaties.push("Trade agreement".into()); }
                if r.open_borders { treaties.push("Open borders".into()); }
                if r.research_agreement { treaties.push("Research agreement".into()); }
                if r.nonaggression_until > sim.year { treaties.push(format!("Non-aggression promise until {}", year_str(r.nonaggression_until))); }
                ui.label(if treaties.is_empty() { "No treaties.".to_string() } else { treaties.join(" · ") });
                egui::Grid::new("ledger").striped(true).num_columns(4).show(ui, |ui| {
                    for h in ["Reason", "Detail", "Value", "Expires"] {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    for e in &r.ledger {
                        ui.label(e.reason.text());
                        ui.label(&e.note);
                        ui.label(RichText::new(format!("{:+.0}", e.value)).color(if e.value < 0.0 { Color32::from_rgb(240, 120, 100) } else { Color32::from_rgb(130, 220, 130) }));
                        ui.label(e.expires.map(year_str).unwrap_or("—".into()));
                        ui.end_row();
                    }
                });
            }
        }
    });
    v.win_diplo = open;
    // ---- planet graphs
    let mut open = v.win_planet;
    egui::Window::new("🌍 Planet").open(&mut open).default_size([620.0, 480.0]).show(ctx, |ui| {
        let p = &sim.planet;
        ui.label(format!(
            "Mean temperature {:.1}°C · CO₂ ×{:.2} · O₂ {:.0}% · particulates {:.2} · ice {:.0}% · sea level {:+.2} · biodiversity {:.0}% · solar {:+.2} · tilt {:.0}°",
            p.global_temp, p.co2, p.o2 * 100.0, p.particulates, p.ice_frac * 100.0, p.sea_level * 10.0, p.biodiv_index * 100.0, p.solar, p.tilt
        ));
        ui.horizontal(|ui| {
            for (k, n) in ["Population", "Temperature", "Biodiversity", "Mean wealth", "CO₂", "Civ scores"].iter().enumerate() {
                ui.selectable_value(&mut v.graph_tab, k, *n);
            }
        });
        let st = &sim.stats;
        let series: Vec<(String, Vec<(i32, f32)>, Color32)> = match v.graph_tab {
            0 => vec![("Population".into(), st.years.iter().copied().zip(st.pop.iter().copied()).collect(), Color32::from_rgb(120, 220, 120))],
            1 => vec![("Temperature °C".into(), st.years.iter().copied().zip(st.temp.iter().copied()).collect(), Color32::from_rgb(240, 140, 80))],
            2 => vec![("Biodiversity".into(), st.years.iter().copied().zip(st.biodiv.iter().copied()).collect(), Color32::from_rgb(60, 220, 160))],
            3 => vec![("Mean wealth".into(), st.years.iter().copied().zip(st.wealth.iter().copied()).collect(), Color32::from_rgb(230, 200, 80))],
            4 => vec![("CO₂".into(), st.years.iter().copied().zip(st.co2.iter().copied()).collect(), Color32::from_rgb(180, 180, 200))],
            _ => sim.civs.iter().enumerate().filter(|(k, c)| !c.minor && st.civ_scores.get(*k).map(|s| s.len() > 1).unwrap_or(false)).map(|(k, c)| (c.name.clone(), st.civ_scores[k].clone(), c32(c.color))).collect(),
        };
        if let Some(id) = graph(ui, &series, &st.markers, &sim.news.items) {
            v.win_why = Some(id);
        }
        ui.label(RichText::new("Dots mark national and global stories; click one to ask what happened there.").small().weak());
        ui.collapsing("Intervention log", |ui| {
            for (y, s) in sim.intervention_log.iter().rev().take(40) {
                ui.label(format!("{} — {}", year_str(*y), s));
            }
        });
    });
    v.win_planet = open;
    // ---- decadal reviews
    let mut open = v.win_reviews;
    egui::Window::new("📅 Decadal reviews").open(&mut open).default_size([520.0, 420.0]).show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for r in sim.reviews.iter().rev().take(80) {
                ui.label(RichText::new(format!("{}–{}", year_str(r.year - 10), year_str(r.year))).strong());
                for l in &r.lines {
                    let warn = l.starts_with("WARNING");
                    ui.label(RichText::new(format!("  {}", l)).color(if warn { Color32::from_rgb(255, 120, 90) } else { Color32::LIGHT_GRAY }));
                }
            }
        });
    });
    v.win_reviews = open;
    // ---- objectives
    let mut open = v.win_objectives;
    if let Some(st) = sim.scenario.clone() {
        egui::Window::new("🎯 Objectives").open(&mut open).default_size([420.0, 240.0]).show(ctx, |ui| {
            ui.heading(&st.name);
            if let Mode::Scenario(s) = sim.mode {
                ui.label(crate::sim::scenario::scn_brief(s));
            }
            ui.separator();
            for (o, s) in &st.objectives {
                let (icon, col) = match s {
                    Some(true) => ("✔", Color32::from_rgb(120, 230, 120)),
                    Some(false) => ("✘", Color32::from_rgb(240, 90, 80)),
                    None => ("…", Color32::LIGHT_GRAY),
                };
                ui.label(RichText::new(format!("{} {}", icon, o)).color(col));
            }
            if let Some(sc) = st.score {
                ui.label(RichText::new(format!("Final score: {:.0}", sc)).strong());
            }
        });
    }
    v.win_objectives = open;
    // ---- settings
    let mut open = v.win_settings;
    egui::Window::new("⚙ Settings").open(&mut open).show(ctx, |ui| {
        ui.label(RichText::new("Auto-pause on alerts").strong());
        ui.checkbox(&mut sim.auto_pause_existential, "Existential (extinction, impacts, city destruction)");
        ui.checkbox(&mut sim.auto_pause_major, "Major (wars, successions, wonders)");
        ui.checkbox(&mut sim.auto_pause_watch, "Watchlist hits");
        ui.separator();
        ui.label(RichText::new("News significance thresholds").strong());
        ui.add(egui::Slider::new(&mut sim.news.threshold_ticker, 0.0..=4.0).text("ticker"));
        ui.add(egui::Slider::new(&mut sim.news.threshold_card, 1.0..=8.0).text("news card"));
        ui.label(RichText::new("Mute categories (watched subjects and followed threads still break through)").strong());
        ui.horizontal_wrapped(|ui| {
            for c in ["economic", "ecological", "health", "political", "religious", "cultural", "military", "technological"] {
                let muted = sim.news.muted.contains(&c);
                if ui.selectable_label(muted, format!("{}{}", if muted { "🔇 " } else { "" }, c)).clicked() {
                    if muted {
                        sim.news.muted.retain(|x| *x != c);
                    } else {
                        sim.news.muted.push(c);
                    }
                }
            }
        });
        ui.separator();
        ui.checkbox(&mut v.show_tooltip, "Map tooltips");
        ui.label(RichText::new(format!("Followed threads: {}", if sim.news.followed_threads.is_empty() { "none".into() } else { sim.news.followed_threads.join(", ") })).small());
        ui.label(RichText::new(format!("Seed {} · {} · difficulty {}", sim.params.seed, sim.mode.name(), sim.difficulty.name())).small().weak());
        ui.label(RichText::new(format!("Event catalog: {} events, {} news templates loaded", EVENTS.len(), sim.templates.list.len())).small().weak());
    });
    v.win_settings = open;
    // ---- help
    let mut open = v.win_help;
    egui::Window::new("Help").open(&mut open).default_size([480.0, 420.0]).show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(RichText::new("You are the planet's steward. You never command a civilization.").strong());
            ui.label("Peoples emerge from wandering tribes, found cities, research, trade, convert, quarrel and go to war on their own. You shape the conditions: terrain, climate, life, and — subtly — fortune.");
            ui.add_space(6.0);
            ui.label(RichText::new("Controls").strong());
            for l in [
                "Drag (left/right/middle) to pan; scroll to zoom; WASD/arrows pan.",
                "Click a city or tile to inspect. Right-click or Esc cancels a power.",
                "Space: pause/resume. 1–4: speed. N: news cards. C: chronicle.",
                "Click any headline to see 'Why did this happen?' — every story cites its causes.",
            ] {
                ui.label(format!("• {}", l));
            }
            ui.add_space(6.0);
            ui.label(RichText::new("Wealth").strong());
            ui.label("Each city has a wealth score 0–1000 (Destitute, Poor, Modest, Prosperous, Rich, Opulent). Wealth gates events in four patterns: A exposure (heists need something to steal), B vulnerability (poverty makes disasters worse), C gaps (rich beside poor breeds smuggling, agitation, migration), D prizes (only the rich can bid for expositions).");
            ui.add_space(6.0);
            ui.label(RichText::new("Gaia Points").strong());
            ui.label("Earned each year, faster on a biodiverse and stable planet, and as a reward when you refrain during a crisis and the city survives. Geological powers grow expensive once peoples rise. Repeated intervention in one region raises Attunement: people start to recognize the Shaper's hand, and faiths form around it.");
        });
    });
    v.win_help = open;
    // ---- alert modal
    if let Some(id) = v.alert {
        let n = sim.news.items[id].clone();
        egui::Window::new("⚠ Alert — paused").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            ui.label(RichText::new(&n.headline).strong().size(17.0));
            ui.label(&n.body);
            ui.horizontal(|ui| {
                if let Some((x, y)) = n.pos {
                    if ui.button("Show on map").clicked() {
                        v.center_request = Some((x, y));
                        if let Some(c) = n.city {
                            v.sel_city = Some(c);
                        }
                    }
                }
                if ui.button("Why?").clicked() {
                    v.win_why = Some(id);
                }
                if ui.button("Resume").clicked() {
                    v.alert = None;
                    v.speed = v.prev_speed.max(1);
                }
                if ui.button("Stay paused").clicked() {
                    v.alert = None;
                }
            });
        });
    }
    // ---- end of game
    if let Some(e) = sim.ended.clone() {
        if !v.end_shown {
            egui::Window::new("The watch ends").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
                ui.label(RichText::new(e).size(16.0));
                ui.horizontal(|ui| {
                    if matches!(sim.mode, Mode::Sandbox | Mode::Chronicle) && ui.button("Keep watching").clicked() {
                        v.end_shown = true;
                    }
                    if ui.button("Read the chronicle").clicked() {
                        v.win_chronicle = true;
                        v.end_shown = true;
                    }
                    if ui.button("Main menu").clicked() {
                        *back = true;
                    }
                });
            });
        }
    }
}

fn graph(ui: &mut Ui, series: &[(String, Vec<(i32, f32)>, Color32)], markers: &[(i32, usize)], items: &[NewsItem]) -> Option<usize> {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 260.0), Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, Color32::from_gray(20));
    let all: Vec<&(i32, f32)> = series.iter().flat_map(|s| s.1.iter()).collect();
    if all.len() < 2 {
        painter.text(rect.center(), Align2::CENTER_CENTER, "Not enough history yet", FontId::proportional(14.0), Color32::GRAY);
        return None;
    }
    let x0 = all.iter().map(|p| p.0).min().unwrap() as f32;
    let x1 = all.iter().map(|p| p.0).max().unwrap().max(x0 as i32 + 1) as f32;
    let mut y0 = all.iter().map(|p| p.1).fold(f32::MAX, f32::min);
    let mut y1 = all.iter().map(|p| p.1).fold(f32::MIN, f32::max);
    if (y1 - y0).abs() < 1e-3 {
        y1 += 1.0;
        y0 -= 1.0;
    }
    if y0 >= 0.0 && y0 < (y1 - y0) * 0.6 {
        y0 = 0.0;
    }
    let pad = (y1 - y0) * 0.08;
    let (y0, y1) = (y0 - pad, y1 + pad);
    let inner = rect.shrink2(Vec2::new(36.0, 14.0));
    let to = |x: f32, y: f32| Pos2::new(inner.min.x + (x - x0) / (x1 - x0) * inner.width(), inner.max.y - (y - y0) / (y1 - y0) * inner.height());
    painter.text(Pos2::new(rect.min.x + 2.0, inner.min.y), Align2::LEFT_TOP, format!("{:.1}", y1), FontId::proportional(10.0), Color32::GRAY);
    painter.text(Pos2::new(rect.min.x + 2.0, inner.max.y), Align2::LEFT_BOTTOM, format!("{:.1}", y0), FontId::proportional(10.0), Color32::GRAY);
    painter.text(Pos2::new(inner.min.x, rect.max.y - 1.0), Align2::LEFT_BOTTOM, format!("Year {}", x0), FontId::proportional(10.0), Color32::GRAY);
    painter.text(Pos2::new(inner.max.x, rect.max.y - 1.0), Align2::RIGHT_BOTTOM, format!("Year {}", x1), FontId::proportional(10.0), Color32::GRAY);
    for (_, pts, col) in series {
        let p: Vec<Pos2> = pts.iter().map(|&(x, y)| to(x as f32, y)).collect();
        painter.add(egui::Shape::line(p, Stroke::new(1.6, *col)));
    }
    // markers along the bottom
    let mut best: Option<(usize, f32)> = None;
    let hover = resp.hover_pos();
    for &(yr, id) in markers {
        let p = Pos2::new(to(yr as f32, y0).x, inner.max.y + 6.0);
        let col = match items[id].scope {
            Scope::Global => Color32::from_rgb(255, 210, 120),
            _ => Color32::from_rgb(150, 170, 220),
        };
        painter.circle_filled(p, 2.5, col);
        if let Some(h) = hover {
            let d = (h.x - p.x).abs();
            if d < 6.0 && best.map(|b| d < b.1).unwrap_or(true) {
                best = Some((id, d));
            }
        }
    }
    if let Some(h) = hover {
        if inner.contains(h) || h.y > inner.max.y {
            painter.line_segment([Pos2::new(h.x, inner.min.y), Pos2::new(h.x, inner.max.y)], Stroke::new(1.0, Color32::from_gray(80)));
            let yr = x0 + (h.x - inner.min.x) / inner.width() * (x1 - x0);
            let mut tip = format!("Year {:.0}", yr);
            for (name, pts, _) in series.iter().take(6) {
                if let Some(p) = pts.iter().min_by_key(|p| (p.0 as f32 - yr).abs() as i32) {
                    tip.push_str(&format!("\n{}: {:.2}", name, p.1));
                }
            }
            if let Some((id, _)) = best {
                tip.push_str(&format!("\n● {}", items[id].headline));
            }
            resp.clone().on_hover_text_at_pointer(tip);
        }
    }
    if resp.clicked() {
        if let Some((id, _)) = best {
            return Some(id);
        }
        // otherwise nearest marker in time
        if let Some(h) = resp.interact_pointer_pos() {
            let yr = x0 + (h.x - inner.min.x) / inner.width() * (x1 - x0);
            return markers.iter().min_by_key(|m| (m.0 as f32 - yr).abs() as i32).map(|m| m.1);
        }
    }
    None
}
