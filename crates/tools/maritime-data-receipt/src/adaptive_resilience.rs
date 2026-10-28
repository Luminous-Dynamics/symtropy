// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Pure reducer for the v0.1 adaptive-resilience *contract fixture*.
//!
//! It computes toy service-trajectory metrics only. It does not simulate
//! maritime operations, estimate probabilities, propagate dependency failures,
//! or grant physical/operational authority.

use serde::Serialize;
use std::collections::BTreeSet;
use std::env;
use std::fs;

use super::JsonValue;

const MAX_FIXTURE_BYTES: usize = 1_000_000;
const ROOT_KEYS: &[&str] = &[
    "schema_id",
    "schema_version",
    "fixture_kind",
    "claim_ceiling",
    "model_admission",
    "authority",
    "time_axis",
    "metrics_policy",
    "service_classes",
    "dependency_nodes",
    "dependency_edges",
    "scenario_cases",
    "conditional_policies",
    "synthetic_reference_traces",
    "unquantified_cases",
    "fixture_warnings",
];
const AUTHORITY_KEYS: &[&str] = &[
    "navigation_authorized",
    "physical_truth_claim_authorized",
    "berth_or_slot_authorized",
    "engineering_admission",
    "physical_execution_authorized",
    "external_approval_claim_authorized",
];

#[derive(Debug, Serialize, PartialEq, Eq)]
struct TrajectoryMetric {
    trace_id: String,
    case_id: String,
    service_id: String,
    unit: String,
    integrated_unmet_service_units: u64,
    recovery_time_hours: Option<u64>,
    recovery_status: String,
    intervals_evaluated: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct AdaptiveResilienceReport {
    status: &'static str,
    schema_id: &'static str,
    schema_version: &'static str,
    fixture_sha256: String,
    fixture_byte_length: usize,
    source_commit_binding: String,
    trajectory_metrics: Vec<TrajectoryMetric>,
    metrics_checked: usize,
    horizon_censored_traces: usize,
    dependency_nodes_declared: usize,
    dependency_edges_declared: usize,
    scenario_cases_declared: usize,
    conditional_policy_rules_declared: usize,
    quantitative_dependency_propagation_performed: bool,
    physical_simulation_performed: bool,
    engineering_admission: &'static str,
    navigation_authorized: bool,
    physical_truth_claim_authorized: bool,
    berth_or_slot_authorized: bool,
    physical_execution_authorized: bool,
    claim_ceiling: &'static str,
}

fn field_text<'a>(root: &'a JsonValue, key: &str, path: &str) -> Result<&'a str, String> {
    super::text(super::field(root, key, path)?, &format!("{path}.{key}"))
}

fn path_text<'a>(value: &'a JsonValue, path: &str) -> Result<&'a str, String> {
    super::text(value, path)
}

fn ratio_number(value: &JsonValue, path: &str) -> Result<f64, String> {
    let parsed = match value {
        JsonValue::Number(number) => number.as_f64(),
        _ => None,
    }
    .ok_or_else(|| format!("{path}: expected a finite JSON number"))?;
    if !parsed.is_finite() {
        return Err(format!("{path}: expected a finite JSON number"));
    }
    Ok(parsed)
}

fn nonempty(value: &JsonValue, path: &str) -> Result<(), String> {
    let actual = path_text(value, path)?;
    if actual.trim().is_empty() {
        return Err(format!("{path}: empty string is forbidden"));
    }
    Ok(())
}

fn string_array(value: &JsonValue, path: &str) -> Result<(), String> {
    for (index, item) in super::array(value, path)?.iter().enumerate() {
        nonempty(item, &format!("{path}[{index}]"))?;
    }
    Ok(())
}

fn validate_authority(root: &JsonValue) -> Result<(), String> {
    super::expect_text(
        super::field(root, "schema_id", "fixture")?,
        "fixture.schema_id",
        "luminous.symtropy.maritime-adaptive-resilience",
    )?;
    super::expect_text(
        super::field(root, "schema_version", "fixture")?,
        "fixture.schema_version",
        "0.1.0",
    )?;
    super::expect_text(
        super::field(root, "fixture_kind", "fixture")?,
        "fixture.fixture_kind",
        "synthetic_contract_example",
    )?;
    super::expect_text(
        super::field(root, "claim_ceiling", "fixture")?,
        "fixture.claim_ceiling",
        "CONTRACT_SHAPE_ONLY_NOT_PHYSICAL_SIMULATION",
    )?;
    super::expect_text(
        super::field(root, "model_admission", "fixture")?,
        "fixture.model_admission",
        "NotAdmitted",
    )?;

    let authority = super::field(root, "authority", "fixture")?;
    super::exact_keys(authority, AUTHORITY_KEYS, "fixture.authority")?;
    for key in AUTHORITY_KEYS {
        if super::bool_value(
            super::field(authority, key, "fixture.authority")?,
            &format!("fixture.authority.{key}"),
        )? {
            return Err(format!(
                "fixture.authority.{key}: authority escalation is forbidden"
            ));
        }
    }
    Ok(())
}

fn validate_time_axis(root: &JsonValue) -> Result<(u64, u64, usize), String> {
    let axis = super::field(root, "time_axis", "fixture")?;
    super::exact_keys(
        axis,
        &[
            "unit",
            "period_semantics",
            "duration_hours",
            "horizon_hours",
            "time_offsets",
        ],
        "fixture.time_axis",
    )?;
    super::expect_text(
        super::field(axis, "unit", "fixture.time_axis")?,
        "fixture.time_axis.unit",
        "hours_from_t0",
    )?;
    super::expect_text(
        super::field(axis, "period_semantics", "fixture.time_axis")?,
        "fixture.time_axis.period_semantics",
        "Each series value is the mean rate over [hour_offset, hour_offset + duration_hours).",
    )?;
    let duration = super::integer(
        super::field(axis, "duration_hours", "fixture.time_axis")?,
        "fixture.time_axis.duration_hours",
    )?;
    let horizon = super::integer(
        super::field(axis, "horizon_hours", "fixture.time_axis")?,
        "fixture.time_axis.horizon_hours",
    )?;
    if duration == 0 || horizon == 0 || horizon % duration != 0 {
        return Err(
            "time_axis: duration and horizon must be positive and evenly divisible".to_owned(),
        );
    }
    let offsets = super::array(
        super::field(axis, "time_offsets", "fixture.time_axis")?,
        "fixture.time_axis.time_offsets",
    )?;
    let expected_count = usize::try_from(horizon / duration)
        .map_err(|_| "time_axis: interval count overflows platform size".to_owned())?;
    if offsets.len() != expected_count {
        return Err(format!(
            "time_axis.time_offsets: expected {expected_count} interval starts, got {}",
            offsets.len()
        ));
    }
    for (index, offset) in offsets.iter().enumerate() {
        let actual = super::integer(offset, &format!("time_axis.time_offsets[{index}]"))?;
        let expected = u64::try_from(index)
            .ok()
            .and_then(|n| n.checked_mul(duration))
            .ok_or_else(|| "time_axis: expected offset overflow".to_owned())?;
        if actual != expected {
            return Err(format!(
                "time_axis.time_offsets[{index}]: expected {expected}, got {actual}"
            ));
        }
    }
    Ok((duration, horizon, expected_count))
}

#[derive(Debug)]
struct ServiceClass {
    unit: String,
    demand_rate: u64,
    minimum_service_ratio: f64,
}

fn validate_service_classes(
    root: &JsonValue,
) -> Result<std::collections::BTreeMap<String, ServiceClass>, String> {
    let values = super::array(
        super::field(root, "service_classes", "fixture")?,
        "fixture.service_classes",
    )?;
    if values.is_empty() {
        return Err("fixture.service_classes must not be empty".to_owned());
    }
    let mut classes = std::collections::BTreeMap::new();
    for (index, item) in values.iter().enumerate() {
        let path = format!("fixture.service_classes[{index}]");
        super::exact_keys(
            item,
            &[
                "service_id",
                "unit",
                "planned_demand_rate",
                "minimum_service_ratio",
                "priority_label",
            ],
            &path,
        )?;
        let id = field_text(item, "service_id", &path)?.to_owned();
        if id.trim().is_empty() || classes.contains_key(&id) {
            return Err(format!(
                "{path}.service_id: empty or duplicate service identifier"
            ));
        }
        let unit = field_text(item, "unit", &path)?.to_owned();
        if unit.trim().is_empty() {
            return Err(format!("{path}.unit: empty unit"));
        }
        let demand = super::integer(
            super::field(item, "planned_demand_rate", &path)?,
            &format!("{path}.planned_demand_rate"),
        )?;
        if demand == 0 {
            return Err(format!(
                "{path}.planned_demand_rate: zero demand makes service ratio undefined"
            ));
        }
        let floor = ratio_number(
            super::field(item, "minimum_service_ratio", &path)?,
            &format!("{path}.minimum_service_ratio"),
        )?;
        if !(floor > 0.0 && floor <= 1.0) {
            return Err(format!(
                "{path}.minimum_service_ratio: expected value in (0, 1]"
            ));
        }
        nonempty(
            super::field(item, "priority_label", &path)?,
            &format!("{path}.priority_label"),
        )?;
        classes.insert(
            id,
            ServiceClass {
                unit,
                demand_rate: demand,
                minimum_service_ratio: floor,
            },
        );
    }
    Ok(classes)
}

fn validate_metrics_policy(root: &JsonValue) -> Result<(f64, usize), String> {
    let policy = super::field(root, "metrics_policy", "fixture")?;
    super::exact_keys(
        policy,
        &[
            "probability_weights_used",
            "aggregate_score_used",
            "service_ratio",
            "integrated_service_loss",
            "recovery_rule",
        ],
        "fixture.metrics_policy",
    )?;
    if super::bool_value(
        super::field(policy, "probability_weights_used", "fixture.metrics_policy")?,
        "metrics_policy.probability_weights_used",
    )? {
        return Err(
            "metrics_policy: ungrounded probability weights are forbidden in v0.1".to_owned(),
        );
    }
    if super::bool_value(
        super::field(policy, "aggregate_score_used", "fixture.metrics_policy")?,
        "metrics_policy.aggregate_score_used",
    )? {
        return Err("metrics_policy: weighted aggregate score is forbidden in v0.1".to_owned());
    }
    super::expect_text(
        super::field(policy, "service_ratio", "fixture.metrics_policy")?,
        "metrics_policy.service_ratio",
        "served_rate / planned_demand_rate when planned_demand_rate > 0; otherwise NOT_DEFINED",
    )?;
    super::expect_text(
        super::field(policy, "integrated_service_loss", "fixture.metrics_policy")?,
        "metrics_policy.integrated_service_loss",
        "sum(max(planned_demand_rate - served_rate, 0) * duration_hours) per service class",
    )?;
    let recovery = super::field(policy, "recovery_rule", "fixture.metrics_policy")?;
    super::exact_keys(
        recovery,
        &[
            "minimum_service_ratio",
            "consecutive_intervals_required",
            "rule",
            "when_not_observed",
        ],
        "fixture.metrics_policy.recovery_rule",
    )?;
    let floor = ratio_number(
        super::field(recovery, "minimum_service_ratio", "recovery_rule")?,
        "metrics_policy.recovery_rule.minimum_service_ratio",
    )?;
    if !(floor > 0.0 && floor <= 1.0) {
        return Err("recovery rule floor must be in (0, 1]".to_owned());
    }
    let consecutive = super::integer(
        super::field(recovery, "consecutive_intervals_required", "recovery_rule")?,
        "metrics_policy.recovery_rule.consecutive_intervals_required",
    )?;
    if consecutive == 0 {
        return Err("recovery rule must require at least one interval".to_owned());
    }
    let consecutive = usize::try_from(consecutive)
        .map_err(|_| "recovery interval count overflows platform size".to_owned())?;
    nonempty(
        super::field(recovery, "rule", "recovery_rule")?,
        "metrics_policy.recovery_rule.rule",
    )?;
    super::expect_text(
        super::field(recovery, "when_not_observed", "recovery_rule")?,
        "metrics_policy.recovery_rule.when_not_observed",
        "NOT_OBSERVED_WITHIN_HORIZON",
    )?;
    Ok((floor, consecutive))
}

fn validate_dependency_contract(root: &JsonValue) -> Result<(usize, usize), String> {
    let nodes = super::array(
        super::field(root, "dependency_nodes", "fixture")?,
        "fixture.dependency_nodes",
    )?;
    let edges = super::array(
        super::field(root, "dependency_edges", "fixture")?,
        "fixture.dependency_edges",
    )?;
    let mut ids = BTreeSet::new();
    for (index, node) in nodes.iter().enumerate() {
        let path = format!("fixture.dependency_nodes[{index}]");
        super::exact_keys(node, &["node_id", "node_kind", "status_semantics"], &path)?;
        let id = field_text(node, "node_id", &path)?;
        nonempty(
            super::field(node, "node_kind", &path)?,
            &format!("{path}.node_kind"),
        )?;
        super::expect_text(
            super::field(node, "status_semantics", &path)?,
            &format!("{path}.status_semantics"),
            "synthetic_declared_state_only",
        )?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("{path}.node_id: duplicate node ID"));
        }
    }
    if ids.is_empty() {
        return Err("dependency_nodes must not be empty".to_owned());
    }

    let mut edge_ids = BTreeSet::new();
    for (index, edge) in edges.iter().enumerate() {
        let path = format!("fixture.dependency_edges[{index}]");
        super::exact_keys(
            edge,
            &[
                "edge_id",
                "from_node",
                "to_node",
                "relation",
                "dependency_kind",
                "propagation_model",
                "evidence_status",
            ],
            &path,
        )?;
        let id = field_text(edge, "edge_id", &path)?;
        if !edge_ids.insert(id.to_owned()) {
            return Err(format!("{path}.edge_id: duplicate edge ID"));
        }
        for key in ["from_node", "to_node"] {
            let node = field_text(edge, key, &path)?;
            if !ids.contains(node) {
                return Err(format!("{path}.{key}: reference to unknown node {node:?}"));
            }
        }
        for key in ["relation", "dependency_kind"] {
            nonempty(super::field(edge, key, &path)?, &format!("{path}.{key}"))?;
        }
        super::expect_text(
            super::field(edge, "propagation_model", &path)?,
            &format!("{path}.propagation_model"),
            "not_declared",
        )?;
        super::expect_text(
            super::field(edge, "evidence_status", &path)?,
            &format!("{path}.evidence_status"),
            "synthetic",
        )?;
    }
    // This v0.1 tool deliberately validates graph shape but does not infer
    // downstream outcomes from these edges.
    Ok((nodes.len(), edges.len()))
}

fn validate_scenario_cases(
    root: &JsonValue,
    horizon: u64,
    node_ids: &BTreeSet<String>,
) -> Result<usize, String> {
    let cases = super::array(
        super::field(root, "scenario_cases", "fixture")?,
        "fixture.scenario_cases",
    )?;
    if cases.is_empty() {
        return Err("scenario_cases must not be empty".to_owned());
    }
    let mut ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let path = format!("fixture.scenario_cases[{index}]");
        super::exact_keys(
            case,
            &[
                "case_id",
                "required_case",
                "probability",
                "probability_status",
                "common_cause_group",
                "perturbations",
            ],
            &path,
        )?;
        let id = field_text(case, "case_id", &path)?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("{path}.case_id: duplicate scenario case"));
        }
        if !super::bool_value(
            super::field(case, "required_case", &path)?,
            &format!("{path}.required_case"),
        )? {
            return Err(format!(
                "{path}: v0.1 fixture cases must be required, not silently optional"
            ));
        }
        if !matches!(super::field(case, "probability", &path)?, JsonValue::Null) {
            return Err(format!(
                "{path}.probability: no probability distribution is admitted"
            ));
        }
        super::expect_text(
            super::field(case, "probability_status", &path)?,
            &format!("{path}.probability_status"),
            "not_assigned",
        )?;
        match super::field(case, "common_cause_group", &path)? {
            JsonValue::Null => {}
            JsonValue::String(group) if !group.trim().is_empty() => {}
            _ => {
                return Err(format!(
                    "{path}.common_cause_group: expected null or nonempty string"
                ));
            }
        }
        let perturbations = super::array(
            super::field(case, "perturbations", &path)?,
            &format!("{path}.perturbations"),
        )?;
        if perturbations.is_empty() {
            return Err(format!("{path}.perturbations must not be empty"));
        }
        for (pindex, p) in perturbations.iter().enumerate() {
            let ppath = format!("{path}.perturbations[{pindex}]");
            super::exact_keys(
                p,
                &[
                    "node_id",
                    "start_hour_inclusive",
                    "end_hour_exclusive",
                    "declared_state",
                    "state_basis",
                ],
                &ppath,
            )?;
            let node = field_text(p, "node_id", &ppath)?;
            if !node_ids.contains(node) {
                return Err(format!("{ppath}.node_id: unknown dependency node"));
            }
            let start = super::integer(
                super::field(p, "start_hour_inclusive", &ppath)?,
                &format!("{ppath}.start_hour_inclusive"),
            )?;
            let end = super::integer(
                super::field(p, "end_hour_exclusive", &ppath)?,
                &format!("{ppath}.end_hour_exclusive"),
            )?;
            if start >= end || end > horizon {
                return Err(format!("{ppath}: invalid perturbation interval"));
            }
            let state = field_text(p, "declared_state", &ppath)?;
            if !["unavailable", "unknown"].contains(&state) {
                return Err(format!("{ppath}.declared_state: unsupported v0.1 state"));
            }
            super::expect_text(
                super::field(p, "state_basis", &ppath)?,
                &format!("{ppath}.state_basis"),
                "synthetic",
            )?;
        }
    }
    Ok(cases.len())
}

fn validate_policy_contract(root: &JsonValue) -> Result<usize, String> {
    let policies = super::array(
        super::field(root, "conditional_policies", "fixture")?,
        "fixture.conditional_policies",
    )?;
    if policies.is_empty() {
        return Err("conditional_policies must not be empty".to_owned());
    }
    let allowed_actions = [
        "request_authoritative_evidence",
        "recompute_candidate_consequences",
        "retain_alternate_as_unconfirmed_option",
    ];
    let mut policy_ids = BTreeSet::new();
    let mut rule_count = 0usize;
    for (index, policy) in policies.iter().enumerate() {
        let path = format!("fixture.conditional_policies[{index}]");
        super::exact_keys(policy, &["policy_id", "policy_kind", "rules"], &path)?;
        let policy_id = field_text(policy, "policy_id", &path)?;
        if !policy_ids.insert(policy_id.to_owned()) {
            return Err(format!("{path}.policy_id: duplicate policy ID"));
        }
        super::expect_text(
            super::field(policy, "policy_kind", &path)?,
            &format!("{path}.policy_kind"),
            "bounded_simulation_only",
        )?;
        let rules = super::array(
            super::field(policy, "rules", &path)?,
            &format!("{path}.rules"),
        )?;
        if rules.is_empty() {
            return Err(format!("{path}.rules must not be empty"));
        }
        for (rindex, rule) in rules.iter().enumerate() {
            let rpath = format!("{path}.rules[{rindex}]");
            super::exact_keys(
                rule,
                &[
                    "rule_id",
                    "observation",
                    "matches_any",
                    "simulated_action",
                    "next_rehearsal_state",
                    "required_claims",
                    "forbidden_effects",
                ],
                &rpath,
            )?;
            nonempty(
                super::field(rule, "rule_id", &rpath)?,
                &format!("{rpath}.rule_id"),
            )?;
            nonempty(
                super::field(rule, "observation", &rpath)?,
                &format!("{rpath}.observation"),
            )?;
            string_array(
                super::field(rule, "matches_any", &rpath)?,
                &format!("{rpath}.matches_any"),
            )?;
            let action = field_text(rule, "simulated_action", &rpath)?;
            if !allowed_actions.contains(&action) {
                return Err(format!(
                    "{rpath}.simulated_action: action outside bounded policy vocabulary"
                ));
            }
            nonempty(
                super::field(rule, "next_rehearsal_state", &rpath)?,
                &format!("{rpath}.next_rehearsal_state"),
            )?;
            string_array(
                super::field(rule, "required_claims", &rpath)?,
                &format!("{rpath}.required_claims"),
            )?;
            let forbidden = super::array(
                super::field(rule, "forbidden_effects", &rpath)?,
                &format!("{rpath}.forbidden_effects"),
            )?;
            if forbidden.is_empty() {
                return Err(format!(
                    "{rpath}.forbidden_effects must explicitly prohibit side effects"
                ));
            }
            for (findex, effect) in forbidden.iter().enumerate() {
                let effect = path_text(effect, &format!("{rpath}.forbidden_effects[{findex}]"))?;
                if ![
                    "create_reservation",
                    "dispatch_vessel",
                    "claim_navigation_authority",
                    "claim_capacity_confirmed",
                ]
                .contains(&effect)
                {
                    return Err(format!(
                        "{rpath}.forbidden_effects: unrecognized effect sentinel"
                    ));
                }
            }
            rule_count = rule_count
                .checked_add(1)
                .ok_or_else(|| "policy rule count overflow".to_owned())?;
        }
    }
    Ok(rule_count)
}

fn compute_metric(
    trace_id: String,
    case_id: String,
    service_id: String,
    served_rates: &[JsonValue],
    service: &ServiceClass,
    duration: u64,
    horizon_count: usize,
    policy_floor: f64,
    consecutive_required: usize,
    path: &str,
) -> Result<TrajectoryMetric, String> {
    if served_rates.len() != horizon_count {
        return Err(format!(
            "{path}.served_rate: expected {horizon_count} values, got {}",
            served_rates.len()
        ));
    }

    // Compute the first qualifying recovery point, with end-of-interval timing.
    let mut saw_breach = false;
    let mut consecutive = 0usize;
    let mut recovery = None;
    for (index, raw_rate) in served_rates.iter().enumerate() {
        let rate = super::integer(raw_rate, &format!("{path}.served_rate[{index}]"))?;
        let ratio = rate as f64 / service.demand_rate as f64;
        if ratio < policy_floor || ratio < service.minimum_service_ratio {
            saw_breach = true;
            consecutive = 0;
        } else if saw_breach {
            consecutive += 1;
            if consecutive >= consecutive_required {
                recovery = Some(
                    u64::try_from(index + 1)
                        .map_err(|_| format!("{path}: recovery offset overflow"))?
                        .checked_mul(duration)
                        .ok_or_else(|| format!("{path}: recovery time overflow"))?,
                );
                break;
            }
        }
    }

    // Integrate the entire trajectory independently of the early recovery stop.
    let total_unmet =
        served_rates
            .iter()
            .enumerate()
            .try_fold(0u64, |sum, (index, raw_rate)| {
                let rate = super::integer(raw_rate, &format!("{path}.served_rate[{index}]"))?;
                let deficit = service.demand_rate.saturating_sub(rate);
                let amount = deficit
                    .checked_mul(duration)
                    .ok_or_else(|| format!("{path}: service-loss multiplication overflow"))?;
                sum.checked_add(amount)
                    .ok_or_else(|| format!("{path}: integrated unmet-service overflow"))
            })?;
    Ok(TrajectoryMetric {
        trace_id,
        case_id,
        service_id,
        unit: service.unit.clone(),
        integrated_unmet_service_units: total_unmet,
        recovery_time_hours: recovery,
        recovery_status: if recovery.is_some() {
            "RECOVERED_WITHIN_DECLARED_HORIZON".to_owned()
        } else {
            "NOT_OBSERVED_WITHIN_HORIZON".to_owned()
        },
        intervals_evaluated: served_rates.len(),
    })
}
fn evaluate_bytes(bytes: &[u8], source_commit: &str) -> Result<AdaptiveResilienceReport, String> {
    if bytes.len() > MAX_FIXTURE_BYTES {
        return Err(format!(
            "adaptive resilience fixture exceeds {MAX_FIXTURE_BYTES}-byte bound"
        ));
    }
    let root = super::parse_strict(bytes)?;
    super::exact_keys(&root, ROOT_KEYS, "adaptive_resilience")?;
    validate_authority(&root)?;

    let (duration, horizon, interval_count) = validate_time_axis(&root)?;
    let (policy_floor, consecutive_required) = validate_metrics_policy(&root)?;
    let service_classes = validate_service_classes(&root)?;
    for service in service_classes.values() {
        if (service.minimum_service_ratio - policy_floor).abs() > f64::EPSILON {
            return Err("service-class and recovery-policy floors must match for v0.1".to_owned());
        }
    }

    let node_array = super::array(
        super::field(&root, "dependency_nodes", "fixture")?,
        "fixture.dependency_nodes",
    )?;
    let mut node_ids = BTreeSet::new();
    for (index, node) in node_array.iter().enumerate() {
        let id = field_text(
            node,
            "node_id",
            &format!("fixture.dependency_nodes[{index}]"),
        )?;
        node_ids.insert(id.to_owned());
    }
    let (node_count, edge_count) = validate_dependency_contract(&root)?;
    let declared_cases = super::array(
        super::field(&root, "scenario_cases", "fixture")?,
        "fixture.scenario_cases",
    )?;
    let mut case_ids = BTreeSet::new();
    for (index, case) in declared_cases.iter().enumerate() {
        let id = field_text(case, "case_id", &format!("fixture.scenario_cases[{index}]"))?;
        if !case_ids.insert(id.to_owned()) {
            return Err(format!(
                "fixture.scenario_cases[{index}].case_id: duplicate scenario case"
            ));
        }
    }
    let case_count = validate_scenario_cases(&root, horizon, &node_ids)?;
    let rule_count = validate_policy_contract(&root)?;

    let traces = super::array(
        super::field(&root, "synthetic_reference_traces", "fixture")?,
        "fixture.synthetic_reference_traces",
    )?;
    if traces.is_empty() {
        return Err("synthetic_reference_traces must not be empty".to_owned());
    }
    let mut trace_ids = BTreeSet::new();
    let mut metrics = Vec::with_capacity(traces.len());
    for (index, trace) in traces.iter().enumerate() {
        let path = format!("fixture.synthetic_reference_traces[{index}]");
        super::exact_keys(
            trace,
            &[
                "trace_id",
                "case_id",
                "service_id",
                "trace_status",
                "served_rate",
                "expected_contract_metrics",
            ],
            &path,
        )?;
        let trace_id = field_text(trace, "trace_id", &path)?.to_owned();
        if trace_id.trim().is_empty() || !trace_ids.insert(trace_id.clone()) {
            return Err(format!("{path}.trace_id: empty or duplicate trace ID"));
        }
        let case_id = field_text(trace, "case_id", &path)?.to_owned();
        if !case_ids.contains(&case_id) {
            return Err(format!("{path}.case_id: unknown scenario case"));
        }
        let service_id = field_text(trace, "service_id", &path)?.to_owned();
        if !service_classes.contains_key(&service_id) {
            return Err(format!("{path}.service_id: unknown service class"));
        }
        super::expect_text(
            super::field(trace, "trace_status", &path)?,
            &format!("{path}.trace_status"),
            "illustrative_not_simulator_output",
        )?;
        let served = super::array(
            super::field(trace, "served_rate", &path)?,
            &format!("{path}.served_rate"),
        )?;
        let service = service_classes
            .get(&service_id)
            .expect("validated service exists");
        let metric = compute_metric(
            trace_id,
            case_id,
            service_id,
            served,
            service,
            duration,
            interval_count,
            policy_floor,
            consecutive_required,
            &path,
        )?;

        let expected = super::field(trace, "expected_contract_metrics", &path)?;
        super::exact_keys(
            expected,
            &[
                "integrated_service_loss_units",
                "recovery_time_hours",
                "recovery_status",
            ],
            &format!("{path}.expected_contract_metrics"),
        )?;
        let expected_loss = super::integer(
            super::field(
                expected,
                "integrated_service_loss_units",
                &format!("{path}.expected_contract_metrics"),
            )?,
            &format!("{path}.expected_contract_metrics.integrated_service_loss_units"),
        )?;
        if expected_loss != metric.integrated_unmet_service_units {
            return Err(format!(
                "{path}: expected unmet-service metric {}, recomputed {}",
                expected_loss, metric.integrated_unmet_service_units
            ));
        }
        let expected_recovery = match super::field(
            expected,
            "recovery_time_hours",
            &format!("{path}.expected_contract_metrics"),
        )? {
            JsonValue::Null => None,
            value => Some(super::integer(
                value,
                &format!("{path}.expected_contract_metrics.recovery_time_hours"),
            )?),
        };
        if expected_recovery != metric.recovery_time_hours {
            return Err(format!(
                "{path}: expected recovery {:?}, recomputed {:?}",
                expected_recovery, metric.recovery_time_hours
            ));
        }
        let expected_status = field_text(
            expected,
            "recovery_status",
            &format!("{path}.expected_contract_metrics"),
        )?;
        if expected_status != metric.recovery_status {
            return Err(format!(
                "{path}: recovery status does not match the independently declared expectation"
            ));
        }
        metrics.push(metric);
    }

    let unknown_cases = super::array(
        super::field(&root, "unquantified_cases", "fixture")?,
        "fixture.unquantified_cases",
    )?;
    if unknown_cases.is_empty() {
        return Err(
            "unquantified_cases must preserve at least one unknown-state control".to_owned(),
        );
    }
    for (index, case) in unknown_cases.iter().enumerate() {
        let path = format!("fixture.unquantified_cases[{index}]");
        super::exact_keys(
            case,
            &[
                "case_id",
                "status",
                "reason_code",
                "quantitative_service_projection",
                "reason",
            ],
            &path,
        )?;
        let case_id = field_text(case, "case_id", &path)?;
        if !case_ids.contains(case_id) {
            return Err(format!("{path}.case_id: unknown scenario case"));
        }
        if metrics.iter().any(|metric| metric.case_id == case_id) {
            return Err(format!(
                "{path}: an unquantified case must not have a numeric trajectory"
            ));
        }
        super::expect_text(
            super::field(case, "status", &path)?,
            &format!("{path}.status"),
            "INDETERMINATE",
        )?;
        super::expect_text(
            super::field(case, "quantitative_service_projection", &path)?,
            &format!("{path}.quantitative_service_projection"),
            "NOT_PRODUCED",
        )?;
        nonempty(
            super::field(case, "reason_code", &path)?,
            &format!("{path}.reason_code"),
        )?;
        nonempty(
            super::field(case, "reason", &path)?,
            &format!("{path}.reason"),
        )?;
    }
    string_array(
        super::field(&root, "fixture_warnings", "fixture")?,
        "fixture.fixture_warnings",
    )?;

    let authority = super::field(&root, "authority", "fixture")?;
    Ok(AdaptiveResilienceReport {
        status: "SYNTHETIC_TRAJECTORY_CONTRACT_VERIFIED_NOT_ADMITTED",
        schema_id: "luminous.symtropy.maritime-adaptive-resilience",
        schema_version: "0.1.0",
        fixture_sha256: super::sha256_bytes(bytes),
        fixture_byte_length: bytes.len(),
        source_commit_binding: source_commit.to_owned(),
        metrics_checked: metrics.len(),
        horizon_censored_traces: metrics
            .iter()
            .filter(|metric| metric.recovery_time_hours.is_none())
            .count(),
        trajectory_metrics: metrics,
        dependency_nodes_declared: node_count,
        dependency_edges_declared: edge_count,
        scenario_cases_declared: case_count,
        conditional_policy_rules_declared: rule_count,
        quantitative_dependency_propagation_performed: false,
        physical_simulation_performed: false,
        engineering_admission: "NotAdmitted",
        navigation_authorized: super::bool_value(
            super::field(authority, "navigation_authorized", "authority")?,
            "authority.navigation_authorized",
        )?,
        physical_truth_claim_authorized: super::bool_value(
            super::field(authority, "physical_truth_claim_authorized", "authority")?,
            "authority.physical_truth_claim_authorized",
        )?,
        berth_or_slot_authorized: super::bool_value(
            super::field(authority, "berth_or_slot_authorized", "authority")?,
            "authority.berth_or_slot_authorized",
        )?,
        physical_execution_authorized: super::bool_value(
            super::field(authority, "physical_execution_authorized", "authority")?,
            "authority.physical_execution_authorized",
        )?,
        claim_ceiling: "Synthetic service-trajectory arithmetic and contract-shape checks only; no physical simulation, propagation, operational approval or real-world performance claim.",
    })
}

pub(super) fn run_cli(fixture_path: &str) -> Result<(), String> {
    let metadata = fs::metadata(fixture_path).map_err(|error| {
        format!("cannot stat adaptive resilience fixture {fixture_path}: {error}")
    })?;
    if metadata.len() > MAX_FIXTURE_BYTES as u64 {
        return Err(format!(
            "adaptive resilience fixture exceeds {MAX_FIXTURE_BYTES}-byte bound"
        ));
    }
    let bytes = fs::read(fixture_path).map_err(|error| {
        format!("cannot read adaptive resilience fixture {fixture_path}: {error}")
    })?;
    let source_commit =
        env::var("MARITIME_SOURCE_COMMIT").unwrap_or_else(|_| "UNBOUND_LOCAL_SOURCE".to_owned());
    let report = evaluate_bytes(&bytes, &source_commit)?;
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize adaptive resilience report: {error}"))?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &[u8] =
        include_bytes!("../../../../docs/tech/MARITIME_ADAPTIVE_RESILIENCE_V0_1.json");

    // Intentionally separate from production computation and parser helpers:
    // this oracle operates on serde_json::Value and direct numeric loops.
    fn independent_oracle() -> Vec<(String, u64, Option<u64>, String)> {
        let root: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        let interval = root["time_axis"]["duration_hours"].as_u64().unwrap();
        let services = root["service_classes"].as_array().unwrap();
        let mut result = Vec::new();
        for trace in root["synthetic_reference_traces"].as_array().unwrap() {
            let id = trace["trace_id"].as_str().unwrap().to_owned();
            let service_id = trace["service_id"].as_str().unwrap();
            let service = services
                .iter()
                .find(|s| s["service_id"] == service_id)
                .unwrap();
            let demand = service["planned_demand_rate"].as_u64().unwrap();
            let floor = service["minimum_service_ratio"].as_f64().unwrap();
            let needed = root["metrics_policy"]["recovery_rule"]["consecutive_intervals_required"]
                .as_u64()
                .unwrap() as usize;
            let rates = trace["served_rate"].as_array().unwrap();
            let mut loss = 0_u64;
            let mut breached = false;
            let mut run = 0_usize;
            let mut recovery = None;
            for (idx, rate) in rates.iter().enumerate() {
                let q = rate.as_u64().unwrap();
                loss += demand.saturating_sub(q) * interval;
                if (q as f64 / demand as f64) < floor {
                    breached = true;
                    run = 0;
                } else if breached {
                    run += 1;
                    if run == needed && recovery.is_none() {
                        recovery = Some((idx as u64 + 1) * interval);
                    }
                }
            }
            let status = if recovery.is_some() {
                "RECOVERED_WITHIN_DECLARED_HORIZON"
            } else {
                "NOT_OBSERVED_WITHIN_HORIZON"
            }
            .to_owned();
            result.push((id, loss, recovery, status));
        }
        result
    }

    #[test]
    fn frozen_fixture_matches_independent_oracle() {
        let report = evaluate_bytes(FIXTURE, "test-commit").expect("fixture must evaluate");
        let oracle = independent_oracle();
        assert_eq!(report.trajectory_metrics.len(), oracle.len());
        for metric in &report.trajectory_metrics {
            let expected = oracle
                .iter()
                .find(|item| item.0 == metric.trace_id)
                .unwrap();
            assert_eq!(metric.integrated_unmet_service_units, expected.1);
            assert_eq!(metric.recovery_time_hours, expected.2);
            assert_eq!(metric.recovery_status, expected.3);
        }
        assert_eq!(report.horizon_censored_traces, 2);
        assert!(!report.physical_simulation_performed);
        assert!(!report.quantitative_dependency_propagation_performed);
        assert_eq!(report.engineering_admission, "NotAdmitted");
    }

    #[test]
    fn report_is_deterministic_and_bound_to_exact_bytes() {
        let first = evaluate_bytes(FIXTURE, "test-commit").unwrap();
        let second = evaluate_bytes(FIXTURE, "test-commit").unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first.fixture_sha256, super::super::sha256_bytes(FIXTURE));
        assert_ne!(
            evaluate_bytes(FIXTURE, "another-commit")
                .unwrap()
                .source_commit_binding,
            first.source_commit_binding
        );
    }

    #[test]
    fn duplicate_json_keys_are_rejected_by_shared_strict_parser() {
        assert!(super::super::parse_strict(br#"{"schema_id":"a","schema_id":"b"}"#).is_err());
    }

    #[test]
    fn unknown_root_fields_fail_closed() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"fixture_kind\": \"synthetic_contract_example\",",
            "\"fixture_kind\": \"synthetic_contract_example\",\n  \"unexpected\": true,",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes(), "test").is_err());
    }

    #[test]
    fn authority_escalation_is_rejected() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"navigation_authorized\": false",
            "\"navigation_authorized\": true",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes(), "test").is_err());
    }

    #[test]
    fn expected_metric_drift_is_rejected() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"integrated_service_loss_units\": 100",
            "\"integrated_service_loss_units\": 99",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes(), "test").is_err());
    }

    #[test]
    fn zero_demand_and_unobserved_as_zero_are_rejected() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"planned_demand_rate\": 100",
            "\"planned_demand_rate\": 0",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes(), "test").is_err());

        let changed = raw.replacen(
            "\"quantitative_service_projection\": \"NOT_PRODUCED\"",
            "\"quantitative_service_projection\": \"0\"",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes(), "test").is_err());
    }

    #[test]
    fn unsupported_propagation_cannot_be_declared_as_model_output() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"propagation_model\": \"not_declared\"",
            "\"propagation_model\": \"invented_model\"",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes(), "test").is_err());
    }

    #[test]
    fn probabilities_and_aggregate_scores_are_not_admitted() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"probability_weights_used\": false",
            "\"probability_weights_used\": true",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes(), "test").is_err());
    }

    #[test]
    fn exact_floor_boundary_recovers_at_the_end_of_the_second_interval() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["synthetic_reference_traces"][0]["served_rate"] =
            serde_json::json!([100, 70, 60, 80, 80, 95]);
        fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["integrated_service_loss_units"] =
            Value::Number(serde_json::Number::from(115));
        fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["recovery_time_hours"] =
            Value::Number(serde_json::Number::from(5));
        let changed = serde_json::to_vec(&fixture).unwrap();
        let report =
            evaluate_bytes(&changed, "test").expect("exact floor is a valid recovery boundary");
        let metric = report
            .trajectory_metrics
            .iter()
            .find(|metric| metric.trace_id == "power-loss-essential-supplies")
            .unwrap();
        assert_eq!(metric.integrated_unmet_service_units, 115);
        assert_eq!(metric.recovery_time_hours, Some(5));
    }

    #[test]
    fn an_interrupted_recovery_streak_does_not_count_as_recovery() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["synthetic_reference_traces"][0]["served_rate"] =
            serde_json::json!([100, 70, 60, 90, 70, 85]);
        fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["integrated_service_loss_units"] =
            Value::Number(serde_json::Number::from(125));
        fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["recovery_time_hours"] =
            Value::Null;
        fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["recovery_status"] =
            Value::String("NOT_OBSERVED_WITHIN_HORIZON".to_owned());
        let changed = serde_json::to_vec(&fixture).unwrap();
        let report =
            evaluate_bytes(&changed, "test").expect("interrupted streak should remain unrecovered");
        let metric = report
            .trajectory_metrics
            .iter()
            .find(|metric| metric.trace_id == "power-loss-essential-supplies")
            .unwrap();
        assert_eq!(metric.integrated_unmet_service_units, 125);
        assert_eq!(metric.recovery_time_hours, None);
        assert_eq!(metric.recovery_status, "NOT_OBSERVED_WITHIN_HORIZON");
    }

    #[test]
    fn recovery_streak_resets_one_point_below_the_floor() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["synthetic_reference_traces"][0]["served_rate"] =
            serde_json::json!([100, 70, 60, 79, 80, 80]);
        fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["integrated_service_loss_units"] =
            Value::Number(serde_json::Number::from(131));
        fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["recovery_time_hours"] =
            Value::Number(serde_json::Number::from(6));
        let changed = serde_json::to_vec(&fixture).unwrap();
        let report =
            evaluate_bytes(&changed, "test").expect("one-under-floor should reset recovery streak");
        let metric = report
            .trajectory_metrics
            .iter()
            .find(|metric| metric.trace_id == "power-loss-essential-supplies")
            .unwrap();
        assert_eq!(metric.integrated_unmet_service_units, 131);
        assert_eq!(metric.recovery_time_hours, Some(6));
    }

    #[test]
    fn trace_cannot_reference_an_undeclared_scenario() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["synthetic_reference_traces"][0]["case_id"] =
            Value::String("undeclared_case".to_owned());
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(
            evaluate_bytes(&changed, "test")
                .unwrap_err()
                .contains("unknown scenario case")
        );
    }

    #[test]
    fn dependency_edges_cannot_reference_unknown_nodes() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["dependency_edges"][0]["to_node"] = Value::String("invented_terminal".to_owned());
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(
            evaluate_bytes(&changed, "test")
                .unwrap_err()
                .contains("unknown node")
        );
    }

    #[test]
    fn perturbation_must_stay_inside_the_declared_horizon() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["scenario_cases"][0]["perturbations"][0]["end_hour_exclusive"] =
            Value::Number(serde_json::Number::from(7));
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(
            evaluate_bytes(&changed, "test")
                .unwrap_err()
                .contains("invalid perturbation interval")
        );
    }

    #[test]
    fn duplicate_trace_identifiers_are_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        let first_id = fixture["synthetic_reference_traces"][0]["trace_id"].clone();
        fixture["synthetic_reference_traces"][1]["trace_id"] = first_id;
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(
            evaluate_bytes(&changed, "test")
                .unwrap_err()
                .contains("duplicate trace ID")
        );
    }

    #[test]
    fn service_rate_exactly_at_floor_counts_as_recovered() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["synthetic_reference_traces"][0]["served_rate"] =
            serde_json::json!([100, 70, 60, 80, 80, 95]);
        fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["integrated_service_loss_units"] =
            Value::Number(serde_json::Number::from(115));
        fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["recovery_time_hours"] =
            Value::Number(serde_json::Number::from(5));
        let changed = serde_json::to_vec(&fixture).unwrap();
        let report = evaluate_bytes(&changed, "test").expect("floor boundary must be inclusive");
        let metric = report
            .trajectory_metrics
            .iter()
            .find(|metric| metric.trace_id == "power-loss-essential-supplies")
            .unwrap();
        assert_eq!(metric.recovery_time_hours, Some(5));
    }

    #[test]
    fn seeded_integer_trajectory_corpus_matches_independent_oracle() {
        let baseline: Value = serde_json::from_slice(FIXTURE).unwrap();
        let demand = 100_u64;
        let floor = 0.8_f64;
        let duration = 1_u64;
        let consecutive_required = 2_usize;

        // A fixed xorshift corpus gives reproducible boundary-rich traces with no
        // additional dependency. The oracle below does not call production helpers.
        for seed in 1_u64..=256 {
            let mut state = seed;
            let mut rates = Vec::with_capacity(6);
            for _ in 0..6 {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                rates.push(state % 121);
            }

            let expected_loss = rates.iter().fold(0_u64, |sum, rate| {
                sum + demand.saturating_sub(*rate) * duration
            });
            let mut saw_breach = false;
            let mut streak = 0_usize;
            let mut expected_recovery = None;
            for (index, rate) in rates.iter().enumerate() {
                if (*rate as f64) / (demand as f64) < floor {
                    saw_breach = true;
                    streak = 0;
                } else if saw_breach {
                    streak += 1;
                    if streak >= consecutive_required && expected_recovery.is_none() {
                        expected_recovery = Some((index as u64 + 1) * duration);
                    }
                }
            }
            let expected_status = if expected_recovery.is_some() {
                "RECOVERED_WITHIN_DECLARED_HORIZON"
            } else {
                "NOT_OBSERVED_WITHIN_HORIZON"
            };

            let mut fixture = baseline.clone();
            fixture["synthetic_reference_traces"][0]["served_rate"] =
                serde_json::to_value(&rates).unwrap();
            fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["integrated_service_loss_units"] =
                Value::Number(serde_json::Number::from(expected_loss));
            fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["recovery_time_hours"] =
                expected_recovery
                    .map(serde_json::Number::from)
                    .map(Value::Number)
                    .unwrap_or(Value::Null);
            fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["recovery_status"] =
                Value::String(expected_status.to_owned());

            let bytes = serde_json::to_vec(&fixture).unwrap();
            let report = evaluate_bytes(&bytes, "seeded-oracle-test").unwrap_or_else(|error| {
                panic!("seed {seed} should satisfy its generated oracle: {error}")
            });
            let actual = report
                .trajectory_metrics
                .iter()
                .find(|metric| metric.trace_id == "power-loss-essential-supplies")
                .expect("the first trace should be reported");
            assert_eq!(
                actual.integrated_unmet_service_units, expected_loss,
                "seed {seed}"
            );
            assert_eq!(actual.recovery_time_hours, expected_recovery, "seed {seed}");
            assert_eq!(actual.recovery_status, expected_status, "seed {seed}");
        }
    }

    #[test]
    fn null_interval_is_rejected_instead_of_becoming_zero() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["synthetic_reference_traces"][0]["served_rate"][3] = Value::Null;
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(
            evaluate_bytes(&changed, "test")
                .unwrap_err()
                .contains("expected integer")
        );
    }

    #[test]
    fn negative_and_fractional_service_rates_are_rejected() {
        let mut negative: Value = serde_json::from_slice(FIXTURE).unwrap();
        negative["synthetic_reference_traces"][0]["served_rate"][3] =
            Value::Number(serde_json::Number::from(-1));
        assert!(evaluate_bytes(&serde_json::to_vec(&negative).unwrap(), "test").is_err());

        let mut fractional: Value = serde_json::from_slice(FIXTURE).unwrap();
        fractional["synthetic_reference_traces"][0]["served_rate"][3] =
            serde_json::Number::from_f64(79.5)
                .map(Value::Number)
                .unwrap();
        assert!(evaluate_bytes(&serde_json::to_vec(&fractional).unwrap(), "test").is_err());
    }

    #[test]
    fn lower_service_rate_cannot_reduce_integrated_unmet_service() {
        let baseline: Value = serde_json::from_slice(FIXTURE).unwrap();
        let baseline_report = evaluate_bytes(FIXTURE, "baseline").unwrap();
        let baseline_metric = baseline_report
            .trajectory_metrics
            .iter()
            .find(|metric| metric.trace_id == "power-loss-essential-supplies")
            .unwrap();

        let mut changed_fixture = baseline;
        changed_fixture["synthetic_reference_traces"][0]["served_rate"] =
            serde_json::json!([100, 70, 60, 84, 90, 95]);
        changed_fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["integrated_service_loss_units"] =
            Value::Number(serde_json::Number::from(101));
        changed_fixture["synthetic_reference_traces"][0]["expected_contract_metrics"]["recovery_time_hours"] =
            Value::Number(serde_json::Number::from(5));
        let changed = serde_json::to_vec(&changed_fixture).unwrap();
        let changed_report = evaluate_bytes(&changed, "changed").unwrap();
        let changed_metric = changed_report
            .trajectory_metrics
            .iter()
            .find(|metric| metric.trace_id == "power-loss-essential-supplies")
            .unwrap();

        assert!(
            changed_metric.integrated_unmet_service_units
                >= baseline_metric.integrated_unmet_service_units
        );
        assert_eq!(baseline_metric.integrated_unmet_service_units, 100);
        assert_eq!(changed_metric.integrated_unmet_service_units, 101);
    }

    #[test]
    fn a_required_unknown_case_cannot_gain_a_quantitative_projection() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"status\": \"INDETERMINATE\"",
            "\"status\": \"FEASIBLE\"",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes(), "test").is_err());
    }
}
