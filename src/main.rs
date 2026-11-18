//! Living Worlds — a planetary simulation where NPC civilizations rise, compete, and report back.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod sim;
mod ui;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

fn batch(args: &[String]) {
    let years: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(500);
    let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(7);
    let mut p = sim::planet::WorldParams::default();
    p.seed = seed;
    let mode = match args.get(4).map(|s| s.as_str()) {
        Some("rivers") => sim::scenario::Mode::Scenario(sim::scenario::Scn::FirstRivers),
        Some("ice") => sim::scenario::Mode::Scenario(sim::scenario::Scn::IceAtTheDoor),
        Some("ash") => sim::scenario::Mode::Scenario(sim::scenario::Scn::AshenCentury),
        Some("two") => sim::scenario::Mode::Scenario(sim::scenario::Scn::TwoContinents),
        Some("sun") => sim::scenario::Mode::Challenge(sim::scenario::Challenge::DyingSun),
        Some("tilt") => sim::scenario::Mode::Challenge(sim::scenario::Challenge::ExtremeTilt),
        Some("super") => sim::scenario::Mode::Challenge(sim::scenario::Challenge::Supercontinent),
        Some("impact") => sim::scenario::Mode::Challenge(sim::scenario::Challenge::ScheduledImpact),
        _ => sim::scenario::Mode::Chronicle,
    };
    let mut s = sim::Sim::new(p, mode, sim::Difficulty::Steward);
    let t = std::time::Instant::now();
    for _ in 0..years {
        s.tick();
    }
    let mut bands = [0; 6];
    for c in s.cities.iter().filter(|c| c.alive) {
        bands[sim::city::band_index(c.wealth)] += 1;
    }
    eprintln!(
        "{} years in {:?} | temp {:.1} biodiv {:.2} | civs alive {} ever {} | cities {} wealth bands {:?} | news {}",
        years,
        t.elapsed(),
        s.planet.global_temp,
        s.planet.biodiv_index,
        s.civs.iter().filter(|c| c.alive).count(),
        s.civs.len(),
        s.cities.iter().filter(|c| c.alive).count(),
        bands,
        s.news.items.len()
    );
    if let Some(e) = &s.ended { eprintln!("ENDED: {}", e); }
    println!("{}", s.chronicle_text());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|a| a == "--batch").unwrap_or(false) {
        batch(&args);
        return;
    }
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.03, 0.05, 0.1)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Living Worlds".into(),
                resolution: WindowResolution::new(1600, 900),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .init_resource::<ui::Game>()
        .add_systems(Startup, ui::setup_camera)
        .add_systems(Update, ui::tick_system)
        .add_systems(EguiPrimaryContextPass, ui::ui_system)
        .run();
}
