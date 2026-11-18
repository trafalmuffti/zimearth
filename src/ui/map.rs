//! Map rendering: layer coloring into a texture, plus vector overlays (cities, routes, armies, flashes).

use super::*;
use crate::sim::city::*;
use crate::sim::planet::*;

pub const SUB: usize = 3;

pub fn biome_color(b: Biome) -> [f32; 3] {
    let c: [u8; 3] = match b {
        Biome::DeepOcean => [16, 36, 82],
        Biome::Ocean => [26, 58, 118],
        Biome::Shallow => [46, 96, 156],
        Biome::SeaIce => [200, 216, 230],
        Biome::Ice => [236, 240, 246],
        Biome::Tundra => [148, 148, 122],
        Biome::Taiga => [50, 90, 64],
        Biome::TemperateForest => [52, 118, 56],
        Biome::Grassland => [128, 168, 80],
        Biome::Shrubland => [166, 158, 98],
        Biome::Desert => [214, 194, 130],
        Biome::Savanna => [178, 172, 86],
        Biome::Rainforest => [26, 98, 42],
        Biome::Wetland => [70, 112, 92],
        Biome::Mountain => [122, 108, 98],
    };
    [c[0] as f32, c[1] as f32, c[2] as f32]
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn ramp(stops: &[[f32; 3]], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 0.9999) * (stops.len() - 1) as f32;
    let k = t as usize;
    lerp3(stops[k], stops[k + 1], t - k as f32)
}

fn u8c(c: [u8; 3]) -> [f32; 3] {
    [c[0] as f32, c[1] as f32, c[2] as f32]
}

pub fn religion_color(r: usize) -> [f32; 3] {
    u8c(crate::sim::hsv((r as f32 * 77.0 + 20.0) % 360.0, 0.7, 0.95))
}

/// Which city's catchment covers each tile (for wealth/stability/religion choropleths).
fn tile_cities(sim: &Sim) -> Vec<Option<u32>> {
    let mut v = vec![None; sim.planet.tiles.len()];
    for c in sim.cities.iter().filter(|c| c.alive) {
        for &t in &c.catchment {
            v[t] = Some(c.id);
        }
    }
    v
}

pub fn build_image(sim: &Sim, layer: Layer, borders: bool) -> egui::ColorImage {
    let p = &sim.planet;
    let (w, h) = (p.w, p.h);
    let tc = if matches!(layer, Layer::Wealth | Layer::Stability | Layer::Religion) { tile_cities(sim) } else { Vec::new() };
    let mut base = vec![[0f32; 3]; w * h];
    for i in 0..w * h {
        let t = &p.tiles[i];
        let land = p.is_land(i);
        let (x, y) = p.xy(i);
        let mut col = biome_color(t.biome);
        if land {
            // hillshade
            let a = p.idx(x - 1, y - 1).map(|j| p.tiles[j].elev).unwrap_or(t.elev);
            let b = p.idx(x + 1, y + 1).map(|j| p.tiles[j].elev).unwrap_or(t.elev);
            let shade = (1.0 + (a - b) * 2.2).clamp(0.7, 1.3);
            col = [col[0] * shade, col[1] * shade, col[2] * shade];
            if t.forest > 0.05 && !matches!(t.biome, Biome::Rainforest | Biome::TemperateForest | Biome::Taiga) {
                col = lerp3(col, [40.0, 95.0, 45.0], t.forest * 0.5);
            }
            if t.ash > 0.2 {
                col = lerp3(col, [90.0, 85.0, 85.0], (t.ash * 0.4).min(0.6));
            }
            if t.pollution > 0.1 {
                col = lerp3(col, [110.0, 100.0, 70.0], t.pollution * 0.5);
            }
        } else {
            let depth = (-p.height(i)).clamp(0.0, 1.0);
            col = lerp3(col, [10.0, 24.0, 60.0], depth * 0.4);
        }
        let dim = |c: [f32; 3], f: f32| {
            let g = (c[0] + c[1] + c[2]) / 3.0;
            lerp3(c, [g, g, g], 0.6).map(|v| v * f)
        };
        col = match layer {
            Layer::Terrain => col,
            Layer::Political => {
                let d = lerp3(col, dim(col, 1.0), 0.5);
                match t.owner {
                    Some(o) if land => lerp3(d, u8c(sim.civs[o as usize].color), 0.55),
                    _ => d,
                }
            }
            Layer::Temperature => {
                let v = (t.temp + 30.0) / 70.0;
                let c = ramp(&[[40.0, 40.0, 160.0], [90.0, 160.0, 230.0], [235.0, 235.0, 220.0], [240.0, 170.0, 60.0], [200.0, 40.0, 30.0]], v);
                if land { c } else { lerp3(c, [20.0, 30.0, 60.0], 0.45) }
            }
            Layer::Rainfall => {
                if land {
                    ramp(&[[150.0, 100.0, 50.0], [200.0, 190.0, 90.0], [70.0, 170.0, 70.0], [40.0, 120.0, 200.0]], t.rain / 1.3)
                } else {
                    [20.0, 30.0, 55.0]
                }
            }
            Layer::Geology => {
                let mut c = if land {
                    let e = p.height(i).clamp(0.0, 1.0);
                    ramp(&[[70.0, 110.0, 70.0], [150.0, 140.0, 100.0], [190.0, 180.0, 170.0], [250.0, 250.0, 250.0]], e)
                } else {
                    lerp3([40.0, 70.0, 130.0], [8.0, 16.0, 45.0], (-p.height(i)).clamp(0.0, 1.0))
                };
                if t.fault {
                    c = lerp3(c, [220.0, 50.0, 50.0], 0.55);
                }
                if t.uplift_stress > 0.05 {
                    c = lerp3(c, [230.0, 60.0, 220.0], t.uplift_stress.min(1.0) * 0.6);
                }
                if t.attune > 0.5 {
                    c = lerp3(c, [255.0, 215.0, 90.0], (t.attune / 8.0).min(0.6));
                }
                c
            }
            Layer::Resources => dim(col, 0.6),
            Layer::Biodiversity => {
                let v = t.biodiv;
                let c = ramp(&[[90.0, 60.0, 50.0], [190.0, 180.0, 80.0], [60.0, 170.0, 70.0], [20.0, 230.0, 160.0]], v);
                if land { c } else { lerp3(c, [20.0, 40.0, 90.0], 0.5) }
            }
            Layer::Religion => {
                let d = dim(col, 0.6);
                match tc.get(i).copied().flatten().and_then(|c| sim.cities[c as usize].religion) {
                    Some(r) => lerp3(d, religion_color(r), 0.65),
                    None => d,
                }
            }
            Layer::Trade => dim(col, 0.55),
            Layer::Wealth => {
                let d = dim(col, 0.45);
                match tc.get(i).copied().flatten() {
                    Some(c) => lerp3(d, u8c(BAND_COLORS[band_index(sim.cities[c as usize].wealth)]), 0.8),
                    None => d,
                }
            }
            Layer::Stability => {
                let d = dim(col, 0.45);
                match tc.get(i).copied().flatten() {
                    Some(c) => {
                        let city = &sim.cities[c as usize];
                        let s = ramp(&[[200.0, 40.0, 40.0], [220.0, 200.0, 60.0], [60.0, 190.0, 90.0]], city.stability / 100.0);
                        if city.plague_severity > 0.05 { lerp3(s, [160.0, 40.0, 200.0], 0.6) } else { s }
                    }
                    None => d,
                }
            }
            Layer::Chronicle => {
                let mut d = dim(col, 0.55);
                if t.scorch > 0.05 {
                    d = lerp3(d, [120.0, 40.0, 30.0], t.scorch.min(1.0));
                }
                d
            }
        };
        base[i] = col;
    }
    let (iw, ih) = (w * SUB, h * SUB);
    let mut px = vec![0u8; iw * ih * 4];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let t = &p.tiles[i];
            let c = base[i];
            for sy in 0..SUB {
                for sx in 0..SUB {
                    let mut cc = c;
                    // border pixels where ownership changes
                    if borders && layer != Layer::Political || layer == Layer::Political {
                        if let Some(o) = t.owner {
                            let (xi, yi) = (x as i32, y as i32);
                            let mut nbs = [None; 4];
                            if sx == 0 { nbs[0] = p.idx(xi - 1, yi); }
                            if sx == SUB - 1 { nbs[1] = p.idx(xi + 1, yi); }
                            if sy == 0 { nbs[2] = p.idx(xi, yi - 1); }
                            if sy == SUB - 1 { nbs[3] = p.idx(xi, yi + 1); }
                            if nbs.iter().flatten().any(|&n| p.tiles[n].owner != Some(o)) {
                                let oc = u8c(sim.civs[o as usize].color);
                                cc = lerp3(oc, [0.0, 0.0, 0.0], 0.15);
                            }
                        }
                    }
                    // rivers through the tile center
                    if t.is_river() && p.is_land(i) && matches!(layer, Layer::Terrain | Layer::Political | Layer::Rainfall | Layer::Geology | Layer::Chronicle) {
                        let big = t.river > RIVER_T * 3.0;
                        if (sx == 1 && sy == 1) || (big && (sx == 1 || sy == 1)) {
                            cc = [60.0, 120.0, 215.0];
                        }
                    }
                    let k = ((y * SUB + sy) * iw + x * SUB + sx) * 4;
                    px[k] = cc[0].clamp(0.0, 255.0) as u8;
                    px[k + 1] = cc[1].clamp(0.0, 255.0) as u8;
                    px[k + 2] = cc[2].clamp(0.0, 255.0) as u8;
                    px[k + 3] = 255;
                }
            }
            if layer == Layer::Resources {
                if let Some(r) = t.resource {
                    let rc = r.kind.color();
                    let known = r.kind.revealed_by().map(|k| sim.civs.iter().any(|c| c.alive && c.known_tech[k])).unwrap_or(true);
                    let f = if known { 1.0 } else { 0.55 };
                    let k = ((y * SUB + 1) * iw + x * SUB + 1) * 4;
                    px[k] = (rc[0] as f32 * f) as u8;
                    px[k + 1] = (rc[1] as f32 * f) as u8;
                    px[k + 2] = (rc[2] as f32 * f) as u8;
                }
            }
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([iw, ih], &px)
}

pub fn c32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub fn draw_map(ui: &mut Ui, sim: &mut Sim, v: &mut View) {
    let rect = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(rect, Sense::click_and_drag());
    let (w, h) = (sim.planet.w as f32, sim.planet.h as f32);
    if !v.cam_init {
        v.zoom = (rect.width() / w).min(rect.height() / h).max(3.0);
        v.cam = Vec2::new(0.0, (h - rect.height() / v.zoom) / 2.0);
        v.cam_init = true;
    }
    // ---- input
    let pointer = resp.hover_pos();
    if resp.dragged_by(egui::PointerButton::Secondary) || resp.dragged_by(egui::PointerButton::Middle) || (resp.dragged_by(egui::PointerButton::Primary) && v.tool.is_none()) {
        v.cam -= resp.drag_delta() / v.zoom;
    }
    if let Some(pp) = pointer {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.0 {
            let before = v.cam + (pp - rect.min) / v.zoom;
            v.zoom = (v.zoom * (1.0 + scroll * 0.0015)).clamp(2.0, 60.0);
            v.cam = before - (pp - rect.min) / v.zoom;
        }
    }
    let kb = ui.input(|i| {
        let mut d = Vec2::ZERO;
        if i.key_down(egui::Key::ArrowLeft) || i.key_down(egui::Key::A) { d.x -= 1.0; }
        if i.key_down(egui::Key::ArrowRight) || i.key_down(egui::Key::D) { d.x += 1.0; }
        if i.key_down(egui::Key::ArrowUp) || i.key_down(egui::Key::W) { d.y -= 1.0; }
        if i.key_down(egui::Key::ArrowDown) || i.key_down(egui::Key::S) { d.y += 1.0; }
        d * i.stable_dt * 600.0
    });
    if kb != Vec2::ZERO && !ui.ctx().egui_wants_keyboard_input() {
        v.cam += kb / v.zoom;
    }
    v.cam.x = v.cam.x.rem_euclid(w);
    let view_h = rect.height() / v.zoom;
    v.cam.y = if view_h >= h { (h - view_h) / 2.0 } else { v.cam.y.clamp(-2.0, h - view_h + 2.0) };
    let (camx, camy, zm) = (v.cam.x, v.cam.y, v.zoom);
    let to_screen = move |tx: f32, ty: f32, k: f32| -> Pos2 { Pos2::new(rect.min.x + (tx - camx + k * w) * zm, rect.min.y + (ty - camy) * zm) };
    let to_tile = move |p: Pos2| -> (i32, i32) {
        let tx = ((p.x - rect.min.x) / zm + camx).floor() as i32;
        let ty = ((p.y - rect.min.y) / zm + camy).floor() as i32;
        (tx.rem_euclid(w as i32), ty)
    };
    let hovered_tile = pointer.and_then(|p| {
        let (tx, ty) = to_tile(p);
        sim.planet.idx(tx, ty)
    });
    if resp.clicked() {
        if let Some(i) = hovered_tile {
            if let Some(tool) = v.tool {
                match sim.apply_power(tool, i) {
                    Ok(m) => v.toast(m),
                    Err(e) => v.toast(e),
                }
                v.tex_dirty = true;
            } else {
                v.sel_tile = Some(i);
                let (tx, ty) = sim.planet.xy(i);
                let city = sim.planet.tiles_in_radius(tx, ty, 1).into_iter().filter_map(|t| sim.planet.tiles[t].city_slot).find(|&c| sim.cities[c as usize].alive);
                v.sel_city = city;
                if let Some(c) = city {
                    v.sel_civ = Some(sim.cities[c as usize].civ);
                    v.right_tab = 0;
                } else if let Some(o) = sim.planet.tiles[i].owner {
                    v.sel_civ = Some(o);
                }
            }
        }
    }
    if resp.secondary_clicked() {
        v.tool = None;
    }
    // ---- texture
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(8, 14, 30));
    if v.tex_dirty || v.tex.is_none() || v.tex_layer != v.layer || v.tex_borders != v.show_borders {
        let img = build_image(sim, v.layer, v.show_borders);
        match &mut v.tex {
            Some(t) => t.set(img, egui::TextureOptions::NEAREST),
            None => v.tex = Some(ui.ctx().load_texture("map", img, egui::TextureOptions::NEAREST)),
        }
        v.tex_dirty = false;
        v.tex_layer = v.layer;
        v.tex_borders = v.show_borders;
    }
    let tex = v.tex.as_ref().unwrap().id();
    let copies: Vec<f32> = (-1..=2).map(|k| k as f32).filter(|&k| {
        let x0 = rect.min.x + (k * w - v.cam.x) * v.zoom;
        x0 < rect.max.x && x0 + w * v.zoom > rect.min.x
    }).collect();
    for &k in &copies {
        let r = Rect::from_min_max(to_screen(0.0, 0.0, k), to_screen(w, h, k));
        painter.image(tex, r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }
    let z = v.zoom;
    let font = |s: f32| FontId::proportional(s);
    // ---- overlays per copy
    for &k in &copies {
        // trade routes
        if v.show_routes || v.layer == Layer::Trade {
            for r in sim.routes.iter().filter(|r| r.alive) {
                let a = &sim.cities[r.a as usize];
                let b = &sim.cities[r.b as usize];
                let dx = sim.planet.wrap_dx(a.x as f32, b.x as f32);
                let p0 = to_screen(a.x as f32 + 0.5, a.y as f32 + 0.5, k);
                let p1 = to_screen(a.x as f32 + 0.5 + dx, b.y as f32 + 0.5, k);
                let raided = r.raided_until > sim.year;
                let col = if raided { Color32::from_rgba_unmultiplied(200, 60, 60, 180) } else if r.sea { Color32::from_rgba_unmultiplied(120, 200, 255, 150) } else { Color32::from_rgba_unmultiplied(240, 200, 90, 160) };
                painter.line_segment([p0, p1], Stroke::new((r.value / 8.0).clamp(1.0, 3.0), col));
            }
        }
        // chronicle markers
        if v.layer == Layer::Chronicle || z > 10.0 {
            for (i, t) in sim.planet.tiles.iter().enumerate() {
                let (x, y) = sim.planet.xy(i);
                let p = to_screen(x as f32 + 0.5, y as f32 + 0.5, k);
                if !rect.contains(p) {
                    continue;
                }
                if t.natural_wonder.is_some() {
                    painter.text(p, Align2::CENTER_CENTER, "✦", font((z * 0.9).clamp(9.0, 18.0)), Color32::from_rgb(140, 255, 230));
                }
                if v.layer == Layer::Chronicle {
                    if t.ruin.is_some() {
                        painter.text(p, Align2::CENTER_CENTER, "⌂", font((z * 0.9).clamp(9.0, 16.0)), Color32::from_rgb(200, 180, 150));
                    } else if t.battle {
                        painter.text(p, Align2::CENTER_CENTER, "⚔", font((z * 0.8).clamp(8.0, 14.0)), Color32::from_rgb(230, 90, 80));
                    }
                }
                if t.volcano > 0.0 && (v.layer == Layer::Geology || z > 14.0) {
                    painter.text(p, Align2::CENTER_CENTER, "▲", font((z * 0.8).clamp(8.0, 14.0)), Color32::from_rgb(255, 120 + (t.volcano.min(1.0) * 100.0) as u8, 40));
                }
            }
        }
        // tribes
        for t in sim.tribes.iter().filter(|t| t.alive) {
            let p = to_screen(t.x as f32 + 0.5, t.y as f32 + 0.5, k);
            painter.circle_filled(p, (z * 0.25).clamp(2.0, 5.0), Color32::from_rgb(200, 190, 170));
            painter.circle_stroke(p, (z * 0.25).clamp(2.0, 5.0), Stroke::new(1.0, Color32::BLACK));
        }
        // cities
        for c in sim.cities.iter().filter(|c| c.alive) {
            let p = to_screen(c.x as f32 + 0.5, c.y as f32 + 0.5, k);
            if !rect.expand(40.0).contains(p) {
                continue;
            }
            let civ = &sim.civs[c.civ as usize];
            let r = (z * (0.28 + c.pop.sqrt() * 0.07)).clamp(3.0, z * 1.2).min(18.0);
            let col = c32(civ.color);
            if civ.minor {
                painter.rect_filled(Rect::from_center_size(p, Vec2::splat(r * 1.6)), 1.0, col);
                painter.rect_stroke(Rect::from_center_size(p, Vec2::splat(r * 1.6)), 1.0, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Outside);
            } else {
                painter.circle_filled(p, r, col);
                painter.circle_stroke(p, r, Stroke::new(1.2, Color32::BLACK));
            }
            if c.is_capital {
                painter.circle_stroke(p, r + 2.0, Stroke::new(1.5, Color32::WHITE));
            }
            if c.protected_until > sim.year {
                painter.circle_stroke(p, r + 4.0, Stroke::new(1.0, Color32::from_rgb(255, 230, 120)));
            }
            if c.plague_severity > 0.05 {
                painter.circle_stroke(p, r + 3.0, Stroke::new(1.5, Color32::from_rgb(170, 60, 220)));
            }
            if c.siege > 0.5 {
                painter.circle_stroke(p, r + 3.0, Stroke::new(2.0, Color32::from_rgb(255, 50, 30)));
            }
            if v.layer == Layer::Wealth && z >= 5.0 {
                painter.text(p + Vec2::new(0.0, -r - 6.0), Align2::CENTER_BOTTOM, format!("{:.0}", c.wealth), font(10.0), c32(BAND_COLORS[band_index(c.wealth)]));
            }
            if v.show_labels && (z >= 7.0 || (z >= 4.0 && (c.is_capital || c.pop > 12.0))) {
                let txt = if c.is_capital { format!("★ {}", c.name) } else { c.name.clone() };
                let pos = p + Vec2::new(0.0, r + 2.0);
                painter.text(pos + Vec2::new(1.0, 1.0), Align2::CENTER_TOP, &txt, font(11.0), Color32::BLACK);
                painter.text(pos, Align2::CENTER_TOP, &txt, font(11.0), Color32::WHITE);
            }
            if v.sel_city == Some(c.id) {
                painter.circle_stroke(p, r + 6.0, Stroke::new(2.0, Color32::YELLOW));
            }
        }
        // armies
        if v.show_armies {
            for a in sim.armies.iter().filter(|a| a.alive) {
                let p = to_screen(a.x + 0.5, a.y + 0.5, k);
                if !rect.contains(p) {
                    continue;
                }
                let s = (z * 0.35).clamp(3.0, 8.0) * (1.0 + (a.strength / 40.0).min(1.0));
                let col = a.owner.map(|o| c32(sim.civs[o as usize].color)).unwrap_or(Color32::from_rgb(40, 40, 40));
                let pts = vec![p + Vec2::new(0.0, -s), p + Vec2::new(s * 0.9, s * 0.7), p + Vec2::new(-s * 0.9, s * 0.7)];
                let stroke = if a.owner.is_none() { Stroke::new(1.5, Color32::from_rgb(230, 60, 60)) } else if a.sieging { Stroke::new(1.5, Color32::from_rgb(255, 80, 40)) } else { Stroke::new(1.0, Color32::BLACK) };
                if a.target.is_some() || a.owner.is_none() {
                    painter.add(egui::Shape::convex_polygon(pts, col, stroke));
                }
            }
        }
        // event flashes
        let now = ui.input(|i| i.time) as f32;
        for &(x, y, yr, col) in &sim.flashes {
            let p = to_screen(x as f32 + 0.5, y as f32 + 0.5, k);
            let age = (sim.year - yr) as f32;
            let pulse = (now * 3.0).sin() * 0.5 + 0.5;
            let r = z * (1.0 + age * 0.6) + pulse * 4.0;
            let a = (255.0 * (1.0 - age / 4.0)).clamp(0.0, 255.0) as u8;
            painter.circle_stroke(p, r, Stroke::new(2.0, Color32::from_rgba_unmultiplied(col[0], col[1], col[2], a)));
        }
        // selection and brush
        if let Some(i) = v.sel_tile {
            let (x, y) = sim.planet.xy(i);
            let r = Rect::from_min_max(to_screen(x as f32, y as f32, k), to_screen(x as f32 + 1.0, y as f32 + 1.0, k));
            painter.rect_stroke(r, 0.0, Stroke::new(1.5, Color32::YELLOW), egui::StrokeKind::Outside);
        }
        if let (Some(tool), Some(i)) = (v.tool, hovered_tile) {
            let (x, y) = sim.planet.xy(i);
            let rad = match tool {
                Power::RaiseLand | Power::LowerLand => 2.0,
                Power::Warm | Power::Cool | Power::Wetten | Power::Dry => 5.0,
                Power::SeedForest | Power::Bounty | Power::Meteor | Power::GreatQuake => 3.0,
                _ => 1.0,
            };
            painter.circle_stroke(to_screen(x as f32 + 0.5, y as f32 + 0.5, k), rad * z + z * 0.5, Stroke::new(1.5, Color32::from_rgb(255, 220, 120)));
        }
    }
    // tooltip
    if let Some(i) = hovered_tile {
        if v.show_tooltip {
            let t = &sim.planet.tiles[i];
            let mut s = format!("{}  {:.0}°C  rain {:.2}", t.biome.name(), t.temp, t.rain);
            if let Some(o) = t.owner {
                s.push_str(&format!("\n{}", sim.civs[o as usize].name));
            }
            if let Some(c) = t.city_slot.filter(|&c| sim.cities[c as usize].alive) {
                let c = &sim.cities[c as usize];
                s.push_str(&format!("\n{} — pop {} — {} ({:.0})", c.name, fmt_pop(c.display_pop(sim.civs[c.civ as usize].era)), c.band(), c.wealth));
            }
            if let Some(r) = t.resource {
                s.push_str(&format!("\n{}", r.kind.name()));
            }
            if let Some(n) = &t.natural_wonder {
                s.push_str(&format!("\n✦ {}", n));
            }
            if let Some(n) = &t.ruin {
                s.push_str(&format!("\n⌂ {}", n));
            }
            if let Some(tool) = v.tool {
                s.push_str(&format!("\n[{} — {:.0} GP] click to apply, right-click to cancel", tool.name(), sim.power_cost(tool)));
            }
            resp.on_hover_text_at_pointer(s);
        }
    }
}

pub fn fmt_pop(p: u64) -> String {
    if p >= 1_000_000 {
        format!("{:.2}M", p as f64 / 1e6)
    } else if p >= 1000 {
        format!("{:.1}k", p as f64 / 1e3)
    } else {
        p.to_string()
    }
}

pub fn center_on(v: &mut View, sim: &Sim, x: i32, y: i32, rect_w: f32, rect_h: f32) {
    v.cam = Vec2::new(x as f32 + 0.5 - rect_w / v.zoom / 2.0, y as f32 + 0.5 - rect_h / v.zoom / 2.0);
    v.cam.x = v.cam.x.rem_euclid(sim.planet.w as f32);
}
