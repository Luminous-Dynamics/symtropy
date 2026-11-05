// SPDX-License-Identifier: AGPL-3.0-or-later
//! Claim-relative admissibility evaluation. This is not a safety controller,
//! an authentication service, a certification tool, or an operation permit.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";
const PACKET_SCHEMA: &str = include_str!(
    "../../../../research/construction-evidence-claim-admissibility-v0-2/schema/claim-admissibility-packet-v0.2.schema.json"
);
const PROFILE_SCHEMA: &str = include_str!(
    "../../../../research/construction-evidence-claim-admissibility-v0-2/schema/claim-requirements-v0.2.schema.json"
);
const RESULT_SCHEMA: &str = include_str!(
    "../../../../research/construction-evidence-claim-admissibility-v0-2/schema/claim-admissibility-result-v0.2.schema.json"
);
const PROFILES: &str = include_str!(
    "../../../../research/construction-evidence-claim-admissibility-v0-2/profiles/claim-requirements-v0.2.json"
);
const PACKET_SHA256: &str = "4704f6532cacc31dc7218a6fa00daeb83d4dd3019e0cf6057615eb00c426e994";
const PROFILE_SCHEMA_SHA256: &str =
    "7a86df940a993071712858a919c3eb6dcf2ced1a10d815cdae7a9b40406816aa";
const RESULT_SCHEMA_SHA256: &str =
    "5909bdc6bc8a2d46bf88b2a2f143b63e3e0a7b6ba320bfd97a8bb85e8103aec2";
const PROFILES_SHA256: &str = "d9cb12698346a2296cde39de2d6f0555ffa371fbfb34540f8f1e44eb87cc7024";
const RESULT_VERSION: &str = "construction-robot-claim-admissibility-result/0.2.1";
pub const VALIDATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct Findings {
    hard: bool,
    unknown: bool,
    reasons: Vec<String>,
}

impl Findings {
    fn hard(&mut self, code: &str) {
        self.hard = true;
        self.reasons.push(code.to_owned());
    }

    fn unknown(&mut self, code: &str) {
        self.unknown = true;
        self.reasons.push(code.to_owned());
    }

    fn decision(&self, scope: &str) -> &'static str {
        if scope == "not_evaluated" {
            "not_evaluated"
        } else if self.hard {
            "inadmissible_for_named_claim"
        } else if self.unknown {
            "indeterminate"
        } else {
            "admissible_for_named_claim"
        }
    }

    fn finish(&mut self) {
        self.reasons.sort();
        self.reasons.dedup();
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse(label: &str, raw: &str) -> Result<Value, String> {
    serde_json::from_str(raw).map_err(|e| format!("{label} is not valid JSON: {e}"))
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |v, key| v.get(*key))
}

fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    at(value, path).and_then(Value::as_str)
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn has_string(value: &Value, key: &str, expected: &str) -> bool {
    strings(value, key).iter().any(|s| s == expected)
}

fn check_local_refs(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if matches!(key.as_str(), "$ref" | "$dynamicRef" | "$recursiveRef") {
                    let target = child
                        .as_str()
                        .ok_or_else(|| format!("non-string schema ref at {path}/{key}"))?;
                    if !target.starts_with('#') {
                        return Err(format!(
                            "remote schema reference is disabled at {path}/{key}: {target}"
                        ));
                    }
                }
                check_local_refs(child, &format!("{path}/{key}"))?;
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                check_local_refs(child, &format!("{path}/{i}"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn schema_document(label: &str, raw: &str) -> Result<Value, String> {
    let schema = parse(label, raw)?;
    if str_at(&schema, &["$schema"]) != Some(DRAFT) {
        return Err(format!("{label} must declare Draft 2020-12"));
    }
    check_local_refs(&schema, "$")?;
    jsonschema::draft202012::meta::validate(&schema)
        .map_err(|e| format!("{label} failed Draft 2020-12 meta-validation: {e}"))?;
    Ok(schema)
}

fn validate_instance(label: &str, schema: &Value, instance: &Value) -> Result<(), String> {
    let validator = jsonschema::draft202012::new(schema)
        .map_err(|e| format!("cannot compile {label} schema: {e}"))?;
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{e} at {}", e.instance_path()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("{label} schema violations:\n{}", errors.join("\n")))
    }
}

fn unique_ids<'a>(items: impl Iterator<Item = Option<&'a str>>, label: &str) -> Result<(), String> {
    let mut seen = HashSet::new();
    for item in items {
        let id = item.ok_or_else(|| format!("{label} id is not a string"))?;
        if !seen.insert(id.to_owned()) {
            return Err(format!("duplicate {label} id: {id}"));
        }
    }
    Ok(())
}

fn axis(req: &Value, unit: &Value, key: &str, path: &[&str], code: &str, out: &mut Findings) {
    let Some(actual) = str_at(unit, path) else {
        out.unknown(code);
        return;
    };
    if has_string(req, key, actual) {
        return;
    }
    if (key == "required_independence_status"
        && matches!(
            actual,
            "partial" | "known_common_mode" | "same_evidence_path"
        ))
        || (key == "required_metrology_status" && actual == "not_applicable")
        || (key == "required_digest_status" && actual == "absent")
    {
        out.hard(code);
    } else {
        out.unknown(code);
    }
}

fn check_unit(req: &Value, unit: &Value, out: &mut Findings) {
    let origin = str_at(unit, &["origin"]).unwrap_or("");
    if !has_string(req, "acceptable_origins", origin) {
        out.hard("evidence_origin_not_acceptable_for_claim");
    }

    axis(
        req,
        unit,
        "required_independence_status",
        &["independence", "status"],
        "reference_independence_not_established",
        out,
    );
    if str_at(unit, &["independence", "status"]) == Some("independent_under_profile") {
        if str_at(unit, &["independence", "assessment_ref"]).is_none_or(str::is_empty) {
            out.unknown("independence_assessment_reference_missing");
        }
        if at(unit, &["independence", "unresolved_dependency_classes"])
            .and_then(Value::as_array)
            .is_some_and(|v| !v.is_empty())
        {
            out.hard("independence_status_conflicts_with_unresolved_dependencies");
        }
    }

    axis(
        req,
        unit,
        "required_metrology_status",
        &["metrology", "status"],
        "metrology_not_qualified_for_claim",
        out,
    );
    axis(
        req,
        unit,
        "required_digest_status",
        &["provenance", "digest_status"],
        "source_bytes_not_verified",
        out,
    );
    axis(
        req,
        unit,
        "required_authentication_status",
        &["provenance", "authentication_status"],
        "issuer_authentication_not_qualified",
        out,
    );
    axis(
        req,
        unit,
        "required_custody_status",
        &["custody", "status"],
        "evidence_custody_not_audited",
        out,
    );
    axis(
        req,
        unit,
        "required_odd_coverage_status",
        &["odd_coverage", "status"],
        "odd_coverage_not_measured_by_stratum",
        out,
    );

    let metrology = str_at(unit, &["metrology", "status"]).unwrap_or("");
    if matches!(
        metrology,
        "calibration_verified" | "uncertainty_reviewed" | "independently_audited"
    ) && strings(
        at(unit, &["metrology"]).unwrap_or(&Value::Null),
        "calibration_record_refs",
    )
    .is_empty()
    {
        out.unknown("metrology_calibration_record_reference_missing");
    }
    if matches!(metrology, "uncertainty_reviewed" | "independently_audited")
        && str_at(unit, &["metrology", "uncertainty_budget_ref"]).is_none_or(str::is_empty)
    {
        out.unknown("metrology_uncertainty_budget_reference_missing");
    }
    if str_at(unit, &["provenance", "authentication_status"]) == Some("issuer_trust_verified")
        && str_at(unit, &["provenance", "issuer_ref"]).is_none_or(str::is_empty)
    {
        out.unknown("trusted_issuer_reference_missing");
    }
    if matches!(
        str_at(unit, &["custody", "status"]),
        Some("custody_declared" | "custody_audited")
    ) && str_at(unit, &["custody", "custody_record_ref"]).is_none_or(str::is_empty)
    {
        out.unknown("custody_record_reference_missing");
    }

    let odd = at(unit, &["odd_coverage"]).unwrap_or(&Value::Null);
    let observed = strings(odd, "observed_strata");
    let unmeasured = strings(odd, "unmeasured_strata");
    for required in strings(req, "required_odd_strata") {
        if observed.contains(&required) {
            continue;
        }
        if unmeasured.contains(&required) {
            out.hard("required_odd_stratum_is_unmeasured");
        } else {
            out.unknown("required_odd_stratum_has_no_measurement_record");
        }
    }

    let replication = at(unit, &["replication"]).unwrap_or(&Value::Null);
    if !has_string(
        req,
        "minimum_replication_status",
        str_at(replication, &["status"]).unwrap_or(""),
    ) {
        out.hard("replication_scope_not_met");
    }
    for required in strings(req, "required_replication_dimensions") {
        if !strings(replication, "independent_dimensions").contains(&required) {
            out.hard("required_replication_dimension_not_established");
        }
    }
    for (profile_key, evidence_key, reason) in [
        (
            "minimum_independent_encounters",
            "independent_encounters",
            "minimum_independent_encounter_count_not_met",
        ),
        (
            "minimum_independent_sites",
            "distinct_sites",
            "minimum_independent_site_count_not_met",
        ),
        (
            "minimum_independent_configurations",
            "distinct_configurations",
            "minimum_independent_configuration_count_not_met",
        ),
    ] {
        if let Some(minimum) = req.get(profile_key).and_then(Value::as_u64) {
            let actual = replication
                .get(evidence_key)
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if actual < minimum {
                out.hard(reason);
            }
        }
    }
}

fn check_context(req: &Value, claim: &Value, out: &mut Findings) {
    if let Some(required) = req.get("required_functional_unit").and_then(Value::as_str) {
        match str_at(claim, &["context", "functional_unit"]) {
            Some(actual) if actual == required => {}
            Some(_) => out.hard("functional_unit_does_not_match_profile"),
            None => out.unknown("required_functional_unit_not_declared"),
        }
    }

    let baseline = strings(req, "required_baseline_match");
    if !baseline.is_empty() {
        match str_at(claim, &["context", "baseline_match_status"]) {
            Some("mismatched") => out.hard("baseline_comparison_explicitly_mismatched"),
            Some("unresolved") => out.unknown("baseline_comparison_unresolved"),
            Some("matched") => {
                if str_at(claim, &["context", "baseline_match_assessment_ref"])
                    .is_none_or(str::is_empty)
                {
                    out.unknown("baseline_match_assessment_reference_missing");
                }
                for dimension in baseline {
                    if !has_string(
                        at(claim, &["context"]).unwrap_or(&Value::Null),
                        "baseline_match",
                        &dimension,
                    ) {
                        out.hard("required_baseline_match_dimension_not_matched");
                    }
                }
            }
            _ => out.unknown("baseline_comparison_not_established"),
        }
    }

    if let Some(required) = req.get("sample_size_policy_ref").and_then(Value::as_str) {
        match str_at(claim, &["context", "sample_size_policy_ref"]) {
            Some(actual) if actual == required => {}
            Some(_) => out.hard("sample_size_policy_reference_does_not_match_profile"),
            None => out.unknown("sample_size_policy_reference_missing"),
        }
    }

    if let Some(required) = req.get("required_holdout_split").and_then(Value::as_str) {
        match str_at(claim, &["context", "holdout_split"]) {
            Some(actual) if actual == required => {}
            Some(_) => out.hard("holdout_split_does_not_match_profile"),
            None => out.unknown("required_holdout_split_not_declared"),
        }
    }
}

fn check_adjudication(req: &Value, assessment: &Value, claim_id: &str, out: &mut Findings) {
    let Some(required) = req.get("required_adjudication").and_then(Value::as_str) else {
        return;
    };
    let records = assessment
        .get("claim_adjudications")
        .and_then(Value::as_array);
    let record = records.and_then(|items| {
        items
            .iter()
            .find(|item| str_at(item, &["claim_id"]) == Some(claim_id))
    });
    match record {
        None => out.unknown("required_claim_adjudication_not_established"),
        Some(item) if str_at(item, &["status"]) != Some(required) => {
            out.hard("claim_adjudication_level_below_profile_requirement")
        }
        Some(item)
            if str_at(item, &["adjudicator_reference"]).is_none_or(str::is_empty)
                || str_at(item, &["adjudication_record_ref"]).is_none_or(str::is_empty) =>
        {
            out.unknown("claim_adjudication_record_reference_missing")
        }
        Some(_) => {}
    }
}

fn evaluate_claim(
    claim: &Value,
    packet: &Value,
    profiles: &Value,
    profiles_digest: &str,
) -> Result<Value, String> {
    let claim_id = str_at(claim, &["claim_id"]).ok_or("claim_id missing")?;
    let kind = str_at(claim, &["claim_kind"]).ok_or("claim_kind missing")?;
    let profile_id =
        str_at(claim, &["requirements_profile_id"]).ok_or("requirements_profile_id missing")?;
    if str_at(claim, &["requirements_profile_sha256"]) != Some(profiles_digest) {
        return Err(format!(
            "claim {claim_id} requirements profile digest mismatch"
        ));
    }
    let profile = profiles
        .get("profiles")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| str_at(item, &["profile_id"]) == Some(profile_id))
        })
        .ok_or_else(|| format!("claim {claim_id} references unknown profile {profile_id}"))?;
    if str_at(profile, &["claim_kind"]) != Some(kind) {
        return Err(format!(
            "claim {claim_id} kind does not match profile {profile_id}"
        ));
    }
    let req = profile
        .get("requirements")
        .ok_or("profile requirements missing")?;
    let scope = str_at(claim, &["evaluation_scope"]).unwrap_or("evaluate");
    let evidence_ids = strings(claim, "supporting_evidence_unit_ids");
    let evidence = packet
        .get("evidence_units")
        .and_then(Value::as_array)
        .ok_or("evidence_units missing")?;
    let mut findings = Findings::default();

    if scope == "not_evaluated" {
        findings.reasons.push("evaluation_not_requested".to_owned());
    } else {
        for evidence_id in &evidence_ids {
            let unit = evidence
                .iter()
                .find(|item| str_at(item, &["evidence_unit_id"]) == Some(evidence_id))
                .ok_or_else(|| {
                    format!("claim {claim_id} references missing evidence unit {evidence_id}")
                })?;
            check_unit(req, unit, &mut findings);
        }
        check_context(req, claim, &mut findings);
        check_adjudication(
            req,
            packet.get("assessment").unwrap_or(&Value::Null),
            claim_id,
            &mut findings,
        );
    }

    findings.finish();
    Ok(json!({
        "claim_id": claim_id,
        "claim_kind": kind,
        "profile_id": profile_id,
        "evaluation_scope": scope,
        "decision": findings.decision(scope),
        "reason_codes": findings.reasons,
        "supporting_evidence_unit_ids": evidence_ids,
        "limitations": claim.get("limitations").cloned().unwrap_or_else(|| json!([]))
    }))
}

/// Validate the input and compute claim-by-claim results using embedded,
/// source-pinned rules. Runtime schema/profile replacement is not supported.
pub fn evaluate_packet(packet_raw: &str) -> Result<Value, String> {
    for (label, actual, expected) in [
        (
            "packet schema",
            sha256(PACKET_SCHEMA.as_bytes()),
            PACKET_SHA256,
        ),
        (
            "profile schema",
            sha256(PROFILE_SCHEMA.as_bytes()),
            PROFILE_SCHEMA_SHA256,
        ),
        (
            "result schema",
            sha256(RESULT_SCHEMA.as_bytes()),
            RESULT_SCHEMA_SHA256,
        ),
        (
            "requirements profiles",
            sha256(PROFILES.as_bytes()),
            PROFILES_SHA256,
        ),
    ] {
        if actual != expected {
            return Err(format!("embedded {label} digest mismatch"));
        }
    }

    let packet_schema = schema_document("packet schema", PACKET_SCHEMA)?;
    let profile_schema = schema_document("profile schema", PROFILE_SCHEMA)?;
    let result_schema = schema_document("result schema", RESULT_SCHEMA)?;
    let packet = parse("claim packet", packet_raw)?;
    let profiles = parse("requirements profiles", PROFILES)?;
    validate_instance("claim packet", &packet_schema, &packet)?;
    validate_instance("requirements profiles", &profile_schema, &profiles)?;

    let units = packet
        .get("evidence_units")
        .and_then(Value::as_array)
        .ok_or("evidence_units missing")?;
    unique_ids(
        units.iter().map(|u| str_at(u, &["evidence_unit_id"])),
        "evidence unit",
    )?;
    let profile_items = profiles
        .get("profiles")
        .and_then(Value::as_array)
        .ok_or("profiles missing")?;
    unique_ids(
        profile_items.iter().map(|p| str_at(p, &["profile_id"])),
        "profile",
    )?;
    let claims = packet
        .get("claims")
        .and_then(Value::as_array)
        .ok_or("claims missing")?;
    unique_ids(claims.iter().map(|c| str_at(c, &["claim_id"])), "claim")?;

    let claim_ids: HashSet<String> = claims
        .iter()
        .filter_map(|c| str_at(c, &["claim_id"]).map(str::to_owned))
        .collect();
    let adjudications = packet
        .get("assessment")
        .and_then(|a| a.get("claim_adjudications"))
        .and_then(Value::as_array)
        .ok_or("claim adjudications missing")?;
    unique_ids(
        adjudications.iter().map(|a| str_at(a, &["claim_id"])),
        "claim adjudication",
    )?;
    for record in adjudications {
        let id = str_at(record, &["claim_id"]).ok_or("adjudication claim_id missing")?;
        if !claim_ids.contains(id) {
            return Err(format!("adjudication references unknown claim {id}"));
        }
    }

    for unit in units {
        let refs = unit
            .get("source_refs")
            .and_then(Value::as_array)
            .ok_or("source_refs missing")?;
        unique_ids(
            refs.iter().map(|r| str_at(r, &["source_id"])),
            "source reference",
        )?;
        let observed = strings(
            at(unit, &["odd_coverage"]).unwrap_or(&Value::Null),
            "observed_strata",
        );
        let unmeasured = strings(
            at(unit, &["odd_coverage"]).unwrap_or(&Value::Null),
            "unmeasured_strata",
        );
        if observed.iter().any(|s| unmeasured.contains(s)) {
            return Err(format!(
                "evidence unit {} marks an ODD stratum observed and unmeasured",
                str_at(unit, &["evidence_unit_id"]).unwrap_or("unknown")
            ));
        }
    }

    let profile_digest = sha256(PROFILES.as_bytes());
    let claim_results = claims
        .iter()
        .map(|c| evaluate_claim(c, &packet, &profiles, &profile_digest))
        .collect::<Result<Vec<_>, _>>()?;
    let result = json!({
        "schema_version": RESULT_VERSION,
        "packet_id": str_at(&packet, &["packet_id"]).ok_or("packet_id missing")?,
        "packet_sha256": sha256(packet_raw.as_bytes()),
        "profile_document_sha256": profile_digest,
        "packet_schema_sha256": sha256(PACKET_SCHEMA.as_bytes()),
        "profile_schema_sha256": sha256(PROFILE_SCHEMA.as_bytes()),
        "result_schema_sha256": sha256(RESULT_SCHEMA.as_bytes()),
        "validator": {
            "name": "symtropy-construction-evidence-validator",
            "version": VALIDATOR_VERSION,
            "decision_semantics": "claim-specific; no aggregate score"
        },
        "claim_results": claim_results
    });
    validate_instance("evaluation result", &result_schema, &result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{PROFILES, evaluate_packet};
    use serde_json::{Value, json};

    const SYNTHETIC: &str = include_str!(
        "../../../../research/construction-evidence-claim-admissibility-v0-2/fixtures/valid/synthetic-contract-only.json"
    );
    const BENCH: &str = include_str!(
        "../../../../research/construction-evidence-claim-admissibility-v0-2/fixtures/valid/bench-stopping-distance-claim.json"
    );
    const FIELD: &str = include_str!(
        "../../../../research/construction-evidence-claim-admissibility-v0-2/fixtures/invalid/field-observation-without-reference-ground-truth.json"
    );
    const PROMOTION: &str = include_str!(
        "../../../../research/construction-evidence-claim-admissibility-v0-2/fixtures/invalid/independent-review-does-not-make-synthetic-physical.json"
    );

    fn decision(result: &Value) -> &str {
        result["claim_results"][0]["decision"]
            .as_str()
            .expect("decision")
    }

    #[test]
    fn synthetic_fixture_is_admissible_only_for_synthetic_contract() {
        let result = evaluate_packet(SYNTHETIC).expect("synthetic packet");
        assert_eq!(decision(&result), "admissible_for_named_claim");
    }

    #[test]
    fn bench_fixture_is_scoped_to_the_bench_claim() {
        let result = evaluate_packet(BENCH).expect("bench packet");
        assert_eq!(decision(&result), "admissible_for_named_claim");
    }

    #[test]
    fn field_detection_without_independent_reference_is_blocked() {
        let result = evaluate_packet(FIELD).expect("well-formed adversarial packet");
        assert_eq!(decision(&result), "inadmissible_for_named_claim");
    }

    #[test]
    fn independent_review_does_not_promote_synthetic_origin() {
        let result = evaluate_packet(PROMOTION).expect("well-formed adversarial packet");
        assert_eq!(decision(&result), "inadmissible_for_named_claim");
    }

    #[test]
    fn packet_author_cannot_supply_a_decision() {
        let mut packet: Value = serde_json::from_str(SYNTHETIC).expect("packet JSON");
        packet["claims"][0]["decision"] = json!("admissible_for_named_claim");
        assert!(
            evaluate_packet(&serde_json::to_string(&packet).expect("serialized packet")).is_err()
        );
    }

    #[test]
    fn profile_digest_cannot_be_changed_by_packet_author() {
        let mut packet: Value = serde_json::from_str(SYNTHETIC).expect("packet JSON");
        packet["claims"][0]["requirements_profile_sha256"] = json!("0".repeat(64));
        assert!(
            evaluate_packet(&serde_json::to_string(&packet).expect("serialized packet")).is_err()
        );
    }

    #[test]
    fn not_evaluated_is_not_misreported_as_admissible() {
        let mut packet: Value = serde_json::from_str(SYNTHETIC).expect("packet JSON");
        packet["claims"][0]["evaluation_scope"] = json!("not_evaluated");
        let result = evaluate_packet(&serde_json::to_string(&packet).expect("serialized packet"))
            .expect("evaluation");
        assert_eq!(decision(&result), "not_evaluated");
    }

    #[test]
    fn sample_counts_are_not_added_across_evidence_units() {
        let mut packet: Value = serde_json::from_str(BENCH).expect("packet JSON");
        let mut second = packet["evidence_units"][0].clone();
        second["evidence_unit_id"] = json!("bench-stop-unit-002");
        second["source_refs"][0]["source_id"] = json!("bench-stop-unit-002-source");
        second["replication"]["independent_encounters"] = json!(15);
        packet["evidence_units"][0]["replication"]["independent_encounters"] = json!(15);
        packet["evidence_units"]
            .as_array_mut()
            .expect("units")
            .push(second);
        packet["claims"][0]["supporting_evidence_unit_ids"] =
            json!(["bench-stop-unit-001", "bench-stop-unit-002"]);
        let result = evaluate_packet(&serde_json::to_string(&packet).expect("serialized packet"))
            .expect("evaluation");
        assert_eq!(decision(&result), "inadmissible_for_named_claim");
    }

    #[test]
    fn result_has_no_aggregate_pass_or_evidence_score() {
        let result = evaluate_packet(SYNTHETIC).expect("synthetic packet");
        assert!(result.get("overall_decision").is_none());
        assert!(result.get("confidence_score").is_none());
        assert_eq!(
            result["profile_document_sha256"],
            super::sha256(PROFILES.as_bytes())
        );
    }
}
