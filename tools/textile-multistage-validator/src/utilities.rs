#![forbid(unsafe_code)]

//! Independent, time-indexed utility-envelope feasibility checks.
//!
//! This module checks declared electricity, steam, water and wastewater
//! demands against known time-window capacities. It is separate from the
//! machine/operator validator so missing meter or demand data cannot be
//! silently treated as zero consumption.

use crate::{Problem, Schedule};
use std::collections::{BTreeMap, BTreeSet};

/// A caller-declared utility measure. Units and tick scale are explicit in the
/// model: electricity is peak watts; fluid rates are declared units per tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UtilityMetric {
    /// Electrical real power, watts.
    ElectricalPowerW,
    /// Steam mass flow, grams per caller-declared tick.
    SteamFlowGramsPerTick,
    /// Water flow, millilitres per caller-declared tick.
    WaterFlowMlPerTick,
    /// Wastewater flow, millilitres per caller-declared tick.
    WastewaterFlowMlPerTick,
}

/// Distinguishes a production operation from a machine setup task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UtilityConsumerKind {
    /// Scheduled production operation.
    Operation,
    /// Sequence-dependent machine setup reservation.
    Setup,
}

/// Stable profile key for one scheduled operation or setup reservation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtilityConsumerKey {
    /// Consumer class.
    pub kind: UtilityConsumerKind,
    /// Operation ID, or setup next-operation ID.
    pub id: String,
}

/// One declared time-invariant demand rate for a consumer and metric.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtilityRate {
    /// Resource metric.
    pub metric: UtilityMetric,
    /// Demand per tick; None means the demand is unknown, not zero.
    pub units_per_tick: Option<u64>,
}

/// A measured or estimated resource profile for one scheduled consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtilityConsumerProfile {
    /// Exact operation/setup profile key.
    pub consumer: UtilityConsumerKey,
    /// True only when every relevant utility demand for this task is declared.
    /// If complete, omitted metrics are explicitly zero for this task.
    pub complete: bool,
    /// Declared metric demand rates.
    pub rates: Vec<UtilityRate>,
}

/// Capacity for one utility metric across a half-open time interval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtilityEnvelope {
    /// Utility metric.
    pub metric: UtilityMetric,
    /// Half-open capacity interval [start_tick, end_tick).
    pub interval: crate::Interval,
    /// Known capacity per tick; None is an explicit unknown meter/capacity.
    pub capacity_units_per_tick: Option<u64>,
}

/// Declared utility model for one scheduling horizon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtilityModel {
    /// Name of one integer tick, for example minute or 15-minute-slot.
    pub tick_unit: String,
    /// Complete/partial consumer profiles. Every scheduled consumer needs one.
    pub profiles: Vec<UtilityConsumerProfile>,
    /// Time-varying envelopes for each utility metric.
    pub envelopes: Vec<UtilityEnvelope>,
}

/// One deterministic utility-envelope finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtilityFinding {
    /// Stable finding code.
    pub code: &'static str,
    /// Operation ID, next-operation ID for setup, or metric label.
    pub entity_id: String,
    /// Human-readable explanation.
    pub message: String,
}

/// Result of utility feasibility validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtilityReport {
    /// True only when no unknown, missing or violated utility constraint exists.
    pub valid: bool,
    /// Problem identity.
    pub problem_name: String,
    /// Caller-declared time unit.
    pub tick_unit: String,
    /// Number of scheduled production operations inspected.
    pub operation_consumers: usize,
    /// Number of setup reservations inspected.
    pub setup_consumers: usize,
    /// Deterministically ordered findings.
    pub findings: Vec<UtilityFinding>,
}

#[derive(Clone, Debug)]
struct ActiveConsumer {
    key: UtilityConsumerKey,
    start_tick: u64,
    finish_tick: u64,
}

fn add(
    findings: &mut Vec<UtilityFinding>,
    code: &'static str,
    entity_id: impl Into<String>,
    message: impl Into<String>,
) {
    findings.push(UtilityFinding {
        code,
        entity_id: entity_id.into(),
        message: message.into(),
    });
}

fn metric_name(metric: UtilityMetric) -> &'static str {
    match metric {
        UtilityMetric::ElectricalPowerW => "electrical_power_w",
        UtilityMetric::SteamFlowGramsPerTick => "steam_flow_grams_per_tick",
        UtilityMetric::WaterFlowMlPerTick => "water_flow_ml_per_tick",
        UtilityMetric::WastewaterFlowMlPerTick => "wastewater_flow_ml_per_tick",
    }
}

/// Validate scheduled operation/setup utility demand against time-varying
/// resource envelopes. Any missing profile, incomplete demand, unknown
/// capacity or uncovered positive demand fails closed.
///
/// This validator does not estimate physical demand. Profiles and envelopes
/// must be grounded in declared machine data, calibrated meters or explicitly
/// labelled engineering estimates before a production feasibility claim.
pub fn validate_utilities(
    problem: &Problem,
    schedule: &Schedule,
    model: &UtilityModel,
) -> UtilityReport {
    let mut findings = Vec::new();
    if model.tick_unit.trim().is_empty() {
        add(
            &mut findings,
            "UTILITY_TICK_UNIT_MISSING",
            &problem.name,
            "utility model must declare the meaning of one time tick",
        );
    }

    let mut operations = BTreeMap::<&str, &crate::Operation>::new();
    for operation in &problem.operations {
        operations.entry(operation.id.as_str()).or_insert(operation);
    }

    let mut consumers = Vec::<ActiveConsumer>::new();
    for assignment in &schedule.assignments {
        if assignment.start_tick >= assignment.finish_tick {
            add(
                &mut findings,
                "UTILITY_INVALID_OPERATION_INTERVAL",
                &assignment.operation_id,
                "utility validation requires a positive half-open operation interval",
            );
            continue;
        }
        if !operations.contains_key(assignment.operation_id.as_str()) {
            add(
                &mut findings,
                "UTILITY_UNKNOWN_OPERATION",
                &assignment.operation_id,
                "utility assignment references an operation absent from the frozen problem",
            );
            continue;
        }
        consumers.push(ActiveConsumer {
            key: UtilityConsumerKey {
                kind: UtilityConsumerKind::Operation,
                id: assignment.operation_id.clone(),
            },
            start_tick: assignment.start_tick,
            finish_tick: assignment.finish_tick,
        });
    }

    for reservation in &schedule.setup_reservations {
        if reservation.start_tick >= reservation.finish_tick {
            add(
                &mut findings,
                "UTILITY_INVALID_SETUP_INTERVAL",
                &reservation.next_operation_id,
                "utility validation requires a positive half-open setup interval",
            );
            continue;
        }
        consumers.push(ActiveConsumer {
            key: UtilityConsumerKey {
                kind: UtilityConsumerKind::Setup,
                id: reservation.next_operation_id.clone(),
            },
            start_tick: reservation.start_tick,
            finish_tick: reservation.finish_tick,
        });
    }

    let mut profiles = BTreeMap::<UtilityConsumerKey, &UtilityConsumerProfile>::new();
    for profile in &model.profiles {
        if profiles.insert(profile.consumer.clone(), profile).is_some() {
            add(
                &mut findings,
                "UTILITY_DUPLICATE_CONSUMER_PROFILE",
                &profile.consumer.id,
                "consumer must have exactly one utility profile",
            );
        }
        let mut metrics = BTreeSet::new();
        for rate in &profile.rates {
            if !metrics.insert(rate.metric) {
                add(
                    &mut findings,
                    "UTILITY_DUPLICATE_METRIC_DEMAND",
                    &profile.consumer.id,
                    format!("duplicate demand for {}", metric_name(rate.metric)),
                );
            }
        }
    }

    let active_keys: BTreeSet<UtilityConsumerKey> =
        consumers.iter().map(|consumer| consumer.key.clone()).collect();
    for profile in &model.profiles {
        if !active_keys.contains(&profile.consumer) {
            add(
                &mut findings,
                "UTILITY_ORPHAN_CONSUMER_PROFILE",
                &profile.consumer.id,
                "utility profile does not correspond to an active scheduled consumer",
            );
        }
    }

    let mut envelope_by_metric = BTreeMap::<UtilityMetric, Vec<&UtilityEnvelope>>::new();
    for envelope in &model.envelopes {
        if envelope.interval.start_tick >= envelope.interval.end_tick {
            add(
                &mut findings,
                "UTILITY_INVALID_ENVELOPE_INTERVAL",
                metric_name(envelope.metric),
                "utility capacity interval is empty or reversed",
            );
            continue;
        }
        envelope_by_metric
            .entry(envelope.metric)
            .or_default()
            .push(envelope);
    }

    for (metric, envelopes) in &mut envelope_by_metric {
        envelopes.sort_by_key(|envelope| {
            (envelope.interval.start_tick, envelope.interval.end_tick)
        });
        for pair in envelopes.windows(2) {
            if pair[1].interval.start_tick < pair[0].interval.end_tick {
                add(
                    &mut findings,
                    "UTILITY_ENVELOPE_OVERLAP",
                    metric_name(*metric),
                    "capacity envelopes for one metric overlap; normalize before qualification",
                );
            }
        }
    }

    let mut boundaries = BTreeSet::<u64>::new();
    for consumer in &consumers {
        boundaries.insert(consumer.start_tick);
        boundaries.insert(consumer.finish_tick);
    }
    for envelopes in envelope_by_metric.values() {
        for envelope in envelopes {
            boundaries.insert(envelope.interval.start_tick);
            boundaries.insert(envelope.interval.end_tick);
        }
    }
    let boundaries: Vec<u64> = boundaries.into_iter().collect();

    let mut demand_unknown = BTreeSet::<UtilityConsumerKey>::new();
    for consumer in &consumers {
        if !profiles.contains_key(&consumer.key) {
            if demand_unknown.insert(consumer.key.clone()) {
                add(
                    &mut findings,
                    "UTILITY_PROFILE_MISSING",
                    &consumer.key.id,
                    "scheduled consumer has no declared utility profile; zero demand is not assumed",
                );
            }
        } else if profiles
            .get(&consumer.key)
            .is_some_and(|profile| !profile.complete)
            && demand_unknown.insert(consumer.key.clone())
        {
            add(
                &mut findings,
                "UTILITY_DEMAND_INCOMPLETE",
                &consumer.key.id,
                "utility profile is partial; unmeasured demand blocks feasibility qualification",
            );
        }
    }

    for window in boundaries.windows(2) {
        let start = window[0];
        let end = window[1];
        if start >= end {
            continue;
        }

        let active: Vec<&ActiveConsumer> = consumers
            .iter()
            .filter(|consumer| consumer.start_tick <= start && end <= consumer.finish_tick)
            .collect();
        if active.is_empty() {
            continue;
        }

        let mut total_demand = BTreeMap::<UtilityMetric, u64>::new();
        for consumer in &active {
            let Some(profile) = profiles.get(&consumer.key).copied() else {
                continue;
            };
            if !profile.complete {
                continue;
            }
            for rate in &profile.rates {
                let Some(units) = rate.units_per_tick else {
                    add(
                        &mut findings,
                        "UTILITY_DEMAND_UNKNOWN",
                        &consumer.key.id,
                        format!(
                            "demand for {} is unknown during [{start}, {end})",
                            metric_name(rate.metric)
                        ),
                    );
                    continue;
                };
                let total = total_demand.entry(rate.metric).or_default();
                match total.checked_add(units) {
                    Some(sum) => *total = sum,
                    None => add(
                        &mut findings,
                        "UTILITY_DEMAND_OVERFLOW",
                        metric_name(rate.metric),
                        "concurrent demand sum overflows u64",
                    ),
                }
            }
        }

        for (metric, demand) in total_demand {
            if demand == 0 {
                continue;
            }
            let covering: Vec<&UtilityEnvelope> = envelope_by_metric
                .get(&metric)
                .into_iter()
                .flatten()
                .filter(|envelope| {
                    envelope.interval.start_tick <= start && end <= envelope.interval.end_tick
                })
                .copied()
                .collect();
            if covering.is_empty() {
                add(
                    &mut findings,
                    "UTILITY_CAPACITY_MISSING",
                    metric_name(metric),
                    format!(
                        "positive demand {demand} has no capacity envelope during [{start}, {end})"
                    ),
                );
                continue;
            }
            if covering.len() > 1 {
                add(
                    &mut findings,
                    "UTILITY_ENVELOPE_OVERLAP",
                    metric_name(metric),
                    format!("multiple capacity envelopes cover [{start}, {end})"),
                );
                continue;
            }
            let Some(capacity) = covering[0].capacity_units_per_tick else {
                add(
                    &mut findings,
                    "UTILITY_CAPACITY_UNKNOWN",
                    metric_name(metric),
                    format!("capacity is unknown during [{start}, {end})"),
                );
                continue;
            };
            if demand > capacity {
                add(
                    &mut findings,
                    "UTILITY_CAPACITY_EXCEEDED",
                    metric_name(metric),
                    format!(
                        "concurrent demand {demand} exceeds capacity {capacity} during [{start}, {end})"
                    ),
                );
            }
        }
    }

    findings.sort_by(|left, right| {
        (&left.entity_id, left.code, &left.message).cmp(&(
            &right.entity_id,
            right.code,
            &right.message,
        ))
    });
    let operation_consumers = consumers
        .iter()
        .filter(|consumer| consumer.key.kind == UtilityConsumerKind::Operation)
        .count();
    let setup_consumers = consumers
        .iter()
        .filter(|consumer| consumer.key.kind == UtilityConsumerKind::Setup)
        .count();
    UtilityReport {
        valid: findings.is_empty(),
        problem_name: problem.name.clone(),
        tick_unit: model.tick_unit.clone(),
        operation_consumers,
        setup_consumers,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assignment, Interval, Operation, Problem, Schedule, SetupReservation};

    fn problem() -> Problem {
        Problem {
            name: "utility-test".to_owned(),
            machines: Vec::new(),
            operators: Vec::new(),
            operations: vec![
                Operation {
                    id: "op-a".to_owned(),
                    job_id: "job-a".to_owned(),
                    stage_index: 0,
                    duration_ticks: 5,
                    release_tick: 0,
                    due_tick: None,
                    required_capability: "cut".to_owned(),
                    required_skill: "operator".to_owned(),
                    required_certification: None,
                    product_family: "woven".to_owned(),
                },
                Operation {
                    id: "op-b".to_owned(),
                    job_id: "job-b".to_owned(),
                    stage_index: 0,
                    duration_ticks: 5,
                    release_tick: 0,
                    due_tick: None,
                    required_capability: "cut".to_owned(),
                    required_skill: "operator".to_owned(),
                    required_certification: None,
                    product_family: "woven".to_owned(),
                },
            ],
            precedences: Vec::new(),
        }
    }

    fn schedule() -> Schedule {
        Schedule {
            assignments: vec![Assignment {
                operation_id: "op-a".to_owned(),
                machine_id: "cut-1".to_owned(),
                operator_id: "worker-1".to_owned(),
                start_tick: 0,
                finish_tick: 5,
            }],
            setup_reservations: Vec::new(),
        }
    }

    fn profile(
        kind: UtilityConsumerKind,
        id: &str,
        complete: bool,
        rates: Vec<UtilityRate>,
    ) -> UtilityConsumerProfile {
        UtilityConsumerProfile {
            consumer: UtilityConsumerKey {
                kind,
                id: id.to_owned(),
            },
            complete,
            rates,
        }
    }

    fn rate(metric: UtilityMetric, units: Option<u64>) -> UtilityRate {
        UtilityRate {
            metric,
            units_per_tick: units,
        }
    }

    fn envelope(
        metric: UtilityMetric,
        start: u64,
        end: u64,
        capacity: Option<u64>,
    ) -> UtilityEnvelope {
        UtilityEnvelope {
            metric,
            interval: Interval {
                start_tick: start,
                end_tick: end,
            },
            capacity_units_per_tick: capacity,
        }
    }

    fn model(
        profiles: Vec<UtilityConsumerProfile>,
        envelopes: Vec<UtilityEnvelope>,
    ) -> UtilityModel {
        UtilityModel {
            tick_unit: "minute".to_owned(),
            profiles,
            envelopes,
        }
    }

    fn has(report: &UtilityReport, code: &str) -> bool {
        report.findings.iter().any(|finding| finding.code == code)
    }

    #[test]
    fn known_operation_demand_within_envelope_passes() {
        let report = validate_utilities(
            &problem(),
            &schedule(),
            &model(
                vec![profile(
                    UtilityConsumerKind::Operation,
                    "op-a",
                    true,
                    vec![rate(UtilityMetric::ElectricalPowerW, Some(800))],
                )],
                vec![envelope(UtilityMetric::ElectricalPowerW, 0, 5, Some(1_000))],
            ),
        );
        assert!(report.valid, "{:#?}", report.findings);
    }

    #[test]
    fn concurrent_operation_demands_cannot_exceed_peak_capacity() {
        let mut schedule = schedule();
        schedule.assignments.push(Assignment {
            operation_id: "op-b".to_owned(),
            machine_id: "cut-2".to_owned(),
            operator_id: "worker-2".to_owned(),
            start_tick: 2,
            finish_tick: 7,
        });
        let report = validate_utilities(
            &problem(),
            &schedule,
            &model(
                vec![
                    profile(
                        UtilityConsumerKind::Operation,
                        "op-a",
                        true,
                        vec![rate(UtilityMetric::ElectricalPowerW, Some(700))],
                    ),
                    profile(
                        UtilityConsumerKind::Operation,
                        "op-b",
                        true,
                        vec![rate(UtilityMetric::ElectricalPowerW, Some(700))],
                    ),
                ],
                vec![
                    envelope(UtilityMetric::ElectricalPowerW, 0, 2, Some(2_000)),
                    envelope(UtilityMetric::ElectricalPowerW, 2, 7, Some(1_000)),
                ],
            ),
        );
        assert!(has(&report, "UTILITY_CAPACITY_EXCEEDED"));
    }

    #[test]
    fn missing_or_incomplete_profiles_never_mean_zero_utility_use() {
        let missing = validate_utilities(&problem(), &schedule(), &model(Vec::new(), Vec::new()));
        assert!(!missing.valid);
        assert!(has(&missing, "UTILITY_PROFILE_MISSING"));

        let incomplete = validate_utilities(
            &problem(),
            &schedule(),
            &model(
                vec![profile(UtilityConsumerKind::Operation, "op-a", false, Vec::new())],
                Vec::new(),
            ),
        );
        assert!(has(&incomplete, "UTILITY_DEMAND_INCOMPLETE"));
    }

    #[test]
    fn unknown_demand_rate_and_unknown_capacity_block_qualification() {
        let unknown_demand = validate_utilities(
            &problem(),
            &schedule(),
            &model(
                vec![profile(
                    UtilityConsumerKind::Operation,
                    "op-a",
                    true,
                    vec![rate(UtilityMetric::SteamFlowGramsPerTick, None)],
                )],
                vec![envelope(UtilityMetric::SteamFlowGramsPerTick, 0, 5, Some(100))],
            ),
        );
        assert!(has(&unknown_demand, "UTILITY_DEMAND_UNKNOWN"));

        let unknown_capacity = validate_utilities(
            &problem(),
            &schedule(),
            &model(
                vec![profile(
                    UtilityConsumerKind::Operation,
                    "op-a",
                    true,
                    vec![rate(UtilityMetric::WaterFlowMlPerTick, Some(100))],
                )],
                vec![envelope(UtilityMetric::WaterFlowMlPerTick, 0, 5, None)],
            ),
        );
        assert!(has(&unknown_capacity, "UTILITY_CAPACITY_UNKNOWN"));
    }

    #[test]
    fn positive_demand_without_time_coverage_fails_closed() {
        let report = validate_utilities(
            &problem(),
            &schedule(),
            &model(
                vec![profile(
                    UtilityConsumerKind::Operation,
                    "op-a",
                    true,
                    vec![rate(UtilityMetric::WastewaterFlowMlPerTick, Some(5))],
                )],
                vec![envelope(
                    UtilityMetric::WastewaterFlowMlPerTick,
                    0,
                    2,
                    Some(10),
                )],
            ),
        );
        assert!(has(&report, "UTILITY_CAPACITY_MISSING"));
    }

    #[test]
    fn setup_consumption_is_counted_as_a_separate_concurrent_consumer() {
        let mut schedule = schedule();
        schedule.setup_reservations.push(SetupReservation {
            machine_id: "cut-1".to_owned(),
            from_family: "woven".to_owned(),
            to_family: "cotton".to_owned(),
            predecessor_operation_id: None,
            next_operation_id: "op-a".to_owned(),
            operator_id: "setup-worker".to_owned(),
            start_tick: 0,
            finish_tick: 2,
        });
        let report = validate_utilities(
            &problem(),
            &schedule,
            &model(
                vec![
                    profile(
                        UtilityConsumerKind::Operation,
                        "op-a",
                        true,
                        vec![rate(UtilityMetric::SteamFlowGramsPerTick, Some(80))],
                    ),
                    profile(
                        UtilityConsumerKind::Setup,
                        "op-a",
                        true,
                        vec![rate(UtilityMetric::SteamFlowGramsPerTick, Some(40))],
                    ),
                ],
                vec![envelope(UtilityMetric::SteamFlowGramsPerTick, 0, 5, Some(100))],
            ),
        );
        assert_eq!(report.setup_consumers, 1);
        assert!(has(&report, "UTILITY_CAPACITY_EXCEEDED"));
    }

    #[test]
    fn overlapping_envelopes_and_duplicate_profile_metrics_are_rejected() {
        let report = validate_utilities(
            &problem(),
            &schedule(),
            &model(
                vec![profile(
                    UtilityConsumerKind::Operation,
                    "op-a",
                    true,
                    vec![
                        rate(UtilityMetric::ElectricalPowerW, Some(1)),
                        rate(UtilityMetric::ElectricalPowerW, Some(1)),
                    ],
                )],
                vec![
                    envelope(UtilityMetric::ElectricalPowerW, 0, 5, Some(10)),
                    envelope(UtilityMetric::ElectricalPowerW, 2, 7, Some(10)),
                ],
            ),
        );
        assert!(has(&report, "UTILITY_DUPLICATE_METRIC_DEMAND"));
        assert!(has(&report, "UTILITY_ENVELOPE_OVERLAP"));
    }

    fn utility_case_block<'a>(corpus: &'a str, case_id: &str) -> Option<&'a str> {
        let needle = format!("\"case_id\": \"{case_id}\"");
        let id_position = corpus.find(&needle)?;
        let object_start = corpus[..id_position].rfind('{')?;
        let object_end = id_position + corpus[id_position..].find('}')? + 1;
        corpus.get(object_start..object_end)
    }

    fn exercise_utility_case(case_id: &str) -> (&'static str, Option<&'static str>) {
        match case_id {
            "UTIL-001-KNOWN-LOAD-UNDER-CAPACITY" => {
                known_operation_demand_within_envelope_passes();
                ("valid", None)
            }
            "UTIL-002-CONCURRENT-PEAK-EXCEEDED" => {
                concurrent_operation_demands_cannot_exceed_peak_capacity();
                ("invalid", Some("UTILITY_CAPACITY_EXCEEDED"))
            }
            "UTIL-003-MISSING-UTILITY-PROFILE" => {
                missing_or_incomplete_profiles_never_mean_zero_utility_use();
                ("invalid", Some("UTILITY_PROFILE_MISSING"))
            }
            "UTIL-004-UNKNOWN-CAPACITY" => {
                unknown_demand_rate_and_unknown_capacity_block_qualification();
                ("invalid", Some("UTILITY_CAPACITY_UNKNOWN"))
            }
            "UTIL-005-ENVELOPE-GAP" => {
                positive_demand_without_time_coverage_fails_closed();
                ("invalid", Some("UTILITY_CAPACITY_MISSING"))
            }
            "UTIL-006-SETUP-PEAK-COUNTED" => {
                setup_consumption_is_counted_as_a_separate_concurrent_consumer();
                ("invalid", Some("UTILITY_CAPACITY_EXCEEDED"))
            }
            "UTIL-007-OVERLAP-AND-DUPLICATE-METRIC" => {
                overlapping_envelopes_and_duplicate_profile_metrics_are_rejected();
                ("invalid", Some("UTILITY_ENVELOPE_OVERLAP"))
            }
            "UTIL-008-HALF-OPEN-TOUCHING-INTERVALS" => {
                touching_half_open_operation_intervals_do_not_overlap();
                ("valid", None)
            }
            other => panic!("unknown textile utility corpus case: {other}"),
        }
    }

    #[test]
    fn synthetic_utility_corpus_executes_every_declared_case() {
        const CORPUS: &str = include_str!(
            "../../../research/textile-multistage-factory/utility_cases_v0.1.json"
        );
        const SCHEMA: &str = include_str!(
            "../../../research/textile-multistage-factory/utility_cases_v0.1.schema.json"
        );

        assert!(CORPUS.contains("\"corpus_id\": \"TEXTILE-UTILITY-CASES-001\""));
        assert!(CORPUS.contains("\"fixture_class\": \"synthetic\""));
        assert!(SCHEMA.contains("\"minItems\": 8"));
        assert!(SCHEMA.contains("\"maxItems\": 8"));
        assert_eq!(CORPUS.matches("\"case_id\":").count(), 8);

        let cases = [
            ("UTIL-001-KNOWN-LOAD-UNDER-CAPACITY", "valid", None),
            ("UTIL-002-CONCURRENT-PEAK-EXCEEDED", "invalid", Some("UTILITY_CAPACITY_EXCEEDED")),
            ("UTIL-003-MISSING-UTILITY-PROFILE", "invalid", Some("UTILITY_PROFILE_MISSING")),
            ("UTIL-004-UNKNOWN-CAPACITY", "invalid", Some("UTILITY_CAPACITY_UNKNOWN")),
            ("UTIL-005-ENVELOPE-GAP", "invalid", Some("UTILITY_CAPACITY_MISSING")),
            ("UTIL-006-SETUP-PEAK-COUNTED", "invalid", Some("UTILITY_CAPACITY_EXCEEDED")),
            ("UTIL-007-OVERLAP-AND-DUPLICATE-METRIC", "invalid", Some("UTILITY_ENVELOPE_OVERLAP")),
            ("UTIL-008-HALF-OPEN-TOUCHING-INTERVALS", "valid", None),
        ];

        let mut seen = BTreeSet::new();
        for (case_id, expected_validity, expected_finding) in cases {
            assert!(seen.insert(case_id), "duplicate utility case {case_id}");
            let block = utility_case_block(CORPUS, case_id)
                .unwrap_or_else(|| panic!("utility corpus case missing: {case_id}"));
            let expected_bool = expected_validity == "valid";
            assert!(
                block.contains(&format!("\"expected_valid\": {expected_bool}")),
                "case {case_id} expected_valid differs from Rust regression mapping"
            );
            match expected_finding {
                Some(code) => assert!(
                    block.contains(&format!("\"expected_finding\": \"{code}\"")),
                    "case {case_id} expected finding differs from Rust regression mapping"
                ),
                None => assert!(
                    block.contains("\"expected_finding\": null"),
                    "case {case_id} must have no expected finding"
                ),
            }
            let (actual_validity, actual_finding) = exercise_utility_case(case_id);
            assert_eq!(actual_validity, expected_validity, "case {case_id} validity");
            assert_eq!(actual_finding, expected_finding, "case {case_id} finding");
        }
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn touching_half_open_operation_intervals_do_not_overlap() {
        let mut schedule = schedule();
        schedule.assignments[0].finish_tick = 5;
        schedule.assignments.push(Assignment {
            operation_id: "op-b".to_owned(),
            machine_id: "cut-2".to_owned(),
            operator_id: "worker-2".to_owned(),
            start_tick: 5,
            finish_tick: 10,
        });
        let report = validate_utilities(
            &problem(),
            &schedule,
            &model(
                vec![
                    profile(
                        UtilityConsumerKind::Operation,
                        "op-a",
                        true,
                        vec![rate(UtilityMetric::ElectricalPowerW, Some(700))],
                    ),
                    profile(
                        UtilityConsumerKind::Operation,
                        "op-b",
                        true,
                        vec![rate(UtilityMetric::ElectricalPowerW, Some(700))],
                    ),
                ],
                vec![envelope(UtilityMetric::ElectricalPowerW, 0, 10, Some(700))],
            ),
        );
        assert!(report.valid, "{:#?}", report.findings);
    }
}
