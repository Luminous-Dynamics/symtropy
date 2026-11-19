//! Independent cumulative-capacity oracle for construction resource scheduling.
//!
//! The exclusive space-time reservation ledger is intentionally capacity-one.
//! This module explores resources with finite shared capacity (staging areas,
//! hoists, power feeds, workcells and similar facilities). It is an assessment
//! oracle only: it does not commit reservations, allocate resources or
//! authorize physical execution.
//!
//! All intervals are half-open [start_tick, end_tick). At a shared boundary,
//! ending demand is removed before newly starting demand is counted.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityResourceDefinition {
    pub resource_id: String,
    /// Owner identity is part of the canonical resource definition digest.
    /// Callers must verify that the digest covers this identity and its scope.
    pub capacity_owner_id: String,
    pub definition_sha256: String,
    pub capacity_units: u64,
    /// Unit is domain-defined (e.g. kW, kg, pallet positions, parallel bays).
    pub unit_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityDemandClaim {
    pub resource_id: String,
    pub resource_definition_sha256: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub demand_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityReservation {
    pub reservation_id: String,
    pub reservation_receipt_sha256: String,
    pub site_clock_domain_id: String,
    pub claims: Vec<CapacityDemandClaim>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CumulativeCapacityScenario {
    pub scenario_id: String,
    pub resource_catalog_sha256: String,
    pub site_snapshot_sha256: String,
    pub site_clock_domain_id: String,
    pub resources: Vec<CapacityResourceDefinition>,
    pub reservations: Vec<CapacityReservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityViolationSegment {
    pub resource_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub observed_load_units: u128,
    pub capacity_units: u64,
    pub unit_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceCapacitySummary {
    pub resource_id: String,
    pub capacity_units: u64,
    pub unit_label: String,
    pub peak_load_units: u128,
    pub overloaded_duration_ticks: u128,
    pub boundary_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CumulativeCapacityReport {
    pub scenario_id: String,
    pub resource_catalog_sha256: String,
    pub site_snapshot_sha256: String,
    pub site_clock_domain_id: String,
    pub resource_count: u64,
    pub reservation_count: u64,
    pub claim_count: u64,
    pub total_boundary_count: u64,
    pub is_feasible: bool,
    pub resource_summaries: Vec<ResourceCapacitySummary>,
    pub violations: Vec<CapacityViolationSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CumulativeCapacityError {
    EmptyScenarioIdentity,
    InvalidScenarioDigest,
    EmptyClockDomain,
    EmptyResourceId,
    InvalidResourceDefinitionDigest(String),
    DuplicateResourceId(String),
    ZeroResourceCapacity(String),
    EmptyResourceUnit(String),
    EmptyReservationId,
    InvalidReservationReceipt(String),
    DuplicateReservationId(String),
    ReservationClockDomainMismatch(String),
    ReservationHasNoClaims(String),
    UnknownResource(String),
    ResourceDefinitionMismatch(String),
    InvalidClaimInterval(String),
    ZeroDemand(String),
    ArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BoundaryDemand {
    starts: u128,
    ends: u128,
}

#[derive(Clone, Debug)]
struct ResourceAccumulator {
    definition: CapacityResourceDefinition,
    boundaries: BTreeMap<u64, BoundaryDemand>,
}

/// Evaluate a frozen set of reservation demands against per-resource
/// capacities using a deterministic sweep line.
///
/// For each resource, events are grouped by tick. At each boundary, ending
/// demand is subtracted first, then starting demand is added. The load after
/// that boundary applies to [tick, next_tick). All additions and subtractions
/// are checked using u128 so a sum of u64 demands cannot silently wrap.
pub fn evaluate_cumulative_resource_capacity(
    scenario: &CumulativeCapacityScenario,
) -> Result<CumulativeCapacityReport, Vec<CumulativeCapacityError>> {
    let mut errors = Vec::new();

    if scenario.scenario_id.trim().is_empty() || scenario.site_clock_domain_id.trim().is_empty() {
        errors.push(CumulativeCapacityError::EmptyScenarioIdentity);
    }
    if !is_sha256(&scenario.resource_catalog_sha256) || !is_sha256(&scenario.site_snapshot_sha256) {
        errors.push(CumulativeCapacityError::InvalidScenarioDigest);
    }

    let mut resources = BTreeMap::<String, ResourceAccumulator>::new();
    for definition in &scenario.resources {
        if definition.resource_id.trim().is_empty() {
            errors.push(CumulativeCapacityError::EmptyResourceId);
            continue;
        }
        if !is_sha256(&definition.definition_sha256) {
            errors.push(CumulativeCapacityError::InvalidResourceDefinitionDigest(
                definition.resource_id.clone(),
            ));
        }
        if definition.capacity_units == 0 {
            errors.push(CumulativeCapacityError::ZeroResourceCapacity(
                definition.resource_id.clone(),
            ));
        }
        if definition.unit_label.trim().is_empty() {
            errors.push(CumulativeCapacityError::EmptyResourceUnit(
                definition.resource_id.clone(),
            ));
        }
        if resources.contains_key(&definition.resource_id) {
            errors.push(CumulativeCapacityError::DuplicateResourceId(
                definition.resource_id.clone(),
            ));
            continue;
        }
        resources.insert(
            definition.resource_id.clone(),
            ResourceAccumulator {
                definition: definition.clone(),
                boundaries: BTreeMap::new(),
            },
        );
    }

    let mut reservation_ids = BTreeSet::new();
    let mut claim_count = 0_u64;
    for reservation in &scenario.reservations {
        if reservation.reservation_id.trim().is_empty() {
            errors.push(CumulativeCapacityError::EmptyReservationId);
            continue;
        }
        if !reservation_ids.insert(reservation.reservation_id.as_str()) {
            errors.push(CumulativeCapacityError::DuplicateReservationId(
                reservation.reservation_id.clone(),
            ));
            continue;
        }
        if !is_sha256(&reservation.reservation_receipt_sha256) {
            errors.push(CumulativeCapacityError::InvalidReservationReceipt(
                reservation.reservation_id.clone(),
            ));
        }
        if reservation.site_clock_domain_id != scenario.site_clock_domain_id {
            errors.push(CumulativeCapacityError::ReservationClockDomainMismatch(
                reservation.reservation_id.clone(),
            ));
        }
        if reservation.claims.is_empty() {
            errors.push(CumulativeCapacityError::ReservationHasNoClaims(
                reservation.reservation_id.clone(),
            ));
            continue;
        }

        for claim in &reservation.claims {
            claim_count = match claim_count.checked_add(1) {
                Some(value) => value,
                None => {
                    errors.push(CumulativeCapacityError::ArithmeticOverflow);
                    u64::MAX
                }
            };
            if claim.start_tick >= claim.end_tick {
                errors.push(CumulativeCapacityError::InvalidClaimInterval(
                    reservation.reservation_id.clone(),
                ));
                continue;
            }
            if claim.demand_units == 0 {
                errors.push(CumulativeCapacityError::ZeroDemand(
                    reservation.reservation_id.clone(),
                ));
                continue;
            }
            let Some(resource) = resources.get_mut(&claim.resource_id) else {
                errors.push(CumulativeCapacityError::UnknownResource(
                    claim.resource_id.clone(),
                ));
                continue;
            };
            if resource.definition.definition_sha256 != claim.resource_definition_sha256 {
                errors.push(CumulativeCapacityError::ResourceDefinitionMismatch(
                    claim.resource_id.clone(),
                ));
                continue;
            }

            let start = resource.boundaries.entry(claim.start_tick).or_default();
            match start.starts.checked_add(u128::from(claim.demand_units)) {
                Some(value) => start.starts = value,
                None => errors.push(CumulativeCapacityError::ArithmeticOverflow),
            }
            let end = resource.boundaries.entry(claim.end_tick).or_default();
            match end.ends.checked_add(u128::from(claim.demand_units)) {
                Some(value) => end.ends = value,
                None => errors.push(CumulativeCapacityError::ArithmeticOverflow),
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut resource_summaries = Vec::with_capacity(resources.len());
    let mut violations = Vec::new();
    let mut total_boundary_count = 0_u64;

    for (resource_id, resource) in resources {
        let capacity = u128::from(resource.definition.capacity_units);
        let boundaries: Vec<(u64, BoundaryDemand)> = resource
            .boundaries
            .iter()
            .map(|(tick, demand)| (*tick, *demand))
            .collect();
        let boundary_count = boundaries.len() as u64;
        total_boundary_count = total_boundary_count
            .checked_add(boundary_count)
            .ok_or_else(|| vec![CumulativeCapacityError::ArithmeticOverflow])?;

        let mut current_load = 0_u128;
        let mut peak_load = 0_u128;
        let mut overloaded_duration = 0_u128;

        for pair in boundaries.windows(2) {
            let (tick, demand) = pair[0];
            let next_tick = pair[1].0;

            // End events are applied before starts at the same boundary,
            // preserving half-open interval semantics.
            current_load = current_load
                .checked_sub(demand.ends)
                .ok_or_else(|| vec![CumulativeCapacityError::ArithmeticOverflow])?;
            current_load = current_load
                .checked_add(demand.starts)
                .ok_or_else(|| vec![CumulativeCapacityError::ArithmeticOverflow])?;
            peak_load = peak_load.max(current_load);

            if next_tick > tick && current_load > capacity {
                let duration = u128::from(next_tick - tick);
                overloaded_duration = overloaded_duration
                    .checked_add(duration)
                    .ok_or_else(|| vec![CumulativeCapacityError::ArithmeticOverflow])?;
                violations.push(CapacityViolationSegment {
                    resource_id: resource_id.clone(),
                    start_tick: tick,
                    end_tick: next_tick,
                    observed_load_units: current_load,
                    capacity_units: resource.definition.capacity_units,
                    unit_label: resource.definition.unit_label.clone(),
                });
            }
        }

        resource_summaries.push(ResourceCapacitySummary {
            resource_id,
            capacity_units: resource.definition.capacity_units,
            unit_label: resource.definition.unit_label,
            peak_load_units: peak_load,
            overloaded_duration_ticks: overloaded_duration,
            boundary_count,
        });
    }

    Ok(CumulativeCapacityReport {
        scenario_id: scenario.scenario_id.clone(),
        resource_catalog_sha256: scenario.resource_catalog_sha256.clone(),
        site_snapshot_sha256: scenario.site_snapshot_sha256.clone(),
        site_clock_domain_id: scenario.site_clock_domain_id.clone(),
        resource_count: resource_summaries.len() as u64,
        reservation_count: reservation_ids.len() as u64,
        claim_count,
        total_boundary_count,
        is_feasible: violations.is_empty(),
        resource_summaries,
        violations,
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

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn definition(id: &str, capacity: u64) -> CapacityResourceDefinition {
        CapacityResourceDefinition {
            resource_id: id.into(),
            capacity_owner_id: format!("{id}-owner"),
            definition_sha256: digest('a'),
            capacity_units: capacity,
            unit_label: "units".into(),
        }
    }

    fn reservation(
        id: &str,
        resource_id: &str,
        start_tick: u64,
        end_tick: u64,
        demand_units: u64,
    ) -> CapacityReservation {
        CapacityReservation {
            reservation_id: id.into(),
            reservation_receipt_sha256: digest('b'),
            site_clock_domain_id: "site-clock-1".into(),
            claims: vec![CapacityDemandClaim {
                resource_id: resource_id.into(),
                resource_definition_sha256: digest('a'),
                start_tick,
                end_tick,
                demand_units,
            }],
        }
    }

    fn scenario(
        resources: Vec<CapacityResourceDefinition>,
        reservations: Vec<CapacityReservation>,
    ) -> CumulativeCapacityScenario {
        CumulativeCapacityScenario {
            scenario_id: "capacity-test".into(),
            resource_catalog_sha256: digest('c'),
            site_snapshot_sha256: digest('d'),
            site_clock_domain_id: "site-clock-1".into(),
            resources,
            reservations,
        }
    }

    #[test]
    fn capacity_one_matches_exclusive_mutex_for_overlapping_claims() {
        let report = evaluate_cumulative_resource_capacity(&scenario(
            vec![definition("crane-zone", 1)],
            vec![
                reservation("r1", "crane-zone", 0, 10, 1),
                reservation("r2", "crane-zone", 5, 15, 1),
            ],
        ))
        .unwrap();
        assert!(!report.is_feasible);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(
            report.violations[0],
            CapacityViolationSegment {
                resource_id: "crane-zone".into(),
                start_tick: 5,
                end_tick: 10,
                observed_load_units: 2,
                capacity_units: 1,
                unit_label: "units".into(),
            }
        );
    }

    #[test]
    fn capacity_two_admits_two_concurrent_units_and_rejects_the_third() {
        let report = evaluate_cumulative_resource_capacity(&scenario(
            vec![definition("shared-hoist", 2)],
            vec![
                reservation("r1", "shared-hoist", 0, 10, 1),
                reservation("r2", "shared-hoist", 0, 10, 1),
                reservation("r3", "shared-hoist", 5, 8, 1),
            ],
        ))
        .unwrap();
        assert!(!report.is_feasible);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].start_tick, 5);
        assert_eq!(report.violations[0].end_tick, 8);
        assert_eq!(report.violations[0].observed_load_units, 3);
        assert_eq!(report.violations[0].capacity_units, 2);
    }

    #[test]
    fn weighted_demands_report_exact_overcapacity_segments() {
        let report = evaluate_cumulative_resource_capacity(&scenario(
            vec![definition("power-feed", 3)],
            vec![
                reservation("robot-a", "power-feed", 0, 10, 2),
                reservation("robot-b", "power-feed", 5, 15, 2),
                reservation("robot-c", "power-feed", 10, 20, 1),
            ],
        ))
        .unwrap();
        assert!(!report.is_feasible);
        assert_eq!(
            report
                .violations
                .iter()
                .map(|segment| (
                    segment.start_tick,
                    segment.end_tick,
                    segment.observed_load_units
                ))
                .collect::<Vec<_>>(),
            vec![(5, 10, 4)]
        );
        assert_eq!(report.resource_summaries[0].peak_load_units, 4);
    }

    #[test]
    fn adjacent_half_open_claims_are_feasible_at_capacity_one() {
        let report = evaluate_cumulative_resource_capacity(&scenario(
            vec![definition("workface", 1)],
            vec![
                reservation("before", "workface", 0, 10, 1),
                reservation("after", "workface", 10, 20, 1),
            ],
        ))
        .unwrap();
        assert!(report.is_feasible);
        assert!(report.violations.is_empty());
        assert_eq!(report.resource_summaries[0].peak_load_units, 1);
    }

    #[test]
    fn different_resources_have_independent_capacity_profiles() {
        let report = evaluate_cumulative_resource_capacity(&scenario(
            vec![definition("crane", 1), definition("power", 2)],
            vec![
                reservation("crane-a", "crane", 0, 10, 1),
                reservation("power-a", "power", 0, 10, 2),
            ],
        ))
        .unwrap();
        assert!(report.is_feasible);
        assert_eq!(report.resource_count, 2);
    }

    #[test]
    fn invalid_catalogs_reservations_and_claims_fail_closed() {
        let mut invalid = scenario(
            vec![definition("r", 1), definition("r", 1)],
            vec![reservation("one", "r", 0, 10, 1)],
        );
        assert!(
            evaluate_cumulative_resource_capacity(&invalid)
                .unwrap_err()
                .contains(&CumulativeCapacityError::DuplicateResourceId("r".into()))
        );

        invalid.resources = vec![definition("r", 1)];
        invalid.reservations[0].claims[0].end_tick = 0;
        assert!(
            evaluate_cumulative_resource_capacity(&invalid)
                .unwrap_err()
                .contains(&CumulativeCapacityError::InvalidClaimInterval("one".into()))
        );

        let mut missing = scenario(vec![definition("r", 1)], vec![]);
        missing
            .reservations
            .push(reservation("missing", "unknown", 0, 10, 1));
        assert!(
            evaluate_cumulative_resource_capacity(&missing)
                .unwrap_err()
                .contains(&CumulativeCapacityError::UnknownResource("unknown".into()))
        );
    }

    #[test]
    fn digest_and_clock_domain_mismatches_are_not_ignored() {
        let mut invalid = scenario(
            vec![definition("r", 1)],
            vec![reservation("one", "r", 0, 10, 1)],
        );
        invalid.reservations[0].claims[0].resource_definition_sha256 = digest('e');
        assert!(
            evaluate_cumulative_resource_capacity(&invalid)
                .unwrap_err()
                .contains(&CumulativeCapacityError::ResourceDefinitionMismatch(
                    "r".into()
                ))
        );

        let mut other_clock = scenario(
            vec![definition("r", 1)],
            vec![reservation("one", "r", 0, 10, 1)],
        );
        other_clock.reservations[0].site_clock_domain_id = "robot-clock".into();
        assert!(
            evaluate_cumulative_resource_capacity(&other_clock)
                .unwrap_err()
                .contains(&CumulativeCapacityError::ReservationClockDomainMismatch(
                    "one".into()
                ))
        );
    }

    #[test]
    fn high_weight_sums_use_wide_arithmetic_without_wrapping() {
        let report = evaluate_cumulative_resource_capacity(&scenario(
            vec![definition("floor-load", u64::MAX)],
            vec![
                reservation("load-a", "floor-load", 0, 10, u64::MAX),
                reservation("load-b", "floor-load", 0, 10, u64::MAX),
            ],
        ))
        .unwrap();
        assert!(!report.is_feasible);
        assert_eq!(
            report.violations[0].observed_load_units,
            u128::from(u64::MAX) * 2
        );
    }

    #[test]
    fn reservation_and_claim_permutations_produce_identical_report() {
        let resources = vec![definition("crane", 2), definition("power", 3)];
        let reservations = vec![
            reservation("r1", "crane", 0, 10, 1),
            reservation("r2", "crane", 5, 12, 2),
            reservation("r3", "power", 0, 20, 2),
        ];
        let baseline = evaluate_cumulative_resource_capacity(&scenario(
            resources.clone(),
            reservations.clone(),
        ))
        .unwrap();

        let mut reversed_reservations = reservations;
        reversed_reservations.reverse();
        let mut reversed_resources = resources;
        reversed_resources.reverse();
        let reversed = evaluate_cumulative_resource_capacity(&scenario(
            reversed_resources,
            reversed_reservations,
        ))
        .unwrap();
        assert_eq!(baseline, reversed);
    }

    #[test]
    fn randomized_sweep_peak_matches_discrete_tick_reference() {
        for seed_start in 0..24_u64 {
            let mut seed = 0xCA9A_C17_u64.wrapping_add(seed_start);
            let mut next = || {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                seed >> 32
            };
            let mut claims = Vec::new();
            for index in 0..24_u64 {
                let start = next() % 90;
                let duration = 1 + next() % (100 - start);
                let demand = 1 + next() % 4;
                claims.push(reservation(
                    &format!("reservation-{index:02}"),
                    "test-resource",
                    start,
                    start + duration,
                    demand,
                ));
            }

            let capacity = 1 + (next() % 8);
            let report = evaluate_cumulative_resource_capacity(&scenario(
                vec![definition("test-resource", capacity)],
                claims.clone(),
            ))
            .unwrap();

            let mut oracle_peak = 0_u128;
            for tick in 0..100_u64 {
                let load: u128 = claims
                    .iter()
                    .flat_map(|reservation| reservation.claims.iter())
                    .filter(|claim| claim.start_tick <= tick && tick < claim.end_tick)
                    .map(|claim| u128::from(claim.demand_units))
                    .sum();
                oracle_peak = oracle_peak.max(load);
            }
            assert_eq!(
                report.resource_summaries[0].peak_load_units, oracle_peak,
                "sweep and discrete oracle diverged for seed {seed_start}"
            );
            assert_eq!(
                report.is_feasible,
                oracle_peak <= u128::from(capacity),
                "feasibility and discrete oracle diverged for seed {seed_start}"
            );
        }
    }
}
