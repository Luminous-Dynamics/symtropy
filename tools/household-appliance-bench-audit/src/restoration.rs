use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:repair-restoration-evidence:0.1.0";
const LEDGER_ID: &str = "REPAIR-RESTORE-001";
const EXPECTED_EPISODES: [&str; 5] = [
    "SYN-REST-001",
    "SYN-REST-002",
    "SYN-REST-003",
    "SYN-REST-004",
    "SYN-REST-005",
];
const EXPECTED_BENCHMARKS: [&str; 7] = [
    "RB-001", "RB-002", "RB-003", "RB-004", "RB-005", "RB-006", "RB-007",
];
const EXPECTED_FAMILIES: [&str; 9] = [
    "F1-WASHER",
    "F2-DISHWASHER",
    "F3-REFRIGERATOR-FREEZER",
    "F4-HEAT-PUMP-DRYER",
    "F5-WASHER-DRYER-COMBO",
    "F6-HEAT-PUMP-WATER-HEATER",
    "F7-RESIDENTIAL-HVAC",
    "F8-COOKING-APPLIANCES",
    "F9-VENTILATION-HRV",
];
const COST_FIELDS: [&str; 9] = [
    "parts_minor",
    "labour_minor",
    "travel_minor",
    "freight_minor",
    "failed_attempts_minor",
    "downtime_minor",
    "rework_minor",
    "waste_handling_minor",
    "discarded_material_grams",
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RestorationAuditSummary {
    pub(crate) episode_count: usize,
    pub(crate) synthetic_episode_count: usize,
    pub(crate) empirical_episode_count: usize,
    pub(crate) started_attempt_count: usize,
    pub(crate) verified_restoration_example_count: usize,
    pub(crate) completed_followup_example_count: usize,
    pub(crate) recurrence_example_count: usize,
    pub(crate) durable_success_example_count: usize,
    pub(crate) benchmark_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) ledger_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("restoration schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("restoration schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "restoration schema does not validate against the Draft 2020-12 meta-schema".to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile restoration schema: {error}"))
}

fn audit_values(
    schema: &Value,
    ledger: &Value,
    schema_raw: &str,
    ledger_raw: &str,
) -> Result<RestorationAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(ledger)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(ledger, "schema_version") != Some("0.1.0")
        || text_at(ledger, "ledger_id") != Some(LEDGER_ID)
        || text_at(ledger, "status") != Some("synthetic_contract_examples_only")
        || text_at(ledger, "claim_ceiling")
            != Some("synthetic_examples_are_not_empirical_or_product_safety_evidence")
    {
        errors.push(
            "canonical restoration fixture must remain synthetic-only and keep its claim ceiling"
                .to_owned(),
        );
    }

    let Some(episodes) = ledger.get("episodes").and_then(Value::as_array) else {
        return Err(vec!["restoration episodes must be an array".to_owned()]);
    };
    if episodes.len() != EXPECTED_EPISODES.len() {
        errors.push(format!(
            "synthetic contract fixture must retain {} episodes",
            EXPECTED_EPISODES.len()
        ));
    }

    let mut episode_ids = BTreeSet::new();
    let mut unit_tokens = BTreeSet::new();
    let mut started_attempt_count = 0usize;
    let mut verified_restoration_count = 0usize;
    let mut completed_followup_count = 0usize;
    let mut recurrence_count = 0usize;
    let mut durable_success_count = 0usize;
    let mut empirical_count = 0usize;

    for (index, episode) in episodes.iter().enumerate() {
        let Some(episode_id) = text_at(episode, "episode_id") else {
            errors.push(format!("episodes[{index}].episode_id must be a string"));
            continue;
        };
        if !episode_ids.insert(episode_id.to_owned()) {
            errors.push(format!("duplicate repair episode ID {episode_id}"));
        }
        if EXPECTED_EPISODES.get(index).copied() != Some(episode_id) {
            errors.push(format!(
                "synthetic episode order/identity changed at index {index}: {episode_id}"
            ));
        }
        if text_at(episode, "evidence_class") != Some("synthetic_fixture") {
            errors.push(format!(
                "{episode_id}: canonical example must remain synthetic"
            ));
        }
        if bool_at(episode, "empirical_eligible") != Some(false) {
            empirical_count += 1;
            errors.push(format!(
                "{episode_id}: synthetic fixture cannot be promoted to empirical evidence"
            ));
        }

        let Some(appliance) = episode.get("appliance") else {
            errors.push(format!("{episode_id}: appliance identity is required"));
            continue;
        };
        let family_id = text_at(appliance, "family_id").unwrap_or("");
        if !EXPECTED_FAMILIES.contains(&family_id) {
            errors.push(format!(
                "{episode_id}: unknown appliance family {family_id}"
            ));
        }
        if let Some(unit_token) = text_at(appliance, "unit_token") {
            if !unit_tokens.insert(unit_token.to_owned()) {
                errors.push(format!(
                    "{episode_id}: duplicate synthetic unit token {unit_token}"
                ));
            }
        }

        let Some(intervention) = episode.get("intervention") else {
            errors.push(format!("{episode_id}: intervention object is required"));
            continue;
        };
        let started = bool_at(intervention, "repair_attempt_started").unwrap_or(false);
        let intervention_status = text_at(intervention, "status").unwrap_or("");
        let outcome = text_at(episode, "outcome").unwrap_or("");
        if started {
            started_attempt_count += 1;
            if intervention_status == "not_started" {
                errors.push(format!(
                    "{episode_id}: a counted repair attempt cannot have status not_started"
                ));
            }
        } else if intervention_status != "not_started" {
            errors.push(format!(
                "{episode_id}: intervention status {intervention_status} contradicts repair_attempt_started=false"
            ));
        }

        let Some(verification) = episode.get("verification") else {
            errors.push(format!("{episode_id}: verification object is required"));
            continue;
        };
        let function_gate = text_at(verification, "core_function_gate").unwrap_or("");
        let safety_gate = text_at(verification, "safety_gate").unwrap_or("");
        let required_tests = verification
            .get("required_test_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let passed_tests = verification
            .get("passed_test_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let measurement_receipts = verification
            .get("measurement_receipt_ids")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if passed_tests > required_tests {
            errors.push(format!(
                "{episode_id}: passed_test_count cannot exceed required_test_count"
            ));
        }
        if outcome == "verified_restoration" {
            verified_restoration_count += 1;
            if !started
                || intervention_status != "completed"
                || function_gate != "passed"
                || safety_gate != "passed"
                || required_tests == 0
                || passed_tests < required_tests
                || measurement_receipts == 0
                || text_at(verification, "useful_output_metric_id").is_none_or(str::is_empty)
            {
                errors.push(format!(
                    "{episode_id}: verified restoration requires a started/completed attempt, passed function and safety gates, all required tests, a useful-output metric, and measurement receipts"
                ));
            }
        }
        if outcome == "failed_repair" && (!started || function_gate == "passed") {
            errors.push(format!(
                "{episode_id}: failed_repair must be a started attempt that did not pass the core-function gate"
            ));
        }
        if outcome == "diagnosis_only" && started {
            errors.push(format!(
                "{episode_id}: diagnosis_only cannot be used to hide a started repair attempt"
            ));
        }
        let diagnosis = episode.get("diagnosis");
        let diagnosis_status = diagnosis.and_then(|value| text_at(value, "status"));
        let hazard_screen = diagnosis.and_then(|value| text_at(value, "hazard_screen"));
        if outcome == "safety_stop"
            && diagnosis_status != Some("safety_stop")
            && hazard_screen != Some("stop_now")
            && hazard_screen != Some("refer_to_qualified_role")
        {
            errors.push(format!(
                "{episode_id}: safety_stop outcome needs an explicit stop or referral record"
            ));
        }
        if bool_at(verification, "energy_water_improvement_claimed") == Some(true)
            && (function_gate != "passed"
                || safety_gate != "passed"
                || text_at(verification, "useful_output_metric_id").is_none_or(str::is_empty))
        {
            errors.push(format!(
                "{episode_id}: energy/water improvement cannot be claimed without verified useful output and a passed safety gate"
            ));
        }

        let Some(follow_up) = episode.get("follow_up") else {
            errors.push(format!("{episode_id}: follow_up object is required"));
            continue;
        };
        let follow_status = text_at(follow_up, "status").unwrap_or("");
        let recurred = follow_up
            .get("same_fault_recurred")
            .and_then(Value::as_bool);
        let recurrence_is_null = follow_up
            .get("same_fault_recurred")
            .is_none_or(Value::is_null);
        let follow_days = follow_up.get("follow_up_days").and_then(Value::as_f64);
        let exposure = follow_up.get("exposure_value").and_then(Value::as_f64);
        let exposure_unit = text_at(follow_up, "exposure_unit").unwrap_or("");
        if follow_status == "complete" {
            completed_followup_count += 1;
            if recurred.is_none()
                || follow_days.is_none_or(|days| days <= 0.0)
                || exposure.is_none_or(|value| value <= 0.0)
                || !matches!(
                    exposure_unit,
                    "operating_hours" | "cycles" | "heating_cycles" | "compressor_hours"
                )
            {
                errors.push(format!(
                    "{episode_id}: completed follow-up requires a known recurrence result, positive calendar duration and positive product-use exposure"
                ));
            }
            if recurred == Some(true) {
                recurrence_count += 1;
            }
            if outcome == "verified_restoration" && recurred == Some(false) {
                durable_success_count += 1;
            }
        } else if !recurrence_is_null {
            errors.push(format!(
                "{episode_id}: censored, pending, lost or not-applicable follow-up cannot be recorded as no recurrence or recurrence"
            ));
        }
        if follow_status == "not_applicable"
            && (follow_days.is_some() || exposure.is_some() || exposure_unit != "not_applicable")
        {
            errors.push(format!(
                "{episode_id}: not_applicable follow-up must not invent a duration or exposure"
            ));
        }

        let Some(costs) = episode.get("costs") else {
            errors.push(format!("{episode_id}: costs object is required"));
            continue;
        };
        let cost_status = text_at(costs, "observation_status").unwrap_or("");
        let currency = text_at(costs, "currency").unwrap_or("");
        let known_cost_count = COST_FIELDS
            .iter()
            .filter(|field| costs.get(**field).is_some_and(|value| !value.is_null()))
            .count();
        match cost_status {
            "complete" if known_cost_count != COST_FIELDS.len() => errors.push(format!(
                "{episode_id}: complete cost record contains unknown cost fields"
            )),
            "partial" if known_cost_count == 0 || known_cost_count == COST_FIELDS.len() => {
                errors.push(format!(
                    "{episode_id}: partial costs must contain both known and unknown values"
                ));
            }
            "unknown" if known_cost_count != 0 || currency != "unknown" => errors.push(format!(
                "{episode_id}: unknown costs must be null, never encoded as zero, with currency unknown"
            )),
            _ => {}
        }

        let preregistration = episode.get("preregistration").unwrap_or(&Value::Null);
        if bool_at(preregistration, "frozen_before_outcome") != Some(true) {
            errors.push(format!(
                "{episode_id}: protocol must be frozen before any eligible outcome"
            ));
        }
        let evidence_receipts = episode
            .get("evidence_receipt_ids")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if evidence_receipts == 0 {
            errors.push(format!(
                "{episode_id}: evidence receipt identity cannot be omitted"
            ));
        }
    }

    for expected in EXPECTED_EPISODES {
        if !episode_ids.contains(expected) {
            errors.push(format!("required synthetic episode {expected} is missing"));
        }
    }
    if empirical_count != 0 {
        errors.push("synthetic fixture must report zero empirical episodes".to_owned());
    }

    let Some(benchmarks) = ledger.get("proposed_benchmarks").and_then(Value::as_array) else {
        return Err(vec!["proposed_benchmarks must be an array".to_owned()]);
    };
    if benchmarks.len() != EXPECTED_BENCHMARKS.len() {
        errors.push(format!(
            "fixture must retain {} proposed benchmark definitions",
            EXPECTED_BENCHMARKS.len()
        ));
    }
    let mut benchmark_ids = BTreeSet::new();
    for (index, benchmark) in benchmarks.iter().enumerate() {
        let Some(id) = text_at(benchmark, "id") else {
            errors.push(format!("proposed_benchmarks[{index}].id must be a string"));
            continue;
        };
        if !benchmark_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate benchmark ID {id}"));
        }
        if EXPECTED_BENCHMARKS.get(index).copied() != Some(id) {
            errors.push(format!(
                "benchmark order/identity changed at index {index}: {id}"
            ));
        }
        if text_at(benchmark, "status").is_none_or(|status| {
            !matches!(status, "proposal_not_frozen" | "guardrail_contract_only")
        }) {
            errors.push(format!(
                "{id}: benchmark must remain a proposal or contract guardrail"
            ));
        }
    }
    for expected in EXPECTED_BENCHMARKS {
        if !benchmark_ids.contains(expected) {
            errors.push(format!("required benchmark {expected} is missing"));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(RestorationAuditSummary {
        episode_count: episodes.len(),
        synthetic_episode_count: episodes
            .iter()
            .filter(|episode| text_at(episode, "evidence_class") == Some("synthetic_fixture"))
            .count(),
        empirical_episode_count: empirical_count,
        started_attempt_count,
        verified_restoration_example_count: verified_restoration_count,
        completed_followup_example_count: completed_followup_count,
        recurrence_example_count: recurrence_count,
        durable_success_example_count: durable_success_count,
        benchmark_count: benchmarks.len(),
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        ledger_sha256: format!("{:x}", Sha256::digest(ledger_raw.as_bytes())),
    })
}

pub(crate) fn audit_restoration_texts(
    schema_raw: &str,
    ledger_raw: &str,
) -> Result<RestorationAuditSummary, Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("restoration schema JSON parse failed: {error}")])?;
    let ledger: Value = serde_json::from_str(ledger_raw)
        .map_err(|error| vec![format!("restoration ledger JSON parse failed: {error}")])?;
    audit_values(&schema, &ledger, schema_raw, ledger_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str =
        include_str!("../../../schemas/appliances/repair_restoration_evidence_v0_1.schema.json");
    const LEDGER: &str =
        include_str!("../../../fixtures/appliances/repair_restoration_evidence_v0_1.json");

    fn canonical_ledger() -> Value {
        serde_json::from_str(LEDGER).expect("restoration fixture must parse")
    }

    fn audit_mutation(ledger: &Value) -> Result<RestorationAuditSummary, Errors> {
        let schema: Value = serde_json::from_str(SCHEMA).expect("restoration schema must parse");
        let schema_raw = serde_json::to_string(&schema).expect("schema serializes");
        let ledger_raw = serde_json::to_string(ledger).expect("ledger serializes");
        audit_values(&schema, ledger, &schema_raw, &ledger_raw)
    }

    #[test]
    fn canonical_ledger_counts_attempts_separately_from_durable_success() {
        let summary = audit_restoration_texts(SCHEMA, LEDGER)
            .expect("canonical synthetic restoration ledger should qualify");
        assert_eq!(summary.episode_count, 5);
        assert_eq!(summary.synthetic_episode_count, 5);
        assert_eq!(summary.empirical_episode_count, 0);
        assert_eq!(summary.started_attempt_count, 4);
        assert_eq!(summary.verified_restoration_example_count, 3);
        assert_eq!(summary.completed_followup_example_count, 2);
        assert_eq!(summary.recurrence_example_count, 1);
        assert_eq!(summary.durable_success_example_count, 1);
        assert_eq!(summary.benchmark_count, 7);
    }

    #[test]
    fn synthetic_episode_cannot_be_promoted_to_empirical_evidence() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][0]["empirical_eligible"] = json!(true);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn failed_started_attempt_cannot_disappear_from_denominator() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][1]["intervention"]["repair_attempt_started"] = json!(false);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn verified_restoration_requires_both_hard_gates_and_test_receipts() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][0]["verification"]["safety_gate"] = json!("unknown");
        assert!(audit_mutation(&ledger).is_err());
        ledger = canonical_ledger();
        ledger["episodes"][0]["verification"]["measurement_receipt_ids"] = json!([]);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn energy_or_water_claim_cannot_compensate_for_failed_core_function() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][1]["verification"]["energy_water_improvement_claimed"] = json!(true);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn censored_or_lost_followup_cannot_be_encoded_as_no_recurrence() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][3]["follow_up"]["same_fault_recurred"] = json!(false);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn unknown_cost_is_null_not_zero() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][2]["costs"]["parts_minor"] = json!(0);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn failed_function_test_cannot_be_called_a_verified_restoration() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][0]["verification"]["passed_test_count"] = json!(2);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn duplicate_episode_identity_is_rejected() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][1]["episode_id"] = json!("SYN-REST-001");
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn duplicate_or_promoted_benchmark_identity_is_rejected() {
        let mut ledger = canonical_ledger();
        ledger["proposed_benchmarks"][1]["id"] = json!("RB-001");
        assert!(audit_mutation(&ledger).is_err());
        ledger = canonical_ledger();
        ledger["proposed_benchmarks"][0]["status"] = json!("validated_winner");
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn complete_followup_requires_actual_exposure_not_just_calendar_time() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][0]["follow_up"]["exposure_value"] = json!(0);
        assert!(audit_mutation(&ledger).is_err());
    }

    #[test]
    fn unknown_costs_require_every_cost_to_remain_null() {
        let mut ledger = canonical_ledger();
        ledger["episodes"][2]["costs"]["observation_status"] = json!("partial");
        assert!(audit_mutation(&ledger).is_err());
    }
}
