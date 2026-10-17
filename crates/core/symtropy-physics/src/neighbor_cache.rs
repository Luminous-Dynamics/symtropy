// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Displacement-bounded reuse of deterministic 2D neighbor pairs.
//!
//! The list is built using the buffered radius interaction_radius + skin.
//! It remains complete for the exact interaction radius only while every point
//! remains strictly within skin / 2 of its position at build time, and the
//! point-ID topology and configuration remain unchanged. Reaching the threshold
//! forces a rebuild. This conservative strict bound avoids relying on equality
//! at a floating-point boundary.
//!
//! This type does not update positions, run interactions, or mutate physics.
//! Callers must invoke requires_rebuild against current positions before each
//! reuse and rebuild or fall back to a naive oracle whenever it returns true or
//! an error. Exact per-tick distances must still be checked by the caller.

use std::collections::{HashMap, HashSet};

use crate::spatial_grid::{SpatialGrid2d, SpatialGridError, SpatialPoint};

/// Stable pair of application-owned point IDs; always a < b.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NeighborPair {
    pub a: u64,
    pub b: u64,
}

impl NeighborPair {
    fn new(first: u64, second: u64) -> Self {
        debug_assert_ne!(first, second);
        if first < second {
            Self {
                a: first,
                b: second,
            }
        } else {
            Self {
                a: second,
                b: first,
            }
        }
    }
}

/// Invalid configuration or point stream supplied to a neighbor-list operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeighborListError {
    InvalidInteractionRadius,
    InvalidSkin,
    BufferedRadiusOverflow,
    NonFinitePoint { id: u64 },
    DuplicateId { id: u64 },
    SpatialGrid(SpatialGridError),
}

impl core::fmt::Display for NeighborListError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInteractionRadius => {
                write!(f, "interaction radius must be finite and non-negative")
            }
            Self::InvalidSkin => write!(f, "skin must be finite and strictly positive"),
            Self::BufferedRadiusOverflow => {
                write!(f, "interaction radius plus skin must have finite squared radius")
            }
            Self::NonFinitePoint { id } => write!(f, "point {id} has a non-finite coordinate"),
            Self::DuplicateId { id } => write!(f, "duplicate point ID {id}"),
            Self::SpatialGrid(error) => write!(f, "spatial grid error: {error}"),
        }
    }
}

impl std::error::Error for NeighborListError {}

impl From<SpatialGridError> for NeighborListError {
    fn from(value: SpatialGridError) -> Self {
        Self::SpatialGrid(value)
    }
}

/// A reusable candidate-pair list built from a buffered spatial cutoff.
#[derive(Clone, Debug)]
pub struct NeighborList2d {
    interaction_radius: f64,
    skin: f64,
    cell_size: f64,
    build_radius: f64,
    build_points: Vec<SpatialPoint>,
    build_positions_by_id: HashMap<u64, [f64; 2]>,
    pairs: Vec<NeighborPair>,
}

impl NeighborList2d {
    /// Build candidates at interaction_radius + skin.
    ///
    /// Input order does not affect pair ordering. IDs must be unique, and all
    /// coordinates and parameters must be finite. Skin must be positive so the
    /// cache has a non-zero displacement budget.
    pub fn build(
        points: impl IntoIterator<Item = SpatialPoint>,
        interaction_radius: f64,
        skin: f64,
        cell_size: f64,
    ) -> Result<Self, NeighborListError> {
        Self::validate_configuration(interaction_radius, skin)?;
        let build_radius = interaction_radius + skin;
        let radius_squared = build_radius * build_radius;
        if !build_radius.is_finite() || !radius_squared.is_finite() {
            return Err(NeighborListError::BufferedRadiusOverflow);
        }

        let mut build_points: Vec<_> = points.into_iter().collect();
        build_points.sort_unstable_by_key(|point| point.id);

        let mut build_positions_by_id = HashMap::with_capacity(build_points.len());
        for point in &build_points {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(NeighborListError::NonFinitePoint { id: point.id });
            }
            if build_positions_by_id
                .insert(point.id, [point.x, point.y])
                .is_some()
            {
                return Err(NeighborListError::DuplicateId { id: point.id });
            }
        }

        let grid = SpatialGrid2d::new(build_points.iter().copied(), cell_size)?;
        let mut pairs = Vec::new();
        for point in &build_points {
            let candidates =
                grid.query_radius_candidates([point.x, point.y], build_radius)?;
            for candidate in candidates {
                if candidate.id <= point.id {
                    continue;
                }
                let dx = candidate.x - point.x;
                let dy = candidate.y - point.y;
                if dx * dx + dy * dy <= radius_squared {
                    pairs.push(NeighborPair::new(point.id, candidate.id));
                }
            }
        }

        // Construction order should already be stable, but canonicalize the
        // public invariant defensively if the grid implementation changes.
        pairs.sort_unstable();
        pairs.dedup();

        Ok(Self {
            interaction_radius,
            skin,
            cell_size,
            build_radius,
            build_points,
            build_positions_by_id,
            pairs,
        })
    }

    /// Whether this cache must be rebuilt before using its candidate pairs.
    ///
    /// Returns true for changed configuration, changed IDs/topology, or any
    /// point reaching skin / 2 displacement from its build position. Same-ID
    /// input order is irrelevant. Invalid coordinates or duplicate IDs return
    /// an error; callers must fail closed instead of reusing the cache.
    pub fn requires_rebuild(
        &self,
        current_points: &[SpatialPoint],
        interaction_radius: f64,
        skin: f64,
        cell_size: f64,
    ) -> Result<bool, NeighborListError> {
        Self::validate_configuration(interaction_radius, skin)?;
        let build_radius = interaction_radius + skin;
        if !build_radius.is_finite() || !(build_radius * build_radius).is_finite() {
            return Err(NeighborListError::BufferedRadiusOverflow);
        }
        if interaction_radius != self.interaction_radius
            || skin != self.skin
            || cell_size != self.cell_size
        {
            return Ok(true);
        }
        if current_points.len() != self.build_points.len() {
            return Ok(true);
        }

        let max_displacement = self.skin * 0.5;
        let max_displacement_squared = max_displacement * max_displacement;
        let mut seen = HashSet::with_capacity(current_points.len());

        for point in current_points {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(NeighborListError::NonFinitePoint { id: point.id });
            }
            if !seen.insert(point.id) {
                return Err(NeighborListError::DuplicateId { id: point.id });
            }
            let Some([build_x, build_y]) = self.build_positions_by_id.get(&point.id) else {
                return Ok(true);
            };
            let dx = point.x - build_x;
            let dy = point.y - build_y;
            let displacement_squared = dx * dx + dy * dy;
            // Equality is a conservative rebuild boundary. This avoids
            // relying on floating-point equality to preserve list coverage.
            if displacement_squared >= max_displacement_squared {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Candidate pair slice, sorted lexicographically with no self-pairs.
    ///
    /// It is valid only while requires_rebuild returns Ok(false) for the current
    /// points and same configuration. Callers must still apply the exact
    /// current interaction-radius test on every tick.
    pub fn pairs(&self) -> &[NeighborPair] {
        &self.pairs
    }

    /// Number of buffered candidate pairs.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether the buffered candidate list contains no pairs.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Interaction radius represented by exact per-tick filtering.
    pub fn interaction_radius(&self) -> f64 {
        self.interaction_radius
    }

    /// Buffered candidate radius.
    pub fn build_radius(&self) -> f64 {
        self.build_radius
    }

    /// Configured safety margin.
    pub fn skin(&self) -> f64 {
        self.skin
    }

    fn validate_configuration(
        interaction_radius: f64,
        skin: f64,
    ) -> Result<(), NeighborListError> {
        if !interaction_radius.is_finite() || interaction_radius < 0.0 {
            return Err(NeighborListError::InvalidInteractionRadius);
        }
        if !skin.is_finite() || skin <= 0.0 {
            return Err(NeighborListError::InvalidSkin);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn point(id: u64, x: f64, y: f64) -> SpatialPoint {
        SpatialPoint { id, x, y }
    }

    fn exact_pairs(points: &[SpatialPoint], radius: f64) -> Vec<NeighborPair> {
        let mut pairs = Vec::new();
        let radius_squared = radius * radius;
        for (index, a) in points.iter().enumerate() {
            for b in points.iter().skip(index + 1) {
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                if dx * dx + dy * dy <= radius_squared {
                    pairs.push(NeighborPair::new(a.id, b.id));
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }

    fn lcg(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    #[test]
    fn buffered_pairs_cover_exact_pairs_under_bounded_random_motion() {
        let mut seed = 0xb8ad_7c31_4f12_aa09_u64;
        for trial in 0..64 {
            let mut initial = Vec::new();
            for id in 0..128 {
                let x = ((lcg(&mut seed) >> 16) % 20_001) as f64 / 100.0;
                let y = ((lcg(&mut seed) >> 16) % 20_001) as f64 / 100.0;
                initial.push(point(id, x, y));
            }
            let cache = NeighborList2d::build(initial.clone(), 2.0, 1.0, 2.0).unwrap();
            let mut moved = initial.clone();
            for item in &mut moved {
                // Each point moves at most 0.49, strictly below skin / 2.
                let dx = (((lcg(&mut seed) >> 16) % 9_801) as f64 / 10_000.0) - 0.49;
                let dy = (((lcg(&mut seed) >> 16) % 9_801) as f64 / 10_000.0) - 0.49;
                let magnitude = dx.hypot(dy);
                let scale = if magnitude > 0.49 { 0.49 / magnitude } else { 1.0 };
                item.x += dx * scale;
                item.y += dy * scale;
            }
            assert!(
                !cache
                    .requires_rebuild(&moved, 2.0, 1.0, 2.0)
                    .unwrap(),
                "bounded-motion trial {trial} unexpectedly requires rebuild"
            );

            let cached = cache.pairs().iter().copied().collect::<BTreeSet<_>>();
            for pair in exact_pairs(&moved, 2.0) {
                assert!(cached.contains(&pair), "trial {trial} missed pair {pair:?}");
            }
        }
    }

    #[test]
    fn opposing_motion_at_buffer_boundary_is_covered_or_forces_rebuild() {
        let initial = [point(10, 0.0, 0.0), point(20, 3.0, 0.0)];
        let cache = NeighborList2d::build(initial, 1.0, 2.0, 1.0).unwrap();
        assert!(cache.pairs().contains(&NeighborPair { a: 10, b: 20 }));

        let moved = [point(10, 0.99, 0.0), point(20, 2.01, 0.0)];
        assert!(!cache.requires_rebuild(&moved, 1.0, 2.0, 1.0).unwrap());
        assert!(exact_pairs(&moved, 1.0).contains(&NeighborPair { a: 10, b: 20 }));

        let threshold = [point(10, 1.0, 0.0), point(20, 2.0, 0.0)];
        assert!(cache.requires_rebuild(&threshold, 1.0, 2.0, 1.0).unwrap());
    }

    #[test]
    fn changed_topology_or_configuration_requires_rebuild() {
        let initial = [point(1, 0.0, 0.0), point(2, 1.0, 0.0)];
        let cache = NeighborList2d::build(initial, 1.0, 1.0, 1.0).unwrap();

        assert!(cache
            .requires_rebuild(&[point(1, 0.0, 0.0)], 1.0, 1.0, 1.0)
            .unwrap());
        assert!(cache
            .requires_rebuild(
                &[point(1, 0.0, 0.0), point(3, 1.0, 0.0)],
                1.0,
                1.0,
                1.0
            )
            .unwrap());
        assert!(cache
            .requires_rebuild(&initial, 1.25, 1.0, 1.0)
            .unwrap());
        assert!(cache
            .requires_rebuild(&initial, 1.0, 1.5, 1.0)
            .unwrap());
        assert!(cache
            .requires_rebuild(&initial, 1.0, 1.0, 0.5)
            .unwrap());
    }

    #[test]
    fn point_order_does_not_change_pairs_or_invalidation_decision() {
        let points = [
            point(9, 1.0, 0.0),
            point(3, 0.0, 0.0),
            point(6, 2.0, 0.0),
        ];
        let cache = NeighborList2d::build(points, 1.0, 1.0, 1.0).unwrap();
        assert!(cache
            .pairs()
            .windows(2)
            .all(|window| window[0] < window[1]));
        assert!(cache.pairs().iter().all(|pair| pair.a < pair.b));
        assert!(!cache
            .requires_rebuild(&[points[2], points[0], points[1]], 1.0, 1.0, 1.0)
            .unwrap());
    }

    #[test]
    fn invalid_motion_inputs_fail_closed() {
        let cache = NeighborList2d::build([point(1, 0.0, 0.0)], 1.0, 1.0, 1.0).unwrap();
        assert_eq!(
            cache.requires_rebuild(&[point(1, f64::NAN, 0.0)], 1.0, 1.0, 1.0),
            Err(NeighborListError::NonFinitePoint { id: 1 })
        );
        assert_eq!(
            cache.requires_rebuild(
                &[point(1, 0.0, 0.0), point(1, 1.0, 0.0)],
                1.0,
                1.0,
                1.0
            ),
            Err(NeighborListError::DuplicateId { id: 1 })
        );
    }

    #[test]
    fn build_rejects_invalid_configuration_and_bad_identity() {
        assert_eq!(
            NeighborList2d::build([point(1, 0.0, 0.0)], -1.0, 1.0, 1.0).unwrap_err(),
            NeighborListError::InvalidInteractionRadius
        );
        assert_eq!(
            NeighborList2d::build([point(1, 0.0, 0.0)], 1.0, 0.0, 1.0).unwrap_err(),
            NeighborListError::InvalidSkin
        );
        assert_eq!(
            NeighborList2d::build([point(1, 0.0, 0.0)], f64::MAX, 1.0, 1.0).unwrap_err(),
            NeighborListError::BufferedRadiusOverflow
        );
        assert_eq!(
            NeighborList2d::build(
                [point(1, 0.0, 0.0), point(1, 2.0, 0.0)],
                1.0,
                1.0,
                1.0
            )
            .unwrap_err(),
            NeighborListError::DuplicateId { id: 1 }
        );
    }
}
