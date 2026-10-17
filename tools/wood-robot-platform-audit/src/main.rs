use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    fmt::Write as FmtWrite,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const EXPECTED_SUITE: &str = "WOOD_ROBOT_PLATFORM";
const EXPECTED_SCENARIOS: usize = 22;
const EXPECTED_AERIAL_SCENARIOS: usize = 13;

type Check<T> = Result<T, Vec<String>>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    purpose: String,
    claim_ceiling: String,
    execution_mode: String,
    units: BTreeMap<String, String>,
    expected_scenario_count: usize,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    platform_profile: String,
    task_type: String,
    material_state: String,
    observations: BTreeMap<String, Value>,
    hard_constraints: BTreeMap<String, Value>,
    expected_decision: String,
    expected_reason_codes: Vec<String>,
    expected_actuation: bool,
}

#[derive(Debug, Serialize)]
struct Summary {
    scenario_count: usize,
    aerial_scenario_count: usize,
    decisions: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
struct Receipt {
    receipt_version: &'static str,
    result: &'static str,
    suite_id: String,
    suite_version: String,
    source_commit: String,
    corpus_sha256: String,
    scenario_count: usize,
    aerial_scenario_count: usize,
    all_plan_only: bool,
    decision_counts: BTreeMap<String, usize>,
    checks: Vec<String>,
    errors: Vec<String>,
    claim_ceiling: String,
}

fn value_str<'a>(values: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a str> {
    values.get(key).and_then(Value::as_str)
}

fn value_bool(values: &BTreeMap<String, Value>, key: &str) -> Option<bool> {
    values.get(key).and_then(Value::as_bool)
}

fn value_u64(values: &BTreeMap<String, Value>, key: &str) -> Option<u64> {
    values.get(key).and_then(Value::as_u64)
}

fn expected_true(values: &BTreeMap<String, Value>, key: &str) -> bool {
    value_bool(values, key) == Some(true)
}

fn has_reason(scenario: &Scenario, code: &str) -> bool {
    scenario.expected_reason_codes.iter().any(|reason| reason == code)
}

fn push_missing(errors: &mut Vec<String>, scenario_id: &str, field: &str) {
    errors.push(format!("{scenario_id}: missing or invalid required field {field}"));
}

fn require_string<'a>(
    values: &'a BTreeMap<String, Value>,
    key: &str,
    scenario_id: &str,
    errors: &mut Vec<String>,
) -> Option<&'a str> {
    match value_str(values, key) {
        Some(v) if !v.trim().is_empty() => Some(v),
        _ => {
            push_missing(errors, scenario_id, key);
            None
        }
    }
}

fn check_aerial_scenario(s: &Scenario, errors: &mut Vec<String>) {
    let id = s.scenario_id.as_str();
    let o = &s.observations;
    let h = &s.hard_constraints;

    let required_hard_gates = [
        "require_site_permission",
        "require_aviation_authorization",
        "require_clear_airspace_and_geofence",
        "require_weather_inside_declared_envelope",
        "require_site_specific_wildlife_policy",
        "require_privacy_handling",
        "require_verified_recovery_plan",
        "require_no_physical_contact",
        "prohibit_payload_release",
        "require_return_reserve",
        "prohibit_default_under_canopy_flight",
        "require_required_surface_observed",
    ];
    for gate in required_hard_gates {
        if !expected_true(h, gate) {
            errors.push(format!("{id}: hard constraint {gate} must be explicitly true"));
        }
    }

    let required_observations = [
        "flight_permission_state",
        "site_permission_state",
        "airspace_state",
        "geofence_state",
        "weather_state",
        "wildlife_policy_state",
        "wildlife_disturbance_state",
        "privacy_handling_state",
        "recovery_plan_state",
        "flight_profile",
        "launch_recovery_state",
        "sensor_package",
        "localization_state",
        "payload_release_state",
    ];
    for field in required_observations {
        require_string(o, field, id, errors);
    }
    for field in ["altitude_m", "configured_min_altitude_m", "remaining_energy_wh", "required_return_reserve_wh"] {
        if value_u64(o, field).is_none() {
            push_missing(errors, id, field);
        }
    }
    if value_bool(o, "contact_detected").is_none() {
        push_missing(errors, id, "contact_detected");
    }
    if value_bool(o, "required_surface_visible").is_none() {
        push_missing(errors, id, "required_surface_visible");
    }
    if value_u64(o, "pose_uncertainty_mm").is_none() {
        push_missing(errors, id, "pose_uncertainty_mm");
    }
    if value_u64(h, "maximum_pose_uncertainty_mm").is_none() {
        push_missing(errors, id, "maximum_pose_uncertainty_mm");
    }
    require_string(o, "human_approval", id, errors);

    let mut blocking_conditions: Vec<(&str, &str)> = Vec::new();
    if value_str(o, "flight_permission_state") != Some("verified") {
        blocking_conditions.push(("FLIGHT_PERMISSION_NOT_VERIFIED", "flight_permission_state"));
    }
    if value_str(o, "site_permission_state") != Some("granted") {
        blocking_conditions.push(("SITE_PERMISSION_NOT_VERIFIED", "site_permission_state"));
    }
    if value_str(o, "airspace_state") != Some("clear")
        || value_str(o, "geofence_state") != Some("valid")
    {
        blocking_conditions.push(("AIRSPACE_OR_GEOFENCE_UNCLEARED", "airspace_state/geofence_state"));
    }
    if value_str(o, "weather_state") != Some("within_declared_envelope") {
        blocking_conditions.push(("WEATHER_OUTSIDE_DECLARED_ENVELOPE", "weather_state"));
    }
    if value_str(o, "wildlife_policy_state") != Some("approved_site_specific_profile") {
        blocking_conditions.push(("WILDLIFE_POLICY_NOT_APPROVED", "wildlife_policy_state"));
    }
    if value_str(o, "wildlife_disturbance_state") != Some("none_detected") {
        blocking_conditions.push(("WILDLIFE_DISTURBANCE_DETECTED", "wildlife_disturbance_state"));
    }
    if value_str(o, "privacy_handling_state") != Some("configured_data_minimization") {
        blocking_conditions.push(("PRIVACY_HANDLING_NOT_CONFIGURED", "privacy_handling_state"));
    }
    if value_str(o, "recovery_plan_state") != Some("verified_designated_recovery_area") {
        blocking_conditions.push(("SAFE_RECOVERY_PLAN_UNRESOLVED", "recovery_plan_state"));
    }
    if value_str(o, "launch_recovery_state") != Some("approved_existing_clearing") {
        blocking_conditions.push(("LAUNCH_RECOVERY_AREA_NOT_APPROVED", "launch_recovery_state"));
    }
    if value_str(o, "flight_profile") != Some("straight_pass_no_hover") {
        blocking_conditions.push(("UNDER_CANOPY_NOT_ALLOWED_IN_DEFAULT_PROFILE", "flight_profile"));
    }
    if value_bool(o, "required_surface_visible") != Some(true) {
        blocking_conditions.push(("REQUIRED_FEATURE_OCCLUDED", "required_surface_visible"));
    }
    match (value_u64(o, "pose_uncertainty_mm"), value_u64(h, "maximum_pose_uncertainty_mm")) {
        (Some(actual), Some(maximum)) if actual > maximum => {
            blocking_conditions.push(("POSE_UNCERTAINTY_EXCEEDS_LIMIT", "pose_uncertainty_mm"));
        }
        (None, _) | (_, None) => {}
        _ => {}
    }
    if value_bool(o, "contact_detected") != Some(false) {
        blocking_conditions.push(("PHYSICAL_CONTACT_DETECTED", "contact_detected"));
    }
    if value_str(o, "payload_release_state") != Some("none") {
        blocking_conditions.push(("PAYLOAD_RELEASE_PROHIBITED", "payload_release_state"));
    }
    if value_str(o, "localization_state") != Some("valid") {
        blocking_conditions.push(("LOCALIZATION_LOST", "localization_state"));
    }
    if value_str(o, "human_approval") != Some("approved") {
        blocking_conditions.push(("HUMAN_APPROVAL_NOT_GRANTED", "human_approval"));
    }
    match (value_u64(o, "altitude_m"), value_u64(o, "configured_min_altitude_m")) {
        (Some(actual), Some(minimum)) if actual < minimum => {
            blocking_conditions.push(("SITE_STANDOFF_POLICY_VIOLATED", "altitude_m"));
        }
        (None, _) | (_, None) => {}
        _ => {}
    }
    match (value_u64(o, "remaining_energy_wh"), value_u64(o, "required_return_reserve_wh")) {
        (Some(available), Some(reserve)) if available < reserve => {
            blocking_conditions.push(("RETURN_RESERVE_INFEASIBLE", "remaining_energy_wh"));
        }
        (None, _) | (_, None) => {}
        _ => {}
    }
    if value_str(o, "link_state") == Some("lost") {
        blocking_conditions.push(("LINK_LOST", "link_state"));
        if s.expected_decision != "blocked_safe_degraded_state" {
            errors.push(format!("{id}: lost command/telemetry link must enter the declared safe degraded state"));
        }
        if !has_reason(s, "USE_PREDECLARED_RECOVERY_ONLY") {
            errors.push(format!("{id}: lost link must name USE_PREDECLARED_RECOVERY_ONLY"));
        }
    }

    if s.expected_decision == "eligible_for_simulated_plan" && !blocking_conditions.is_empty() {
        let codes = blocking_conditions
            .iter()
            .map(|(code, _)| *code)
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(format!("{id}: plan is eligible despite hard-blocking conditions: {codes}"));
    }
    for (code, field) in blocking_conditions {
        if !has_reason(s, code) {
            errors.push(format!("{id}: {field} requires reason code {code}"));
        }
    }
}

fn check_general_eligibility(s: &Scenario, errors: &mut Vec<String>) {
    if s.expected_decision != "eligible_for_simulated_plan" {
        return;
    }
    let id = s.scenario_id.as_str();
    let o = &s.observations;
    let h = &s.hard_constraints;

    if let Some(approval) = value_str(o, "human_approval") {
        if approval != "approved" {
            errors.push(format!("{id}: eligible decision requires explicit human approval"));
        }
    }
    if let Some(calibration) = value_str(o, "calibration_state") {
        if calibration != "current" {
            errors.push(format!("{id}: eligible decision cannot use non-current calibration"));
        }
    }
    if value_str(o, "fixture_state") == Some("mismatch") {
        errors.push(format!("{id}: eligible decision cannot use a mismatched fixture"));
    }
    if value_str(o, "localization_state") == Some("lost") {
        errors.push(format!("{id}: eligible decision cannot use lost localization"));
    }
    if expected_true(h, "require_interlock_when_applicable")
        && value_str(o, "interlock_state") == Some("unavailable")
    {
        errors.push(format!("{id}: eligible decision cannot bypass an unavailable interlock"));
    }
    if expected_true(h, "require_extraction_for_dust_producing_tasks")
        && s.task_type.contains("dust_producing")
        && value_str(o, "extraction_state") != Some("available")
    {
        errors.push(format!("{id}: dust-producing task requires available extraction"));
    }
    match (value_u64(o, "remaining_energy_wh"), value_u64(o, "required_return_reserve_wh")) {
        (Some(available), Some(reserve)) if available < reserve => {
            errors.push(format!("{id}: eligible decision violates return energy reserve"));
        }
        _ => {}
    }
    if expected_true(h, "require_temporary_support_when_declared")
        && value_str(o, "temporary_support_state") == Some("missing")
    {
        errors.push(format!("{id}: eligible decision requires declared temporary support"));
    }
    if expected_true(h, "require_fixture_confirmation")
        && value_str(o, "fixture_state") != Some("confirmed")
    {
        errors.push(format!("{id}: eligible decision requires confirmed fixture state"));
    }
}

fn validate(corpus: &Corpus) -> Check<Summary> {
    let mut errors = Vec::new();
    if corpus.suite_id != EXPECTED_SUITE {
        errors.push(format!("unexpected suite_id {}", corpus.suite_id));
    }
    if corpus.suite_version != "0.1.0" {
        errors.push(format!("unexpected suite_version {}", corpus.suite_version));
    }
    if corpus.evidence_class != "synthetic" {
        errors.push("corpus evidence_class must be synthetic".to_owned());
    }
    if corpus.execution_mode != "plan_only_no_actuation" {
        errors.push("execution_mode must be plan_only_no_actuation".to_owned());
    }
    if corpus.purpose.trim().is_empty() {
        errors.push("purpose must be non-empty".to_owned());
    }
    let ceiling = corpus.claim_ceiling.to_lowercase();
    if !ceiling.contains("no robot performance") || !ceiling.contains("physical safety") {
        errors.push("claim_ceiling must explicitly deny robot-performance and physical-safety claims".to_owned());
    }
    if !corpus.units.contains_key("altitude") {
        errors.push("units must define altitude".to_owned());
    }
    if corpus.scenarios.len() != corpus.expected_scenario_count {
        errors.push(format!(
            "expected_scenario_count={} but found {} scenarios",
            corpus.expected_scenario_count,
            corpus.scenarios.len()
        ));
    }
    if corpus.expected_scenario_count != EXPECTED_SCENARIOS {
        errors.push(format!(
            "version 0.1.0 requires {EXPECTED_SCENARIOS} scenarios, declared {}",
            corpus.expected_scenario_count
        ));
    }

    let valid_decisions: BTreeSet<&str> = [
        "eligible_for_simulated_plan",
        "blocked",
        "insufficient_evidence",
        "blocked_safe_degraded_state",
    ].into_iter().collect();
    let mut ids = BTreeSet::new();
    let mut aerial_count = 0usize;
    let mut decision_counts = BTreeMap::<String, usize>::new();

    for s in &corpus.scenarios {
        let id = s.scenario_id.as_str();
        if id.trim().is_empty() {
            errors.push("scenario_id must be non-empty".to_owned());
        }
        if !ids.insert(id.to_owned()) {
            errors.push(format!("duplicate scenario_id {id}"));
        }
        if s.platform_profile.trim().is_empty() || s.task_type.trim().is_empty() || s.material_state.trim().is_empty() {
            errors.push(format!("{id}: platform_profile, task_type and material_state must be non-empty"));
        }
        if !valid_decisions.contains(s.expected_decision.as_str()) {
            errors.push(format!("{id}: unsupported expected_decision {}", s.expected_decision));
        }
        if s.expected_reason_codes.is_empty()
            || s.expected_reason_codes.iter().any(|code| code.trim().is_empty())
        {
            errors.push(format!("{id}: expected_reason_codes must contain non-empty codes"));
        }
        let mut reason_set = BTreeSet::new();
        for code in &s.expected_reason_codes {
            if !reason_set.insert(code) {
                errors.push(format!("{id}: duplicate expected reason code {code}"));
            }
        }
        if s.expected_actuation {
            errors.push(format!("{id}: physical actuation must remain false in this corpus"));
        }
        *decision_counts.entry(s.expected_decision.clone()).or_insert(0) += 1;
        check_general_eligibility(s, &mut errors);

        if s.platform_profile == "P0_AERIAL_FOREST_SCOUT" {
            aerial_count += 1;
            check_aerial_scenario(s, &mut errors);
        }
    }

    if aerial_count != EXPECTED_AERIAL_SCENARIOS {
        errors.push(format!(
            "expected {EXPECTED_AERIAL_SCENARIOS} aerial Scout scenarios, found {aerial_count}"
        ));
    }

    if errors.is_empty() {
        Ok(Summary {
            scenario_count: corpus.scenarios.len(),
            aerial_scenario_count: aerial_count,
            decisions: decision_counts,
        })
    } else {
        Err(errors)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

fn valid_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn args() -> Result<(String, String), String> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(|| {
        "usage: wood-robot-platform-audit <corpus.json> --source-commit <40-hex-sha>".to_owned()
    })?;
    let flag = args.next().ok_or_else(|| {
        "expected --source-commit <40-hex-sha>".to_owned()
    })?;
    if flag != "--source-commit" {
        return Err("expected --source-commit <40-hex-sha>".to_owned());
    }
    let sha = args.next().ok_or_else(|| "missing source commit SHA".to_owned())?;
    if !valid_commit_sha(&sha) {
        return Err("source commit must be a 40-character hexadecimal Git SHA".to_owned());
    }
    if args.next().is_some() {
        return Err("unexpected extra arguments".to_owned());
    }
    Ok((path, sha.to_ascii_lowercase()))
}

fn run() -> Result<(Receipt, bool), String> {
    let (path, source_commit) = args()?;
    let bytes = fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let corpus: Corpus = serde_json::from_slice(&bytes)
        .map_err(|e| format!("invalid corpus JSON: {e}"))?;
    let corpus_sha256 = sha256_hex(&bytes);
    let suite_id = corpus.suite_id.clone();
    let suite_version = corpus.suite_version.clone();
    let claim_ceiling = corpus.claim_ceiling.clone();
    let all_plan_only = corpus.scenarios.iter().all(|s| !s.expected_actuation);
    let decision_counts = {
        let mut counts = BTreeMap::<String, usize>::new();
        for scenario in &corpus.scenarios {
            *counts.entry(scenario.expected_decision.clone()).or_insert(0) += 1;
        }
        counts
    };
    let actual_count = corpus.scenarios.len();
    let aerial_count = corpus.scenarios.iter()
        .filter(|s| s.platform_profile == "P0_AERIAL_FOREST_SCOUT")
        .count();

    let checked = validate(&corpus);
    let (passed, checks, errors) = match checked {
        Ok(_summary) => (
            true,
            vec![
                "synthetic_plan_only_corpus".to_owned(),
                "scenario_count_and_unique_ids".to_owned(),
                "decision_and_reason_code_contracts".to_owned(),
                "generic_hard_constraints".to_owned(),
                "aerial_permission_wildlife_privacy_recovery_gates".to_owned(),
                "no_physical_actuation".to_owned(),
            ],
            Vec::new(),
        ),
        Err(errors) => (false, Vec::new(), errors),
    };
    let receipt = Receipt {
        receipt_version: VERSION,
        result: if passed { "pass" } else { "fail" },
        suite_id,
        suite_version,
        source_commit,
        corpus_sha256,
        scenario_count: actual_count,
        aerial_scenario_count: aerial_count,
        all_plan_only,
        decision_counts,
        checks,
        errors,
        claim_ceiling,
    };
    Ok((receipt, passed))
}

fn main() -> ExitCode {
    match run() {
        Ok((receipt, passed)) => {
            match serde_json::to_string_pretty(&receipt) {
                Ok(text) => println!("{text}"),
                Err(e) => {
                    eprintln!("cannot serialize audit receipt: {e}");
                    return ExitCode::from(2);
                }
            }
            if passed { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frozen_corpus() -> Corpus {
        serde_json::from_slice(include_bytes!("../../../docs/tech/wood-robot-platform-corpus-v0.1.json"))
            .expect("frozen corpus must parse")
    }

    #[test]
    fn frozen_corpus_satisfies_declared_contract() {
        let corpus = frozen_corpus();
        let summary = validate(&corpus).expect("frozen corpus should pass contract checks");
        assert_eq!(summary.scenario_count, EXPECTED_SCENARIOS);
        assert_eq!(summary.aerial_scenario_count, EXPECTED_AERIAL_SCENARIOS);
    }

    #[test]
    fn missing_aerial_site_permission_fails_closed() {
        let mut corpus = frozen_corpus();
        let scenario = corpus.scenarios.iter_mut()
            .find(|s| s.scenario_id == "SYNTHETIC_WOODROBOT_011_AERIAL_NOMINAL_STANDOFF_SURVEY")
            .expect("nominal aerial scenario exists");
        scenario.observations.insert("site_permission_state".to_owned(), json!("unknown"));
        assert!(validate(&corpus).is_err());
    }

    #[test]
    fn contact_detected_cannot_be_eligible() {
        let mut corpus = frozen_corpus();
        let scenario = corpus.scenarios.iter_mut()
            .find(|s| s.scenario_id == "SYNTHETIC_WOODROBOT_011_AERIAL_NOMINAL_STANDOFF_SURVEY")
            .expect("nominal aerial scenario exists");
        scenario.observations.insert("contact_detected".to_owned(), json!(true));
        assert!(validate(&corpus).is_err());
    }

    #[test]
    fn under_canopy_is_not_default_aerial_profile() {
        let mut corpus = frozen_corpus();
        let scenario = corpus.scenarios.iter_mut()
            .find(|s| s.scenario_id == "SYNTHETIC_WOODROBOT_011_AERIAL_NOMINAL_STANDOFF_SURVEY")
            .expect("nominal aerial scenario exists");
        scenario.observations.insert("flight_profile".to_owned(), json!("under_canopy_autonomous"));
        assert!(validate(&corpus).is_err());
    }

    #[test]
    fn no_scenario_can_request_actuation() {
        let mut corpus = frozen_corpus();
        corpus.scenarios[0].expected_actuation = true;
        assert!(validate(&corpus).is_err());
    }

    #[test]
    fn duplicate_scenario_ids_are_rejected() {
        let mut corpus = frozen_corpus();
        corpus.scenarios[1].scenario_id = corpus.scenarios[0].scenario_id.clone();
        assert!(validate(&corpus).is_err());
    }

    #[test]
    fn changed_corpus_bytes_change_digest() {
        let bytes = include_bytes!("../../../docs/tech/wood-robot-platform-corpus-v0.1.json");
        let mut changed = bytes.to_vec();
        changed.push(b' ');
        assert_ne!(sha256_hex(bytes), sha256_hex(&changed));
    }
}
