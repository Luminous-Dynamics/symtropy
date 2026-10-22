// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Deterministic spatial keys and residency-budget planning for galactic catalogs.
//!
//! This is a catalog/indexing primitive, not an astronomical coordinate authority.
//! Inputs must already be transformed into one declared Cartesian frame and one
//! catalog epoch. The caller must retain source provenance, covariance/uncertainty,
//! frame identity, epoch, transformation receipts, and ingest revision outside this
//! geometric key. It intentionally does not infer missing stellar distances.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// Half-width of the indexed cube in parsecs. The cube spans 100,000 pc per axis.
pub const GALACTIC_DOMAIN_HALF_WIDTH_PC: f64 = 50_000.0;
/// Fixed-point coordinate quantum. One microparsec is about 0.206 AU.
pub const GALACTIC_COORDINATE_QUANTUM_MICROPC: i64 = 1;
/// Maximum octree depth; 36 bits per axis produce a 108-bit Morton path.
pub const GALAXY_MAX_CELL_DEPTH: u8 = 36;

const MICROPC_PER_PC: f64 = 1_000_000.0;
const HALF_WIDTH_MICROPC: i64 = 50_000_000_000;
const DOMAIN_SPAN_MICROPC: u128 = 100_000_000_000;
const GRID_AXIS_CELLS: u128 = 1_u128 << GALAXY_MAX_CELL_DEPTH;

/// Validated, fixed-point Cartesian coordinates in microparsecs.
///
/// Frame and epoch are deliberately not implied by this type. A catalog snapshot
/// must bind this position to its declared coordinate frame and epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GalacticPosition {
    microparsecs: [i64; 3],
}

impl GalacticPosition {
    /// Convert a Cartesian parsec coordinate after the caller has selected a
    /// common frame and epoch. Values are rounded to the nearest microparsec.
    pub fn from_parsecs(position_pc: [f64; 3]) -> Result<Self, GalaxyIndexError> {
        let mut microparsecs = [0_i64; 3];
        for (axis, value) in position_pc.into_iter().enumerate() {
            if !value.is_finite() {
                return Err(GalaxyIndexError::NonFiniteCoordinate);
            }
            if !(-GALACTIC_DOMAIN_HALF_WIDTH_PC..=GALACTIC_DOMAIN_HALF_WIDTH_PC)
                .contains(&value)
            {
                return Err(GalaxyIndexError::CoordinateOutsideDomain);
            }
            microparsecs[axis] = (value * MICROPC_PER_PC).round() as i64;
        }
        Self::from_microparsecs(microparsecs)
    }

    /// Construct from fixed-point microparsecs, rejecting points outside the
    /// indexed cube rather than wrapping or clamping them.
    pub fn from_microparsecs(microparsecs: [i64; 3]) -> Result<Self, GalaxyIndexError> {
        if microparsecs
            .iter()
            .any(|value| !(-HALF_WIDTH_MICROPC..=HALF_WIDTH_MICROPC).contains(value))
        {
            return Err(GalaxyIndexError::CoordinateOutsideDomain);
        }
        Ok(Self { microparsecs })
    }

    /// Return the exact stored fixed-point coordinate.
    pub const fn microparsecs(self) -> [i64; 3] {
        self.microparsecs
    }

    /// Compute the deterministic Morton-prefix cell for a requested octree depth.
    pub fn cell_at_depth(self, depth: u8) -> Result<GalaxyCellId, GalaxyIndexError> {
        if depth > GALAXY_MAX_CELL_DEPTH {
            return Err(GalaxyIndexError::InvalidCellDepth);
        }

        let x = quantize_axis(self.microparsecs[0]);
        let y = quantize_axis(self.microparsecs[1]);
        let z = quantize_axis(self.microparsecs[2]);
        let morton = interleave_morton(x, y, z);
        let discarded_bits = 3 * (GALAXY_MAX_CELL_DEPTH - depth);
        Ok(GalaxyCellId {
            depth,
            prefix: morton >> discarded_bits,
        })
    }
}

fn quantize_axis(coordinate_micro_pc: i64) -> u64 {
    // Validation bounds the product to about 2^73, so u128 avoids overflow.
    let offset = (coordinate_micro_pc + HALF_WIDTH_MICROPC) as u128;
    ((offset * (GRID_AXIS_CELLS - 1)) / DOMAIN_SPAN_MICROPC) as u64
}

fn interleave_morton(x: u64, y: u64, z: u64) -> u128 {
    let mut code = 0_u128;
    for bit in 0..GALAXY_MAX_CELL_DEPTH {
        let bit = u32::from(bit);
        code |= (u128::from((x >> bit) & 1)) << (3 * bit + 2);
        code |= (u128::from((y >> bit) & 1)) << (3 * bit + 1);
        code |= u128::from((z >> bit) & 1) << (3 * bit);
    }
    code
}

/// Stable identity of one node in the fixed galaxy-cube octree.
///
/// The prefix contains 3 bits per octree level. It is a deterministic partition
/// key; it does not certify that an astronomical source is measured, complete,
/// current, or physically certain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GalaxyCellId {
    depth: u8,
    prefix: u128,
}

impl GalaxyCellId {
    /// Root cell covering the entire indexed cube.
    pub const fn root() -> Self {
        Self {
            depth: 0,
            prefix: 0,
        }
    }

    pub const fn depth(self) -> u8 {
        self.depth
    }

    /// Return the compact path prefix. Pair it with depth when serializing identity.
    pub const fn prefix(self) -> u128 {
        self.prefix
    }

    pub const fn path_bits(self) -> u32 {
        3 * self.depth as u32
    }

    /// Construct one child octant. Octant bits encode x/y/z half-space choices.
    pub fn child(self, octant: u8) -> Result<Self, GalaxyIndexError> {
        if self.depth >= GALAXY_MAX_CELL_DEPTH {
            return Err(GalaxyIndexError::CellDepthLimitReached);
        }
        if octant > 7 {
            return Err(GalaxyIndexError::InvalidOctant);
        }
        Ok(Self {
            depth: self.depth + 1,
            prefix: (self.prefix << 3) | u128::from(octant),
        })
    }

    pub fn parent(self) -> Option<Self> {
        if self.depth == 0 {
            return None;
        }
        Some(Self {
            depth: self.depth - 1,
            prefix: self.prefix >> 3,
        })
    }

    /// True when this cell contains the supplied cell in the octree hierarchy.
    pub fn contains(self, other: Self) -> bool {
        if self.depth > other.depth {
            return false;
        }
        let shift = 3 * u32::from(other.depth - self.depth);
        (other.prefix >> shift) == self.prefix
    }
}

/// Estimated residency cost for one independently loadable catalog tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GalaxyTileCandidate {
    pub cell: GalaxyCellId,
    /// Larger values are admitted first when budgets are constrained.
    pub priority: u32,
    pub estimated_records: u64,
    pub resident_bytes: u64,
    /// Content that cannot be silently dropped (for example a selected target).
    pub pinned: bool,
}

/// Hard upper bounds for one streaming/admission pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GalaxyStreamingBudget {
    pub max_tiles: usize,
    pub max_records: u64,
    pub max_resident_bytes: u64,
}

/// Deterministic tile admission result. Selected cells are in admission order:
/// pinned first, then priority descending, then stable cell identity ascending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalaxyTilePlan {
    pub selected: Vec<GalaxyCellId>,
    pub deferred_tiles: usize,
    pub admitted_records: u64,
    pub admitted_resident_bytes: u64,
}

/// Fail-closed errors for galaxy-cell construction and streaming admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalaxyIndexError {
    NonFiniteCoordinate,
    CoordinateOutsideDomain,
    InvalidCellDepth,
    CellDepthLimitReached,
    InvalidOctant,
    DuplicateTileCell,
    PinnedContentExceedsBudget,
    BudgetArithmeticOverflow,
}

impl fmt::Display for GalaxyIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for GalaxyIndexError {}

/// Select candidate tiles without exceeding any declared residency budget.
///
/// Pinned content is validated as a whole before selection. If all pinned items
/// cannot fit, the function errors rather than returning a plan that silently
/// loses a required target. Non-pinned tiles that do not fit are deferred.
pub fn plan_galaxy_tiles(
    candidates: &[GalaxyTileCandidate],
    budget: GalaxyStreamingBudget,
) -> Result<GalaxyTilePlan, GalaxyIndexError> {
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if !seen.insert(candidate.cell) {
            return Err(GalaxyIndexError::DuplicateTileCell);
        }
    }

    let mut pinned_count = 0_usize;
    let mut pinned_records = 0_u64;
    let mut pinned_bytes = 0_u64;
    for candidate in candidates.iter().filter(|candidate| candidate.pinned) {
        pinned_count = pinned_count
            .checked_add(1)
            .ok_or(GalaxyIndexError::BudgetArithmeticOverflow)?;
        pinned_records = pinned_records
            .checked_add(candidate.estimated_records)
            .ok_or(GalaxyIndexError::BudgetArithmeticOverflow)?;
        pinned_bytes = pinned_bytes
            .checked_add(candidate.resident_bytes)
            .ok_or(GalaxyIndexError::BudgetArithmeticOverflow)?;
    }
    if pinned_count > budget.max_tiles
        || pinned_records > budget.max_records
        || pinned_bytes > budget.max_resident_bytes
    {
        return Err(GalaxyIndexError::PinnedContentExceedsBudget);
    }

    let mut ranked = candidates.to_vec();
    ranked.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| right.priority.cmp(&left.priority))
            .then_with(|| left.cell.cmp(&right.cell))
    });

    let mut selected = Vec::new();
    let mut deferred_tiles = 0_usize;
    let mut admitted_records = 0_u64;
    let mut admitted_resident_bytes = 0_u64;

    for candidate in ranked {
        let next_records = admitted_records
            .checked_add(candidate.estimated_records)
            .ok_or(GalaxyIndexError::BudgetArithmeticOverflow)?;
        let next_bytes = admitted_resident_bytes
            .checked_add(candidate.resident_bytes)
            .ok_or(GalaxyIndexError::BudgetArithmeticOverflow)?;
        let fits = selected.len() < budget.max_tiles
            && next_records <= budget.max_records
            && next_bytes <= budget.max_resident_bytes;

        if fits {
            selected.push(candidate.cell);
            admitted_records = next_records;
            admitted_resident_bytes = next_bytes;
        } else if candidate.pinned {
            // The aggregate check should make this unreachable; stay fail-closed.
            return Err(GalaxyIndexError::PinnedContentExceedsBudget);
        } else {
            deferred_tiles = deferred_tiles
                .checked_add(1)
                .ok_or(GalaxyIndexError::BudgetArithmeticOverflow)?;
        }
    }

    Ok(GalaxyTilePlan {
        selected,
        deferred_tiles,
        admitted_records,
        admitted_resident_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(x_pc: f64, y_pc: f64, z_pc: f64) -> GalacticPosition {
        GalacticPosition::from_parsecs([x_pc, y_pc, z_pc]).expect("valid fixture point")
    }

    fn candidate(
        x_pc: f64,
        priority: u32,
        bytes: u64,
        records: u64,
        pinned: bool,
    ) -> GalaxyTileCandidate {
        GalaxyTileCandidate {
            cell: position(x_pc, 0.0, 0.0)
                .cell_at_depth(12)
                .expect("valid test depth"),
            priority,
            estimated_records: records,
            resident_bytes: bytes,
            pinned,
        }
    }

    #[test]
    fn fixed_point_round_trip_and_cube_boundaries_are_exact() {
        let point = GalacticPosition::from_parsecs([-50_000.0, 0.0, 50_000.0])
            .expect("inclusive cube bounds");
        assert_eq!(point.microparsecs(), [-50_000_000_000, 0, 50_000_000_000]);
        assert_eq!(
            GalacticPosition::from_microparsecs(point.microparsecs()),
            Ok(point)
        );
        assert_eq!(
            position(50_000.0, 0.0, 0.0)
                .cell_at_depth(GALAXY_MAX_CELL_DEPTH)
                .unwrap()
                .depth(),
            GALAXY_MAX_CELL_DEPTH
        );
    }

    #[test]
    fn non_finite_and_out_of_domain_coordinates_reject() {
        assert_eq!(
            GalacticPosition::from_parsecs([f64::NAN, 0.0, 0.0]),
            Err(GalaxyIndexError::NonFiniteCoordinate)
        );
        assert_eq!(
            GalacticPosition::from_parsecs([50_000.000_001, 0.0, 0.0]),
            Err(GalaxyIndexError::CoordinateOutsideDomain)
        );
        assert_eq!(
            GalacticPosition::from_microparsecs([0, i64::MAX, 0]),
            Err(GalaxyIndexError::CoordinateOutsideDomain)
        );
    }

    #[test]
    fn morton_cell_is_deterministic_and_descends_from_root() {
        let point = position(812.25, -112.0, 0.5);
        let parent = point.cell_at_depth(18).expect("valid depth");
        let leaf = point
            .cell_at_depth(GALAXY_MAX_CELL_DEPTH)
            .expect("leaf depth");
        assert_eq!(
            parent,
            point.cell_at_depth(18).expect("same input is stable")
        );
        assert!(GalaxyCellId::root().contains(parent));
        assert!(parent.contains(leaf));
        assert_eq!(leaf.parent().unwrap().parent().unwrap().depth(), 34);
        assert_eq!(GalaxyCellId::root().parent(), None);
    }

    #[test]
    fn all_eight_root_octants_have_distinct_stable_keys() {
        let root = GalaxyCellId::root();
        let children: BTreeSet<_> = (0..=7)
            .map(|octant| root.child(octant).expect("valid child"))
            .collect();
        assert_eq!(children.len(), 8);
        assert_eq!(root.child(8), Err(GalaxyIndexError::InvalidOctant));
    }

    #[test]
    fn cell_depth_and_child_limit_are_enforced() {
        assert_eq!(
            position(0.0, 0.0, 0.0).cell_at_depth(GALAXY_MAX_CELL_DEPTH + 1),
            Err(GalaxyIndexError::InvalidCellDepth)
        );
        let mut leaf = GalaxyCellId::root();
        for octant in 0..GALAXY_MAX_CELL_DEPTH {
            leaf = leaf.child(octant % 8).expect("within maximum depth");
        }
        assert_eq!(
            leaf.child(0),
            Err(GalaxyIndexError::CellDepthLimitReached)
        );
    }

    #[test]
    fn budget_planner_prioritizes_pinned_then_priority_and_never_overcommits() {
        let candidates = [
            candidate(-1_000.0, 100, 50, 50, false),
            candidate(0.0, 1, 25, 10, true),
            candidate(1_000.0, 10, 60, 60, false),
        ];
        let plan = plan_galaxy_tiles(
            &candidates,
            GalaxyStreamingBudget {
                max_tiles: 2,
                max_records: 100,
                max_resident_bytes: 100,
            },
        )
        .expect("pinned tile and best fitting high-priority tile fit");
        assert_eq!(plan.selected.len(), 2);
        assert_eq!(plan.selected[0], candidates[1].cell);
        assert_eq!(plan.selected[1], candidates[0].cell);
        assert_eq!(plan.admitted_records, 60);
        assert_eq!(plan.admitted_resident_bytes, 75);
        assert_eq!(plan.deferred_tiles, 1);
    }

    #[test]
    fn input_permutation_does_not_change_tile_plan() {
        let first = [
            candidate(-1_000.0, 100, 20, 2, false),
            candidate(0.0, 20, 30, 3, true),
            candidate(1_000.0, 10, 10, 1, false),
        ];
        let shuffled = [first[2], first[0], first[1]];
        let budget = GalaxyStreamingBudget {
            max_tiles: 3,
            max_records: 20,
            max_resident_bytes: 100,
        };
        assert_eq!(
            plan_galaxy_tiles(&first, budget),
            plan_galaxy_tiles(&shuffled, budget)
        );
    }

    #[test]
    fn required_content_failure_is_explicit_not_silent() {
        let candidates = [
            candidate(-1_000.0, 100, 80, 8, true),
            candidate(1_000.0, 1, 1, 1, true),
        ];
        assert_eq!(
            plan_galaxy_tiles(
                &candidates,
                GalaxyStreamingBudget {
                    max_tiles: 2,
                    max_records: 20,
                    max_resident_bytes: 50,
                }
            ),
            Err(GalaxyIndexError::PinnedContentExceedsBudget)
        );
    }

    #[test]
    fn duplicate_cell_candidate_is_rejected() {
        let repeated = candidate(123.0, 1, 10, 1, false);
        assert_eq!(
            plan_galaxy_tiles(
                &[repeated, repeated],
                GalaxyStreamingBudget {
                    max_tiles: 4,
                    max_records: 100,
                    max_resident_bytes: 100,
                }
            ),
            Err(GalaxyIndexError::DuplicateTileCell)
        );
    }
}
