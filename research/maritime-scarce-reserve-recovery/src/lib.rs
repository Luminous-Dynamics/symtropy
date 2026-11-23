// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeSet;

const WORLD_COUNT: usize = 6;
const MAX_ACQUISITION_BUDGET_SERVICE_UNITS: u32 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Participant {
    A,
    B,
}

#[derive(Clone, Copy, Debug)]
struct World {
    id: &'static str,
    critical: Participant,
    reserve_available: bool,
    a_authorized: bool,
    b_authorized: bool,
}

// Both primary berths are disrupted in every world. There is exactly one
// reserve berth, and its operational/authorization state varies by world.
const WORLDS: [World; WORLD_COUNT] = [
    World {
        id: "a_critical_reserve_ready",
        critical: Participant::A,
        reserve_available: true,
        a_authorized: true,
        b_authorized: true,
    },
    World {
        id: "b_critical_reserve_ready",
        critical: Participant::B,
        reserve_available: true,
        a_authorized: true,
        b_authorized: true,
    },
    World {
        id: "a_critical_reserve_outage",
        critical: Participant::A,
        reserve_available: false,
        a_authorized: true,
        b_authorized: true,
    },
    World {
        id: "a_critical_a_consent_revoked",
        critical: Participant::A,
        reserve_available: true,
        a_authorized: false,
        b_authorized: true,
    },
    World {
        id: "b_critical_b_consent_revoked",
        critical: Participant::B,
        reserve_available: true,
        a_authorized: true,
        b_authorized: false,
    },
    World {
        id: "a_critical_reserve_ready_sensor_only_fault",
        critical: Participant::A,
        reserve_available: true,
        a_authorized: true,
        b_authorized: true,
    },
];

// Evaluator-only hidden causes. These values are emitted only in the adjudication
// receipt; they are not fields on World and are not passed into solve_policy.
// The same observed MISSING token can arise from a common-cause physical outage
// or a sensor-only fault while reserve capacity is actually available.
const EVALUATOR_MISSINGNESS_CAUSES: [&str; WORLD_COUNT] = [
    "NOT_MISSING",
    "NOT_MISSING",
    "COMMON_CAUSE_RESERVE_AND_GATEWAY_OUTAGE",
    "NOT_MISSING",
    "NOT_MISSING",
    "SENSOR_ONLY_FAILURE_RESERVE_READY",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Action {
    ReserveToA,
    ReserveToB,
    Hold,
}

const ACTIONS: [Action; 3] = [Action::ReserveToA, Action::ReserveToB, Action::Hold];

impl Action {
    fn id(self) -> &'static str {
        match self {
            Self::ReserveToA => "reserve_to_a",
            Self::ReserveToB => "reserve_to_b",
            Self::Hold => "hold",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WorldOutcome {
    a_service_units: u32,
    b_service_units: u32,
    missed_critical_deadlines: u8,
    reserve_used: bool,
}

impl WorldOutcome {
    fn total_service_units(self) -> u32 {
        self.a_service_units + self.b_service_units
    }
}

// This is a declared lexicographic objective, not a weighted score:
// first minimize additional critical-connection misses, then minimize
// service-unit loss. Hard capacity and consent constraints are evaluated
// before objective comparison.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
struct Regret {
    additional_critical_deadlines_missed: u8,
    service_loss_units: u32,
}

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
enum ObservationMode {
    NoObservation,
    AdvisoryOnly,
    IndependentWitness,
}

impl ObservationMode {
    fn id(self) -> &'static str {
        match self {
            Self::NoObservation => "NO_OBSERVATION",
            Self::AdvisoryOnly => "ADVISORY_ONLY",
            Self::IndependentWitness => "INDEPENDENT_WITNESS",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct EvidenceGate {
    dependency: DependencyDisposition,
    event_association: EventAssociation,
    capture_complete: bool,
    freshness_current: bool,
    read_only_authorized: bool,
    authorization_snapshot_current: bool,
    dependency_receipt: &'static str,
    association_receipt: &'static str,
    capture_receipt: &'static str,
    authorization_snapshot_receipt: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct SensorSpec {
    id: &'static str,
    mode: ObservationMode,
    gate: Option<EvidenceGate>,
    outcome_partitions: [usize; WORLD_COUNT],
    energy_kwh: u32,
    service_units_per_kwh: u32,
    latency_ticks: u32,
    service_units_per_latency_tick: u32,
}

const NO_OBSERVATION: SensorSpec = SensorSpec {
    id: "no_observation",
    mode: ObservationMode::NoObservation,
    gate: None,
    outcome_partitions: [0; WORLD_COUNT],
    energy_kwh: 0,
    service_units_per_kwh: 0,
    latency_ticks: 0,
    service_units_per_latency_tick: 0,
};

const SENSORS: [SensorSpec; 10] = [
    // This source reports priority classes but shares the TOS/gateway path.
    // Advisory evidence cannot authorize use of the reserve berth.
    SensorSpec {
        id: "shared_tos_signal",
        mode: ObservationMode::AdvisoryOnly,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Shared,
            event_association: EventAssociation::NotApplicable,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:shared-tos-gateway",
            association_receipt: "synthetic:not-applicable",
            capture_receipt: "synthetic:complete-tos-snapshot",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        outcome_partitions: [0, 1, 2, 0, 1, 2],
        energy_kwh: 2,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 2,
    },
    // Independent capacity/physical-state observation; the participant
    // authorization snapshot is consumed separately as an owner receipt.
    SensorSpec {
        id: "independent_reserve_inspection",
        mode: ObservationMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:unique-reserve-state-correspondence",
            capture_receipt: "synthetic:complete-independent-inspection",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        // The reserve inspection resolves operational state but cannot identify
        // why a different TOS sensor failed. Physically identical worlds 0/5
        // therefore produce the same independent observation.
        outcome_partitions: [0, 1, 2, 3, 4, 0],
        energy_kwh: 3,
        service_units_per_kwh: 1,
        latency_ticks: 2,
        service_units_per_latency_tick: 2,
    },
    SensorSpec {
        id: "unknown_dependency_feed",
        mode: ObservationMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Unknown,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:unknown-dependency",
            association_receipt: "synthetic:unique-match-claimed",
            capture_receipt: "synthetic:complete-capture-claimed",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        outcome_partitions: [0, 1, 2, 3, 4, 5],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    SensorSpec {
        id: "potentially_correlated_feed",
        mode: ObservationMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::PotentiallyCorrelated,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:potential-common-root",
            association_receipt: "synthetic:unique-event-match",
            capture_receipt: "synthetic:complete-capture",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        outcome_partitions: [0, 1, 2, 3, 4, 5],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    SensorSpec {
        id: "revoked_dependency_feed",
        mode: ObservationMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Revoked,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:revoked-dependency",
            association_receipt: "synthetic:unique-event-match",
            capture_receipt: "synthetic:complete-capture",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        outcome_partitions: [0, 1, 2, 3, 4, 5],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    SensorSpec {
        id: "ambiguous_event_camera",
        mode: ObservationMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Ambiguous,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:ambiguous-event-match",
            capture_receipt: "synthetic:complete-capture",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        outcome_partitions: [0, 1, 2, 3, 4, 5],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    SensorSpec {
        id: "unmatched_event_receipt",
        mode: ObservationMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Unmatched,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:no-event-correspondence",
            capture_receipt: "synthetic:complete-capture",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        outcome_partitions: [0, 1, 2, 3, 4, 5],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    SensorSpec {
        id: "stale_field_sensor",
        mode: ObservationMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: false,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:unique-event-match",
            capture_receipt: "synthetic:stale-field-capture",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        outcome_partitions: [0, 1, 2, 3, 4, 5],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    SensorSpec {
        id: "incomplete_capture_sensor",
        mode: ObservationMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Unique,
            capture_complete: false,
            freshness_current: true,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:unique-event-match",
            capture_receipt: "synthetic:partial-capture",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        outcome_partitions: [0, 1, 2, 3, 4, 5],
        energy_kwh: 0,
        service_units_per_kwh: 1,
        latency_ticks: 0,
        service_units_per_latency_tick: 1,
    },
    SensorSpec {
        id: "overbudget_independent_audit",
        mode: ObservationMode::IndependentWitness,
        gate: Some(EvidenceGate {
            dependency: DependencyDisposition::Separated,
            event_association: EventAssociation::Unique,
            capture_complete: true,
            freshness_current: true,
            read_only_authorized: true,
            authorization_snapshot_current: true,
            dependency_receipt: "synthetic:stage-dimension-separated",
            association_receipt: "synthetic:unique-event-match",
            capture_receipt: "synthetic:complete-independent-capture",
            authorization_snapshot_receipt: "synthetic:mycelix-auth-snapshot-r10",
        }),
        outcome_partitions: [0, 1, 2, 3, 4, 5],
        energy_kwh: 5,
        service_units_per_kwh: 1,
        latency_ticks: 3,
        service_units_per_latency_tick: 2,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct PolicySolution {
    worst_regret: Regret,
    partition_regrets: Vec<(usize, Regret)>,
    actions_by_partition: Vec<(usize, Action)>,
}

#[derive(Clone, Debug)]
struct WorldDecision {
    world_index: usize,
    action: Action,
    outcome: WorldOutcome,
    regret: Regret,
    partition: usize,
}

#[derive(Clone, Debug)]
struct CandidateResult {
    id: &'static str,
    mode: ObservationMode,
    status: &'static str,
    acquisition_cost_service_units: Option<u32>,
    solution: Option<PolicySolution>,
    world_decisions: Vec<WorldDecision>,
    evidence_receipts: String,
    recommended: bool,
}

fn action_feasible(action: Action, world: &World) -> bool {
    match action {
        Action::ReserveToA => world.reserve_available && world.a_authorized,
        Action::ReserveToB => world.reserve_available && world.b_authorized,
        Action::Hold => true,
    }
}

fn mode_allows_action(mode: ObservationMode, action: Action) -> bool {
    match mode {
        ObservationMode::NoObservation | ObservationMode::IndependentWitness => true,
        ObservationMode::AdvisoryOnly => action == Action::Hold,
    }
}

fn outcome_for(action: Action, world: &World) -> Option<WorldOutcome> {
    if !action_feasible(action, world) {
        return None;
    }

    let (a_service_units, b_service_units, reserve_used) = match action {
        Action::ReserveToA => (80, 30, true),
        Action::ReserveToB => (30, 80, true),
        Action::Hold => (30, 30, false),
    };

    let critical_served = matches!(
        (world.critical, action),
        (Participant::A, Action::ReserveToA) | (Participant::B, Action::ReserveToB)
    );

    Some(WorldOutcome {
        a_service_units,
        b_service_units,
        missed_critical_deadlines: u8::from(!critical_served),
        reserve_used,
    })
}

fn better_utility(candidate: WorldOutcome, incumbent: WorldOutcome) -> bool {
    candidate.missed_critical_deadlines < incumbent.missed_critical_deadlines
        || (candidate.missed_critical_deadlines == incumbent.missed_critical_deadlines
            && candidate.total_service_units() > incumbent.total_service_units())
}

fn best_world_outcome(world: &World) -> Option<WorldOutcome> {
    let mut best = None;
    for action in ACTIONS {
        let Some(candidate) = outcome_for(action, world) else {
            continue;
        };
        if best.is_none_or(|incumbent| better_utility(candidate, incumbent)) {
            best = Some(candidate);
        }
    }
    best
}

fn regret_for(
    actual: WorldOutcome,
    best: WorldOutcome,
    acquisition_cost_service_units: u32,
) -> Regret {
    Regret {
        additional_critical_deadlines_missed: actual
            .missed_critical_deadlines
            .saturating_sub(best.missed_critical_deadlines),
        service_loss_units: best
            .total_service_units()
            .saturating_sub(actual.total_service_units())
            .saturating_add(acquisition_cost_service_units),
    }
}

fn unique_labels(labels: &[usize; WORLD_COUNT]) -> Vec<usize> {
    labels
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn solve_policy(
    worlds: &[World; WORLD_COUNT],
    outcome_partitions: &[usize; WORLD_COUNT],
    mode: ObservationMode,
    acquisition_cost_service_units: u32,
) -> Option<PolicySolution> {
    let mut actions_by_partition = Vec::new();
    let mut partition_regrets = Vec::new();
    let mut worst_regret = Regret::default();

    for partition in unique_labels(outcome_partitions) {
        let world_indices = (0..WORLD_COUNT)
            .filter(|&index| outcome_partitions[index] == partition)
            .collect::<Vec<_>>();

        let mut best_choice: Option<(Regret, Action)> = None;

        for action in ACTIONS {
            if !mode_allows_action(mode, action)
                || !world_indices
                    .iter()
                    .all(|&index| action_feasible(action, &worlds[index]))
            {
                continue;
            }

            let mut action_worst_regret = Regret::default();
            for &index in &world_indices {
                let best = best_world_outcome(&worlds[index])?;
                let actual = outcome_for(action, &worlds[index])?;
                action_worst_regret = action_worst_regret.max(regret_for(
                    actual,
                    best,
                    acquisition_cost_service_units,
                ));
            }

            let choice = (action_worst_regret, action);
            if best_choice.is_none_or(|incumbent| choice < incumbent) {
                best_choice = Some(choice);
            }
        }

        let (partition_regret, action) = best_choice?;
        worst_regret = worst_regret.max(partition_regret);
        partition_regrets.push((partition, partition_regret));
        actions_by_partition.push((partition, action));
    }

    Some(PolicySolution {
        worst_regret,
        partition_regrets,
        actions_by_partition,
    })
}

fn evidence_blocker(sensor: &SensorSpec) -> Option<&'static str> {
    if sensor.mode == ObservationMode::NoObservation {
        return None;
    }

    let Some(gate) = sensor.gate else {
        return Some("BLOCKED_MISSING_GATE_RECEIPT");
    };

    if gate.dependency_receipt.is_empty() {
        return Some("BLOCKED_MISSING_DEPENDENCY_RECEIPT");
    }
    if gate.capture_receipt.is_empty() {
        return Some("BLOCKED_MISSING_CAPTURE_RECEIPT");
    }
    if gate.authorization_snapshot_receipt.is_empty() {
        return Some("BLOCKED_MISSING_AUTHORIZATION_SNAPSHOT");
    }
    if sensor.mode == ObservationMode::IndependentWitness && gate.association_receipt.is_empty() {
        return Some("BLOCKED_MISSING_ASSOCIATION_RECEIPT");
    }
    if !gate.read_only_authorized {
        return Some("BLOCKED_NOT_AUTHORIZED_READ_ONLY");
    }
    if !gate.authorization_snapshot_current {
        return Some("BLOCKED_STALE_AUTHORIZATION");
    }
    if !gate.capture_complete {
        return Some("BLOCKED_INCOMPLETE_CAPTURE");
    }
    if !gate.freshness_current {
        return Some("BLOCKED_STALE_EVIDENCE");
    }

    match gate.dependency {
        DependencyDisposition::Unknown => {
            return Some("BLOCKED_UNKNOWN_DEPENDENCY");
        }
        DependencyDisposition::PotentiallyCorrelated => {
            return Some("BLOCKED_POTENTIALLY_CORRELATED");
        }
        DependencyDisposition::Revoked => {
            return Some("BLOCKED_REVOKED_DEPENDENCY");
        }
        DependencyDisposition::Shared if sensor.mode == ObservationMode::IndependentWitness => {
            return Some("BLOCKED_SHARED_DEPENDENCY");
        }
        DependencyDisposition::Separated | DependencyDisposition::Shared => {}
    }

    if sensor.mode == ObservationMode::IndependentWitness {
        match gate.dependency {
            DependencyDisposition::Separated => {}
            _ => return Some("BLOCKED_DEPENDENCY_NOT_SEPARATED"),
        }
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
    } else if sensor.mode == ObservationMode::AdvisoryOnly {
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

fn acquisition_cost(sensor: &SensorSpec) -> Option<u32> {
    sensor
        .energy_kwh
        .checked_mul(sensor.service_units_per_kwh)?
        .checked_add(
            sensor
                .latency_ticks
                .checked_mul(sensor.service_units_per_latency_tick)?,
        )
}

fn receipt_summary(sensor: &SensorSpec) -> String {
    sensor
        .gate
        .map(|gate| {
            format!(
                "{}:{}:{}:{}",
                gate.dependency_receipt,
                gate.association_receipt,
                gate.capture_receipt,
                gate.authorization_snapshot_receipt
            )
        })
        .unwrap_or_else(|| "none".to_owned())
}

fn evaluate_sensor(sensor: &SensorSpec) -> CandidateResult {
    let receipts = receipt_summary(sensor);

    let blocked = |status| CandidateResult {
        id: sensor.id,
        mode: sensor.mode,
        status,
        acquisition_cost_service_units: None,
        solution: None,
        world_decisions: Vec::new(),
        evidence_receipts: receipts.clone(),
        recommended: false,
    };

    if let Some(reason) = evidence_blocker(sensor) {
        return blocked(reason);
    }

    let Some(cost) = acquisition_cost(sensor) else {
        return blocked("BLOCKED_COST_OVERFLOW");
    };

    if cost > MAX_ACQUISITION_BUDGET_SERVICE_UNITS {
        let mut result = blocked("OVER_BUDGET");
        result.acquisition_cost_service_units = Some(cost);
        return result;
    }

    let Some(solution) = solve_policy(&WORLDS, &sensor.outcome_partitions, sensor.mode, cost)
    else {
        let mut result = blocked("NO_COMMON_FEASIBLE_ACTION");
        result.acquisition_cost_service_units = Some(cost);
        return result;
    };

    let mut world_decisions = Vec::with_capacity(WORLD_COUNT);
    for (world_index, world) in WORLDS.iter().enumerate() {
        let partition = sensor.outcome_partitions[world_index];
        let Some(action) = solution
            .actions_by_partition
            .iter()
            .find(|(candidate_partition, _)| *candidate_partition == partition)
            .map(|(_, action)| *action)
        else {
            return blocked("BLOCKED_MISSING_POLICY_PARTITION");
        };
        let Some(outcome) = outcome_for(action, world) else {
            return blocked("BLOCKED_INFEASIBLE_POLICY_ACTION");
        };
        let Some(best) = best_world_outcome(world) else {
            return blocked("BLOCKED_NO_FEASIBLE_REFERENCE_ACTION");
        };
        world_decisions.push(WorldDecision {
            world_index,
            action,
            outcome,
            regret: regret_for(outcome, best, cost),
            partition,
        });
    }

    CandidateResult {
        id: sensor.id,
        mode: sensor.mode,
        status: "EVALUATED",
        acquisition_cost_service_units: Some(cost),
        solution: Some(solution),
        world_decisions,
        evidence_receipts: receipts,
        recommended: false,
    }
}

fn choose_recommendation(results: &[CandidateResult]) -> Option<&'static str> {
    results
        .iter()
        .filter(|result| result.status == "EVALUATED")
        .filter_map(|result| {
            Some((
                result.solution.as_ref()?.worst_regret,
                result.acquisition_cost_service_units?,
                result.id,
            ))
        })
        .min()
        .map(|(_, _, id)| id)
}

fn evaluate_benchmark() -> Vec<CandidateResult> {
    let mut results = Vec::with_capacity(SENSORS.len() + 1);
    results.push(evaluate_sensor(&NO_OBSERVATION));
    results.extend(SENSORS.iter().map(evaluate_sensor));

    let recommendation = choose_recommendation(&results);
    for result in &mut results {
        result.recommended = Some(result.id) == recommendation;
    }
    results
}

// Independent exhaustive oracle.
//
// This implementation intentionally does not call solve_policy, action_feasible,
// outcome_for, best_world_outcome or regret_for. It enumerates every deterministic
// mapping from observed partitions to actions and evaluates feasibility and
// lexicographic regret through a separate direct formulation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(test)]
struct OraclePolicy {
    worst_regret: Regret,
    partition_regrets: Vec<Regret>,
    action_indices: Vec<usize>,
}

#[cfg(test)]
fn oracle_world_outcome(world: &World, action_index: usize) -> Option<(u32, u32, u8)> {
    match action_index {
        0 if world.reserve_available && world.a_authorized => {
            Some((80, 30, u8::from(world.critical != Participant::A)))
        }
        1 if world.reserve_available && world.b_authorized => {
            Some((30, 80, u8::from(world.critical != Participant::B)))
        }
        2 => Some((30, 30, 1)),
        _ => None,
    }
}

#[cfg(test)]
fn oracle_mode_allows(mode: ObservationMode, action_index: usize) -> bool {
    match mode {
        ObservationMode::NoObservation | ObservationMode::IndependentWitness => action_index < 3,
        ObservationMode::AdvisoryOnly => action_index == 2,
    }
}

#[cfg(test)]
fn oracle_best_world_value(world: &World) -> Option<(u32, u32, u8)> {
    let mut best: Option<(u32, u32, u8)> = None;
    for action_index in 0..3 {
        let Some((a, b, misses)) = oracle_world_outcome(world, action_index) else {
            continue;
        };
        let candidate = (a, b, misses);
        let candidate_is_better = best.is_none_or(|incumbent| {
            misses < incumbent.2 || (misses == incumbent.2 && a + b > incumbent.0 + incumbent.1)
        });
        if candidate_is_better {
            best = Some(candidate);
        }
    }
    best
}

#[cfg(test)]
fn oracle_regret(world: &World, action_index: usize, acquisition_cost: u32) -> Option<Regret> {
    let (actual_a, actual_b, actual_misses) = oracle_world_outcome(world, action_index)?;
    let (best_a, best_b, best_misses) = oracle_best_world_value(world)?;
    Some(Regret {
        additional_critical_deadlines_missed: actual_misses.saturating_sub(best_misses),
        service_loss_units: (best_a + best_b)
            .saturating_sub(actual_a + actual_b)
            .saturating_add(acquisition_cost),
    })
}

#[cfg(test)]
fn oracle_policy_score(
    worlds: &[World; WORLD_COUNT],
    labels: &[usize; WORLD_COUNT],
    unique: &[usize],
    mode: ObservationMode,
    acquisition_cost: u32,
    action_indices: &[usize],
) -> Option<OraclePolicy> {
    let mut partition_regrets = Vec::with_capacity(unique.len());
    let mut worst = Regret::default();

    for (position, label) in unique.iter().enumerate() {
        let action_index = action_indices[position];
        if !oracle_mode_allows(mode, action_index) {
            return None;
        }

        let indices = (0..WORLD_COUNT)
            .filter(|&index| labels[index] == *label)
            .collect::<Vec<_>>();
        let mut partition_worst = Regret::default();

        for index in indices {
            oracle_world_outcome(&worlds[index], action_index)?;
            let regret = oracle_regret(&worlds[index], action_index, acquisition_cost)?;
            partition_worst = partition_worst.max(regret);
        }

        worst = worst.max(partition_worst);
        partition_regrets.push(partition_worst);
    }

    Some(OraclePolicy {
        worst_regret: worst,
        partition_regrets,
        action_indices: action_indices.to_vec(),
    })
}

#[cfg(test)]
fn oracle_policy_is_better(candidate: &OraclePolicy, incumbent: &OraclePolicy) -> bool {
    (
        candidate.worst_regret,
        &candidate.partition_regrets,
        &candidate.action_indices,
    ) < (
        incumbent.worst_regret,
        &incumbent.partition_regrets,
        &incumbent.action_indices,
    )
}

#[cfg(test)]
struct OracleSearch<'a> {
    worlds: &'a [World; WORLD_COUNT],
    labels: &'a [usize; WORLD_COUNT],
    unique: &'a [usize],
    mode: ObservationMode,
    acquisition_cost: u32,
}

#[cfg(test)]
fn enumerate_oracle_policies(
    search: &OracleSearch<'_>,
    position: usize,
    current: &mut Vec<usize>,
    best: &mut Option<OraclePolicy>,
) {
    if position == search.unique.len() {
        let Some(candidate) = oracle_policy_score(
            search.worlds,
            search.labels,
            search.unique,
            search.mode,
            search.acquisition_cost,
            current,
        ) else {
            return;
        };
        if best
            .as_ref()
            .is_none_or(|incumbent| oracle_policy_is_better(&candidate, incumbent))
        {
            *best = Some(candidate);
        }
        return;
    }

    for action_index in 0..3 {
        current.push(action_index);
        enumerate_oracle_policies(search, position + 1, current, best);
        current.pop();
    }
}

#[cfg(test)]
fn exhaustive_oracle(
    worlds: &[World; WORLD_COUNT],
    labels: &[usize; WORLD_COUNT],
    mode: ObservationMode,
    acquisition_cost: u32,
) -> Option<OraclePolicy> {
    let unique = unique_labels(labels);
    let search = OracleSearch {
        worlds,
        labels,
        unique: &unique,
        mode,
        acquisition_cost,
    };
    let mut best = None;
    enumerate_oracle_policies(&search, 0, &mut Vec::with_capacity(unique.len()), &mut best);
    best
}

fn authorization_disposition(world: &World) -> &'static str {
    match (world.a_authorized, world.b_authorized) {
        (true, true) => "BOTH_VALID",
        (false, true) => "A_REVOKED",
        (true, false) => "B_REVOKED",
        (false, false) => "BOTH_REVOKED",
    }
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "sensor_id,status,mode,worst_missed_deadline_regret,worst_service_loss_units,acquisition_cost_service_units,recommended,outcome_partition,observed_outcome_label,world_id,selected_action,reserve_used,a_service_units,b_service_units,missed_critical_participant,reserve_available,a_authorized,b_authorized,authorization_disposition,world_missed_deadline_regret,world_service_loss_units,partition_missed_deadline_regret,partition_service_loss_units,evidence_receipts,evaluator_missingness_cause\n",
    );

    for result in evaluate_benchmark() {
        let worst_misses = result.solution.as_ref().map_or_else(
            || "NA".to_owned(),
            |solution| {
                solution
                    .worst_regret
                    .additional_critical_deadlines_missed
                    .to_string()
            },
        );
        let worst_service_loss = result.solution.as_ref().map_or_else(
            || "NA".to_owned(),
            |solution| solution.worst_regret.service_loss_units.to_string(),
        );
        let acquisition_cost = result
            .acquisition_cost_service_units
            .map_or_else(|| "NA".to_owned(), |value| value.to_string());

        if result.world_decisions.is_empty() {
            output.push_str(&format!(
                "{},{},{},{},{},{},{},NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,{},NA\n",
                result.id,
                result.status,
                result.mode.id(),
                worst_misses,
                worst_service_loss,
                acquisition_cost,
                result.recommended,
                result.evidence_receipts
            ));
            continue;
        }

        for decision in &result.world_decisions {
            let world = &WORLDS[decision.world_index];
            let missed_participant = if decision.outcome.missed_critical_deadlines == 0 {
                "NONE"
            } else {
                match world.critical {
                    Participant::A => "A",
                    Participant::B => "B",
                }
            };

            let partition_regret = result.solution.as_ref().and_then(|solution| {
                solution
                    .partition_regrets
                    .iter()
                    .find(|(partition, _)| *partition == decision.partition)
                    .map(|(_, regret)| *regret)
            });
            let partition_misses = partition_regret.map_or_else(
                || "MISSING".to_owned(),
                |regret| regret.additional_critical_deadlines_missed.to_string(),
            );
            let partition_service_loss = partition_regret.map_or_else(
                || "MISSING".to_owned(),
                |regret| regret.service_loss_units.to_string(),
            );

            let observed_outcome_label = if result.id == "shared_tos_signal" {
                match decision.partition {
                    0 => "A_PRIORITY",
                    1 => "B_PRIORITY",
                    2 => "MISSING",
                    _ => "UNKNOWN_PARTITION",
                }
            } else {
                "QUALIFIED_RESULT"
            };

            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                result.id,
                result.status,
                result.mode.id(),
                worst_misses,
                worst_service_loss,
                acquisition_cost,
                result.recommended,
                decision.partition,
                observed_outcome_label,
                world.id,
                decision.action.id(),
                decision.outcome.reserve_used,
                decision.outcome.a_service_units,
                decision.outcome.b_service_units,
                missed_participant,
                world.reserve_available,
                world.a_authorized,
                world.b_authorized,
                authorization_disposition(world),
                decision.regret.additional_critical_deadlines_missed,
                decision.regret.service_loss_units,
                partition_misses,
                partition_service_loss,
                result.evidence_receipts,
                EVALUATOR_MISSINGNESS_CAUSES[decision.world_index]
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate<'a>(results: &'a [CandidateResult], id: &str) -> &'a CandidateResult {
        results
            .iter()
            .find(|result| result.id == id)
            .expect("known synthetic sensor")
    }

    fn all_partitions(count: usize) -> Vec<Vec<usize>> {
        fn extend(
            count: usize,
            labels: &mut Vec<usize>,
            max_label: usize,
            output: &mut Vec<Vec<usize>>,
        ) {
            if labels.len() == count {
                output.push(labels.clone());
                return;
            }
            for next_label in 0..=max_label + 1 {
                labels.push(next_label);
                extend(count, labels, max_label.max(next_label), output);
                labels.pop();
            }
        }

        let mut output = Vec::new();
        let mut labels = vec![0];
        extend(count, &mut labels, 0, &mut output);
        output
    }

    #[test]
    fn no_observation_preserves_the_reserve_and_exposes_network_regret() {
        let results = evaluate_benchmark();
        let baseline = candidate(&results, "no_observation");
        let solution = baseline.solution.as_ref().expect("evaluated baseline");

        assert_eq!(
            solution.worst_regret,
            Regret {
                additional_critical_deadlines_missed: 1,
                service_loss_units: 50,
            }
        );
        assert!(
            solution
                .actions_by_partition
                .iter()
                .all(|(_, action)| *action == Action::Hold)
        );
        assert_eq!(baseline.world_decisions.len(), WORLD_COUNT);
    }

    #[test]
    fn no_observation_may_choose_any_action_but_hard_feasibility_forces_hold() {
        let results = evaluate_benchmark();
        let baseline = candidate(&results, "no_observation");
        let policy = baseline.solution.as_ref().expect("robust baseline");

        assert_eq!(policy.actions_by_partition, vec![(0, Action::Hold)]);
        assert!(
            WORLDS
                .iter()
                .all(|world| action_feasible(Action::Hold, world))
        );
        assert!(
            !WORLDS
                .iter()
                .all(|world| action_feasible(Action::ReserveToA, world))
        );
        assert!(
            !WORLDS
                .iter()
                .all(|world| action_feasible(Action::ReserveToB, world))
        );
    }

    #[test]
    fn shared_tos_signal_cannot_authorize_the_scarce_reserve() {
        let results = evaluate_benchmark();
        let shared = candidate(&results, "shared_tos_signal");
        let solution = shared.solution.as_ref().expect("evaluated advisory signal");

        assert_eq!(solution.actions_by_partition.len(), 3);
        assert!(
            solution
                .actions_by_partition
                .iter()
                .all(|(_, action)| *action == Action::Hold)
        );
        assert_eq!(
            solution.worst_regret,
            Regret {
                additional_critical_deadlines_missed: 1,
                service_loss_units: 52,
            }
        );
    }

    #[test]
    fn independent_inspection_selects_the_right_participant_and_preserves_consent() {
        let results = evaluate_benchmark();
        let inspection = candidate(&results, "independent_reserve_inspection");
        let solution = inspection.solution.as_ref().expect("qualified inspection");

        assert_eq!(
            solution.worst_regret,
            Regret {
                additional_critical_deadlines_missed: 0,
                service_loss_units: 7,
            }
        );
        assert_eq!(
            solution.actions_by_partition,
            vec![
                (0, Action::ReserveToA),
                (1, Action::ReserveToB),
                (2, Action::Hold),
                (3, Action::ReserveToB),
                (4, Action::ReserveToA),
            ]
        );

        let revoked_a = &inspection.world_decisions[3];
        assert_eq!(revoked_a.action, Action::ReserveToB);
        assert_eq!(authorization_disposition(&WORLDS[3]), "A_REVOKED");

        let revoked_b = &inspection.world_decisions[4];
        assert_eq!(revoked_b.action, Action::ReserveToA);
        assert_eq!(authorization_disposition(&WORLDS[4]), "B_REVOKED");

        // Absolute deadline misses remain visible in worlds where no
        // consent-respecting feasible action can serve the critical party.
        // They are not additional regret relative to the feasible oracle.
        assert_eq!(
            inspection
                .world_decisions
                .iter()
                .filter(|decision| decision.outcome.missed_critical_deadlines > 0)
                .count(),
            3
        );
        assert!(
            inspection
                .world_decisions
                .iter()
                .all(|decision| decision.regret.additional_critical_deadlines_missed == 0)
        );

        assert!(inspection.recommended);
    }

    #[test]
    fn reserve_is_never_allocated_when_unavailable_or_without_participant_consent() {
        let results = evaluate_benchmark();
        let inspection = candidate(&results, "independent_reserve_inspection");

        assert_eq!(inspection.world_decisions[2].action, Action::Hold);
        assert!(!inspection.world_decisions[2].outcome.reserve_used);
        assert_eq!(inspection.world_decisions[3].action, Action::ReserveToB);
        assert_eq!(inspection.world_decisions[4].action, Action::ReserveToA);
    }

    #[test]
    fn unknown_correlated_revoked_stale_ambiguous_unmatched_and_incomplete_evidence_block() {
        let results = evaluate_benchmark();
        let expected = [
            ("unknown_dependency_feed", "BLOCKED_UNKNOWN_DEPENDENCY"),
            (
                "potentially_correlated_feed",
                "BLOCKED_POTENTIALLY_CORRELATED",
            ),
            ("revoked_dependency_feed", "BLOCKED_REVOKED_DEPENDENCY"),
            ("ambiguous_event_camera", "BLOCKED_AMBIGUOUS_ASSOCIATION"),
            ("unmatched_event_receipt", "BLOCKED_UNMATCHED_ASSOCIATION"),
            ("stale_field_sensor", "BLOCKED_STALE_EVIDENCE"),
            ("incomplete_capture_sensor", "BLOCKED_INCOMPLETE_CAPTURE"),
        ];

        for (id, status) in expected {
            let result = candidate(&results, id);
            assert_eq!(result.status, status, "{id}");
            assert!(result.solution.is_none(), "{id}");
            assert!(!result.recommended, "{id}");
        }
    }

    #[test]
    fn overbudget_inspection_is_not_recommended_even_with_exact_outcomes() {
        let results = evaluate_benchmark();
        let audit = candidate(&results, "overbudget_independent_audit");

        assert_eq!(audit.status, "OVER_BUDGET");
        assert_eq!(audit.acquisition_cost_service_units, Some(11));
        assert!(audit.solution.is_none());
        assert!(!audit.recommended);
    }

    #[test]
    fn same_missing_token_hides_distinct_common_cause_and_sensor_only_worlds() {
        let shared = &SENSORS[0];

        assert_eq!(shared.outcome_partitions[2], shared.outcome_partitions[5]);
        assert_eq!(
            SENSORS[1].outcome_partitions[0],
            SENSORS[1].outcome_partitions[5]
        );
        let inspection = candidate(&evaluate_benchmark(), "independent_reserve_inspection");
        assert_eq!(
            inspection.world_decisions[0].action,
            inspection.world_decisions[5].action
        );
        assert_eq!(
            EVALUATOR_MISSINGNESS_CAUSES[2],
            "COMMON_CAUSE_RESERVE_AND_GATEWAY_OUTAGE"
        );
        assert_eq!(
            EVALUATOR_MISSINGNESS_CAUSES[5],
            "SENSOR_ONLY_FAILURE_RESERVE_READY"
        );
        assert!(!WORLDS[2].reserve_available);
        assert!(WORLDS[5].reserve_available);

        // The policy receives the same MISSING partition for both worlds;
        // the evaluator-only cause is not part of the policy input.
        let shared_solution = solve_policy(
            &WORLDS,
            &shared.outcome_partitions,
            shared.mode,
            acquisition_cost(shared).expect("valid shared-source cost"),
        )
        .expect("advisory signal remains evaluable");
        let missing_partition_action = shared_solution
            .actions_by_partition
            .iter()
            .find(|(partition, _)| *partition == shared.outcome_partitions[2])
            .map(|(_, action)| *action)
            .expect("missing observation partition");
        assert_eq!(missing_partition_action, Action::Hold);
    }

    #[test]
    fn independent_observation_is_the_only_recommendation() {
        let results = evaluate_benchmark();
        let selected = results
            .iter()
            .filter(|result| result.recommended)
            .map(|result| result.id)
            .collect::<Vec<_>>();

        assert_eq!(selected, vec!["independent_reserve_inspection"]);
    }

    #[test]
    fn production_partition_solver_matches_independent_exhaustive_policy_oracle() {
        let partitions = all_partitions(WORLD_COUNT);
        assert_eq!(partitions.len(), 203);

        for partition in partitions {
            let labels: [usize; WORLD_COUNT] = partition
                .as_slice()
                .try_into()
                .expect("five-world partition");
            let production = solve_policy(&WORLDS, &labels, ObservationMode::IndependentWitness, 0)
                .expect("hold is feasible in every partition");
            let oracle =
                exhaustive_oracle(&WORLDS, &labels, ObservationMode::IndependentWitness, 0)
                    .expect("at least one feasible deterministic policy");

            assert_eq!(production.worst_regret, oracle.worst_regret);
            assert_eq!(
                production.partition_regrets.len(),
                oracle.partition_regrets.len()
            );
            assert_eq!(
                production
                    .partition_regrets
                    .iter()
                    .map(|(_, regret)| *regret)
                    .collect::<Vec<_>>(),
                oracle.partition_regrets
            );
            assert_eq!(
                production
                    .actions_by_partition
                    .iter()
                    .map(|(_, action)| *action)
                    .collect::<Vec<_>>(),
                oracle
                    .action_indices
                    .iter()
                    .map(|index| ACTIONS[*index])
                    .collect::<Vec<_>>()
            );
        }
    }

    fn generated_worlds(seed: u64) -> [World; WORLD_COUNT] {
        std::array::from_fn(|index| {
            let mut key = seed.wrapping_add((index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            key ^= key >> 30;
            key = key.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            key ^= key >> 27;
            key = key.wrapping_mul(0x94D0_49BB_1331_11EB);
            key ^= key >> 31;

            World {
                id: "generated_world",
                critical: if key & 1 == 0 {
                    Participant::A
                } else {
                    Participant::B
                },
                reserve_available: key & 2 != 0,
                a_authorized: key & 4 != 0,
                b_authorized: key & 8 != 0,
            }
        })
    }

    #[test]
    fn generated_world_sets_match_exhaustive_oracle_for_every_partition() {
        let partitions = all_partitions(WORLD_COUNT);
        assert_eq!(partitions.len(), 203);

        for seed in 0..64 {
            let worlds = generated_worlds(seed);
            for partition in &partitions {
                let labels: [usize; WORLD_COUNT] = partition
                    .as_slice()
                    .try_into()
                    .expect("five-world partition");
                let production =
                    solve_policy(&worlds, &labels, ObservationMode::IndependentWitness, 0)
                        .expect("hold is feasible in every world");
                let oracle =
                    exhaustive_oracle(&worlds, &labels, ObservationMode::IndependentWitness, 0)
                        .expect("independent exhaustive policy exists");

                assert_eq!(production.worst_regret, oracle.worst_regret);
                assert_eq!(
                    production
                        .partition_regrets
                        .iter()
                        .map(|(_, regret)| *regret)
                        .collect::<Vec<_>>(),
                    oracle.partition_regrets,
                    "seed {seed}, partition {partition:?}"
                );
                assert_eq!(
                    production
                        .actions_by_partition
                        .iter()
                        .map(|(_, action)| *action)
                        .collect::<Vec<_>>(),
                    oracle
                        .action_indices
                        .iter()
                        .map(|index| ACTIONS[*index])
                        .collect::<Vec<_>>(),
                    "seed {seed}, partition {partition:?}"
                );
            }
        }
    }

    #[test]
    fn all_three_named_observation_policies_match_exhaustive_oracle() {
        for sensor in [&NO_OBSERVATION, &SENSORS[0], &SENSORS[1]] {
            let cost = acquisition_cost(sensor).expect("valid synthetic acquisition cost");
            let production = solve_policy(&WORLDS, &sensor.outcome_partitions, sensor.mode, cost)
                .expect("valid deterministic policy");
            let oracle = exhaustive_oracle(&WORLDS, &sensor.outcome_partitions, sensor.mode, cost)
                .expect("independent exhaustive oracle");

            assert_eq!(production.worst_regret, oracle.worst_regret);
            assert_eq!(
                production
                    .actions_by_partition
                    .iter()
                    .map(|(_, action)| *action)
                    .collect::<Vec<_>>(),
                oracle
                    .action_indices
                    .iter()
                    .map(|index| ACTIONS[*index])
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn every_selected_policy_preserves_the_declared_participant_service_floor() {
        let results = evaluate_benchmark();
        let inspection = candidate(&results, "independent_reserve_inspection");

        assert_eq!(inspection.world_decisions.len(), WORLD_COUNT);
        for decision in &inspection.world_decisions {
            assert!(decision.outcome.a_service_units >= 30);
            assert!(decision.outcome.b_service_units >= 30);
        }
    }

    #[test]
    fn unauthorised_read_only_scope_and_stale_authorization_block_sensing() {
        let mut not_read_only = SENSORS[1];
        let mut gate = not_read_only.gate.expect("inspection gate");
        gate.read_only_authorized = false;
        not_read_only.gate = Some(gate);
        assert_eq!(
            evidence_blocker(&not_read_only),
            Some("BLOCKED_NOT_AUTHORIZED_READ_ONLY")
        );

        let mut stale_authorization = SENSORS[1];
        let mut gate = stale_authorization.gate.expect("inspection gate");
        gate.authorization_snapshot_current = false;
        stale_authorization.gate = Some(gate);
        assert_eq!(
            evidence_blocker(&stale_authorization),
            Some("BLOCKED_STALE_AUTHORIZATION")
        );
    }

    #[test]
    fn csv_rows_match_header_width_and_preserve_missingness_adjudication() {
        let report = render_csv();
        let mut rows = report.lines();
        let header = rows.next().expect("CSV header");
        let column_count = header.split(',').count();

        assert_eq!(column_count, 25);
        for row in rows {
            assert_eq!(row.split(',').count(), column_count, "{row}");
        }

        assert!(report.contains(
            "MISSING,a_critical_reserve_outage,hold,false,30,30,A,false,true,true,BOTH_VALID,0,2,1,52"
        ));
        assert!(report.contains(
            "MISSING,a_critical_reserve_ready_sensor_only_fault,hold,false,30,30,A,true,true,true,BOTH_VALID,1,52,1,52"
        ));
        assert!(report.contains("COMMON_CAUSE_RESERVE_AND_GATEWAY_OUTAGE"));
        assert!(report.contains("SENSOR_ONLY_FAILURE_RESERVE_READY"));
    }

    #[test]
    fn deterministic_report_replay_is_byte_identical() {
        assert_eq!(render_csv(), render_csv());
    }

    #[test]
    fn missing_gate_and_missing_authorization_receipt_fail_closed() {
        let mut missing_gate = SENSORS[1];
        missing_gate.gate = None;
        assert_eq!(
            evidence_blocker(&missing_gate),
            Some("BLOCKED_MISSING_GATE_RECEIPT")
        );

        let mut missing_authorization = SENSORS[1];
        let mut gate = missing_authorization.gate.expect("inspection gate");
        gate.authorization_snapshot_receipt = "";
        missing_authorization.gate = Some(gate);
        assert_eq!(
            evidence_blocker(&missing_authorization),
            Some("BLOCKED_MISSING_AUTHORIZATION_SNAPSHOT")
        );
    }
}
