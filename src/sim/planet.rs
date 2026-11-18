//! Planet: tile grid on a cylinder, plate-tectonic generation, coupled climate, rivers and biomes.

use noise::{NoiseFn, Perlin};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Biome {
    DeepOcean,
    Ocean,
    Shallow,
    SeaIce,
    Ice,
    Tundra,
    Taiga,
    TemperateForest,
    Grassland,
    Shrubland,
    Desert,
    Savanna,
    Rainforest,
    Wetland,
    Mountain,
}

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::DeepOcean => "Deep ocean",
            Biome::Ocean => "Ocean",
            Biome::Shallow => "Shallow sea",
            Biome::SeaIce => "Sea ice",
            Biome::Ice => "Ice sheet",
            Biome::Tundra => "Tundra",
            Biome::Taiga => "Boreal forest",
            Biome::TemperateForest => "Temperate forest",
            Biome::Grassland => "Grassland",
            Biome::Shrubland => "Shrubland",
            Biome::Desert => "Desert",
            Biome::Savanna => "Savanna",
            Biome::Rainforest => "Rainforest",
            Biome::Wetland => "Wetland",
            Biome::Mountain => "Mountains",
        }
    }
    pub fn is_water(self) -> bool {
        matches!(self, Biome::DeepOcean | Biome::Ocean | Biome::Shallow | Biome::SeaIce)
    }
    pub fn forest_target(self) -> f32 {
        match self {
            Biome::Rainforest => 0.95,
            Biome::TemperateForest => 0.8,
            Biome::Taiga => 0.7,
            Biome::Wetland => 0.4,
            Biome::Savanna => 0.2,
            Biome::Grassland => 0.12,
            Biome::Shrubland => 0.1,
            Biome::Mountain => 0.15,
            _ => 0.0,
        }
    }
    pub fn base_biodiv(self) -> f32 {
        match self {
            Biome::Rainforest => 1.0,
            Biome::Wetland => 0.8,
            Biome::TemperateForest => 0.7,
            Biome::Shallow => 0.75,
            Biome::Savanna => 0.6,
            Biome::Grassland => 0.5,
            Biome::Taiga => 0.45,
            Biome::Shrubland => 0.35,
            Biome::Mountain => 0.35,
            Biome::Ocean => 0.4,
            Biome::DeepOcean => 0.2,
            Biome::Tundra => 0.2,
            Biome::Desert => 0.12,
            Biome::SeaIce | Biome::Ice => 0.05,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResClass {
    Stock,
    Biological,
    Luxury,
    Strategic,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ResKind {
    Stone,
    Copper,
    Iron,
    Coal,
    Gold,
    Gems,
    Oil,
    Uranium,
    Spices,
    Silk,
    Dyes,
    Incense,
    Horses,
    Salt,
    Marble,
    Wine,
}

pub const ALL_RES: [ResKind; 16] = [
    ResKind::Stone,
    ResKind::Copper,
    ResKind::Iron,
    ResKind::Coal,
    ResKind::Gold,
    ResKind::Gems,
    ResKind::Oil,
    ResKind::Uranium,
    ResKind::Spices,
    ResKind::Silk,
    ResKind::Dyes,
    ResKind::Incense,
    ResKind::Horses,
    ResKind::Salt,
    ResKind::Marble,
    ResKind::Wine,
];

impl ResKind {
    pub fn name(self) -> &'static str {
        match self {
            ResKind::Stone => "Stone",
            ResKind::Copper => "Copper",
            ResKind::Iron => "Iron",
            ResKind::Coal => "Coal",
            ResKind::Gold => "Gold",
            ResKind::Gems => "Gems",
            ResKind::Oil => "Oil",
            ResKind::Uranium => "Uranium",
            ResKind::Spices => "Spices",
            ResKind::Silk => "Silk",
            ResKind::Dyes => "Dyes",
            ResKind::Incense => "Incense",
            ResKind::Horses => "Horses",
            ResKind::Salt => "Salt",
            ResKind::Marble => "Marble",
            ResKind::Wine => "Wine",
        }
    }
    pub fn class(self) -> ResClass {
        match self {
            ResKind::Stone | ResKind::Marble | ResKind::Salt => ResClass::Stock,
            ResKind::Copper | ResKind::Iron | ResKind::Coal | ResKind::Oil | ResKind::Uranium => {
                ResClass::Strategic
            }
            ResKind::Horses => ResClass::Biological,
            ResKind::Gold | ResKind::Gems => ResClass::Luxury,
            ResKind::Spices | ResKind::Silk | ResKind::Dyes | ResKind::Incense | ResKind::Wine => {
                ResClass::Luxury
            }
        }
    }
    /// Technology index (see civ::TECHS) that reveals this deposit, if hidden.
    pub fn revealed_by(self) -> Option<usize> {
        use crate::sim::civ::tech_index;
        match self {
            ResKind::Iron => Some(tech_index("Iron Working")),
            ResKind::Coal => Some(tech_index("Steam Power")),
            ResKind::Oil => Some(tech_index("Combustion")),
            ResKind::Uranium => Some(tech_index("Nuclear Fission")),
            ResKind::Horses => Some(tech_index("Animal Husbandry")),
            _ => None,
        }
    }
    pub fn color(self) -> [u8; 3] {
        match self.class() {
            ResClass::Strategic => [230, 90, 60],
            ResClass::Luxury => [240, 210, 60],
            ResClass::Stock => [200, 200, 200],
            ResClass::Biological => [120, 230, 120],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Resource {
    pub kind: ResKind,
    pub richness: f32,
    pub stock: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Improvement {
    None,
    Farm,
    Mine,
    Pasture,
    Plantation,
    Quarry,
    Road,
    Park,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub elev: f32,
    pub temp: f32,
    pub rain: f32,
    pub fert: f32,
    pub soil: f32,
    pub biome: Biome,
    pub forest: f32,
    pub river: f32,
    pub resource: Option<Resource>,
    pub owner: Option<u16>,
    pub city_slot: Option<u32>,
    pub improvement: Improvement,
    pub pollution: f32,
    pub fault: bool,
    pub volcano: f32,
    pub ash: f32,
    pub fish: f32,
    pub game: f32,
    pub biodiv: f32,
    pub attune: f32,
    pub scorch: f32,
    pub temp_nudge: f32,
    pub rain_nudge: f32,
    pub uplift_stress: f32,
    pub ruin: Option<String>,
    pub battle: bool,
    pub continent: u16,
    pub plate: u8,
    pub natural_wonder: Option<String>,
    pub blessed: f32,
}

impl Tile {
    fn new() -> Self {
        Tile {
            elev: 0.0,
            temp: 10.0,
            rain: 0.5,
            fert: 0.5,
            soil: 1.0,
            biome: Biome::Ocean,
            forest: 0.0,
            river: 0.0,
            resource: None,
            owner: None,
            city_slot: None,
            improvement: Improvement::None,
            pollution: 0.0,
            fault: false,
            volcano: 0.0,
            ash: 0.0,
            fish: 1.0,
            game: 1.0,
            biodiv: 0.5,
            attune: 0.0,
            scorch: 0.0,
            temp_nudge: 0.0,
            rain_nudge: 0.0,
            uplift_stress: 0.0,
            ruin: None,
            battle: false,
            continent: 0,
            plate: 0,
            natural_wonder: None,
            blessed: 0.0,
        }
    }
    pub fn is_river(&self) -> bool {
        self.river > RIVER_T
    }
}

pub const RIVER_T: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Atmosphere {
    Reducing,
    EarthLike,
    OxygenRich,
}

#[derive(Clone, Debug)]
pub struct WorldParams {
    pub w: usize,
    pub h: usize,
    pub land_frac: f32,
    pub tectonics: f32,
    pub tilt: f32,
    pub solar: f32,
    pub atmosphere: Atmosphere,
    pub moons: u8,
    pub cosmic: f32,
    pub seed: u64,
    pub supercontinent: bool,
}

impl Default for WorldParams {
    fn default() -> Self {
        WorldParams {
            w: 100,
            h: 64,
            land_frac: 0.38,
            tectonics: 0.5,
            tilt: 23.5,
            solar: 0.0,
            atmosphere: Atmosphere::EarthLike,
            moons: 1,
            cosmic: 0.15,
            seed: 1,
            supercontinent: false,
        }
    }
}

pub struct Planet {
    pub w: usize,
    pub h: usize,
    pub tiles: Vec<Tile>,
    pub sea_level: f32,
    pub co2: f32,
    pub o2: f32,
    pub particulates: f32,
    pub solar: f32,
    pub tilt: f32,
    pub global_temp: f32,
    pub ice_frac: f32,
    pub biodiv_index: f32,
    pub land_tiles: usize,
    pub continents: Vec<(u16, usize)>,
    pub moons: u8,
    pub ocean_names: Vec<String>,
    pub ice0: f32,
}

pub fn lat(y: usize, h: usize) -> f32 {
    ((y as f32 + 0.5) / h as f32) * 2.0 - 1.0
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Planet {
    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if y < 0 || y >= self.h as i32 {
            return None;
        }
        let x = x.rem_euclid(self.w as i32);
        Some(y as usize * self.w + x as usize)
    }
    #[inline]
    pub fn xy(&self, i: usize) -> (i32, i32) {
        ((i % self.w) as i32, (i / self.w) as i32)
    }
    #[inline]
    pub fn is_land(&self, i: usize) -> bool {
        self.tiles[i].elev > self.sea_level
    }
    pub fn height(&self, i: usize) -> f32 {
        self.tiles[i].elev - self.sea_level
    }
    /// Wrapped horizontal distance aware squared distance.
    pub fn dist2(&self, ax: i32, ay: i32, bx: i32, by: i32) -> f32 {
        let mut dx = (ax - bx).abs();
        let w = self.w as i32;
        if dx > w / 2 {
            dx = w - dx;
        }
        let dy = ay - by;
        (dx * dx + dy * dy) as f32
    }
    pub fn dist(&self, ax: i32, ay: i32, bx: i32, by: i32) -> f32 {
        self.dist2(ax, ay, bx, by).sqrt()
    }
    /// Shortest signed dx from a to b on the cylinder.
    pub fn wrap_dx(&self, ax: f32, bx: f32) -> f32 {
        let w = self.w as f32;
        let mut dx = bx - ax;
        if dx > w / 2.0 {
            dx -= w;
        }
        if dx < -w / 2.0 {
            dx += w;
        }
        dx
    }
    pub fn neighbors8(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let (x, y) = self.xy(i);
        const D: [(i32, i32); 8] = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        D.iter().filter_map(move |(dx, dy)| self.idx(x + dx, y + dy))
    }
    pub fn tiles_in_radius(&self, x: i32, y: i32, r: i32) -> Vec<usize> {
        let mut v = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r + r {
                    if let Some(i) = self.idx(x + dx, y + dy) {
                        v.push(i);
                    }
                }
            }
        }
        v
    }
    pub fn is_coastal(&self, i: usize) -> bool {
        self.is_land(i) && self.neighbors8(i).any(|n| !self.is_land(n))
    }

    pub fn generate(p: &WorldParams, rng: &mut ChaCha8Rng) -> Planet {
        let (w, h) = (p.w, p.h);
        let mut tiles = vec![Tile::new(); w * h];
        let perlin = Perlin::new(rng.r#gen::<u32>());
        let perlin2 = Perlin::new(rng.r#gen::<u32>());
        // --- plates
        let n_plates = if p.supercontinent { 7 } else { 8 + (w * h) / 1100 };
        struct Plate {
            x: f32,
            y: f32,
            cont: bool,
            vx: f32,
            vy: f32,
        }
        let mut plates: Vec<Plate> = Vec::new();
        for k in 0..n_plates {
            let cont = if p.supercontinent {
                k < 3
            } else {
                rng.r#gen::<f32>() < (p.land_frac * 1.1 + 0.05)
            };
            let (px, py) = if p.supercontinent && cont {
                (
                    w as f32 * 0.5 + rng.gen_range(-8.0..8.0),
                    h as f32 * 0.5 + rng.gen_range(-8.0..8.0),
                )
            } else {
                (
                    rng.gen_range(0.0..w as f32),
                    rng.gen_range(h as f32 * 0.1..h as f32 * 0.9),
                )
            };
            plates.push(Plate {
                x: px,
                y: py,
                cont,
                vx: rng.gen_range(-1.0..1.0),
                vy: rng.gen_range(-1.0..1.0),
            });
        }
        let wf = w as f32;
        let mut raw = vec![0f32; w * h];
        let mut boundary = vec![0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                // warp the sample point to get organic plate edges
                let theta = x as f64 / w as f64 * std::f64::consts::TAU;
                let r = w as f64 / std::f64::consts::TAU / 9.0;
                let nx = theta.cos() * r;
                let nz = theta.sin() * r;
                let ny = y as f64 / 9.0;
                let warp_x = perlin2.get([nx, ny, nz]) as f32 * 7.0;
                let warp_y = perlin2.get([nx + 31.0, ny, nz - 17.0]) as f32 * 7.0;
                let sx = x as f32 + warp_x;
                let sy = y as f32 + warp_y;
                let mut best = (f32::MAX, 0usize);
                let mut second = (f32::MAX, 0usize);
                for (k, pl) in plates.iter().enumerate() {
                    let mut dx = (sx - pl.x).abs() % wf;
                    if dx > wf / 2.0 {
                        dx = wf - dx;
                    }
                    let dy = sy - pl.y;
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < best.0 {
                        second = best;
                        best = (d, k);
                    } else if d < second.0 {
                        second = (d, k);
                    }
                }
                let pa = &plates[best.1];
                let pb = &plates[second.1];
                tiles[i].plate = best.1 as u8;
                let base = if pa.cont { 0.32 } else { -0.45 };
                let edge = second.0 - best.0;
                // relative motion along boundary normal
                let mut nxv = pb.x - pa.x;
                if nxv > wf / 2.0 {
                    nxv -= wf;
                }
                if nxv < -wf / 2.0 {
                    nxv += wf;
                }
                let nyv = pb.y - pa.y;
                let nl = (nxv * nxv + nyv * nyv).sqrt().max(0.01);
                let conv = ((pa.vx - pb.vx) * nxv / nl + (pa.vy - pb.vy) * nyv / nl) * 0.5;
                let fall = (-edge / 2.2).exp();
                let mut e = base;
                if conv > 0.0 {
                    let up = if pa.cont && pb.cont {
                        0.75
                    } else if pa.cont {
                        0.55
                    } else {
                        0.25
                    };
                    e += conv * up * fall * (0.4 + p.tectonics * 1.2);
                } else {
                    let rift = if pa.cont { -0.25 } else { 0.12 };
                    e += -conv * rift * fall * (0.4 + p.tectonics);
                }
                boundary[i] = fall * conv.abs();
                // continental shelves blend
                if pa.cont != pb.cont {
                    let blend = (1.0 - (edge / 6.0)).clamp(0.0, 1.0) * 0.5;
                    let other = if pb.cont { 0.32 } else { -0.45 };
                    e = e * (1.0 - blend) + other * blend;
                }
                // fractal noise
                let mut amp = 1.0f64;
                let mut freq = 1.0f64;
                let mut n = 0.0f64;
                for _ in 0..6 {
                    let r = w as f64 / std::f64::consts::TAU / 14.0 * freq;
                    n += perlin.get([
                        theta.cos() * r,
                        y as f64 / 14.0 * freq,
                        theta.sin() * r,
                    ]) * amp;
                    amp *= 0.5;
                    freq *= 2.0;
                }
                e += n as f32 * 0.42;
                // polar caps of land are less likely
                let l = lat(y, h).abs();
                e -= smoothstep(0.85, 1.0, l) * 0.2;
                raw[i] = e;
            }
        }
        // sea level by quantile
        let mut sorted = raw.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let q = ((1.0 - p.land_frac.clamp(0.1, 0.9)) * (w * h) as f32) as usize;
        let thresh = sorted[q.min(w * h - 1)];
        let maxv = sorted[w * h - 1] - thresh;
        let minv = thresh - sorted[0];
        for i in 0..w * h {
            let e = raw[i] - thresh;
            tiles[i].elev = if e > 0.0 {
                (e / maxv).powf(1.3)
            } else {
                e / minv
            };
            let b = boundary[i];
            if b > 0.2 * (1.2 - p.tectonics) && p.tectonics > 0.05 {
                tiles[i].fault = true;
            }
        }
        // volcanoes at strong convergent boundaries
        let n_volc = (3.0 + p.tectonics * 18.0 * (w * h) as f32 / 6400.0) as usize;
        let mut cand: Vec<usize> = (0..w * h).filter(|&i| boundary[i] > 0.15).collect();
        for _ in 0..n_volc {
            if cand.is_empty() {
                break;
            }
            let k = rng.gen_range(0..cand.len());
            let i = cand.swap_remove(k);
            tiles[i].volcano = rng.gen_range(0.2..1.0) * (0.4 + p.tectonics);
            if tiles[i].elev < 0.05 {
                tiles[i].elev = rng.gen_range(0.05..0.3);
            }
        }
        let mut planet = Planet {
            w,
            h,
            tiles,
            sea_level: 0.0,
            co2: match p.atmosphere {
                Atmosphere::Reducing => 1.6,
                Atmosphere::EarthLike => 1.0,
                Atmosphere::OxygenRich => 0.85,
            },
            o2: match p.atmosphere {
                Atmosphere::Reducing => 0.12,
                Atmosphere::EarthLike => 0.21,
                Atmosphere::OxygenRich => 0.3,
            },
            particulates: 0.0,
            solar: p.solar,
            tilt: p.tilt,
            global_temp: 14.0,
            ice_frac: 0.0,
            biodiv_index: 0.5,
            land_tiles: 0,
            continents: Vec::new(),
            moons: p.moons,
            ocean_names: Vec::new(),
            ice0: 0.0,
        };
        planet.smooth(1);
        planet.update_climate();
        planet.update_climate();
        planet.compute_rivers();
        planet.update_biomes(true);
        planet.compute_continents();
        planet.place_resources(rng);
        planet.place_natural_wonders(rng);
        for t in planet.tiles.iter_mut() {
            t.forest = t.biome.forest_target();
            t.biodiv = t.biome.base_biodiv();
            t.game = 1.0;
            t.fish = if t.biome == Biome::Shallow { 1.0 } else { 0.6 };
        }
        planet.update_biodiv_index();
        planet.ice0 = planet.ice_frac;
        planet
    }

    fn smooth(&mut self, passes: usize) {
        for _ in 0..passes {
            let old: Vec<f32> = self.tiles.iter().map(|t| t.elev).collect();
            for i in 0..self.tiles.len() {
                let mut s = old[i] * 2.0;
                let mut n = 2.0;
                for j in self.neighbors8(i) {
                    s += old[j];
                    n += 1.0;
                }
                let avg = s / n;
                // keep sign so coastlines don't drift too much
                let v = old[i] * 0.5 + avg * 0.5;
                self.tiles[i].elev = if (old[i] > 0.0) == (v > 0.0) { v } else { old[i] };
            }
        }
    }

    pub fn compute_continents(&mut self) {
        let n = self.tiles.len();
        for t in self.tiles.iter_mut() {
            t.continent = 0;
        }
        let mut id = 0u16;
        let mut sizes = Vec::new();
        let mut stack = Vec::new();
        for s in 0..n {
            if !self.is_land(s) || self.tiles[s].continent != 0 {
                continue;
            }
            id += 1;
            let mut count = 0;
            stack.push(s);
            self.tiles[s].continent = id;
            while let Some(i) = stack.pop() {
                count += 1;
                let ns: Vec<usize> = self.neighbors8(i).collect();
                for j in ns {
                    if self.is_land(j) && self.tiles[j].continent == 0 {
                        self.tiles[j].continent = id;
                        stack.push(j);
                    }
                }
            }
            sizes.push((id, count));
        }
        self.land_tiles = sizes.iter().map(|s| s.1).sum();
        self.continents = sizes;
    }

    /// Coupled climate model: temperature from latitude, elevation, solar, greenhouse,
    /// ice albedo and westerly ocean transport; rainfall from wind bands, upwind moisture,
    /// rain shadows and forest transpiration.
    pub fn update_climate(&mut self) {
        let (w, h) = (self.w, self.h);
        let greenhouse = 4.2 * self.co2.max(0.05).ln();
        let solar = self.solar * 9.0;
        let albedo = -self.ice_frac * 7.0;
        let dust = -self.particulates * 4.0;
        let global = solar + greenhouse + albedo + dust;
        let polar_drop = 46.0 - (self.tilt - 23.5) * 0.45;
        let mut temps = vec![0f32; w * h];
        for y in 0..h {
            let a = lat(y, h).abs();
            let base = 32.5 - polar_drop * a.powf(1.55);
            for x in 0..w {
                let i = y * w + x;
                let t = &self.tiles[i];
                let hgt = (t.elev - self.sea_level).max(0.0);
                temps[i] = base - hgt * 26.0 + global + t.temp_nudge;
            }
        }
        // ocean heat transport: smooth sea temperatures
        for _ in 0..3 {
            let old = temps.clone();
            for i in 0..w * h {
                if !self.is_land(i) {
                    let mut s = old[i] * 2.0;
                    let mut n = 2.0;
                    for j in self.neighbors8(i) {
                        if !self.is_land(j) {
                            s += old[j];
                            n += 1.0;
                        }
                    }
                    temps[i] = s / n;
                }
            }
        }
        // westerly transport warms downwind coasts in the mid latitudes
        for y in 0..h {
            let a = lat(y, h).abs();
            if !(0.3..0.72).contains(&a) {
                continue;
            }
            for x in 0..w {
                let i = y * w + x;
                if !self.is_land(i) {
                    continue;
                }
                for k in 1..5 {
                    if let Some(j) = self.idx(x as i32 - k, y as i32) {
                        if !self.is_land(j) {
                            temps[i] += 3.2 * (1.0 - k as f32 / 5.0);
                            break;
                        }
                    }
                }
            }
        }
        // rainfall
        let mut rain = vec![0f32; w * h];
        for y in 0..h {
            let l = lat(y, h);
            let a = l.abs();
            let dir: i32 = if a < 0.33 {
                1 // trade winds blow east->west; upwind is east
            } else if a < 0.68 {
                -1 // westerlies; upwind is west
            } else {
                1
            };
            let band = 0.32
                + 0.95 * (-(a / 0.16).powi(2)).exp()
                + 0.6 * (-((a - 0.57) / 0.13).powi(2)).exp();
            let band = band * (1.0 - 0.5 * smoothstep(0.75, 1.0, a));
            for x in 0..w {
                let i = y * w + x;
                let mut m = 0.35f32;
                let mut prev_h = 0.0f32;
                for k in (0..20).rev() {
                    let j = match self.idx(x as i32 + dir * k, y as i32) {
                        Some(j) => j,
                        None => continue,
                    };
                    let tj = &self.tiles[j];
                    let hj = (tj.elev - self.sea_level).max(0.0);
                    if hj <= 0.0 {
                        let warm = ((temps[j] + 5.0) / 30.0).clamp(0.2, 1.2);
                        m = (m + 0.22 * warm).min(1.2);
                    } else {
                        m *= 0.92 + tj.forest * 0.06;
                        if hj - prev_h > 0.12 {
                            m *= 0.55;
                        }
                    }
                    prev_h = hj;
                }
                let t = &self.tiles[i];
                rain[i] = (m * band + t.rain_nudge).clamp(0.0, 1.5);
            }
        }
        let mut ice = 0usize;
        let mut sum_t = 0.0;
        for i in 0..w * h {
            let t = &mut self.tiles[i];
            t.temp = temps[i];
            t.rain = rain[i];
            sum_t += temps[i];
            let land = t.elev > self.sea_level;
            if (land && temps[i] < -9.0) || (!land && temps[i] < -4.0) {
                ice += 1;
            }
        }
        self.ice_frac = ice as f32 / (w * h) as f32;
        self.global_temp = sum_t / (w * h) as f32;
    }

    pub fn compute_rivers(&mut self) {
        let n = self.tiles.len();
        let mut order: Vec<usize> = (0..n).filter(|&i| self.is_land(i)).collect();
        order.sort_by(|&a, &b| self.tiles[b].elev.partial_cmp(&self.tiles[a].elev).unwrap());
        let mut flow = vec![0f32; n];
        for &i in &order {
            flow[i] += self.tiles[i].rain.max(0.0) * 0.9;
            let mut best = None;
            let mut be = self.tiles[i].elev;
            for j in self.neighbors8(i) {
                if self.tiles[j].elev < be {
                    be = self.tiles[j].elev;
                    best = Some(j);
                }
            }
            if let Some(j) = best {
                if self.is_land(j) {
                    flow[j] += flow[i];
                }
            }
        }
        for i in 0..n {
            self.tiles[i].river = if self.is_land(i) { flow[i] } else { 0.0 };
        }
    }

    pub fn update_biomes(&mut self, init: bool) {
        for i in 0..self.tiles.len() {
            let land = self.is_land(i);
            let hgt = self.height(i);
            let t = &mut self.tiles[i];
            let b = if !land {
                if t.temp < -4.0 {
                    Biome::SeaIce
                } else if hgt > -0.12 {
                    Biome::Shallow
                } else if hgt > -0.5 {
                    Biome::Ocean
                } else {
                    Biome::DeepOcean
                }
            } else if t.temp < -9.0 {
                Biome::Ice
            } else if hgt > 0.62 {
                Biome::Mountain
            } else if t.temp < -1.0 {
                Biome::Tundra
            } else if t.temp < 5.0 {
                if t.rain > 0.3 { Biome::Taiga } else { Biome::Tundra }
            } else if t.temp < 18.0 {
                if t.rain > 0.95 && hgt < 0.06 {
                    Biome::Wetland
                } else if t.rain > 0.55 {
                    Biome::TemperateForest
                } else if t.rain > 0.3 {
                    Biome::Grassland
                } else if t.rain > 0.16 {
                    Biome::Shrubland
                } else {
                    Biome::Desert
                }
            } else if t.rain > 0.85 {
                Biome::Rainforest
            } else if t.rain > 0.45 {
                Biome::Savanna
            } else if t.rain > 0.22 {
                Biome::Shrubland
            } else {
                Biome::Desert
            };
            t.biome = b;
            if init {
                t.forest = b.forest_target();
            }
            // fertility: water + warmth + river + volcanic ash + soil health
            let water = (t.rain / 0.7).min(1.0) * (1.0 - ((t.rain - 1.1).max(0.0)));
            let warmth = ((t.temp - 0.0) / 14.0).clamp(0.0, 1.0)
                * (1.0 - ((t.temp - 30.0) / 10.0).clamp(0.0, 0.6));
            let river = if t.river > RIVER_T { 0.35 } else { 0.0 };
            let mut f = (water * warmth * 0.85 + river + t.ash * 0.2) * t.soil;
            if !land || matches!(b, Biome::Ice | Biome::Mountain) {
                f = 0.0;
            }
            f -= t.pollution * 0.3 + t.scorch * 0.4;
            t.fert = f.clamp(0.0, 1.2);
        }
    }

    pub fn update_biodiv_index(&mut self) {
        let mut s = 0.0;
        let mut n = 0.0;
        for t in &self.tiles {
            s += t.biodiv;
            n += 1.0;
        }
        self.biodiv_index = s / n;
    }

    fn place_resources(&mut self, rng: &mut ChaCha8Rng) {
        let n = self.tiles.len();
        for i in 0..n {
            if !self.is_land(i) {
                continue;
            }
            let hgt = self.height(i);
            let t = &self.tiles[i];
            let mut options: Vec<(ResKind, f32)> = Vec::new();
            let fault = t.fault || t.volcano > 0.0;
            if hgt > 0.3 {
                options.push((ResKind::Stone, 0.05));
                options.push((ResKind::Copper, 0.04));
                options.push((ResKind::Iron, 0.05));
                options.push((ResKind::Marble, 0.02));
            }
            if fault {
                options.push((ResKind::Gold, 0.05));
                options.push((ResKind::Gems, 0.04));
                options.push((ResKind::Uranium, 0.03));
            }
            if hgt < 0.25 && matches!(t.biome, Biome::TemperateForest | Biome::Taiga | Biome::Wetland)
            {
                options.push((ResKind::Coal, 0.05));
            }
            if hgt < 0.2 && matches!(t.biome, Biome::Desert | Biome::Tundra | Biome::Wetland | Biome::Shrubland) {
                options.push((ResKind::Oil, 0.05));
                options.push((ResKind::Salt, 0.025));
            }
            match t.biome {
                Biome::Rainforest => {
                    options.push((ResKind::Spices, 0.05));
                    options.push((ResKind::Dyes, 0.03));
                }
                Biome::Savanna | Biome::Shrubland => {
                    options.push((ResKind::Incense, 0.03));
                }
                Biome::Grassland => {
                    options.push((ResKind::Horses, 0.05));
                    options.push((ResKind::Wine, 0.02));
                }
                Biome::TemperateForest => {
                    options.push((ResKind::Silk, 0.025));
                    options.push((ResKind::Wine, 0.02));
                }
                _ => {}
            }
            for (k, p) in options {
                if rng.r#gen::<f32>() < p {
                    let richness = rng.gen_range(0.5..1.5);
                    self.tiles[i].resource = Some(Resource {
                        kind: k,
                        richness,
                        stock: match k.class() {
                            ResClass::Stock | ResClass::Strategic | ResClass::Luxury
                                if matches!(
                                    k,
                                    ResKind::Gold | ResKind::Gems | ResKind::Stone
                                        | ResKind::Copper | ResKind::Iron | ResKind::Coal
                                        | ResKind::Oil | ResKind::Uranium | ResKind::Marble
                                        | ResKind::Salt
                                ) =>
                            {
                                richness * rng.gen_range(150.0..500.0)
                            }
                            _ => f32::INFINITY,
                        },
                    });
                    break;
                }
            }
        }
    }

    fn place_natural_wonders(&mut self, rng: &mut ChaCha8Rng) {
        let names = [
            "the Glass Falls",
            "the Singing Dunes",
            "the Sky Pillars",
            "the Mirror Lake",
            "the Emerald Rift",
            "the Burning Spring",
            "the Old Giant Tree",
            "the Thunder Gorge",
        ];
        let land: Vec<usize> = (0..self.tiles.len()).filter(|&i| self.is_land(i)).collect();
        if land.is_empty() {
            return;
        }
        let k = 2 + self.tiles.len() / 2500;
        for n in 0..k.min(names.len()) {
            let i = land[rng.gen_range(0..land.len())];
            self.tiles[i].natural_wonder = Some(names[(n * 3 + rng.gen_range(0..2)) % names.len()].to_string());
        }
    }

    /// Yearly physical update (step 1 of the tick order).
    pub fn year_update(&mut self, year: i32) {
        // nudges decay toward equilibrium, the climate pushes back
        for t in self.tiles.iter_mut() {
            t.temp_nudge *= 0.97;
            t.rain_nudge *= 0.96;
            t.ash *= 0.85;
            t.scorch *= 0.93;
            t.pollution *= 0.985;
            t.uplift_stress *= 0.98;
            t.blessed = (t.blessed - 1.0).max(0.0);
            if t.temp_nudge.abs() < 0.01 {
                t.temp_nudge = 0.0;
            }
        }
        // carbon cycle: forests absorb, equilibrium drift toward 1.0
        let forest: f32 = self.tiles.iter().map(|t| t.forest).sum::<f32>() / self.tiles.len() as f32;
        self.co2 += (1.0 - self.co2) * 0.004 - forest * 0.0015 + 0.0006;
        self.co2 = self.co2.max(0.2);
        self.particulates *= 0.75;
        self.update_climate();
        // sea level follows ice volume slowly
        let target_sea = (self.ice0 - self.ice_frac) * 0.3;
        self.sea_level += (target_sea - self.sea_level) * 0.02;
        if year % 20 == 0 {
            self.compute_rivers();
            self.compute_continents();
        }
        self.update_biomes(false);
        // forest and stocks regrow toward biome targets
        for t in self.tiles.iter_mut() {
            let target = t.biome.forest_target() * (1.0 - t.pollution).max(0.0);
            if t.improvement == Improvement::None || t.improvement == Improvement::Park {
                t.forest += (target - t.forest) * 0.03;
            }
            t.game = (t.game + 0.04 * (1.0 - t.game)).min(1.0);
            t.fish = (t.fish + 0.06 * (1.0 - t.fish)).min(1.0);
            t.soil = (t.soil + 0.004).min(1.0);
            let ff = if t.biome.is_water() || t.biome.forest_target() < 0.05 { 1.0 } else { (0.55 + 0.45 * t.forest / t.biome.forest_target()).min(1.0) };
            let target_bd = t.biome.base_biodiv() * ff
                * (1.0 - t.pollution * 0.8)
                * if t.improvement == Improvement::Park { 1.2 } else { 1.0 };
            t.biodiv += (target_bd - t.biodiv) * 0.03;
        }
        self.update_biodiv_index();
    }

    pub fn season_variability(&self, y: usize) -> f32 {
        let a = lat(y, self.h).abs();
        (self.tilt / 23.5) * a * 0.5
    }
}
