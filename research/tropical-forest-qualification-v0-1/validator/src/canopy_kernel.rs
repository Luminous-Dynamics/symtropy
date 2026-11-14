//! Minimal headless spatial cohort-canopy kernel for synthetic process tests.
//!
//! This module is an analytical representation experiment, not a calibrated radiation,
//! physiology, growth, recruitment, or forest-dynamics model. It intentionally contains
//! no renderer, camera, animation, or visual-LOD inputs.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct CanopyCohort {
    id: String,
    support: BTreeSet<(usize, usize)>,
    lai_per_cell: f64,
    alive: bool,
}

impl CanopyCohort {
    pub fn id(&self) -> &str { &self.id }
    pub fn support(&self) -> &BTreeSet<(usize, usize)> { &self.support }
    pub fn lai_per_cell(&self) -> f64 { self.lai_per_cell }
    pub fn is_alive(&self) -> bool { self.alive }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanopyPatch {
    width: usize,
    height: usize,
    cohorts: BTreeMap<String, CanopyCohort>,
}

impl CanopyPatch {
    pub fn new(width: usize, height: usize) -> Result<Self, String> {
        if width == 0 || height == 0 || width.checked_mul(height).is_none() {
            return Err("canopy dimensions must be non-zero and fit addressable memory".to_string());
        }
        Ok(Self { width, height, cohorts: BTreeMap::new() })
    }

    pub fn dimensions(&self) -> (usize, usize) { (self.width, self.height) }
    pub fn cohorts(&self) -> &BTreeMap<String, CanopyCohort> { &self.cohorts }

    /// Add one cohort's per-cell LAI contribution over its declared horizontal support.
    ///
    /// Overlapping cohorts sum their contributions. The caller supplies explicit support;
    /// no centroid-to-crown geometry or biological allometry is inferred here.
    pub fn add_cohort(
        &mut self,
        id: impl Into<String>,
        support: impl IntoIterator<Item = (usize, usize)>,
        lai_per_cell: f64,
    ) -> Result<(), String> {
        let id = id.into();
        if id.trim().is_empty() { return Err("cohort id must not be empty".to_string()); }
        if self.cohorts.contains_key(&id) {
            return Err(format!("duplicate canopy cohort id: {id}"));
        }
        if !lai_per_cell.is_finite() || lai_per_cell <= 0.0 {
            return Err("cohort LAI contribution must be finite and positive".to_string());
        }

        let coordinates: Vec<(usize, usize)> = support.into_iter().collect();
        if coordinates.is_empty() { return Err("cohort support must contain at least one cell".to_string()); }
        let support: BTreeSet<(usize, usize)> = coordinates.iter().copied().collect();
        if support.len() != coordinates.len() {
            return Err(format!("cohort {id} contains duplicate support coordinates"));
        }
        for (x, y) in &support {
            if *x >= self.width || *y >= self.height {
                return Err(format!("cohort {id} support cell ({x}, {y}) is outside the patch"));
            }
        }

        self.cohorts.insert(id.clone(), CanopyCohort { id, support, lai_per_cell, alive: true });
        Ok(())
    }

    /// Mortality is a one-way state transition in this v0 kernel.
    /// Crown support is removed from the derived LAI field on the next evaluation.
    pub fn kill_cohort(&mut self, id: &str) -> Result<(), String> {
        let cohort = self.cohorts.get_mut(id)
            .ok_or_else(|| format!("unknown canopy cohort: {id}"))?;
        if !cohort.alive { return Err(format!("canopy cohort is already dead: {id}")); }
        cohort.alive = false;
        Ok(())
    }

    /// Derive a row-major LAI field from living cohort supports.
    pub fn lai_field(&self) -> Vec<f64> {
        let mut field = vec![0.0; self.width * self.height];
        for cohort in self.cohorts.values().filter(|cohort| cohort.alive) {
            for (x, y) in &cohort.support {
                field[y * self.width + x] += cohort.lai_per_cell;
            }
        }
        field
    }

    pub fn total_lai(&self) -> f64 { self.lai_field().iter().sum() }
    pub fn mean_lai(&self) -> f64 { self.total_lai() / (self.width * self.height) as f64 }

    /// Analytical direct-beam transmittance T=exp(-k*LAI); k is synthetic unless calibrated externally.
    pub fn transmittance_field(&self, extinction_coefficient: f64) -> Result<Vec<f64>, String> {
        if !extinction_coefficient.is_finite() || extinction_coefficient < 0.0 {
            return Err("extinction coefficient must be finite and non-negative".to_string());
        }
        Ok(self.lai_field().iter()
            .map(|lai| (-extinction_coefficient * lai).exp())
            .collect())
    }

    pub fn open_cells(&self, maximum_lai: f64) -> Result<BTreeSet<(usize, usize)>, String> {
        if !maximum_lai.is_finite() || maximum_lai < 0.0 {
            return Err("maximum open-cell LAI must be finite and non-negative".to_string());
        }
        let field = self.lai_field();
        let mut open = BTreeSet::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if field[y * self.width + x] <= maximum_lai { open.insert((x, y)); }
            }
        }
        Ok(open)
    }

    /// Four-neighbour connected components of cells at or below the maximum LAI.
    pub fn gap_components(&self, maximum_lai: f64) -> Result<Vec<Vec<(usize, usize)>>, String> {
        let mut remaining = self.open_cells(maximum_lai)?;
        let mut components = Vec::new();
        while let Some(start) = remaining.pop_first() {
            let mut component = vec![start];
            let mut queue = VecDeque::new();
            queue.push_back(start);
            while let Some((x, y)) = queue.pop_front() {
                let mut neighbours = Vec::with_capacity(4);
                if let Some(left) = x.checked_sub(1) { neighbours.push((left, y)); }
                if x + 1 < self.width { neighbours.push((x + 1, y)); }
                if let Some(up) = y.checked_sub(1) { neighbours.push((x, up)); }
                if y + 1 < self.height { neighbours.push((x, y + 1)); }
                for neighbour in neighbours {
                    if remaining.remove(&neighbour) {
                        component.push(neighbour);
                        queue.push_back(neighbour);
                    }
                }
            }
            component.sort_unstable();
            components.push(component);
        }
        components.sort_by_key(|component| component.first().copied());
        Ok(components)
    }

    /// Return cells whose analytical transmittance clears a declared light threshold.
    /// This is an optical eligibility mask, not a seedling recruitment prediction.
    pub fn understory_light_eligible_cells(
        &self,
        extinction_coefficient: f64,
        minimum_transmittance: f64,
    ) -> Result<BTreeSet<(usize, usize)>, String> {
        if !minimum_transmittance.is_finite() || !(0.0..=1.0).contains(&minimum_transmittance) {
            return Err("minimum transmittance must be finite and within [0, 1]".to_string());
        }
        let transmittance = self.transmittance_field(extinction_coefficient)?;
        let mut eligible = BTreeSet::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if transmittance[y * self.width + x] >= minimum_transmittance {
                    eligible.insert((x, y));
                }
            }
        }
        Ok(eligible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_pattern(patch: &mut CanopyPatch, occupied: impl IntoIterator<Item = (usize, usize)>) {
        for (index, cell) in occupied.into_iter().enumerate() {
            patch.add_cohort(format!("cohort-{index:03}"), [cell], 2.0)
                .expect("synthetic single-cell cohort should be valid");
        }
    }

    fn checkerboard_cells(width: usize, height: usize) -> Vec<(usize, usize)> {
        let mut cells = Vec::new();
        for y in 0..height { for x in 0..width {
            if (x + y) % 2 == 0 { cells.push((x, y)); }
        }}
        cells
    }

    fn block_cells(width: usize, height: usize) -> Vec<(usize, usize)> {
        let mut cells = Vec::new();
        for y in 0..height { for x in 0..(width / 2) { cells.push((x, y)); }}
        cells
    }

    #[test]
    fn equal_total_lai_canopies_have_different_gap_topology() {
        let mut checkerboard = CanopyPatch::new(8, 8).unwrap();
        let mut clustered = CanopyPatch::new(8, 8).unwrap();
        add_pattern(&mut checkerboard, checkerboard_cells(8, 8));
        add_pattern(&mut clustered, block_cells(8, 8));
        assert!((checkerboard.total_lai() - 64.0).abs() < 1e-12);
        assert!((clustered.total_lai() - 64.0).abs() < 1e-12);
        assert!((checkerboard.mean_lai() - clustered.mean_lai()).abs() < 1e-12);
        assert_eq!(checkerboard.gap_components(0.0).unwrap().len(), 32);
        assert_eq!(clustered.gap_components(0.0).unwrap().len(), 1);
    }

    #[test]
    fn cohort_mortality_changes_only_declared_crown_support() {
        let mut patch = CanopyPatch::new(4, 2).unwrap();
        patch.add_cohort("upper", [(0, 0), (1, 0)], 2.0).unwrap();
        patch.add_cohort("understory", [(1, 0), (2, 0)], 0.5).unwrap();
        let before = patch.lai_field();
        patch.kill_cohort("upper").unwrap();
        let after = patch.lai_field();
        assert_eq!(before[0], 2.0);
        assert_eq!(before[1], 2.5);
        assert_eq!(before[2], 0.5);
        assert_eq!(after[0], 0.0);
        assert_eq!(after[1], 0.5);
        assert_eq!(after[2], 0.5);
        assert_eq!(after[3], 0.0);
    }

    #[test]
    fn mortality_is_one_way_and_unknown_cohorts_fail_closed() {
        let mut patch = CanopyPatch::new(2, 2).unwrap();
        patch.add_cohort("a", [(0, 0)], 1.0).unwrap();
        patch.kill_cohort("a").unwrap();
        assert!(patch.kill_cohort("a").is_err());
        assert!(patch.kill_cohort("missing").is_err());
    }

    #[test]
    fn duplicate_ids_supports_and_out_of_bounds_cells_are_rejected() {
        let mut patch = CanopyPatch::new(2, 2).unwrap();
        patch.add_cohort("a", [(0, 0)], 1.0).unwrap();
        assert!(patch.add_cohort("a", [(1, 1)], 1.0).is_err());
        assert!(patch.add_cohort("b", [(0, 0), (0, 0)], 1.0).is_err());
        assert!(patch.add_cohort("c", [(2, 0)], 1.0).is_err());
        assert!(patch.add_cohort("d", [(0, 1)], f64::NAN).is_err());
    }

    #[test]
    fn translation_preserves_gap_component_count_when_inside_bounds() {
        let mut original = CanopyPatch::new(8, 8).unwrap();
        let mut translated = CanopyPatch::new(8, 8).unwrap();
        let cells = vec![(2, 2), (3, 2), (4, 2), (2, 3), (4, 3), (2, 4), (3, 4), (4, 4)];
        add_pattern(&mut original, cells.iter().copied());
        add_pattern(&mut translated, cells.iter().map(|(x, y)| (x + 1, y + 1)));
        assert_eq!(original.gap_components(0.0).unwrap().len(), translated.gap_components(0.0).unwrap().len());
        assert_eq!(original.total_lai(), translated.total_lai());
    }

    #[test]
    fn transmittance_and_light_eligibility_are_deterministic() {
        let mut patch = CanopyPatch::new(2, 2).unwrap();
        patch.add_cohort("c", [(0, 0)], 2.0).unwrap();
        let first = patch.transmittance_field(0.5).unwrap();
        let second = patch.transmittance_field(0.5).unwrap();
        assert_eq!(first, second);
        assert_eq!(patch.understory_light_eligible_cells(0.5, 0.9).unwrap(),
            BTreeSet::from([(0, 1), (1, 0), (1, 1)]));
    }

    #[test]
    fn invalid_light_parameters_fail_closed() {
        let patch = CanopyPatch::new(2, 2).unwrap();
        assert!(patch.transmittance_field(-0.1).is_err());
        assert!(patch.transmittance_field(f64::INFINITY).is_err());
        assert!(patch.understory_light_eligible_cells(0.5, 1.1).is_err());
    }
}
