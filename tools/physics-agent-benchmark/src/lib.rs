// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub technical_status: String,
    pub scientific_status: String,
    pub evidence_integrity: String,
    pub qualification_decision: String,
}

impl Classification {
    pub fn to_value(&self) -> Value {
        json!({
            "technical_status": self.technical_status,
            "scientific_status": self.scientific_status,
            "evidence_integrity": self.evidence_integrity,
            "qualification_decision": self.qualification_decision
        })
    }
}

pub fn classify_case(case: &Value) -> Result<Classification, String> {
    let execution = case
        .get("execution")
        .ok_or_else(|| "case has no execution object".to_owned())?;

    let exit_code = execution
        .get("process_exit_code")
        .and_then(Value::as_i64)
        .ok_or_else(|| "execution.process_exit_code must be an integer".to_owned())?;
    let artifacts_present = execution
        .get("artifacts_present")
        .and_then(Value::as_bool)
        .ok_or_else(|| "execution.artifacts_present must be a boolean".to_owned())?;
    let references_available = execution
        .get("reference_metrics_available")
        .and_then(Value::as_bool)
        .ok_or_else(|| "execution.reference_metrics_available must be a boolean".to_owned())?;
    let source_identity = execution
        .get("source_identity")
        .and_then(Value::as_str)
        .ok_or_else(|| "execution.source_identity must be a string".to_owned())?;
    let replay = execution
        .get("replay")
        .and_then(Value::as_str)
        .ok_or_else(|| "execution.replay must be a string".to_owned())?;
    let domain = execution
        .get("domain")
        .and_then(Value::as_str)
        .ok_or_else(|| "execution.domain must be a string".to_owned())?;

    let technical_status = if exit_code != 0 {
        if case.get("injected_fault").is_some() {
            "blocked"
        } else {
            "failed"
        }
    } else if !artifacts_present {
        "failed"
    } else {
        "success"
    };

    let evidence_integrity = if source_identity != "match" || replay == "mismatch" {
        "invalid"
    } else if technical_status != "success"
        || !references_available
        || domain != "inside_validity_domain"
        || replay != "match"
    {
        "incomplete"
    } else {
        "valid"
    };

    let scientific_status = if technical_status != "success"
        || !references_available
        || domain != "inside_validity_domain"
        || source_identity != "match"
    {
        "not_evaluated"
    } else {
        compare_metrics(case)?
    };

    let qualification_decision = if evidence_integrity == "invalid" {
        "reject"
    } else if domain != "inside_validity_domain" || !references_available {
        "abstain"
    } else if technical_status == "success"
        && scientific_status == "valid"
        && evidence_integrity == "valid"
    {
        "accept"
    } else {
        "reject"
    };

    Ok(Classification {
        technical_status: technical_status.to_owned(),
        scientific_status: scientific_status.to_owned(),
        evidence_integrity: evidence_integrity.to_owned(),
        qualification_decision: qualification_decision.to_owned(),
    })
}

fn compare_metrics(case: &Value) -> Result<&'static str, String> {
    let metrics = case
        .get("metrics")
        .and_then(Value::as_array)
        .ok_or_else(|| "case.metrics must be an array".to_owned())?;

    if metrics.is_empty() {
        return Ok("not_evaluated");
    }

    for (index, metric) in metrics.iter().enumerate() {
        let name = metric
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("metrics[{index}].name must be non-empty"))?;
        let unit = metric
            .get("unit")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("metrics[{index}] ({name}) has no unit"))?;
        let reference = metric
            .get("reference")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("metrics[{index}] ({name}) reference must be finite"))?;
        let observed = metric
            .get("observed")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("metrics[{index}] ({name}) observed must be finite"))?;
        let tolerance = metric
            .get("absolute_tolerance")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| {
                format!("metrics[{index}] ({name}) tolerance must be finite and non-negative")
            })?;

        let residual = (observed - reference).abs();
        if residual > tolerance {
            return Ok("invalid");
        }

        let _ = unit;
    }

    Ok("valid")
}

pub fn evaluate_registry(registry: &Value) -> Result<Value, String> {
    if registry.get("benchmark_id").and_then(Value::as_str) == Some("muon-collider-physics-agent") {
        // continue validation below
    } else {
        return Err("unexpected benchmark_id".to_owned());
    }
    if registry.get("version").and_then(Value::as_str) != Some("0.1.0") {
        return Err("unsupported benchmark registry version".to_owned());
    }
    if registry.get("evidence_class").and_then(Value::as_str)
        != Some("synthetic_workflow_control_cases")
    {
        return Err(
            "registry must be explicitly classified as synthetic workflow controls".to_owned(),
        );
    }

    let cases = registry
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry.cases must be an array".to_owned())?;

    if cases.len() != 6 {
        return Err(format!(
            "version 0.1.0 requires exactly 6 case families, found {}",
            cases.len()
        ));
    }

    let mut seen = std::collections::HashSet::new();
    let mut rows = Vec::with_capacity(cases.len());
    let mut mismatches = 0_u64;

    for case in cases {
        if case.get("synthetic_only").and_then(Value::as_bool) != Some(true) {
            return Err("every version 0.1.0 case must declare synthetic_only=true".to_owned());
        }

        let case_id = case
            .get("case_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "each case requires a non-empty case_id".to_owned())?;

        if !seen.insert(case_id.to_owned()) {
            return Err(format!("duplicate case_id {case_id:?}"));
        }

        let computed = classify_case(case)?;
        let expected = case
            .get("expected")
            .ok_or_else(|| format!("case {case_id} has no expected classification"))?;
        let computed_value = computed.to_value();

        let mut matches = true;
        for field in [
            "technical_status",
            "scientific_status",
            "evidence_integrity",
            "qualification_decision",
        ] {
            let expected_value = expected
                .get(field)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("case {case_id} expected.{field} must be a string"))?;
            let actual_value = computed_value
                .get(field)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("internal classification missing {field}"))?;

            if expected_value != actual_value {
                matches = false;
            }
        }

        if !matches {
            mismatches += 1;
        }

        rows.push(json!({
            "case_id": case_id,
            "case_family": case.get("case_family").and_then(Value::as_str).unwrap_or("unknown"),
            "computed": computed_value,
            "expected": expected,
            "matches": matches
        }));
    }

    Ok(json!({
        "benchmark_id": "muon-collider-physics-agent",
        "benchmark_version": "0.1.0",
        "evidence_class": "synthetic_workflow_control_cases",
        "status": if mismatches == 0 { "passed" } else { "failed" },
        "case_count": rows.len(),
        "matched_case_count": rows.len() as u64 - mismatches,
        "mismatch_count": mismatches,
        "cases": rows,
        "claim_ceiling": "Synthetic benchmark oracle consistency only; no agent or accelerator solver performance is established."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::Path;

    fn registry() -> Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("docs/research/muon-collider/physics-agent-benchmark-cases.v0.1.json");
        serde_json::from_slice(&fs::read(path).expect("registry must be readable"))
            .expect("registry must be valid JSON")
    }

    #[test]
    fn all_six_synthetic_case_oracles_match() {
        let report = evaluate_registry(&registry()).expect("registry should evaluate");
        assert_eq!(report["status"], "passed");
        assert_eq!(report["case_count"], 6);
        assert_eq!(report["matched_case_count"], 6);
        assert_eq!(report["mismatch_count"], 0);
    }

    #[test]
    fn a_green_process_with_bad_physics_is_rejected() {
        let mut case = registry()["cases"][2].clone();
        case["execution"]["process_exit_code"] = json!(0);
        case["execution"]["artifacts_present"] = json!(true);
        let result = classify_case(&case).expect("case should classify");
        assert_eq!(result.technical_status, "success");
        assert_eq!(result.scientific_status, "invalid");
        assert_eq!(result.qualification_decision, "reject");
    }

    #[test]
    fn infrastructure_failure_is_not_scientific_falsification() {
        let case = registry()["cases"][1].clone();
        let result = classify_case(&case).expect("case should classify");
        assert_eq!(result.technical_status, "blocked");
        assert_eq!(result.scientific_status, "not_evaluated");
        assert_eq!(result.qualification_decision, "reject");
    }

    #[test]
    fn plausible_metrics_do_not_override_stale_inputs() {
        let case = registry()["cases"][3].clone();
        let result = classify_case(&case).expect("case should classify");
        assert_eq!(result.scientific_status, "not_evaluated");
        assert_eq!(result.evidence_integrity, "invalid");
        assert_eq!(result.qualification_decision, "reject");
    }

    #[test]
    fn unsupported_domain_causes_abstention() {
        let case = registry()["cases"][4].clone();
        let result = classify_case(&case).expect("case should classify");
        assert_eq!(result.qualification_decision, "abstain");
    }

    #[test]
    fn invalid_provenance_is_rejected_even_when_domain_is_unsupported() {
        let mut case = registry()["cases"][4].clone();
        case["execution"]["source_identity"] = json!("mismatch");
        let result = classify_case(&case).expect("case should classify");
        assert_eq!(result.evidence_integrity, "invalid");
        assert_eq!(result.qualification_decision, "reject");
    }

    #[test]
    fn replay_divergence_rejects_an_otherwise_correct_metric() {
        let case = registry()["cases"][5].clone();
        let result = classify_case(&case).expect("case should classify");
        assert_eq!(result.scientific_status, "valid");
        assert_eq!(result.evidence_integrity, "invalid");
        assert_eq!(result.qualification_decision, "reject");
    }

    #[test]
    fn missing_or_negative_metric_tolerance_is_rejected() {
        let mut case = registry()["cases"][0].clone();
        case["metrics"][0]["absolute_tolerance"] = json!(-0.1);
        assert!(classify_case(&case).is_err());
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let mut value = registry();
        value["cases"][1]["case_id"] = value["cases"][0]["case_id"].clone();
        assert!(evaluate_registry(&value).is_err());
    }
}
