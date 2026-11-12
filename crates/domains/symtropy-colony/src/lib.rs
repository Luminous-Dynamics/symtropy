// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Ant Colony V0: stigmergic foraging loop.
//!
//! The colony is the primary cognitive organism. Individual ants are lightweight
//! local inferants operating against shared fields.

use std::collections::{HashMap, VecDeque};

use symtropy_lifesim_core::{DiffusionParams, FieldGrid, FieldLayer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntRole {
    Scout,
    Forager,
}

/// Declares what information is available to colony agents.
///
/// The legacy constructor retains the oracle-assisted mode for compatibility. New
/// collective-competence comparisons should construct all worlds with an explicit mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColonyInformationMode {
    /// Legacy upper bound: agents receive hidden shortest-path hints to actual food sources
    /// and the nest, in addition to environmental pheromone fields.
    OraclePathAssisted,
    /// Decentralized stigmergic mode: agents use only locally sampled pheromone/danger
    /// gradients and direct contact with food; no path search over hidden food locations.
    StigmergicFieldOnly,
    /// Independent local-search control: no shared food/home trail emission or sensing;
    /// agents use deterministic local exploration, direct food contact, local obstacle/
    /// danger perception, and a declared nest landmark when returning.
    IndependentLocalSearch,
}

#[derive(Debug, Clone)]
pub struct AntCohort {
    pub x: usize,
    pub y: usize,
    pub role: AntRole,
    pub carrying_food: bool,
    pub energy: f32,
    pub precision: f32,
    pub exploration_bias: f32,
}

#[derive(Debug, Clone)]
pub struct ColonyMind {
    pub food_reserve: u32,
    pub scout_ratio: f32,
    pub stress: f32,
    pub trail_entropy: f32,
    pub pheromone_coherence: f32,
}

impl Default for ColonyMind {
    fn default() -> Self {
        Self {
            food_reserve: 0,
            scout_ratio: 0.25,
            stress: 0.0,
            trail_entropy: 1.0,
            pheromone_coherence: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColonyDebugMetrics {
    pub tick: u64,
    /// Explicit counter-based exploration seed. Zero preserves legacy deterministic jitter.
    pub seed: u64,
    pub information_mode: ColonyInformationMode,
    pub food_reserve: u32,
    pub trail_entropy: f32,
    pub pheromone_coherence: f32,
    pub stress: f32,
}

/// Failure to transfer only the colony's shared food/home environmental memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColonyMemoryTransferError {
    DimensionsMismatch {
        source_width: usize,
        source_height: usize,
        target_width: usize,
        target_height: usize,
    },
}

impl std::fmt::Display for ColonyMemoryTransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DimensionsMismatch {
                source_width,
                source_height,
                target_width,
                target_height,
            } => write!(
                f,
                "social-memory field dimensions differ: source {source_width}x{source_height}, target {target_width}x{target_height}"
            ),
        }
    }
}

impl std::error::Error for ColonyMemoryTransferError {}

#[derive(Debug, Clone)]
pub struct ColonyWorld {
    pub fields: FieldGrid,
    pub mind: ColonyMind,
    pub nest: (usize, usize),
    food_sources: HashMap<(usize, usize), u32>,
    ants: Vec<AntCohort>,
    information_mode: ColonyInformationMode,
    seed: u64,
    /// Whether food sources/nest emit exogenous pheromone fields each tick.
    /// Disable this when testing persistence of agent-generated environmental traces.
    source_emission_enabled: bool,
    tick: u64,
}

impl ColonyWorld {
    /// Construct the legacy oracle-assisted colony for backwards compatibility.
    ///
    /// This mode uses shortest-path information over the complete food-source map. It is
    /// an omniscient upper bound, not a decentralized stigmergic collective. Use
    /// new_with_information_mode for controlled comparisons.
    pub fn new(width: usize, height: usize, nest: (usize, usize), ant_count: usize) -> Self {
        Self::new_with_information_mode(
            width,
            height,
            nest,
            ant_count,
            ColonyInformationMode::OraclePathAssisted,
        )
    }

    /// Construct a colony with an explicit information/communication boundary.
    pub fn new_with_information_mode(
        width: usize,
        height: usize,
        nest: (usize, usize),
        ant_count: usize,
        information_mode: ColonyInformationMode,
    ) -> Self {
        Self::new_with_information_mode_and_seed(
            width,
            height,
            nest,
            ant_count,
            information_mode,
            0,
        )
    }

    /// Construct a colony with an explicit information boundary and exploration seed.
    ///
    /// Seed zero preserves the original deterministic jitter for backwards compatibility.
    /// Non-zero seeds select a counter-based SplitMix64 jitter field keyed by
    /// (seed, tick, ant_index, x, y). This provides paired, replayable runs with distinct
    /// exploration sequences without mutable RNG state in candidate scoring.
    pub fn new_with_information_mode_and_seed(
        width: usize,
        height: usize,
        nest: (usize, usize),
        ant_count: usize,
        information_mode: ColonyInformationMode,
        seed: u64,
    ) -> Self {
        let mut fields = FieldGrid::new(width, height);
        fields.set(FieldLayer::HomePheromone, nest.0, nest.1, 500.0);
        let mut ants = Vec::with_capacity(ant_count);
        for i in 0..ant_count {
            ants.push(AntCohort {
                x: nest.0,
                y: nest.1,
                role: if i % 4 == 0 {
                    AntRole::Scout
                } else {
                    AntRole::Forager
                },
                carrying_food: false,
                energy: 1.0,
                precision: 1.0,
                exploration_bias: 0.25 + (i % 7) as f32 * 0.03,
            });
        }

        Self {
            fields,
            mind: ColonyMind::default(),
            nest,
            food_sources: HashMap::new(),
            ants,
            information_mode,
            seed,
            source_emission_enabled: true,
            tick: 0,
        }
    }

    pub const fn information_mode(&self) -> ColonyInformationMode {
        self.information_mode
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub const fn source_emission_enabled(&self) -> bool {
        self.source_emission_enabled
    }

    /// Enable/disable exogenous food-source and nest pheromone emission.
    ///
    /// This is distinct from ant-generated trail deposition and is required to
    /// separate source broadcasting from environmental memory created by agents.
    pub fn set_source_emission_enabled(&mut self, enabled: bool) {
        self.source_emission_enabled = enabled;
    }

    /// Remove only the shared food/home pheromone channels, leaving danger, obstacles,
    /// nutrients, geometry and all other environmental layers unchanged.
    pub fn clear_social_memory(&mut self) {
        for y in 0..self.fields.height() {
            for x in 0..self.fields.width() {
                self.fields.set(FieldLayer::FoodPheromone, x, y, 0.0);
                self.fields.set(FieldLayer::HomePheromone, x, y, 0.0);
            }
        }
    }

    /// Copy only the food/home pheromone fields from another world.
    ///
    /// This models transfer of environmental traces to a replacement cohort. Agent
    /// state, food inventory, danger fields, obstacles and other channels are not copied.
    pub fn copy_social_memory_from(
        &mut self,
        source: &Self,
    ) -> Result<(), ColonyMemoryTransferError> {
        if self.fields.width() != source.fields.width()
            || self.fields.height() != source.fields.height()
        {
            return Err(ColonyMemoryTransferError::DimensionsMismatch {
                source_width: source.fields.width(),
                source_height: source.fields.height(),
                target_width: self.fields.width(),
                target_height: self.fields.height(),
            });
        }

        for y in 0..self.fields.height() {
            for x in 0..self.fields.width() {
                self.fields.set(
                    FieldLayer::FoodPheromone,
                    x,
                    y,
                    source.fields.get(FieldLayer::FoodPheromone, x, y),
                );
                self.fields.set(
                    FieldLayer::HomePheromone,
                    x,
                    y,
                    source.fields.get(FieldLayer::HomePheromone, x, y),
                );
            }
        }
        Ok(())
    }

    pub fn ants(&self) -> &[AntCohort] {
        &self.ants
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub fn debug_metrics(&self) -> ColonyDebugMetrics {
        ColonyDebugMetrics {
            tick: self.tick,
            seed: self.seed,
            information_mode: self.information_mode,
            food_reserve: self.mind.food_reserve,
            trail_entropy: self.mind.trail_entropy,
            pheromone_coherence: self.mind.pheromone_coherence,
            stress: self.mind.stress,
        }
    }

    pub fn add_food_source(&mut self, x: usize, y: usize, amount: u32) {
        self.food_sources.insert((x, y), amount);
    }

    pub fn set_obstacle(&mut self, x: usize, y: usize, blocked: bool) {
        self.fields
            .set(FieldLayer::Obstacle, x, y, if blocked { 1.0 } else { 0.0 });
    }

    pub fn add_danger(&mut self, x: usize, y: usize, intensity: f32) {
        self.fields
            .add(FieldLayer::DangerPheromone, x, y, intensity);
    }

    /// Couple the colony to a basin-scale substrate.
    ///
    /// Basin toxin and standing moisture become ant danger, so a colony can
    /// reroute without depending directly on `symtropy-basin` types.
    pub fn absorb_basin_fields(&mut self, basin_fields: &FieldGrid) {
        assert_eq!(
            self.fields.width(),
            basin_fields.width(),
            "field widths differ"
        );
        assert_eq!(
            self.fields.height(),
            basin_fields.height(),
            "field heights differ"
        );

        for y in 0..self.fields.height() {
            for x in 0..self.fields.width() {
                let toxin = basin_fields.get(FieldLayer::Toxin, x, y);
                let moisture = basin_fields.get(FieldLayer::Moisture, x, y);
                let flood_pressure = (moisture - 35.0).max(0.0) * 0.08;
                let danger = toxin * 0.06 + flood_pressure;
                if danger > 0.0 {
                    self.fields.add(FieldLayer::DangerPheromone, x, y, danger);
                }
            }
        }
    }

    pub fn step(&mut self) {
        self.tick += 1;
        if self.source_emission_enabled
            && self.information_mode != ColonyInformationMode::IndependentLocalSearch
        {
            self.reinforce_sources();
        }

        let information_mode = self.information_mode;
        let params = DiffusionParams {
            diffusion: 0.08,
            decay: 0.035,
            dt: 1.0,
            max_value: 1_000.0,
        };
        self.fields
            .step_diffuse_decay(FieldLayer::FoodPheromone, params);
        self.fields
            .step_diffuse_decay(FieldLayer::HomePheromone, params);
        self.fields.step_diffuse_decay(
            FieldLayer::DangerPheromone,
            DiffusionParams {
                decay: 0.02,
                ..params
            },
        );

        let mut delivered = 0;
        let width = self.fields.width();
        let height = self.fields.height();
        for i in 0..self.ants.len() {
            let target = self.next_target(i, width, height);
            let ant = &mut self.ants[i];
            ant.x = target.0;
            ant.y = target.1;

            if ant.carrying_food {
                if (ant.x, ant.y) == self.nest {
                    ant.carrying_food = false;
                    delivered += 1;
                } else if information_mode != ColonyInformationMode::IndependentLocalSearch {
                    self.fields
                        .add(FieldLayer::FoodPheromone, ant.x, ant.y, 4.0);
                }
            } else {
                if information_mode != ColonyInformationMode::IndependentLocalSearch {
                    self.fields
                        .add(FieldLayer::HomePheromone, ant.x, ant.y, 1.5);
                }
                // Food is only collected on direct cell contact. The source map is not
                // queried for guidance outside OraclePathAssisted food_score().
                if let Some(food) = self.food_sources.get_mut(&(ant.x, ant.y))
                    && *food > 0
                {
                    *food -= 1;
                    ant.carrying_food = true;
                }
            }
        }

        self.mind.food_reserve += delivered;
        self.update_mind();
    }

    pub fn run_steps(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    pub fn traffic_in_danger_region(&self, min_x: usize, max_x: usize) -> usize {
        self.ants
            .iter()
            .filter(|ant| ant.x >= min_x && ant.x <= max_x)
            .count()
    }

    /// Export a simple PPM heatmap for debug visualization.
    ///
    /// This is intentionally dependency-free. It gives tests, scripts, and early
    /// design tools a concrete way to inspect field organization before a Bevy
    /// or WGSL viewer exists.
    pub fn to_ppm_heatmap(&self) -> String {
        let width = self.fields.width();
        let height = self.fields.height();
        let max_food = self.fields.channel_sum(FieldLayer::FoodPheromone).max(1.0);
        let mut out = format!(
            "P3\n# symtropy-colony tick {}\n{} {}\n255\n",
            self.tick, width, height
        );

        for y in 0..height {
            for x in 0..width {
                let (r, g, b) = self.debug_pixel(x, y, max_food);
                out.push_str(&format!("{r} {g} {b} "));
            }
            out.push('\n');
        }

        out
    }

    fn debug_pixel(&self, x: usize, y: usize, max_food: f32) -> (u8, u8, u8) {
        if self.fields.is_obstacle(x, y) {
            return (12, 12, 12);
        }
        if (x, y) == self.nest {
            return (70, 130, 255);
        }
        if self.food_sources.get(&(x, y)).copied().unwrap_or(0) > 0 {
            return (40, 220, 80);
        }
        if self.ants.iter().any(|ant| ant.x == x && ant.y == y) {
            return (255, 240, 120);
        }

        let danger = self
            .fields
            .get(FieldLayer::DangerPheromone, x, y)
            .min(255.0) as u8;
        let food = self.fields.get(FieldLayer::FoodPheromone, x, y);
        let food_norm = ((food / max_food.sqrt()).sqrt() * 255.0).clamp(0.0, 255.0) as u8;
        let home = self.fields.get(FieldLayer::HomePheromone, x, y).min(255.0) as u8;

        (danger.max(food_norm), home / 3, food_norm)
    }

    fn reinforce_sources(&mut self) {
        let (nx, ny) = self.nest;
        self.fields.add(FieldLayer::HomePheromone, nx, ny, 30.0);
        for (&(x, y), &amount) in &self.food_sources {
            if amount > 0 {
                self.fields.add(FieldLayer::FoodPheromone, x, y, 35.0);
            }
        }
    }

    fn next_target(&self, ant_index: usize, width: usize, height: usize) -> (usize, usize) {
        let ant = &self.ants[ant_index];
        let options = self.neighbors(ant.x, ant.y, width, height);
        let mut best = (ant.x, ant.y);
        let mut best_score = f32::MIN;

        for (x, y) in options {
            if self.fields.is_obstacle(x, y) {
                continue;
            }

            let danger = self.fields.get(FieldLayer::DangerPheromone, x, y);
            let target_score = if ant.carrying_food {
                self.home_score(x, y)
            } else {
                self.food_score(x, y, ant.role, ant_index)
            };
            let score = target_score - danger * 4.0;
            if score > best_score {
                best_score = score;
                best = (x, y);
            }
        }

        best
    }

    fn food_score(&self, x: usize, y: usize, role: AntRole, ant_index: usize) -> f32 {
        let field = match self.information_mode {
            ColonyInformationMode::OraclePathAssisted
            | ColonyInformationMode::StigmergicFieldOnly => {
                self.fields.get(FieldLayer::FoodPheromone, x, y)
            }
            ColonyInformationMode::IndependentLocalSearch => 0.0,
        };

        let mut score = field;
        if self.information_mode == ColonyInformationMode::OraclePathAssisted {
            if let Some(path) = self.shortest_path_distance((x, y), |p| {
                self.food_sources.get(&p).copied().unwrap_or(0) > 0
            }) {
                score += 160.0 / (path as f32 + 1.0);
            }
        }

        let explore = match self.information_mode {
            ColonyInformationMode::OraclePathAssisted
            | ColonyInformationMode::StigmergicFieldOnly => role == AntRole::Scout,
            ColonyInformationMode::IndependentLocalSearch => true,
        };
        if explore {
            let scale = if self.information_mode == ColonyInformationMode::IndependentLocalSearch {
                1.0
            } else {
                0.1
            };
            score += self.deterministic_jitter(x, y, ant_index) * scale;
        }
        score
    }

    fn home_score(&self, x: usize, y: usize) -> f32 {
        let home_field = match self.information_mode {
            ColonyInformationMode::OraclePathAssisted
            | ColonyInformationMode::StigmergicFieldOnly => {
                self.fields.get(FieldLayer::HomePheromone, x, y)
            }
            ColonyInformationMode::IndependentLocalSearch => 0.0,
        };
        let distance = x.abs_diff(self.nest.0) + y.abs_diff(self.nest.1);
        match self.information_mode {
            ColonyInformationMode::OraclePathAssisted => {
                let path_bonus = self
                    .shortest_path_distance((x, y), |p| p == self.nest)
                    .map(|path| 180.0 / (path as f32 + 1.0))
                    .unwrap_or(0.0);
                home_field + path_bonus + 20.0 / (distance as f32 + 1.0)
            }
            ColonyInformationMode::StigmergicFieldOnly => home_field,
            ColonyInformationMode::IndependentLocalSearch => -(distance as f32),
        }
    }

    fn update_mind(&mut self) {
        let total = self.fields.channel_sum(FieldLayer::FoodPheromone);
        let active = self.count_active_food_cells();
        self.mind.trail_entropy = if total <= f32::EPSILON {
            1.0
        } else {
            (active as f32 / (self.fields.width() * self.fields.height()) as f32).min(1.0)
        };

        self.mind.pheromone_coherence = if total <= f32::EPSILON {
            0.0
        } else {
            self.strongest_food_corridor_sum() / total
        };

        self.mind.stress = self.fields.channel_sum(FieldLayer::DangerPheromone)
            / (self.fields.width() * self.fields.height()) as f32;
        self.mind.scout_ratio = if self.mind.trail_entropy > 0.35 || self.mind.stress > 0.5 {
            0.45
        } else {
            0.20
        };
    }

    fn count_active_food_cells(&self) -> usize {
        let mut count = 0;
        for y in 0..self.fields.height() {
            for x in 0..self.fields.width() {
                if self.fields.get(FieldLayer::FoodPheromone, x, y) > 1.0 {
                    count += 1;
                }
            }
        }
        count
    }

    fn strongest_food_corridor_sum(&self) -> f32 {
        let Some((&food_pos, _)) = self.food_sources.iter().find(|(_, amount)| **amount > 0) else {
            return 0.0;
        };
        let Some(path) = self.shortest_path(food_pos, self.nest) else {
            return 0.0;
        };
        path.into_iter()
            .map(|(x, y)| self.fields.get(FieldLayer::FoodPheromone, x, y))
            .sum()
    }

    fn shortest_path_distance<F>(&self, start: (usize, usize), goal: F) -> Option<usize>
    where
        F: Fn((usize, usize)) -> bool,
    {
        self.shortest_path_to(start, goal).map(|path| path.len())
    }

    fn shortest_path(
        &self,
        start: (usize, usize),
        goal: (usize, usize),
    ) -> Option<Vec<(usize, usize)>> {
        self.shortest_path_to(start, |p| p == goal)
    }

    fn shortest_path_to<F>(&self, start: (usize, usize), goal: F) -> Option<Vec<(usize, usize)>>
    where
        F: Fn((usize, usize)) -> bool,
    {
        let width = self.fields.width();
        let height = self.fields.height();
        let mut visited = vec![false; width * height];
        let mut prev: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut queue = VecDeque::from([start]);
        visited[self.fields.idx(start.0, start.1)] = true;

        while let Some(pos) = queue.pop_front() {
            if goal(pos) {
                let mut path = vec![pos];
                let mut cursor = pos;
                while let Some(&p) = prev.get(&cursor) {
                    path.push(p);
                    cursor = p;
                }
                path.reverse();
                return Some(path);
            }

            for next in self.neighbors(pos.0, pos.1, width, height) {
                if self.fields.is_obstacle(next.0, next.1) {
                    continue;
                }
                let idx = self.fields.idx(next.0, next.1);
                if !visited[idx] {
                    visited[idx] = true;
                    prev.insert(next, pos);
                    queue.push_back(next);
                }
            }
        }

        None
    }

    fn neighbors(&self, x: usize, y: usize, width: usize, height: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::with_capacity(5);
        out.push((x, y));
        if x > 0 {
            out.push((x - 1, y));
        }
        if x + 1 < width {
            out.push((x + 1, y));
        }
        if y > 0 {
            out.push((x, y - 1));
        }
        if y + 1 < height {
            out.push((x, y + 1));
        }
        out
    }

    fn deterministic_jitter(&self, x: usize, y: usize, ant_index: usize) -> f32 {
        let key = (x as u64 * 73_856_093)
            ^ (y as u64 * 19_349_663)
            ^ (ant_index as u64 * 83_492_791)
            ^ self.tick;
        if self.seed == 0 {
            // Preserve the historical deterministic jitter for existing callers.
            return (key % 1_000) as f32 / 1_000.0;
        }

        // Counter-based SplitMix64: no mutable RNG state is consumed while scoring
        // candidate cells, so candidate enumeration order cannot perturb future draws.
        let mixed = splitmix64(key ^ self.seed);
        (mixed >> 40) as f32 / 16_777_216.0
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nest_discovers_food_forms_trail_and_increases_reserve() {
        let mut world = ColonyWorld::new(32, 16, (2, 8), 64);
        world.add_food_source(28, 8, 5_000);

        world.run_steps(180);

        assert!(world.mind.food_reserve > 20);
        assert!(world.fields.channel_sum(FieldLayer::FoodPheromone) > 100.0);
        assert!(world.mind.pheromone_coherence > 0.12);
    }

    #[test]
    fn broken_trail_increases_entropy_and_recovers_via_new_path() {
        let mut world = ColonyWorld::new(32, 16, (2, 8), 64);
        world.add_food_source(28, 8, 5_000);
        world.run_steps(120);
        let baseline_reserve = world.mind.food_reserve;

        for y in 1..15 {
            if y != 3 {
                world.set_obstacle(15, y, true);
            }
        }
        world.run_steps(120);

        assert!(world.mind.food_reserve > baseline_reserve);
        assert!(world.mind.scout_ratio >= 0.20);
        assert!(world.mind.pheromone_coherence > 0.08);
    }

    #[test]
    fn danger_pheromone_reroutes_foragers() {
        let mut world = ColonyWorld::new(32, 16, (2, 8), 64);
        world.add_food_source(28, 8, 5_000);
        for y in 0..16 {
            world.add_danger(14, y, 200.0);
            world.add_danger(15, y, 200.0);
            world.add_danger(16, y, 200.0);
        }

        world.run_steps(100);

        assert!(world.traffic_in_danger_region(14, 16) < 10);
    }

    #[test]
    fn ppm_heatmap_exports_visible_colony_state() {
        let mut world = ColonyWorld::new(8, 4, (1, 2), 8);
        world.add_food_source(6, 2, 100);
        world.set_obstacle(3, 1, true);
        world.add_danger(4, 1, 200.0);
        world.run_steps(4);

        let ppm = world.to_ppm_heatmap();

        assert!(ppm.starts_with("P3\n# symtropy-colony tick 4\n8 4\n255\n"));
        assert!(ppm.contains("70 130 255"));
        assert!(ppm.contains("40 220 80"));
        assert!(ppm.contains("12 12 12"));
    }

    #[test]
    fn social_memory_transfer_copies_only_food_and_home_fields() {
        let mut donor = ColonyWorld::new_with_information_mode_and_seed(
            12,
            8,
            (1, 4),
            4,
            ColonyInformationMode::StigmergicFieldOnly,
            7,
        );
        donor.fields.set(FieldLayer::FoodPheromone, 3, 4, 17.0);
        donor.fields.set(FieldLayer::HomePheromone, 2, 4, 9.0);
        donor.fields.set(FieldLayer::DangerPheromone, 3, 4, 5.0);
        donor.fields.set(FieldLayer::Nutrient, 3, 4, 11.0);

        let mut recipient = ColonyWorld::new_with_information_mode_and_seed(
            12,
            8,
            (1, 4),
            4,
            ColonyInformationMode::StigmergicFieldOnly,
            8,
        );
        recipient.fields.set(FieldLayer::DangerPheromone, 3, 4, 2.0);
        recipient.copy_social_memory_from(&donor).unwrap();

        assert_eq!(recipient.fields.get(FieldLayer::FoodPheromone, 3, 4), 17.0);
        assert_eq!(recipient.fields.get(FieldLayer::HomePheromone, 2, 4), 9.0);
        assert_eq!(recipient.fields.get(FieldLayer::DangerPheromone, 3, 4), 2.0);
        assert_eq!(recipient.fields.get(FieldLayer::Nutrient, 3, 4), 0.0);
        assert_eq!(recipient.ants().len(), 4);
        assert_eq!(recipient.mind.food_reserve, 0);
    }

    #[test]
    fn replacement_cohort_uses_transferred_trace_without_source_broadcast() {
        let mut donor = ColonyWorld::new_with_information_mode_and_seed(
            12,
            8,
            (1, 4),
            4,
            ColonyInformationMode::StigmergicFieldOnly,
            7,
        );
        donor.fields.set(FieldLayer::FoodPheromone, 2, 4, 1.0);
        donor.fields.set(FieldLayer::FoodPheromone, 3, 4, 30.0);
        donor.fields.set(FieldLayer::HomePheromone, 1, 4, 30.0);

        let mut replacement = ColonyWorld::new_with_information_mode_and_seed(
            12,
            8,
            (1, 4),
            4,
            ColonyInformationMode::StigmergicFieldOnly,
            19,
        );
        replacement.set_source_emission_enabled(false);
        replacement.copy_social_memory_from(&donor).unwrap();
        let target_with_memory = replacement.next_target(1, 12, 8);

        let mut erased = ColonyWorld::new_with_information_mode_and_seed(
            12,
            8,
            (1, 4),
            4,
            ColonyInformationMode::StigmergicFieldOnly,
            19,
        );
        erased.set_source_emission_enabled(false);
        erased.clear_social_memory();
        let target_without_memory = erased.next_target(1, 12, 8);

        assert_eq!(target_with_memory, (2, 4));
        assert_ne!(target_with_memory, target_without_memory);
        assert!(!replacement.source_emission_enabled());
        assert_eq!(replacement.tick(), 0);
    }

    #[test]
    fn exogenous_common_field_control_can_reproduce_a_transferred_field_decision() {
        let mut donor = ColonyWorld::new_with_information_mode(
            12,
            8,
            (1, 4),
            4,
            ColonyInformationMode::StigmergicFieldOnly,
        );
        donor.fields.set(FieldLayer::FoodPheromone, 2, 4, 1.0);
        donor.fields.set(FieldLayer::FoodPheromone, 3, 4, 30.0);

        let mut transferred = ColonyWorld::new_with_information_mode(
            12,
            8,
            (1, 4),
            4,
            ColonyInformationMode::StigmergicFieldOnly,
        );
        transferred.set_source_emission_enabled(false);
        transferred.copy_social_memory_from(&donor).unwrap();

        let mut externally_signaled = ColonyWorld::new_with_information_mode(
            12,
            8,
            (1, 4),
            4,
            ColonyInformationMode::StigmergicFieldOnly,
        );
        externally_signaled.set_source_emission_enabled(false);
        externally_signaled.fields.set(FieldLayer::FoodPheromone, 2, 4, 1.0);
        externally_signaled.fields.set(FieldLayer::FoodPheromone, 3, 4, 30.0);

        assert_eq!(
            transferred.next_target(1, 12, 8),
            externally_signaled.next_target(1, 12, 8)
        );
    }

    #[test]
    fn social_memory_transfer_rejects_different_grid_dimensions() {
        let donor = ColonyWorld::new(12, 8, (1, 4), 4);
        let mut recipient = ColonyWorld::new(10, 8, (1, 4), 4);

        assert_eq!(
            recipient.copy_social_memory_from(&donor),
            Err(ColonyMemoryTransferError::DimensionsMismatch {
                source_width: 12,
                source_height: 8,
                target_width: 10,
                target_height: 8,
            })
        );
    }

    #[test]
    fn seeded_exploration_is_replayable_and_varies_across_seeds() {
        let make = |seed| {
            ColonyWorld::new_with_information_mode_and_seed(
                16,
                12,
                (2, 6),
                8,
                ColonyInformationMode::IndependentLocalSearch,
                seed,
            )
        };
        let left = make(101);
        let replay = make(101);
        let other = make(202);

        assert_eq!(left.seed(), 101);
        assert_eq!(left.debug_metrics().seed, 101);
        assert_eq!(
            left.deterministic_jitter(4, 5, 3),
            replay.deterministic_jitter(4, 5, 3)
        );
        assert_ne!(
            left.deterministic_jitter(4, 5, 3),
            other.deterministic_jitter(4, 5, 3)
        );
    }

    #[test]
    fn seed_zero_preserves_legacy_jitter_formula() {
        let world = ColonyWorld::new_with_information_mode(
            16,
            12,
            (2, 6),
            8,
            ColonyInformationMode::IndependentLocalSearch,
        );
        let x = 4usize;
        let y = 5usize;
        let ant_index = 3usize;
        let expected_key = (x as u64 * 73_856_093)
            ^ (y as u64 * 19_349_663)
            ^ (ant_index as u64 * 83_492_791)
            ^ world.tick();
        let expected = (expected_key % 1_000) as f32 / 1_000.0;
        assert_eq!(world.seed(), 0);
        assert_eq!(world.deterministic_jitter(x, y, ant_index), expected);
    }

    #[test]
    fn legacy_constructor_is_explicitly_tagged_as_oracle_assisted() {
        let world = ColonyWorld::new(12, 8, (1, 4), 8);
        assert_eq!(
            world.information_mode(),
            ColonyInformationMode::OraclePathAssisted
        );
        assert_eq!(
            world.debug_metrics().information_mode,
            ColonyInformationMode::OraclePathAssisted
        );
    }

    #[test]
    fn stigmergic_food_score_does_not_depend_on_hidden_food_coordinates() {
        let mut left = ColonyWorld::new_with_information_mode(
            16, 8, (1, 4), 8, ColonyInformationMode::StigmergicFieldOnly,
        );
        let mut right = ColonyWorld::new_with_information_mode(
            16, 8, (1, 4), 8, ColonyInformationMode::StigmergicFieldOnly,
        );
        left.add_food_source(14, 4, 100);
        right.add_food_source(8, 1, 100);
        left.fields.set(FieldLayer::FoodPheromone, 3, 4, 0.0);
        right.fields.set(FieldLayer::FoodPheromone, 3, 4, 0.0);

        let left_score = left.food_score(3, 4, AntRole::Forager, 1);
        let right_score = right.food_score(3, 4, AntRole::Forager, 1);

        assert_eq!(left_score, right_score);
        assert_eq!(left_score, 0.0);
    }

    #[test]
    fn stigmergic_home_score_does_not_depend_on_hidden_nest_coordinates() {
        let mut left = ColonyWorld::new_with_information_mode(
            16, 8, (1, 4), 8, ColonyInformationMode::StigmergicFieldOnly,
        );
        let mut right = ColonyWorld::new_with_information_mode(
            16, 8, (14, 4), 8, ColonyInformationMode::StigmergicFieldOnly,
        );
        left.fields.set(FieldLayer::HomePheromone, 5, 4, 12.0);
        right.fields.set(FieldLayer::HomePheromone, 5, 4, 12.0);

        assert_eq!(left.home_score(5, 4), right.home_score(5, 4));
        assert_eq!(left.home_score(5, 4), 12.0);
    }

    #[test]
    fn oracle_mode_retains_global_path_hint_as_an_upper_bound() {
        let mut world = ColonyWorld::new(16, 8, (1, 4), 8);
        world.add_food_source(14, 4, 100);
        world.fields.set(FieldLayer::FoodPheromone, 3, 4, 0.0);

        let score = world.food_score(3, 4, AntRole::Forager, 1);
        assert!(score > 0.0);
    }

    #[test]
    fn independent_local_search_ignores_pheromone_fields() {
        let mut left = ColonyWorld::new_with_information_mode(
            16, 8, (1, 4), 8, ColonyInformationMode::IndependentLocalSearch,
        );
        let mut right = left.clone();
        left.fields.set(FieldLayer::FoodPheromone, 2, 4, 100.0);
        left.fields.set(FieldLayer::HomePheromone, 2, 4, 100.0);
        right.fields.set(FieldLayer::FoodPheromone, 2, 4, 0.0);
        right.fields.set(FieldLayer::HomePheromone, 2, 4, 0.0);

        assert_eq!(
            left.food_score(2, 4, AntRole::Forager, 3),
            right.food_score(2, 4, AntRole::Forager, 3)
        );
        assert_eq!(left.home_score(2, 4), right.home_score(2, 4));
    }

    #[test]
    fn independent_local_search_does_not_emit_shared_pheromones() {
        let mut with_agents = ColonyWorld::new_with_information_mode(
            16, 8, (1, 4), 8, ColonyInformationMode::IndependentLocalSearch,
        );
        let mut no_agents = ColonyWorld::new_with_information_mode(
            16, 8, (1, 4), 0, ColonyInformationMode::IndependentLocalSearch,
        );
        with_agents.add_food_source(14, 4, 100);
        no_agents.add_food_source(14, 4, 100);

        with_agents.run_steps(10);
        no_agents.run_steps(10);

        assert_eq!(
            with_agents.fields.max_abs_diff(&no_agents.fields, FieldLayer::FoodPheromone),
            0.0
        );
        assert_eq!(
            with_agents.fields.max_abs_diff(&no_agents.fields, FieldLayer::HomePheromone),
            0.0
        );
    }

    #[test]
    fn debug_metrics_track_colony_state() {
        let mut world = ColonyWorld::new(16, 8, (1, 4), 16);
        world.add_food_source(14, 4, 500);
        world.run_steps(20);

        let metrics = world.debug_metrics();

        assert_eq!(metrics.tick, 20);
        assert_eq!(metrics.food_reserve, world.mind.food_reserve);
        assert!(metrics.pheromone_coherence >= 0.0);
    }

    #[test]
    fn basin_toxin_and_moisture_raise_colony_danger() {
        let mut world = ColonyWorld::new(8, 5, (1, 2), 8);
        let mut basin = FieldGrid::new(8, 5);
        basin.set(FieldLayer::Toxin, 4, 2, 100.0);
        basin.set(FieldLayer::Moisture, 4, 2, 60.0);

        world.absorb_basin_fields(&basin);

        assert!(world.fields.get(FieldLayer::DangerPheromone, 4, 2) > 7.0);
        assert_eq!(world.fields.get(FieldLayer::DangerPheromone, 1, 2), 0.0);
    }
}
