//! Uncertainty-aware, time-varying capacity assessment.
//!
//! The base cumulative-capacity oracle assumes a fixed resource capacity.
//! Construction resources often have a changing or uncertain available
//! capacity: temporary power derating, hoist restrictions, partially blocked
//! staging, weather-related equipment limits, or qualified floor-load bands.
//!
//! This module overlays owner-attested half-open availability windows on the
//! fixed-capacity oracle. It reports three outcomes:
//! - WithinLowerBound: demand is no greater than the lower qualified capacity;
//! - Indeterminate: demand exceeds the lower bound but not the upper bound;
//! - DefinitelyOverCapacity: demand exceeds even the upper bound.
//!
//! Only WithinLowerBound is robustly feasible. Indeterminate is not treated as
//! safe. Receipt digests and owner identities are bound here, but canonical
//! receipt bytes, source measurements and signatures remain caller-verified.

use std::collections::BTreeMap;

use crate::cumulative_resource_capacity::{
    CumulativeCapacityError, CumulativeCapacityScenario, evaluate_cumulative_resource_capacity,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityAvailabilityWindow {
    pub resource_id: String,
    pub resource_definition_sha256: String,
    pub site_snapshot_sha256: String,
    pub capacity_owner_id: String,
    pub evidence_receipt_sha256: String,
    /// Half-open interval [start_tick, end_tick) in the scenario site clock.
    pub start_tick: u64,
    pub end_tick: u64,
    pub lower_capacity_units: u64,
    pub nominal_capacity_units: u64,
    pub upper_capacity_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustCumulativeCapacityScenario {
    pub base: CumulativeCapacityScenario,
    /// Non-overlapping windows override the static definition's exact capacity.
    /// Outside these windows, the static capacity is treated as exact.
    pub availability_windows: Vec<CapacityAvailabilityWindow>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RobustCapacityAssessment {
    WithinLowerBound,
    IndeterminateWithinCapacityRange,
    DefinitelyOverUpperBound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustCapacitySegment {
    pub resource_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub observed_load_units: u128,
    pub lower_capacity_units: u64,
    pub nominal_capacity_units: u64,
    pub upper_capacity_units: u64,
    pub unit_label: String,
    /// Stable reservation IDs whose demand contributes to this non-robust
    /// resource/time segment. This is impact lineage, not actor authorization.
    pub contributing_reservation_ids: Vec<String>,
    pub assessment: RobustCapacityAssessment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustResourceCapacitySummary {
    pub resource_id: String,
    pub unit_label: String,
    pub peak_load_units: u128,
    pub minimum_lower_capacity_units: u64,
    pub minimum_nominal_capacity_units: u64,
    pub minimum_upper_capacity_units: u64,
    pub indeterminate_duration_ticks: u128,
    pub definitely_over_capacity_duration_ticks: u128,
    pub boundary_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustCumulativeCapacityReport {
    pub scenario_id: String,
    pub resource_catalog_sha256: String,
    pub site_snapshot_sha256: String,
    pub site_clock_domain_id: String,
    pub resource_count: u64,
    pub reservation_count: u64,
    pub claim_count: u64,
    pub capacity_window_count: u64,
    pub total_boundary_count: u64,
    pub is_robustly_feasible: bool,
    pub indeterminate_segment_count: u64,
    pub definitely_over_capacity_segment_count: u64,
    pub resource_summaries: Vec<RobustResourceCapacitySummary>,
    /// Every non-robust interval. Consumers must keep Indeterminate and
    /// DefinitelyOverUpperBound separate; neither is an admission success.
    pub non_robust_segments: Vec<RobustCapacitySegment>,
}


impl RobustCumulativeCapacityReport {
    /// Return a stable, deduplicated list of reservations touched by any
    /// indeterminate or definitely-over-capacity interval.
    pub fn impacted_reservation_ids(&self) -> Vec<String> {
        let mut impacted = std::collections::BTreeSet::new();
        for segment in &self.non_robust_segments {
            impacted.extend(segment.contributing_reservation_ids.iter().cloned());
        }
        impacted.into_iter().collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RobustCapacityError {
    InvalidBaseScenario(Vec<CumulativeCapacityError>),
    EmptyCapacityOwner(String),
    InvalidEvidenceReceipt(String),
    InvalidSiteSnapshotBinding(String),
    UnknownResource(String),
    ResourceDefinitionMismatch(String),
    InvalidCapacityWindow(String),
    InvalidCapacityBounds(String),
    CapacityWindowExceedsDeclaredMaximum(String),
    OverlappingCapacityWindows(String),
    ArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BoundaryDemand {
    starts: u128,
    ends: u128,
    reservations_starting: BTreeMap<String, u64>,
    reservations_ending: BTreeMap<String, u64>,
}

#[derive(Clone, Debug)]
struct ResourceSweep {
    definition_capacity: u64,
    unit_label: String,
    boundaries: BTreeMap<u64, BoundaryDemand>,
}

pub fn evaluate_robust_cumulative_resource_capacity(
    scenario: &RobustCumulativeCapacityScenario,
) -> Result<RobustCumulativeCapacityReport, Vec<RobustCapacityError>> {
    let base_report = evaluate_cumulative_resource_capacity(&scenario.base)
        .map_err(|errors| vec![RobustCapacityError::InvalidBaseScenario(errors)])?;

    let definitions: BTreeMap<_, _> = scenario
        .base
        .resources
        .iter()
        .map(|definition| (definition.resource_id.as_str(), definition))
        .collect();
    let mut windows_by_resource =
        BTreeMap::<String, Vec<&CapacityAvailabilityWindow>>::new();
    let mut errors = Vec::new();

    for window in &scenario.availability_windows {
        if window.resource_id.trim().is_empty()
            || window.capacity_owner_id.trim().is_empty()
            || window.start_tick >= window.end_tick
        {
            errors.push(RobustCapacityError::InvalidCapacityWindow(
                window.resource_id.clone(),
            ));
            continue;
        }
        if !is_sha256(&window.evidence_receipt_sha256) {
            errors.push(RobustCapacityError::InvalidEvidenceReceipt(
                window.resource_id.clone(),
            ));
        }
        if window.site_snapshot_sha256 != scenario.base.site_snapshot_sha256 {
            errors.push(RobustCapacityError::InvalidSiteSnapshotBinding(
                window.resource_id.clone(),
            ));
        }
        let Some(definition) = definitions.get(window.resource_id.as_str()) else {
            errors.push(RobustCapacityError::UnknownResource(
                window.resource_id.clone(),
            ));
            continue;
        };
        if definition.definition_sha256 != window.resource_definition_sha256 {
            errors.push(RobustCapacityError::ResourceDefinitionMismatch(
                window.resource_id.clone(),
            ));
        }
        if window.lower_capacity_units > window.nominal_capacity_units
            || window.nominal_capacity_units > window.upper_capacity_units
        {
            errors.push(RobustCapacityError::InvalidCapacityBounds(
                window.resource_id.clone(),
            ));
        }
        // This profile models time-varying derating/availability, not a
        // unilateral increase in installed resource capacity. An increase
        // requires a new owner-approved resource definition/catalog digest.
        if window.upper_capacity_units > definition.capacity_units {
            errors.push(RobustCapacityError::CapacityWindowExceedsDeclaredMaximum(
                window.resource_id.clone(),
            ));
        }
        windows_by_resource
            .entry(window.resource_id.clone())
            .or_default()
            .push(window);
    }

    for (resource_id, windows) in &mut windows_by_resource {
        windows.sort_by_key(|window| (window.start_tick, window.end_tick));
        for pair in windows.windows(2) {
            if pair[1].start_tick < pair[0].end_tick {
                errors.push(RobustCapacityError::OverlappingCapacityWindows(
                    resource_id.clone(),
                ));
                break;
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut sweeps = BTreeMap::<String, ResourceSweep>::new();
    for definition in &scenario.base.resources {
        sweeps.insert(
            definition.resource_id.clone(),
            ResourceSweep {
                definition_capacity: definition.capacity_units,
                unit_label: definition.unit_label.clone(),
                boundaries: BTreeMap::new(),
            },
        );
    }

    for reservation in &scenario.base.reservations {
        for claim in &reservation.claims {
            let sweep = sweeps
                .get_mut(&claim.resource_id)
                .ok_or_else(|| vec![RobustCapacityError::UnknownResource(
                    claim.resource_id.clone(),
                )])?;
            let start = sweep.boundaries.entry(claim.start_tick).or_default();
            start.starts = start
                .starts
                .checked_add(u128::from(claim.demand_units))
                .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
            let start_count = start
                .reservations_starting
                .entry(reservation.reservation_id.clone())
                .or_default();
            *start_count = start_count
                .checked_add(1)
                .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;

            let end = sweep.boundaries.entry(claim.end_tick).or_default();
            end.ends = end
                .ends
                .checked_add(u128::from(claim.demand_units))
                .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
            let end_count = end
                .reservations_ending
                .entry(reservation.reservation_id.clone())
                .or_default();
            *end_count = end_count
                .checked_add(1)
                .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
        }
    }

    for window in &scenario.availability_windows {
        let sweep = sweeps
            .get_mut(&window.resource_id)
            .ok_or_else(|| vec![RobustCapacityError::UnknownResource(
                window.resource_id.clone(),
            )])?;
        sweep.boundaries.entry(window.start_tick).or_default();
        sweep.boundaries.entry(window.end_tick).or_default();
    }

    let mut resource_summaries = Vec::with_capacity(sweeps.len());
    let mut non_robust_segments = Vec::new();
    let mut total_boundary_count = 0_u64;
    let mut indeterminate_segment_count = 0_u64;
    let mut definitely_over_capacity_segment_count = 0_u64;

    for (resource_id, sweep) in sweeps {
        let boundaries: Vec<(u64, BoundaryDemand)> = sweep
            .boundaries
            .iter()
            .map(|(tick, demand)| (*tick, *demand))
            .collect();
        let boundary_count = u64::try_from(boundaries.len())
            .map_err(|_| vec![RobustCapacityError::ArithmeticOverflow])?;
        total_boundary_count = total_boundary_count
            .checked_add(boundary_count)
            .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;

        let windows = windows_by_resource
            .get(&resource_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut window_index = 0_usize;
        let mut current_load = 0_u128;
        let mut active_reservations = BTreeMap::<String, u64>::new();
        let mut peak_load = 0_u128;
        let mut minimum_lower = sweep.definition_capacity;
        let mut minimum_nominal = sweep.definition_capacity;
        let mut minimum_upper = sweep.definition_capacity;
        let mut indeterminate_duration = 0_u128;
        let mut over_capacity_duration = 0_u128;

        for pair in boundaries.windows(2) {
            let (tick, demand) = pair[0];
            let next_tick = pair[1].0;
            current_load = current_load
                .checked_sub(demand.ends)
                .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
            for (reservation_id, ending_count) in &demand.reservations_ending {
                let should_remove = {
                    let active_count = active_reservations
                        .get_mut(reservation_id)
                        .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
                    *active_count = active_count
                        .checked_sub(*ending_count)
                        .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
                    *active_count == 0
                };
                if should_remove {
                    active_reservations.remove(reservation_id);
                }
            }
            current_load = current_load
                .checked_add(demand.starts)
                .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
            for (reservation_id, starting_count) in &demand.reservations_starting {
                let active_count = active_reservations
                    .entry(reservation_id.clone())
                    .or_default();
                *active_count = active_count
                    .checked_add(*starting_count)
                    .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
            }
            peak_load = peak_load.max(current_load);
            if next_tick <= tick {
                continue;
            }

            while window_index < windows.len() && windows[window_index].end_tick <= tick {
                window_index += 1;
            }
            let active_window = windows.get(window_index).copied().filter(|window| {
                window.start_tick <= tick && tick < window.end_tick
            });
            let (lower_capacity, nominal_capacity, upper_capacity) = active_window
                .map(|window| {
                    (
                        window.lower_capacity_units,
                        window.nominal_capacity_units,
                        window.upper_capacity_units,
                    )
                })
                .unwrap_or((
                    sweep.definition_capacity,
                    sweep.definition_capacity,
                    sweep.definition_capacity,
                ));
            minimum_lower = minimum_lower.min(lower_capacity);
            minimum_nominal = minimum_nominal.min(nominal_capacity);
            minimum_upper = minimum_upper.min(upper_capacity);

            let assessment = if current_load <= u128::from(lower_capacity) {
                RobustCapacityAssessment::WithinLowerBound
            } else if current_load <= u128::from(upper_capacity) {
                RobustCapacityAssessment::IndeterminateWithinCapacityRange
            } else {
                RobustCapacityAssessment::DefinitelyOverUpperBound
            };
            if assessment != RobustCapacityAssessment::WithinLowerBound {
                let duration = u128::from(next_tick - tick);
                match assessment {
                    RobustCapacityAssessment::WithinLowerBound => {}
                    RobustCapacityAssessment::IndeterminateWithinCapacityRange => {
                        indeterminate_duration = indeterminate_duration
                            .checked_add(duration)
                            .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
                        indeterminate_segment_count = indeterminate_segment_count
                            .checked_add(1)
                            .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
                    }
                    RobustCapacityAssessment::DefinitelyOverUpperBound => {
                        over_capacity_duration = over_capacity_duration
                            .checked_add(duration)
                            .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
                        definitely_over_capacity_segment_count =
                            definitely_over_capacity_segment_count
                                .checked_add(1)
                                .ok_or_else(|| vec![RobustCapacityError::ArithmeticOverflow])?;
                    }
                }
                non_robust_segments.push(RobustCapacitySegment {
                    resource_id: resource_id.clone(),
                    start_tick: tick,
                    end_tick: next_tick,
                    observed_load_units: current_load,
                    lower_capacity_units: lower_capacity,
                    nominal_capacity_units: nominal_capacity,
                    upper_capacity_units: upper_capacity,
                    unit_label: sweep.unit_label.clone(),
                    contributing_reservation_ids: active_reservations.keys().cloned().collect(),
                    assessment,
                });
            }
        }

        resource_summaries.push(RobustResourceCapacitySummary {
            resource_id,
            unit_label: sweep.unit_label,
            peak_load_units: peak_load,
            minimum_lower_capacity_units: minimum_lower,
            minimum_nominal_capacity_units: minimum_nominal,
            minimum_upper_capacity_units: minimum_upper,
            indeterminate_duration_ticks: indeterminate_duration,
            definitely_over_capacity_duration_ticks: over_capacity_duration,
            boundary_count,
        });
    }

    Ok(RobustCumulativeCapacityReport {
        scenario_id: scenario.base.scenario_id.clone(),
        resource_catalog_sha256: scenario.base.resource_catalog_sha256.clone(),
        site_snapshot_sha256: scenario.base.site_snapshot_sha256.clone(),
        site_clock_domain_id: scenario.base.site_clock_domain_id.clone(),
        resource_count: base_report.resource_count,
        reservation_count: base_report.reservation_count,
        claim_count: base_report.claim_count,
        capacity_window_count: scenario.availability_windows.len() as u64,
        total_boundary_count,
        is_robustly_feasible: non_robust_segments.is_empty(),
        indeterminate_segment_count,
        definitely_over_capacity_segment_count,
        resource_summaries,
        non_robust_segments,
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cumulative_resource_capacity::{
        CapacityDemandClaim, CapacityReservation, CapacityResourceDefinition,
    };

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn definition(id: &str, capacity: u64) -> CapacityResourceDefinition {
        CapacityResourceDefinition {
            resource_id: id.into(),
            definition_sha256: digest('a'),
            capacity_units: capacity,
            unit_label: "units".into(),
        }
    }

    fn reservation(id: &str, resource_id: &str, start: u64, end: u64, demand: u64) -> CapacityReservation {
        CapacityReservation {
            reservation_id: id.into(),
            reservation_receipt_sha256: digest('b'),
            site_clock_domain_id: "site-clock-1".into(),
            claims: vec![CapacityDemandClaim {
                resource_id: resource_id.into(),
                resource_definition_sha256: digest('a'),
                start_tick: start,
                end_tick: end,
                demand_units: demand,
            }],
        }
    }

    fn base() -> CumulativeCapacityScenario {
        CumulativeCapacityScenario {
            scenario_id: "robust-capacity-test".into(),
            resource_catalog_sha256: digest('c'),
            site_snapshot_sha256: digest('d'),
            site_clock_domain_id: "site-clock-1".into(),
            resources: vec![definition("hoist", 4)],
            reservations: vec![reservation("robot-load", "hoist", 0, 30, 3)],
        }
    }

    fn window(start: u64, end: u64, lower: u64, nominal: u64, upper: u64) -> CapacityAvailabilityWindow {
        CapacityAvailabilityWindow {
            resource_id: "hoist".into(),
            resource_definition_sha256: digest('a'),
            site_snapshot_sha256: digest('d'),
            capacity_owner_id: "hoist-owner".into(),
            evidence_receipt_sha256: digest('e'),
            start_tick: start,
            end_tick: end,
            lower_capacity_units: lower,
            nominal_capacity_units: nominal,
            upper_capacity_units: upper,
        }
    }

    #[test]
    fn time_varying_derating_is_applied_only_inside_its_window() {
        let report = evaluate_robust_cumulative_resource_capacity(
            &RobustCumulativeCapacityScenario {
                base: base(),
                availability_windows: vec![window(10, 20, 2, 2, 2)],
            },
        )
        .unwrap();
        assert!(!report.is_robustly_feasible);
        assert_eq!(report.definitely_over_capacity_segment_count, 1);
        assert_eq!(report.non_robust_segments[0].start_tick, 10);
        assert_eq!(report.non_robust_segments[0].end_tick, 20);
        assert_eq!(
            report.non_robust_segments[0].assessment,
            RobustCapacityAssessment::DefinitelyOverUpperBound
        );
        assert!(report.non_robust_segments[0].observed_load_units == 3);
    }

    #[test]
    fn uncertain_capacity_is_indeterminate_not_a_false_safe_pass() {
        let report = evaluate_robust_cumulative_resource_capacity(
            &RobustCumulativeCapacityScenario {
                base: base(),
                availability_windows: vec![window(10, 20, 2, 3, 4)],
            },
        )
        .unwrap();
        assert!(!report.is_robustly_feasible);
        assert_eq!(report.indeterminate_segment_count, 1);
        assert_eq!(report.definitely_over_capacity_segment_count, 0);
        assert_eq!(
            report.non_robust_segments[0].assessment,
            RobustCapacityAssessment::IndeterminateWithinCapacityRange
        );
    }

    #[test]
    fn demand_at_lower_bound_is_robustly_feasible() {
        let mut scenario = base();
        scenario.base.reservations[0].claims[0].demand_units = 2;
        scenario.availability_windows = vec![window(10, 20, 2, 3, 4)];
        let report = evaluate_robust_cumulative_resource_capacity(&scenario).unwrap();
        assert!(report.is_robustly_feasible);
        assert!(report.non_robust_segments.is_empty());
    }

    #[test]
    fn unavailable_resource_has_zero_capacity_and_blocks_all_positive_load() {
        let report = evaluate_robust_cumulative_resource_capacity(
            &RobustCumulativeCapacityScenario {
                base: base(),
                availability_windows: vec![window(10, 20, 0, 0, 0)],
            },
        )
        .unwrap();
        assert_eq!(
            report.non_robust_segments[0].assessment,
            RobustCapacityAssessment::DefinitelyOverUpperBound
        );
    }

    #[test]
    fn overlapping_windows_and_invalid_evidence_are_rejected() {
        let invalid = RobustCumulativeCapacityScenario {
            base: base(),
            availability_windows: vec![
                window(10, 20, 2, 3, 4),
                window(15, 25, 2, 3, 4),
            ],
        };
        assert!(
            evaluate_robust_cumulative_resource_capacity(&invalid)
                .unwrap_err()
                .contains(&RobustCapacityError::OverlappingCapacityWindows("hoist".into()))
        );

        let mut invalid_receipt = window(10, 20, 2, 3, 4);
        invalid_receipt.evidence_receipt_sha256 = "invalid".into();
        assert!(
            evaluate_robust_cumulative_resource_capacity(
                &RobustCumulativeCapacityScenario {
                    base: base(),
                    availability_windows: vec![invalid_receipt],
                }
            )
            .unwrap_err()
            .contains(&RobustCapacityError::InvalidEvidenceReceipt("hoist".into()))
        );
    }

    #[test]
    fn owner_snapshot_and_definition_bindings_are_enforced() {
        let mut stale_snapshot = window(10, 20, 2, 3, 4);
        stale_snapshot.site_snapshot_sha256 = digest('f');
        assert!(
            evaluate_robust_cumulative_resource_capacity(
                &RobustCumulativeCapacityScenario {
                    base: base(),
                    availability_windows: vec![stale_snapshot],
                }
            )
            .unwrap_err()
            .contains(&RobustCapacityError::InvalidSiteSnapshotBinding("hoist".into()))
        );

        let mut changed_definition = window(10, 20, 2, 3, 4);
        changed_definition.resource_definition_sha256 = digest('f');
        assert!(
            evaluate_robust_cumulative_resource_capacity(
                &RobustCumulativeCapacityScenario {
                    base: base(),
                    availability_windows: vec![changed_definition],
                }
            )
            .unwrap_err()
            .contains(&RobustCapacityError::ResourceDefinitionMismatch("hoist".into()))
        );
    }

    #[test]
    fn reservation_and_profile_permutations_preserve_robust_report() {
        let mut forward = base();
        forward.base.resources.push(definition("power", 10));
        forward.base.reservations.push(reservation("power-draw", "power", 0, 30, 4));
        forward.availability_windows = vec![
            window(10, 20, 2, 3, 4),
            CapacityAvailabilityWindow {
                resource_id: "power".into(),
                resource_definition_sha256: digest('a'),
                site_snapshot_sha256: digest('d'),
                capacity_owner_id: "power-owner".into(),
                evidence_receipt_sha256: digest('f'),
                start_tick: 5,
                end_tick: 25,
                lower_capacity_units: 5,
                nominal_capacity_units: 8,
                upper_capacity_units: 10,
            },
        ];
        let mut reverse = forward.clone();
        reverse.base.resources.reverse();
        reverse.base.reservations.reverse();
        reverse.availability_windows.reverse();
        assert_eq!(
            evaluate_robust_cumulative_resource_capacity(&forward).unwrap(),
            evaluate_robust_cumulative_resource_capacity(&reverse).unwrap()
        );
    }

    #[test]
    fn derating_profile_cannot_increase_declared_installed_capacity() {
        let invalid = RobustCumulativeCapacityScenario {
            base: base(),
            availability_windows: vec![window(10, 20, 4, 5, 5)],
        };
        assert!(
            evaluate_robust_cumulative_resource_capacity(&invalid)
                .unwrap_err()
                .contains(&RobustCapacityError::CapacityWindowExceedsDeclaredMaximum(
                    "hoist".into()
                ))
        );
    }


    #[test]
    fn generated_derating_sweeps_match_independent_tick_oracle() {
        for initial_seed in 0..24_u64 {
            let mut seed = 0xD3A7_1A6_u64.wrapping_add(initial_seed);
            let mut next = || {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                seed >> 32
            };

            let resources = vec![
                definition("hoist", 10),
                CapacityResourceDefinition {
                    resource_id: "temporary-power".into(),
                    definition_sha256: digest('f'),
                    capacity_units: 10,
                    unit_label: "kW".into(),
                },
            ];
            let mut reservations = Vec::new();
            for index in 0..24_u64 {
                let resource_id = if index % 2 == 0 {
                    "hoist"
                } else {
                    "temporary-power"
                };
                let resource_digest = if index % 2 == 0 { digest('a') } else { digest('f') };
                let start_tick = next() % 40;
                let duration = 1 + next() % 15;
                let demand_units = 1 + next() % 5;
                reservations.push(CapacityReservation {
                    reservation_id: format!("seed-{initial_seed}-reservation-{index}"),
                    reservation_receipt_sha256: digest('b'),
                    site_clock_domain_id: "site-clock-1".into(),
                    claims: vec![CapacityDemandClaim {
                        resource_id: resource_id.into(),
                        resource_definition_sha256: resource_digest,
                        start_tick,
                        end_tick: start_tick + duration,
                        demand_units,
                    }],
                });
            }
            let snapshot = digest('d');
            let windows = vec![
                window(10, 30, 3, 5, 7),
                CapacityAvailabilityWindow {
                    resource_id: "temporary-power".into(),
                    resource_definition_sha256: digest('f'),
                    site_snapshot_sha256: snapshot.clone(),
                    capacity_owner_id: "power-owner".into(),
                    evidence_receipt_sha256: digest('e'),
                    start_tick: 15,
                    end_tick: 35,
                    lower_capacity_units: 4,
                    nominal_capacity_units: 6,
                    upper_capacity_units: 8,
                },
            ];
            let scenario = RobustCumulativeCapacityScenario {
                base: CumulativeCapacityScenario {
                    scenario_id: format!("generated-robust-{initial_seed}"),
                    resource_catalog_sha256: digest('c'),
                    site_snapshot_sha256: snapshot,
                    site_clock_domain_id: "site-clock-1".into(),
                    resources,
                    reservations,
                },
                availability_windows: windows,
            };
            let report = evaluate_robust_cumulative_resource_capacity(&scenario).unwrap();

            let mut expected_robust = true;
            for (resource_id, definition) in [
                ("hoist", 10_u64),
                ("temporary-power", 10_u64),
            ] {
                for tick in 0..60_u64 {
                    let load: u128 = scenario
                        .base
                        .reservations
                        .iter()
                        .flat_map(|reservation| reservation.claims.iter())
                        .filter(|claim| {
                            claim.resource_id == resource_id
                                && claim.start_tick <= tick
                                && tick < claim.end_tick
                        })
                        .map(|claim| u128::from(claim.demand_units))
                        .sum();
                    let profile = scenario.availability_windows.iter().find(|window| {
                        window.resource_id == resource_id
                            && window.start_tick <= tick
                            && tick < window.end_tick
                    });
                    let (lower, upper) = profile
                        .map(|window| {
                            (window.lower_capacity_units, window.upper_capacity_units)
                        })
                        .unwrap_or((definition, definition));
                    let expected_assessment = if load <= u128::from(lower) {
                        None
                    } else if load <= u128::from(upper) {
                        expected_robust = false;
                        Some(RobustCapacityAssessment::IndeterminateWithinCapacityRange)
                    } else {
                        expected_robust = false;
                        Some(RobustCapacityAssessment::DefinitelyOverUpperBound)
                    };
                    let reported = report.non_robust_segments.iter().find(|segment| {
                        segment.resource_id == resource_id
                            && segment.start_tick <= tick
                            && tick < segment.end_tick
                    });
                    match (expected_assessment, reported) {
                        (None, None) => {}
                        (Some(expected), Some(actual)) => {
                            assert_eq!(actual.assessment, expected);
                            assert_eq!(actual.observed_load_units, load);
                            assert_eq!(actual.lower_capacity_units, lower);
                            assert_eq!(actual.upper_capacity_units, upper);
                        }
                        (expected, actual) => panic!(
                            "generated robust sweep disagreed at seed {initial_seed}, resource {resource_id}, tick {tick}: expected {expected:?}, got {actual:?}"
                        ),
                    }
                }
            }
            assert_eq!(report.is_robustly_feasible, expected_robust);
        }
    }


    #[test]
    fn non_robust_segments_report_exact_active_reservation_contributors() {
        let base = CumulativeCapacityScenario {
            scenario_id: "contributor-test".into(),
            resource_catalog_sha256: digest('c'),
            site_snapshot_sha256: digest('d'),
            site_clock_domain_id: "site-clock-1".into(),
            resources: vec![definition("hoist", 4)],
            reservations: vec![
                reservation("robot-a", "hoist", 10, 20, 2),
                reservation("robot-b", "hoist", 15, 25, 2),
                reservation("unrelated", "hoist", 30, 40, 1),
            ],
        };
        let report = evaluate_robust_cumulative_resource_capacity(
            &RobustCumulativeCapacityScenario {
                base,
                availability_windows: vec![window(10, 25, 2, 3, 4)],
            },
        )
        .unwrap();
        assert_eq!(report.non_robust_segments.len(), 1);
        assert_eq!(report.non_robust_segments[0].start_tick, 15);
        assert_eq!(report.non_robust_segments[0].end_tick, 20);
        assert_eq!(
            report.non_robust_segments[0].contributing_reservation_ids,
            vec!["robot-a".to_owned(), "robot-b".to_owned()]
        );
        assert_eq!(
            report.impacted_reservation_ids(),
            vec!["robot-a".to_owned(), "robot-b".to_owned()]
        );
    }

    #[test]
    fn overlapping_claims_from_one_reservation_keep_contributor_until_last_claim_ends() {
        let base = CumulativeCapacityScenario {
            scenario_id: "multi-claim-same-reservation".into(),
            resource_catalog_sha256: digest('c'),
            site_snapshot_sha256: digest('d'),
            site_clock_domain_id: "site-clock-1".into(),
            resources: vec![definition("hoist", 4)],
            reservations: vec![CapacityReservation {
                reservation_id: "one-reservation".into(),
                reservation_receipt_sha256: digest('b'),
                site_clock_domain_id: "site-clock-1".into(),
                claims: vec![
                    CapacityDemandClaim {
                        resource_id: "hoist".into(),
                        resource_definition_sha256: digest('a'),
                        start_tick: 10,
                        end_tick: 20,
                        demand_units: 2,
                    },
                    CapacityDemandClaim {
                        resource_id: "hoist".into(),
                        resource_definition_sha256: digest('a'),
                        start_tick: 15,
                        end_tick: 25,
                        demand_units: 2,
                    },
                ],
            }],
        };
        let report = evaluate_robust_cumulative_resource_capacity(
            &RobustCumulativeCapacityScenario {
                base,
                availability_windows: vec![window(10, 25, 2, 3, 4)],
            },
        )
        .unwrap();
        assert_eq!(report.non_robust_segments.len(), 2);
        for segment in &report.non_robust_segments {
            assert_eq!(
                segment.contributing_reservation_ids,
                vec!["one-reservation".to_owned()]
            );
        }
    }

}
