use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const VERSION: &str = "0.1.0";
const BENCHMARK_ID: &str = "robotics-persistent-mission-series-v0.1";
const CLAIM_CEILING: &str =
    "mission_series_audit_does_not_establish_physical_reliability_or_safety";
const EVIDENCE_CLASSES: &[&str] = &[
    "synthetic_contract_fixture",
    "simulator",
    "hardware_in_loop",
    "physical_field",
    "independently_reviewed",
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Dataset {
    schema_version: String,
    benchmark_id: String,
    claim_ceiling: String,
    series: Vec<MissionSeries>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MissionSeries {
    series_id: String,
    comparison_group_id: String,
    scenario_family: String,
    policy_id: String,
    role: String,
    seed: u64,
    evidence_class: String,
    battery_capacity_wh: f64,
    initial_energy_wh: f64,
    energy_tolerance_wh: f64,
    initial_state_epoch: u64,
    initial_state_token: String,
    missions: Vec<Mission>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Mission {
    sequence_index: u32,
    mission_id: String,
    mission_success: bool,
    useful_work_units: f64,
    battery_start_wh: f64,
    energy_used_wh: f64,
    energy_charged_wh: f64,
    battery_end_wh: f64,
    elapsed_minutes: f64,
    recovery_minutes: f64,
    maintenance_minutes: f64,
    operator_interventions: u64,
    safety_violation: bool,
    reset_occurred: bool,
    reset_authorized: bool,
    reset_reason: String,
    state_epoch: u64,
    prior_state_token: String,
    state_token: String,
}

#[derive(Clone, Debug, Serialize)]
struct SeriesMetrics {
    series_id: String,
    comparison_group_id: String,
    scenario_family: String,
    policy_id: String,
    role: String,
    evidence_class: String,
    mission_count: usize,
    completed_missions: usize,
    completion_rate: f64,
    total_useful_work_units: f64,
    total_energy_used_wh: f64,
    useful_work_per_wh: Option<f64>,
    recovery_minutes_total: f64,
    maintenance_minutes_total: f64,
    operator_interventions_total: u64,
    authorized_reset_count: u64,
    unauthorized_reset_count: u64,
    safety_violation_count: u64,
    end_energy_reserve_fraction: f64,
    useful_work_slope_units_per_mission: f64,
    safety_gate_pass: bool,
    findings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ComparisonMetrics {
    comparison_group_id: String,
    scenario_family: String,
    seed: u64,
    mission_count: usize,
    candidate_minus_baseline_completion_rate: f64,
    candidate_minus_baseline_useful_work_per_wh: Option<f64>,
    candidate_minus_baseline_recovery_minutes: f64,
    candidate_minus_baseline_maintenance_minutes: f64,
    candidate_minus_baseline_operator_interventions: i64,
    candidate_minus_baseline_end_reserve_fraction: f64,
    candidate_minus_baseline_useful_work_slope: f64,
    candidate_minus_baseline_authorized_resets: i64,
}

#[derive(Clone, Debug, Serialize)]
struct Summary {
    series_count: usize,
    comparison_group_count: usize,
    mission_count: usize,
    scenario_family_count: usize,
    safety_violation_count: u64,
    unauthorized_reset_count: u64,
    safety_gate_failure_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct Receipt {
    audit_version: String,
    result: String,
    audit_integrity_pass: bool,
    safety_gate_pass: bool,
    benchmark_id: String,
    evaluator_source_commit: String,
    input_sha256: String,
    checks_performed: Vec<String>,
    summary: Summary,
    series: Vec<SeriesMetrics>,
    comparisons: Vec<ComparisonMetrics>,
    errors: Vec<String>,
    claim_ceiling: String,
}

fn allowed(value: &str, accepted: &[&str]) -> bool {
    accepted.contains(&value)
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite_nonnegative(name: &str, value: f64, errors: &mut Vec<String>) {
    if !value.is_finite() || value < 0.0 {
        errors.push(format!("{name} must be finite and non-negative"));
    }
}

fn close_enough(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance
}

fn slope(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let n = values.len() as f64;
    let x_mean = (n - 1.0) / 2.0;
    let y_mean = values.iter().sum::<f64>() / n;
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for (index, value) in values.iter().enumerate() {
        let x = index as f64;
        numerator += (x - x_mean) * (*value - y_mean);
        denominator += (x - x_mean).powi(2);
    }
    if denominator <= f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

fn audit_series(series: &MissionSeries) -> (SeriesMetrics, Vec<String>) {
    let mut errors = Vec::new();
    let mut findings = Vec::new();
    if series.series_id.trim().is_empty()
        || series.comparison_group_id.trim().is_empty()
        || series.scenario_family.trim().is_empty()
        || series.policy_id.trim().is_empty()
    {
        errors.push("series metadata must be non-empty".to_string());
    }
    if !allowed(&series.evidence_class, EVIDENCE_CLASSES) {
        errors.push(format!("{}: invalid evidence_class", series.series_id));
    }
    if series.role != "candidate" && series.role != "conservative_baseline" {
        errors.push(format!("{}: role must be candidate or conservative_baseline", series.series_id));
    }
    finite_nonnegative("battery_capacity_wh", series.battery_capacity_wh, &mut errors);
    finite_nonnegative("initial_energy_wh", series.initial_energy_wh, &mut errors);
    finite_nonnegative("energy_tolerance_wh", series.energy_tolerance_wh, &mut errors);
    if series.battery_capacity_wh <= 0.0 {
        errors.push(format!("{}: battery capacity must be positive", series.series_id));
    }
    if series.energy_tolerance_wh <= 0.0 {
        errors.push(format!("{}: energy tolerance must be positive", series.series_id));
    }
    if series.initial_energy_wh > series.battery_capacity_wh + series.energy_tolerance_wh {
        errors.push(format!("{}: initial energy exceeds capacity", series.series_id));
    }
    if series.initial_state_token.trim().is_empty() {
        errors.push(format!("{}: initial state token is empty", series.series_id));
    }
    if series.missions.len() < 4 {
        errors.push(format!("{}: at least four missions are required", series.series_id));
    }

    let mut expected_energy = series.initial_energy_wh;
    let mut expected_epoch = series.initial_state_epoch;
    let mut expected_token = series.initial_state_token.as_str();
    let mut work_values = Vec::with_capacity(series.missions.len());
    let mut total_work = 0.0;
    let mut total_energy = 0.0;
    let mut recovery_total = 0.0;
    let mut maintenance_total = 0.0;
    let mut interventions_total = 0_u64;
    let mut completed = 0_usize;
    let mut authorized_resets = 0_u64;
    let mut unauthorized_resets = 0_u64;
    let mut safety_violations = 0_u64;
    let mut seen_mission_ids = BTreeSet::new();

    for (index, mission) in series.missions.iter().enumerate() {
        let label = format!("{}: mission {}", series.series_id, mission.sequence_index);
        let expected_index = index as u32 + 1;
        if mission.sequence_index != expected_index {
            errors.push(format!("{label}: sequence_index must be contiguous from 1"));
        }
        if mission.mission_id.trim().is_empty() || !seen_mission_ids.insert(mission.mission_id.clone()) {
            errors.push(format!("{label}: empty or duplicate mission_id"));
        }
        if mission.prior_state_token != expected_token {
            errors.push(format!("{label}: prior_state_token does not link to previous state"));
        }
        if mission.state_token.trim().is_empty() {
            errors.push(format!("{label}: state_token is empty"));
        }
        for (name, value) in [
            ("useful_work_units", mission.useful_work_units),
            ("battery_start_wh", mission.battery_start_wh),
            ("energy_used_wh", mission.energy_used_wh),
            ("energy_charged_wh", mission.energy_charged_wh),
            ("battery_end_wh", mission.battery_end_wh),
            ("elapsed_minutes", mission.elapsed_minutes),
            ("recovery_minutes", mission.recovery_minutes),
            ("maintenance_minutes", mission.maintenance_minutes),
        ] {
            finite_nonnegative(&format!("{label}: {name}"), value, &mut errors);
        }
        if !close_enough(
            mission.battery_start_wh,
            expected_energy,
            series.energy_tolerance_wh,
        ) {
            errors.push(format!("{label}: battery start does not carry over previous end state"));
        }
        let accounted_end =
            mission.battery_start_wh + mission.energy_charged_wh - mission.energy_used_wh;
        if !close_enough(
            mission.battery_end_wh,
            accounted_end,
            series.energy_tolerance_wh,
        ) {
            errors.push(format!("{label}: energy balance does not reconcile within tolerance"));
        }
        if mission.battery_start_wh > series.battery_capacity_wh + series.energy_tolerance_wh
            || mission.battery_end_wh > series.battery_capacity_wh + series.energy_tolerance_wh
        {
            errors.push(format!("{label}: battery energy exceeds declared capacity"));
        }
        if mission.battery_end_wh < 0.0 {
            errors.push(format!("{label}: battery energy falls below zero"));
        }

        if mission.reset_occurred {
            if mission.reset_authorized {
                authorized_resets += 1;
                if mission.reset_reason.trim().is_empty() {
                    errors.push(format!("{label}: authorized reset requires a reason"));
                }
            } else {
                unauthorized_resets += 1;
                findings.push(format!("{label}: reset occurred without explicit authorization"));
            }
            let expected_reset_epoch = expected_epoch.saturating_add(1);
            if mission.state_epoch != expected_reset_epoch {
                errors.push(format!("{label}: reset must increment state_epoch exactly once"));
            }
        } else {
            if mission.reset_authorized || !mission.reset_reason.is_empty() {
                errors.push(format!("{label}: reset authorization/reason present without a reset"));
            }
            if mission.state_epoch != expected_epoch {
                errors.push(format!("{label}: unexplained state_epoch change without a reset"));
            }
        }
        expected_epoch = mission.state_epoch;
        expected_token = mission.state_token.as_str();
        expected_energy = mission.battery_end_wh;

        if mission.safety_violation {
            safety_violations += 1;
            findings.push(format!("{label}: declared safety violation"));
        }
        if mission.mission_success {
            completed += 1;
        }
        work_values.push(mission.useful_work_units);
        total_work += mission.useful_work_units;
        total_energy += mission.energy_used_wh;
        recovery_total += mission.recovery_minutes;
        maintenance_total += mission.maintenance_minutes;
        interventions_total += mission.operator_interventions;
    }

    findings.sort();
    errors.sort();
    let completion_rate = if series.missions.is_empty() {
        0.0
    } else {
        completed as f64 / series.missions.len() as f64
    };
    let reserve = if series.battery_capacity_wh > 0.0 {
        expected_energy / series.battery_capacity_wh
    } else {
        0.0
    };
    let metrics = SeriesMetrics {
        series_id: series.series_id.clone(),
        comparison_group_id: series.comparison_group_id.clone(),
        scenario_family: series.scenario_family.clone(),
        policy_id: series.policy_id.clone(),
        role: series.role.clone(),
        evidence_class: series.evidence_class.clone(),
        mission_count: series.missions.len(),
        completed_missions: completed,
        completion_rate,
        total_useful_work_units: total_work,
        total_energy_used_wh: total_energy,
        useful_work_per_wh: if total_energy > f64::EPSILON {
            Some(total_work / total_energy)
        } else {
            None
        },
        recovery_minutes_total: recovery_total,
        maintenance_minutes_total: maintenance_total,
        operator_interventions_total: interventions_total,
        authorized_reset_count: authorized_resets,
        unauthorized_reset_count: unauthorized_resets,
        safety_violation_count: safety_violations,
        end_energy_reserve_fraction: reserve,
        useful_work_slope_units_per_mission: slope(&work_values),
        safety_gate_pass: findings.is_empty(),
        findings,
    };
    (metrics, errors)
}

fn pairs_match(baseline: &MissionSeries, candidate: &MissionSeries) -> Result<(), String> {
    if baseline.scenario_family != candidate.scenario_family
        || baseline.seed != candidate.seed
        || !close_enough(
            baseline.battery_capacity_wh,
            candidate.battery_capacity_wh,
            f64::EPSILON,
        )
        || !close_enough(
            baseline.initial_energy_wh,
            candidate.initial_energy_wh,
            f64::EPSILON,
        )
        || baseline.initial_state_epoch != candidate.initial_state_epoch
        || baseline.initial_state_token != candidate.initial_state_token
        || baseline.missions.len() != candidate.missions.len()
    {
        return Err(format!(
            "{}: baseline and candidate do not share matched initial conditions",
            baseline.comparison_group_id
        ));
    }
    for (base, cand) in baseline.missions.iter().zip(candidate.missions.iter()) {
        if base.sequence_index != cand.sequence_index || base.mission_id != cand.mission_id {
            return Err(format!(
                "{}: baseline and candidate mission schedules differ",
                baseline.comparison_group_id
            ));
        }
    }
    Ok(())
}

fn comparison(
    group_id: &str,
    baseline: &SeriesMetrics,
    candidate: &SeriesMetrics,
    source: &MissionSeries,
) -> ComparisonMetrics {
    ComparisonMetrics {
        comparison_group_id: group_id.to_string(),
        scenario_family: source.scenario_family.clone(),
        seed: source.seed,
        mission_count: baseline.mission_count,
        candidate_minus_baseline_completion_rate: candidate.completion_rate - baseline.completion_rate,
        candidate_minus_baseline_useful_work_per_wh: match (
            candidate.useful_work_per_wh,
            baseline.useful_work_per_wh,
        ) {
            (Some(c), Some(b)) => Some(c - b),
            _ => None,
        },
        candidate_minus_baseline_recovery_minutes:
            candidate.recovery_minutes_total - baseline.recovery_minutes_total,
        candidate_minus_baseline_maintenance_minutes:
            candidate.maintenance_minutes_total - baseline.maintenance_minutes_total,
        candidate_minus_baseline_operator_interventions:
            candidate.operator_interventions_total as i64 - baseline.operator_interventions_total as i64,
        candidate_minus_baseline_end_reserve_fraction:
            candidate.end_energy_reserve_fraction - baseline.end_energy_reserve_fraction,
        candidate_minus_baseline_useful_work_slope:
            candidate.useful_work_slope_units_per_mission - baseline.useful_work_slope_units_per_mission,
        candidate_minus_baseline_authorized_resets:
            candidate.authorized_reset_count as i64 - baseline.authorized_reset_count as i64,
    }
}

fn build_receipt(raw: &str, source_commit: &str) -> Result<Receipt, String> {
    if !valid_commit(source_commit) {
        return Err("source commit must be 40 lowercase hexadecimal characters".to_string());
    }
    let dataset: Dataset =
        serde_json::from_str(raw).map_err(|error| format!("invalid dataset JSON: {error}"))?;
    let input_sha256 = Sha256::digest(raw.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut errors = Vec::new();
    if dataset.schema_version != "0.1" {
        errors.push("schema_version must be 0.1".to_string());
    }
    if dataset.benchmark_id != BENCHMARK_ID {
        errors.push("unexpected benchmark_id".to_string());
    }
    if dataset.claim_ceiling != CLAIM_CEILING {
        errors.push("claim ceiling is missing or weakened".to_string());
    }
    if dataset.series.len() < 8 {
        errors.push("at least four paired comparison groups (eight series) are required".to_string());
    }

    let mut series_ids = BTreeSet::new();
    let mut families = BTreeSet::new();
    for series in &dataset.series {
        if !series_ids.insert(series.series_id.clone()) {
            errors.push(format!("duplicate series_id: {}", series.series_id));
        }
        families.insert(series.scenario_family.clone());
    }
    if families.len() < 4 {
        errors.push("at least four distinct scenario families are required".to_string());
    }

    let mut grouped: BTreeMap<String, Vec<&MissionSeries>> = BTreeMap::new();
    for series in &dataset.series {
        grouped.entry(series.comparison_group_id.clone()).or_default().push(series);
    }
    let mut metrics = Vec::with_capacity(dataset.series.len());
    for series in &dataset.series {
        let (row, mut row_errors) = audit_series(series);
        errors.append(&mut row_errors);
        metrics.push(row);
    }

    let mut comparisons = Vec::new();
    for (group_id, members) in &grouped {
        if members.len() != 2 {
            errors.push(format!("{group_id}: exactly one baseline and one candidate are required"));
            continue;
        }
        let baseline = members.iter().copied().find(|s| s.role == "conservative_baseline");
        let candidate = members.iter().copied().find(|s| s.role == "candidate");
        let (Some(baseline), Some(candidate)) = (baseline, candidate) else {
            errors.push(format!("{group_id}: missing baseline or candidate role"));
            continue;
        };
        if let Err(error) = pairs_match(baseline, candidate) {
            errors.push(error);
            continue;
        }
        let base_metrics = metrics.iter().find(|m| m.series_id == baseline.series_id);
        let candidate_metrics = metrics.iter().find(|m| m.series_id == candidate.series_id);
        if let (Some(base_metrics), Some(candidate_metrics)) = (base_metrics, candidate_metrics) {
            comparisons.push(comparison(group_id, base_metrics, candidate_metrics, baseline));
        }
    }
    comparisons.sort_by(|a, b| a.comparison_group_id.cmp(&b.comparison_group_id));
    errors.sort();

    let mission_count: usize = metrics.iter().map(|row| row.mission_count).sum();
    let safety_violation_count: u64 = metrics.iter().map(|row| row.safety_violation_count).sum();
    let unauthorized_reset_count: u64 = metrics.iter().map(|row| row.unauthorized_reset_count).sum();
    let safety_gate_failure_count = metrics.iter().filter(|row| !row.safety_gate_pass).count();
    let audit_integrity_pass = errors.is_empty();
    let safety_gate_pass = safety_gate_failure_count == 0;
    let result = if audit_integrity_pass && safety_gate_pass {
        "pass"
    } else {
        "fail"
    };
    let summary = Summary {
        series_count: dataset.series.len(),
        comparison_group_count: comparisons.len(),
        mission_count,
        scenario_family_count: families.len(),
        safety_violation_count,
        unauthorized_reset_count,
        safety_gate_failure_count,
    };

    Ok(Receipt {
        audit_version: VERSION.to_string(),
        result: result.to_string(),
        audit_integrity_pass,
        safety_gate_pass,
        benchmark_id: dataset.benchmark_id,
        evaluator_source_commit: source_commit.to_string(),
        input_sha256,
        checks_performed: vec![
            "exact input SHA-256 and evaluator source commit".to_string(),
            "matched baseline/candidate scenario, seed, initial state and mission schedule".to_string(),
            "contiguous mission indices and prior/current state-token linkage".to_string(),
            "battery carryover, capacity limits and energy balance within declared tolerance".to_string(),
            "explicit reset authorization and one-step state-epoch transitions".to_string(),
            "safety violations and unauthorized resets as hard gates".to_string(),
            "per-series efficiency, reserve, recovery, maintenance, intervention and degradation metrics".to_string(),
            "candidate-minus-baseline comparisons without a blended scalar".to_string(),
        ],
        summary,
        series: metrics,
        comparisons,
        errors,
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: robotics-mission-series-audit <dataset.json> --source-commit <40-hex-sha>");
        return ExitCode::FAILURE;
    };
    let Some(flag) = args.next() else {
        eprintln!("missing --source-commit");
        return ExitCode::FAILURE;
    };
    let Some(commit) = args.next() else {
        eprintln!("missing source commit");
        return ExitCode::FAILURE;
    };
    if flag != "--source-commit" || args.next().is_some() {
        eprintln!("usage: robotics-mission-series-audit <dataset.json> --source-commit <40-hex-sha>");
        return ExitCode::FAILURE;
    }
    let raw = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let receipt = match build_receipt(&raw, &commit) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mission-series audit: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", serde_json::to_string_pretty(&receipt).expect("receipt serializes"));
    if receipt.result == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../fixtures/robotics/mission_series_v0_1.json");
    const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn receipt() -> Receipt {
        build_receipt(FIXTURE, COMMIT).expect("valid fixture receipt")
    }

    #[test]
    fn fixture_passes_integrity_and_hard_gates() {
        let report = receipt();
        assert_eq!(report.result, "pass", "{:?}", report.errors);
        assert_eq!(report.summary.series_count, 8);
        assert_eq!(report.summary.comparison_group_count, 4);
        assert_eq!(report.summary.mission_count, 32);
        assert_eq!(report.summary.scenario_family_count, 4);
        assert_eq!(report.summary.safety_violation_count, 0);
        assert_eq!(report.summary.unauthorized_reset_count, 0);
    }

    #[test]
    fn broken_state_chain_fails_integrity() {
        let mut data: Dataset = serde_json::from_str(FIXTURE).unwrap();
        data.series[0].missions[1].prior_state_token = "broken-link".to_string();
        let raw = serde_json::to_string(&data).unwrap();
        let report = build_receipt(&raw, COMMIT).unwrap();
        assert!(!report.audit_integrity_pass);
        assert_eq!(report.result, "fail");
    }

    #[test]
    fn energy_accounting_mismatch_fails_integrity() {
        let mut data: Dataset = serde_json::from_str(FIXTURE).unwrap();
        data.series[0].missions[0].battery_end_wh += 2.0;
        let raw = serde_json::to_string(&data).unwrap();
        let report = build_receipt(&raw, COMMIT).unwrap();
        assert!(!report.audit_integrity_pass);
    }

    #[test]
    fn unauthorized_reset_fails_safety_gate_even_when_logged() {
        let mut data: Dataset = serde_json::from_str(FIXTURE).unwrap();
        let mission = &mut data.series[0].missions[1];
        mission.reset_occurred = true;
        mission.reset_authorized = false;
        mission.reset_reason = "unplanned restart".to_string();
        mission.state_epoch = 1;
        let raw = serde_json::to_string(&data).unwrap();
        let report = build_receipt(&raw, COMMIT).unwrap();
        assert!(!report.safety_gate_pass);
        assert_eq!(report.summary.unauthorized_reset_count, 1);
    }

    #[test]
    fn declared_safety_violation_fails_safety_gate() {
        let mut data: Dataset = serde_json::from_str(FIXTURE).unwrap();
        data.series[2].missions[3].safety_violation = true;
        let raw = serde_json::to_string(&data).unwrap();
        let report = build_receipt(&raw, COMMIT).unwrap();
        assert!(!report.safety_gate_pass);
    }

    #[test]
    fn authorized_reset_is_visible_and_epoch_bounded() {
        let mut data: Dataset = serde_json::from_str(FIXTURE).unwrap();
        let mission = &mut data.series[0].missions[1];
        mission.reset_occurred = true;
        mission.reset_authorized = true;
        mission.reset_reason = "planned maintenance".to_string();
        mission.maintenance_minutes = 12.0;
        mission.state_epoch = 1;
        data.series[0].missions[2].state_epoch = 1;
        data.series[0].missions[3].state_epoch = 1;
        let raw = serde_json::to_string(&data).unwrap();
        let report = build_receipt(&raw, COMMIT).unwrap();
        assert!(report.audit_integrity_pass, "{:?}", report.errors);
        assert!(report.safety_gate_pass, "{:?}", report.series[0].findings);
        assert_eq!(report.series[0].authorized_reset_count, 1);
        assert_eq!(report.series[0].maintenance_minutes_total, 12.0);
    }

    #[test]
    fn candidate_baseline_pair_mismatch_fails_integrity() {
        let mut data: Dataset = serde_json::from_str(FIXTURE).unwrap();
        data.series[1].missions[1].mission_id = "wrong-mission".to_string();
        let raw = serde_json::to_string(&data).unwrap();
        let report = build_receipt(&raw, COMMIT).unwrap();
        assert!(!report.audit_integrity_pass);
    }

    #[test]
    fn comparison_reports_work_per_energy_and_recovery_deltas() {
        let report = receipt();
        let rough = report
            .comparisons
            .iter()
            .find(|row| row.scenario_family == "rough-terrain-courier")
            .unwrap();
        assert!(rough.candidate_minus_baseline_useful_work_per_wh.unwrap() > 0.0);
        assert!(rough.candidate_minus_baseline_recovery_minutes < 0.0);
    }

    #[test]
    fn invalid_evaluator_commit_is_rejected() {
        assert!(build_receipt(FIXTURE, "bad-sha").is_err());
    }
}
