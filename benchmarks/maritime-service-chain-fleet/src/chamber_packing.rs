//! Exact tiny-instance two-dimensional lock-chamber vessel packing.
//!
//! This module consumes already-qualified rectangular vessel footprints. It
//! proves only geometric fit under the declared integer model; it does not
//! establish navigational clearance, mooring safety, chamber hydraulics or
//! authority to operate a lock.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChamberSpec {
    pub chamber_id: String,
    pub length_mm: u64,
    pub width_mm: u64,
    /// Per-hull clearance to each chamber wall and each vessel footprint.
    ///
    /// Each footprint is inflated by this amount on all four sides. Therefore
    /// the resulting minimum hull-to-hull gap is twice this value.
    pub clearance_mm: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VesselFootprint {
    pub vessel_id: String,
    pub length_mm: u64,
    pub beam_mm: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChamberPlacement {
    pub vessel_id: String,
    pub chamber_id: String,
    /// Longitudinal coordinate of the vessel hull's lower-left corner.
    pub x_mm: u64,
    /// Transverse coordinate of the vessel hull's lower-left corner.
    pub y_mm: u64,
    pub length_mm: u64,
    pub beam_mm: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackingTermination {
    /// A complete placement witness was found.
    Feasible,
    /// All candidate placements in the declared finite model were exhausted.
    Infeasible,
    /// The candidate-placement budget ended before a proof or witness existed.
    BudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChamberPackingResult {
    pub placements: Vec<ChamberPlacement>,
    pub states_explored: u64,
    pub termination: PackingTermination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChamberPackingError {
    EmptyChamberId,
    EmptyVesselId,
    DuplicateChamberId(String),
    DuplicateVesselId(String),
    ZeroChamberDimension(String),
    ZeroVesselDimension(String),
    ClearanceConsumesChamber(String),
    DimensionOverflow(String),
    InvalidSearchBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InflatedPlacement {
    vessel_index: usize,
    chamber_index: usize,
    x_mm: u64,
    y_mm: u64,
    length_mm: u64,
    width_mm: u64,
}

struct PackingSearch<'a> {
    chambers: &'a [ChamberSpec],
    vessels: &'a [VesselFootprint],
    placements: Vec<InflatedPlacement>,
    states_explored: u64,
    max_states: u64,
    budget_exhausted: bool,
}

fn validate_inputs(
    chambers: &[ChamberSpec],
    vessels: &[VesselFootprint],
) -> Result<(), ChamberPackingError> {
    let mut chamber_ids = BTreeSet::new();
    for chamber in chambers {
        if chamber.chamber_id.trim().is_empty() {
            return Err(ChamberPackingError::EmptyChamberId);
        }
        if !chamber_ids.insert(chamber.chamber_id.clone()) {
            return Err(ChamberPackingError::DuplicateChamberId(
                chamber.chamber_id.clone(),
            ));
        }
        if chamber.length_mm == 0 || chamber.width_mm == 0 {
            return Err(ChamberPackingError::ZeroChamberDimension(
                chamber.chamber_id.clone(),
            ));
        }
        let twice_clearance = chamber
            .clearance_mm
            .checked_mul(2)
            .ok_or_else(|| ChamberPackingError::DimensionOverflow(chamber.chamber_id.clone()))?;
        if twice_clearance >= chamber.length_mm || twice_clearance >= chamber.width_mm {
            return Err(ChamberPackingError::ClearanceConsumesChamber(
                chamber.chamber_id.clone(),
            ));
        }
    }

    let mut vessel_ids = BTreeSet::new();
    for vessel in vessels {
        if vessel.vessel_id.trim().is_empty() {
            return Err(ChamberPackingError::EmptyVesselId);
        }
        if !vessel_ids.insert(vessel.vessel_id.clone()) {
            return Err(ChamberPackingError::DuplicateVesselId(
                vessel.vessel_id.clone(),
            ));
        }
        if vessel.length_mm == 0 || vessel.beam_mm == 0 {
            return Err(ChamberPackingError::ZeroVesselDimension(
                vessel.vessel_id.clone(),
            ));
        }
    }
    Ok(())
}

fn inflated_dimensions(
    vessel: &VesselFootprint,
    chamber: &ChamberSpec,
) -> Result<Option<(u64, u64)>, ChamberPackingError> {
    let margin_twice = chamber
        .clearance_mm
        .checked_mul(2)
        .ok_or_else(|| ChamberPackingError::DimensionOverflow(vessel.vessel_id.clone()))?;
    let length = vessel
        .length_mm
        .checked_add(margin_twice)
        .ok_or_else(|| ChamberPackingError::DimensionOverflow(vessel.vessel_id.clone()))?;
    let width = vessel
        .beam_mm
        .checked_add(margin_twice)
        .ok_or_else(|| ChamberPackingError::DimensionOverflow(vessel.vessel_id.clone()))?;
    if length > chamber.length_mm || width > chamber.width_mm {
        return Ok(None);
    }
    Ok(Some((length, width)))
}

fn candidate_coordinates(
    existing: &[InflatedPlacement],
    chamber_index: usize,
    item_length: u64,
    item_width: u64,
) -> (Vec<u64>, Vec<u64>) {
    let mut xs = BTreeSet::from([0_u64]);
    let mut ys = BTreeSet::from([0_u64]);

    for placement in existing
        .iter()
        .filter(|placement| placement.chamber_index == chamber_index)
    {
        if let Some(right) = placement.x_mm.checked_add(placement.length_mm) {
            xs.insert(right);
        }
        if let Some(left) = placement.x_mm.checked_sub(item_length) {
            xs.insert(left);
        }
        xs.insert(placement.x_mm);

        if let Some(top) = placement.y_mm.checked_add(placement.width_mm) {
            ys.insert(top);
        }
        if let Some(bottom) = placement.y_mm.checked_sub(item_width) {
            ys.insert(bottom);
        }
        ys.insert(placement.y_mm);
    }

    (xs.into_iter().collect(), ys.into_iter().collect())
}

fn inside_chamber(
    chamber: &ChamberSpec,
    x_mm: u64,
    y_mm: u64,
    length_mm: u64,
    width_mm: u64,
) -> bool {
    x_mm.checked_add(length_mm)
        .is_some_and(|right| right <= chamber.length_mm)
        && y_mm
            .checked_add(width_mm)
            .is_some_and(|top| top <= chamber.width_mm)
}

fn overlaps(left: InflatedPlacement, right: InflatedPlacement) -> bool {
    left.x_mm < right.x_mm.saturating_add(right.length_mm)
        && right.x_mm < left.x_mm.saturating_add(left.length_mm)
        && left.y_mm < right.y_mm.saturating_add(right.width_mm)
        && right.y_mm < left.y_mm.saturating_add(left.width_mm)
}

impl PackingSearch<'_> {
    fn search(&mut self, remaining: &[usize]) -> Result<bool, ChamberPackingError> {
        if remaining.is_empty() {
            return Ok(true);
        }
        if self.budget_exhausted {
            return Ok(false);
        }

        // Enumerate every remaining vessel as the next item. This avoids making
        // input order part of the geometric search completeness argument.
        for (remaining_position, vessel_index) in remaining.iter().copied().enumerate() {
            let vessel = &self.vessels[vessel_index];
            for chamber_index in 0..self.chambers.len() {
                let chamber = &self.chambers[chamber_index];
                let Some((item_length, item_width)) = inflated_dimensions(vessel, chamber)? else {
                    continue;
                };
                let (xs, ys) =
                    candidate_coordinates(&self.placements, chamber_index, item_length, item_width);

                for x_mm in xs {
                    for y_mm in ys.iter().copied() {
                        if self.states_explored >= self.max_states {
                            self.budget_exhausted = true;
                            return Ok(false);
                        }
                        self.states_explored += 1;

                        if !inside_chamber(chamber, x_mm, y_mm, item_length, item_width) {
                            continue;
                        }
                        let candidate = InflatedPlacement {
                            vessel_index,
                            chamber_index,
                            x_mm,
                            y_mm,
                            length_mm: item_length,
                            width_mm: item_width,
                        };
                        if self.placements.iter().copied().any(|known| {
                            known.chamber_index == chamber_index && overlaps(known, candidate)
                        }) {
                            continue;
                        }

                        self.placements.push(candidate);
                        let mut next_remaining = remaining.to_vec();
                        next_remaining.remove(remaining_position);
                        if self.search(&next_remaining)? {
                            return Ok(true);
                        }
                        self.placements.pop();
                        if self.budget_exhausted {
                            return Ok(false);
                        }
                    }
                }
            }
        }
        Ok(false)
    }
}

/// Find a complete chamber assignment and two-dimensional placement for every
/// vessel in a small finite geometry model.
///
/// The search is exact for the declared axis-aligned, integer-millimetre model:
/// each stable placement is anchored at a wall or at an edge of an already
/// placed inflated rectangle. Every vessel ordering is explored. The budget
/// counts candidate (chamber, x, y) placement attempts. It does not count
/// validation, coordinate generation, recursion calls or memory allocations.
pub fn exact_chamber_packing(
    chambers: &[ChamberSpec],
    vessels: &[VesselFootprint],
    max_states: u64,
) -> Result<ChamberPackingResult, ChamberPackingError> {
    if max_states == 0 {
        return Err(ChamberPackingError::InvalidSearchBudget);
    }
    validate_inputs(chambers, vessels)?;

    if vessels.is_empty() {
        return Ok(ChamberPackingResult {
            placements: Vec::new(),
            states_explored: 0,
            termination: PackingTermination::Feasible,
        });
    }
    if chambers.is_empty() {
        return Ok(ChamberPackingResult {
            placements: Vec::new(),
            states_explored: 0,
            termination: PackingTermination::Infeasible,
        });
    }

    let mut canonical_chambers = chambers.to_vec();
    canonical_chambers.sort_by(|left, right| left.chamber_id.cmp(&right.chamber_id));
    let mut canonical_vessels = vessels.to_vec();
    canonical_vessels.sort_by(|left, right| left.vessel_id.cmp(&right.vessel_id));
    let vessel_order: Vec<usize> = (0..canonical_vessels.len()).collect();

    let mut search = PackingSearch {
        chambers: &canonical_chambers,
        vessels: &canonical_vessels,
        placements: Vec::with_capacity(canonical_vessels.len()),
        states_explored: 0,
        max_states,
        budget_exhausted: false,
    };
    let feasible = search.search(&vessel_order)?;
    if feasible {
        let mut placements: Vec<ChamberPlacement> = search
            .placements
            .iter()
            .map(|placement| {
                let chamber = &canonical_chambers[placement.chamber_index];
                let vessel = &canonical_vessels[placement.vessel_index];
                ChamberPlacement {
                    vessel_id: vessel.vessel_id.clone(),
                    chamber_id: chamber.chamber_id.clone(),
                    x_mm: placement.x_mm + chamber.clearance_mm,
                    y_mm: placement.y_mm + chamber.clearance_mm,
                    length_mm: vessel.length_mm,
                    beam_mm: vessel.beam_mm,
                }
            })
            .collect();
        placements.sort();
        return Ok(ChamberPackingResult {
            placements,
            states_explored: search.states_explored,
            termination: PackingTermination::Feasible,
        });
    }

    Ok(ChamberPackingResult {
        placements: Vec::new(),
        states_explored: search.states_explored,
        termination: if search.budget_exhausted {
            PackingTermination::BudgetExhausted
        } else {
            PackingTermination::Infeasible
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chamber(id: &str, length_mm: u64, width_mm: u64, clearance_mm: u64) -> ChamberSpec {
        ChamberSpec {
            chamber_id: id.into(),
            length_mm,
            width_mm,
            clearance_mm,
        }
    }

    fn vessel(id: &str, length_mm: u64, beam_mm: u64) -> VesselFootprint {
        VesselFootprint {
            vessel_id: id.into(),
            length_mm,
            beam_mm,
        }
    }

    #[test]
    fn single_vessel_fits_with_declared_wall_clearance() {
        let result =
            exact_chamber_packing(&[chamber("c1", 120, 30, 5)], &[vessel("v1", 100, 20)], 100)
                .unwrap();
        assert_eq!(result.termination, PackingTermination::Feasible);
        assert_eq!(
            result.placements,
            vec![ChamberPlacement {
                vessel_id: "v1".into(),
                chamber_id: "c1".into(),
                x_mm: 5,
                y_mm: 5,
                length_mm: 100,
                beam_mm: 20,
            }]
        );
    }

    #[test]
    fn two_vessels_pack_side_by_side_with_the_declared_clearance() {
        let result = exact_chamber_packing(
            &[chamber("c1", 120, 60, 2)],
            &[vessel("v1", 100, 20), vessel("v2", 100, 20)],
            10_000,
        )
        .unwrap();
        assert_eq!(result.termination, PackingTermination::Feasible);
        assert_eq!(result.placements.len(), 2);
        assert_eq!(result.placements[0].y_mm, 2);
        assert_eq!(result.placements[1].y_mm, 26);
    }

    #[test]
    fn two_vessels_pack_end_to_end_when_beam_exceeds_chamber_width() {
        let result = exact_chamber_packing(
            &[chamber("c1", 220, 40, 2)],
            &[vessel("v1", 100, 35), vessel("v2", 100, 35)],
            10_000,
        )
        .unwrap();
        assert_eq!(result.termination, PackingTermination::Feasible);
        assert_eq!(result.placements.len(), 2);
        assert!(result.placements[0].x_mm != result.placements[1].x_mm);
    }

    #[test]
    fn two_vessels_do_not_fit_when_only_scalar_lockage_capacity_would_allow_them() {
        let result = exact_chamber_packing(
            &[chamber("c1", 120, 30, 1)],
            &[vessel("v1", 100, 20), vessel("v2", 100, 20)],
            10_000,
        )
        .unwrap();
        assert_eq!(result.termination, PackingTermination::Infeasible);
        assert!(result.placements.is_empty());
    }

    #[test]
    fn multiple_chambers_are_considered() {
        let result = exact_chamber_packing(
            &[chamber("c1", 120, 30, 1), chamber("c2", 220, 50, 1)],
            &[vessel("v1", 200, 40), vessel("v2", 100, 20)],
            100_000,
        )
        .unwrap();
        assert_eq!(result.termination, PackingTermination::Feasible);
        assert_eq!(
            result
                .placements
                .iter()
                .map(|placement| placement.chamber_id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );
    }

    #[test]
    fn exact_clearance_boundary_is_admitted() {
        let result =
            exact_chamber_packing(&[chamber("c1", 104, 44, 2)], &[vessel("v1", 100, 40)], 100)
                .unwrap();
        assert_eq!(result.termination, PackingTermination::Feasible);
    }

    #[test]
    fn impossible_clearance_is_not_silently_reduced() {
        let error = exact_chamber_packing(&[chamber("c1", 10, 10, 5)], &[vessel("v1", 1, 1)], 100)
            .unwrap_err();
        assert_eq!(
            error,
            ChamberPackingError::ClearanceConsumesChamber("c1".into())
        );
    }

    #[test]
    fn duplicate_vessel_ids_are_rejected() {
        let error = exact_chamber_packing(
            &[chamber("c1", 20, 20, 0)],
            &[vessel("v1", 5, 5), vessel("v1", 5, 5)],
            100,
        )
        .unwrap_err();
        assert_eq!(error, ChamberPackingError::DuplicateVesselId("v1".into()));
    }

    #[test]
    fn zero_search_budget_is_invalid() {
        let error = exact_chamber_packing(&[chamber("c1", 20, 20, 0)], &[vessel("v1", 5, 5)], 0)
            .unwrap_err();
        assert_eq!(error, ChamberPackingError::InvalidSearchBudget);
    }

    #[test]
    fn budget_exhaustion_is_not_reported_as_infeasibility() {
        let result = exact_chamber_packing(
            &[chamber("c1", 100, 100, 0)],
            &[vessel("v1", 10, 10), vessel("v2", 10, 10)],
            1,
        )
        .unwrap();
        assert_eq!(result.termination, PackingTermination::BudgetExhausted);
        assert!(result.placements.is_empty());
    }

    #[test]
    fn input_permutation_has_identical_canonical_output() {
        let chambers = [chamber("c1", 100, 60, 1), chamber("c2", 80, 60, 1)];
        let reversed_chambers = [chambers[1].clone(), chambers[0].clone()];
        let first = [vessel("v1", 50, 20), vessel("v2", 40, 20)];
        let reversed_vessels = [vessel("v2", 40, 20), vessel("v1", 50, 20)];
        let forward = exact_chamber_packing(&chambers, &first, 10_000).unwrap();
        let reverse = exact_chamber_packing(&reversed_chambers, &reversed_vessels, 10_000).unwrap();
        assert_eq!(forward, reverse);
    }
}
