// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

const MAX_INPUT_BYTES: u64 = 4 * 1024 * 1024;
const MIN_SESSIONS_PER_PERIOD: usize = 5;
const MAX_SESSIONS_PER_PERIOD: usize = 100;
const MIN_REPEATS_PER_TARGET: u32 = 6;
const MAX_TARGETS: usize = 128;
const MIN_PERMUTATIONS: usize = 999;
const MAX_PERMUTATIONS: usize = 100_000;
const CLAIM_CEILING: &str = "Longitudinal repeatability/precision change on a pinned synthetic control setup only; absolute accuracy, calibration, registration validity and engineering task fitness are not evaluated.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: String,
    monitor_id: String,
    sensor: SensorIdentity,
    setup: ControlSetup,
    alpha: f64,
    permutation_count: usize,
    seed: u64,
    baseline_sessions: Vec<Session>,
    current_sessions: Vec<Session>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SensorIdentity {
    manufacturer: String,
    model: String,
    unit_id: String,
    firmware_version: String,
    profile_id: String,
    profile_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlSetup {
    artifact_id: String,
    artifact_reference_sha256: String,
    acquisition_config_sha256: String,
    coordinate_frame_id: String,
    range_stratum: String,
    incidence_stratum: String,
    environment_stratum: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Session {
    session_id: String,
    captured_at_utc: String,
    targets: Vec<TargetPrecision>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetPrecision {
    target_id: String,
    repeat_count: u32,
    within_session_standard_deviation_mm: f64,
}

#[derive(Debug, Serialize)]
struct TargetResult {
    target_id: String,
    baseline_median_standard_deviation_mm: f64,
    current_median_standard_deviation_mm: f64,
    standard_deviation_ratio_testing_over_baseline: f64,
    direction: String,
}

#[derive(Debug, Serialize)]
struct HealthReport {
    schema_version: String,
    monitor_id: String,
    sensor_unit_id: String,
    profile_id: String,
    profile_sha256: String,
    artifact_id: String,
    artifact_reference_sha256: String,
    baseline_session_count: usize,
    current_session_count: usize,
    target_count: usize,
    alpha: f64,
    permutation_count: usize,
    exceedance_count: usize,
    p_value: f64,
    observed_max_abs_log_standard_deviation_ratio: f64,
    standard_deviation_ratio_testing_over_baseline: Vec<f64>,
    target_results: Vec<TargetResult>,
    disposition: String,
    precision_direction: String,
    accuracy_disposition: String,
    task_fitness_disposition: String,
    statistical_method: String,
    input_sha256: String,
    claim_ceiling: String,
}

#[derive(Debug, Clone)]
struct CanonicalSession {
    session_id: String,
    is_baseline: bool,
    values_mm: Vec<f64>,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn validate_non_empty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} must be non-empty"))
    } else {
        Ok(())
    }
}

fn validate_input(input: &Input) -> Result<(Vec<String>, Vec<CanonicalSession>, usize), String> {
    if input.schema_version != "0.1.0" {
        return Err("unsupported schema_version".to_owned());
    }
    validate_non_empty(&input.monitor_id, "monitor_id")?;
    validate_non_empty(&input.sensor.manufacturer, "sensor.manufacturer")?;
    validate_non_empty(&input.sensor.model, "sensor.model")?;
    validate_non_empty(&input.sensor.unit_id, "sensor.unit_id")?;
    validate_non_empty(&input.sensor.firmware_version, "sensor.firmware_version")?;
    validate_non_empty(&input.sensor.profile_id, "sensor.profile_id")?;
    if !is_sha256(&input.sensor.profile_sha256) {
        return Err("sensor.profile_sha256 must be a lowercase SHA-256 digest".to_owned());
    }

    validate_non_empty(&input.setup.artifact_id, "setup.artifact_id")?;
    for (value, label) in [
        (
            input.setup.artifact_reference_sha256.as_str(),
            "setup.artifact_reference_sha256",
        ),
        (
            input.setup.acquisition_config_sha256.as_str(),
            "setup.acquisition_config_sha256",
        ),
    ] {
        if !is_sha256(value) {
            return Err(format!("{label} must be a lowercase SHA-256 digest"));
        }
    }
    validate_non_empty(&input.setup.coordinate_frame_id, "setup.coordinate_frame_id")?;
    validate_non_empty(&input.setup.range_stratum, "setup.range_stratum")?;
    validate_non_empty(&input.setup.incidence_stratum, "setup.incidence_stratum")?;
    validate_non_empty(&input.setup.environment_stratum, "setup.environment_stratum")?;

    if !input.alpha.is_finite() || !(0.001..=0.2).contains(&input.alpha) {
        return Err("alpha must be finite and within [0.001, 0.2]".to_owned());
    }
    if !(MIN_PERMUTATIONS..=MAX_PERMUTATIONS).contains(&input.permutation_count) {
        return Err(format!(
            "permutation_count must be within {MIN_PERMUTATIONS}..={MAX_PERMUTATIONS}"
        ));
    }
    if input.seed == 0 {
        return Err("seed must be non-zero".to_owned());
    }
    for (sessions, label) in [
        (input.baseline_sessions.as_slice(), "baseline_sessions"),
        (input.current_sessions.as_slice(), "current_sessions"),
    ] {
        if !(MIN_SESSIONS_PER_PERIOD..=MAX_SESSIONS_PER_PERIOD).contains(&sessions.len()) {
            return Err(format!(
                "{label} must contain {MIN_SESSIONS_PER_PERIOD}..={MAX_SESSIONS_PER_PERIOD} independent sessions"
            ));
        }
    }

    let mut session_ids = BTreeSet::new();
    let mut target_ids = BTreeSet::new();
    let mut canonical = Vec::with_capacity(
        input.baseline_sessions.len() + input.current_sessions.len(),
    );

    for (sessions, label, is_baseline) in [
        (input.baseline_sessions.as_slice(), "baseline", true),
        (input.current_sessions.as_slice(), "current", false),
    ] {
        for session in sessions {
            validate_non_empty(&session.session_id, "session_id")?;
            validate_non_empty(&session.captured_at_utc, "captured_at_utc")?;
            if !session_ids.insert(session.session_id.as_str()) {
                return Err(format!("duplicate session_id: {}", session.session_id));
            }
            if session.targets.is_empty() || session.targets.len() > MAX_TARGETS {
                return Err(format!(
                    "{label} session {} must contain 1..={MAX_TARGETS} targets",
                    session.session_id
                ));
            }
            let mut per_session = BTreeMap::new();
            for target in &session.targets {
                validate_non_empty(&target.target_id, "target_id")?;
                if target.repeat_count < MIN_REPEATS_PER_TARGET {
                    return Err(format!(
                        "target {} in session {} has fewer than {MIN_REPEATS_PER_TARGET} repeats",
                        target.target_id, session.session_id
                    ));
                }
                let sigma = target.within_session_standard_deviation_mm;
                if !sigma.is_finite() || sigma <= 0.0 {
                    return Err(format!(
                        "target {} in session {} has a non-positive or non-finite standard deviation",
                        target.target_id, session.session_id
                    ));
                }
                if per_session
                    .insert(target.target_id.as_str(), sigma)
                    .is_some()
                {
                    return Err(format!(
                        "duplicate target {} in session {}",
                        target.target_id, session.session_id
                    ));
                }
                target_ids.insert(target.target_id.clone());
            }
            canonical.push((session.session_id.clone(), per_session, is_baseline));
        }
    }

    if target_ids.is_empty() || target_ids.len() > MAX_TARGETS {
        return Err(format!("target count must be within 1..={MAX_TARGETS}"));
    }
    let target_ids: Vec<String> = target_ids.into_iter().collect();
    let expected: BTreeSet<&str> = target_ids.iter().map(String::as_str).collect();
    let mut sessions = Vec::with_capacity(canonical.len());

    for (session_id, values, is_baseline) in canonical {
        let observed: BTreeSet<&str> = values.keys().copied().collect();
        if observed != expected {
            return Err(format!(
                "session {session_id} does not contain the exact common target set"
            ));
        }
        let values_mm = target_ids
            .iter()
            .map(|target_id| values[target_id.as_str()])
            .collect();
        sessions.push(CanonicalSession {
            session_id,
            is_baseline,
            values_mm,
        });
    }
    sessions.sort_by(|left, right| left.session_id.cmp(&right.session_id));

    Ok((
        target_ids,
        sessions,
        input.baseline_sessions.len(),
    ))
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn max_log_ratio(
    sessions: &[CanonicalSession],
    baseline_indices: &[usize],
    current_indices: &[usize],
    target_count: usize,
) -> (f64, Vec<(f64, f64, f64)>) {
    let mut maximum = 0.0_f64;
    let mut summaries = Vec::with_capacity(target_count);
    for target_index in 0..target_count {
        let mut baseline_values: Vec<f64> = baseline_indices
            .iter()
            .map(|index| sessions[*index].values_mm[target_index])
            .collect();
        let mut current_values: Vec<f64> = current_indices
            .iter()
            .map(|index| sessions[*index].values_mm[target_index])
            .collect();
        let baseline_median = median(&mut baseline_values);
        let current_median = median(&mut current_values);
        let ratio = current_median / baseline_median;
        maximum = maximum.max(ratio.ln().abs());
        summaries.push((baseline_median, current_median, ratio));
    }
    (maximum, summaries)
}

#[derive(Debug)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }
}

fn permutation_p_value(
    sessions: &[CanonicalSession],
    baseline_count: usize,
    target_count: usize,
    observed_statistic: f64,
    permutation_count: usize,
    seed: u64,
) -> (usize, f64) {
    let mut rng = XorShift64::new(seed);
    let mut order: Vec<usize> = (0..sessions.len()).collect();
    let mut exceedances = 0_usize;

    for _ in 0..permutation_count {
        for index in (1..order.len()).rev() {
            let swap_index = (rng.next_u64() % (index as u64 + 1)) as usize;
            order.swap(index, swap_index);
        }
        let (baseline_indices, current_indices) = order.split_at(baseline_count);
        let (statistic, _) = max_log_ratio(
            sessions,
            baseline_indices,
            current_indices,
            target_count,
        );
        if statistic >= observed_statistic - 1e-12 {
            exceedances += 1;
        }
    }
    let p_value =
        (exceedances as f64 + 1.0) / (permutation_count as f64 + 1.0);
    (exceedances, p_value)
}

fn evaluate_bytes(bytes: &[u8]) -> Result<HealthReport, Box<dyn Error>> {
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("input exceeds the 4 MiB monitor budget".into());
    }
    let input: Input = serde_json::from_slice(bytes)?;
    let (target_ids, sessions, baseline_count) = validate_input(&input)?;
    let current_count = sessions.len() - baseline_count;
    let baseline_indices: Vec<usize> = sessions
        .iter()
        .enumerate()
        .filter_map(|(index, session)| session.is_baseline.then_some(index))
        .collect();
    let current_indices: Vec<usize> = sessions
        .iter()
        .enumerate()
        .filter_map(|(index, session)| (!session.is_baseline).then_some(index))
        .collect();
    let (observed_statistic, summaries) = max_log_ratio(
        &sessions,
        &baseline_indices,
        &current_indices,
        target_ids.len(),
    );
    let (exceedance_count, p_value) = permutation_p_value(
        &sessions,
        baseline_count,
        target_ids.len(),
        observed_statistic,
        input.permutation_count,
        input.seed,
    );

    let target_results: Vec<TargetResult> = target_ids
        .iter()
        .zip(summaries.iter())
        .map(|(target_id, (baseline_median, current_median, ratio))| TargetResult {
            target_id: target_id.clone(),
            baseline_median_standard_deviation_mm: *baseline_median,
            current_median_standard_deviation_mm: *current_median,
            standard_deviation_ratio_testing_over_baseline: *ratio,
            direction: if *ratio > 1.0 {
                "degraded".to_owned()
            } else if *ratio < 1.0 {
                "improved".to_owned()
            } else {
                "unchanged".to_owned()
            },
        })
        .collect();
    let ratios: Vec<f64> = target_results
        .iter()
        .map(|target| target.standard_deviation_ratio_testing_over_baseline)
        .collect();
    let changed = p_value < input.alpha;
    let precision_direction = if !changed {
        "no_significant_change".to_owned()
    } else if ratios.iter().any(|ratio| *ratio > 1.0) {
        "degraded".to_owned()
    } else {
        "improved".to_owned()
    };

    Ok(HealthReport {
        schema_version: input.schema_version,
        monitor_id: input.monitor_id,
        sensor_unit_id: input.sensor.unit_id,
        profile_id: input.sensor.profile_id,
        profile_sha256: input.sensor.profile_sha256,
        artifact_id: input.setup.artifact_id,
        artifact_reference_sha256: input.setup.artifact_reference_sha256,
        baseline_session_count: baseline_count,
        current_session_count: current_count,
        target_count: target_ids.len(),
        alpha: input.alpha,
        permutation_count: input.permutation_count,
        exceedance_count,
        p_value,
        observed_max_abs_log_standard_deviation_ratio: observed_statistic,
        standard_deviation_ratio_testing_over_baseline: ratios,
        target_results,
        disposition: if changed {
            "SIGNIFICANT_PRECISION_CHANGE_DETECTED".to_owned()
        } else {
            "NO_SIGNIFICANT_PRECISION_CHANGE_DETECTED".to_owned()
        },
        precision_direction,
        accuracy_disposition: "NOT_EVALUATED".to_owned(),
        task_fitness_disposition: "NOT_EVALUATED".to_owned(),
        statistical_method: "session_label_permutation_max_abs_log_median_standard_deviation_ratio".to_owned(),
        input_sha256: sha256_hex(bytes),
        claim_ceiling: CLAIM_CEILING.to_owned(),
    })
}

fn parse_input_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut input = PathBuf::from(
        "tools/scan-bim-tls-health-monitor/fixtures/no-change.json",
    );
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => {
                input = args.next().ok_or("--input requires a path")?.into();
            }
            "--help" | "-h" => {
                println!("scan-bim-tls-health-monitor [--input PATH]");
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}").into()),
        }
    }
    Ok(input)
}

fn run() -> Result<bool, Box<dyn Error>> {
    let path = parse_input_path()?;
    let bytes = fs::read(path)?;
    let report = evaluate_bytes(&bytes)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(report.disposition == "NO_SIGNIFICANT_PRECISION_CHANGE_DETECTED"
        || report.disposition == "SIGNIFICANT_PRECISION_CHANGE_DETECTED")
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("scan-bim-tls-health-monitor: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_CHANGE: &str =
        include_str!("../fixtures/no-change.json");
    const PRECISION_CHANGE: &str =
        include_str!("../fixtures/precision-change.json");

    #[test]
    fn no_change_control_is_not_a_significant_precision_change() {
        let report = evaluate_bytes(NO_CHANGE.as_bytes()).expect("no-change fixture evaluates");
        assert_eq!(report.disposition, "NO_SIGNIFICANT_PRECISION_CHANGE_DETECTED");
        assert!(report.p_value >= report.alpha);
        assert_eq!(report.accuracy_disposition, "NOT_EVALUATED");
        assert_eq!(report.task_fitness_disposition, "NOT_EVALUATED");
    }

    #[test]
    fn known_precision_degradation_is_detected() {
        let report =
            evaluate_bytes(PRECISION_CHANGE.as_bytes()).expect("precision fixture evaluates");
        assert_eq!(report.disposition, "SIGNIFICANT_PRECISION_CHANGE_DETECTED");
        assert!(report.p_value < report.alpha);
        assert!(report.standard_deviation_ratio_testing_over_baseline[2] > 2.9);
        assert_eq!(report.accuracy_disposition, "NOT_EVALUATED");
        assert_eq!(report.task_fitness_disposition, "NOT_EVALUATED");
    }

    #[test]
    fn precision_improvement_is_also_a_detectable_change() {
        let mut input: Input =
            serde_json::from_str(PRECISION_CHANGE).expect("precision fixture parses");
        for session in &mut input.current_sessions {
            session.targets[2].within_session_standard_deviation_mm /= 3.0;
        }
        let bytes = serde_json::to_vec(&input).expect("improvement fixture serializes");
        let report = evaluate_bytes(&bytes).expect("improvement fixture evaluates");
        assert_eq!(report.disposition, "SIGNIFICANT_PRECISION_CHANGE_DETECTED");
        assert_eq!(report.precision_direction, "improved");
    }

    #[test]
    fn session_and_target_order_do_not_change_semantic_result() {
        let mut input: Input =
            serde_json::from_str(PRECISION_CHANGE).expect("precision fixture parses");
        input.baseline_sessions.reverse();
        input.current_sessions.reverse();
        for session in input
            .baseline_sessions
            .iter_mut()
            .chain(input.current_sessions.iter_mut())
        {
            session.targets.reverse();
        }
        let bytes = serde_json::to_vec(&input).expect("reordered fixture serializes");
        let reordered = evaluate_bytes(&bytes).expect("reordered fixture evaluates");
        let original =
            evaluate_bytes(PRECISION_CHANGE.as_bytes()).expect("original fixture evaluates");
        assert_eq!(reordered.disposition, original.disposition);
        assert_eq!(reordered.p_value, original.p_value);
        assert_eq!(
            reordered.standard_deviation_ratio_testing_over_baseline,
            original.standard_deviation_ratio_testing_over_baseline
        );
        assert_ne!(reordered.input_sha256, original.input_sha256);
    }

    #[test]
    fn changed_sensor_identity_or_configuration_is_not_an_input_comparison() {
        let mut input: serde_json::Value =
            serde_json::from_str(NO_CHANGE).expect("fixture parses");
        input["sensor"]["unit_id"] = serde_json::Value::String("another-scanner".to_owned());
        let bytes = serde_json::to_vec(&input).expect("modified input serializes");
        // Both periods share one explicit sensor identity; a different unit is a different
        // monitor baseline, not an inter-period comparison. The input remains valid as a
        // new unit's independent baseline. The report must identify that exact unit.
        let report = evaluate_bytes(&bytes).expect("new-unit baseline evaluates");
        assert_eq!(report.sensor_unit_id, "another-scanner");
    }

    #[test]
    fn duplicate_sessions_are_rejected() {
        let mut input: serde_json::Value =
            serde_json::from_str(NO_CHANGE).expect("fixture parses");
        let first = input["baseline_sessions"][0].clone();
        input["current_sessions"][0] = first;
        let bytes = serde_json::to_vec(&input).expect("modified input serializes");
        assert!(evaluate_bytes(&bytes).is_err());
    }

    #[test]
    fn a_missing_target_in_one_session_is_rejected() {
        let mut input: serde_json::Value =
            serde_json::from_str(NO_CHANGE).expect("fixture parses");
        input["current_sessions"][0]["targets"]
            .as_array_mut()
            .unwrap()
            .pop();
        let bytes = serde_json::to_vec(&input).expect("modified input serializes");
        assert!(evaluate_bytes(&bytes).is_err());
    }

    #[test]
    fn invalid_standard_deviation_is_rejected() {
        let mut input: serde_json::Value =
            serde_json::from_str(NO_CHANGE).expect("fixture parses");
        input["baseline_sessions"][0]["targets"][0]
            ["within_session_standard_deviation_mm"] = serde_json::Value::from(0.0);
        let bytes = serde_json::to_vec(&input).expect("modified input serializes");
        assert!(evaluate_bytes(&bytes).is_err());
    }

    #[test]
    fn insufficient_session_count_is_rejected() {
        let mut input: serde_json::Value =
            serde_json::from_str(NO_CHANGE).expect("fixture parses");
        input["baseline_sessions"].as_array_mut().unwrap().pop();
        let bytes = serde_json::to_vec(&input).expect("modified input serializes");
        assert!(evaluate_bytes(&bytes).is_err());
    }

    #[test]
    fn zero_seed_and_excessive_permutation_budget_are_rejected() {
        let mut input: serde_json::Value =
            serde_json::from_str(NO_CHANGE).expect("fixture parses");
        input["seed"] = serde_json::Value::from(0_u64);
        let bytes = serde_json::to_vec(&input).expect("zero seed serializes");
        assert!(evaluate_bytes(&bytes).is_err());

        let mut input: serde_json::Value =
            serde_json::from_str(NO_CHANGE).expect("fixture parses");
        input["permutation_count"] = serde_json::Value::from(MAX_PERMUTATIONS + 1);
        let bytes = serde_json::to_vec(&input).expect("large budget serializes");
        assert!(evaluate_bytes(&bytes).is_err());
    }

    #[test]
    fn report_never_claims_accuracy_or_task_fitness() {
        let report = evaluate_bytes(PRECISION_CHANGE.as_bytes()).expect("fixture evaluates");
        assert_eq!(report.accuracy_disposition, "NOT_EVALUATED");
        assert_eq!(report.task_fitness_disposition, "NOT_EVALUATED");
    }
}
