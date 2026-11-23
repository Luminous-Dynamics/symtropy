// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeMap;

const WORLD_COUNT: usize = 3;
const MAX_ACQUISITION_BUDGET_SERVICE_UNITS: u32 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DependencyDisposition {
    Separated,
    Shared,
    PotentiallyCorrelated,
    Unknown,
    Revoked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EventAssociation {
    NotApplicable,
    Unique,
    Ambiguous,
    Unmatched,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SensingMode {
    NoObservation,
    AdvisoryOnly,
    IndependentWitness,
}

#[derive(Clone, Copy, Debug)]
struct EvidenceGate {
    dependency: DependencyDisposition,
    event_association: EventAssociation,
    capture_complete: bool,
    freshness_current: bool,
    read_only_authorized: bool,
    dependency_receipt: &'static str,
    association_receipt: &'static str,
    capture_receipt: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct SensorAction {
    id: &'static str,
    mode: SensingMode,
    gate: Option<EvidenceGate>,
    outcomes: [&'static str; WORLD_COUNT],
    energy_kwh: u32,
    service_units_per_kwh: u32,
    latency_ticks: u32,
    service_units_per_latency_tick: u32,
}

#[derive(Clone, Copy, Debug)]
struct World {
    id: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct RecourseAction {
    id: &'static str,
    feasible: [bool; WORLD_COUNT],
    delivered_service_units: [u32; WORLD_COUNT],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PartitionDecision {
    outcome: &'static str,
    action_id: &'static str,
    worst_case_regret_service_units: u32,
}

#[derive(Clone, Debug)]
struct CandidateResult {
    id: &'static str,
    status: &'static str,
    claim_ceiling: &'static str,
    energy_kwh: u32,
    service_units_per_kwh: u32,
    latency_ticks: u32,
    service_units_per_latency_tick: u32,
    acquisition_cost_service_units: Option<u32>,
    partition_count: Option<usize>,
    worst_case_regret_service_units: Option<u32>,
    partition_actions: Vec<PartitionDecision>,
    evidence_receipts: &'static str,
    recommended: bool,
}

const WORLDS: [World; WORLD_COUNT] = [
    World {
        id: "primary_ready",
    },
    World {
        id: "primary_delayed_reserve_ready",
    },
    World {
        id: "primary_delayed_reserve_unavailable",
    },
];

const ACTIONS: [RecourseAction; 3] = [
    RecourseAction {
        id: "maintain_primary",
        feasible: [true, true, true],
        delivered_service_units: [100, 20, 20],
    },
    RecourseAction {
        id: "use_reserve",
        feasible: [true, true, false],
        delivered_service_units: [80, 90, 0],
    },
    RecourseAction {
        id: "hold",
        feasible: [true, true, true],
        delivered_service_units: [0, 0, 0],
    },
];

const NO_OBSERVATION: SensorAction = SensorAction {
    id: "no_observation",
    mode: SensingMode::NoObservation,
    gate: None,
    outcomes: ["CURRENT_EVIDENCE"; WORLD_COUNT],
    energy_kwh: 0,
    service_units_per_kwh: 0,
    latency_ticks: 0,
    service_units_per_latency_tick: 0,
};

const SENSORS: [SensorAction; 6] = [
    // This is useful operational telemetry, but shares the TOS/gateway path.
    // It is deliberately advisory and cannot become an independent witness.
    SensorAction {
        id: "shared_tos_signal",
        mode: SensingMode::AdvisoryOnly,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Shared,
            event_association: EventAssociation::NotApplicable,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            dependency_receipt: "synthetic:shared-gateway-dependency",
            association_receipt: "synthetic:not-applicable",
            capture_receipt: "synthetic:complete-tos-capture",
        }),
        outcomes: ["PRIMARY_READY", "NO_REPORT", "NO_REPORT"],
        energy_kwh: 1,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 3,
    },
    // The only independent observation in the known-answer case. The gate is
    // a consumed upstream receipt, not a second implementation of its owners.
    SensorAction {
        id: "independent_field_inspection",
        mode: SensingMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:unique-physical-event-match",
            capture_receipt: "synthetic:complete-independent-capture",
        }),
        outcomes: [
            "PRIMARY_READY",
            "PRIMARY_DELAY_RESERVE_READY",
            "PRIMARY_DELAY_RESERVE_UNAVAILABLE",
        ],
        energy_kwh: 2,
        service_units_per_kwh: 1,
        latency_ticks: 1,
        service_units_per_latency_tick: 3,
    },
    // A tempting perfect/cheap channel is rejected because its dependency
    // inventory is unknown. It must never enter the decision partition.
    SensorAction {
        id: "unknown_dependency_feed",
        mode: SensingMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Unknown,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            dependency_receipt: "synthetic:unknown-dependency-inventory",
            association_receipt: "synthetic:unique-match-claimed",
            capture_receipt: "synthetic:complete-capture-claimed",
        }),
        outcomes: [
            "PRIMARY_READY",
            "PRIMARY_DELAY_RESERVE_READY",
            "PRIMARY_DELAY_RESERVE_UNAVAILABLE",
        ],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    // A separated channel is not enough when physical-event correspondence is
    // ambiguous. This control checks that the event-association gate remains
    // independent of the dependency-separation gate.
    SensorAction {
        id: "ambiguous_event_camera",
        mode: SensingMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Ambiguous,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:ambiguous-physical-event-match",
            capture_receipt: "synthetic:complete-independent-capture",
        }),
        outcomes: [
            "PRIMARY_READY",
            "PRIMARY_DELAY_RESERVE_READY",
            "PRIMARY_DELAY_RESERVE_UNAVAILABLE",
        ],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    SensorAction {
        id: "stale_field_sensor",
        mode: SensingMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: false,
            read_only_authorized: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:unique-physical-event-match",
            capture_receipt: "synthetic:stale-capture",
        }),
        outcomes: [
            "PRIMARY_READY",
            "PRIMARY_DELAY_RESERVE_READY",
            "PRIMARY_DELAY_RESERVE_UNAVAILABLE",
        ],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    // High-quality evidence is still not selected when the declared
    // energy-plus-delay cost exceeds the preregistered acquisition budget.
    SensorAction {
        id: "overbudget_independent_audit",
        mode: SensingMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:unique-physical-event-match",
            capture_receipt: "synthetic:complete-independent-capture",
        }),
        outcomes: [
            "PRIMARY_READY",
            "PRIMARY_DELAY_RESERVE_READY",
            "PRIMARY_DELAY_RESERVE_UNAVAILABLE",
        ],
        energy_kwh: 4,
        service_units_per_kwh: 1,
        latency_ticks: 2,
        service_units_per_latency_tick: 4,
    },
];

fn evidence_blocker(sensor: &SensorAction) -> Option<&'static str> {
    if sensor.mode == SensingMode::NoObservation {
        return None;
    }

    let gate = sensor.gate?;

    if !gate.read_only_authorized {
        return Some("BLOCKED_NOT_AUTHORIZED_READ_ONLY");
    }
    if !gate.capture_complete {
        return Some("BLOCKED_INCOMPLETE_CAPTURE");
    }
    if !gate.freshness_current {
        return Some("BLOCKED_STALE_EVIDENCE");
    }

    match (sensor.mode, gate.dependency) {
        (_, DependencyDisposition::Unknown) => {
            return Some("BLOCKED_UNKNOWN_DEPENDENCY");
        }
        (_, DependencyDisposition::Revoked) => {
            return Some("BLOCKED_REVOKED_DEPENDENCY");
        }
        (
            SensingMode::IndependentWitness,
            DependencyDisposition::Shared,
        ) => {
            return Some("BLOCKED_SHARED_DEPENDENCY");
        }
        (
            SensingMode::IndependentWitness,
            DependencyDisposition::PotentiallyCorrelated,
        ) => {
            return Some("BLOCKED_POTENTIALLY_CORRELATED");
        }
        (SensingMode::AdvisoryOnly, DependencyDisposition::PotentiallyCorrelated) => {
            return Some("BLOCKED_POTENTIALLY_CORRELATED");
        }
        _ => {}
    }

    if sensor.mode == SensingMode::IndependentWitness
        && gate.dependency != DependencyDisposition::Separated
    {
        return Some("BLOCKED_DEPENDENCY_NOT_SEPARATED");
    }

    if sensor.mode == SensingMode::IndependentWitness {
        match gate.event_association {
            EventAssociation::Unique => {}
            EventAssociation::Ambiguous => {
                return Some("BLOCKED_AMBIGUOUS_ASSOCIATION");
            }
            EventAssociation::Unmatched => {
                return Some("BLOCKED_UNMATCHED_ASSOCIATION");
            }
            EventAssociation::NotApplicable => {
                return Some("BLOCKED_MISSING_ASSOCIATION_RECEIPT");
            }
        }
    } else if sensor.mode == SensingMode::AdvisoryOnly {
        match gate.event_association {
            EventAssociation::Ambiguous => {
                return Some("BLOCKED_AMBIGUOUS_ASSOCIATION");
            }
            EventAssociation::Unmatched => {
                return Some("BLOCKED_UNMATCHED_ASSOCIATION");
            }
            EventAssociation::NotApplicable | EventAssociation::Unique => {}
        }
    }

    None
}

fn acquisition_cost(sensor: &SensorAction) -> Option<u32> {
    let energy_cost = sensor
        .energy_kwh
        .checked_mul(sensor.service_units_per_kwh)?;
    let delay_cost = sensor
        .latency_ticks
        .checked_mul(sensor.service_units_per_latency_tick)?;
    energy_cost.checked_add(delay_cost)
}

fn clairvoyant_values() -> Option<[u32; WORLD_COUNT]> {
    let mut values = [0; WORLD_COUNT];

    for (world_index, value) in values.iter_mut().enumerate() {
        let mut found_feasible = false;
        for action in &ACTIONS {
            if action.feasible[world_index] {
                *value = (*value).max(action.delivered_service_units[world_index]);
                found_feasible = true;
            }
        }
        if !found_feasible {
            return None;
        }
    }

    Some(values)
}

fn evaluate_partitions(
    sensor: &SensorAction,
    oracle: &[u32; WORLD_COUNT],
    cost: u32,
) -> Option<(u32, Vec<PartitionDecision>)> {
    let mut partitions: BTreeMap<&'static str, Vec<usize>> = BTreeMap::new();

    for (world_index, outcome) in sensor.outcomes.iter().enumerate() {
        partitions.entry(outcome).or_default().push(world_index);
    }

    let mut decisions = Vec::with_capacity(partitions.len());
    let mut worst_case_regret = 0;

    for (outcome, world_indices) in partitions {
        let mut best_action: Option<(u32, &'static str)> = None;

        for action in &ACTIONS {
            if !world_indices
                .iter()
                .all(|&world_index| action.feasible[world_index])
            {
                continue;
            }

            let mut action_worst_regret = 0;
            for &world_index in &world_indices {
                let physical_regret = oracle[world_index]
                    .checked_sub(action.delivered_service_units[world_index])?;
                let regret = physical_regret.checked_add(cost)?;
                action_worst_regret = action_worst_regret.max(regret);
            }

            let candidate = (action_worst_regret, action.id);
            if best_action.is_none_or(|best| candidate < best) {
                best_action = Some(candidate);
            }
        }

        let (partition_regret, action_id) = best_action?;
        worst_case_regret = worst_case_regret.max(partition_regret);
        decisions.push(PartitionDecision {
            outcome,
            action_id,
            worst_case_regret_service_units: partition_regret,
        });
    }

    Some((worst_case_regret, decisions))
}

fn evaluate_sensor(
    sensor: &SensorAction,
    oracle: &[u32; WORLD_COUNT],
) -> CandidateResult {
    let evidence_receipts = sensor
        .gate
        .map(|gate| {
            format!(
                "{};{};{}",
                gate.dependency_receipt, gate.association_receipt, gate.capture_receipt
            )
        })
        .unwrap_or_else(|| "none".to_owned());

    let base = |status: &'static str,
                cost: Option<u32>,
                partitions: Option<usize>,
                regret: Option<u32>,
                actions: Vec<PartitionDecision>| CandidateResult {
        id: sensor.id,
        status,
        claim_ceiling: match sensor.mode {
            SensingMode::NoObservation => "NO_NEW_EVIDENCE",
            SensingMode::AdvisoryOnly => "ADVISORY_ONLY_NO_INDEPENDENT_CORROBORATION",
            SensingMode::IndependentWitness => {
                "INDEPENDENT_OBSERVATION_CANDIDATE_NOT_DELIVERY_PROOF"
            }
        },
        energy_kwh: sensor.energy_kwh,
        service_units_per_kwh: sensor.service_units_per_kwh,
        latency_ticks: sensor.latency_ticks,
        service_units_per_latency_tick: sensor.service_units_per_latency_tick,
        acquisition_cost_service_units: cost,
        partition_count: partitions,
        worst_case_regret_service_units: regret,
        partition_actions: actions,
        evidence_receipts: Box::leak(evidence_receipts.clone().into_boxed_str()),
        recommended: false,
    };

    if let Some(reason) = evidence_blocker(sensor) {
        return base(reason, None, None, None, Vec::new());
    }

    let Some(cost) = acquisition_cost(sensor) else {
        return base("BLOCKED_COST_OVERFLOW", None, None, None, Vec::new());
    };

    if cost > MAX_ACQUISITION_BUDGET_SERVICE_UNITS {
        return base("OVER_BUDGET", Some(cost), None, None, Vec::new());
    }

    let Some((regret, decisions)) = evaluate_partitions(sensor, oracle, cost) else {
        return base("NO_COMMON_FEASIBLE_RECOURSE", Some(cost), None, None, Vec::new());
    };

    let partition_count = decisions.len();
    base(
        "EVALUATED",
        Some(cost),
        Some(partition_count),
        Some(regret),
        decisions,
    )
}

fn choose_recommendation(results: &[CandidateResult]) -> Option<&'static str> {
    results
        .iter()
        .filter(|result| result.status == "EVALUATED")
        .filter_map(|result| {
            Some((
                result.worst_case_regret_service_units?,
                result.acquisition_cost_service_units?,
                result.id,
            ))
        })
        .min()
        .map(|(_, _, id)| id)
}

pub fn evaluate_benchmark() -> Vec<CandidateResult> {
    let Some(oracle) = clairvoyant_values() else {
        return Vec::new();
    };

    let mut results = Vec::with_capacity(SENSORS.len() + 1);
    results.push(evaluate_sensor(&NO_OBSERVATION, &oracle));
    results.extend(SENSORS.iter().map(|sensor| evaluate_sensor(sensor, &oracle)));

    let recommendation = choose_recommendation(&results);
    for result in &mut results {
        result.recommended = Some(result.id) == recommendation;
    }

    results
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "sensor_id,status,claim_ceiling,energy_kwh,service_units_per_kwh,latency_ticks,service_units_per_latency_tick,acquisition_cost_service_units,partition_count,worst_case_regret_service_units,recommended,evidence_receipts,partition_actions\n",
    );

    for result in evaluate_benchmark() {
        let cost = result
            .acquisition_cost_service_units
            .map_or_else(|| "NA".to_owned(), |value| value.to_string());
        let partitions = result
            .partition_count
            .map_or_else(|| "NA".to_owned(), |value| value.to_string());
        let regret = result
            .worst_case_regret_service_units
            .map_or_else(|| "NA".to_owned(), |value| value.to_string());
        let actions = if result.partition_actions.is_empty() {
            "NA".to_owned()
        } else {
            result
                .partition_actions
                .iter()
                .map(|decision| {
                    format!(
                        "{}:{}:{}",
                        decision.outcome,
                        decision.action_id,
                        decision.worst_case_regret_service_units
                    )
                })
                .collect::<Vec<_>>()
                .join(";")
        };

        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            result.id,
            result.status,
            result.claim_ceiling,
            result.energy_kwh,
            result.service_units_per_kwh,
            result.latency_ticks,
            result.service_units_per_latency_tick,
            cost,
            partitions,
            regret,
            result.recommended,
            result.evidence_receipts,
            actions
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result<'a>(results: &'a [CandidateResult], id: &str) -> &'a CandidateResult {
        results
            .iter()
            .find(|candidate| candidate.id == id)
            .expect("known benchmark row")
    }

    #[test]
    fn no_observation_has_the_hand_calculated_seventy_unit_regret() {
        let results = evaluate_benchmark();
        let baseline = result(&results, "no_observation");

        assert_eq!(baseline.status, "EVALUATED");
        assert_eq!(baseline.acquisition_cost_service_units, Some(0));
        assert_eq!(baseline.partition_count, Some(1));
        assert_eq!(baseline.worst_case_regret_service_units, Some(70));
        assert_eq!(baseline.partition_actions[0].action_id, "maintain_primary");
    }

    #[test]
    fn independent_inspection_reduces_worst_case_regret_net_of_cost() {
        let results = evaluate_benchmark();
        let inspection = result(&results, "independent_field_inspection");

        assert_eq!(inspection.status, "EVALUATED");
        assert_eq!(inspection.acquisition_cost_service_units, Some(5));
        assert_eq!(inspection.partition_count, Some(3));
        assert_eq!(inspection.worst_case_regret_service_units, Some(5));
        assert!(inspection.recommended);

        let selected = inspection
            .partition_actions
            .iter()
            .map(|decision| (decision.outcome, decision.action_id))
            .collect::<Vec<_>>();
        assert_eq!(
            selected,
            vec![
                ("PRIMARY_DELAY_RESERVE_READY", "use_reserve"),
                (
                    "PRIMARY_DELAY_RESERVE_UNAVAILABLE",
                    "maintain_primary"
                ),
                ("PRIMARY_READY", "maintain_primary"),
            ]
        );
    }

    #[test]
    fn shared_telemetry_is_advisory_and_has_no_net_decision_value() {
        let results = evaluate_benchmark();
        let telemetry = result(&results, "shared_tos_signal");

        assert_eq!(telemetry.status, "EVALUATED");
        assert_eq!(
            telemetry.claim_ceiling,
            "ADVISORY_ONLY_NO_INDEPENDENT_CORROBORATION"
        );
        assert_eq!(telemetry.partition_count, Some(2));
        assert_eq!(telemetry.worst_case_regret_service_units, Some(71));
        assert!(!telemetry.recommended);
        assert!(telemetry.evidence_receipts.contains("shared-gateway"));
    }

    #[test]
    fn unknown_dependency_is_blocked_even_for_a_free_perfect_signal() {
        let results = evaluate_benchmark();
        let unknown = result(&results, "unknown_dependency_feed");

        assert_eq!(unknown.status, "BLOCKED_UNKNOWN_DEPENDENCY");
        assert_eq!(unknown.worst_case_regret_service_units, None);
        assert!(!unknown.recommended);
    }

    #[test]
    fn ambiguous_event_association_and_stale_evidence_fail_closed() {
        let results = evaluate_benchmark();
        assert_eq!(
            result(&results, "ambiguous_event_camera").status,
            "BLOCKED_AMBIGUOUS_ASSOCIATION"
        );
        assert_eq!(
            result(&results, "stale_field_sensor").status,
            "BLOCKED_STALE_EVIDENCE"
        );
    }

    #[test]
    fn over_budget_audit_is_not_recommended_even_with_perfect_discrimination() {
        let results = evaluate_benchmark();
        let audit = result(&results, "overbudget_independent_audit");

        assert_eq!(audit.acquisition_cost_service_units, Some(12));
        assert_eq!(audit.status, "OVER_BUDGET");
        assert_eq!(audit.worst_case_regret_service_units, None);
        assert!(!audit.recommended);
    }

    #[test]
    fn reserve_is_never_selected_when_the_partition_contains_no_reserve_capacity() {
        let results = evaluate_benchmark();
        let inspection = result(&results, "independent_field_inspection");
        let unavailable = inspection
            .partition_actions
            .iter()
            .find(|decision| decision.outcome == "PRIMARY_DELAY_RESERVE_UNAVAILABLE")
            .expect("known independent-inspection partition");

        assert_eq!(unavailable.action_id, "maintain_primary");
    }

    #[test]
    fn only_the_independent_inspection_is_recommended() {
        let results = evaluate_benchmark();
        let selected = results
            .iter()
            .filter(|candidate| candidate.recommended)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();

        assert_eq!(selected, vec!["independent_field_inspection"]);
    }

    #[test]
    fn report_replay_is_byte_identical() {
        assert_eq!(render_csv(), render_csv());
    }
}
