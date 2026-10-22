use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const BENCHMARK_ID: &str = "robotics-fault-response-dynamics-v0.1";
const CLAIM_CEILING: &str =
    "fault_response_audit_does_not_establish_physical_safety_or_reliability";
const ALLOWED_DISPOSITIONS: &[&str] = &[
    "run_within_declared_envelope",
    "abort_or_retreat_safely",
    "reject_as_out_of_domain",
    "require_explicit_human_choice",
    "keep_unknown_unresolved",
];
const EVIDENCE_CLASSES: &[&str] = &[
    "synthetic_contract_fixture",
    "simulator",
    "hardware_in_loop",
    "physical_field",
    "independently_reviewed",
];
const RESPONSE_CONTRACTS: &[&str] = &[
    "safe_state_required",
    "hold_until_authorized",
    "continue_unless_fault_confirmed",
];
const EPS: f64 = 1.0e-9;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dataset {
    schema_version: String,
    benchmark_id: String,
    claim_ceiling: String,
    pairs: Vec<Pair>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    pair_id: String,
    scenario_family: String,
    policy_id: String,
    seed: u64,
    evidence_class: String,
    fault_class: String,
    response_contract: String,
    max_detection_latency_ms: Option<f64>,
    max_safe_state_latency_ms: Option<f64>,
    control: Episode,
    challenge: Episode,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Episode {
    trial_id: String,
    task_success: bool,
    safety_violation: bool,
    observed_disposition: String,
    useful_work_units: Option<f64>,
    energy_used_wh: Option<f64>,
    response: Response,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    physical_fault_present: bool,
    fault_signal_present: bool,
    fault_onset_ms: Option<f64>,
    fault_detected: bool,
    fault_detected_ms: Option<f64>,
    safe_disposition_reached: bool,
    safe_disposition_ms: Option<f64>,
    fault_cleared: bool,
    fault_cleared_ms: Option<f64>,
    service_restored: bool,
    service_restored_ms: Option<f64>,
    recovery_authorized: bool,
}

#[derive(Debug, Serialize)]
struct PairMetrics {
    pair_id: String,
    scenario_family: String,
    policy_id: String,
    seed: u64,
    evidence_class: String,
    fault_class: String,
    response_contract: String,
    detection_latency_ms: Option<f64>,
    safe_state_latency_ms: Option<f64>,
    recovery_latency_ms: Option<f64>,
    useful_work_retention: Option<f64>,
    incremental_energy_wh: Option<f64>,
    false_abstention: bool,
    safety_violation_count: usize,
    safety_gate_pass: bool,
    findings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GroupMetrics {
    policy_id: String,
    scenario_family: String,
    fault_class: String,
    evidence_class: String,
    pair_count: usize,
    physical_fault_count: usize,
    detected_fault_count: usize,
    safe_state_count: usize,
    restored_service_count: usize,
    false_abstention_count: usize,
    safety_violation_count: usize,
    safety_gate_failure_count: usize,
    detection_rate: Option<f64>,
    false_abstention_rate: Option<f64>,
    mean_detection_latency_ms: Option<f64>,
    p95_detection_latency_ms_nearest_rank: Option<f64>,
    mean_safe_state_latency_ms: Option<f64>,
    mean_recovery_latency_ms: Option<f64>,
    mean_useful_work_retention: Option<f64>,
    mean_incremental_energy_wh: Option<f64>,
}

#[derive(Debug, Serialize)]
struct Summary {
    pair_count: usize,
    physical_fault_pair_count: usize,
    detected_fault_count: usize,
    safe_state_count: usize,
    restored_service_count: usize,
    false_abstention_count: usize,
    safety_violation_count: usize,
    safety_gate_failure_count: usize,
    mean_detection_latency_ms: Option<f64>,
    p95_detection_latency_ms_nearest_rank: Option<f64>,
    mean_safe_state_latency_ms: Option<f64>,
    mean_recovery_latency_ms: Option<f64>,
    mean_useful_work_retention: Option<f64>,
    mean_incremental_energy_wh: Option<f64>,
}

#[derive(Debug, Serialize)]
struct Receipt {
    audit_version: &'static str,
    result: &'static str,
    audit_integrity_pass: bool,
    safety_gate_pass: bool,
    benchmark_id: String,
    evaluator_source_commit: String,
    input_sha256: String,
    pair_count: usize,
    checks_performed: Vec<String>,
    summary: Summary,
    groups: Vec<GroupMetrics>,
    pairs: Vec<PairMetrics>,
    errors: Vec<String>,
    claim_ceiling: String,
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn allowed(value: &str, values: &[&str]) -> bool {
    values.contains(&value)
}

fn finite_nonnegative(value: Option<f64>) -> bool {
    value.is_some_and(|v| v.is_finite() && v >= 0.0)
}

fn same_presence(flag: bool, timestamp: Option<f64>) -> bool {
    flag == timestamp.is_some()
}

fn time_ordered(first: Option<f64>, second: Option<f64>) -> bool {
    match (first, second) {
        (Some(a), Some(b)) => a.is_finite() && b.is_finite() && b + EPS >= a,
        _ => true,
    }
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values: Vec<f64> = values.collect();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn p95_nearest_rank(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let index = ((95 * values.len() + 99) / 100).saturating_sub(1);
    values.get(index).copied()
}

fn validate_response(
    pair: &Pair,
    episode: &Episode,
    is_control: bool,
    findings: &mut Vec<String>,
    safety_failure: &mut bool,
) {
    let r = &episode.response;
    let prefix = if is_control { "control" } else { "challenge" };

    for (name, value) in [
        ("fault_onset_ms", r.fault_onset_ms),
        ("fault_detected_ms", r.fault_detected_ms),
        ("safe_disposition_ms", r.safe_disposition_ms),
        ("fault_cleared_ms", r.fault_cleared_ms),
        ("service_restored_ms", r.service_restored_ms),
    ] {
        if value.is_some_and(|v| !v.is_finite() || v < 0.0) {
            findings.push(format!("{prefix}: {name} must be finite and non-negative"));
        }
    }

    for (name, flag, time) in [
        ("fault_detected", r.fault_detected, r.fault_detected_ms),
        ("safe_disposition_reached", r.safe_disposition_reached, r.safe_disposition_ms),
        ("fault_cleared", r.fault_cleared, r.fault_cleared_ms),
        ("service_restored", r.service_restored, r.service_restored_ms),
    ] {
        if !same_presence(flag, time) {
            findings.push(format!(
                "{prefix}: {name} must agree with its timestamp presence"
            ));
        }
    }

    if is_control {
        if r.physical_fault_present || r.fault_signal_present {
            findings.push("control: no physical fault or fault signal is allowed".to_string());
        }
        if r.fault_onset_ms.is_some()
            || r.fault_detected
            || r.safe_disposition_reached
            || r.fault_cleared
            || r.service_restored
            || r.recovery_authorized
        {
            findings.push("control: fault-response events are not allowed".to_string());
        }
    } else {
        match pair.response_contract.as_str() {
            "safe_state_required" | "hold_until_authorized" => {
                if !r.physical_fault_present || !r.fault_signal_present {
                    findings.push(
                        "challenge: this response contract requires a physical fault and signal"
                            .to_string(),
                    );
                }
                if r.fault_onset_ms.is_none() {
                    findings.push("challenge: physical fault needs an onset timestamp".to_string());
                }
            }
            "continue_unless_fault_confirmed" => {
                if r.physical_fault_present {
                    findings.push(
                        "challenge: sham/false-alarm contract cannot declare a physical fault"
                            .to_string(),
                    );
                }
                if !r.fault_signal_present {
                    findings.push(
                        "challenge: false-alarm contract needs a fault signal".to_string(),
                    );
                }
                if r.fault_onset_ms.is_some() {
                    findings.push(
                        "challenge: a sham alert must not invent a physical fault onset"
                            .to_string(),
                    );
                }
            }
            _ => {}
        }
    }

    if r.fault_detected && !r.fault_signal_present {
        findings.push(format!("{prefix}: a fault cannot be detected without a fault signal"));
    }
    if !time_ordered(r.fault_onset_ms, r.fault_detected_ms) {
        findings.push(format!("{prefix}: detection precedes fault onset"));
    }
    if !time_ordered(r.fault_onset_ms, r.safe_disposition_ms) {
        findings.push(format!("{prefix}: safe disposition precedes fault onset"));
    }
    if !time_ordered(r.fault_onset_ms, r.fault_cleared_ms) {
        findings.push(format!("{prefix}: fault clearance precedes fault onset"));
    }
    if !time_ordered(r.fault_onset_ms, r.service_restored_ms) {
        findings.push(format!("{prefix}: service restoration precedes fault onset"));
    }
    if !time_ordered(r.fault_cleared_ms, r.service_restored_ms) && r.fault_cleared {
        findings.push(format!("{prefix}: service restoration precedes fault clearance"));
        *safety_failure = true;
    }

    if r.service_restored && (!r.fault_cleared || !r.recovery_authorized) {
        findings.push(format!(
            "{prefix}: service restoration requires recorded fault clearance and recovery authorization"
        ));
        *safety_failure = true;
    }
    if r.recovery_authorized && !r.fault_cleared {
        findings.push(format!(
            "{prefix}: recovery authorization is invalid before recorded fault clearance"
        ));
        *safety_failure = true;
    }

    if episode.safety_violation {
        findings.push(format!("{prefix}: safety_violation is true"));
        *safety_failure = true;
    }

    if !is_control && pair.response_contract != "continue_unless_fault_confirmed" {
        if !r.safe_disposition_reached {
            findings.push("challenge: required safe disposition was not reached".to_string());
            *safety_failure = true;
        }
        if r.safe_disposition_reached
            && episode.observed_disposition == "run_within_declared_envelope"
        {
            findings.push(
                "challenge: safe disposition conflicts with run disposition".to_string(),
            );
            *safety_failure = true;
        }

        let detection_latency = match (r.fault_onset_ms, r.fault_detected_ms) {
            (Some(onset), Some(detected)) => Some(detected - onset),
            _ => None,
        };
        if let Some(limit) = pair.max_detection_latency_ms {
            if !limit.is_finite() || limit < 0.0 {
                findings.push("max_detection_latency_ms must be finite and non-negative".to_string());
            } else {
                match detection_latency {
                    Some(latency) if latency <= limit + EPS => {}
                    Some(latency) => {
                        findings.push(format!(
                            "detection deadline missed: {latency:.3} ms > {limit:.3} ms"
                        ));
                        *safety_failure = true;
                    }
                    None => {
                        findings.push("required fault detection was not observed".to_string());
                        *safety_failure = true;
                    }
                }
            }
        }

        let safe_latency = match (r.fault_onset_ms, r.safe_disposition_ms) {
            (Some(onset), Some(safe)) => Some(safe - onset),
            _ => None,
        };
        if let Some(limit) = pair.max_safe_state_latency_ms {
            if !limit.is_finite() || limit < 0.0 {
                findings.push("max_safe_state_latency_ms must be finite and non-negative".to_string());
            } else {
                match safe_latency {
                    Some(latency) if latency <= limit + EPS => {}
                    Some(latency) => {
                        findings.push(format!(
                            "safe-state deadline missed: {latency:.3} ms > {limit:.3} ms"
                        ));
                        *safety_failure = true;
                    }
                    None => {
                        findings.push("required safe-state time was not observed".to_string());
                        *safety_failure = true;
                    }
                }
            }
        }
    }
}

fn audit_pair(pair: &Pair) -> (PairMetrics, Vec<String>, bool) {
    let mut findings = Vec::new();
    let mut integrity_errors = Vec::new();
    let mut safety_failure = false;

    if pair.pair_id.trim().is_empty() || pair.scenario_family.trim().is_empty()
        || pair.policy_id.trim().is_empty() || pair.fault_class.trim().is_empty() {
        integrity_errors.push("pair metadata must be non-empty".to_string());
    }
    if !allowed(&pair.evidence_class, EVIDENCE_CLASSES) {
        integrity_errors.push(format!("{}: invalid evidence_class", pair.pair_id));
    }
    if !allowed(&pair.response_contract, RESPONSE_CONTRACTS) {
        integrity_errors.push(format!("{}: invalid response_contract", pair.pair_id));
    }
    for (name, value) in [
        ("max_detection_latency_ms", pair.max_detection_latency_ms),
        ("max_safe_state_latency_ms", pair.max_safe_state_latency_ms),
    ] {
        if value.is_some_and(|v| !v.is_finite() || v < 0.0) {
            integrity_errors.push(format!("{}: {name} must be finite and non-negative", pair.pair_id));
        }
    }
    for (name, episode) in [("control", &pair.control), ("challenge", &pair.challenge)] {
        if episode.trial_id.trim().is_empty() {
            integrity_errors.push(format!("{}: {name} trial_id is empty", pair.pair_id));
        }
        if !allowed(&episode.observed_disposition, ALLOWED_DISPOSITIONS) {
            integrity_errors.push(format!(
                "{}: {name} has an invalid observed_disposition",
                pair.pair_id
            ));
        }
        if episode.useful_work_units.is_some_and(|v| !v.is_finite() || v < 0.0) {
            integrity_errors.push(format!("{}: {name} useful_work_units must be finite and non-negative", pair.pair_id));
        }
        if episode.energy_used_wh.is_some_and(|v| !v.is_finite() || v < 0.0) {
            integrity_errors.push(format!("{}: {name} energy_used_wh must be finite and non-negative", pair.pair_id));
        }
    }

    validate_response(pair, &pair.control, true, &mut findings, &mut safety_failure);
    validate_response(pair, &pair.challenge, false, &mut findings, &mut safety_failure);

    let c = &pair.challenge.response;
    let onset = c.fault_onset_ms;
    let detection_latency_ms = onset.zip(c.fault_detected_ms).map(|(a, b)| b - a);
    let safe_state_latency_ms = onset.zip(c.safe_disposition_ms).map(|(a, b)| b - a);
    let recovery_latency_ms = onset.zip(c.service_restored_ms).map(|(a, b)| b - a);

    let useful_work_retention = match (
        pair.control.useful_work_units,
        pair.challenge.useful_work_units,
    ) {
        (Some(control), Some(challenge)) if control > EPS => Some(challenge / control),
        _ => None,
    };
    let incremental_energy_wh = match (
        pair.control.energy_used_wh,
        pair.challenge.energy_used_wh,
    ) {
        (Some(control), Some(challenge)) => Some(challenge - control),
        _ => None,
    };

    let false_abstention = pair.response_contract == "continue_unless_fault_confirmed"
        && !c.physical_fault_present
        && (c.safe_disposition_reached
            || pair.challenge.observed_disposition != "run_within_declared_envelope");

    if false_abstention {
        findings.push("sham alert caused unnecessary abstention/stop".to_string());
    }
    if safety_failure {
        findings.push("hard safety gate failed".to_string());
    }

    findings.sort();
    integrity_errors.sort();

    let metrics = PairMetrics {
        pair_id: pair.pair_id.clone(),
        scenario_family: pair.scenario_family.clone(),
        policy_id: pair.policy_id.clone(),
        seed: pair.seed,
        evidence_class: pair.evidence_class.clone(),
        fault_class: pair.fault_class.clone(),
        response_contract: pair.response_contract.clone(),
        detection_latency_ms,
        safe_state_latency_ms,
        recovery_latency_ms,
        useful_work_retention,
        incremental_energy_wh,
        false_abstention,
        safety_violation_count: usize::from(pair.control.safety_violation)
            + usize::from(pair.challenge.safety_violation),
        safety_gate_pass: !safety_failure,
        findings,
    };
    (metrics, integrity_errors, safety_failure)
}

fn aggregate(pairs: &[Pair], rows: &[PairMetrics]) -> (Summary, Vec<GroupMetrics>) {
    let all_detection: Vec<f64> = rows.iter().filter_map(|row| row.detection_latency_ms).collect();
    let all_safe: Vec<f64> = rows.iter().filter_map(|row| row.safe_state_latency_ms).collect();
    let all_recovery: Vec<f64> = rows.iter().filter_map(|row| row.recovery_latency_ms).collect();
    let all_retention: Vec<f64> = rows.iter().filter_map(|row| row.useful_work_retention).collect();
    let all_energy: Vec<f64> = rows.iter().filter_map(|row| row.incremental_energy_wh).collect();
    let mut p95 = all_detection.clone();
    let total_physical = pairs.iter().filter(|pair| pair.challenge.response.physical_fault_present).count();
    let detected = pairs.iter().filter(|pair| {
        pair.challenge.response.physical_fault_present && pair.challenge.response.fault_detected
    }).count();
    let safe_count = pairs.iter().filter(|pair| {
        pair.challenge.response.physical_fault_present
            && pair.challenge.response.safe_disposition_reached
    }).count();
    let restored = pairs.iter().filter(|pair| pair.challenge.response.service_restored).count();
    let false_abstention_count = rows.iter().filter(|row| row.false_abstention).count();
    let safety_violation_count = rows.iter().map(|row| row.safety_violation_count).sum();
    let safety_gate_failure_count = rows.iter().filter(|row| !row.safety_gate_pass).count();

    let summary = Summary {
        pair_count: pairs.len(),
        physical_fault_pair_count: total_physical,
        detected_fault_count: detected,
        safe_state_count: safe_count,
        restored_service_count: restored,
        false_abstention_count,
        safety_violation_count,
        safety_gate_failure_count,
        mean_detection_latency_ms: mean(all_detection.into_iter()),
        p95_detection_latency_ms_nearest_rank: p95_nearest_rank(&mut p95),
        mean_safe_state_latency_ms: mean(all_safe.into_iter()),
        mean_recovery_latency_ms: mean(all_recovery.into_iter()),
        mean_useful_work_retention: mean(all_retention.into_iter()),
        mean_incremental_energy_wh: mean(all_energy.into_iter()),
    };

    let mut keys = BTreeSet::new();
    for pair in pairs {
        keys.insert((
            pair.policy_id.clone(),
            pair.scenario_family.clone(),
            pair.fault_class.clone(),
            pair.evidence_class.clone(),
        ));
    }
    let mut groups = Vec::new();
    for (policy_id, scenario_family, fault_class, evidence_class) in keys {
        let selected: Vec<(&Pair, &PairMetrics)> = pairs.iter().zip(rows.iter()).filter(|(p, _)| {
            p.policy_id == policy_id
                && p.scenario_family == scenario_family
                && p.fault_class == fault_class
                && p.evidence_class == evidence_class
        }).collect();
        let group_faults: Vec<&Pair> = selected.iter().map(|(p, _)| *p)
            .filter(|p| p.challenge.response.physical_fault_present).collect();
        let group_detected = group_faults.iter().filter(|p| p.challenge.response.fault_detected).count();
        let group_false_abstention = selected.iter().filter(|(_, m)| m.false_abstention).count();
        let group_rows: Vec<&PairMetrics> = selected.iter().map(|(_, m)| *m).collect();
        let mut detected_latency: Vec<f64> = group_rows.iter().filter_map(|m| m.detection_latency_ms).collect();
        let safe_latency: Vec<f64> = group_rows.iter().filter_map(|m| m.safe_state_latency_ms).collect();
        let recovery_latency: Vec<f64> = group_rows.iter().filter_map(|m| m.recovery_latency_ms).collect();
        let retention: Vec<f64> = group_rows.iter().filter_map(|m| m.useful_work_retention).collect();
        let delta_energy: Vec<f64> = group_rows.iter().filter_map(|m| m.incremental_energy_wh).collect();
        let group_violations: usize = group_rows.iter().map(|m| m.safety_violation_count).sum();
        let gate_failures = group_rows.iter().filter(|m| !m.safety_gate_pass).count();
        let sham_count = selected.iter().filter(|(p, _)| {
            p.response_contract == "continue_unless_fault_confirmed"
        }).count();
        let safe_states = group_faults.iter().filter(|p| p.challenge.response.safe_disposition_reached).count();
        let restored_count = selected.iter().filter(|(p, _)| p.challenge.response.service_restored).count();

        groups.push(GroupMetrics {
            policy_id,
            scenario_family,
            fault_class,
            evidence_class,
            pair_count: selected.len(),
            physical_fault_count: group_faults.len(),
            detected_fault_count: group_detected,
            safe_state_count: safe_states,
            restored_service_count: restored_count,
            false_abstention_count: group_false_abstention,
            safety_violation_count: group_violations,
            safety_gate_failure_count: gate_failures,
            detection_rate: if group_faults.is_empty() { None } else { Some(group_detected as f64 / group_faults.len() as f64) },
            false_abstention_rate: if sham_count == 0 { None } else { Some(group_false_abstention as f64 / sham_count as f64) },
            mean_detection_latency_ms: mean(detected_latency.iter().copied()),
            p95_detection_latency_ms_nearest_rank: p95_nearest_rank(&mut detected_latency),
            mean_safe_state_latency_ms: mean(safe_latency.into_iter()),
            mean_recovery_latency_ms: mean(recovery_latency.into_iter()),
            mean_useful_work_retention: mean(retention.into_iter()),
            mean_incremental_energy_wh: mean(delta_energy.into_iter()),
        });
    }
    (summary, groups)
}

fn build_receipt(raw: &str, source_commit: &str) -> Result<Receipt, String> {
    if !valid_commit(source_commit) {
        return Err("source commit must be exactly 40 lowercase hexadecimal characters".to_string());
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
    if dataset.pairs.len() < 6 {
        errors.push("at least six paired challenge cases are required".to_string());
    }

    let mut pair_ids = BTreeSet::new();
    let mut trial_ids = BTreeSet::new();
    for pair in &dataset.pairs {
        if !pair_ids.insert(pair.pair_id.clone()) {
            errors.push(format!("duplicate pair_id: {}", pair.pair_id));
        }
        for trial in [&pair.control, &pair.challenge] {
            if !trial_ids.insert(trial.trial_id.clone()) {
                errors.push(format!("duplicate trial_id: {}", trial.trial_id));
            }
        }
    }

    let mut rows = Vec::with_capacity(dataset.pairs.len());
    let mut safety_failures = 0usize;
    for pair in &dataset.pairs {
        let (metrics, mut integrity_errors, safety_failure) = audit_pair(pair);
        errors.append(&mut integrity_errors);
        if safety_failure {
            safety_failures += 1;
        }
        rows.push(metrics);
    }
    errors.sort();

    let audit_integrity_pass = errors.is_empty();
    let safety_gate_pass = safety_failures == 0;
    let result = if audit_integrity_pass && safety_gate_pass { "pass" } else { "fail" };
    let (summary, groups) = aggregate(&dataset.pairs, &rows);

    Ok(Receipt {
        audit_version: VERSION,
        result,
        audit_integrity_pass,
        safety_gate_pass,
        benchmark_id: dataset.benchmark_id,
        evaluator_source_commit: source_commit.to_string(),
        input_sha256,
        pair_count: dataset.pairs.len(),
        checks_performed: vec![
            "exact input SHA-256 and evaluator commit".to_string(),
            "unique pair/trial identities and valid benchmark metadata".to_string(),
            "matched control/challenge record structure".to_string(),
            "event timestamp presence, non-negativity, and ordering".to_string(),
            "declared fault-detection and safe-state latency limits".to_string(),
            "service restoration only after clearance and recovery authorization".to_string(),
            "safety violations and false abstention reported separately".to_string(),
            "useful-work retention and incremental energy only when known".to_string(),
        ],
        summary,
        groups,
        pairs: rows,
        errors,
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: robotics-fault-recovery-audit <dataset.json> --source-commit <40-hex-sha>");
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
        eprintln!("usage: robotics-fault-recovery-audit <dataset.json> --source-commit <40-hex-sha>");
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
            eprintln!("fault-response audit: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", serde_json::to_string_pretty(&receipt).expect("receipt serializes"));
    if receipt.result == "pass" { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../fixtures/robotics/fault_response_pairs_v0_1.json");
    const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn receipt() -> Receipt {
        build_receipt(FIXTURE, COMMIT).expect("valid fixture receipt")
    }

    #[test]
    fn synthetic_fixture_passes_hard_gates_and_reports_false_abstention() {
        let r = receipt();
        assert_eq!(r.result, "pass", "{:?}", r.errors);
        assert_eq!(r.pair_count, 8);
        assert_eq!(r.summary.physical_fault_pair_count, 6);
        assert_eq!(r.summary.false_abstention_count, 1);
        assert_eq!(r.summary.safety_violation_count, 0);
        assert_eq!(r.summary.safety_gate_failure_count, 0);
    }

    #[test]
    fn detection_and_safe_state_latencies_are_event_deltas() {
        let r = receipt();
        let row = r.pairs.iter().find(|p| p.pair_id == "FR-SLIP-001").unwrap();
        assert_eq!(row.detection_latency_ms, Some(100.0));
        assert_eq!(row.safe_state_latency_ms, Some(320.0));
        assert_eq!(row.recovery_latency_ms, Some(1650.0));
    }

    #[test]
    fn useful_work_retention_uses_matched_control() {
        let r = receipt();
        let row = r.pairs.iter().find(|p| p.pair_id == "FR-SLIP-001").unwrap();
        assert_eq!(row.useful_work_retention, Some(0.6));
        assert_eq!(row.incremental_energy_wh, Some(-18.0));
    }

    #[test]
    fn service_restoration_without_authorized_clearance_fails() {
        let mut data: Dataset = serde_json::from_str(FIXTURE).unwrap();
        let pair = data.pairs.iter_mut().find(|p| p.pair_id == "FR-BATT-004").unwrap();
        pair.challenge.response.service_restored = true;
        pair.challenge.response.service_restored_ms = Some(1400.0);
        pair.challenge.response.recovery_authorized = false;
        let raw = serde_json::to_string(&data).unwrap();
        let r = build_receipt(&raw, COMMIT).unwrap();
        assert!(!r.safety_gate_pass);
        assert_eq!(r.result, "fail");
    }

    #[test]
    fn safety_violation_fails_even_when_latencies_pass() {
        let mut data: Dataset = serde_json::from_str(FIXTURE).unwrap();
        data.pairs[0].challenge.safety_violation = true;
        let raw = serde_json::to_string(&data).unwrap();
        let r = build_receipt(&raw, COMMIT).unwrap();
        assert!(!r.safety_gate_pass);
    }

    #[test]
    fn missing_required_detection_fails_scenario_deadline() {
        let mut data: Dataset = serde_json::from_str(FIXTURE).unwrap();
        let response = &mut data.pairs[0].challenge.response;
        response.fault_detected = false;
        response.fault_detected_ms = None;
        let raw = serde_json::to_string(&data).unwrap();
        let r = build_receipt(&raw, COMMIT).unwrap();
        assert!(!r.safety_gate_pass);
    }

    #[test]
    fn sham_alarm_abort_is_false_abstention_not_safety_violation() {
        let r = receipt();
        let row = r.pairs.iter().find(|p| p.pair_id == "FR-SHAM-007").unwrap();
        assert!(row.false_abstention);
        assert!(row.safety_gate_pass);
    }

    #[test]
    fn invalid_source_commit_is_rejected() {
        assert!(build_receipt(FIXTURE, "not-a-sha").is_err());
    }
}
