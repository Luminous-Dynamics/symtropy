// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Mycelial Network V0: field-based growth, transport, and toxin avoidance.
//!
//! This crate is deliberately CPU-first. It proves that `symtropy-lifesim-core`
//! is not ant-specific by reusing the same dense field substrate for a sessile,
//! graph-like living system.
//!
//! The explicit topology-only graph reference is available in [`hyphal_graph`].
//! It is intentionally separate from the legacy field-growth model below.

pub mod bmx_exchange_reference;
pub mod crossing_qualification;
pub mod cytoplasmic_flow;
pub mod hyphal_graph;
pub mod hyphal_highway;
pub mod hyphal_spectroscopy;
pub mod isotope_transfer;
pub mod legacy_accounting;
pub mod multimodal_imaging;
pub mod network_evidence;
pub mod observation_evidence;
pub mod pareto_front;
pub mod root_anatomy_energy;
pub mod spatial_field_evidence;
pub mod study_comparability;
pub mod symbiosis_checkpoints;
pub mod temporal_alignment;
pub mod transport;

use std::collections::{HashMap, VecDeque};

use symtropy_lifesim_core::{DiffusionParams, FieldGrid, FieldLayer};

use crate::legacy_accounting::LegacyMyceliumStepReport;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MyceliumMetrics {
    pub tick: u64,
    pub central_biomass: f32,
    pub total_biomass: f32,
    pub total_nutrient: f32,
    pub total_toxin: f32,
}

#[derive(Debug, Clone)]
pub struct MyceliumWorld {
    pub fields: FieldGrid,
    pub origin: (usize, usize),
    tick: u64,
}

impl MyceliumWorld {
    pub fn new(width: usize, height: usize, origin: (usize, usize)) -> Self {
        let mut fields = FieldGrid::new(width, height);
        fields.set(FieldLayer::Biomass, origin.0, origin.1, 8.0);
        Self {
            fields,
            origin,
            tick: 0,
        }
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub fn add_dead_biomass(&mut self, x: usize, y: usize, nutrients: f32) {
        self.fields.add(FieldLayer::Nutrient, x, y, nutrients);
    }

    pub fn add_toxin(&mut self, x: usize, y: usize, intensity: f32) {
        self.fields.add(FieldLayer::Toxin, x, y, intensity);
    }

    /// Couple the network to a basin-scale substrate without depending on the
    /// basin crate. Basin nutrients feed growth; basin toxins suppress it.
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
                self.fields.add(
                    FieldLayer::Nutrient,
                    x,
                    y,
                    basin_fields.get(FieldLayer::Nutrient, x, y) * 0.20,
                );
                self.fields.add(
                    FieldLayer::Toxin,
                    x,
                    y,
                    basin_fields.get(FieldLayer::Toxin, x, y) * 0.12,
                );
            }
        }
    }

    /// Let established biomass buffer basin toxins and return nutrients.
    pub fn buffer_basin_fields(&self, basin_fields: &mut FieldGrid) {
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
                let biomass = self.fields.get(FieldLayer::Biomass, x, y);
                if biomass <= 0.0 {
                    continue;
                }

                let toxin = basin_fields.get(FieldLayer::Toxin, x, y);
                let buffered = (biomass * 0.05).min(toxin);
                basin_fields.set(FieldLayer::Toxin, x, y, toxin - buffered);
                basin_fields.add(FieldLayer::Nutrient, x, y, buffered * 0.35);
            }
        }
    }

    pub fn biomass_at(&self, x: usize, y: usize) -> f32 {
        self.fields.get(FieldLayer::Biomass, x, y)
    }

    pub fn metrics(&self) -> MyceliumMetrics {
        MyceliumMetrics {
            tick: self.tick,
            central_biomass: self.biomass_at(self.origin.0, self.origin.1),
            total_biomass: self.fields.channel_sum(FieldLayer::Biomass),
            total_nutrient: self.fields.channel_sum(FieldLayer::Nutrient),
            total_toxin: self.fields.channel_sum(FieldLayer::Toxin),
        }
    }

    /// Export a simple PPM heatmap for debug visualization.
    ///
    /// Biomass is green, nutrient is blue, toxin is red, and the origin is
    /// highlighted. This mirrors the colony debug export so early viewers can
    /// consume both organisms without a rendering dependency.
    pub fn to_ppm_heatmap(&self) -> String {
        let width = self.fields.width();
        let height = self.fields.height();
        let max_biomass = self.fields.channel_sum(FieldLayer::Biomass).max(1.0);
        let max_nutrient = self.fields.channel_sum(FieldLayer::Nutrient).max(1.0);
        let max_toxin = self.fields.channel_sum(FieldLayer::Toxin).max(1.0);
        let mut out = format!(
            "P3\n# symtropy-mycelium tick {}\n{} {}\n255\n",
            self.tick, width, height
        );

        for y in 0..height {
            for x in 0..width {
                let (r, g, b) = self.debug_pixel(x, y, max_biomass, max_nutrient, max_toxin);
                out.push_str(&format!("{r} {g} {b} "));
            }
            out.push('\n');
        }

        out
    }

    pub fn run_steps(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    pub fn step(&mut self) {
        let _ = self.step_internal(false);
    }

    /// Execute the legacy step and collect a model-unit accounting report.
    ///
    /// This diagnostic path does not convert legacy heuristics into physical
    /// units or claim mass conservation. The ordinary `step()` path avoids
    /// the extra full-field summations required only for reporting.
    pub fn step_with_report(&mut self) -> LegacyMyceliumStepReport {
        self.step_internal(true)
            .expect("report collection is enabled for step_with_report")
    }

    fn step_internal(&mut self, collect_report: bool) -> Option<LegacyMyceliumStepReport> {
        let tick_before = self.tick;
        let totals_before = if collect_report {
            Some((
                self.total_layer(FieldLayer::Biomass),
                self.total_layer(FieldLayer::Nutrient),
                self.total_layer(FieldLayer::Toxin),
            ))
        } else {
            None
        };

        self.tick += 1;
        self.fields.step_diffuse_decay(
            FieldLayer::Nutrient,
            DiffusionParams {
                diffusion: 0.12,
                decay: 0.005,
                dt: 1.0,
                max_value: 1_000.0,
            },
        );
        self.fields.step_diffuse_decay(
            FieldLayer::Toxin,
            DiffusionParams {
                diffusion: 0.05,
                decay: 0.01,
                dt: 1.0,
                max_value: 1_000.0,
            },
        );

        let totals_after_diffusion = if collect_report {
            Some((
                self.total_layer(FieldLayer::Nutrient),
                self.total_layer(FieldLayer::Toxin),
            ))
        } else {
            None
        };
        let mut biomass_delta = HashMap::<(usize, usize), f32>::new();
        let mut nutrient_delta = HashMap::<(usize, usize), f32>::new();
        let mut transported_to_origin = 0.0;
        let mut assimilation_total = 0.0;
        let mut growth_target_biomass_credit = 0.0;
        let mut growth_source_biomass_debit = 0.0;
        let mut growth_nutrient_debit = 0.0;

        // The field layers are not mutated until all deltas are applied below.
        // Cache the global nutrient peak once instead of rescanning the whole
        // grid for every biomass-bearing cell.
        let nutrient_peak = self.strongest_nutrient_cell();
        for y in 0..self.fields.height() {
            for x in 0..self.fields.width() {
                let biomass = self.fields.get(FieldLayer::Biomass, x, y);
                if biomass < 0.25 {
                    continue;
                }

                let local_nutrient = self.fields.get(FieldLayer::Nutrient, x, y);
                let local_toxin = self.fields.get(FieldLayer::Toxin, x, y);
                let assimilation = (local_nutrient * 0.08).min(biomass * 0.04).max(0.0);
                if local_toxin < 5.0 && assimilation > 0.0 {
                    *biomass_delta.entry((x, y)).or_default() += assimilation;
                    *nutrient_delta.entry((x, y)).or_default() -= assimilation;
                    transported_to_origin += assimilation * 0.25;
                    if collect_report {
                        assimilation_total += assimilation;
                    }
                }

                if biomass < 0.05 {
                    continue;
                }

                let Some(target) = self.best_growth_neighbor(x, y, nutrient_peak) else {
                    continue;
                };
                let toxin = self.fields.get(FieldLayer::Toxin, target.0, target.1);
                if toxin >= 20.0 {
                    continue;
                }
                let nutrient = self.fields.get(FieldLayer::Nutrient, target.0, target.1);
                let growth = (0.08 + nutrient * 0.06).min(biomass * 0.25);
                if growth > 0.0 {
                    *biomass_delta.entry(target).or_default() += growth;
                    *biomass_delta.entry((x, y)).or_default() -= growth * 0.05;
                    *nutrient_delta.entry(target).or_default() -= growth * 0.35;
                    if collect_report {
                        growth_target_biomass_credit += growth;
                        growth_source_biomass_debit += growth * 0.05;
                        growth_nutrient_debit += growth * 0.35;
                    }
                }
            }
        }

        for ((x, y), delta) in biomass_delta {
            self.fields.add(FieldLayer::Biomass, x, y, delta);
        }
        for ((x, y), delta) in nutrient_delta {
            self.fields.add(FieldLayer::Nutrient, x, y, delta);
        }
        self.fields.add(
            FieldLayer::Biomass,
            self.origin.0,
            self.origin.1,
            transported_to_origin,
        );

        let (biomass_before_decay, nutrient_after_step) = if collect_report {
            (
                Some(self.total_layer(FieldLayer::Biomass)),
                Some(self.total_layer(FieldLayer::Nutrient)),
            )
        } else {
            (None, None)
        };

        self.fields.step_diffuse_decay(
            FieldLayer::Biomass,
            DiffusionParams {
                diffusion: 0.0,
                decay: 0.002,
                dt: 1.0,
                max_value: 1_000.0,
            },
        );

        if !collect_report {
            return None;
        }

        let biomass_after_decay = self.total_layer(FieldLayer::Biomass);
        let (biomass_before, nutrient_before, toxin_before) =
            totals_before.expect("report collection captured initial totals");
        let (nutrient_after_diffusion_decay, toxin_after_diffusion_decay) =
            totals_after_diffusion.expect("report collection captured diffusion totals");
        let biomass_before_decay =
            biomass_before_decay.expect("report collection captured pre-decay biomass");
        let nutrient_after_step =
            nutrient_after_step.expect("report collection captured post-step nutrient");

        Some(LegacyMyceliumStepReport {
            tick_before,
            tick_after: self.tick,
            biomass_before,
            biomass_before_decay,
            biomass_after_decay,
            nutrient_before,
            nutrient_after_diffusion_decay,
            nutrient_after_step,
            toxin_before,
            toxin_after_diffusion_decay,
            assimilation_nutrient_debit: assimilation_total,
            assimilation_local_biomass_credit: assimilation_total,
            origin_biomass_extra_credit: transported_to_origin,
            growth_target_biomass_credit,
            growth_source_biomass_debit,
            growth_nutrient_debit,
            observed_biomass_decay_loss: biomass_before_decay - biomass_after_decay,
        })
    }

    fn total_layer(&self, layer: FieldLayer) -> f32 {
        let mut total = 0.0;
        for y in 0..self.fields.height() {
            for x in 0..self.fields.width() {
                total += self.fields.get(layer, x, y);
            }
        }
        total
    }

    pub fn has_biomass_path_to(&self, goal: (usize, usize), threshold: f32) -> bool {
        if self.biomass_at(self.origin.0, self.origin.1) < threshold {
            return false;
        }
        let mut visited = vec![false; self.fields.width() * self.fields.height()];
        let mut queue = VecDeque::from([self.origin]);
        visited[self.fields.idx(self.origin.0, self.origin.1)] = true;

        while let Some(pos) = queue.pop_front() {
            if pos == goal {
                return true;
            }
            for next in self.neighbors(pos.0, pos.1) {
                let idx = self.fields.idx(next.0, next.1);
                if visited[idx] || self.biomass_at(next.0, next.1) < threshold {
                    continue;
                }
                visited[idx] = true;
                queue.push_back(next);
            }
        }

        false
    }

    fn debug_pixel(
        &self,
        x: usize,
        y: usize,
        max_biomass: f32,
        max_nutrient: f32,
        max_toxin: f32,
    ) -> (u8, u8, u8) {
        if (x, y) == self.origin {
            return (240, 240, 120);
        }

        let biomass = self.fields.get(FieldLayer::Biomass, x, y);
        let nutrient = self.fields.get(FieldLayer::Nutrient, x, y);
        let toxin = self.fields.get(FieldLayer::Toxin, x, y);

        let r = ((toxin / max_toxin.sqrt()).sqrt() * 255.0).clamp(0.0, 255.0) as u8;
        let g = ((biomass / max_biomass.sqrt()).sqrt() * 255.0).clamp(0.0, 255.0) as u8;
        let b = ((nutrient / max_nutrient.sqrt()).sqrt() * 255.0).clamp(0.0, 255.0) as u8;

        (r, g, b)
    }

    fn best_growth_neighbor(
        &self,
        x: usize,
        y: usize,
        nutrient_peak: Option<(usize, usize)>,
    ) -> Option<(usize, usize)> {
        self.neighbors(x, y).max_by(|a, b| {
            self.growth_score(*a, nutrient_peak)
                .partial_cmp(&self.growth_score(*b, nutrient_peak))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    fn growth_score(&self, pos: (usize, usize), nutrient_peak: Option<(usize, usize)>) -> f32 {
        let nutrient = self.fields.get(FieldLayer::Nutrient, pos.0, pos.1);
        let toxin = self.fields.get(FieldLayer::Toxin, pos.0, pos.1);
        let existing = self.fields.get(FieldLayer::Biomass, pos.0, pos.1);
        let peak_pull = nutrient_peak
            .map(|peak| {
                let distance = pos.0.abs_diff(peak.0) + pos.1.abs_diff(peak.1);
                12.0 / (distance as f32 + 1.0)
            })
            .unwrap_or(0.0);
        nutrient * 2.0 + peak_pull - existing * 0.05 - toxin * 4.0
    }

    fn strongest_nutrient_cell(&self) -> Option<(usize, usize)> {
        let mut best = None;
        let mut best_value = 0.0;
        for y in 0..self.fields.height() {
            for x in 0..self.fields.width() {
                let value = self.fields.get(FieldLayer::Nutrient, x, y);
                if value > best_value {
                    best_value = value;
                    best = Some((x, y));
                }
            }
        }
        best
    }

    fn neighbors(&self, x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> {
        let left = if x > 0 { Some((x - 1, y)) } else { None };
        let right = (x + 1 < self.fields.width()).then_some((x + 1, y));
        let up = if y > 0 { Some((x, y - 1)) } else { None };
        let down = (y + 1 < self.fields.height()).then_some((x, y + 1));
        [left, right, up, down].into_iter().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighbor_iteration_preserves_order_without_heap_collection() {
        let world = MyceliumWorld::new(3, 3, (1, 1));
        assert_eq!(
            world.neighbors(1, 1).collect::<Vec<_>>(),
            vec![(0, 1), (2, 1), (1, 0), (1, 2)]
        );
        assert_eq!(
            world.neighbors(0, 0).collect::<Vec<_>>(),
            vec![(1, 0), (0, 1)]
        );
    }

    #[test]
    fn strongest_nutrient_peak_is_stable_before_deferred_field_updates() {
        let mut world = MyceliumWorld::new(4, 3, (0, 1));
        world.add_dead_biomass(3, 2, 10.0);
        world.add_dead_biomass(1, 0, 10.0);
        assert_eq!(world.strongest_nutrient_cell(), Some((1, 0)));
    }

    #[test]
    fn growth_follows_nutrient_gradient() {
        let mut world = MyceliumWorld::new(16, 9, (3, 4));
        world.add_dead_biomass(13, 4, 400.0);

        world.run_steps(80);

        let east_biomass: f32 = (8..16).map(|x| world.biomass_at(x, 4)).sum();
        let west_biomass: f32 = (0..3).map(|x| world.biomass_at(x, 4)).sum();
        assert!(
            east_biomass > west_biomass,
            "east_biomass={east_biomass}, west_biomass={west_biomass}"
        );
    }

    #[test]
    fn toxin_suppresses_expansion() {
        let mut clean = MyceliumWorld::new(16, 9, (3, 4));
        clean.add_dead_biomass(13, 4, 400.0);
        clean.run_steps(80);

        let mut toxic = MyceliumWorld::new(16, 9, (3, 4));
        toxic.add_dead_biomass(13, 4, 400.0);
        for x in 7..11 {
            for y in 2..7 {
                toxic.add_toxin(x, y, 200.0);
            }
        }
        toxic.run_steps(80);

        let clean_far_biomass: f32 = (11..16).map(|x| clean.biomass_at(x, 4)).sum();
        let toxic_far_biomass: f32 = (11..16).map(|x| toxic.biomass_at(x, 4)).sum();
        assert!(
            toxic_far_biomass < clean_far_biomass * 0.5,
            "toxic_far_biomass={toxic_far_biomass}, clean_far_biomass={clean_far_biomass}"
        );
    }

    #[test]
    fn network_preserves_connected_path_to_food() {
        let mut world = MyceliumWorld::new(16, 9, (3, 4));
        world.add_dead_biomass(13, 4, 800.0);

        world.run_steps(120);

        assert!(
            world.has_biomass_path_to((13, 4), 0.08),
            "metrics={:?}, target_biomass={}",
            world.metrics(),
            world.biomass_at(13, 4)
        );
    }

    #[test]
    fn nutrient_transport_increases_central_biomass() {
        let mut world = MyceliumWorld::new(16, 9, (3, 4));
        let before = world.metrics().central_biomass;
        world.add_dead_biomass(10, 4, 600.0);

        world.run_steps(100);

        assert!(world.metrics().central_biomass > before);
    }

    #[test]
    fn legacy_step_report_exposes_unpaired_origin_credit() {
        let mut world = MyceliumWorld::new(5, 5, (1, 1));
        world.fields.add(FieldLayer::Biomass, 3, 3, 10.0);
        world.fields.add(FieldLayer::Nutrient, 3, 3, 100.0);

        let report = world.step_with_report();

        assert!(report.assimilation_local_biomass_credit > 0.0);
        assert_eq!(
            report.assimilation_local_biomass_credit,
            report.assimilation_nutrient_debit
        );
        assert!(
            (report.origin_biomass_extra_credit - report.assimilation_local_biomass_credit * 0.25)
                .abs()
                < 0.000_001
        );
        assert!((report.origin_credit_fraction_of_assimilation() - 0.25).abs() < 0.000_001);
        assert!(report.biomass_after_decay < report.biomass_before_decay);
        assert_eq!(report.tick_after, report.tick_before + 1);
    }

    #[test]
    fn legacy_step_and_reported_step_preserve_identical_field_behavior() {
        let mut legacy = MyceliumWorld::new(8, 5, (1, 2));
        let mut audited = MyceliumWorld::new(8, 5, (1, 2));
        legacy.add_dead_biomass(6, 2, 200.0);
        audited.add_dead_biomass(6, 2, 200.0);
        legacy.add_toxin(4, 1, 50.0);
        audited.add_toxin(4, 1, 50.0);

        for _ in 0..8 {
            legacy.step();
            let report = audited.step_with_report();
            assert_eq!(report.tick_after, audited.tick());

            for y in 0..5 {
                for x in 0..8 {
                    for layer in [FieldLayer::Biomass, FieldLayer::Nutrient, FieldLayer::Toxin] {
                        assert_eq!(
                            legacy.fields.get(layer, x, y),
                            audited.fields.get(layer, x, y)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn ppm_heatmap_exports_visible_mycelium_state() {
        let mut world = MyceliumWorld::new(8, 5, (1, 2));
        world.add_dead_biomass(6, 2, 200.0);
        world.add_toxin(4, 1, 100.0);
        world.run_steps(4);

        let ppm = world.to_ppm_heatmap();

        assert!(ppm.starts_with("P3\n# symtropy-mycelium tick 4\n8 5\n255\n"));
        assert!(ppm.contains("240 240 120"));
    }

    #[test]
    fn basin_fields_feed_and_stress_mycelium() {
        let mut world = MyceliumWorld::new(8, 5, (1, 2));
        let mut basin = FieldGrid::new(8, 5);
        basin.set(FieldLayer::Nutrient, 6, 2, 100.0);
        basin.set(FieldLayer::Toxin, 4, 2, 80.0);

        world.absorb_basin_fields(&basin);

        assert_eq!(world.fields.get(FieldLayer::Nutrient, 6, 2), 20.0);
        assert!((world.fields.get(FieldLayer::Toxin, 4, 2) - 9.6).abs() < 0.001);
    }

    #[test]
    fn biomass_buffers_basin_toxins() {
        let world = MyceliumWorld::new(8, 5, (1, 2));
        let mut basin = FieldGrid::new(8, 5);
        basin.set(FieldLayer::Toxin, 1, 2, 10.0);

        world.buffer_basin_fields(&mut basin);

        assert!(basin.get(FieldLayer::Toxin, 1, 2) < 10.0);
        assert!(basin.get(FieldLayer::Nutrient, 1, 2) > 0.0);
    }
}
