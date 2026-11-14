use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

mod repair_graph_audit;

const CORPUS_SCHEMA: &str = "personal-family-vehicle-scenario-corpus/0.1";
const CORPUS_STATUS: &str = "synthetic-research-fixture-not-validated-physics";
const REQUIRED_IDS: [&str; 21] = [
    "PV01_NOMINAL_DRY",
    "PV02_WET_LOW_MU",
    "PV03_FRICTION_TRANSITION",
    "PV04_SPLIT_MU",
    "PV05_PAYLOAD_CG_SHIFT",
    "PV06_OVERLOAD",
    "PV07_BRAKE_FADE",
    "PV08_REGEN_FRICTION_HANDOFF",
    "PV09_ROUGH_ROAD",
    "PV10_WHEEL_UNLOADING",
    "PV11_TIRE_REVISION_CHANGE",
    "PV12_STEERING_DEGRADATION",
    "PV13_SENSOR_BIAS_STALE_CALIBRATION",
    "PV14_POWERTRAIN_DERATING",
    "PV15_OVERDUE_MAINTENANCE",
    "PV16_INCOMPATIBLE_PART",
    "PV17_INVALID_UNITS",
    "PV18_MISSING_PROVENANCE",
    "PV19_SOLVER_IDENTITY_MUTATION",
    "PV20_NONCONVERGED_SOLVER",
    "PV21_OUT_OF_DOMAIN",
];

const REQUIRED_NEGATIVE_IDS: [&str; 9] = [
    "PV06_OVERLOAD",
    "PV11_TIRE_REVISION_CHANGE",
    "PV13_SENSOR_BIAS_STALE_CALIBRATION",
    "PV16_INCOMPATIBLE_PART",
    "PV17_INVALID_UNITS",
    "PV18_MISSING_PROVENANCE",
    "PV19_SOLVER_IDENTITY_MUTATION",
    "PV20_NONCONVERGED_SOLVER",
    "PV21_OUT_OF_DOMAIN",
];
const REQUIRED_SOURCE_IDS: [&str; 15] = [
    "SRC_EPA_DYNAMOMETER_SCHEDULES",
    "SRC_NHTSA_ADS_SCENARIO_FRAMEWORK",
    "SRC_SAE_J1555_201910",
    "SRC_ISO_21448_2022",
    "SRC_ISO_26262_PART_3_2018",
    "SRC_SA_MIB_REGISTRATION",
    "SRC_SA_NRCs_LETTER_OF_AUTHORITY",
    "SRC_SA_ROADWORTHINESS",
    "SRC_UN_R155",
    "SRC_UN_R156",
    "SRC_SA_DTIC_SKD_POLICY_2026_10",
    "SRC_SA_SARS_APDP2",
    "SRC_TOYOTA_SA_MODEL_RANGE",
    "SRC_SUZUKI_SWIFT_OFFICIAL_PRICES",
    "SRC_SUZUKI_SWIFT_OFFICIAL_SPECS",
];

const REQUIRED_BASELINE_IDS: [&str; 5] = [
    "BASELINE_TOYOTA_VITZ_ENTRY_HATCH",
    "BASELINE_SUZUKI_SWIFT_GL_MT",
    "BASELINE_TOYOTA_STARLET_ENTRY_HATCH",
    "BASELINE_TOYOTA_STARLET_CROSS_ENTRY",
    "BASELINE_TOYOTA_RUMION_ENTRY_MPV",
];

const REQUIRED_EVIDENCE_FIXTURE_IDS: [&str; 15] = [
    "EV01_VALID_DESIGN_HYPOTHESIS",
    "EV02_VALID_SIMULATION",
    "EV03_VALID_BENCH_MEASUREMENT",
    "EV04_VALID_INDEPENDENT_REVIEW",
    "EV05_VALID_REGULATORY_RECEIPT",
    "EV06_SIMULATION_PROMOTED_TO_PHYSICAL",
    "EV07_REVIEW_PROMOTED_TO_REGULATORY",
    "EV08_MISSING_LIMITATIONS",
    "EV09_MISSING_SOURCE",
    "EV10_INVALID_OUTPUT_DIGEST",
    "EV11_UNSCHEMATIZED_SENSITIVE_FIELD",
    "EV12_VEHICLE_CONFIG_WITHOUT_DIGEST",
    "EV13_UNKNOWN_EVIDENCE_CLASS",
    "EV14_WRONG_REVISION_SHAPE",
    "EV15_UNKNOWN_SOURCE_RIGHTS",
];

type AuditResult<T> = Result<T, Vec<String>>;

#[derive(Debug, PartialEq, Eq)]
struct ScenarioSummary {
    case_count: usize,
    negative_control_count: usize,
    sha256: String,
}

#[derive(Debug, PartialEq, Eq)]
struct AuditSummary {
    case_count: usize,
    negative_control_count: usize,
    sha256: String,
    source_registry_count: usize,
    source_registry_sha256: String,
    evidence_fixture_count: usize,
    evidence_schema_sha256: String,
    evidence_fixtures_sha256: String,
    baseline_count: usize,
    baseline_sha256: String,
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn case_by_id<'a>(cases: &'a [Value], id: &str) -> Option<&'a Value> {
    cases.iter().find(|case| string_at(case, "id") == Some(id))
}

#[cfg(test)]
fn case_by_id_mut<'a>(cases: &'a mut [Value], id: &str) -> Option<&'a mut Value> {
    cases
        .iter_mut()
        .find(|case| string_at(case, "id") == Some(id))
}

fn has_invariant(case: &Value, invariant: &str) -> bool {
    case.get("required_invariants")
        .and_then(Value::as_array)
        .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(invariant)))
}

fn check_exact_keys(value: &Value, expected: &[&str], at: &str, errors: &mut Vec<String>) {
    let Some(object) = value.as_object() else {
        errors.push(format!("{at}: expected an object"));
        return;
    };

    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();

    if !missing.is_empty() || !unknown.is_empty() {
        errors.push(format!(
            "{at}: missing keys {missing:?}; unexpected keys {unknown:?}"
        ));
    }
}

fn require_string(value: &Value, key: &str, at: &str, errors: &mut Vec<String>) -> Option<String> {
    match string_at(value, key) {
        Some(text) if !text.trim().is_empty() => Some(text.to_owned()),
        Some(_) => {
            errors.push(format!("{at}.{key}: must not be empty"));
            None
        }
        None => {
            errors.push(format!("{at}.{key}: missing or not a string"));
            None
        }
    }
}

fn validate_special_case(case: &Value, errors: &mut Vec<String>) {
    let Some(id) = string_at(case, "id") else {
        return;
    };
    let inputs = case.get("inputs").unwrap_or(&Value::Null);
    let disposition = string_at(case, "expected_disposition").unwrap_or("");

    match id {
        "PV01_NOMINAL_DRY" => {
            if string_at(case, "kind") != Some("positive_control") {
                errors.push("PV01_NOMINAL_DRY: must remain the positive control".to_owned());
            }
        }
        "PV03_FRICTION_TRANSITION" => {
            let before = number_at(inputs, "road_mu_before");
            let after = number_at(inputs, "road_mu_after");
            if !matches!((before, after), (Some(a), Some(b)) if a > b && b >= 0.0 && a <= 1.0) {
                errors.push(
                    "PV03_FRICTION_TRANSITION: must declare a valid friction drop".to_owned(),
                );
            }
        }
        "PV05_PAYLOAD_CG_SHIFT" => {
            let payload = number_at(inputs, "payload_kg");
            if !matches!(payload, Some(p) if p >= 0.0 && p <= 400.0) {
                errors.push("PV05_PAYLOAD_CG_SHIFT: payload must remain within the synthetic 400 kg envelope".to_owned());
            }
            if number_at(inputs, "payload_cg_z_m").is_none() {
                errors
                    .push("PV05_PAYLOAD_CG_SHIFT: center-of-gravity height is required".to_owned());
            }
        }
        "PV06_OVERLOAD" => {
            let payload = number_at(inputs, "payload_kg");
            let limit = number_at(inputs, "payload_profile_limit_kg");
            if !matches!((payload, limit), (Some(p), Some(l)) if p > l)
                || disposition != "reject_before_simulation"
            {
                errors.push(
                    "PV06_OVERLOAD: overload must exceed limit and be rejected before simulation"
                        .to_owned(),
                );
            }
        }
        "PV11_TIRE_REVISION_CHANGE" => {
            if string_at(inputs, "scenario_tire_revision")
                == string_at(inputs, "calibrated_tire_revision")
                || disposition != "reject_as_mismatched_configuration"
            {
                errors.push(
                    "PV11_TIRE_REVISION_CHANGE: mismatched tire revisions must be rejected"
                        .to_owned(),
                );
            }
        }
        "PV13_SENSOR_BIAS_STALE_CALIBRATION" => {
            let age = number_at(inputs, "calibration_age_days");
            let max_age = number_at(inputs, "max_calibration_age_days");
            if !matches!((age, max_age), (Some(a), Some(m)) if a > m)
                || disposition != "insufficient_evidence"
            {
                errors.push(
                    "PV13_SENSOR_BIAS_STALE_CALIBRATION: stale calibration must cause abstention"
                        .to_owned(),
                );
            }
        }
        "PV16_INCOMPATIBLE_PART" => {
            let installed = string_at(inputs, "installed_part_revision");
            let allowed = inputs
                .get("allowed_part_revisions")
                .and_then(Value::as_array);
            let is_compatible = allowed
                .is_some_and(|values| values.iter().any(|value| value.as_str() == installed));
            if is_compatible || disposition != "reject_as_mismatched_configuration" {
                errors.push(
                    "PV16_INCOMPATIBLE_PART: incompatible part revision must not be accepted"
                        .to_owned(),
                );
            }
        }
        "PV17_INVALID_UNITS" => {
            let speed_unit = inputs
                .get("speed")
                .and_then(|v| v.get("unit"))
                .and_then(Value::as_str);
            if speed_unit != Some("horsepower") || disposition != "reject_before_simulation" {
                errors.push(
                    "PV17_INVALID_UNITS: invalid speed units must be rejected before simulation"
                        .to_owned(),
                );
            }
        }
        "PV18_MISSING_PROVENANCE" => {
            let solver = string_at(inputs, "solver_id");
            let source_is_null = inputs
                .get("parameter_source_ref")
                .is_some_and(Value::is_null);
            if solver != Some("") || !source_is_null || disposition != "insufficient_evidence" {
                errors.push("PV18_MISSING_PROVENANCE: missing solver/source must remain insufficient evidence".to_owned());
            }
        }
        "PV19_SOLVER_IDENTITY_MUTATION" => {
            let manifest_solver = string_at(inputs, "manifest_solver_version");
            let result_solver = string_at(inputs, "result_solver_version");
            if manifest_solver == result_solver || disposition != "reject_as_tampered_evidence" {
                errors.push(
                    "PV19_SOLVER_IDENTITY_MUTATION: solver provenance mismatch must be rejected"
                        .to_owned(),
                );
            }
        }
        "PV20_NONCONVERGED_SOLVER" => {
            if string_at(inputs, "solver_status") != Some("non_converged")
                || disposition != "simulation_failure_not_promoted"
            {
                errors.push(
                    "PV20_NONCONVERGED_SOLVER: non-converged result must not be promoted"
                        .to_owned(),
                );
            }
        }
        "PV21_OUT_OF_DOMAIN" => {
            let speed = number_at(inputs, "speed_mps");
            let max_speed = number_at(inputs, "validated_speed_max_mps");
            let temperature = number_at(inputs, "ambient_temperature_c");
            let max_temperature = number_at(inputs, "validated_temperature_max_c");
            let out_of_domain = matches!((speed, max_speed), (Some(s), Some(m)) if s > m)
                || matches!((temperature, max_temperature), (Some(t), Some(m)) if t > m);
            if !out_of_domain || disposition != "insufficient_evidence" {
                errors.push(
                    "PV21_OUT_OF_DOMAIN: out-of-domain input must result in abstention".to_owned(),
                );
            }
        }
        _ => {}
    }
}

fn validate_corpus(document: &Value, raw_bytes: &[u8]) -> AuditResult<ScenarioSummary> {
    let mut errors = Vec::new();
    check_exact_keys(
        document,
        &[
            "schema_version",
            "corpus_id",
            "created_date",
            "status",
            "vehicle_profile",
            "status_semantics",
            "claim_ceiling",
            "cases",
        ],
        "corpus",
        &mut errors,
    );

    if string_at(document, "schema_version") != Some(CORPUS_SCHEMA) {
        errors.push(format!("corpus.schema_version: expected {CORPUS_SCHEMA}"));
    }
    if string_at(document, "status") != Some(CORPUS_STATUS) {
        errors.push(format!(
            "corpus.status: synthetic/non-validated status must be preserved"
        ));
    }
    if string_at(document, "corpus_id")
        != Some("symtropy-personal-family-road-vehicle-synthetic-v0.1")
    {
        errors.push("corpus.corpus_id: unexpected corpus identity".to_owned());
    }

    let Some(profile) = document.get("vehicle_profile") else {
        errors.push("corpus.vehicle_profile: missing".to_owned());
        return Err(errors);
    };
    if string_at(profile, "id") != Some("family-road-vehicle-synthetic-v0.1") {
        errors.push("vehicle_profile.id: unexpected profile identity".to_owned());
    }
    let assumptions = profile.get("assumptions").and_then(Value::as_array);
    if !assumptions.is_some_and(|values| values.len() >= 3 && values.iter().all(Value::is_string)) {
        errors.push(
            "vehicle_profile.assumptions: at least three explicit assumptions are required"
                .to_owned(),
        );
    }

    let semantics = document.get("status_semantics").and_then(Value::as_object);
    if !semantics.is_some_and(|values| !values.is_empty()) {
        errors.push("status_semantics: expected non-empty object".to_owned());
    }

    let claims = document.get("claim_ceiling").and_then(Value::as_array);
    if !claims.is_some_and(|values| {
        !values.is_empty()
            && values.iter().all(Value::is_string)
            && values
                .iter()
                .filter_map(Value::as_str)
                .any(|value| value.contains("No case establishes physical braking"))
    }) {
        errors.push("claim_ceiling: explicit physical-evidence limit is required".to_owned());
    }

    let Some(cases) = document.get("cases").and_then(Value::as_array) else {
        errors.push("cases: expected array".to_owned());
        return Err(errors);
    };
    if cases.len() != REQUIRED_IDS.len() {
        errors.push(format!(
            "cases: expected {} entries, found {}",
            REQUIRED_IDS.len(),
            cases.len()
        ));
    }

    let mut seen_ids = BTreeSet::new();
    let mut negative_count = 0usize;
    for (index, case) in cases.iter().enumerate() {
        let at = format!("cases[{index}]");
        check_exact_keys(
            case,
            &[
                "id",
                "kind",
                "description",
                "inputs",
                "required_invariants",
                "expected_disposition",
            ],
            &at,
            &mut errors,
        );

        let Some(id) = require_string(case, "id", &at, &mut errors) else {
            continue;
        };
        if !seen_ids.insert(id.clone()) {
            errors.push(format!("{at}: duplicate case id {id}"));
        }
        if !REQUIRED_IDS.contains(&id.as_str()) {
            errors.push(format!("{at}: unknown case id {id}"));
        }
        if require_string(case, "description", &at, &mut errors).is_none() {
            continue;
        }
        let Some(kind) = require_string(case, "kind", &at, &mut errors) else {
            continue;
        };
        if !["positive_control", "stress_case", "negative_control"].contains(&kind.as_str()) {
            errors.push(format!("{at}: unknown case kind {kind}"));
        }
        if kind == "negative_control" {
            negative_count += 1;
            if !REQUIRED_NEGATIVE_IDS.contains(&id.as_str()) {
                errors.push(format!("{at}: unregistered negative control {id}"));
            }
        } else if REQUIRED_NEGATIVE_IDS.contains(&id.as_str()) {
            errors.push(format!("{at}: required negative control {id} was weakened"));
        }

        let input_ok = case.get("inputs").is_some_and(Value::is_object);
        if !input_ok {
            errors.push(format!("{at}.inputs: expected object"));
        }
        match case.get("required_invariants").and_then(Value::as_array) {
            Some(invariants) if !invariants.is_empty() => {
                let mut invariant_set = BTreeSet::new();
                for invariant in invariants {
                    match invariant.as_str() {
                        Some(text) if !text.trim().is_empty() => {
                            if !invariant_set.insert(text) {
                                errors.push(format!("{at}: duplicate invariant {text}"));
                            }
                        }
                        _ => {
                            errors.push(format!("{at}: every invariant must be a non-empty string"))
                        }
                    }
                }
            }
            _ => errors.push(format!(
                "{at}.required_invariants: must be a non-empty array"
            )),
        }

        if let Some(disposition) = string_at(case, "expected_disposition") {
            if !semantics.is_some_and(|values| values.contains_key(disposition)) {
                errors.push(format!(
                    "{at}: disposition {disposition} has no declared semantics"
                ));
            }
        } else {
            errors.push(format!(
                "{at}.expected_disposition: missing or not a string"
            ));
        }
        validate_special_case(case, &mut errors);
    }

    for required_id in REQUIRED_IDS {
        if !seen_ids.contains(required_id) {
            errors.push(format!("cases: missing required case {required_id}"));
        }
    }
    if negative_count != REQUIRED_NEGATIVE_IDS.len() {
        errors.push(format!(
            "cases: expected {} negative controls, found {negative_count}",
            REQUIRED_NEGATIVE_IDS.len()
        ));
    }

    for (id, invariant) in [
        ("PV01_NOMINAL_DRY", "does_not_emit_physical_safety_claim"),
        ("PV06_OVERLOAD", "does_not_clamp_overload_silently"),
        ("PV17_INVALID_UNITS", "does_not_coerce_unknown_units"),
        ("PV20_NONCONVERGED_SOLVER", "retains_failure_receipt"),
        ("PV21_OUT_OF_DOMAIN", "does_not_extrapolate_silently"),
    ] {
        if let Some(case) = case_by_id(cases, id) {
            if !has_invariant(case, invariant) {
                errors.push(format!("{id}: required invariant {invariant} was removed"));
            }
        }
    }

    if errors.is_empty() {
        let sha256 = format!("{:x}", Sha256::digest(raw_bytes));
        Ok(ScenarioSummary {
            case_count: cases.len(),
            negative_control_count: negative_count,
            sha256,
        })
    } else {
        Err(errors)
    }
}

fn validate_source_registry(document: &Value, raw_bytes: &[u8]) -> AuditResult<(usize, String)> {
    let mut errors = Vec::new();
    check_exact_keys(
        document,
        &[
            "schema_version",
            "registry_id",
            "created_date",
            "status",
            "policy",
            "sources",
            "next_evidence_gates",
            "claim_ceiling",
        ],
        "source_registry",
        &mut errors,
    );
    if string_at(document, "schema_version") != Some("vehicle-benchmark-source-registry/0.1") {
        errors.push("source_registry.schema_version: unexpected version".to_owned());
    }
    if string_at(document, "registry_id")
        != Some("symtropy-personal-family-vehicle-source-registry-v0.1")
    {
        errors.push("source_registry.registry_id: unexpected identity".to_owned());
    }
    if string_at(document, "status")
        != Some("source inventory only; rights/reuse and applicability require review")
    {
        errors.push(
            "source_registry.status: rights/reuse review status must remain explicit".to_owned(),
        );
    }
    if !document.get("policy").is_some_and(|policy| {
        policy
            .get("no_vendoring_without_rights_review")
            .and_then(Value::as_bool)
            == Some(true)
            && policy
                .get("rules")
                .and_then(Value::as_array)
                .is_some_and(|rules| rules.len() >= 4)
    }) {
        errors.push(
            "source_registry.policy: rights gate and source-claim rules are required".to_owned(),
        );
    }

    let Some(sources) = document.get("sources").and_then(Value::as_array) else {
        return Err(vec!["source_registry.sources: expected array".to_owned()]);
    };
    if sources.len() != REQUIRED_SOURCE_IDS.len() {
        errors.push(format!(
            "source_registry.sources: expected {} entries, found {}",
            REQUIRED_SOURCE_IDS.len(),
            sources.len()
        ));
    }
    let mut seen = BTreeSet::new();
    for (index, source) in sources.iter().enumerate() {
        let at = format!("source_registry.sources[{index}]");
        let Some(id) = string_at(source, "id") else {
            errors.push(format!("{at}.id: missing or not a string"));
            continue;
        };
        if !seen.insert(id.to_owned()) {
            errors.push(format!("{at}: duplicate source id {id}"));
        }
        if !REQUIRED_SOURCE_IDS.contains(&id) {
            errors.push(format!("{at}: unregistered source id {id}"));
        }
        for key in [
            "title",
            "organization",
            "accessed_on",
            "role",
            "evidence_class",
            "status",
        ] {
            if string_at(source, key).is_none_or(|value| value.trim().is_empty()) {
                errors.push(format!("{at}.{key}: missing or blank"));
            }
        }
        if !string_at(source, "url").is_some_and(|url| url.starts_with("https://")) {
            errors.push(format!("{at}.url: source must use HTTPS"));
        }
        for key in ["usable_for", "not_usable_for", "rights_and_provenance"] {
            let values = source.get(key).and_then(Value::as_array);
            if !values.is_some_and(|values| {
                !values.is_empty()
                    && values
                        .iter()
                        .all(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
            }) {
                errors.push(format!(
                    "{at}.{key}: explicit non-empty string list required"
                ));
            }
        }
    }
    for id in REQUIRED_SOURCE_IDS {
        if !seen.contains(id) {
            errors.push(format!("source_registry: missing required source {id}"));
        }
    }
    if !document
        .get("next_evidence_gates")
        .and_then(Value::as_array)
        .is_some_and(|items| items.len() >= 5)
    {
        errors.push(
            "source_registry.next_evidence_gates: at least five gates are required".to_owned(),
        );
    }
    let claims = document.get("claim_ceiling").and_then(Value::as_array);
    if !claims.is_some_and(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .any(|text| text.contains("not a dataset license register"))
            && items
                .iter()
                .filter_map(Value::as_str)
                .any(|text| text.contains("contains no measured vehicle data"))
    }) {
        errors.push(
            "source_registry.claim_ceiling: source/measurement limits must be explicit".to_owned(),
        );
    }

    if errors.is_empty() {
        Ok((sources.len(), format!("{:x}", Sha256::digest(raw_bytes))))
    } else {
        Err(errors)
    }
}

fn validate_baseline_matrix(document: &Value, source_registry: &Value) -> AuditResult<usize> {
    let mut errors = Vec::new();
    check_exact_keys(
        document,
        &[
            "schema_version",
            "status",
            "created_date",
            "market",
            "currency",
            "scope",
            "price_policy",
            "candidates",
            "benchmark_selection_rules",
            "claim_ceiling",
        ],
        "baseline_matrix",
        &mut errors,
    );
    if string_at(document, "schema_version")
        != Some("south-africa-personal-family-vehicle-baseline-shortlist/0.1")
    {
        errors.push("baseline_matrix.schema_version: unexpected version".to_owned());
    }
    if string_at(document, "status")
        != Some("screening shortlist only; not an endorsed design or purchase recommendation")
    {
        errors.push(
            "baseline_matrix.status: shortlist must not imply recommendation or endorsement"
                .to_owned(),
        );
    }
    if string_at(document, "created_date") != Some("2026-10-17")
        || string_at(document, "market") != Some("South Africa")
        || string_at(document, "currency") != Some("ZAR")
    {
        errors.push("baseline_matrix: market/date/currency snapshot identity mismatch".to_owned());
    }

    let policy = document.get("price_policy");
    if !policy.is_some_and(|policy| {
        policy
            .get("must_reverify_before_use")
            .and_then(Value::as_bool)
            == Some(true)
            && string_at(policy, "meaning").is_some_and(|text| text.contains("not quote"))
            && policy
                .get("excludes")
                .and_then(Value::as_array)
                .is_some_and(|items| items.len() >= 5)
    }) {
        errors.push("baseline_matrix.price_policy: dated advertised-from prices must be reverified and exclusions retained".to_owned());
    }
    let rules = document
        .get("benchmark_selection_rules")
        .and_then(Value::as_array);
    if !rules.is_some_and(|items| {
        items.len() >= 5
            && items
                .iter()
                .all(|item| item.as_str().is_some_and(|text| !text.trim().is_empty()))
            && items
                .iter()
                .filter_map(Value::as_str)
                .any(|text| text.contains("not equivalent to lowest 10-year cost"))
    }) {
        errors.push("baseline_matrix.benchmark_selection_rules: comparability and low-price claim ceiling required".to_owned());
    }
    let claims = document.get("claim_ceiling").and_then(Value::as_array);
    if !claims.is_some_and(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .any(|text| text.contains("not verified dealer quotations"))
            && items
                .iter()
                .filter_map(Value::as_str)
                .any(|text| text.contains("No cost-of-ownership"))
    }) {
        errors.push(
            "baseline_matrix.claim_ceiling: quote/spec/TCO limitations must remain explicit"
                .to_owned(),
        );
    }

    let source_ids: BTreeSet<String> = source_registry
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|source| string_at(source, "id").map(str::to_owned))
        .collect();
    let source_urls: BTreeMap<String, String> = source_registry
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|source| {
            Some((
                string_at(source, "id")?.to_owned(),
                string_at(source, "url")?.to_owned(),
            ))
        })
        .collect();

    let Some(candidates) = document.get("candidates").and_then(Value::as_array) else {
        return Err(vec![
            "baseline_matrix.candidates: expected array".to_owned(),
        ]);
    };
    if candidates.len() != REQUIRED_BASELINE_IDS.len() {
        errors.push(format!(
            "baseline_matrix.candidates: expected {} entries, found {}",
            REQUIRED_BASELINE_IDS.len(),
            candidates.len()
        ));
    }

    let expected_price_sources = BTreeMap::from([
        (
            "BASELINE_TOYOTA_VITZ_ENTRY_HATCH",
            "SRC_TOYOTA_SA_MODEL_RANGE",
        ),
        (
            "BASELINE_SUZUKI_SWIFT_GL_MT",
            "SRC_SUZUKI_SWIFT_OFFICIAL_PRICES",
        ),
        (
            "BASELINE_TOYOTA_STARLET_ENTRY_HATCH",
            "SRC_TOYOTA_SA_MODEL_RANGE",
        ),
        (
            "BASELINE_TOYOTA_STARLET_CROSS_ENTRY",
            "SRC_TOYOTA_SA_MODEL_RANGE",
        ),
        (
            "BASELINE_TOYOTA_RUMION_ENTRY_MPV",
            "SRC_TOYOTA_SA_MODEL_RANGE",
        ),
    ]);
    let expected_prices_zar = BTreeMap::from([
        ("BASELINE_TOYOTA_VITZ_ENTRY_HATCH", 182400.0),
        ("BASELINE_SUZUKI_SWIFT_GL_MT", 228900.0),
        ("BASELINE_TOYOTA_STARLET_ENTRY_HATCH", 273700.0),
        ("BASELINE_TOYOTA_STARLET_CROSS_ENTRY", 311000.0),
        ("BASELINE_TOYOTA_RUMION_ENTRY_MPV", 314100.0),
    ]);
    let mut seen = BTreeSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let at = format!("baseline_matrix.candidates[{index}]");
        let Some(id) = require_string(candidate, "id", &at, &mut errors) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            errors.push(format!("{at}: duplicate candidate {id}"));
        }
        if !REQUIRED_BASELINE_IDS.contains(&id.as_str()) {
            errors.push(format!("{at}: unregistered baseline candidate {id}"));
        }
        for key in [
            "manufacturer",
            "model",
            "role",
            "price_source_id",
            "source_url",
        ] {
            if require_string(candidate, key, &at, &mut errors).is_none() {
                continue;
            }
        }
        let price_source_id = string_at(candidate, "price_source_id").unwrap_or("");
        if !source_ids.contains(price_source_id) {
            errors.push(format!(
                "{at}: price source {price_source_id} is not in source registry"
            ));
        }
        if let Some(expected_source) = expected_price_sources.get(id.as_str()) {
            if price_source_id != *expected_source {
                errors.push(format!("{at}: unexpected price source for {id}"));
            }
        }
        let source_url = string_at(candidate, "source_url").unwrap_or("");
        if !source_url.starts_with("https://") {
            errors.push(format!("{at}.source_url: must use HTTPS"));
        }
        if source_urls
            .get(price_source_id)
            .is_some_and(|registered| registered.as_str() != source_url)
        {
            errors.push(format!(
                "{at}: source_url does not match the registered source URL"
            ));
        }
        let price = candidate
            .get("advertised_from_price_zar")
            .and_then(Value::as_f64);
        if !matches!(price, Some(value) if value > 0.0) {
            errors.push(format!("{at}.advertised_from_price_zar: must be positive"));
        }
        if let Some(expected_price) = expected_prices_zar.get(id.as_str()) {
            if price != Some(*expected_price) {
                errors.push(format!("{at}: dated starting-price snapshot changed without an explicit baseline refresh"));
            }
        }
        if let Some(spec_source_id) = string_at(candidate, "specification_source_id") {
            if !source_ids.contains(spec_source_id) {
                errors.push(format!(
                    "{at}: specification source {spec_source_id} is not registered"
                ));
            }
            if let Some(spec_url) = string_at(candidate, "specification_source_url") {
                if source_urls
                    .get(spec_source_id)
                    .is_some_and(|registered| registered.as_str() != spec_url)
                {
                    errors.push(format!(
                        "{at}: specification URL does not match source registry"
                    ));
                }
            }
        } else if string_at(candidate, "specification_status").is_none() {
            errors.push(format!(
                "{at}: missing specification source or explicit pending status"
            ));
        }
        for key in ["compare_dimensions", "key_unknowns"] {
            let values = candidate.get(key).and_then(Value::as_array);
            if !values.is_some_and(|items| {
                !items.is_empty()
                    && items
                        .iter()
                        .all(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
            }) {
                errors.push(format!("{at}.{key}: explicit non-empty strings required"));
            }
        }
    }
    for id in REQUIRED_BASELINE_IDS {
        if !seen.contains(id) {
            errors.push(format!("baseline_matrix: missing baseline candidate {id}"));
        }
    }

    if errors.is_empty() {
        Ok(candidates.len())
    } else {
        Err(errors)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct EvidenceEnvelopeSummary {
    fixture_count: usize,
    schema_sha256: String,
    fixtures_sha256: String,
}

fn evidence_class_matches(artifact_kind: &str, evidence_class: &str) -> bool {
    match artifact_kind {
        "design_candidate" => evidence_class == "hypothesis",
        "simulation_run" => evidence_class == "simulation",
        "calibration_dataset" => [
            "bench_measurement",
            "instrumented_vehicle_test",
            "external_source",
        ]
        .contains(&evidence_class),
        "physical_measurement" => {
            ["bench_measurement", "instrumented_vehicle_test"].contains(&evidence_class)
        }
        "independent_review" => evidence_class == "independent_review",
        "regulatory_receipt" => evidence_class == "regulatory_receipt",
        "maintenance_event" => evidence_class == "maintenance_assertion",
        "source_record" => evidence_class == "external_source",
        _ => false,
    }
}

fn validate_evidence_envelope_fixtures(
    schema: &Value,
    fixtures_document: &Value,
    schema_raw: &[u8],
    fixtures_raw: &[u8],
) -> AuditResult<EvidenceEnvelopeSummary> {
    let mut errors = Vec::new();
    if string_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        errors.push("evidence_schema.$schema: expected Draft 2020-12".to_owned());
    }
    if string_at(schema, "$id")
        != Some("urn:luminous-dynamics:mobility:engineering-evidence-envelope:0.1")
    {
        errors.push("evidence_schema.$id: unexpected schema identity".to_owned());
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        errors.push("evidence_schema: failed Draft 2020-12 meta-schema validation".to_owned());
    }
    let validator = match jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => Some(validator),
        Err(error) => {
            errors.push(format!(
                "evidence_schema: could not build validator: {error}"
            ));
            None
        }
    };

    check_exact_keys(
        fixtures_document,
        &[
            "corpus_version",
            "status",
            "schema_ref",
            "fixtures",
            "claim_ceiling",
        ],
        "evidence_fixtures",
        &mut errors,
    );
    if string_at(fixtures_document, "corpus_version")
        != Some("mobility-engineering-evidence-envelope-fixtures/0.1")
    {
        errors.push("evidence_fixtures.corpus_version: unexpected version".to_owned());
    }
    if string_at(fixtures_document, "status")
        != Some("synthetic-contract-fixtures-not-qualification")
    {
        errors
            .push("evidence_fixtures.status: claim ceiling status must remain explicit".to_owned());
    }

    let Some(fixtures) = fixtures_document.get("fixtures").and_then(Value::as_array) else {
        return Err(vec![
            "evidence_fixtures.fixtures: expected array".to_owned(),
        ]);
    };
    if fixtures.len() != REQUIRED_EVIDENCE_FIXTURE_IDS.len() {
        errors.push(format!(
            "evidence_fixtures.fixtures: expected {} fixtures, found {}",
            REQUIRED_EVIDENCE_FIXTURE_IDS.len(),
            fixtures.len()
        ));
    }

    let mut seen = BTreeSet::new();
    for (index, fixture) in fixtures.iter().enumerate() {
        let at = format!("evidence_fixtures.fixtures[{index}]");
        check_exact_keys(
            fixture,
            &["id", "expectation", "input", "reason"],
            &at,
            &mut errors,
        );
        let Some(id) = require_string(fixture, "id", &at, &mut errors) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            errors.push(format!("{at}: duplicate fixture id {id}"));
        }
        if !REQUIRED_EVIDENCE_FIXTURE_IDS.contains(&id.as_str()) {
            errors.push(format!("{at}: unregistered fixture id {id}"));
        }
        let expectation = string_at(fixture, "expectation").unwrap_or("");
        if !["accept", "reject_schema", "reject_semantics"].contains(&expectation) {
            errors.push(format!("{at}: unknown expectation {expectation}"));
        }
        if require_string(fixture, "reason", &at, &mut errors).is_none() {
            continue;
        }
        let Some(envelope) = fixture.get("input") else {
            errors.push(format!("{at}.input: missing envelope"));
            continue;
        };
        let Some(validator) = validator.as_ref() else {
            continue;
        };
        let schema_errors: Vec<String> = validator
            .iter_errors(envelope)
            .map(|error| error.to_string())
            .collect();
        match expectation {
            "accept" => {
                if !schema_errors.is_empty() {
                    errors.push(format!(
                        "{at}: valid envelope rejected by schema: {}",
                        schema_errors.join("; ")
                    ));
                }
                let kind = string_at(envelope, "artifact_kind").unwrap_or("");
                let class = string_at(envelope, "evidence_class").unwrap_or("");
                if !evidence_class_matches(kind, class) {
                    errors.push(format!(
                        "{at}: artifact kind {kind} cannot assert evidence class {class}"
                    ));
                }
            }
            "reject_schema" => {
                if schema_errors.is_empty() {
                    errors.push(format!(
                        "{at}: malformed envelope unexpectedly passed the schema"
                    ));
                }
            }
            "reject_semantics" => {
                if !schema_errors.is_empty() {
                    errors.push(format!(
                        "{at}: semantic negative control is also schema-invalid: {}",
                        schema_errors.join("; ")
                    ));
                } else {
                    let kind = string_at(envelope, "artifact_kind").unwrap_or("");
                    let class = string_at(envelope, "evidence_class").unwrap_or("");
                    if evidence_class_matches(kind, class) {
                        errors.push(format!(
                            "{at}: incompatible artifact/evidence pair {kind}/{class} was accepted"
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    for id in REQUIRED_EVIDENCE_FIXTURE_IDS {
        if !seen.contains(id) {
            errors.push(format!("evidence_fixtures: missing required fixture {id}"));
        }
    }

    let claims = fixtures_document
        .get("claim_ceiling")
        .and_then(Value::as_array);
    if !claims.is_some_and(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .any(|text| text.contains("No fixture establishes truth, physical performance"))
            && items.iter().filter_map(Value::as_str).any(|text| {
                text.contains("Fixtures test schema shape and evidence-class compatibility only")
            })
    }) {
        errors.push(
            "evidence_fixtures.claim_ceiling: evidence limits must remain explicit".to_owned(),
        );
    }

    if errors.is_empty() {
        Ok(EvidenceEnvelopeSummary {
            fixture_count: fixtures.len(),
            schema_sha256: format!("{:x}", Sha256::digest(schema_raw)),
            fixtures_sha256: format!("{:x}", Sha256::digest(fixtures_raw)),
        })
    } else {
        Err(errors)
    }
}

fn default_corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/personal_vehicle/scenario_corpus_v0_1.json")
}

fn default_registry_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/personal_vehicle/benchmark_source_registry_v0_1.json")
}

fn default_baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/personal_vehicle/south_africa_baseline_shortlist_v0_1.json")
}

fn default_evidence_schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/transport/engineering_evidence_envelope_v0_1.schema.json")
}

fn default_evidence_fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/personal_vehicle/engineering_evidence_envelope_fixtures_v0_1.json")
}

fn read_cli() -> Result<(PathBuf, PathBuf, PathBuf, PathBuf, PathBuf), String> {
    let mut args = env::args_os().skip(1);
    let mut corpus_path = None;
    let mut registry_path = None;
    let mut baseline_path = None;
    let mut evidence_schema_path = None;
    let mut evidence_fixtures_path = None;
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!(
                "Usage: personal-vehicle-corpus-audit [--corpus PATH] [--registry PATH] [--baseline PATH] [--envelope-schema PATH] [--envelope-fixtures PATH]"
            );
            println!(
                "Checks synthetic scenario/source/baseline/evidence-envelope contracts only; it does not run vehicle physics."
            );
            std::process::exit(0);
        } else if arg == "--corpus" {
            let Some(value) = args.next() else {
                return Err("--corpus requires a path".to_owned());
            };
            corpus_path = Some(PathBuf::from(value));
        } else if arg == "--registry" {
            let Some(value) = args.next() else {
                return Err("--registry requires a path".to_owned());
            };
            registry_path = Some(PathBuf::from(value));
        } else if arg == "--baseline" {
            let Some(value) = args.next() else {
                return Err("--baseline requires a path".to_owned());
            };
            baseline_path = Some(PathBuf::from(value));
        } else if arg == "--envelope-schema" {
            let Some(value) = args.next() else {
                return Err("--envelope-schema requires a path".to_owned());
            };
            evidence_schema_path = Some(PathBuf::from(value));
        } else if arg == "--envelope-fixtures" {
            let Some(value) = args.next() else {
                return Err("--envelope-fixtures requires a path".to_owned());
            };
            evidence_fixtures_path = Some(PathBuf::from(value));
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()));
        }
    }
    Ok((
        corpus_path.unwrap_or_else(default_corpus_path),
        registry_path.unwrap_or_else(default_registry_path),
        baseline_path.unwrap_or_else(default_baseline_path),
        evidence_schema_path.unwrap_or_else(default_evidence_schema_path),
        evidence_fixtures_path.unwrap_or_else(default_evidence_fixtures_path),
    ))
}

fn main() -> ExitCode {
    let (corpus_path, registry_path, baseline_path, evidence_schema_path, evidence_fixtures_path) =
        match read_cli() {
            Ok(paths) => paths,
            Err(error) => {
                eprintln!("CORPUS_AUDIT_FAIL: {error}");
                return ExitCode::from(2);
            }
        };
    let load_json = |path: &PathBuf, label: &str| -> Result<(Vec<u8>, Value), String> {
        let raw =
            fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let value = serde_json::from_slice(&raw)
            .map_err(|error| format!("malformed {label} JSON: {error}"))?;
        Ok((raw, value))
    };

    let (corpus_raw, corpus) = match load_json(&corpus_path, "scenario corpus") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("CORPUS_AUDIT_FAIL: {error}");
            return ExitCode::from(2);
        }
    };
    let (registry_raw, registry) = match load_json(&registry_path, "source registry") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("CORPUS_AUDIT_FAIL: {error}");
            return ExitCode::from(2);
        }
    };
    let (baseline_raw, baseline) =
        match load_json(&baseline_path, "South African baseline shortlist") {
            Ok(value) => value,
            Err(error) => {
                eprintln!("CORPUS_AUDIT_FAIL: {error}");
                return ExitCode::from(2);
            }
        };
    let (evidence_schema_raw, evidence_schema) =
        match load_json(&evidence_schema_path, "engineering evidence schema") {
            Ok(value) => value,
            Err(error) => {
                eprintln!("CORPUS_AUDIT_FAIL: {error}");
                return ExitCode::from(2);
            }
        };
    let (evidence_fixtures_raw, evidence_fixtures) =
        match load_json(&evidence_fixtures_path, "engineering evidence fixtures") {
            Ok(value) => value,
            Err(error) => {
                eprintln!("CORPUS_AUDIT_FAIL: {error}");
                return ExitCode::from(2);
            }
        };

    let mut errors = Vec::new();
    let summary = match validate_corpus(&corpus, &corpus_raw) {
        Ok(summary) => Some(summary),
        Err(corpus_errors) => {
            errors.extend(corpus_errors);
            None
        }
    };
    let registry_result = match validate_source_registry(&registry, &registry_raw) {
        Ok(result) => Some(result),
        Err(registry_errors) => {
            errors.extend(registry_errors);
            None
        }
    };
    let baseline_result = match validate_baseline_matrix(&baseline, &registry) {
        Ok(result) => Some(result),
        Err(baseline_errors) => {
            errors.extend(baseline_errors);
            None
        }
    };
    let evidence_result = match validate_evidence_envelope_fixtures(
        &evidence_schema,
        &evidence_fixtures,
        &evidence_schema_raw,
        &evidence_fixtures_raw,
    ) {
        Ok(result) => Some(result),
        Err(evidence_errors) => {
            errors.extend(evidence_errors);
            None
        }
    };

    let repair_graph_count = match repair_graph_audit::validate_fixture_corpus() {
        Ok(count) => Some(count),
        Err(graph_errors) => {
            errors.extend(
                graph_errors
                    .into_iter()
                    .map(|error| format!("repair_task_graph: {error}")),
            );
            None
        }
    };

    if errors.is_empty() {
        let summary = summary.expect("scenario summary exists after successful validation");
        let (source_registry_count, source_registry_sha256) =
            registry_result.expect("source registry result exists after successful validation");
        let evidence =
            evidence_result.expect("evidence envelope result exists after successful validation");
        let baseline_count =
            baseline_result.expect("baseline shortlist result exists after successful validation");
        let repair_graph_fixture_count = repair_graph_count
            .expect("repair graph fixture count exists after successful validation");
        let report = AuditSummary {
            case_count: summary.case_count,
            negative_control_count: summary.negative_control_count,
            sha256: summary.sha256,
            source_registry_count,
            source_registry_sha256,
            evidence_fixture_count: evidence.fixture_count,
            evidence_schema_sha256: evidence.schema_sha256,
            evidence_fixtures_sha256: evidence.fixtures_sha256,
            baseline_count,
            baseline_sha256: format!("{:x}", Sha256::digest(&baseline_raw)),
        };
        println!(
            "CORPUS_AUDIT_PASS cases={} negative_controls={} corpus_sha256={} source_registry_entries={} source_registry_sha256={} baseline_candidates={} baseline_sha256={} evidence_fixtures={} envelope_schema_sha256={} envelope_fixtures_sha256={} repair_graph_fixtures={}",
            report.case_count,
            report.negative_control_count,
            report.sha256,
            report.source_registry_count,
            report.source_registry_sha256,
            report.baseline_count,
            report.baseline_sha256,
            report.evidence_fixture_count,
            report.evidence_schema_sha256,
            report.evidence_fixtures_sha256,
            repair_graph_fixture_count
        );
        println!(
            "CLAIM_CEILING synthetic scenario/evidence contracts and sourced price-screening snapshot only; no physics, licensing, TCO or safety result"
        );
        ExitCode::SUCCESS
    } else {
        eprintln!("CORPUS_AUDIT_FAIL errors={}", errors.len());
        for error in errors {
            eprintln!("- {error}");
        }
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Value {
        serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/scenario_corpus_v0_1.json"
        ))
        .expect("repository corpus should parse as JSON")
    }

    fn validate_fixture(document: &Value) -> Result<ScenarioSummary, Vec<String>> {
        let bytes = serde_json::to_vec(document).expect("fixture serialization should succeed");
        validate_corpus(document, &bytes)
    }

    fn baseline_matrix() -> Value {
        serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/south_africa_baseline_shortlist_v0_1.json"
        ))
        .expect("baseline shortlist should parse as JSON")
    }

    fn validate_baseline_fixture(document: &Value, registry: &Value) -> Result<usize, Vec<String>> {
        validate_baseline_matrix(document, registry)
    }

    #[test]
    fn south_african_baseline_shortlist_has_complete_source_bindings() {
        let matrix = baseline_matrix();
        let registry = source_registry();
        assert_eq!(validate_baseline_fixture(&matrix, &registry), Ok(5));
    }

    #[test]
    fn baseline_shortlist_rejects_unregistered_price_source() {
        let mut matrix = baseline_matrix();
        matrix["candidates"][0]["price_source_id"] = Value::String("SRC_UNREGISTERED".to_owned());
        assert!(validate_baseline_fixture(&matrix, &source_registry()).is_err());
    }

    #[test]
    fn baseline_prices_cannot_be_promoted_to_total_ownership_cost() {
        let mut matrix = baseline_matrix();
        matrix["price_policy"]["must_reverify_before_use"] = Value::Bool(false);
        assert!(validate_baseline_fixture(&matrix, &source_registry()).is_err());
    }

    #[test]
    fn baseline_candidate_census_cannot_drop_a_body_style() {
        let mut matrix = baseline_matrix();
        matrix["candidates"]
            .as_array_mut()
            .expect("candidate array")
            .pop();
        assert!(validate_baseline_fixture(&matrix, &source_registry()).is_err());
    }

    fn evidence_schema() -> Value {
        serde_json::from_str(include_str!(
            "../../../schemas/transport/engineering_evidence_envelope_v0_1.schema.json"
        ))
        .expect("evidence envelope schema should parse as JSON")
    }

    fn evidence_fixtures() -> Value {
        serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/engineering_evidence_envelope_fixtures_v0_1.json"
        ))
        .expect("evidence envelope fixtures should parse as JSON")
    }

    fn evidence_fixture_errors(schema: &Value, fixtures: &Value) -> Vec<String> {
        let schema_bytes = serde_json::to_vec(schema).expect("schema serialization");
        let fixtures_bytes = serde_json::to_vec(fixtures).expect("fixtures serialization");
        validate_evidence_envelope_fixtures(schema, fixtures, &schema_bytes, &fixtures_bytes)
            .err()
            .unwrap_or_default()
    }

    #[test]
    fn evidence_envelope_schema_and_frozen_fixtures_validate() {
        let schema = evidence_schema();
        let fixtures = evidence_fixtures();
        let schema_bytes = serde_json::to_vec(&schema).expect("schema bytes");
        let fixtures_bytes = serde_json::to_vec(&fixtures).expect("fixture bytes");
        let result =
            validate_evidence_envelope_fixtures(&schema, &fixtures, &schema_bytes, &fixtures_bytes)
                .expect("evidence contract should pass");
        assert_eq!(result.fixture_count, 15);
        assert_eq!(result.schema_sha256.len(), 64);
        assert_eq!(result.fixtures_sha256.len(), 64);
    }

    #[test]
    fn simulation_cannot_be_promoted_to_instrumented_vehicle_evidence() {
        let mut fixtures = evidence_fixtures();
        let items = fixtures["fixtures"].as_array_mut().expect("fixture array");
        let promoted = items
            .iter_mut()
            .find(|item| item["id"] == "EV06_SIMULATION_PROMOTED_TO_PHYSICAL")
            .expect("promotion fixture");
        promoted["expectation"] = Value::String("accept".to_owned());
        assert!(!evidence_fixture_errors(&evidence_schema(), &fixtures).is_empty());
    }

    #[test]
    fn independent_review_cannot_be_promoted_to_regulatory_receipt() {
        let mut fixtures = evidence_fixtures();
        let items = fixtures["fixtures"].as_array_mut().expect("fixture array");
        let promoted = items
            .iter_mut()
            .find(|item| item["id"] == "EV07_REVIEW_PROMOTED_TO_REGULATORY")
            .expect("promotion fixture");
        promoted["expectation"] = Value::String("accept".to_owned());
        assert!(!evidence_fixture_errors(&evidence_schema(), &fixtures).is_empty());
    }

    #[test]
    fn envelope_schema_rejects_extra_sensitive_top_level_fields() {
        let mut fixtures = evidence_fixtures();
        let items = fixtures["fixtures"].as_array_mut().expect("fixture array");
        let leak = items
            .iter_mut()
            .find(|item| item["id"] == "EV02_VALID_SIMULATION")
            .expect("simulation fixture");
        leak["input"]["raw_vin"] = Value::String("SYNTHETICVIN0000001".to_owned());
        assert!(!evidence_fixture_errors(&evidence_schema(), &fixtures).is_empty());
    }

    fn source_registry() -> Value {
        serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/benchmark_source_registry_v0_1.json"
        ))
        .expect("source registry should parse as JSON")
    }

    fn validate_source_registry_fixture(document: &Value) -> Result<(usize, String), Vec<String>> {
        let bytes = serde_json::to_vec(document).expect("registry serialization should succeed");
        validate_source_registry(document, &bytes)
    }

    #[test]
    fn source_registry_has_complete_reviewed_census() {
        let registry = source_registry();
        let (count, digest) =
            validate_source_registry_fixture(&registry).expect("source registry contract");
        assert_eq!(count, 15);
        assert_eq!(digest.len(), 64);
    }

    #[test]
    fn source_registry_cannot_hide_reuse_limits() {
        let mut registry = source_registry();
        registry["sources"][0]
            .as_object_mut()
            .expect("source object")
            .remove("not_usable_for");
        assert!(validate_source_registry_fixture(&registry).is_err());
    }

    #[test]
    fn duplicate_source_ids_are_rejected() {
        let mut registry = source_registry();
        registry["sources"][1]["id"] = registry["sources"][0]["id"].clone();
        assert!(validate_source_registry_fixture(&registry).is_err());
    }

    #[test]
    fn frozen_corpus_is_valid_and_has_expected_census() {
        let result =
            validate_fixture(&corpus()).expect("frozen corpus should satisfy its contract");
        assert_eq!(result.case_count, 21);
        assert_eq!(result.negative_control_count, 9);
        assert_eq!(result.sha256.len(), 64);
    }

    #[test]
    fn duplicate_case_identity_is_rejected() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        cases[1]["id"] = cases[0]["id"].clone();
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn overload_cannot_be_promoted_to_eligible() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV06_OVERLOAD").expect("overload case")["expected_disposition"] =
            Value::String("eligible_for_model_evaluation".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn out_of_domain_case_must_abstain() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV21_OUT_OF_DOMAIN").expect("out-of-domain case")["expected_disposition"] =
            Value::String("eligible_for_model_evaluation".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn solver_identity_mutation_must_be_rejected() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV19_SOLVER_IDENTITY_MUTATION").expect("solver mutation case")["expected_disposition"] =
            Value::String("eligible_for_model_evaluation".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn missing_safety_claim_boundary_is_rejected() {
        let mut document = corpus();
        document["claim_ceiling"] = serde_json::json!(["generic test data only"]);
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn weakened_negative_control_classification_is_rejected() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV17_INVALID_UNITS").expect("units case")["kind"] =
            Value::String("stress_case".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn invalid_speed_unit_is_rejected_if_expected_disposition_changes() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV17_INVALID_UNITS").expect("units case")["inputs"]["speed"]["unit"] =
            Value::String("m/s".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn missing_invariant_is_rejected() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV21_OUT_OF_DOMAIN").expect("out-of-domain case")["required_invariants"] =
            serde_json::json!(["detects_out_of_domain"]);
        assert!(validate_fixture(&document).is_err());
    }
}
