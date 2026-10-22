use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const EXPECTED_SCENARIOS: usize = 29;
const EXPECTED_MUTATIONS: usize = 39;
const ELIGIBLE: &str = "eligible_for_simulated_plan";
const SAFETY_CEILING: &str =
    "Synthetic robotics qualification gates only; no robot performance, physical safety, field readiness, or standards conformity claim.";

#[derive(Debug)]
struct ScenarioAudit {
    scenario_id: String,
    decision: String,
    blockers: Vec<String>,
    quality_findings: Vec<String>,
    violations: Vec<String>,
}

#[derive(Debug)]
struct Receipt {
    source_commit: String,
    corpus_sha256: String,
    mutation_matrix_sha256: String,
    scenario_count: usize,
    mutation_count: usize,
    blocker_counts: BTreeMap<String, usize>,
    quality_finding_counts: BTreeMap<String, usize>,
    scenario_violations: Vec<String>,
    mutation_failures: Vec<String>,
    claim_ceiling: &'static str,
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn u64_at(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn object_at<'a>(value: &'a Value, key: &str) -> Option<&'a serde_json::Map<String, Value>> {
    value.get(key).and_then(Value::as_object)
}

fn push_once(items: &mut Vec<String>, code: &str) {
    if !items.iter().any(|existing| existing == code) {
        items.push(code.to_owned());
    }
}

fn blocker(items: &mut Vec<String>, condition: bool, code: &str) {
    if condition {
        push_once(items, code);
    }
}

fn has_reason(scenario: &Value, code: &str) -> bool {
    scenario
        .get("expected_reason_codes")
        .and_then(Value::as_array)
        .is_some_and(|codes| codes.iter().any(|value| value.as_str() == Some(code)))
}

fn validate_scenario_shape(scenario: &Value) -> Result<(), String> {
    if !scenario.is_object() {
        return Err("scenario must be an object".to_owned());
    }

    for field in [
        "scenario_id",
        "platform_profile",
        "task_type",
        "expected_decision",
    ] {
        match str_at(scenario, field) {
            Some(value) if !value.trim().is_empty() => {}
            _ => return Err(format!("missing or invalid scenario field {field}")),
        }
    }

    if object_at(scenario, "observations").is_none() {
        return Err("observations must be an object".to_owned());
    }
    if object_at(scenario, "hard_constraints").is_none() {
        return Err("hard_constraints must be an object".to_owned());
    }
    if scenario
        .get("expected_reason_codes")
        .and_then(Value::as_array)
        .is_none()
    {
        return Err("expected_reason_codes must be an array".to_owned());
    }
    if bool_at(scenario, "expected_actuation").is_none() {
        return Err("expected_actuation must be a boolean".to_owned());
    }

    Ok(())
}

/// Independent observation-driven safety-floor rules.
///
/// This intentionally consumes JSON values directly. It does not use the existing
/// platform audit crate's typed records, validation routines, or expected-decision
/// switch. Expected labels are consulted only after the observed constraints are
/// assessed, when checking whether a claimed disposition is consistent.
fn evaluate_blockers(scenario: &Value) -> (Vec<String>, Vec<String>) {
    let observations = &scenario["observations"];
    let constraints = &scenario["hard_constraints"];
    let profile = str_at(scenario, "platform_profile").unwrap_or_default();
    let task = str_at(scenario, "task_type").unwrap_or_default();
    let mut blockers = Vec::new();
    let mut quality_findings = Vec::new();

    if bool_at(scenario, "expected_actuation") != Some(false) {
        blocker(&mut blockers, true, "PHYSICAL_ACTUATION_FORBIDDEN");
    }

    if let Some(minimum) = str_at(constraints, "minimum_calibration_state") {
        if str_at(observations, "calibration_state") != Some(minimum) {
            let code = if str_at(observations, "calibration_state") == Some("expired") {
                "CALIBRATION_EXPIRED"
            } else {
                "CALIBRATION_NOT_CURRENT"
            };
            blocker(&mut blockers, true, code);
        }
    }

    if bool_at(constraints, "require_fixture_confirmation") == Some(true)
        && str_at(observations, "fixture_state") != Some("confirmed")
    {
        let code = if str_at(observations, "fixture_state") == Some("mismatch") {
            "FIXTURE_PART_MISMATCH"
        } else {
            "FIXTURE_CONFIRMATION_MISSING"
        };
        blocker(&mut blockers, true, code);
    }

    if bool_at(constraints, "require_interlock_when_applicable") == Some(true)
        && str_at(observations, "interlock_state") == Some("unavailable")
    {
        blocker(&mut blockers, true, "INTERLOCK_UNAVAILABLE");
    }

    if bool_at(constraints, "require_extraction_for_dust_producing_tasks") == Some(true)
        && task.contains("dust_producing")
        && str_at(observations, "extraction_state") != Some("available")
    {
        blocker(&mut blockers, true, "REQUIRED_EXTRACTION_UNAVAILABLE");
    }

    if bool_at(constraints, "require_human_approval") == Some(true)
        && str_at(observations, "human_approval") != Some("approved")
    {
        let code = if str_at(observations, "human_approval") == Some("declined") {
            "HUMAN_APPROVAL_DECLINED"
        } else {
            "HUMAN_APPROVAL_NOT_GRANTED"
        };
        blocker(&mut blockers, true, code);
    }

    if bool_at(constraints, "require_return_reserve") == Some(true) {
        match (
            u64_at(observations, "remaining_energy_wh"),
            u64_at(observations, "required_return_reserve_wh"),
        ) {
            (Some(available), Some(required)) if available < required => {
                blocker(&mut blockers, true, "RETURN_RESERVE_INFEASIBLE");
            }
            (None, _) | (_, None) => {
                blocker(&mut blockers, true, "RETURN_RESERVE_UNKNOWN");
            }
            _ => {}
        }
    }

    if bool_at(constraints, "require_temporary_support_when_declared") == Some(true)
        && str_at(observations, "temporary_support_state") == Some("missing")
    {
        blocker(&mut blockers, true, "TEMPORARY_SUPPORT_MISSING");
    }

    if (bool_at(constraints, "require_valid_localization") == Some(true)
        || str_at(observations, "localization_state").is_some())
        && str_at(observations, "localization_state") != Some("valid")
    {
        blocker(&mut blockers, true, "LOCALIZATION_LOST");
        if bool_at(constraints, "require_safe_degraded_state_on_localization_loss") == Some(true) {
            blocker(&mut blockers, true, "MOTION_NOT_AUTHORIZED");
        }
    }

    if str_at(observations, "tool_state").is_some_and(|value| value != "compatible") {
        blocker(&mut blockers, true, "TOOL_PROFILE_MISMATCH");
    }

    if bool_at(constraints, "require_grade_evidence_for_structural_use") == Some(true)
        && str_at(observations, "grade_evidence_state") != Some("qualified")
    {
        blocker(&mut blockers, true, "STRUCTURAL_GRADE_UNQUALIFIED");
    }

    if bool_at(constraints, "require_hidden_fastener_resolution_before_disassembly") == Some(true)
        && str_at(observations, "hidden_fastener_state") != Some("resolved")
    {
        blocker(&mut blockers, true, "HIDDEN_FASTENER_UNRESOLVED");
    }

    if bool_at(constraints, "require_temporary_support_when_declared") == Some(true)
        && str_at(observations, "temporary_support_state") == Some("missing")
    {
        blocker(&mut blockers, true, "TEMPORARY_SUPPORT_MISSING");
    }

    if bool_at(constraints, "require_declared_lift_plan") == Some(true)
        && str_at(observations, "lift_plan_state") != Some("verified")
    {
        blocker(&mut blockers, true, "LIFT_PLAN_NOT_SUPPLIED");
    }

    if bool_at(observations, "required_surface_visible") == Some(false) {
        let code = if profile == "P0_AERIAL_FOREST_SCOUT" {
            "REQUIRED_FEATURE_OCCLUDED"
        } else {
            "REQUIRED_SURFACE_OCCLUDED"
        };
        blocker(&mut blockers, true, code);
    }

    if profile == "P0_AERIAL_FOREST_SCOUT" {
        blocker(
            &mut blockers,
            str_at(observations, "flight_permission_state") != Some("verified"),
            "FLIGHT_PERMISSION_NOT_VERIFIED",
        );
        blocker(
            &mut blockers,
            str_at(observations, "site_permission_state") != Some("granted"),
            "SITE_PERMISSION_NOT_VERIFIED",
        );
        blocker(
            &mut blockers,
            str_at(observations, "airspace_state") != Some("clear")
                || str_at(observations, "geofence_state") != Some("valid"),
            "AIRSPACE_OR_GEOFENCE_UNCLEARED",
        );
        blocker(
            &mut blockers,
            str_at(observations, "weather_state") != Some("within_declared_envelope"),
            "WEATHER_OUTSIDE_DECLARED_ENVELOPE",
        );
        blocker(
            &mut blockers,
            str_at(observations, "wildlife_policy_state")
                != Some("approved_site_specific_profile"),
            "WILDLIFE_POLICY_NOT_APPROVED",
        );
        if str_at(observations, "wildlife_disturbance_state") != Some("none_detected") {
            blocker(&mut blockers, true, "WILDLIFE_DISTURBANCE_DETECTED");
            blocker(
                &mut blockers,
                true,
                "NO_REPEAT_SORTIE_WITHOUT_HUMAN_REVIEW",
            );
        }
        blocker(
            &mut blockers,
            str_at(observations, "privacy_handling_state")
                != Some("configured_data_minimization"),
            "PRIVACY_HANDLING_NOT_CONFIGURED",
        );
        blocker(
            &mut blockers,
            str_at(observations, "recovery_plan_state")
                != Some("verified_designated_recovery_area"),
            "SAFE_RECOVERY_PLAN_UNRESOLVED",
        );
        blocker(
            &mut blockers,
            str_at(observations, "launch_recovery_state")
                != Some("approved_existing_clearing"),
            "LAUNCH_RECOVERY_AREA_NOT_APPROVED",
        );
        blocker(
            &mut blockers,
            str_at(observations, "flight_profile") != Some("straight_pass_no_hover"),
            "UNDER_CANOPY_NOT_ALLOWED_IN_DEFAULT_PROFILE",
        );
        blocker(
            &mut blockers,
            match (
                u64_at(observations, "pose_uncertainty_mm"),
                u64_at(constraints, "maximum_pose_uncertainty_mm"),
            ) {
                (Some(actual), Some(maximum)) => actual > maximum,
                _ => true,
            },
            "POSE_UNCERTAINTY_EXCEEDS_LIMIT",
        );
        if bool_at(observations, "contact_detected") != Some(false) {
            blocker(&mut blockers, true, "PHYSICAL_CONTACT_DETECTED");
            blocker(&mut blockers, true, "NO_CONTINUED_MISSION_AFTER_CONTACT");
        }
        blocker(
            &mut blockers,
            str_at(observations, "payload_release_state") != Some("none"),
            "PAYLOAD_RELEASE_PROHIBITED",
        );
        blocker(
            &mut blockers,
            str_at(observations, "localization_state") != Some("valid"),
            "LOCALIZATION_LOST",
        );
        blocker(
            &mut blockers,
            str_at(observations, "human_approval") != Some("approved"),
            "HUMAN_APPROVAL_NOT_GRANTED",
        );
        blocker(
            &mut blockers,
            match (
                u64_at(observations, "altitude_m"),
                u64_at(observations, "configured_min_altitude_m"),
            ) {
                (Some(actual), Some(minimum)) => actual < minimum,
                _ => true,
            },
            "SITE_STANDOFF_POLICY_VIOLATED",
        );
        if str_at(observations, "link_state") == Some("lost") {
            blocker(&mut blockers, true, "LINK_LOST");
            blocker(&mut blockers, true, "USE_PREDECLARED_RECOVERY_ONLY");
        }
    }

    if profile == "P6_REUSE_REMANUFACTURE_STATION"
        && [
            "screen_deadwood_for_recovery",
            "compare_extraction_modes",
            "screen_declining_live_tree",
        ]
        .contains(&task)
    {
        blocker(
            &mut blockers,
            str_at(observations, "harvest_permission_state") != Some("verified"),
            "HARVEST_PERMISSION_UNVERIFIED",
        );
        blocker(
            &mut blockers,
            str_at(observations, "landholder_consent_state") != Some("verified"),
            "LANDHOLDER_CONSENT_UNVERIFIED",
        );
        blocker(
            &mut blockers,
            str_at(observations, "protected_species_status") != Some("checked_not_protected"),
            "PROTECTED_SPECIES_STATUS_UNKNOWN",
        );

        match str_at(observations, "deadwood_retention_disposition") {
            Some("retain_habitat") => {
                blocker(&mut blockers, true, "DEADWOOD_RETENTION_REQUIRED");
            }
            Some("approved_for_removal_after_ecological_review") => {}
            _ => blocker(&mut blockers, true, "ECOLOGICAL_REVIEW_INCOMPLETE"),
        }
        blocker(
            &mut blockers,
            str_at(observations, "biodiversity_review_state") != Some("complete"),
            "ECOLOGICAL_REVIEW_INCOMPLETE",
        );
        blocker(
            &mut blockers,
            str_at(observations, "hazard_review_state") != Some("complete"),
            "HAZARD_REVIEW_INCOMPLETE",
        );
        if str_at(observations, "pest_quarantine_state") != Some("cleared") {
            blocker(&mut blockers, true, "BIOSECURITY_HOLD");
            blocker(&mut blockers, true, "NO_MOVEMENT_BEFORE_CLEARANCE");
        }
        if str_at(observations, "tree_condition") == Some("declining_live")
            && str_at(observations, "separate_silvicultural_decision") != Some("approved")
        {
            blocker(
                &mut blockers,
                true,
                "LIVE_TREE_REMOVAL_REQUIRES_SEPARATE_APPROVAL",
            );
        }

        if task == "screen_deadwood_for_recovery"
            && str_at(scenario, "expected_decision") == Some(ELIGIBLE)
            && (!has_reason(scenario, "SCREENING_ONLY")
                || !has_reason(scenario, "NO_EXTRACTION_AUTHORIZED"))
        {
            quality_findings.push("EXTRACTION_AUTHORITY_BOUNDARY_MISSING".to_owned());
        }

        if task == "compare_extraction_modes" {
            let feasible = bool_at(observations, "ground_or_cable_access_feasible");
            let selected = str_at(observations, "mode_selected");
            let helicopter_cost = u64_at(observations, "helicopter_all_in_cost_cents_per_m3");
            let cable_cost = u64_at(observations, "cable_yarder_all_in_cost_cents_per_m3");
            if feasible == Some(true) {
                if let (Some(heli), Some(cable)) = (helicopter_cost, cable_cost) {
                    if cable < heli && selected != Some("cable_yarder") {
                        quality_findings.push("CHEAPER_FEASIBLE_CABLE_NOT_SELECTED".to_owned());
                    }
                    if cable < heli
                        && !has_reason(scenario, "CABLE_YARDER_SELECTED_LOWER_DECLARED_COST")
                    {
                        quality_findings.push("COST_RATIONALE_NOT_RECORDED".to_owned());
                    }
                }
            } else if feasible == Some(false)
                && selected != Some("helicopter_candidate_for_operator_review")
            {
                quality_findings.push("UNAUTHORIZED_HELICOPTER_DECISION".to_owned());
            }

            if str_at(scenario, "expected_decision") == Some(ELIGIBLE)
                && !has_reason(scenario, "NO_EXTRACTION_AUTHORIZED")
            {
                quality_findings.push("EXTRACTION_AUTHORITY_BOUNDARY_MISSING".to_owned());
            }
        }
    }

    // A loaded or presented tool mismatch is an unconditional task gate when it exists.
    // Missing observation fields are not silently replaced with a favorable default above.
    (blockers, quality_findings)
}

fn audit_scenario(scenario: &Value) -> Result<ScenarioAudit, String> {
    validate_scenario_shape(scenario)?;
    let scenario_id = str_at(scenario, "scenario_id").unwrap_or_default().to_owned();
    let decision = str_at(scenario, "expected_decision")
        .unwrap_or_default()
        .to_owned();
    let (blockers, quality_findings) = evaluate_blockers(scenario);
    let mut violations = Vec::new();

    let known_decisions = [
        "eligible_for_simulated_plan",
        "blocked",
        "insufficient_evidence",
        "blocked_safe_degraded_state",
    ];
    if !known_decisions.contains(&decision.as_str()) {
        push_once(&mut violations, "UNKNOWN_DECISION");
    }

    for code in &blockers {
        if !has_reason(scenario, code) {
            push_once(&mut violations, &format!("MISSING_REASON_CODE:{code}"));
        }
    }

    if bool_at(scenario, "expected_actuation") != Some(false) {
        push_once(&mut violations, "PHYSICAL_ACTUATION_NOT_FALSE");
    }

    if decision == ELIGIBLE && !blockers.is_empty() {
        push_once(&mut violations, "UNSAFE_ELIGIBILITY_CLAIM");
    } else if decision != ELIGIBLE && blockers.is_empty() && quality_findings.is_empty() {
        push_once(&mut violations, "NON_ELIGIBLE_WITHOUT_INDEPENDENT_FINDING");
    }

    if !quality_findings.is_empty() {
        push_once(&mut violations, "COST_OR_AUTHORITY_CONTRACT_VIOLATION");
    }

    Ok(ScenarioAudit {
        scenario_id,
        decision,
        blockers,
        quality_findings,
        violations,
    })
}

fn apply_mutation(
    source: &Value,
    case: &Value,
) -> Result<Value, String> {
    let mut scenario = source.clone();
    let operator = str_at(case, "operator").ok_or("mutation case missing operator")?;
    match operator {
        "set_observation" => {
            let field = str_at(case, "field").ok_or("set_observation missing field")?;
            scenario["observations"][field] = case
                .get("value")
                .ok_or("set_observation missing value")?
                .clone();
        }
        "set_expected_decision" => {
            scenario["expected_decision"] = case
                .get("value")
                .ok_or("set_expected_decision missing value")?
                .clone();
        }
        other => return Err(format!("unsupported mutation operator {other}")),
    }
    Ok(scenario)
}

fn analyze_corpus(corpus: &Value, matrix: &Value) -> Result<Receipt, String> {
    if str_at(corpus, "suite_id") != Some("WOOD_ROBOT_PLATFORM")
        || str_at(corpus, "suite_version") != Some("0.1.0")
        || str_at(corpus, "evidence_class") != Some("synthetic")
        || str_at(corpus, "execution_mode") != Some("plan_only_no_actuation")
    {
        return Err("corpus identity or execution boundary is not the pinned synthetic plan-only suite".to_owned());
    }
    let ceiling = str_at(corpus, "claim_ceiling").unwrap_or_default().to_lowercase();
    if !ceiling.contains("no robot performance") || !ceiling.contains("physical safety") {
        return Err("corpus claim ceiling must deny robot-performance and physical-safety claims".to_owned());
    }

    let scenarios = corpus["scenarios"]
        .as_array()
        .ok_or("corpus scenarios must be an array")?;
    if scenarios.len() != EXPECTED_SCENARIOS
        || u64_at(corpus, "expected_scenario_count") != Some(EXPECTED_SCENARIOS as u64)
    {
        return Err(format!(
            "scenario-count mismatch: expected {EXPECTED_SCENARIOS}, found {}",
            scenarios.len()
        ));
    }

    if str_at(matrix, "schema_version") != Some("robotics-qualification-mutation-matrix-v0.1")
        || str_at(matrix, "status") != Some("synthetic_adversarial_contract_cases")
        || bool_at(matrix, "physical_actuation_authorized") != Some(false)
    {
        return Err("mutation matrix identity or claim boundary is invalid".to_owned());
    }
    let cases = matrix["cases"]
        .as_array()
        .ok_or("mutation matrix cases must be an array")?;
    if cases.len() != EXPECTED_MUTATIONS
        || u64_at(matrix, "case_count") != Some(EXPECTED_MUTATIONS as u64)
    {
        return Err(format!(
            "mutation-count mismatch: expected {EXPECTED_MUTATIONS}, found {}",
            cases.len()
        ));
    }

    let mut ids = BTreeSet::new();
    let mut scenario_by_id = BTreeMap::<String, &Value>::new();
    for scenario in scenarios {
        let id = str_at(scenario, "scenario_id").ok_or("scenario missing scenario_id")?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate scenario id {id}"));
        }
        scenario_by_id.insert(id.to_owned(), scenario);
    }

    let mut scenario_violations = Vec::new();
    let mut blocker_counts = BTreeMap::<String, usize>::new();
    let mut quality_finding_counts = BTreeMap::<String, usize>::new();

    for scenario in scenarios {
        let audit = audit_scenario(scenario)?;
        for code in &audit.blockers {
            *blocker_counts.entry(code.clone()).or_insert(0) += 1;
        }
        for code in &audit.quality_findings {
            *quality_finding_counts.entry(code.clone()).or_insert(0) += 1;
        }
        for violation in audit.violations {
            scenario_violations.push(format!("{}:{violation}", audit.scenario_id));
        }
    }

    let mut mutation_failures = Vec::new();
    let mut case_ids = BTreeSet::new();
    for case in cases {
        let case_id = str_at(case, "case_id").ok_or("mutation missing case_id")?;
        if !case_ids.insert(case_id.to_owned()) {
            return Err(format!("duplicate mutation case id {case_id}"));
        }
        let source_id =
            str_at(case, "source_scenario_id").ok_or("mutation missing source_scenario_id")?;
        let source = scenario_by_id
            .get(source_id)
            .ok_or_else(|| format!("{case_id}: unknown source scenario {source_id}"))?;
        let changed = apply_mutation(source, case)?;
        let expected_finding =
            str_at(case, "expected_finding").ok_or("mutation missing expected_finding")?;
        let audit = audit_scenario(&changed)?;
        let finding_observed = audit.blockers.iter().any(|x| x == expected_finding)
            || audit.quality_findings.iter().any(|x| x == expected_finding);
        if !finding_observed {
            mutation_failures.push(format!(
                "{case_id}: expected finding {expected_finding} absent; blockers={:?}; quality={:?}",
                audit.blockers, audit.quality_findings
            ));
        }
        if case["operator"] == "set_expected_decision"
            || (case["operator"] == "set_observation" && !audit.blockers.is_empty())
        {
            if !audit.violations.iter().any(|v| v == "UNSAFE_ELIGIBILITY_CLAIM") {
                mutation_failures.push(format!(
                    "{case_id}: hazard was not prevented from retaining simulated-plan eligibility"
                ));
            }
        }
        if !audit.quality_findings.is_empty()
            && !audit
                .violations
                .iter()
                .any(|v| v == "COST_OR_AUTHORITY_CONTRACT_VIOLATION")
        {
            mutation_failures.push(format!("{case_id}: quality/authority violation was not surfaced"));
        }
    }

    Ok(Receipt {
        source_commit: String::new(),
        corpus_sha256: String::new(),
        mutation_matrix_sha256: String::new(),
        scenario_count: scenarios.len(),
        mutation_count: cases.len(),
        blocker_counts,
        quality_finding_counts,
        scenario_violations,
        mutation_failures,
        claim_ceiling: SAFETY_CEILING,
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn run() -> Result<(Receipt, bool), String> {
    let mut args = env::args().skip(1);
    let corpus_path = args
        .next()
        .ok_or("usage: robotics-qualification-oracle <corpus.json> <mutation-matrix.json> --source-commit <40-hex-sha>")?;
    let matrix_path = args
        .next()
        .ok_or("usage: robotics-qualification-oracle <corpus.json> <mutation-matrix.json> --source-commit <40-hex-sha>")?;
    let flag = args.next().ok_or("expected --source-commit <40-hex-sha>")?;
    if flag != "--source-commit" {
        return Err("expected --source-commit <40-hex-sha>".to_owned());
    }
    let source_commit = args.next().ok_or("missing source commit SHA")?;
    if !valid_commit_sha(&source_commit) || args.next().is_some() {
        return Err("invalid source commit SHA or unexpected arguments".to_owned());
    }

    let corpus_bytes =
        fs::read(&corpus_path).map_err(|error| format!("cannot read {corpus_path}: {error}"))?;
    let matrix_bytes =
        fs::read(&matrix_path).map_err(|error| format!("cannot read {matrix_path}: {error}"))?;
    let corpus: Value = serde_json::from_slice(&corpus_bytes)
        .map_err(|error| format!("invalid corpus JSON: {error}"))?;
    let matrix: Value = serde_json::from_slice(&matrix_bytes)
        .map_err(|error| format!("invalid mutation matrix JSON: {error}"))?;

    let mut receipt = analyze_corpus(&corpus, &matrix)?;
    receipt.source_commit = source_commit.to_ascii_lowercase();
    receipt.corpus_sha256 = sha256(&corpus_bytes);
    receipt.mutation_matrix_sha256 = sha256(&matrix_bytes);

    let passed = receipt.scenario_violations.is_empty() && receipt.mutation_failures.is_empty();
    Ok((receipt, passed))
}

fn main() -> ExitCode {
    match run() {
        Ok((receipt, passed)) => {
            let result = serde_json::json!({
                "receipt_version": "robotics-qualification-oracle-receipt-v0.1",
                "result": if passed { "pass" } else { "fail" },
                "source_commit": receipt.source_commit,
                "corpus_sha256": receipt.corpus_sha256,
                "mutation_matrix_sha256": receipt.mutation_matrix_sha256,
                "scenario_count": receipt.scenario_count,
                "mutation_count": receipt.mutation_count,
                "blocker_counts": receipt.blocker_counts,
                "quality_finding_counts": receipt.quality_finding_counts,
                "scenario_violations": receipt.scenario_violations,
                "mutation_failures": receipt.mutation_failures,
                "claim_ceiling": receipt.claim_ceiling,
                "external_domain_adjudication": false,
                "blind_holdout": false
            });
            println!("{}", serde_json::to_string_pretty(&result).expect("receipt serializes"));
            if passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
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
    use serde_json::{json, Value};

    fn frozen_corpus() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/tech/wood-robot-platform-corpus-v0.1.json"
        ))
        .expect("frozen robotics corpus parses")
    }

    fn frozen_matrix() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/tech/robotics-qualification-mutation-matrix-v0.1.json"
        ))
        .expect("robotics mutation matrix parses")
    }

    #[test]
    fn frozen_corpus_satisfies_independent_safety_floor() {
        let corpus = frozen_corpus();
        let matrix = frozen_matrix();
        let receipt = analyze_corpus(&corpus, &matrix).expect("pinned corpus is structurally valid");
        assert!(
            receipt.scenario_violations.is_empty(),
            "unexpected scenario violations: {:?}",
            receipt.scenario_violations
        );
        assert_eq!(receipt.scenario_count, EXPECTED_SCENARIOS);
        assert_eq!(receipt.mutation_count, EXPECTED_MUTATIONS);
        println!("robotics_oracle_frozen_scenarios={EXPECTED_SCENARIOS}");
    }

    #[test]
    fn all_declared_mutations_are_detected() {
        let corpus = frozen_corpus();
        let matrix = frozen_matrix();
        let receipt = analyze_corpus(&corpus, &matrix).expect("matrix structure is valid");
        assert!(
            receipt.mutation_failures.is_empty(),
            "missed mutations: {:?}",
            receipt.mutation_failures
        );
        println!("robotics_oracle_mutation_cases={EXPECTED_MUTATIONS}");
    }

    #[test]
    fn cost_oracle_never_recommends_helicopter_over_cheaper_feasible_cable() {
        let mut corpus = frozen_corpus();
        let scenarios = corpus["scenarios"]
            .as_array_mut()
            .expect("scenario array");
        let scenario = scenarios
            .iter_mut()
            .find(|scenario| {
                str_at(scenario, "scenario_id")
                    == Some("SYNTHETIC_WOODROBOT_027_AIRLIFT_DOMINATED_BY_CABLE")
            })
            .expect("cost comparison scenario exists");
        scenario["observations"]["mode_selected"] =
            json!("helicopter_candidate_for_operator_review");
        let audit = audit_scenario(scenario).expect("mutated scenario parses");
        assert!(audit
            .quality_findings
            .iter()
            .any(|finding| finding == "CHEAPER_FEASIBLE_CABLE_NOT_SELECTED"));
        assert!(audit
            .violations
            .iter()
            .any(|violation| violation == "COST_OR_AUTHORITY_CONTRACT_VIOLATION"));
    }

    #[test]
    fn screening_and_extraction_never_authorize_actuation() {
        let corpus = frozen_corpus();
        for scenario in corpus["scenarios"].as_array().expect("scenario array") {
            let task = str_at(scenario, "task_type").unwrap_or_default();
            if str_at(scenario, "platform_profile")
                == Some("P6_REUSE_REMANUFACTURE_STATION")
                && ["screen_deadwood_for_recovery", "compare_extraction_modes"]
                    .contains(&task)
            {
                assert_eq!(bool_at(scenario, "expected_actuation"), Some(false));
                if str_at(scenario, "expected_decision") == Some(ELIGIBLE) {
                    assert!(has_reason(scenario, "NO_EXTRACTION_AUTHORIZED"));
                }
            }
        }
    }

    #[test]
    fn exact_frozen_corpus_and_matrix_digests_are_sensitive_to_byte_changes() {
        let corpus_bytes = include_bytes!(
            "../../../docs/tech/wood-robot-platform-corpus-v0.1.json"
        );
        let matrix_bytes = include_bytes!(
            "../../../docs/tech/robotics-qualification-mutation-matrix-v0.1.json"
        );
        let mut changed_corpus = corpus_bytes.to_vec();
        changed_corpus.push(b' ');
        let mut changed_matrix = matrix_bytes.to_vec();
        changed_matrix.push(b' ');
        assert_ne!(sha256(corpus_bytes), sha256(&changed_corpus));
        assert_ne!(sha256(matrix_bytes), sha256(&changed_matrix));
    }
}
