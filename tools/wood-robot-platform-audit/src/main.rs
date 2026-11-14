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
const EXPECTED_SCENARIOS: usize = 29;
const EXPECTED_AERIAL_SCENARIOS: usize = 13;
const EXPECTED_SALVAGE_SCENARIOS: usize = 7;

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
    salvage_scenario_count: usize,
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
    salvage_scenario_count: usize,
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
        if s.expected_decision != "blocked_safe_degraded_state" {
            errors.push(format!("{id}: detected wildlife disturbance must stop/divert the planned sortie"));
        }
        if !has_reason(s, "NO_REPEAT_SORTIE_WITHOUT_HUMAN_REVIEW") {
            errors.push(format!("{id}: wildlife disturbance must require human review before a repeat sortie"));
        }
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
        if s.expected_decision != "blocked_safe_degraded_state" {
            errors.push(format!("{id}: physical contact must stop the simulated sortie"));
        }
        if !has_reason(s, "NO_CONTINUED_MISSION_AFTER_CONTACT") {
            errors.push(format!("{id}: contact event must prohibit continuation of the mission"));
        }
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

fn check_salvage_scenario(s: &Scenario, errors: &mut Vec<String>) {
    let id = s.scenario_id.as_str();
    let o = &s.observations;
    let h = &s.hard_constraints;
    let mut blockers: Vec<&str> = Vec::new();

    for field in [
        "tree_condition",
        "harvest_permission_state",
        "landholder_consent_state",
        "protected_species_status",
        "deadwood_retention_disposition",
        "biodiversity_review_state",
        "hazard_review_state",
        "pest_quarantine_state",
    ] {
        require_string(o, field, id, errors);
    }

    if value_str(o, "harvest_permission_state") != Some("verified") {
        blockers.push("HARVEST_PERMISSION_UNVERIFIED");
    }
    if value_str(o, "landholder_consent_state") != Some("verified") {
        blockers.push("LANDHOLDER_CONSENT_UNVERIFIED");
    }
    if value_str(o, "protected_species_status") != Some("checked_not_protected") {
        blockers.push("PROTECTED_SPECIES_STATUS_UNKNOWN");
    }
    match value_str(o, "deadwood_retention_disposition") {
        Some("retain_habitat") => blockers.push("DEADWOOD_RETENTION_REQUIRED"),
        Some("approved_for_removal_after_ecological_review") => {}
        _ => blockers.push("ECOLOGICAL_REVIEW_INCOMPLETE"),
    }
    if value_str(o, "biodiversity_review_state") != Some("complete") {
        blockers.push("ECOLOGICAL_REVIEW_INCOMPLETE");
    }
    if value_str(o, "hazard_review_state") != Some("complete") {
        blockers.push("HAZARD_REVIEW_INCOMPLETE");
    }
    if value_str(o, "pest_quarantine_state") != Some("cleared") {
        blockers.push("BIOSECURITY_HOLD");
        blockers.push("NO_MOVEMENT_BEFORE_CLEARANCE");
    }

    if value_str(o, "tree_condition") == Some("declining_live") {
        let separate = expected_true(h, "require_separate_silvicultural_decision_for_live_tree_removal")
            && value_str(o, "separate_silvicultural_decision") == Some("approved");
        if !separate {
            blockers.push("LIVE_TREE_REMOVAL_REQUIRES_SEPARATE_APPROVAL");
        }
    }

    if s.expected_decision == "eligible_for_simulated_plan" && !blockers.is_empty() {
        errors.push(format!(
            "{id}: salvage/extraction screening is eligible despite hard blockers: {}",
            blockers.join(", ")
        ));
    }
    for code in blockers {
        if !has_reason(s, code) {
            errors.push(format!("{id}: salvage/extraction blocker requires reason code {code}"));
        }
    }

    match s.task_type.as_str() {
        "screen_deadwood_for_recovery" => {
            if s.expected_decision == "eligible_for_simulated_plan" {
                for code in ["SCREENING_ONLY", "NO_EXTRACTION_AUTHORIZED"] {
                    if !has_reason(s, code) {
                        errors.push(format!("{id}: eligible screening must include reason code {code}"));
                    }
                }
            }
        }
        "screen_declining_live_tree" => {
            if value_str(o, "tree_condition") == Some("declining_live")
                && value_str(o, "separate_silvicultural_decision") != Some("approved")
                && s.expected_decision == "eligible_for_simulated_plan"
            {
                errors.push(format!("{id}: a declining live tree cannot enter the deadwood salvage path without a separate decision"));
            }
        }
        "compare_extraction_modes" => {
            let ground_or_cable = value_bool(o, "ground_or_cable_access_feasible");
            let selected = value_str(o, "mode_selected");
            let helicopter_cost = value_u64(o, "helicopter_all_in_cost_cents_per_m3");
            let cable_cost = value_u64(o, "cable_yarder_all_in_cost_cents_per_m3");

            if expected_true(h, "compare_all_in_cost_including_mobilization")
                && (helicopter_cost.is_none() || (ground_or_cable == Some(true) && cable_cost.is_none()))
            {
                errors.push(format!("{id}: extraction-mode comparison lacks declared all-in cost inputs"));
            }

            if ground_or_cable == Some(true) {
                if let (Some(heli), Some(cable)) = (helicopter_cost, cable_cost) {
                    if cable < heli {
                        if selected != Some("cable_yarder") {
                            errors.push(format!("{id}: cheaper feasible cable extraction must not be displaced by helicopter cost-only preference"));
                        }
                        if !has_reason(s, "CABLE_YARDER_SELECTED_LOWER_DECLARED_COST") {
                            errors.push(format!("{id}: cost-dominated helicopter option must include CABLE_YARDER_SELECTED_LOWER_DECLARED_COST"));
                        }
                    }
                }
            } else if ground_or_cable == Some(false) && selected == Some("helicopter_candidate_for_operator_review") {
                let margin = value_u64(o, "expected_recovered_product_margin_cents_per_m3");
                if let (Some(heli), Some(margin)) = (helicopter_cost, margin) {
                    if margin <= heli {
                        errors.push(format!("{id}: helicopter candidate fails declared synthetic margin screen"));
                    }
                } else {
                    errors.push(format!("{id}: remote helicopter candidate lacks declared synthetic margin/cost"));
                }
                if value_str(o, "lift_plan_state") != Some("requires_qualified_operator_and_rigging_review") {
                    errors.push(format!("{id}: helicopter candidate requires qualified lift-plan and rigging review"));
                }
                if value_str(o, "rotor_clearance_state") != Some("qualified_site_assessment_required") {
                    errors.push(format!("{id}: rotor/obstacle clearance must remain an explicit site assessment"));
                }
                if value_str(o, "weather_state") != Some("within_declared_operating_envelope") {
                    errors.push(format!("{id}: helicopter candidate lacks an in-envelope weather declaration"));
                }
                for code in [
                    "HELICOPTER_ONLY_A_CANDIDATE_FOR_REVIEW",
                    "SYNTHETIC_COST_COMPARISON_ONLY",
                    "NO_EXTRACTION_AUTHORIZED",
                ] {
                    if !has_reason(s, code) {
                        errors.push(format!("{id}: helicopter proposal must include reason code {code}"));
                    }
                }
                if s.expected_actuation {
                    errors.push(format!("{id}: salvage screening cannot authorize a physical lift"));
                }
            }
        }
        _ => {}
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
    if !corpus.units.contains_key("cost") {
        errors.push("units must define explicit synthetic cost units for extraction-mode comparisons".to_owned());
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
    let mut salvage_count = 0usize;
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
        if s.platform_profile == "P6_REUSE_REMANUFACTURE_STATION"
            && ["screen_deadwood_for_recovery", "compare_extraction_modes", "screen_declining_live_tree"].contains(&s.task_type.as_str())
        {
            salvage_count += 1;
            check_salvage_scenario(s, &mut errors);
        }
    }

    if aerial_count != EXPECTED_AERIAL_SCENARIOS {
        errors.push(format!(
            "expected {EXPECTED_AERIAL_SCENARIOS} aerial Scout scenarios, found {aerial_count}"
        ));
    }

    if salvage_count != EXPECTED_SALVAGE_SCENARIOS {
        errors.push(format!(
            "expected {EXPECTED_SALVAGE_SCENARIOS} deadwood-salvage/extraction scenarios, found {salvage_count}"
        ));
    }

    if errors.is_empty() {
        Ok(Summary {
            scenario_count: corpus.scenarios.len(),
            aerial_scenario_count: aerial_count,
            salvage_scenario_count: salvage_count,
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
    let salvage_count = corpus.scenarios.iter()
        .filter(|s| s.platform_profile == "P6_REUSE_REMANUFACTURE_STATION"
            && ["screen_deadwood_for_recovery", "compare_extraction_modes", "screen_declining_live_tree"].contains(&s.task_type.as_str()))
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
        salvage_scenario_count: salvage_count,
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
        assert_eq!(summary.salvage_scenario_count, EXPECTED_SALVAGE_SCENARIOS);
        assert_eq!(summary.scenario_count, EXPECTED_SCENARIOS);
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
    fn habitat_retention_blocks_salvage_removal() {
        let mut corpus = frozen_corpus();
        let scenario = corpus.scenarios.iter_mut()
            .find(|s| s.scenario_id == "SYNTHETIC_WOODROBOT_024_DEADWOOD_RETAINED_FOR_HABITAT")
            .expect("habitat retention fixture exists");
        scenario.expected_decision = "eligible_for_simulated_plan".to_owned();
        assert!(validate(&corpus).is_err());
    }

    #[test]
    fn declining_live_tree_requires_separate_silvicultural_decision() {
        let mut corpus = frozen_corpus();
        let scenario = corpus.scenarios.iter_mut()
            .find(|s| s.scenario_id == "SYNTHETIC_WOODROBOT_029_DECLINING_LIVE_TREE_NEEDS_SEPARATE_DECISION")
            .expect("declining live tree fixture exists");
        scenario.observations.insert("separate_silvicultural_decision".to_owned(), json!("missing"));
        scenario.expected_decision = "eligible_for_simulated_plan".to_owned();
        assert!(validate(&corpus).is_err());
    }

    #[test]
    fn cable_yarder_dominates_synthetic_helicopter_cost_case() {
        let mut corpus = frozen_corpus();
        let scenario = corpus.scenarios.iter_mut()
            .find(|s| s.scenario_id == "SYNTHETIC_WOODROBOT_027_AIRLIFT_DOMINATED_BY_CABLE")
            .expect("cost-dominated helicopter fixture exists");
        scenario.observations.insert("mode_selected".to_owned(), json!("helicopter_candidate_for_operator_review"));
        assert!(validate(&corpus).is_err());
    }

    #[test]
    fn suspected_pest_blocks_moving_recently_dead_timber() {
        let mut corpus = frozen_corpus();
        let scenario = corpus.scenarios.iter_mut()
            .find(|s| s.scenario_id == "SYNTHETIC_WOODROBOT_023_RECENTLY_DEAD_TREE_SCREEN_READY")
            .expect("salvage positive fixture exists");
        scenario.observations.insert("pest_quarantine_state".to_owned(), json!("suspected_pest_or_pathogen"));
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
