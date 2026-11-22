// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Fixed-target exact-binomial screening only. This is not a POD-curve,
// a90/95, ASTM E2862, MIL-HDBK-1823A, certification, or acceptance engine.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;

const CORPUS_SCHEMA: &str = "luminous.aero.nde-fixed-target-screen.v0.1";
const CLAIM_CEILING: &str = "fixed_target_binomial_screening_only";
const OUTPUT_SCHEMA: &str = "luminous.aero.nde-fixed-target-receipt.v0.1";
const BINOMIAL_INDEPENDENCE_NOTE: &str =
    "Bounds are conditional on independent Bernoulli trials within the declared fixed scope; independence and population representativeness are not authenticated.";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema: String,
    benchmark_id: String,
    evidence_class: String,
    claim_ceiling: String,
    scope: Scope,
    criteria: Criteria,
    base_study: Value,
    cases: Vec<CaseSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    scope_ref: String,
    method_id: String,
    procedure_revision: String,
    material_system: String,
    defect_family: String,
    defect_orientation: String,
    environment_id: String,
    target_flaw_size_mm: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Criteria {
    minimum_inspectors: usize,
    minimum_flaw_trials_per_inspector: usize,
    minimum_blank_trials_per_inspector: usize,
    target_detection_probability: f64,
    detection_one_sided_confidence: f64,
    maximum_false_call_probability: f64,
    false_call_one_sided_confidence: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseSpec {
    case_id: String,
    #[serde(default)]
    mutations: Vec<Mutation>,
    expected_disposition: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mutation {
    operation: String,
    #[serde(default)]
    inspector_id: Option<String>,
    #[serde(default)]
    trial_index: Option<usize>,
    #[serde(default)]
    value: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Study {
    study_id: String,
    evidence_class: String,
    scope_ref: String,
    inspectors: Vec<Inspector>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inspector {
    inspector_id: String,
    equipment_serial: String,
    calibration_id: String,
    calibration_valid: bool,
    flaw_trials: Vec<FlawTrial>,
    blank_trials: Vec<BlankTrial>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FlawTrial {
    specimen_id: String,
    scope_ref: String,
    flaw_size_mm: f64,
    detected: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlankTrial {
    specimen_id: String,
    scope_ref: String,
    false_call: bool,
}

#[derive(Debug, Serialize)]
struct InspectorResult {
    inspector_id: String,
    equipment_serial: String,
    calibration_id: String,
    flaw_trials: usize,
    detections: usize,
    detection_fraction: f64,
    detection_lower_bound: f64,
    detection_confidence: f64,
    blank_trials: usize,
    false_calls: usize,
    false_call_fraction: f64,
    false_call_upper_bound: f64,
    false_call_confidence: f64,
    detection_bound_pass: bool,
    false_call_bound_pass: bool,
    inspector_screen_pass: bool,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case_id: String,
    disposition: String,
    reasons: Vec<String>,
    inspector_count: usize,
    inspectors_passing: usize,
    worst_detection_lower_bound: Option<f64>,
    worst_false_call_upper_bound: Option<f64>,
    expected_disposition: String,
    expected_matched: bool,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema: &'static str,
    benchmark_id: String,
    corpus_sha256: String,
    source_commit: String,
    evidence_class: String,
    claim_ceiling: &'static str,
    authority_granted: bool,
    physical_pod: &'static str,
    a90_95: &'static str,
    astm_e2862_compliance: &'static str,
    mil_hdbk_1823a_compliance: &'static str,
    inspection_acceptance: &'static str,
    independence_note: &'static str,
    case_count: usize,
    expected_matches: usize,
    expected_mismatches: usize,
    results: Vec<CaseResult>,
    inspector_details: Vec<Vec<InspectorResult>>,
}

type ScreenResult<T> = std::result::Result<T, String>;

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut value = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut value, "{byte:02x}").expect("writing into String cannot fail");
    }
    value
}

fn validate_criteria(criteria: &Criteria) -> ScreenResult<()> {
    if criteria.minimum_inspectors == 0
        || criteria.minimum_flaw_trials_per_inspector == 0
        || criteria.minimum_blank_trials_per_inspector == 0
    {
        return Err("criteria sample counts must be positive".to_owned());
    }
    for (name, value) in [
        ("target_detection_probability", criteria.target_detection_probability),
        ("detection_one_sided_confidence", criteria.detection_one_sided_confidence),
        ("maximum_false_call_probability", criteria.maximum_false_call_probability),
        ("false_call_one_sided_confidence", criteria.false_call_one_sided_confidence),
    ] {
        if !value.is_finite() || !(0.0..1.0).contains(&value) {
            return Err(format!("{name} must be finite and strictly between zero and one"));
        }
    }
    Ok(())
}

fn validate_scope(scope: &Scope) -> ScreenResult<()> {
    for (name, value) in [
        ("scope_ref", &scope.scope_ref),
        ("method_id", &scope.method_id),
        ("procedure_revision", &scope.procedure_revision),
        ("material_system", &scope.material_system),
        ("defect_family", &scope.defect_family),
        ("defect_orientation", &scope.defect_orientation),
        ("environment_id", &scope.environment_id),
    ] {
        if value.trim().is_empty() {
            return Err(format!("scope field {name} is empty"));
        }
    }
    if !scope.target_flaw_size_mm.is_finite() || scope.target_flaw_size_mm <= 0.0 {
        return Err("target flaw size must be finite and positive".to_owned());
    }
    Ok(())
}

fn inspector_value_mut<'a>(study: &'a mut Value, inspector_id: &str) -> ScreenResult<&'a mut Value> {
    study
        .get_mut("inspectors")
        .and_then(Value::as_array_mut)
        .and_then(|items| {
            items.iter_mut().find(|item| {
                item.get("inspector_id").and_then(Value::as_str) == Some(inspector_id)
            })
        })
        .ok_or_else(|| format!("inspector not found: {inspector_id}"))
}

fn set_trial_field(
    study: &mut Value,
    inspector_id: &str,
    collection: &str,
    trial_index: usize,
    field: &str,
    value: Value,
) -> ScreenResult<()> {
    let inspector = inspector_value_mut(study, inspector_id)?;
    let trial = inspector
        .get_mut(collection)
        .and_then(Value::as_array_mut)
        .and_then(|items| items.get_mut(trial_index))
        .ok_or_else(|| format!("trial not found: {inspector_id}/{collection}/{trial_index}"))?;
    let object = trial
        .as_object_mut()
        .ok_or_else(|| "trial record is not an object".to_owned())?;
    if !object.contains_key(field) {
        return Err(format!("trial field not found: {field}"));
    }
    object.insert(field.to_owned(), value);
    Ok(())
}

fn apply_mutation(study: &mut Value, mutation: &Mutation) -> ScreenResult<()> {
    match mutation.operation.as_str() {
        "remove_inspector" => {
            let inspector_id = mutation
                .inspector_id
                .as_deref()
                .ok_or_else(|| "remove_inspector requires inspector_id".to_owned())?;
            let inspectors = study
                .get_mut("inspectors")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| "inspectors collection missing".to_owned())?;
            let before = inspectors.len();
            inspectors.retain(|item| {
                item.get("inspector_id").and_then(Value::as_str) != Some(inspector_id)
            });
            if before == inspectors.len() {
                return Err(format!("inspector not found: {inspector_id}"));
            }
            Ok(())
        }
        "set_calibration_valid" => {
            let inspector_id = mutation
                .inspector_id
                .as_deref()
                .ok_or_else(|| "set_calibration_valid requires inspector_id".to_owned())?;
            let value = mutation
                .value
                .as_ref()
                .and_then(Value::as_bool)
                .ok_or_else(|| "set_calibration_valid requires boolean value".to_owned())?;
            let inspector = inspector_value_mut(study, inspector_id)?;
            let object = inspector
                .as_object_mut()
                .ok_or_else(|| "inspector is not an object".to_owned())?;
            object.insert("calibration_valid".to_owned(), Value::Bool(value));
            Ok(())
        }
        "set_flaw_detected" | "set_blank_false_call" | "set_flaw_size_mm"
        | "set_flaw_specimen_id" => {
            let inspector_id = mutation
                .inspector_id
                .as_deref()
                .ok_or_else(|| "trial mutation requires inspector_id".to_owned())?;
            let trial_index = mutation
                .trial_index
                .ok_or_else(|| "trial mutation requires trial_index".to_owned())?;
            let value = mutation
                .value
                .clone()
                .ok_or_else(|| "trial mutation requires value".to_owned())?;
            let (collection, field) = match mutation.operation.as_str() {
                "set_flaw_detected" => ("flaw_trials", "detected"),
                "set_blank_false_call" => ("blank_trials", "false_call"),
                "set_flaw_size_mm" => ("flaw_trials", "flaw_size_mm"),
                "set_flaw_specimen_id" => ("flaw_trials", "specimen_id"),
                _ => unreachable!(),
            };
            set_trial_field(study, inspector_id, collection, trial_index, field, value)
        }
        "truncate_blank_trials" => {
            let inspector_id = mutation
                .inspector_id
                .as_deref()
                .ok_or_else(|| "truncate_blank_trials requires inspector_id".to_owned())?;
            let length = mutation
                .value
                .as_ref()
                .and_then(Value::as_u64)
                .ok_or_else(|| "truncate_blank_trials requires a non-negative integer value".to_owned())?
                as usize;
            let inspector = inspector_value_mut(study, inspector_id)?;
            let trials = inspector
                .get_mut("blank_trials")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| "blank_trials collection missing".to_owned())?;
            if length > trials.len() {
                return Err("cannot extend blank trials with truncate_blank_trials".to_owned());
            }
            trials.truncate(length);
            Ok(())
        }
        other => Err(format!("unsupported mutation operation: {other}")),
    }
}

fn log_choose(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    (1..=k)
        .map(|i| ((n + 1 - i) as f64 / i as f64).ln())
        .sum()
}

fn log_binomial_probability(n: usize, k: usize, p: f64) -> f64 {
    if p <= 0.0 {
        return if k == 0 { 0.0 } else { f64::NEG_INFINITY };
    }
    if p >= 1.0 {
        return if k == n { 0.0 } else { f64::NEG_INFINITY };
    }
    log_choose(n, k) + k as f64 * p.ln() + (n - k) as f64 * (-p).ln_1p()
}

fn log_sum_exp(values: &[f64]) -> f64 {
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max.is_finite() {
        return max;
    }
    max + values.iter().map(|value| (*value - max).exp()).sum::<f64>().ln()
}

fn binomial_upper_tail(n: usize, x: usize, p: f64) -> f64 {
    let terms = (x..=n)
        .map(|k| log_binomial_probability(n, k, p))
        .collect::<Vec<_>>();
    log_sum_exp(&terms).exp().clamp(0.0, 1.0)
}

fn binomial_lower_tail(n: usize, x: usize, p: f64) -> f64 {
    let terms = (0..=x)
        .map(|k| log_binomial_probability(n, k, p))
        .collect::<Vec<_>>();
    log_sum_exp(&terms).exp().clamp(0.0, 1.0)
}

fn detection_lower_bound(n: usize, hits: usize, confidence: f64) -> f64 {
    if hits == 0 {
        return 0.0;
    }
    let alpha = 1.0 - confidence;
    if hits == n {
        return alpha.powf(1.0 / n as f64);
    }
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..100 {
        let mid = (low + high) / 2.0;
        if binomial_upper_tail(n, hits, mid) < alpha {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) / 2.0
}

fn false_call_upper_bound(n: usize, false_calls: usize, confidence: f64) -> f64 {
    if false_calls == n {
        return 1.0;
    }
    let alpha = 1.0 - confidence;
    if false_calls == 0 {
        return 1.0 - alpha.powf(1.0 / n as f64);
    }
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..100 {
        let mid = (low + high) / 2.0;
        if binomial_lower_tail(n, false_calls, mid) > alpha {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) / 2.0
}

fn result_without_inspectors(
    case_id: &str,
    disposition: &str,
    reason: &str,
    expected: &str,
) -> (CaseResult, Vec<InspectorResult>) {
    (
        CaseResult {
            case_id: case_id.to_owned(),
            disposition: disposition.to_owned(),
            reasons: vec![reason.to_owned()],
            inspector_count: 0,
            inspectors_passing: 0,
            worst_detection_lower_bound: None,
            worst_false_call_upper_bound: None,
            expected_disposition: expected.to_owned(),
            expected_matched: disposition == expected,
        },
        Vec::new(),
    )
}

fn evaluate_study(
    case_id: &str,
    raw_study: Value,
    scope: &Scope,
    criteria: &Criteria,
    expected: &str,
) -> ScreenResult<(CaseResult, Vec<InspectorResult>)> {
    let study: Study = serde_json::from_value(raw_study)
        .map_err(|error| format!("{case_id}: invalid study: {error}"))?;

    if study.study_id.trim().is_empty() || study.evidence_class != "synthetic" {
        return Ok(result_without_inspectors(
            case_id,
            "NOT_EVALUABLE_EVIDENCE_CLASS",
            "study must have a non-empty ID and remain labelled synthetic in v0.1",
            expected,
        ));
    }
    if study.scope_ref != scope.scope_ref {
        return Ok(result_without_inspectors(
            case_id,
            "NOT_EVALUABLE_SCOPE_MISMATCH",
            "study scope reference does not match the frozen benchmark scope",
            expected,
        ));
    }
    if study.inspectors.len() < criteria.minimum_inspectors {
        return Ok(result_without_inspectors(
            case_id,
            "INSUFFICIENT_INSPECTOR_COUNT",
            "inspector count is below the predeclared minimum",
            expected,
        ));
    }

    let mut inspector_ids = BTreeSet::new();
    let mut specimen_ids = BTreeSet::new();
    for inspector in &study.inspectors {
        if inspector.inspector_id.trim().is_empty()
            || inspector.equipment_serial.trim().is_empty()
            || inspector.calibration_id.trim().is_empty()
            || !inspector_ids.insert(inspector.inspector_id.clone())
        {
            return Ok(result_without_inspectors(
                case_id,
                "NOT_EVALUABLE_DUPLICATE_OR_MISSING_IDENTITY",
                "inspector, equipment or calibration identity is missing or duplicated",
                expected,
            ));
        }
        if !inspector.calibration_valid {
            return Ok(result_without_inspectors(
                case_id,
                "NOT_EVALUABLE_INVALID_CALIBRATION",
                "at least one inspector equipment calibration is not current and valid",
                expected,
            ));
        }
        for trial in &inspector.flaw_trials {
            if trial.specimen_id.trim().is_empty() || !specimen_ids.insert(trial.specimen_id.clone()) {
                return Ok(result_without_inspectors(
                    case_id,
                    "NOT_EVALUABLE_DUPLICATE_SPECIMEN",
                    "specimen identity is missing or reused across observations",
                    expected,
                ));
            }
            if trial.scope_ref != scope.scope_ref
                || !trial.flaw_size_mm.is_finite()
                || (trial.flaw_size_mm - scope.target_flaw_size_mm).abs() > 1.0e-9
            {
                return Ok(result_without_inspectors(
                    case_id,
                    "NOT_EVALUABLE_SCOPE_MISMATCH",
                    "flaw trial does not match the exact fixed target size and scope",
                    expected,
                ));
            }
        }
        for trial in &inspector.blank_trials {
            if trial.specimen_id.trim().is_empty() || !specimen_ids.insert(trial.specimen_id.clone()) {
                return Ok(result_without_inspectors(
                    case_id,
                    "NOT_EVALUABLE_DUPLICATE_SPECIMEN",
                    "specimen identity is missing or reused across observations",
                    expected,
                ));
            }
            if trial.scope_ref != scope.scope_ref {
                return Ok(result_without_inspectors(
                    case_id,
                    "NOT_EVALUABLE_SCOPE_MISMATCH",
                    "blank trial does not match the declared inspection scope",
                    expected,
                ));
            }
        }
        if inspector.flaw_trials.len() < criteria.minimum_flaw_trials_per_inspector
            || inspector.blank_trials.len() < criteria.minimum_blank_trials_per_inspector
        {
            return Ok(result_without_inspectors(
                case_id,
                "SAMPLE_COUNT_INSUFFICIENT",
                "at least one inspector has fewer than the predeclared flaw or blank trials",
                expected,
            ));
        }
    }

    let mut details = Vec::with_capacity(study.inspectors.len());
    for inspector in study.inspectors {
        let flaw_trials = inspector.flaw_trials.len();
        let detections = inspector.flaw_trials.iter().filter(|trial| trial.detected).count();
        let detection_fraction = detections as f64 / flaw_trials as f64;
        let detection_lower = detection_lower_bound(
            flaw_trials,
            detections,
            criteria.detection_one_sided_confidence,
        );

        let blank_trials = inspector.blank_trials.len();
        let false_calls = inspector.blank_trials.iter().filter(|trial| trial.false_call).count();
        let false_call_fraction = false_calls as f64 / blank_trials as f64;
        let false_call_upper = false_call_upper_bound(
            blank_trials,
            false_calls,
            criteria.false_call_one_sided_confidence,
        );
        let detection_pass = detection_lower + 1.0e-12 >= criteria.target_detection_probability;
        let false_call_pass = false_call_upper <= criteria.maximum_false_call_probability + 1.0e-12;
        details.push(InspectorResult {
            inspector_id: inspector.inspector_id,
            equipment_serial: inspector.equipment_serial,
            calibration_id: inspector.calibration_id,
            flaw_trials,
            detections,
            detection_fraction,
            detection_lower_bound: detection_lower,
            detection_confidence: criteria.detection_one_sided_confidence,
            blank_trials,
            false_calls,
            false_call_fraction,
            false_call_upper_bound: false_call_upper,
            false_call_confidence: criteria.false_call_one_sided_confidence,
            detection_bound_pass: detection_pass,
            false_call_bound_pass: false_call_pass,
            inspector_screen_pass: detection_pass && false_call_pass,
        });
    }

    let inspectors_passing = details.iter().filter(|item| item.inspector_screen_pass).count();
    let worst_detection = details
        .iter()
        .map(|item| item.detection_lower_bound)
        .reduce(f64::min);
    let worst_false_call = details
        .iter()
        .map(|item| item.false_call_upper_bound)
        .reduce(f64::max);
    let all_pass = inspectors_passing == details.len();
    let disposition = if all_pass {
        "PASS_FIXED_TARGET_SCREEN"
    } else {
        "SCREENING_BOUNDS_NOT_MET"
    };
    let result = CaseResult {
        case_id: case_id.to_owned(),
        disposition: disposition.to_owned(),
        reasons: if all_pass {
            vec!["every inspector meets both predeclared fixed-target one-sided binomial bounds".to_owned()]
        } else {
            vec!["one or more inspectors do not meet the detection lower-bound and/or false-call upper-bound criterion".to_owned()]
        },
        inspector_count: details.len(),
        inspectors_passing,
        worst_detection_lower_bound: worst_detection,
        worst_false_call_upper_bound: worst_false_call,
        expected_disposition: expected.to_owned(),
        expected_matched: disposition == expected,
    };
    Ok((result, details))
}

fn run_corpus(bytes: &[u8], source_commit: &str) -> ScreenResult<Receipt> {
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be exactly 40 lowercase hexadecimal characters".to_owned());
    }
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid corpus JSON: {error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.benchmark_id.trim().is_empty()
        || corpus.cases.is_empty()
    {
        return Err("corpus schema, evidence class, claim ceiling, ID or case list is invalid".to_owned());
    }
    validate_scope(&corpus.scope)?;
    validate_criteria(&corpus.criteria)?;

    let mut results = Vec::with_capacity(corpus.cases.len());
    let mut details = Vec::with_capacity(corpus.cases.len());
    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if case.case_id.trim().is_empty() || !case_ids.insert(case.case_id.clone()) {
            return Err("case IDs must be non-empty and unique".to_owned());
        }
        let mut raw_study = corpus.base_study.clone();
        for mutation in &case.mutations {
            apply_mutation(&mut raw_study, mutation)
                .map_err(|error| format!("{}: {error}", case.case_id))?;
        }
        let (result, inspector_results) = evaluate_study(
            &case.case_id,
            raw_study,
            &corpus.scope,
            &corpus.criteria,
            &case.expected_disposition,
        )?;
        results.push(result);
        details.push(inspector_results);
    }

    let expected_matches = results.iter().filter(|item| item.expected_matched).count();
    let expected_mismatches = results.len() - expected_matches;
    Ok(Receipt {
        schema: OUTPUT_SCHEMA,
        benchmark_id: corpus.benchmark_id,
        corpus_sha256: sha256_hex(bytes),
        source_commit: source_commit.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING,
        authority_granted: false,
        physical_pod: "NOT_ASSESSED",
        a90_95: "NOT_ESTIMATED",
        astm_e2862_compliance: "NOT_ASSESSED",
        mil_hdbk_1823a_compliance: "NOT_ASSESSED",
        inspection_acceptance: "NOT_AUTHORIZED",
        independence_note: BINOMIAL_INDEPENDENCE_NOTE,
        case_count: results.len(),
        expected_matches,
        expected_mismatches,
        results,
        inspector_details: details,
    })
}

fn main() -> ScreenResult<()> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("usage: aero-nde-binomial-screen <corpus.json> <source-commit-sha>".to_owned());
    }
    let bytes = fs::read(&args[1])
        .map_err(|error| format!("unable to read {}: {error}", args[1]))?;
    let receipt = run_corpus(&bytes, &args[2])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("unable to serialize receipt: {error}"))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_target_bounds_allow_one_miss_and_one_false_call() {
        let detection = detection_lower_bound(60, 59, 0.95);
        let false_call = false_call_upper_bound(200, 1, 0.50);
        assert!((detection - 0.9233600050654955).abs() < 1.0e-12);
        assert!((false_call - 0.00837751942939624).abs() < 1.0e-12);
        assert!(detection >= 0.90);
        assert!(false_call <= 0.01);
    }

    #[test]
    fn a_second_miss_or_false_call_breaks_the_declared_bounds() {
        assert!(detection_lower_bound(60, 58, 0.95) < 0.90);
        assert!(false_call_upper_bound(200, 2, 0.50) > 0.01);
    }

    #[test]
    fn binomial_bounds_are_monotone_in_hits_and_false_calls() {
        assert!(detection_lower_bound(60, 60, 0.95) > detection_lower_bound(60, 59, 0.95));
        assert!(false_call_upper_bound(200, 2, 0.50) > false_call_upper_bound(200, 1, 0.50));
    }
}
