// SPDX-License-Identifier: AGPL-3.0-or-later

//! Axis-aligned space-time intersection weights for the EHD impulse remapper.
//!
//! N includes spatial axes plus time: N=2 is 1D space × time, N=3 is 2D
//! space × time, and N=4 is 3D space × time. Only axis-aligned boxes are
//! supported; curved, rotated and general unstructured elements are out of scope.

use crate::exchange::{
    ImpulseEvent, MappingError, MappingTolerance, OverlapWeight, map_integrated_impulses,
};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpaceTimeBox<const N: usize> {
    pub id: u64,
    pub lower: [f64; N],
    pub upper: [f64; N],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasureTolerance {
    pub absolute: f64,
    pub relative: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeometryError {
    ZeroDimensions,
    InvalidTolerance,
    InvalidBox(u64),
    DuplicateDestinationId(u64),
    NonFiniteMeasure(u64),
    OverlappingDestinationIntersections {
        first_id: u64,
        second_id: u64,
        overlap_measure: f64,
        allowed_measure: f64,
    },
    /// Conservative sum of all pairwise intersection measures exceeds the
    /// source-cell overlap budget. This upper-bounds multiply-covered measure.
    OverlapBudgetExceeded {
        pairwise_overlap_upper_bound: f64,
        allowed_measure: f64,
    },
    CoverageMismatch {
        source_id: u64,
        source_measure: f64,
        covered_measure: f64,
        allowed_measure: f64,
    },
    InvalidIntersectionFraction(u64),
    ImpulseKernelRejected(String),
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimensions => write!(f, "space-time boxes must have at least one dimension"),
            Self::InvalidTolerance => {
                write!(f, "geometry tolerance must be finite and non-negative")
            }
            Self::InvalidBox(id) => {
                write!(f, "space-time box {id} has invalid or non-finite bounds")
            }
            Self::DuplicateDestinationId(id) => write!(f, "duplicate destination box id: {id}"),
            Self::NonFiniteMeasure(id) => {
                write!(f, "space-time measure overflow/underflow for box {id}")
            }
            Self::OverlappingDestinationIntersections {
                first_id,
                second_id,
                overlap_measure,
                allowed_measure,
            } => write!(
                f,
                "destination boxes {first_id} and {second_id} overlap inside source by {overlap_measure}, allowed {allowed_measure}"
            ),
            Self::OverlapBudgetExceeded {
                pairwise_overlap_upper_bound,
                allowed_measure,
            } => write!(
                f,
                "cumulative pairwise-overlap upper bound {pairwise_overlap_upper_bound} exceeds source overlap budget {allowed_measure}"
            ),
            Self::CoverageMismatch {
                source_id,
                source_measure,
                covered_measure,
                allowed_measure,
            } => write!(
                f,
                "destination intersections cover {covered_measure} of source {source_id} measure {source_measure}; allowed measure error {allowed_measure}"
            ),
            Self::InvalidIntersectionFraction(id) => write!(
                f,
                "intersection fraction for destination {id} is outside [0, 1]"
            ),
            Self::ImpulseKernelRejected(reason) => write!(
                f,
                "integrated-impulse kernel rejected geometry self-check: {reason}"
            ),
        }
    }
}
impl Error for GeometryError {}

#[derive(Clone, Copy, Debug, Default)]
struct Kahan {
    sum: f64,
    correction: f64,
}
impl Kahan {
    fn add(&mut self, value: f64) {
        let y = value - self.correction;
        let next = self.sum + y;
        self.correction = (next - self.sum) - y;
        self.sum = next;
    }
}

fn validate_box<const N: usize>(cell: &SpaceTimeBox<N>) -> Result<f64, GeometryError> {
    if N == 0 {
        return Err(GeometryError::ZeroDimensions);
    }
    let mut measure = 1.0;
    for axis in 0..N {
        let lower = cell.lower[axis];
        let upper = cell.upper[axis];
        if !lower.is_finite() || !upper.is_finite() || upper <= lower {
            return Err(GeometryError::InvalidBox(cell.id));
        }
        measure *= upper - lower;
        if !measure.is_finite() || measure <= 0.0 {
            return Err(GeometryError::NonFiniteMeasure(cell.id));
        }
    }
    Ok(measure)
}

fn pair_intersection_measure<const N: usize>(
    left: &SpaceTimeBox<N>,
    right: &SpaceTimeBox<N>,
) -> Result<f64, GeometryError> {
    let mut measure = 1.0;
    for axis in 0..N {
        let lower = left.lower[axis].max(right.lower[axis]);
        let upper = left.upper[axis].min(right.upper[axis]);
        if upper <= lower {
            return Ok(0.0);
        }
        measure *= upper - lower;
        if !measure.is_finite() {
            return Err(GeometryError::NonFiniteMeasure(left.id));
        }
    }
    Ok(measure)
}

fn triple_intersection_measure<const N: usize>(
    source: &SpaceTimeBox<N>,
    left: &SpaceTimeBox<N>,
    right: &SpaceTimeBox<N>,
) -> Result<f64, GeometryError> {
    let mut measure = 1.0;
    for axis in 0..N {
        let lower = source.lower[axis]
            .max(left.lower[axis])
            .max(right.lower[axis]);
        let upper = source.upper[axis]
            .min(left.upper[axis])
            .min(right.upper[axis]);
        if upper <= lower {
            return Ok(0.0);
        }
        measure *= upper - lower;
        if !measure.is_finite() {
            return Err(GeometryError::NonFiniteMeasure(source.id));
        }
    }
    Ok(measure)
}

/// Calculate source-to-destination space-time overlap fractions for one source box.
/// Destination intersections must be pairwise interior-disjoint and cover the
/// full source measure within the supplied case-specific tolerance.
pub fn build_overlap_weights<const N: usize>(
    source: &SpaceTimeBox<N>,
    destinations: &[SpaceTimeBox<N>],
    tolerance: MeasureTolerance,
) -> Result<Vec<OverlapWeight>, GeometryError> {
    if N == 0 {
        return Err(GeometryError::ZeroDimensions);
    }
    if !tolerance.absolute.is_finite()
        || !tolerance.relative.is_finite()
        || tolerance.absolute < 0.0
        || tolerance.relative < 0.0
    {
        return Err(GeometryError::InvalidTolerance);
    }
    let source_measure = validate_box(source)?;
    let allowed = tolerance.absolute + tolerance.relative * source_measure;
    if !allowed.is_finite() {
        return Err(GeometryError::InvalidTolerance);
    }

    let mut ids = BTreeSet::new();
    let mut intersections: Vec<(SpaceTimeBox<N>, f64)> = Vec::new();
    for destination in destinations {
        validate_box(destination)?;
        if !ids.insert(destination.id) {
            return Err(GeometryError::DuplicateDestinationId(destination.id));
        }
        let measure = pair_intersection_measure(source, destination)?;
        if measure > 0.0 {
            intersections.push((*destination, measure));
        }
    }
    intersections.sort_by_key(|(cell, _)| cell.id);

    // Detect overlaps independently of the total coverage sum: equal-sized
    // overlap and gap defects must not cancel and pass as a complete partition.
    //
    // The sum of pairwise intersection measures is a conservative upper bound
    // on the measure covered more than once: at a point covered by k boxes it
    // counts C(k, 2), which is >= k - 1 for k >= 2. Checking only each pair
    // independently is insufficient because many individually sub-tolerance
    // overlaps can exceed the total source-cell error budget.
    let mut pairwise_overlap_upper_bound = Kahan::default();
    for i in 0..intersections.len() {
        for j in (i + 1)..intersections.len() {
            let overlap =
                triple_intersection_measure(source, &intersections[i].0, &intersections[j].0)?;
            if overlap > allowed {
                return Err(GeometryError::OverlappingDestinationIntersections {
                    first_id: intersections[i].0.id,
                    second_id: intersections[j].0.id,
                    overlap_measure: overlap,
                    allowed_measure: allowed,
                });
            }
            pairwise_overlap_upper_bound.add(overlap);
        }
    }
    if pairwise_overlap_upper_bound.sum > allowed {
        return Err(GeometryError::OverlapBudgetExceeded {
            pairwise_overlap_upper_bound: pairwise_overlap_upper_bound.sum,
            allowed_measure: allowed,
        });
    }

    let mut covered = Kahan::default();
    for (_, measure) in &intersections {
        covered.add(*measure);
    }
    if (covered.sum - source_measure).abs() > allowed {
        return Err(GeometryError::CoverageMismatch {
            source_id: source.id,
            source_measure,
            covered_measure: covered.sum,
            allowed_measure: allowed,
        });
    }

    let mut weights = Vec::with_capacity(intersections.len());
    for (destination, measure) in intersections {
        let fraction = measure / source_measure;
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(GeometryError::InvalidIntersectionFraction(destination.id));
        }
        weights.push(OverlapWeight {
            source_event_id: source.id,
            destination_cell_id: destination.id,
            fraction,
        });
    }
    Ok(weights)
}

/// End-to-end synthetic check: a 3D-space × time partition is geometrically
/// validated, then the resulting weights are consumed by the impulse remapper.
pub fn reference_self_check() -> Result<(), GeometryError> {
    let source = SpaceTimeBox {
        id: 1,
        lower: [0.0, 0.0, 0.0, 0.0],
        upper: [2.0, 1.0, 1.0, 2.0],
    };
    let destinations = [
        SpaceTimeBox {
            id: 10,
            lower: [0.0, 0.0, 0.0, 0.0],
            upper: [1.0, 1.0, 1.0, 2.0],
        },
        SpaceTimeBox {
            id: 20,
            lower: [1.0, 0.0, 0.0, 0.0],
            upper: [2.0, 1.0, 1.0, 2.0],
        },
    ];
    let weights = build_overlap_weights(
        &source,
        &destinations,
        MeasureTolerance {
            absolute: 1.0e-12,
            relative: 1.0e-12,
        },
    )?;
    let report = map_integrated_impulses(
        &[ImpulseEvent {
            source_event_id: 1,
            impulse_ns: [8.0, -4.0, 2.0],
        }],
        &weights,
        MappingTolerance {
            fraction_sum_abs: 1.0e-12,
            impulse_abs_ns: 1.0e-12,
            impulse_rel: 1.0e-12,
        },
    )
    .map_err(|error: MappingError| GeometryError::ImpulseKernelRejected(error.to_string()))?;

    if report.destination_total_ns != [8.0, -4.0, 2.0]
        || report.destination_impulses_ns.get(&10) != Some(&[4.0, -2.0, 1.0])
        || report.destination_impulses_ns.get(&20) != Some(&[4.0, -2.0, 1.0])
    {
        return Err(GeometryError::ImpulseKernelRejected(
            "synthetic space-time partition did not preserve expected impulse".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tol() -> MeasureTolerance {
        MeasureTolerance {
            absolute: 1.0e-12,
            relative: 1.0e-12,
        }
    }

    #[test]
    fn reference_space_time_backend_self_check_passes() {
        reference_self_check().unwrap();
    }

    #[test]
    fn one_dimensional_space_by_time_partition_is_exact() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [2.0, 2.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 2,
                lower: [0.0, 0.0],
                upper: [1.0, 2.0],
            },
            SpaceTimeBox {
                id: 3,
                lower: [1.0, 0.0],
                upper: [2.0, 2.0],
            },
        ];
        let weights = build_overlap_weights(&source, &dest, tol()).unwrap();
        assert_eq!(weights.len(), 2);
        assert_eq!(weights[0].fraction, 0.5);
        assert_eq!(weights[1].fraction, 0.5);
    }

    #[test]
    fn two_dimensional_space_by_time_partition_is_exact() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0, 0.0],
            upper: [2.0, 2.0, 1.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 2,
                lower: [0.0, 0.0, 0.0],
                upper: [2.0, 1.0, 1.0],
            },
            SpaceTimeBox {
                id: 3,
                lower: [0.0, 1.0, 0.0],
                upper: [2.0, 2.0, 1.0],
            },
        ];
        let weights = build_overlap_weights(&source, &dest, tol()).unwrap();
        assert_eq!(weights.len(), 2);
        assert_eq!(weights[0].fraction, 0.5);
        assert_eq!(weights[1].fraction, 0.5);
    }

    #[test]
    fn three_dimensional_space_by_time_partition_is_exact() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0, 0.0, 0.0],
            upper: [2.0, 2.0, 2.0, 2.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 2,
                lower: [0.0, 0.0, 0.0, 0.0],
                upper: [2.0, 2.0, 1.0, 2.0],
            },
            SpaceTimeBox {
                id: 3,
                lower: [0.0, 0.0, 1.0, 0.0],
                upper: [2.0, 2.0, 2.0, 2.0],
            },
        ];
        let weights = build_overlap_weights(&source, &dest, tol()).unwrap();
        assert_eq!(weights.len(), 2);
        assert_eq!(weights[0].fraction, 0.5);
        assert_eq!(weights[1].fraction, 0.5);
    }

    #[test]
    fn temporal_slabs_can_be_repartitioned_independently() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0, 0.0, 0.0],
            upper: [1.0, 1.0, 1.0, 2.0],
        };
        let destinations = [
            SpaceTimeBox {
                id: 2,
                lower: [0.0, 0.0, 0.0, 0.0],
                upper: [1.0, 1.0, 1.0, 0.5],
            },
            SpaceTimeBox {
                id: 3,
                lower: [0.0, 0.0, 0.0, 0.5],
                upper: [1.0, 1.0, 1.0, 2.0],
            },
        ];
        let weights = build_overlap_weights(&source, &destinations, tol()).unwrap();
        assert_eq!(weights[0].fraction, 0.25);
        assert_eq!(weights[1].fraction, 0.75);
    }

    #[test]
    fn rejects_coverage_gap() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [2.0, 2.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 2,
                lower: [0.0, 0.0],
                upper: [0.9, 2.0],
            },
            SpaceTimeBox {
                id: 3,
                lower: [1.1, 0.0],
                upper: [2.0, 2.0],
            },
        ];
        assert!(matches!(
            build_overlap_weights(&source, &dest, tol()).unwrap_err(),
            GeometryError::CoverageMismatch { source_id: 1, .. }
        ));
    }

    #[test]
    fn rejects_equal_measure_overlap_and_gap_adversary() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [2.0, 2.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 2,
                lower: [0.0, 0.0],
                upper: [1.1, 2.0],
            },
            SpaceTimeBox {
                id: 3,
                lower: [0.9, 0.0],
                upper: [1.8, 2.0],
            },
        ];
        assert!(matches!(
            build_overlap_weights(&source, &dest, tol()).unwrap_err(),
            GeometryError::OverlappingDestinationIntersections {
                first_id: 2,
                second_id: 3,
                ..
            }
        ));
    }

    #[test]
    fn rejects_cumulative_sub_tolerance_overlaps_hidden_by_a_compensating_gap() {
        // Each individual overlap is 0.06, below the 0.10 overlap budget.
        // The total pairwise-overlap upper bound is 0.12, above budget.
        // A 0.12 gap makes the sum of destination measures exactly equal
        // to the source measure, so a coverage-sum-only test would pass.
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [1.0, 1.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 2,
                lower: [0.00, 0.0],
                upper: [0.30, 1.0],
            },
            SpaceTimeBox {
                id: 3,
                lower: [0.24, 0.0],
                upper: [0.54, 1.0],
            },
            SpaceTimeBox {
                id: 4,
                lower: [0.48, 0.0],
                upper: [0.78, 1.0],
            },
            SpaceTimeBox {
                id: 5,
                lower: [0.90, 0.0],
                upper: [1.00, 1.0],
            },
        ];
        let budget = MeasureTolerance {
            absolute: 0.10,
            relative: 0.0,
        };
        assert!(matches!(
            build_overlap_weights(&source, &dest, budget).unwrap_err(),
            GeometryError::OverlapBudgetExceeded {
                pairwise_overlap_upper_bound,
                allowed_measure
            } if (pairwise_overlap_upper_bound - 0.12).abs() < 1.0e-12
                && (allowed_measure - 0.10).abs() < 1.0e-12
        ));
    }

    #[test]
    fn accepts_cumulative_overlap_within_declared_budget() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [1.0, 1.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 2,
                lower: [0.00, 0.0],
                upper: [0.50, 1.0],
            },
            SpaceTimeBox {
                id: 3,
                lower: [0.49, 0.0],
                upper: [1.00, 1.0],
            },
        ];
        let budget = MeasureTolerance {
            absolute: 0.02,
            relative: 0.0,
        };
        assert!(build_overlap_weights(&source, &dest, budget).is_ok());
    }

    #[test]
    fn boundary_touching_destination_boxes_are_not_overlapping() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [2.0, 1.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 2,
                lower: [0.0, 0.0],
                upper: [1.0, 1.0],
            },
            SpaceTimeBox {
                id: 3,
                lower: [1.0, 0.0],
                upper: [2.0, 1.0],
            },
        ];
        assert_eq!(
            build_overlap_weights(&source, &dest, tol()).unwrap().len(),
            2
        );
    }

    #[test]
    fn rejects_duplicate_destination_ids() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [1.0, 1.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 2,
                lower: [0.0, 0.0],
                upper: [0.5, 1.0],
            },
            SpaceTimeBox {
                id: 2,
                lower: [0.5, 0.0],
                upper: [1.0, 1.0],
            },
        ];
        assert_eq!(
            build_overlap_weights(&source, &dest, tol()).unwrap_err(),
            GeometryError::DuplicateDestinationId(2)
        );
    }

    #[test]
    fn rejects_zero_width_and_non_finite_bounds() {
        let zero = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [0.0, 1.0],
        };
        assert_eq!(
            build_overlap_weights(&zero, &[], tol()).unwrap_err(),
            GeometryError::InvalidBox(1)
        );
        let infinite = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [f64::INFINITY, 1.0],
        };
        assert_eq!(
            build_overlap_weights(&infinite, &[], tol()).unwrap_err(),
            GeometryError::InvalidBox(1)
        );
    }

    #[test]
    fn destination_input_order_does_not_change_weights() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [2.0, 2.0],
        };
        let dest = [
            SpaceTimeBox {
                id: 3,
                lower: [1.0, 0.0],
                upper: [2.0, 2.0],
            },
            SpaceTimeBox {
                id: 2,
                lower: [0.0, 0.0],
                upper: [1.0, 2.0],
            },
        ];
        let a = build_overlap_weights(&source, &dest, tol()).unwrap();
        let b = build_overlap_weights(&source, &[dest[1], dest[0]], tol()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_missing_destination_coverage() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [1.0, 1.0],
        };
        assert!(matches!(
            build_overlap_weights::<2>(&source, &[], tol()).unwrap_err(),
            GeometryError::CoverageMismatch { source_id: 1, .. }
        ));
    }

    #[test]
    fn rejects_invalid_geometry_tolerance() {
        let source = SpaceTimeBox {
            id: 1,
            lower: [0.0, 0.0],
            upper: [1.0, 1.0],
        };
        let invalid = MeasureTolerance {
            absolute: -1.0,
            relative: 0.0,
        };
        assert_eq!(
            build_overlap_weights(&source, &[], invalid).unwrap_err(),
            GeometryError::InvalidTolerance
        );
    }
}
