// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conservative remapping of already-integrated vector impulses.
//!
//! This is a reference kernel, not a geometric intersection engine. The caller
//! supplies non-overlapping space-time intersection fractions for each source
//! space-time element. Fractions are measured relative to the source element's
//! space-time measure. The source impulse is treated as piecewise constant over
//! that element (first-order reconstruction).
//!
//! The kernel deliberately maps extensive impulse [N s], not intensive force
//! density or velocity. It fails closed when coverage is incomplete, a source
//! is unmapped, or the global impulse residual exceeds the caller's
//! case-specific tolerance.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

/// Integrated impulse contributed by one source cell × time slab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpulseEvent {
    pub source_event_id: u64,
    pub impulse_ns: [f64; 3],
}

/// One source-to-destination space-time overlap contribution.
///
/// fraction is the measure of this intersection divided by the measure of
/// the complete source cell × time slab. The geometry/time backend must ensure
/// the intersections are non-overlapping and exhaustive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlapWeight {
    pub source_event_id: u64,
    pub destination_cell_id: u64,
    pub fraction: f64,
}

/// Case-specific acceptance tolerances. There is intentionally no universal
/// default: tolerances must come from the physical case's numerical budget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MappingTolerance {
    pub fraction_sum_abs: f64,
    pub impulse_abs_ns: f64,
    pub impulse_rel: f64,
}

/// Deterministic result of the conservative impulse transfer.
#[derive(Clone, Debug, PartialEq)]
pub struct MappingReport {
    pub destination_impulses_ns: BTreeMap<u64, [f64; 3]>,
    pub source_total_ns: [f64; 3],
    pub destination_total_ns: [f64; 3],
    /// Destination total minus source total, component by component.
    pub conservation_residual_ns: [f64; 3],
    pub max_source_coverage_error: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MappingError {
    EmptySourceEvents,
    InvalidTolerance(&'static str),
    DuplicateSourceEventId(u64),
    NonFiniteImpulse(u64),
    NonFiniteWeight(u64),
    NegativeWeight(u64),
    WeightExceedsOne(u64),
    UnknownSourceEvent(u64),
    UnmappedSourceEvent(u64),
    SourceCoverageMismatch {
        source_event_id: u64,
        fraction_sum: f64,
        tolerance: f64,
    },
    NonFiniteAccumulation,
    ConservationFailure {
        component: usize,
        residual_ns: f64,
        allowed_ns: f64,
    },
}

impl fmt::Display for MappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySourceEvents => write!(formatter, "no source impulse events supplied"),
            Self::InvalidTolerance(name) => write!(formatter, "invalid mapping tolerance: {name}"),
            Self::DuplicateSourceEventId(id) => {
                write!(formatter, "duplicate source event id: {id}")
            }
            Self::NonFiniteImpulse(id) => {
                write!(formatter, "source event {id} has a non-finite impulse")
            }
            Self::NonFiniteWeight(id) => write!(
                formatter,
                "source event {id} has a non-finite overlap weight"
            ),
            Self::NegativeWeight(id) => {
                write!(formatter, "source event {id} has a negative overlap weight")
            }
            Self::WeightExceedsOne(id) => write!(
                formatter,
                "source event {id} has an overlap fraction greater than one"
            ),
            Self::UnknownSourceEvent(id) => {
                write!(formatter, "overlap references unknown source event {id}")
            }
            Self::UnmappedSourceEvent(id) => {
                write!(formatter, "source event {id} has no destination overlap")
            }
            Self::SourceCoverageMismatch {
                source_event_id,
                fraction_sum,
                tolerance,
            } => write!(
                formatter,
                "source event {source_event_id} overlap fractions sum to {fraction_sum}, outside 1 ± {tolerance}"
            ),
            Self::NonFiniteAccumulation => {
                write!(formatter, "impulse accumulation became non-finite")
            }
            Self::ConservationFailure {
                component,
                residual_ns,
                allowed_ns,
            } => write!(
                formatter,
                "global impulse component {component} residual {residual_ns} N s exceeds allowed {allowed_ns} N s"
            ),
        }
    }
}

impl Error for MappingError {}

#[derive(Clone, Copy, Debug, Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let adjusted = value - self.correction;
        let next = self.sum + adjusted;
        self.correction = (next - self.sum) - adjusted;
        self.sum = next;
    }

    fn value(self) -> f64 {
        self.sum
    }
}

/// Conservatively map integrated impulses from source space-time elements to
/// destination cells.
///
/// The weights must describe a complete, non-overlapping partition of every
/// source element in space-time. Their fractions are relative to each source
/// element, so every source event's fractions must sum to one.
///
/// The kernel aggregates repeated intersections into the same destination
/// cell, uses deterministic ordering and compensated summation, and verifies
/// both per-source coverage and the global vector impulse balance. It does not
/// compute geometric intersections, rotate coordinate frames, reconstruct
/// higher-order fields, or infer missing boundary fluxes.
pub fn map_integrated_impulses(
    events: &[ImpulseEvent],
    weights: &[OverlapWeight],
    tolerance: MappingTolerance,
) -> Result<MappingReport, MappingError> {
    if events.is_empty() {
        return Err(MappingError::EmptySourceEvents);
    }
    validate_tolerance(tolerance)?;

    let mut sorted_events = events.to_vec();
    sorted_events.sort_by_key(|event| event.source_event_id);

    let mut event_ids = BTreeSet::new();
    for event in &sorted_events {
        if !event_ids.insert(event.source_event_id) {
            return Err(MappingError::DuplicateSourceEventId(event.source_event_id));
        }
        if !event
            .impulse_ns
            .iter()
            .all(|component| component.is_finite())
        {
            return Err(MappingError::NonFiniteImpulse(event.source_event_id));
        }
    }

    let known_ids = event_ids;
    let mut sorted_weights = weights.to_vec();
    sorted_weights.sort_by(|left, right| {
        left.source_event_id
            .cmp(&right.source_event_id)
            .then_with(|| left.destination_cell_id.cmp(&right.destination_cell_id))
            .then_with(|| left.fraction.total_cmp(&right.fraction))
    });

    let mut fractions_by_source: BTreeMap<u64, CompensatedSum> = BTreeMap::new();
    let mut fractions_by_destination: BTreeMap<(u64, u64), CompensatedSum> = BTreeMap::new();

    for weight in &sorted_weights {
        if !known_ids.contains(&weight.source_event_id) {
            return Err(MappingError::UnknownSourceEvent(weight.source_event_id));
        }
        if !weight.fraction.is_finite() {
            return Err(MappingError::NonFiniteWeight(weight.source_event_id));
        }
        if weight.fraction < 0.0 {
            return Err(MappingError::NegativeWeight(weight.source_event_id));
        }
        if weight.fraction > 1.0 {
            return Err(MappingError::WeightExceedsOne(weight.source_event_id));
        }

        fractions_by_source
            .entry(weight.source_event_id)
            .or_default()
            .add(weight.fraction);
        fractions_by_destination
            .entry((weight.source_event_id, weight.destination_cell_id))
            .or_default()
            .add(weight.fraction);
    }

    let event_by_id: BTreeMap<u64, ImpulseEvent> = sorted_events
        .iter()
        .copied()
        .map(|event| (event.source_event_id, event))
        .collect();

    // Do not renormalize incomplete or near-complete weights. Preserve the
    // supplied measure and expose any resulting impulse residual to the caller.
    let mut max_source_coverage_error: f64 = 0.0;
    for event in &sorted_events {
        let Some(sum) = fractions_by_source.get(&event.source_event_id).copied() else {
            return Err(MappingError::UnmappedSourceEvent(event.source_event_id));
        };
        let error = (sum.value() - 1.0).abs();
        max_source_coverage_error = max_source_coverage_error.max(error);
        if error > tolerance.fraction_sum_abs {
            return Err(MappingError::SourceCoverageMismatch {
                source_event_id: event.source_event_id,
                fraction_sum: sum.value(),
                tolerance: tolerance.fraction_sum_abs,
            });
        }
    }

    let mut destination_accumulators: BTreeMap<u64, [CompensatedSum; 3]> = BTreeMap::new();
    let mut source_total = [CompensatedSum::default(); 3];
    let mut source_absolute_total = [CompensatedSum::default(); 3];

    for event in &sorted_events {
        for (component, impulse) in event.impulse_ns.iter().copied().enumerate() {
            source_total[component].add(impulse);
            source_absolute_total[component].add(impulse.abs());
        }
    }

    for ((source_id, destination_id), fraction) in fractions_by_destination {
        let event = event_by_id
            .get(&source_id)
            .expect("source id was validated before accumulation");
        let destination = destination_accumulators
            .entry(destination_id)
            .or_insert([CompensatedSum::default(); 3]);

        for (component, impulse) in event.impulse_ns.iter().copied().enumerate() {
            destination[component].add(impulse * fraction.value());
        }
    }

    let mut destination_impulses_ns = BTreeMap::new();
    let mut destination_total = [CompensatedSum::default(); 3];
    for (destination_id, accumulator) in destination_accumulators {
        let impulse = [
            accumulator[0].value(),
            accumulator[1].value(),
            accumulator[2].value(),
        ];
        if !impulse.iter().all(|component| component.is_finite()) {
            return Err(MappingError::NonFiniteAccumulation);
        }
        for component in 0..3 {
            destination_total[component].add(impulse[component]);
        }
        destination_impulses_ns.insert(destination_id, impulse);
    }

    let source_total_ns = [
        source_total[0].value(),
        source_total[1].value(),
        source_total[2].value(),
    ];
    let destination_total_ns = [
        destination_total[0].value(),
        destination_total[1].value(),
        destination_total[2].value(),
    ];
    let mut conservation_residual_ns = [0.0; 3];

    for component in 0..3 {
        let source_scale = source_absolute_total[component].value();
        let allowed = tolerance.impulse_abs_ns + tolerance.impulse_rel * source_scale;
        let residual = destination_total_ns[component] - source_total_ns[component];
        if !allowed.is_finite() || !residual.is_finite() {
            return Err(MappingError::NonFiniteAccumulation);
        }
        conservation_residual_ns[component] = residual;
        if residual.abs() > allowed {
            return Err(MappingError::ConservationFailure {
                component,
                residual_ns: residual,
                allowed_ns: allowed,
            });
        }
    }

    Ok(MappingReport {
        destination_impulses_ns,
        source_total_ns,
        destination_total_ns,
        conservation_residual_ns,
        max_source_coverage_error,
    })
}

/// Deterministic executable smoke check used by the offline audit CLI.
/// This verifies the transfer arithmetic on a synthetic two-destination case;
/// it does not validate a geometric intersection backend or a plasma solver.
pub fn reference_self_check() -> Result<(), MappingError> {
    let tolerance = MappingTolerance {
        fraction_sum_abs: 1.0e-12,
        impulse_abs_ns: 1.0e-12,
        impulse_rel: 1.0e-12,
    };
    let report = map_integrated_impulses(
        &[ImpulseEvent {
            source_event_id: 1,
            impulse_ns: [4.0, -2.0, 1.0],
        }],
        &[
            OverlapWeight {
                source_event_id: 1,
                destination_cell_id: 10,
                fraction: 0.25,
            },
            OverlapWeight {
                source_event_id: 1,
                destination_cell_id: 20,
                fraction: 0.75,
            },
        ],
        tolerance,
    )?;

    if report.source_total_ns != [4.0, -2.0, 1.0]
        || report.destination_total_ns != [4.0, -2.0, 1.0]
        || report.conservation_residual_ns != [0.0, 0.0, 0.0]
        || report.destination_impulses_ns.get(&10) != Some(&[1.0, -0.5, 0.25])
        || report.destination_impulses_ns.get(&20) != Some(&[3.0, -1.5, 0.75])
    {
        return Err(MappingError::ConservationFailure {
            component: 0,
            residual_ns: report.destination_total_ns[0] - report.source_total_ns[0],
            allowed_ns: 0.0,
        });
    }

    Ok(())
}

fn validate_tolerance(tolerance: MappingTolerance) -> Result<(), MappingError> {
    for (name, value) in [
        ("fraction_sum_abs", tolerance.fraction_sum_abs),
        ("impulse_abs_ns", tolerance.impulse_abs_ns),
        ("impulse_rel", tolerance.impulse_rel),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(MappingError::InvalidTolerance(name));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerance() -> MappingTolerance {
        MappingTolerance {
            fraction_sum_abs: 1.0e-12,
            impulse_abs_ns: 1.0e-12,
            impulse_rel: 1.0e-12,
        }
    }

    fn event(id: u64, impulse_ns: [f64; 3]) -> ImpulseEvent {
        ImpulseEvent {
            source_event_id: id,
            impulse_ns,
        }
    }

    fn weight(source: u64, destination: u64, fraction: f64) -> OverlapWeight {
        OverlapWeight {
            source_event_id: source,
            destination_cell_id: destination,
            fraction,
        }
    }

    #[test]
    fn reference_self_check_passes() {
        reference_self_check().unwrap();
    }

    #[test]
    fn maps_identity_without_changing_integrated_impulse() {
        let result = map_integrated_impulses(
            &[event(7, [2.0, -3.0, 4.0])],
            &[weight(7, 70, 1.0)],
            tolerance(),
        )
        .unwrap();
        assert_eq!(result.destination_impulses_ns[&70], [2.0, -3.0, 4.0]);
        assert_eq!(result.source_total_ns, result.destination_total_ns);
        assert_eq!(result.conservation_residual_ns, [0.0; 3]);
    }

    #[test]
    fn maps_one_source_to_multiple_destination_cells() {
        let result = map_integrated_impulses(
            &[event(1, [12.0, -8.0, 4.0])],
            &[weight(1, 10, 0.25), weight(1, 20, 0.75)],
            tolerance(),
        )
        .unwrap();
        assert_eq!(result.destination_impulses_ns[&10], [3.0, -2.0, 1.0]);
        assert_eq!(result.destination_impulses_ns[&20], [9.0, -6.0, 3.0]);
        assert_eq!(result.destination_total_ns, [12.0, -8.0, 4.0]);
    }

    #[test]
    fn aggregates_multiple_source_events_into_one_destination() {
        let result = map_integrated_impulses(
            &[event(2, [1.0, 2.0, 3.0]), event(1, [4.0, 5.0, 6.0])],
            &[weight(1, 9, 1.0), weight(2, 9, 1.0)],
            tolerance(),
        )
        .unwrap();
        assert_eq!(result.destination_impulses_ns[&9], [5.0, 7.0, 9.0]);
    }

    #[test]
    fn split_space_time_partition_preserves_total_impulse() {
        let result = map_integrated_impulses(
            &[event(1, [2.0, 0.0, 0.0]), event(2, [6.0, 0.0, 0.0])],
            &[
                weight(1, 10, 0.5),
                weight(1, 11, 0.5),
                weight(2, 10, 0.25),
                weight(2, 12, 0.75),
            ],
            tolerance(),
        )
        .unwrap();
        assert_eq!(result.destination_total_ns, [8.0, 0.0, 0.0]);
        assert_eq!(result.destination_impulses_ns[&10], [2.5, 0.0, 0.0]);
        assert_eq!(result.destination_impulses_ns[&11], [1.0, 0.0, 0.0]);
        assert_eq!(result.destination_impulses_ns[&12], [4.5, 0.0, 0.0]);
    }

    #[test]
    fn mapping_is_deterministic_under_input_permutation() {
        let events = [event(2, [1.0e16, 1.0, -1.0]), event(1, [1.0, 2.0, 3.0])];
        let weights = [weight(2, 9, 0.5), weight(1, 8, 1.0), weight(2, 8, 0.5)];
        let forward = map_integrated_impulses(&events, &weights, tolerance()).unwrap();
        let reverse = map_integrated_impulses(
            &[events[1], events[0]],
            &[weights[2], weights[1], weights[0]],
            tolerance(),
        )
        .unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn rejects_incomplete_source_coverage() {
        let error = map_integrated_impulses(
            &[event(1, [1.0, 0.0, 0.0])],
            &[weight(1, 2, 0.75)],
            tolerance(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            MappingError::SourceCoverageMismatch {
                source_event_id: 1,
                ..
            }
        ));
    }

    #[test]
    fn rejects_an_unmapped_source_event() {
        let error =
            map_integrated_impulses(&[event(4, [1.0, 0.0, 0.0])], &[], tolerance()).unwrap_err();
        assert_eq!(error, MappingError::UnmappedSourceEvent(4));
    }

    #[test]
    fn rejects_weights_referencing_unknown_source_events() {
        let error = map_integrated_impulses(
            &[event(1, [1.0, 0.0, 0.0])],
            &[weight(99, 2, 1.0)],
            tolerance(),
        )
        .unwrap_err();
        assert_eq!(error, MappingError::UnknownSourceEvent(99));
    }

    #[test]
    fn rejects_negative_and_non_finite_weights() {
        let negative = map_integrated_impulses(
            &[event(1, [1.0, 0.0, 0.0])],
            &[weight(1, 2, -0.1)],
            tolerance(),
        )
        .unwrap_err();
        assert_eq!(negative, MappingError::NegativeWeight(1));

        let non_finite = map_integrated_impulses(
            &[event(1, [1.0, 0.0, 0.0])],
            &[weight(1, 2, f64::NAN)],
            tolerance(),
        )
        .unwrap_err();
        assert_eq!(non_finite, MappingError::NonFiniteWeight(1));
    }

    #[test]
    fn rejects_non_finite_source_impulse() {
        let error = map_integrated_impulses(
            &[event(1, [1.0, f64::INFINITY, 0.0])],
            &[weight(1, 2, 1.0)],
            tolerance(),
        )
        .unwrap_err();
        assert_eq!(error, MappingError::NonFiniteImpulse(1));
    }

    #[test]
    fn rejects_duplicate_source_event_ids() {
        let error = map_integrated_impulses(
            &[event(1, [1.0, 0.0, 0.0]), event(1, [2.0, 0.0, 0.0])],
            &[weight(1, 2, 1.0)],
            tolerance(),
        )
        .unwrap_err();
        assert_eq!(error, MappingError::DuplicateSourceEventId(1));
    }

    #[test]
    fn rejects_invalid_tolerances() {
        let invalid = MappingTolerance {
            fraction_sum_abs: -1.0,
            ..tolerance()
        };
        let error =
            map_integrated_impulses(&[event(1, [1.0, 0.0, 0.0])], &[weight(1, 2, 1.0)], invalid)
                .unwrap_err();
        assert_eq!(error, MappingError::InvalidTolerance("fraction_sum_abs"));
    }

    #[test]
    fn does_not_renormalize_coverage_within_loose_fraction_tolerance() {
        let permissive_coverage = MappingTolerance {
            fraction_sum_abs: 0.01,
            impulse_abs_ns: 1.0e-12,
            impulse_rel: 0.0,
        };
        let error = map_integrated_impulses(
            &[event(1, [10.0, 0.0, 0.0])],
            &[weight(1, 2, 0.999)],
            permissive_coverage,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            MappingError::ConservationFailure { component: 0, .. }
        ));
    }

    #[test]
    fn checks_each_source_even_when_global_impulses_cancel() {
        let error = map_integrated_impulses(
            &[event(1, [100.0, 0.0, 0.0]), event(2, [-100.0, 0.0, 0.0])],
            &[weight(1, 3, 0.5), weight(2, 4, 1.0)],
            tolerance(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            MappingError::SourceCoverageMismatch {
                source_event_id: 1,
                ..
            }
        ));
    }
}
