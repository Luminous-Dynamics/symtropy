// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Deterministic broadphase for radius-based 2D interactions.
//!
//! Returns a superset of points inside a Euclidean radius by querying the
//! radius-aligned square. Callers must retain their exact distance predicate,
//! including its original strict/inclusive boundary rule. Candidate points are
//! returned in ascending stable-ID order; ECS iteration order is not a
//! determinism guarantee.
//!
//! This is a derived index, never the source of truth. Keep a simple scan as a
//! differential oracle before replacing production interaction loops.

use std::collections::BTreeMap;

/// Stable application-owned point identity and 2D position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialPoint {
    pub id: u64,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Cell(i64, i64);

/// Construction/query failures that cannot be safely interpreted as an empty result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpatialGridError {
    InvalidCellSize,
    InvalidRadius,
    NonFinitePoint { id: u64 },
    NonFiniteQuery,
    DuplicateId { id: u64 },
    PointOutsideIndexRange { id: u64 },
}

impl core::fmt::Display for SpatialGridError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidCellSize => write!(f, "cell size must be finite and positive"),
            Self::InvalidRadius => write!(f, "query radius must be finite and non-negative"),
            Self::NonFinitePoint { id } => write!(f, "point {id} has a non-finite coordinate"),
            Self::NonFiniteQuery => write!(f, "query center must have finite coordinates"),
            Self::DuplicateId { id } => write!(f, "duplicate spatial point ID {id}"),
            Self::PointOutsideIndexRange { id } => {
                write!(f, "point {id} is outside the representable grid range")
            }
        }
    }
}

impl std::error::Error for SpatialGridError {}

/// Deterministic uniform-grid index over stable-ID 2D points.
///
/// Construction sorts points by ID and rejects duplicate IDs. Each point is
/// stored in exactly one cell. The BTreeMap gives deterministic cell structure;
/// returned candidate lists are sorted by stable ID independently of cell walk
/// order.
#[derive(Clone, Debug)]
pub struct SpatialGrid2d {
    cell_size: f64,
    points: Vec<SpatialPoint>,
    cells: BTreeMap<Cell, Vec<usize>>,
}

impl SpatialGrid2d {
    /// Build the index. Cell size is in the same units as the coordinates.
    pub fn new(
        points: impl IntoIterator<Item = SpatialPoint>,
        cell_size: f64,
    ) -> Result<Self, SpatialGridError> {
        if !cell_size.is_finite() || cell_size <= 0.0 {
            return Err(SpatialGridError::InvalidCellSize);
        }

        let mut points: Vec<_> = points.into_iter().collect();
        points.sort_unstable_by_key(|point| point.id);

        for point in &points {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(SpatialGridError::NonFinitePoint { id: point.id });
            }
        }
        for pair in points.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(SpatialGridError::DuplicateId { id: pair[0].id });
            }
        }

        let mut cells: BTreeMap<Cell, Vec<usize>> = BTreeMap::new();
        for (index, point) in points.iter().enumerate() {
            let cell = Self::cell_for(point.x, point.y, cell_size)
                .ok_or(SpatialGridError::PointOutsideIndexRange { id: point.id })?;
            cells.entry(cell).or_default().push(index);
        }

        Ok(Self {
            cell_size,
            points,
            cells,
        })
    }

    /// Return broadphase candidates for a radius query.
    ///
    /// Every point whose Euclidean distance from center is at most radius is
    /// guaranteed to be present. False positives are expected; apply the
    /// caller's original exact distance check before mutating simulation state.
    ///
    /// When a query spans too many cells, or its bounds exceed representable
    /// cell coordinates, this falls back to every point rather than risking a
    /// missed neighbor. This preserves correctness but may not improve cost for
    /// huge radii or adversarial coordinate ranges.
    pub fn query_radius_candidates(
        &self,
        center: [f64; 2],
        radius: f64,
    ) -> Result<Vec<SpatialPoint>, SpatialGridError> {
        if !center[0].is_finite() || !center[1].is_finite() {
            return Err(SpatialGridError::NonFiniteQuery);
        }
        if !radius.is_finite() || radius < 0.0 {
            return Err(SpatialGridError::InvalidRadius);
        }
        if self.points.is_empty() {
            return Ok(Vec::new());
        }

        let min_x = center[0] - radius;
        let max_x = center[0] + radius;
        let min_y = center[1] - radius;
        let max_y = center[1] + radius;
        let Some((low_x, high_x)) = Self::cell_range(min_x, max_x, self.cell_size) else {
            return Ok(self.points.clone());
        };
        let Some((low_y, high_y)) = Self::cell_range(min_y, max_y, self.cell_size) else {
            return Ok(self.points.clone());
        };

        let width = (high_x as i128 - low_x as i128 + 1) as u128;
        let height = (high_y as i128 - low_y as i128 + 1) as u128;
        let cell_count = width.checked_mul(height);
        let scan_limit = self.points.len().saturating_mul(16).max(4096) as u128;
        if cell_count.is_none_or(|count| count > scan_limit) {
            return Ok(self.points.clone());
        }

        let mut indices = Vec::new();
        for x in low_x..=high_x {
            for y in low_y..=high_y {
                if let Some(cell_indices) = self.cells.get(&Cell(x, y)) {
                    indices.extend_from_slice(cell_indices);
                }
            }
        }
        // Every point is stored in one cell. Sorting indices restores the
        // stable-ID order and makes results independent of cell traversal order.
        indices.sort_unstable();
        indices.dedup();
        Ok(indices.into_iter().map(|index| self.points[index]).collect())
    }

    /// Number of indexed points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether the index contains no points.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Configured cell width in coordinate units.
    pub fn cell_size(&self) -> f64 {
        self.cell_size
    }

    fn cell_range(low: f64, high: f64, cell_size: f64) -> Option<(i64, i64)> {
        if !low.is_finite() || !high.is_finite() {
            return None;
        }
        let min = Self::cell_index(low, cell_size)?;
        let max = Self::cell_index(high, cell_size)?;
        (min <= max).then_some((min, max))
    }

    fn cell_for(x: f64, y: f64, cell_size: f64) -> Option<Cell> {
        Some(Cell(
            Self::cell_index(x, cell_size)?,
            Self::cell_index(y, cell_size)?,
        ))
    }

    fn cell_index(coordinate: f64, cell_size: f64) -> Option<i64> {
        let cell = (coordinate / cell_size).floor();
        // i64::MAX as f64 rounds up to +2^63; use an exclusive upper boundary
        // to avoid saturating float-to-integer casts.
        if !cell.is_finite()
            || cell < i64::MIN as f64
            || cell >= 9_223_372_036_854_775_808.0_f64
        {
            return None;
        }
        Some(cell as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_ids(points: &[SpatialPoint], center: [f64; 2], radius: f64) -> Vec<u64> {
        let radius_squared = radius * radius;
        let mut ids: Vec<_> = points
            .iter()
            .filter(|point| {
                let dx = point.x - center[0];
                let dy = point.y - center[1];
                dx * dx + dy * dy <= radius_squared
            })
            .map(|point| point.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    fn lcg(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    #[test]
    fn indexed_candidates_never_omit_exact_neighbors_in_seeded_corpus() {
        let mut seed = 0x7a31_89c5_d4e2_106f_u64;
        let mut points = Vec::new();
        for id in 0..4096 {
            let x = ((lcg(&mut seed) >> 16) % 40_001) as f64 / 100.0 - 200.0;
            let y = ((lcg(&mut seed) >> 16) % 40_001) as f64 / 100.0 - 200.0;
            points.push(SpatialPoint { id, x, y });
        }

        let grid = SpatialGrid2d::new(points.clone(), 3.75).unwrap();
        for query_index in 0..256 {
            let center = [
                ((lcg(&mut seed) >> 16) % 40_001) as f64 / 100.0 - 200.0,
                ((lcg(&mut seed) >> 16) % 40_001) as f64 / 100.0 - 200.0,
            ];
            let radius = [0.0, 0.1, 1.25, 3.75, 8.0, 25.0][query_index % 6];
            let exact = exact_ids(&points, center, radius);
            let candidates = grid.query_radius_candidates(center, radius).unwrap();
            let candidate_ids: Vec<_> = candidates.iter().map(|point| point.id).collect();

            // The broadphase can overselect, but exact filtering must match the
            // simple full scan with no missing hits or extra accepted neighbors.
            assert_eq!(exact_ids(&candidates, center, radius), exact);
            assert!(candidate_ids.windows(2).all(|pair| pair[0] < pair[1]));
            for id in exact {
                assert!(
                    candidate_ids.binary_search(&id).is_ok(),
                    "missed true neighbor id {id}, center={center:?}, radius={radius}"
                );
            }
        }
    }

    #[test]
    fn query_is_independent_of_construction_order_and_candidates_are_sorted() {
        let points = vec![
            SpatialPoint { id: 9, x: -2.0, y: 3.0 },
            SpatialPoint { id: 2, x: 0.0, y: 0.0 },
            SpatialPoint { id: 7, x: 0.5, y: 0.5 },
            SpatialPoint { id: 1, x: 5.0, y: -5.0 },
        ];
        let forward = SpatialGrid2d::new(points.clone(), 1.0).unwrap();
        let reverse = SpatialGrid2d::new(points.into_iter().rev(), 1.0).unwrap();
        let a = forward.query_radius_candidates([0.0, 0.0], 2.0).unwrap();
        let b = reverse.query_radius_candidates([0.0, 0.0], 2.0).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.iter().map(|point| point.id).collect::<Vec<_>>(), vec![2, 7]);
    }

    #[test]
    fn cell_edges_negative_coordinates_and_zero_radius_are_safe() {
        let points = vec![
            SpatialPoint { id: 1, x: -1.0, y: -1.0 },
            SpatialPoint { id: 2, x: 0.0, y: 0.0 },
            SpatialPoint { id: 3, x: 1.0, y: 1.0 },
            SpatialPoint { id: 4, x: 2.0, y: 0.0 },
        ];
        let grid = SpatialGrid2d::new(points, 1.0).unwrap();
        let candidates = grid.query_radius_candidates([0.0, 0.0], 0.0).unwrap();
        assert!(candidates.iter().any(|point| point.id == 2));
        let exact = exact_ids(&candidates, [0.0, 0.0], 0.0);
        assert_eq!(exact, vec![2]);
        let edge = grid.query_radius_candidates([1.0, 0.0], 1.0).unwrap();
        assert!(exact_ids(&edge, [1.0, 0.0], 1.0).contains(&4));
    }

    #[test]
    fn large_or_unrepresentable_query_falls_back_to_all_points() {
        let points = vec![
            SpatialPoint { id: 3, x: -10.0, y: 0.0 },
            SpatialPoint { id: 1, x: 0.0, y: 0.0 },
            SpatialPoint { id: 2, x: 10.0, y: 0.0 },
        ];
        let grid = SpatialGrid2d::new(points, 1.0).unwrap();
        assert_eq!(grid.query_radius_candidates([0.0, 0.0], 1.0e100).unwrap().len(), 3);
        assert_eq!(
            grid.query_radius_candidates([9.3e18, 0.0], 1.0).unwrap().len(),
            3
        );
    }

    #[test]
    fn malformed_inputs_fail_closed() {
        assert_eq!(
            SpatialGrid2d::new(std::iter::empty::<SpatialPoint>(), 0.0).unwrap_err(),
            SpatialGridError::InvalidCellSize
        );
        assert_eq!(
            SpatialGrid2d::new([SpatialPoint { id: 1, x: f64::NAN, y: 0.0 }], 1.0)
                .unwrap_err(),
            SpatialGridError::NonFinitePoint { id: 1 }
        );
        assert_eq!(
            SpatialGrid2d::new(
                [
                    SpatialPoint { id: 1, x: 0.0, y: 0.0 },
                    SpatialPoint { id: 1, x: 1.0, y: 1.0 },
                ],
                1.0
            )
            .unwrap_err(),
            SpatialGridError::DuplicateId { id: 1 }
        );
        let grid = SpatialGrid2d::new(std::iter::empty::<SpatialPoint>(), 1.0).unwrap();
        assert_eq!(
            grid.query_radius_candidates([0.0, f64::INFINITY], 1.0),
            Err(SpatialGridError::NonFiniteQuery)
        );
        assert_eq!(
            grid.query_radius_candidates([0.0, 0.0], -1.0),
            Err(SpatialGridError::InvalidRadius)
        );
    }

    #[test]
    fn point_outside_representable_index_range_is_rejected() {
        assert_eq!(
            SpatialGrid2d::new(
                [SpatialPoint { id: 8, x: 9.3e18, y: 0.0 }],
                1.0
            )
            .unwrap_err(),
            SpatialGridError::PointOutsideIndexRange { id: 8 }
        );
    }
}
