# Living Worlds

*A planetary simulation where NPC civilizations rise, compete, and report back.*

Built in **Rust** with **Bevy 0.20** and **bevy_egui**. You are the planet's steward: you never control a
civilization. Continents, climate, life and — subtly — fortune are yours to shape; tribes crystallize into
civilizations that found cities, research, trade, convert, argue and go to war on their own, and the news
system explains why.

## Download (Windows)

`dist/LivingWorlds.exe` is a standalone 64-bit Windows build. Double-click to run (requires a GPU with
DirectX 12 or Vulkan support). An exported chronicle is written next to the executable.

## Build from source

```sh
cargo run --release                       # play
cargo run --release -- --batch 600 42     # headless: simulate 600 years of seed 42, print the chronicle
cargo build --release --target x86_64-pc-windows-gnu   # cross-compile the Windows exe (needs mingw-w64)
```

Linux builds need the usual Bevy system packages (`libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`).

## What's in the simulation

| System | Implementation |
| --- | --- |
| **Planet** | Cylindrical tile grid; plate-tectonic generation (convergent mountains, rifts, faults, volcanoes); land fraction, tilt, solar output, atmosphere, moons, cosmic volatility. |
| **Climate** | Coupled model: latitude + elevation lapse + solar + CO₂ greenhouse + ice albedo + particulates; ocean heat transport and westerly warming; rainfall from wind bands, upwind moisture, rain shadows and forest transpiration; flow-accumulation rivers; dynamic sea level and biomes. |
| **Biosphere** | Biodiversity per tile, forests, game, fisheries, soil depletion and recovery; one generated sapient species with traits. |
| **Civilizations** | Emerge from tribes (food surplus + good site + density + contact); generated languages, names, colors, unique abilities, visible + hidden agendas; leaders with an 8-axis personality that drifts with succession and biography (famine, betrayal). City-states with suzerains. Civil wars split civilizations into successor states; extinction leaves ruins and grievances. |
| **Cities** | Worked tiles, improvements, 23 buildings, 18 wonders (with races), governors with competence/corruption, maintenance backlog, housing, health, stability, culture borders, loyalty flips, specializations. |
| **Wealth** | The canonical 0–1000 score with the exact six weighted components (treasury & income 30%, trade 25%, resources 15%, buildings 15%, prosperity 10%, stability 5%) and six bands, shown as a breakdown bar. |
| **Technology** | 37 technologies + 17 civics across 9 eras, eurekas from real in-world actions, diffusion from contacts, environmental gating, hidden deposits revealed by technology. 10 governments with succession types. |
| **Religion** | Founded from faith; 8 themes (incl. a Shaper cult born of attunement), spread by proximity, routes and missionaries; schisms. |
| **Diplomacy** | Inspectable relationship ledger with expiring reasons, 7 relationship states, trade/research agreements, open borders, alliances, non-aggression promises tracked across witnesses, casus belli, war aims, allies joining, peace treaties with reparations, espionage with "suspected actor". |
| **War** | Armies march on the map, sieges and field battles, walls/garrison/maintenance in defense, captured or razed cities, refugees, scorched land, raiders who target rich weak cities. |
| **Events** | The full 60-event starter catalog with prerequisites, weights, cooldowns, NPC responses and the four wealth patterns (A exposure, B vulnerability with counterfactual lines, C gaps, D prizes); chains: drought → famine → exodus → relief/dust bowl, gold rush → boomtown → bust/banking, plague → peak/spread → breakthrough/scapegoat, relic → pilgrimage/theft, river shift → canal/decline, industrial accident → reform/cover-up, succession crisis → civil war/reunification, predatory loan → debt takeover. Eruptions and meteors come from the planet first. |
| **News** | Story seeds → data-driven templates (`assets/news_templates.txt`, moddable via `data/news_templates.txt` next to the exe) → tone and perspective (one war, three headlines) → significance → ticker / cards / chronicle / city gazettes; decadal reviews, half-century rankings, richest-city beat, followable threads, watchlists, auto-pause classes. Stories may only cite causes in their seed. |
| **Player** | 16 powers in 5 categories (geological, climatic, biological, subtle, disaster) paid with Gaia Points; costs scale by age; attunement makes peoples notice you; timed intervention windows with a reward for restraint; stewardship score. |
| **Modes** | Sandbox, Chronicle (hands-off), scenarios *First Rivers*, *Ice at the Door*, *Ashen Century*, *Two Continents, One Sky*, and challenge planets *Dying Sun*, *Extreme Tilt*, *Supercontinent*, *Scheduled Impact*; four difficulties. |
| **Interface** | 12 map layers + layer pairs, contextual inspector (tile / city / civilization), news cards, "Why did this happen?" panel on every story, searchable chronicle with export, diplomacy matrix with ledgers, planet graphs whose event markers open the story behind them, decadal reviews, objectives. |

Deterministic: a seed plus the intervention log reproduces a history.

## Controls

Drag to pan, scroll to zoom, WASD/arrows to pan · click a city/tile to inspect · Space pause, 1–4 speed ·
N news cards · C chronicle · right-click or Esc cancels a power · click any headline for its causes.
