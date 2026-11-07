// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact bounded setpoint-scheduling reference for trajectory-conditioned WPT.
//!
//! This oracle consumes already-qualified per-interval receiver-power and
//! additive asset-resource envelopes. It does not infer coupling, efficiency,
//! thermal response, pavement stress or safety limits from vehicle offset.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_INTERVALS: usize = 32;
const MAX_SETPOINT_QUANTA: u64 = 32;
const MAX_SEARCH_NODES: u64 = 1_000_000;
const MAX_REQUEST_MWH: u64 = 1_000_000;
const SHA256_HEX_LEN: usize = 64;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in object(value, path)?.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn uint_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn array_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a [Value], String> {
    field(value, key, path)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an array"))
}

fn is_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Clone)]
struct EnvelopeInterval {
    sample_id: String,
    envelope_id: String,
    envelope_sha256: String,
    max_setpoint_quanta: u64,
    useful_energy_mwh_per_quantum: u64,
    asset_stress_units_per_quantum: u64,
}

#[derive(Debug, Clone)]
struct Scenario {
    scenario_id: String,
    trajectory_id: String,
    coordinate_frame_id: String,
    requested_energy_mwh: u64,
    minimum_service_mwh: u64,
    initial_setpoint_quanta: u64,
    max_slew_quanta_per_interval: u64,
    cumulative_stress_budget_units: u64,
    max_search_nodes: u64,
    candidate_setpoints_quanta: Vec<u64>,
    intervals: Vec<EnvelopeInterval>,
}

fn parse_scenario(value: &Value) -> Result<Scenario, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "scenario_id",
            "evidence_kind",
            "quality_flags",
            "trajectory_id",
            "coordinate_frame_id",
            "request",
            "control_limits",
            "policy",
            "candidate_controller_setpoints_quanta",
            "intervals",
        ],
        "$",
    )?;

    if string_field(value, "schema_version", "$")? != "wpt-trajectory-control-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let scenario_id = string_field(value, "scenario_id", "$")?.to_string();
    if string_field(value, "evidence_kind", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$.evidence_kind",
            "v0 admits synthetic known-truth scenarios only",
        ));
    }
    let flags = array_field(value, "quality_flags", "$")?;
    if flags.len() != 1 || flags[0].as_str() != Some("SYNTHETIC_ONLY") {
        return Err(fail(
            "$.quality_flags",
            "v0 requires exactly SYNTHETIC_ONLY",
        ));
    }
    let trajectory_id = string_field(value, "trajectory_id", "$")?.to_string();
    let coordinate_frame_id = string_field(value, "coordinate_frame_id", "$")?.to_string();

    let request = field(value, "request", "$")?;
    exact_keys(
        request,
        &[
            "requested_useful_energy_mwh",
            "minimum_service_energy_mwh",
            "initial_setpoint_quanta",
        ],
        "$.request",
    )?;
    let requested_energy_mwh =
        uint_field(request, "requested_useful_energy_mwh", "$.request")?;
    let minimum_service_mwh =
        uint_field(request, "minimum_service_energy_mwh", "$.request")?;
    let initial_setpoint_quanta =
        uint_field(request, "initial_setpoint_quanta", "$.request")?;
    if requested_energy_mwh == 0
        || requested_energy_mwh > MAX_REQUEST_MWH
        || minimum_service_mwh == 0
        || minimum_service_mwh > requested_energy_mwh
    {
        return Err(fail(
            "$.request",
            format!(
                "require 1 <= minimum_service_energy_mwh <= requested_useful_energy_mwh <= {MAX_REQUEST_MWH}"
            ),
        ));
    }
    if initial_setpoint_quanta > MAX_SETPOINT_QUANTA {
        return Err(fail(
            "$.request.initial_setpoint_quanta",
            format!("must be <= {MAX_SETPOINT_QUANTA}"),
        ));
    }

    let limits = field(value, "control_limits", "$")?;
    exact_keys(
        limits,
        &[
            "max_slew_quanta_per_interval",
            "cumulative_asset_stress_budget_units",
        ],
        "$.control_limits",
    )?;
    let max_slew_quanta_per_interval =
        uint_field(limits, "max_slew_quanta_per_interval", "$.control_limits")?;
    if max_slew_quanta_per_interval > MAX_SETPOINT_QUANTA {
        return Err(fail(
            "$.control_limits.max_slew_quanta_per_interval",
            format!("must be <= {MAX_SETPOINT_QUANTA}"),
        ));
    }
    let cumulative_stress_budget_units =
        uint_field(limits, "cumulative_asset_stress_budget_units", "$.control_limits")?;

    let policy = field(value, "policy", "$")?;
    exact_keys(policy, &["maximum_search_nodes"], "$.policy")?;
    let max_search_nodes = uint_field(policy, "maximum_search_nodes", "$.policy")?;
    if max_search_nodes == 0 || max_search_nodes > MAX_SEARCH_NODES {
        return Err(fail(
            "$.policy.maximum_search_nodes",
            format!("must be within 1..={MAX_SEARCH_NODES}"),
        ));
    }

    let candidate_rows = array_field(value, "candidate_controller_setpoints_quanta", "$")?;
    let intervals_rows = array_field(value, "intervals", "$")?;
    if !(2..=MAX_INTERVALS).contains(&intervals_rows.len()) {
        return Err(fail(
            "$.intervals",
            format!("must contain 2..={MAX_INTERVALS} intervals"),
        ));
    }
    if candidate_rows.len() != intervals_rows.len() {
        return Err(fail(
            "$.candidate_controller_setpoints_quanta",
            "candidate controller must provide exactly one setpoint per interval",
        ));
    }
    let candidate_setpoints_quanta = candidate_rows
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value.as_u64().ok_or_else(|| {
                fail(
                    &format!("$.candidate_controller_setpoints_quanta[{index}]"),
                    "must be a non-negative integer",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut intervals = Vec::with_capacity(intervals_rows.len());
    let mut sample_ids = BTreeSet::new();
    for (index, row) in intervals_rows.iter().enumerate() {
        let path = format!("$.intervals[{index}]");
        exact_keys(
            row,
            &[
                "trajectory_sample_id",
                "envelope_id",
                "envelope_sha256",
                "support_state",
                "max_setpoint_quanta",
                "useful_energy_mwh_per_setpoint_quantum",
                "asset_stress_units_per_setpoint_quantum",
            ],
            &path,
        )?;
        let sample_id = string_field(row, "trajectory_sample_id", &path)?.to_string();
        if !sample_ids.insert(sample_id.clone()) {
            return Err(fail(&path, format!("duplicate trajectory_sample_id {sample_id}")));
        }
        let envelope_id = string_field(row, "envelope_id", &path)?.to_string();
        let envelope_sha256 = string_field(row, "envelope_sha256", &path)?.to_string();
        if !is_sha256(&envelope_sha256) {
            return Err(fail(
                &format!("{path}.envelope_sha256"),
                "must be 64 lowercase hexadecimal characters",
            ));
        }
        if string_field(row, "support_state", &path)? != "WITHIN_VALIDATED_SUPPORT" {
            return Err(fail(
                &format!("{path}.support_state"),
                "unsupported or out-of-domain envelope cannot enter the control oracle",
            ));
        }
        let max_setpoint_quanta = uint_field(row, "max_setpoint_quanta", &path)?;
        if max_setpoint_quanta > MAX_SETPOINT_QUANTA {
            return Err(fail(
                &format!("{path}.max_setpoint_quanta"),
                format!("must be <= {MAX_SETPOINT_QUANTA}"),
            ));
        }
        let useful_energy_mwh_per_quantum =
            uint_field(row, "useful_energy_mwh_per_setpoint_quantum", &path)?;
        if useful_energy_mwh_per_quantum == 0 {
            return Err(fail(
                &format!("{path}.useful_energy_mwh_per_setpoint_quantum"),
                "must be positive",
            ));
        }
        let asset_stress_units_per_quantum =
            uint_field(row, "asset_stress_units_per_setpoint_quantum", &path)?;
        intervals.push(EnvelopeInterval {
            sample_id,
            envelope_id,
            envelope_sha256,
            max_setpoint_quanta,
            useful_energy_mwh_per_quantum,
            asset_stress_units_per_quantum,
        });
    }

    Ok(Scenario {
        scenario_id,
        trajectory_id,
        coordinate_frame_id,
        requested_energy_mwh,
        minimum_service_mwh,
        initial_setpoint_quanta,
        max_slew_quanta_per_interval,
        cumulative_stress_budget_units,
        max_search_nodes,
        candidate_setpoints_quanta,
        intervals,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervalSetpoint {
    pub trajectory_sample_id: String,
    pub setpoint_quanta: u64,
    pub useful_energy_mwh: u64,
    pub asset_stress_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleOutcome {
    pub setpoints_quanta: Vec<u64>,
    pub interval_results: Vec<IntervalSetpoint>,
    pub delivered_useful_energy_mwh: u64,
    pub shortfall_mwh: u64,
    pub cumulative_asset_stress_units: u64,
    pub control_constraints_pass: bool,
    pub service_floor_met: bool,
    pub feasible: bool,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrajectoryControlReport {
    pub scenario_id: String,
    pub trajectory_id: String,
    pub coordinate_frame_id: String,
    pub interval_count: usize,
    pub search_nodes: u64,
    pub peak_dp_states: usize,
    pub feasible: bool,
    pub optimal: ScheduleOutcome,
    pub candidate_controller: ScheduleOutcome,
    pub fixed_setpoint_baseline: Option<ScheduleOutcome>,
    pub greedy_baseline: ScheduleOutcome,
    pub candidate_optimality_gap_mwh: Option<u64>,
    pub fixed_baseline_optimality_gap_mwh: Option<u64>,
    pub greedy_optimality_gap_mwh: Option<u64>,
    pub envelope_receipts: Vec<(String, String)>,
    pub claim_ceiling: &'static str,
}

fn checked_mul(left: u64, right: u64, path: &str) -> Result<u64, String> {
    left.checked_mul(right)
        .ok_or_else(|| fail(path, "integer multiplication overflowed"))
}

fn score_schedule(
    scenario: &Scenario,
    setpoints: &[u64],
    schedule_name: &str,
) -> Result<ScheduleOutcome, String> {
    if setpoints.len() != scenario.intervals.len() {
        return Err(fail(
            schedule_name,
            "must contain exactly one setpoint per trajectory interval",
        ));
    }
    let mut delivered = 0_u64;
    let mut cumulative_stress = 0_u64;
    let mut previous = scenario.initial_setpoint_quanta;
    let mut violations = Vec::new();
    let mut interval_results = Vec::with_capacity(setpoints.len());

    for (index, (interval, setpoint)) in scenario.intervals.iter().zip(setpoints).enumerate() {
        let path = format!("{schedule_name}[{index}]");
        let energy = checked_mul(
            *setpoint,
            interval.useful_energy_mwh_per_quantum,
            &path,
        )?;
        let stress = checked_mul(
            *setpoint,
            interval.asset_stress_units_per_quantum,
            &path,
        )?;
        delivered = delivered
            .checked_add(energy)
            .ok_or_else(|| fail(&path, "cumulative useful energy overflowed"))?;
        cumulative_stress = cumulative_stress
            .checked_add(stress)
            .ok_or_else(|| fail(&path, "cumulative asset stress overflowed"))?;

        if *setpoint > interval.max_setpoint_quanta {
            violations.push(format!(
                "{}: setpoint {} exceeds per-sample envelope {}",
                interval.sample_id, setpoint, interval.max_setpoint_quanta
            ));
        }
        let slew = setpoint.abs_diff(previous);
        if slew > scenario.max_slew_quanta_per_interval {
            violations.push(format!(
                "{}: slew {} exceeds limit {}",
                interval.sample_id, slew, scenario.max_slew_quanta_per_interval
            ));
        }
        if cumulative_stress > scenario.cumulative_stress_budget_units {
            violations.push(format!(
                "{}: cumulative stress {} exceeds budget {}",
                interval.sample_id,
                cumulative_stress,
                scenario.cumulative_stress_budget_units
            ));
        }

        interval_results.push(IntervalSetpoint {
            trajectory_sample_id: interval.sample_id.clone(),
            setpoint_quanta: *setpoint,
            useful_energy_mwh: energy.min(
                scenario
                    .requested_energy_mwh
                    .saturating_sub(delivered.saturating_sub(energy)),
            ),
            asset_stress_units: stress,
        });
        previous = *setpoint;
    }

    let delivered_useful_energy_mwh = delivered.min(scenario.requested_energy_mwh);
    let shortfall_mwh = scenario
        .requested_energy_mwh
        .saturating_sub(delivered_useful_energy_mwh);
    let control_constraints_pass = violations.is_empty();
    let service_floor_met = delivered_useful_energy_mwh >= scenario.minimum_service_mwh;
    Ok(ScheduleOutcome {
        setpoints_quanta: setpoints.to_vec(),
        interval_results,
        delivered_useful_energy_mwh,
        shortfall_mwh,
        cumulative_asset_stress_units: cumulative_stress,
        control_constraints_pass,
        service_floor_met,
        feasible: control_constraints_pass && service_floor_met,
        violations,
    })
}

fn better_schedule(left: &ScheduleOutcome, right: &ScheduleOutcome) -> bool {
    left.delivered_useful_energy_mwh > right.delivered_useful_energy_mwh
        || (left.delivered_useful_energy_mwh == right.delivered_useful_energy_mwh
            && (left.cumulative_asset_stress_units < right.cumulative_asset_stress_units
                || (left.cumulative_asset_stress_units == right.cumulative_asset_stress_units
                    && left.setpoints_quanta < right.setpoints_quanta)))
}

#[derive(Debug, Clone)]
struct Node {
    delivered_mwh: u64,
    setpoints: Vec<u64>,
}

fn fixed_baseline(scenario: &Scenario) -> Result<Option<ScheduleOutcome>, String> {
    let max_constant = scenario
        .intervals
        .iter()
        .map(|interval| interval.max_setpoint_quanta)
        .min()
        .unwrap_or(0);
    let mut best: Option<ScheduleOutcome> = None;
    for q in 0..=max_constant {
        let candidate = score_schedule(
            scenario,
            &vec![q; scenario.intervals.len()],
            "$.fixed_setpoint_baseline",
        )?;
        if candidate.control_constraints_pass
            && best.as_ref().is_none_or(|current| better_schedule(&candidate, current))
        {
            best = Some(candidate);
        }
    }
    Ok(best)
}

fn greedy_baseline(scenario: &Scenario) -> Result<ScheduleOutcome, String> {
    let mut setpoints = Vec::with_capacity(scenario.intervals.len());
    let mut previous = scenario.initial_setpoint_quanta;
    let mut stress = 0_u64;
    let mut delivered = 0_u64;

    for interval in &scenario.intervals {
        let lower = previous.saturating_sub(scenario.max_slew_quanta_per_interval);
        let requested_remainder = scenario.requested_energy_mwh.saturating_sub(delivered);
        let request_limited_max = requested_remainder
            .div_ceil(interval.useful_energy_mwh_per_quantum);
        let upper = interval
            .max_setpoint_quanta
            .min(previous.saturating_add(scenario.max_slew_quanta_per_interval))
            .min(request_limited_max);
        let mut chosen = None;
        for q in (lower..=upper).rev() {
            let next_stress = stress
                .checked_add(checked_mul(
                    q,
                    interval.asset_stress_units_per_quantum,
                    "$.greedy_baseline",
                )?)
                .ok_or_else(|| fail("$.greedy_baseline", "stress arithmetic overflowed"))?;
            if next_stress > scenario.cumulative_stress_budget_units {
                continue;
            }
            chosen = Some((q, next_stress));
            break;
        }
        let (q, next_stress) = chosen.unwrap_or((0, stress));
        let energy = checked_mul(
            q,
            interval.useful_energy_mwh_per_quantum,
            "$.greedy_baseline",
        )?;
        delivered = delivered.saturating_add(energy).min(scenario.requested_energy_mwh);
        stress = next_stress;
        previous = q;
        setpoints.push(q);
    }
    score_schedule(scenario, &setpoints, "$.greedy_baseline")
}

fn solve_exact(scenario: &Scenario) -> Result<(ScheduleOutcome, u64, usize), String> {
    let mut states = BTreeMap::<(u64, u64), Node>::new();
    states.insert(
        (scenario.initial_setpoint_quanta, 0),
        Node {
            delivered_mwh: 0,
            setpoints: Vec::new(),
        },
    );
    let mut search_nodes = 0_u64;
    let mut peak_states = 1_usize;

    for interval in &scenario.intervals {
        let mut next = BTreeMap::<(u64, u64), Node>::new();
        for ((previous_setpoint, stress_used), node) in &states {
            let lower = previous_setpoint
                .saturating_sub(scenario.max_slew_quanta_per_interval);
            let upper = interval
                .max_setpoint_quanta
                .min(previous_setpoint.saturating_add(scenario.max_slew_quanta_per_interval));

            for setpoint in lower..=upper {
                search_nodes = search_nodes
                    .checked_add(1)
                    .ok_or_else(|| "search node counter overflowed".to_string())?;
                if search_nodes > scenario.max_search_nodes {
                    return Err(fail(
                        "$.policy.maximum_search_nodes",
                        format!(
                            "exact search exceeded {} nodes; no approximate schedule is returned",
                            scenario.max_search_nodes
                        ),
                    ));
                }

                let interval_stress = checked_mul(
                    setpoint,
                    interval.asset_stress_units_per_quantum,
                    "$.intervals.asset_stress_units_per_setpoint_quantum",
                )?;
                let new_stress = stress_used
                    .checked_add(interval_stress)
                    .ok_or_else(|| fail("$.intervals", "cumulative stress overflowed"))?;
                if new_stress > scenario.cumulative_stress_budget_units {
                    continue;
                }

                let interval_energy = checked_mul(
                    setpoint,
                    interval.useful_energy_mwh_per_quantum,
                    "$.intervals.useful_energy_mwh_per_setpoint_quantum",
                )?;
                let new_delivered = node
                    .delivered_mwh
                    .saturating_add(interval_energy)
                    .min(scenario.requested_energy_mwh);
                let mut new_path = node.setpoints.clone();
                new_path.push(setpoint);
                let key = (setpoint, new_stress);
                let replace = next.get(&key).is_none_or(|current| {
                    new_delivered > current.delivered_mwh
                        || (new_delivered == current.delivered_mwh
                            && new_path < current.setpoints)
                });
                if replace {
                    next.insert(
                        key,
                        Node {
                            delivered_mwh: new_delivered,
                            setpoints: new_path,
                        },
                    );
                }
            }
        }
        peak_states = peak_states.max(next.len());
        if next.len() > scenario.max_search_nodes as usize {
            return Err(fail(
                "$.policy.maximum_search_nodes",
                "dynamic-programming state count exceeded the declared exact-search bound",
            ));
        }
        states = next;
        if states.is_empty() {
            break;
        }
    }

    let mut best: Option<((u64, u64), Node)> = None;
    for (key, node) in states {
        let is_service_feasible = node.delivered_mwh >= scenario.minimum_service_mwh;
        let best_is_service_feasible = best.as_ref().is_some_and(|(_, current)| {
            current.delivered_mwh >= scenario.minimum_service_mwh
        });
        if best.is_none()
            || (is_service_feasible && !best_is_service_feasible)
            || (is_service_feasible == best_is_service_feasible
                && (node.delivered_mwh > best.as_ref().unwrap().1.delivered_mwh
                    || (node.delivered_mwh == best.as_ref().unwrap().1.delivered_mwh
                        && (key.1 < best.as_ref().unwrap().0.1
                            || (key.1 == best.as_ref().unwrap().0.1
                                && node.setpoints < best.as_ref().unwrap().1.setpoints)))))
        {
            best = Some((key, node));
        }
    }

    let Some((_key, node)) = best else {
        let report = score_schedule(
            scenario,
            &vec![0; scenario.intervals.len()],
            "$.optimal_schedule",
        )?;
        return Ok((report, search_nodes, peak_states));
    };
    let outcome = score_schedule(scenario, &node.setpoints, "$.optimal_schedule")?;
    Ok((outcome, search_nodes, peak_states))
}

/// Compute the exact bounded optimum, evaluate candidate/fixed/greedy schedules,
/// and report energy gaps. Envelopes are supplied inputs, never inferred here.
pub fn evaluate_scenario(value: &Value) -> Result<TrajectoryControlReport, String> {
    let scenario = parse_scenario(value)?;
    let candidate_controller = score_schedule(
        &scenario,
        &scenario.candidate_setpoints_quanta,
        "$.candidate_controller_setpoints_quanta",
    )?;
    let (optimal, search_nodes, peak_dp_states) = solve_exact(&scenario)?;
    let fixed = fixed_baseline(&scenario)?;
    let greedy = greedy_baseline(&scenario)?;
    let candidate_gap = candidate_controller
        .control_constraints_pass
        .then(|| {
            optimal
                .delivered_useful_energy_mwh
                .saturating_sub(candidate_controller.delivered_useful_energy_mwh)
        });
    let fixed_gap = fixed.as_ref().map(|baseline| {
        optimal
            .delivered_useful_energy_mwh
            .saturating_sub(baseline.delivered_useful_energy_mwh)
    });
    let greedy_gap = greedy.control_constraints_pass.then(|| {
        optimal
            .delivered_useful_energy_mwh
            .saturating_sub(greedy.delivered_useful_energy_mwh)
    });
    let envelope_receipts = scenario
        .intervals
        .iter()
        .map(|interval| (interval.envelope_id.clone(), interval.envelope_sha256.clone()))
        .collect();

    Ok(TrajectoryControlReport {
        scenario_id: scenario.scenario_id,
        trajectory_id: scenario.trajectory_id,
        coordinate_frame_id: scenario.coordinate_frame_id,
        interval_count: scenario.intervals.len(),
        search_nodes,
        peak_dp_states,
        feasible: optimal.feasible,
        optimal,
        candidate_controller,
        fixed_setpoint_baseline: fixed,
        greedy_baseline: greedy,
        candidate_optimality_gap_mwh: candidate_gap,
        fixed_baseline_optimality_gap_mwh: fixed_gap,
        greedy_optimality_gap_mwh: greedy_gap,
        envelope_receipts,
        claim_ceiling: "Exact bounded schedule optimization for synthetic integer setpoint envelopes and an additive declared stress proxy only; no magnetic, nonlinear converter, thermal, pavement, AC, EMC, safety, field-controller or SOTA claim.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-trajectory-control-v0/scenario.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic WPT trajectory control scenario JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn known_truth_exact_dp_beats_candidate_and_fixed_baselines() {
        let report = evaluate_scenario(&fixture()).unwrap();
        assert!(report.feasible);
        assert_eq!(report.optimal.setpoints_quanta, vec![1, 2, 1, 2]);
        assert_eq!(report.optimal.delivered_useful_energy_mwh, 11);
        assert_eq!(report.optimal.cumulative_asset_stress_units, 7);
        assert_eq!(report.candidate_controller.delivered_useful_energy_mwh, 7);
        assert_eq!(report.candidate_optimality_gap_mwh, Some(4));
        assert_eq!(
            report.fixed_setpoint_baseline.as_ref().unwrap().delivered_useful_energy_mwh,
            7
        );
        assert_eq!(report.fixed_baseline_optimality_gap_mwh, Some(4));
        assert_eq!(report.greedy_baseline.delivered_useful_energy_mwh, 9);
        assert_eq!(report.greedy_optimality_gap_mwh, Some(2));
        assert!(report.greedy_baseline.control_constraints_pass);
    }

    #[test]
    fn exact_schedule_never_exceeds_any_sample_envelope_or_slew_limit() {
        let report = evaluate_scenario(&fixture()).unwrap();
        assert!(report.optimal.control_constraints_pass);
        assert!(report.optimal.setpoints_quanta.iter().zip([3, 2, 3, 2]).all(|(q, cap)| *q <= cap));
        let mut previous = 1_u64;
        for setpoint in &report.optimal.setpoints_quanta {
            assert!(setpoint.abs_diff(previous) <= 1);
            previous = *setpoint;
        }
    }

    #[test]
    fn impossible_minimum_service_returns_best_effort_but_not_feasible() {
        let mut scenario = fixture();
        scenario["request"]["minimum_service_energy_mwh"] = Value::from(12);
        let report = evaluate_scenario(&scenario).unwrap();
        assert!(!report.feasible);
        assert!(report.optimal.delivered_useful_energy_mwh < 12);
    }

    #[test]
    fn out_of_domain_or_unsupported_envelope_fails_closed() {
        let mut scenario = fixture();
        scenario["intervals"][1]["support_state"] = Value::String("OUT_OF_DOMAIN".into());
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("unsupported or out-of-domain")
        );
    }

    #[test]
    fn duplicate_trajectory_samples_and_invalid_envelope_digest_are_rejected() {
        let mut scenario = fixture();
        scenario["intervals"][1]["trajectory_sample_id"] =
            scenario["intervals"][0]["trajectory_sample_id"].clone();
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("duplicate trajectory_sample_id")
        );

        let mut scenario = fixture();
        scenario["intervals"][0]["envelope_sha256"] = Value::String("not-a-digest".into());
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("64 lowercase hexadecimal")
        );
    }

    #[test]
    fn excessive_ramp_rate_in_candidate_schedule_is_reported_not_silently_clamped() {
        let mut scenario = fixture();
        scenario["candidate_controller_setpoints_quanta"] =
            Value::Array(vec![Value::from(3), Value::from(0), Value::from(3), Value::from(0)]);
        let report = evaluate_scenario(&scenario).unwrap();
        assert!(!report.candidate_controller.control_constraints_pass);
        assert!(
            report
                .candidate_controller
                .violations
                .iter()
                .any(|violation| violation.contains("slew"))
        );
        assert_eq!(report.candidate_optimality_gap_mwh, None);
    }

    #[test]
    fn stress_budget_is_a_hard_constraint_not_a_weight() {
        let mut scenario = fixture();
        scenario["control_limits"]["cumulative_asset_stress_budget_units"] = Value::from(4);
        let report = evaluate_scenario(&scenario).unwrap();
        assert!(report.optimal.control_constraints_pass);
        assert!(report.optimal.cumulative_asset_stress_units <= 4);
        assert_eq!(report.optimal.delivered_useful_energy_mwh, 7);
        assert!(!report.feasible);
    }

    #[test]
    fn search_bound_exhaustion_returns_error_not_approximate_answer() {
        let mut scenario = fixture();
        scenario["policy"]["maximum_search_nodes"] = Value::from(1);
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("exact search exceeded")
        );
    }

    #[test]
    fn input_envelope_order_is_preserved_with_trajectory_sample_identity() {
        let original = evaluate_scenario(&fixture()).unwrap();
        let mut scenario = fixture();
        scenario["intervals"].as_array_mut().unwrap().reverse();
        scenario["candidate_controller_setpoints_quanta"]
            .as_array_mut()
            .unwrap()
            .reverse();
        let reversed = evaluate_scenario(&scenario).unwrap();
        assert_eq!(original.interval_count, 4);
        assert_eq!(reversed.interval_count, 4);
        assert_eq!(
            original.optimal.interval_results[0].trajectory_sample_id,
            "pose-000"
        );
        assert_eq!(
            reversed.optimal.interval_results[0].trajectory_sample_id,
            "pose-003"
        );
    }

    #[test]
    fn request_bounds_and_non_integer_values_fail_closed() {
        let mut scenario = fixture();
        scenario["request"]["minimum_service_energy_mwh"] = Value::from(13);
        assert!(evaluate_scenario(&scenario).is_err());

        let mut scenario = fixture();
        scenario["intervals"][0]["max_setpoint_quanta"] = Value::from("3");
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("non-negative integer")
        );
    }
}
