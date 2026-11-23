// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeSet;

const WORLD_COUNT: usize = 5;
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
        outcome_partitions: [0, 1, 0, 0, 1],
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
        outcome_partitions: [0, 1, 2, 3, 4],
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
        outcome_partitions: [0, 1, 2, 3, 4],
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
        outcome_partitions: [0, 1, 2, 3, 4],
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
        outcome_partitions: [0, 1, 2, 3, 4],
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
        outcome_partitions: [0, 1, 2, 3, 4],
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
        outcome_partitions: [0, 1, 2, 3, 4],
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
        outcome_partitions: [0, 1, 2, 3, 4],
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
        outcome_partitions: [0, 1, 2, 3, 4],
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
        outcome_partitions: [0, 1, 2, 3, 4],
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
        ObservationMode::NoObservation | ObservationMode::AdvisoryOnly => action == Action::Hold,
        ObservationMode::IndependentWitness => true,
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
struct OraclePolicy {
    worst_regret: Regret,
    partition_regrets: Vec<Regret>,
    action_indices: Vec<usize>,
}

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

fn oracle_mode_allows(mode: ObservationMode, action_index: usize) -> bool {
    match mode {
        ObservationMode::NoObservation | ObservationMode::AdvisoryOnly => action_index == 2,
        ObservationMode::IndependentWitness => action_index < 3,
    }
}

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
            if oracle_world_outcome(&worlds[index], action_index).is_none() {
                return None;
            }
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

fn enumerate_oracle_policies(
    position: usize,
    worlds: &[World; WORLD_COUNT],
    labels: &[usize; WORLD_COUNT],
    unique: &[usize],
    mode: ObservationMode,
    acquisition_cost: u32,
    current: &mut Vec<usize>,
    best: &mut Option<OraclePolicy>,
) {
    if position == unique.len() {
        let Some(candidate) =
            oracle_policy_score(worlds, labels, unique, mode, acquisition_cost, current)
        else {
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
        enumerate_oracle_policies(
            position + 1,
            worlds,
            labels,
            unique,
            mode,
            acquisition_cost,
            current,
            best,
        );
        current.pop();
    }
}

fn exhaustive_oracle(
    worlds: &[World; WORLD_COUNT],
    labels: &[usize; WORLD_COUNT],
    mode: ObservationMode,
    acquisition_cost: u32,
) -> Option<OraclePolicy> {
    let unique = unique_labels(labels);
    let mut best = None;
    enumerate_oracle_policies(
        0,
        worlds,
        labels,
        &unique,
        mode,
        acquisition_cost,
        &mut Vec::with_capacity(unique.len()),
        &mut best,
    );
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
        "sensor_id,status,mode,worst_missed_deadline_regret,worst_service_loss_units,acquisition_cost_service_units,recommended,outcome_partition,world_id,selected_action,a_service_units,b_service_units,missed_critical_participant,reserve_available,a_authorized,b_authorized,authorization_disposition,world_missed_deadline_regret,world_service_loss_units,evidence_receipts\n",
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
                "{},{},{},{},{},{},{},NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,{}\n",
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

            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                result.id,
                result.status,
                result.mode.id(),
                worst_misses,
                worst_service_loss,
                acquisition_cost,
                result.recommended,
                decision.partition,
                world.id,
                decision.action.id(),
                decision.outcome.a_service_units,
                decision.outcome.b_service_units,
                missed_participant,
                world.reserve_available,
                world.a_authorized,
                world.b_authorized,
                authorization_disposition(world),
                decision.regret.additional_critical_deadlines_missed,
                decision.regret.service_loss_units,
                result.evidence_receipts
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
    fn shared_tos_signal_cannot_authorize_the_scarce_reserve() {
        let results = evaluate_benchmark();
        let shared = candidate(&results, "shared_tos_signal");
        let solution = shared.solution.as_ref().expect("evaluated advisory signal");

        assert_eq!(solution.actions_by_partition.len(), 2);
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
        assert_eq!(partitions.len(), 52);

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
