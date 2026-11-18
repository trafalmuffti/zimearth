//! Bevy + egui presentation layer. The simulation is the single source of truth; the UI only reads
//! it and issues player powers.

pub mod map;
pub mod panels;

use crate::sim::Difficulty;
use crate::sim::Sim;
use crate::sim::planet::{Atmosphere, WorldParams};
use crate::sim::powers::Power;
use crate::sim::scenario::{Challenge, Mode, Scn};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use egui::{Align2, Color32, FontId, LayerId, Pos2, Rect, RichText, Sense, Stroke, Ui, UiBuilder, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Terrain,
    Political,
    Temperature,
    Rainfall,
    Geology,
    Resources,
    Biodiversity,
    Religion,
    Trade,
    Wealth,
    Stability,
    Chronicle,
}

pub const LAYERS: [(Layer, &str, &str); 12] = [
    (Layer::Terrain, "Terrain", "Biomes, relief and rivers"),
    (Layer::Political, "Political", "Borders and civilizations"),
    (Layer::Temperature, "Temperature", "Annual mean temperature"),
    (Layer::Rainfall, "Rainfall", "Rain from wind bands, upwind oceans, rain shadows and forests"),
    (Layer::Geology, "Geology & hazards", "Elevation, faults (red), uplift stress (magenta), volcanoes, attunement (gold)"),
    (Layer::Resources, "Resources", "Deposits: strategic red, luxury yellow, stock grey; dimmed = not yet revealed by technology"),
    (Layer::Biodiversity, "Biodiversity", "Living richness per tile"),
    (Layer::Religion, "Religion", "Majority faith of each city's lands"),
    (Layer::Trade, "Trade", "Trade routes (gold: land, blue: sea, red: raided/quarantined)"),
    (Layer::Wealth, "Wealth", "City wealth bands as a choropleth — the fastest way to see gaps forming"),
    (Layer::Stability, "Stability & disease", "Stability red→green; purple = plague"),
    (Layer::Chronicle, "Chronicle", "Ruins, battlefields, scorched land, wonders"),
];

pub const SPEEDS: [(f32, &str); 5] = [(0.0, "⏸"), (0.5, "1×"), (1.5, "3×"), (5.0, "10×"), (20.0, "40×")];

pub struct View {
    pub layer: Layer,
    pub show_routes: bool,
    pub show_borders: bool,
    pub show_labels: bool,
    pub show_armies: bool,
    pub show_tooltip: bool,
    pub cam: Vec2,
    pub zoom: f32,
    pub cam_init: bool,
    pub tex: Option<egui::TextureHandle>,
    pub tex_layer: Layer,
    pub tex_borders: bool,
    pub tex_dirty: bool,
    pub sel_tile: Option<usize>,
    pub sel_city: Option<u32>,
    pub sel_civ: Option<u16>,
    pub tool: Option<Power>,
    pub speed: usize,
    pub prev_speed: usize,
    pub acc: f32,
    pub win_cards: bool,
    pub win_chronicle: bool,
    pub win_diplo: bool,
    pub win_planet: bool,
    pub win_civs: bool,
    pub win_reviews: bool,
    pub win_help: bool,
    pub win_objectives: bool,
    pub win_settings: bool,
    pub win_why: Option<usize>,
    pub diplo_pair: Option<(u16, u16)>,
    pub chron_filter: String,
    pub chron_cat: String,
    pub toasts: Vec<(String, f32)>,
    pub alert: Option<usize>,
    pub end_shown: bool,
    pub right_tab: usize,
    pub map_rect: Rect,
    pub center_request: Option<(i32, i32)>,
    pub graph_tab: usize,
}

impl Default for View {
    fn default() -> Self {
        View {
            layer: Layer::Political,
            show_routes: true,
            show_borders: true,
            show_labels: true,
            show_armies: true,
            show_tooltip: true,
            cam: Vec2::ZERO,
            zoom: 8.0,
            cam_init: false,
            tex: None,
            tex_layer: Layer::Political,
            tex_borders: true,
            tex_dirty: true,
            sel_tile: None,
            sel_city: None,
            sel_civ: None,
            tool: None,
            speed: 2,
            prev_speed: 2,
            acc: 0.0,
            win_cards: false,
            win_chronicle: false,
            win_diplo: false,
            win_planet: false,
            win_civs: false,
            win_reviews: false,
            win_help: false,
            win_objectives: false,
            win_settings: false,
            win_why: None,
            diplo_pair: None,
            chron_filter: String::new(),
            chron_cat: String::new(),
            toasts: Vec::new(),
            alert: None,
            end_shown: false,
            right_tab: 0,
            map_rect: Rect::NOTHING,
            center_request: None,
            graph_tab: 0,
        }
    }
}

impl View {
    pub fn toast(&mut self, s: impl Into<String>) {
        self.toasts.push((s.into(), 4.0));
    }
}

pub struct MenuState {
    pub params: WorldParams,
    pub size: usize,
    pub mode: usize,
    pub difficulty: Difficulty,
    pub seed_text: String,
}

pub const SIZES: [(&str, usize, usize); 4] = [("Small", 64, 40), ("Standard", 100, 64), ("Large", 136, 86), ("Huge", 160, 104)];

pub fn modes() -> Vec<Mode> {
    vec![
        Mode::Sandbox,
        Mode::Chronicle,
        Mode::Scenario(Scn::FirstRivers),
        Mode::Scenario(Scn::IceAtTheDoor),
        Mode::Scenario(Scn::AshenCentury),
        Mode::Scenario(Scn::TwoContinents),
        Mode::Challenge(Challenge::DyingSun),
        Mode::Challenge(Challenge::ExtremeTilt),
        Mode::Challenge(Challenge::Supercontinent),
        Mode::Challenge(Challenge::ScheduledImpact),
    ]
}

#[derive(Resource)]
pub struct Game {
    pub sim: Option<Sim>,
    pub view: View,
    pub menu: MenuState,
    pub style_set: bool,
}

impl Default for Game {
    fn default() -> Self {
        let seed = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(1) % 1_000_000) as u64;
        Game {
            sim: None,
            view: View::default(),
            menu: MenuState { params: WorldParams { seed, ..Default::default() }, size: 1, mode: 0, difficulty: Difficulty::Steward, seed_text: seed.to_string() },
            style_set: false,
        }
    }
}

pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

pub fn tick_system(time: Res<Time>, mut game: ResMut<Game>) {
    let g = &mut *game;
    let Some(sim) = g.sim.as_mut() else { return };
    let v = &mut g.view;
    let dt = time.delta_secs();
    for t in v.toasts.iter_mut() {
        t.1 -= dt;
    }
    v.toasts.retain(|t| t.1 > 0.0);
    if v.alert.is_some() || v.speed == 0 {
        return;
    }
    v.acc += dt * SPEEDS[v.speed].0;
    let mut n = 0;
    while v.acc >= 1.0 && n < 6 {
        sim.tick();
        v.acc -= 1.0;
        n += 1;
        v.tex_dirty = true;
        if !sim.alerts.is_empty() {
            v.alert = Some(sim.alerts.remove(0));
            sim.alerts.clear();
            v.prev_speed = v.speed;
            v.speed = 0;
            break;
        }
        if sim.ended.is_some() && !v.end_shown {
            v.prev_speed = v.speed;
            v.speed = 0;
            break;
        }
    }
    if v.acc > 3.0 {
        v.acc = 0.0;
    }
}

pub fn ui_system(mut contexts: EguiContexts, mut game: ResMut<Game>) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    let g = &mut *game;
    if !g.style_set {
        ctx.set_visuals(egui::Visuals::dark());
        g.style_set = true;
    }
    let mut root = Ui::new(ctx.clone(), "root".into(), UiBuilder::new().layer_id(LayerId::background()).max_rect(ctx.viewport_rect()));
    if g.sim.is_none() {
        if let Some((params, mode, diff)) = draw_menu(&mut root, &mut g.menu) {
            g.sim = Some(Sim::new(params, mode, diff));
            g.view = View::default();
            if matches!(mode, Mode::Scenario(_) | Mode::Challenge(_)) {
                g.view.win_objectives = true;
            }
            if matches!(mode, Mode::Chronicle) {
                g.view.speed = 3;
            }
        }
        return Ok(());
    }
    let back = panels::draw_game(&mut root, &ctx, g.sim.as_mut().unwrap(), &mut g.view);
    if back {
        g.sim = None;
    }
    Ok(())
}

fn draw_menu(root: &mut Ui, m: &mut MenuState) -> Option<(WorldParams, Mode, Difficulty)> {
    let mut start = None;
    egui::CentralPanel::default().show(root, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(18.0);
                ui.label(RichText::new("LIVING WORLDS").size(46.0).strong().color(Color32::from_rgb(140, 210, 160)));
                ui.label(RichText::new("A planetary simulation where NPC civilizations rise, compete, and report back").size(16.0).italics());
                ui.add_space(4.0);
                ui.label("You do not rule a people. You shape the planet beneath them.");
                ui.add_space(14.0);
            });
            ui.columns(2, |cols| {
                let ui = &mut cols[0];
                ui.heading("Game mode");
                let ms = modes();
                for (k, md) in ms.iter().enumerate() {
                    ui.radio_value(&mut m.mode, k, md.name());
                }
                ui.add_space(6.0);
                let md = ms[m.mode];
                let brief = match md {
                    Mode::Sandbox => "No win condition. Shape the world and watch its history unfold. The chronicle and stewardship score are your soft goals.",
                    Mode::Chronicle => "Interventions disabled. Configure, start and watch a full history generate for export.",
                    Mode::Scenario(s) => crate::sim::scenario::scn_brief(s),
                    Mode::Challenge(Challenge::DyingSun) => "The sun dims year after year. Keep a civilization alive to Year 500.",
                    Mode::Challenge(Challenge::ExtremeTilt) => "40° axial tilt: brutal seasons and huge frozen zones. Keep a civilization alive to Year 500.",
                    Mode::Challenge(Challenge::Supercontinent) => "One supercontinent: crowded, dry interiors. Keep a civilization alive to Year 500.",
                    Mode::Challenge(Challenge::ScheduledImpact) => "Two impacts are scheduled for Year 250. Keep a civilization alive to Year 500.",
                };
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(brief);
                });
                ui.add_space(10.0);
                ui.heading("Difficulty");
                for d in [Difficulty::Storyteller, Difficulty::Steward, Difficulty::Challenger, Difficulty::HarshPlanet] {
                    ui.radio_value(&mut m.difficulty, d, d.name()).on_hover_text(d.desc());
                }
                ui.label(RichText::new(m.difficulty.desc()).weak());
                let ui = &mut cols[1];
                ui.heading("Planet");
                let p = &mut m.params;
                egui::Grid::new("params").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Size");
                    egui::ComboBox::from_id_salt("size").selected_text(SIZES[m.size].0).show_ui(ui, |ui| {
                        for (k, s) in SIZES.iter().enumerate() {
                            ui.selectable_value(&mut m.size, k, format!("{} ({}×{})", s.0, s.1, s.2));
                        }
                    });
                    ui.end_row();
                    ui.label("Land fraction");
                    ui.add(egui::Slider::new(&mut p.land_frac, 0.15..=0.8).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
                    ui.end_row();
                    ui.label("Tectonic activity");
                    ui.add(egui::Slider::new(&mut p.tectonics, 0.0..=1.0));
                    ui.end_row();
                    ui.label("Axial tilt");
                    ui.add(egui::Slider::new(&mut p.tilt, 0.0..=40.0).suffix("°"));
                    ui.end_row();
                    ui.label("Solar output");
                    ui.add(egui::Slider::new(&mut p.solar, -1.0..=1.0));
                    ui.end_row();
                    ui.label("Atmosphere");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut p.atmosphere, Atmosphere::Reducing, "Reducing");
                        ui.radio_value(&mut p.atmosphere, Atmosphere::EarthLike, "Earth-like");
                        ui.radio_value(&mut p.atmosphere, Atmosphere::OxygenRich, "Oxygen-rich");
                    });
                    ui.end_row();
                    ui.label("Moons");
                    ui.add(egui::Slider::new(&mut p.moons, 0..=2));
                    ui.end_row();
                    ui.label("Cosmic volatility");
                    ui.add(egui::Slider::new(&mut p.cosmic, 0.0..=1.0));
                    ui.end_row();
                    ui.label("Seed");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut m.seed_text).desired_width(90.0));
                        if ui.button("Random").clicked() {
                            let s = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(7) % 1_000_000) as u64;
                            m.seed_text = s.to_string();
                        }
                    });
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.label(RichText::new("Consequences").strong());
                let mut lines = Vec::new();
                lines.push(if p.land_frac > 0.55 { "Much land: crowded continents, big dry interiors, navies matter less." } else if p.land_frac < 0.28 { "Little land: island peoples, sea trade and navigation dominate." } else { "Balanced land and sea." });
                lines.push(if p.tectonics > 0.7 { "Violent tectonics: tall mountains, many faults and volcanoes, rich ores — and frequent quakes." } else if p.tectonics < 0.2 { "Dormant crust: gentle terrain, few hazards, scarcer ore." } else { "Moderate tectonics." });
                lines.push(if p.tilt > 30.0 { "High tilt: stronger seasons, larger frozen zones, more variable harvests." } else if p.tilt < 10.0 { "Low tilt: mild seasons, stable harvests, colder poles." } else { "Earth-like seasons." });
                lines.push(if p.solar > 0.3 { "Bright sun: a warm world, smaller ice caps, higher seas." } else if p.solar < -0.3 { "Dim sun: a cold world with large ice caps." } else { "Temperate sun." });
                lines.push(match p.atmosphere { Atmosphere::Reducing => "Reducing air: strong greenhouse, warm and humid.", Atmosphere::EarthLike => "Earth-like air.", Atmosphere::OxygenRich => "Oxygen-rich air: weaker greenhouse, more wildfires." });
                lines.push(match p.moons { 0 => "No moon: weak tides, poorer coastal fisheries.", 1 => "One moon: tides enrich coasts; moon faiths possible.", _ => "Two moons: strong tides; moon faiths likely." });
                lines.push(if p.cosmic > 0.5 { "High cosmic volatility: impacts will happen." } else { "Impacts are rare." });
                for l in lines {
                    ui.label(format!("• {}", l));
                }
                ui.add_space(14.0);
                if ui.add(egui::Button::new(RichText::new("   Begin the world   ").size(22.0).strong()).fill(Color32::from_rgb(40, 100, 70))).clicked() {
                    let mut params = m.params.clone();
                    params.w = SIZES[m.size].1;
                    params.h = SIZES[m.size].2;
                    params.seed = m.seed_text.trim().parse().unwrap_or_else(|_| m.seed_text.bytes().fold(7u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64)));
                    start = Some((params, ms[m.mode], m.difficulty));
                }
            });
        });
    });
    start
}
