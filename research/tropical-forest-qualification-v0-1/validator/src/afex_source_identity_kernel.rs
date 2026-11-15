// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Synthetic AFEX source/site/plot/sample-frame identity gate.
//! This validates join dispositions; it does not map or join real AFEX rows.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("String write cannot fail");
    }
    output
}

fn equal_identity(left: &Value, right: &Value, key: &str) -> Result<bool, String> {
    Ok(string_at(left, key)? == string_at(right, key)?)
}

fn validate_depth_compatibility(left: &Value, right: &Value) -> Result<bool, String> {
    match (left.get("depth_cm"), right.get("depth_cm")) {
        (None, None) | (Some(Value::Null), Some(Value::Null)) => Ok(true),
        (Some(Value::Null), None) | (None, Some(Value::Null)) => Ok(true),
        (Some(a), Some(b)) => Ok(a == b),
        (None, Some(_)) | (Some(_), None) => Ok(false),
    }
}

fn evaluate_case(case: &Value) -> Result<(String, String), String> {
    let requested_scope = string_at(case, "requested_join_scope")?;
    if requested_scope != "plot_context" && requested_scope != "observation" {
        return Err("requested_join_scope must be plot_context or observation".to_string());
    }
    let left = case.get("left").ok_or_else(|| "left identity required".to_string())?;
    let right = case.get("right").ok_or_else(|| "right identity required".to_string())?;

    if !equal_identity(left, right, "experiment_id")? {
        return Ok(("SITE_OR_SUPPORT_CONFLICT".to_string(), "experiment IDs differ".to_string()));
    }

    let left_site = optional_string(left, "site_id");
    let right_site = optional_string(right, "site_id");
    match (&left_site, &right_site) {
        (Some(a), Some(b)) if a != b => {
            return Ok(("SITE_OR_SUPPORT_CONFLICT".to_string(), "explicit site IDs differ".to_string()));
        }
        (None, None) => {
            let left_description = optional_string(left, "site_description").unwrap_or_default();
            let right_description = optional_string(right, "site_description").unwrap_or_default();
            if left_description != right_description {
                return Ok(("UNRESOLVED_IDENTITY".to_string(), "site descriptions differ without an authoritative site ID".to_string()));
            }
            return Ok(("UNRESOLVED_IDENTITY".to_string(), "site identity is absent".to_string()));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Ok(("UNRESOLVED_IDENTITY".to_string(), "one side lacks authoritative site ID".to_string()));
        }
        _ => {}
    }

    for key in ["block_id", "treatment_id"] {
        if !equal_identity(left, right, key)? {
            return Ok(("SITE_OR_SUPPORT_CONFLICT".to_string(), format!("{key} differs")));
        }
    }

    let left_plot = string_at(left, "plot_id")?;
    let right_plot = string_at(right, "plot_id")?;
    if left_plot != right_plot {
        let alias = case.get("alias_evidence");
        if let Some(alias) = alias {
            let valid_alias =
                string_at(alias, "left_plot_id")? == left_plot
                && string_at(alias, "right_plot_id")? == right_plot
                && string_at(alias, "site_id")? == left_site.as_ref().ok_or_else(|| "left site missing".to_string())?.to_string()
                && string_at(alias, "block_id")? == string_at(left, "block_id")?
                && string_at(alias, "treatment_id")? == string_at(left, "treatment_id")?
                && string_at(alias, "scope")? == "plot"
                && !string_at(alias, "evidence_id")?.trim().is_empty()
                && string_at(alias, "source_reference")?.starts_with("synthetic://");
            if valid_alias && requested_scope == "plot_context" {
                return Ok(("DOCUMENTED_ALIAS_MATCH".to_string(), "bounded explicit alias evidence accepted".to_string()));
            }
            return Ok(("SITE_OR_SUPPORT_CONFLICT".to_string(), "plot IDs differ and alias evidence is absent, invalid or out of scope".to_string()));
        }
        return Ok(("SITE_OR_SUPPORT_CONFLICT".to_string(), "plot IDs differ and no documented alias is supplied".to_string()));
    }

    if requested_scope == "plot_context" {
        return Ok(("EXACT_SOURCE_KEY_MATCH".to_string(), "same experiment/site/block/plot/treatment; source-native frames remain distinct".to_string()));
    }

    for key in ["sample_frame_type", "sample_frame_id", "support_level", "time_support"] {
        if !equal_identity(left, right, key)? {
            return Ok(("SITE_OR_SUPPORT_CONFLICT".to_string(), format!("{key} differs")));
        }
    }
    if !validate_depth_compatibility(left, right)? {
        return Ok(("SITE_OR_SUPPORT_CONFLICT".to_string(), "depth support differs".to_string()));
    }

    Ok(("EXACT_SOURCE_KEY_MATCH".to_string(), "observation identity and declared support match".to_string()))
}

pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    if string_at(manifest, "schema_version")? != "0.1.0"
        || string_at(manifest, "benchmark_id")? != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest, "corpus_id")? != "AFEX-SOURCE-IDENTITY-001"
    {
        return Err("AFEX source-identity corpus identity mismatch".to_string());
    }
    let ceiling = string_at(manifest, "claim_ceiling")?;
    if !ceiling.contains("No real AFEX plot/site crosswalk has been ingested")
        || !ceiling.contains("80 km versus 100 km source descriptions remain unresolved")
    {
        return Err("AFEX source-identity claim ceiling is incomplete".to_string());
    }
    let experiment = manifest.get("experiment_contract")
        .ok_or_else(|| "experiment_contract required".to_string())?;
    if string_at(experiment, "experiment_id")? != "AFEX"
        || experiment.get("core_plot_count").and_then(Value::as_u64) != Some(32)
        || experiment.get("block_count").and_then(Value::as_u64) != Some(4)
        || experiment.get("treatments_per_block").and_then(Value::as_u64) != Some(8)
        || string_at(experiment, "assignment_unit")? != "plot"
    {
        return Err("AFEX core experimental identity contract invalid".to_string());
    }
    let cases = manifest.get("cases").and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_string())?;
    if cases.len() != 7 {
        return Err("expected seven synthetic identity cases".to_string());
    }
    let mut ids = BTreeSet::new();
    let mut results = Vec::new();
    for case in cases {
        let case_id = string_at(case, "case_id")?;
        if !ids.insert(case_id.clone()) {
            return Err(format!("duplicate source identity case {case_id}"));
        }
        let (disposition, reason) = evaluate_case(case)?;
        let expected = string_at(case, "expected_disposition")?;
        if disposition != expected {
            return Err(format!("case {case_id} disposition {disposition} does not match expected {expected}"));
        }
        results.push(json!({
            "case_id":case_id,
            "requested_join_scope":string_at(case,"requested_join_scope")?,
            "disposition":disposition,
            "reason":reason,
            "source_rows_joined":false,
            "synthetic_policy_case":true
        }));
    }
    for required in [
        "EXACT_PLOT_CONTEXT_MATCH",
        "UNRESOLVED_80KM_100KM_SITE_DESCRIPTIONS",
        "SAME_PLOT_STRING_DIFFERENT_SITE",
        "SAME_NESTED_ID_DIFFERENT_FRAME",
        "DOCUMENTED_PLOT_ALIAS",
        "TREATMENT_LABEL_MATCH_BLOCK_CONFLICT",
        "OBSERVATION_INTERVAL_CONFLICT",
    ] {
        if !ids.contains(required) {
            return Err(format!("required identity case {required} missing"));
        }
    }
    Ok(json!({
        "schema_version":"0.1.0",
        "benchmark_id":"LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id":"AFEX-SOURCE-IDENTITY-001",
        "result":"PASS_SYNTHETIC_SOURCE_IDENTITY_POLICY",
        "source_manifest_sha256":sha256_hex(raw),
        "case_count":results.len(),
        "exact_source_key_match_count":results.iter().filter(|x|x["disposition"]=="EXACT_SOURCE_KEY_MATCH").count(),
        "documented_alias_match_count":results.iter().filter(|x|x["disposition"]=="DOCUMENTED_ALIAS_MATCH").count(),
        "unresolved_identity_count":results.iter().filter(|x|x["disposition"]=="UNRESOLVED_IDENTITY").count(),
        "site_or_support_conflict_count":results.iter().filter(|x|x["disposition"]=="SITE_OR_SUPPORT_CONFLICT").count(),
        "real_site_discrepancy_adjudicated":false,
        "real_plot_aliases_admitted":false,
        "raw_source_rows_joined":false,
        "case_results":results,
        "claim_ceiling":ceiling
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE:&str=include_str!("../../cases/afex-source-identity-synthetic-v0.1.json");

    #[test]
    fn source_identity_policy_reproduces_exact_unresolved_conflict_and_alias_cases() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture parses");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        assert_eq!(receipt["result"],"PASS_SYNTHETIC_SOURCE_IDENTITY_POLICY");
        assert_eq!(receipt["case_count"],7);
        assert_eq!(receipt["exact_source_key_match_count"],1);
        assert_eq!(receipt["documented_alias_match_count"],1);
        assert_eq!(receipt["unresolved_identity_count"],1);
        assert_eq!(receipt["site_or_support_conflict_count"],4);
        assert_eq!(receipt["real_site_discrepancy_adjudicated"],false);
    }

    #[test]
    fn site_description_cannot_be_promoted_without_site_id() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["cases"][1]["expected_disposition"]=json!("EXACT_SOURCE_KEY_MATCH");
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn plot_alias_without_evidence_is_rejected() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["cases"][4]["alias_evidence"]["evidence_id"]=json!("");
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn nested_id_collision_cannot_be_observation_joined() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["cases"][3]["expected_disposition"]=json!("EXACT_SOURCE_KEY_MATCH");
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }
}
