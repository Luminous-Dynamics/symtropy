// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic evaluator for the public synthetic RCQ v0.1 development corpus.
//!
//! This is not a production qualification engine. Its explicit rules are
//! fixture semantics only; owner-controlled interfaces from #10543, #4918,
//! #1834, #3522 and #3531 are not implemented or inferred here.

use ring::digest::{SHA256, digest};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const EVALUATOR_ID: &str = "robot-config-change-reference-evaluator/0.1";
pub const RESULT_SCHEMA_VERSION: &str = "robot-config-change-development-result-v0.1";
pub const FIXTURE_WITNESS_STATUS: &str = "admitted_synthetic_development_witness";
pub const PACKAGE_DIR: &str = "research/robot-config-change-qualification-v0-1";
pub const RCQ_DISPOSITIONS: &[&str] = &[
    "PreservedByExactIdentity",
    "PreservedByQualifiedEquivalence",
    "PreservedByExactRecomputation",
    "RequiresApplicabilityReview",
    "RequiresBenchReverification",
    "RequiresPhysicalReverification",
    "RequiresIndependentAdjudication",
    "NotPreserved",
    "UnknownImpact",
    "Unsupported",
];

#[derive(Clone, Copy, Debug)]
struct Rule {
    disposition: &'static str,
    reason_code: &'static str,
    rationale: &'static str,
}

fn rule(disposition: &'static str, reason_code: &'static str, rationale: &'static str) -> Rule {
    Rule {
        disposition,
        reason_code,
        rationale,
    }
}

pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root must resolve")
}

pub fn read_json(path: &Path) -> Result<Value, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot_read_file:{}:{error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("invalid_json:{}:{error}", path.display()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let hashed = digest(&SHA256, bytes);
    let mut output = String::with_capacity(64);
    for byte in hashed.as_ref() {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

pub fn schema_is_valid(schema: &Value) -> bool {
    jsonschema::draft202012::meta::is_valid(schema)
}

pub fn validate_instance(schema: &Value, instance: &Value) -> Vec<String> {
    let validator = match jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(_) => return vec!["schema_could_not_be_compiled".to_owned()],
    };
    let mut errors = validator
        .iter_errors(instance)
        .map(|error| format!("schema_violation:{}", error.instance_path()))
        .collect::<Vec<_>>();
    errors.sort();
    errors.dedup();
    errors
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}

fn array_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Vec<Value>> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_array()
}

fn nonempty_string(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
}

fn unique_strings(values: &[Value], context: &str, errors: &mut Vec<String>) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let Some(text) = value.as_str() else {
            errors.push(format!("{context}_not_string:{index}"));
            continue;
        };
        if !seen.insert(text.to_owned()) {
            errors.push(format!("{context}_duplicate:{text}"));
        }
    }
    seen
}

fn claim_catalog<'a>(corpus: &'a Value, errors: &mut Vec<String>) -> BTreeMap<String, &'a Value> {
    let Some(claims) = array_at(corpus, &["claim_catalog"]) else {
        errors.push("claim_catalog_missing".to_owned());
        return BTreeMap::new();
    };
    let mut result = BTreeMap::new();
    for (index, claim) in claims.iter().enumerate() {
        let Some(id) = string_at(claim, &["claim_id"]) else {
            errors.push(format!("claim_id_missing:{index}"));
            continue;
        };
        if result.insert(id.to_owned(), claim).is_some() {
            errors.push(format!("claim_id_duplicate:{id}"));
        }
        let Some(dependencies) = array_at(claim, &["depends_on"]) else {
            errors.push(format!("claim_dependencies_missing:{id}"));
            continue;
        };
        unique_strings(dependencies, &format!("claim_dependency:{id}"), errors);
    }
    result
}

/// Structural checks beyond the schemas: exact catalog references, unique
/// identities, witness scope, and conservative treatment of incomplete closure.
pub fn corpus_invariants(corpus: &Value, oracle: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let catalog = claim_catalog(corpus, &mut errors);
    let Some(cases) = array_at(corpus, &["cases"]) else {
        errors.push("cases_missing".to_owned());
        return sorted(errors);
    };
    let Some(oracle_rows) = array_at(oracle, &["dispositions"]) else {
        errors.push("oracle_dispositions_missing".to_owned());
        return sorted(errors);
    };

    let mut cases_by_id = BTreeMap::new();
    for (case_index, case) in cases.iter().enumerate() {
        let Some(case_id) = string_at(case, &["case_id"]) else {
            errors.push(format!("case_id_missing:{case_index}"));
            continue;
        };
        if cases_by_id.insert(case_id.to_owned(), case).is_some() {
            errors.push(format!("case_id_duplicate:{case_id}"));
        }

        for field in ["source_configuration_ref", "target_configuration_ref"] {
            if !nonempty_string(case.get(field)) {
                errors.push(format!("case_reference_missing:{case_id}:{field}"));
            }
        }

        let changed = array_at(case, &["changed_components"]);
        if changed.is_none_or(Vec::is_empty) {
            errors.push(format!("changed_components_empty:{case_id}"));
        }
        let mut changed_ids = BTreeSet::new();
        for (index, component) in changed.into_iter().flatten().enumerate() {
            let Some(component_id) = string_at(component, &["component_id"]) else {
                errors.push(format!("changed_component_id_missing:{case_id}:{index}"));
                continue;
            };
            if !changed_ids.insert(component_id.to_owned()) {
                errors.push(format!(
                    "changed_component_duplicate:{case_id}:{component_id}"
                ));
            }
            for field in ["source_ref", "target_ref", "declared_scope"] {
                if !nonempty_string(component.get(field)) {
                    errors.push(format!(
                        "changed_component_field_missing:{case_id}:{component_id}:{field}"
                    ));
                }
            }
        }

        let witnesses = array_at(case, &["equivalence_witnesses"])
            .cloned()
            .unwrap_or_default();
        let mut witness_ids = BTreeSet::new();
        for witness in &witnesses {
            let Some(witness_id) = string_at(witness, &["witness_id"]) else {
                errors.push(format!("witness_id_missing:{case_id}"));
                continue;
            };
            if !witness_ids.insert(witness_id.to_owned()) {
                errors.push(format!("witness_id_duplicate:{case_id}:{witness_id}"));
            }
            if string_at(witness, &["status"]) != Some(FIXTURE_WITNESS_STATUS) {
                errors.push(format!(
                    "witness_status_not_admitted_synthetic:{case_id}:{witness_id}"
                ));
            }
            let covers = array_at(witness, &["covers_claims"]);
            if covers.is_none_or(Vec::is_empty) {
                errors.push(format!("witness_coverage_empty:{case_id}:{witness_id}"));
            }
            for claim_id in covers.into_iter().flatten().filter_map(Value::as_str) {
                if !catalog.contains_key(claim_id) {
                    errors.push(format!(
                        "witness_claim_not_in_catalog:{case_id}:{witness_id}:{claim_id}"
                    ));
                }
            }
        }

        let claims = array_at(case, &["claims_to_evaluate"]);
        if claims.is_none_or(Vec::is_empty) {
            errors.push(format!("claims_to_evaluate_empty:{case_id}"));
        }
        let mut evaluated_claims = BTreeSet::new();
        for (index, claim) in claims.into_iter().flatten().enumerate() {
            let Some(claim_id) = string_at(claim, &["claim_id"]) else {
                errors.push(format!("evaluated_claim_id_missing:{case_id}:{index}"));
                continue;
            };
            if !evaluated_claims.insert(claim_id.to_owned()) {
                errors.push(format!("evaluated_claim_duplicate:{case_id}:{claim_id}"));
            }
            if !catalog.contains_key(claim_id) {
                errors.push(format!(
                    "evaluated_claim_not_in_catalog:{case_id}:{claim_id}"
                ));
            }
            for field in ["prior_evidence_ref", "target_claim_profile_ref"] {
                if !nonempty_string(claim.get(field)) {
                    errors.push(format!(
                        "evaluated_claim_reference_missing:{case_id}:{claim_id}:{field}"
                    ));
                }
            }
        }
    }

    let mut oracle_by_id = BTreeMap::new();
    for (index, row) in oracle_rows.iter().enumerate() {
        let Some(case_id) = string_at(row, &["case_id"]) else {
            errors.push(format!("oracle_case_id_missing:{index}"));
            continue;
        };
        if oracle_by_id.insert(case_id.to_owned(), row).is_some() {
            errors.push(format!("oracle_case_id_duplicate:{case_id}"));
        }
        if !cases_by_id.contains_key(case_id) {
            errors.push(format!("oracle_case_not_in_corpus:{case_id}"));
        }
    }

    for (case_id, case) in &cases_by_id {
        let Some(oracle_row) = oracle_by_id.get(case_id) else {
            errors.push(format!("oracle_case_missing:{case_id}"));
            continue;
        };
        let corpus_claims = array_at(case, &["claims_to_evaluate"])
            .into_iter()
            .flatten()
            .filter_map(|claim| string_at(claim, &["claim_id"]))
            .collect::<BTreeSet<_>>();
        let oracle_claims = array_at(oracle_row, &["claims"])
            .into_iter()
            .flatten()
            .filter_map(|tuple| tuple.get(0).and_then(Value::as_str))
            .collect::<BTreeSet<_>>();
        if corpus_claims != oracle_claims {
            errors.push(format!("oracle_claim_coverage_mismatch:{case_id}"));
        }

        if bool_at(case, &["dependency_snapshot_complete"]) == Some(false) {
            for tuple in array_at(oracle_row, &["claims"]).into_iter().flatten() {
                let disposition = tuple.get(1).and_then(Value::as_str);
                if !matches!(disposition, Some("UnknownImpact" | "Unsupported")) {
                    errors.push(format!(
                        "incomplete_dependency_oracle_not_conservative:{case_id}:{}",
                        tuple
                            .get(0)
                            .and_then(Value::as_str)
                            .unwrap_or("unknown_claim")
                    ));
                }
            }
        }
    }
    for case_id in oracle_by_id.keys() {
        if !cases_by_id.contains_key(case_id) {
            errors.push(format!("oracle_extra_case:{case_id}"));
        }
    }
    sorted(errors)
}

fn sorted(mut errors: Vec<String>) -> Vec<String> {
    errors.sort();
    errors.dedup();
    errors
}

fn dependency_intersection(claim: &Value, case: &Value) -> Vec<String> {
    let dependencies = array_at(claim, &["depends_on"])
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let changed = array_at(case, &["changed_components"])
        .into_iter()
        .flatten()
        .filter_map(|component| string_at(component, &["component_id"]))
        .collect::<BTreeSet<_>>();
    dependencies
        .intersection(&changed)
        .map(|value| (*value).to_owned())
        .collect()
}

fn witness_admitted(case: &Value, family: &str, claim_id: &str) -> bool {
    let allowed_type = match (family, claim_id) {
        ("tool_equivalent_replacement", "reach.envelope" | "tool.swept_clearance") => {
            Some("qualified_tool_geometry_and_interface_equivalence")
        }
        (
            "payload_within_qualified_envelope",
            "payload.handling" | "mobility.surface_envelope" | "stopping.distance",
        ) => Some("qualified_mass_com_inertia_envelope"),
        ("battery_replacement_same_mechanical_envelope", "stopping.distance") => {
            Some("qualified_mechanical_interface_equivalence")
        }
        _ => None,
    };
    let Some(allowed_type) = allowed_type else {
        return false;
    };
    array_at(case, &["equivalence_witnesses"])
        .into_iter()
        .flatten()
        .any(|witness| {
            string_at(witness, &["witness_type"]) == Some(allowed_type)
                && string_at(witness, &["status"]) == Some(FIXTURE_WITNESS_STATUS)
                && array_at(witness, &["covers_claims"])
                    .is_some_and(|claims| claims.iter().any(|id| id.as_str() == Some(claim_id)))
        })
}

fn unaffected_reason(family: &str, claim_id: &str) -> &'static str {
    match (family, claim_id) {
        ("representation_preserving", _) => "DEPENDENCY_CLOSURE_UNCHANGED",
        ("camera_mount_extrinsic_change", "measurement.synchronization") => "TIMEBASE_UNCHANGED",
        ("battery_replacement_same_mechanical_envelope", "reach.envelope") => {
            "GEOMETRIC_DEPENDENCIES_UNCHANGED"
        }
        ("firmware_change_scope_unproven", "reach.envelope") => "MECHANICAL_DEPENDENCIES_UNCHANGED",
        ("timebase_replaced", "reach.envelope") => "TIMEBASE_UNRELATED_TO_GEOMETRIC_REACH",
        ("site_frame_epoch_changed", "reach.envelope") => "INTRINSIC_GEOMETRY_UNCHANGED",
        _ => "UNRELATED_DEPENDENCY_CLOSURE",
    }
}

/// The rule table is intentionally finite and specific to the 16 public RCQ
/// fixtures. It is not generalized to arbitrary robot configurations.
fn affected_rule(family: &str, claim_id: &str) -> Option<Rule> {
    let physical = |reason, rationale| rule("RequiresPhysicalReverification", reason, rationale);
    let bench = |reason, rationale| rule("RequiresBenchReverification", reason, rationale);
    let review = |reason, rationale| rule("RequiresApplicabilityReview", reason, rationale);
    let not_preserved = |reason, rationale| rule("NotPreserved", reason, rationale);
    let qualified = |reason, rationale| rule("PreservedByQualifiedEquivalence", reason, rationale);

    match (family, claim_id) {
        ("hmi_scope_unproven", "operator.recovery.observability") => Some(review(
            "HMI_CHANGE_SCOPE_UNPROVEN",
            "The changed display is in this claim's fixture dependency closure and no scope witness resolves the asserted cosmetic change.",
        )),
        ("tool_equivalent_replacement", "reach.envelope") => Some(qualified(
            "QUALIFIED_TOOL_GEOMETRY_EQUIVALENCE",
            "The synthetic witness covers the changed tool geometry and interface for the named reach claim.",
        )),
        ("tool_equivalent_replacement", "tool.swept_clearance") => Some(qualified(
            "QUALIFIED_TOOL_GEOMETRY_EQUIVALENCE",
            "The synthetic witness explicitly covers the tool geometry and mount for this clearance claim.",
        )),
        ("tool_equivalent_replacement", "process.output_quality") => Some(bench(
            "PROCESS_TOOL_REPLACEMENT",
            "Geometry and interface equivalence do not establish process output quality, wear state or process calibration.",
        )),
        ("tool_geometry_changed", "reach.envelope") => Some(physical(
            "TOOL_GEOMETRY_CHANGED",
            "Reach depends on the changed tool geometry and mount in this fixture profile.",
        )),
        ("tool_geometry_changed", "tool.swept_clearance") => Some(physical(
            "SWEPT_VOLUME_CHANGED",
            "The prior swept-volume evidence does not cover the changed tool.",
        )),
        ("tool_geometry_changed", "process.output_quality") => Some(physical(
            "PROCESS_TOOL_CHANGED",
            "The tool change affects the physical process output in this fixture profile.",
        )),
        ("payload_within_qualified_envelope", "payload.handling") => Some(qualified(
            "WITHIN_QUALIFIED_LOAD_ENVELOPE",
            "The synthetic witness covers mass, centre-of-mass and inertia for this claim.",
        )),
        ("payload_within_qualified_envelope", "mobility.surface_envelope") => Some(qualified(
            "WITHIN_QUALIFIED_MOBILITY_ENVELOPE",
            "The synthetic witness explicitly covers the new payload state for this claim.",
        )),
        ("payload_within_qualified_envelope", "stopping.distance") => Some(qualified(
            "WITHIN_QUALIFIED_STOPPING_LOAD_ENVELOPE",
            "The synthetic witness covers payload variables in this claim's declared load envelope.",
        )),
        ("payload_outside_qualified_envelope", "payload.handling") => Some(not_preserved(
            "OUTSIDE_QUALIFIED_LOAD_ENVELOPE",
            "The target payload lies outside the prior synthetic qualified envelope.",
        )),
        ("payload_outside_qualified_envelope", "mobility.surface_envelope") => Some(not_preserved(
            "OUTSIDE_QUALIFIED_MOBILITY_ENVELOPE",
            "The prior mobility evidence does not cover the target load and centre-of-mass state.",
        )),
        ("payload_outside_qualified_envelope", "stopping.distance") => Some(not_preserved(
            "OUTSIDE_QUALIFIED_STOPPING_LOAD_ENVELOPE",
            "The prior stopping evidence does not cover the target payload state.",
        )),
        ("camera_serial_replacement", "perception.obstacle_detection") => Some(bench(
            "CAMERA_SERIAL_CALIBRATION_MISSING",
            "The replacement camera serial has no supplied serial-specific calibration evidence.",
        )),
        ("camera_serial_replacement", "localization.visual") => Some(bench(
            "CAMERA_SERIAL_CALIBRATION_MISSING",
            "The camera serial and calibration path changed without target-frame verification.",
        )),
        ("camera_mount_extrinsic_change", "localization.visual") => Some(physical(
            "CAMERA_EXTRINSIC_CHANGED",
            "The changed sensor mount alters the physical frame relation.",
        )),
        ("camera_mount_extrinsic_change", "process.output_quality") => Some(physical(
            "INSPECTION_FRAME_CHANGED",
            "The output acceptance frame depends on the changed sensor mount and extrinsic.",
        )),
        ("battery_replacement_same_mechanical_envelope", "energy.return_reserve") => {
            Some(physical(
                "BATTERY_HEALTH_UNMEASURED",
                "The replacement battery serial has no measured health or usable-capacity evidence.",
            ))
        }
        ("battery_replacement_same_mechanical_envelope", "stopping.distance") => Some(qualified(
            "BATTERY_MASS_INTERFACE_EQUIVALENCE",
            "The synthetic witness covers the unchanged mass and mechanical envelope relevant to this stopping claim; electrical-health claims remain separate.",
        )),
        ("drive_module_replacement", "mobility.surface_envelope") => Some(physical(
            "DRIVE_MODULE_CHANGED",
            "Traction and terrain mobility depend on the changed drive module.",
        )),
        ("drive_module_replacement", "localization.visual") => Some(physical(
            "ODOMETRY_SOURCE_CHANGED",
            "The drive/odometry source changed in the localization chain.",
        )),
        ("drive_module_replacement", "stopping.distance") => Some(physical(
            "BRAKING_TRACTION_CONFIGURATION_CHANGED",
            "Stopping depends on drive, tyre/track and surface interaction.",
        )),
        ("safety_chain_revision", "safety.interlock_response") => Some(physical(
            "SAFETY_CHAIN_REVISION",
            "The safety controller and braking chain revisions changed.",
        )),
        ("safety_chain_revision", "stopping.distance") => Some(physical(
            "BRAKE_CHAIN_REVISION",
            "The physical stopping response must be re-characterized for the target chain.",
        )),
        ("firmware_change_scope_unproven", "localization.visual") => Some(review(
            "FIRMWARE_SCOPE_UNPROVEN",
            "The non-functional firmware assertion has no complete scope or equivalence witness.",
        )),
        ("firmware_change_scope_unproven", "operator.recovery.observability") => Some(review(
            "DIAGNOSTIC_FIRMWARE_SCOPE_UNPROVEN",
            "The changed driver/diagnostic path may affect fault visibility and event semantics.",
        )),
        ("controller_timing_changed", "stopping.distance") => Some(physical(
            "MOTION_TIMING_CHANGED",
            "Changed command timing affects the physical stopping response.",
        )),
        ("controller_timing_changed", "measurement.synchronization") => Some(bench(
            "TIMESTAMP_PIPELINE_CHANGED",
            "The timestamp pipeline changed and requires controlled synchronization testing.",
        )),
        ("controller_timing_changed", "tool.swept_clearance") => Some(physical(
            "TRAJECTORY_CONTROL_CHANGED",
            "The prior motion-envelope evidence is configuration-specific.",
        )),
        ("timebase_replaced", "measurement.synchronization") => Some(bench(
            "TIMEBASE_REPLACED_NO_MAPPING",
            "The changed clock chain has no independently measured alignment and uncertainty result.",
        )),
        ("timebase_replaced", "localization.visual") => Some(bench(
            "SENSOR_TIME_ALIGNMENT_UNPROVEN",
            "Localization fusion timing is not demonstrated to remain applicable.",
        )),
        ("site_frame_epoch_changed", "localization.visual") => Some(not_preserved(
            "SITE_FRAME_EPOCH_CHANGED",
            "Site/map/frame-specific localization evidence is bound to the previous epoch.",
        )),
        ("site_frame_epoch_changed", "process.output_quality") => Some(physical(
            "NEW_SITE_FRAME_REQUIRES_REFEREE_CHECK",
            "The measurement and acceptance frame changed and requires a new referee check.",
        )),
        _ => None,
    }
}

fn evaluate_claim(family: &str, case: &Value, claim: &Value, catalog_claim: &Value) -> Value {
    let claim_id = string_at(claim, &["claim_id"]).unwrap_or("unknown_claim");
    if bool_at(case, &["dependency_snapshot_complete"]) != Some(true) {
        return json!({
            "claim_id": claim_id,
            "changed_dependencies": dependency_intersection(catalog_claim, case),
            "disposition": "UnknownImpact",
            "reason_code": "DEPENDENCY_CLOSURE_INCOMPLETE",
            "rationale": "The case dependency snapshot is incomplete; absence of a declared dependency edge cannot prove independence."
        });
    }

    let changed_dependencies = dependency_intersection(catalog_claim, case);
    if changed_dependencies.is_empty() {
        return json!({
            "claim_id": claim_id,
            "changed_dependencies": [],
            "disposition": "PreservedByExactIdentity",
            "reason_code": unaffected_reason(family, claim_id),
            "rationale": "No component in this claim's declared synthetic dependency closure changed."
        });
    }

    let Some(mut selected) = affected_rule(family, claim_id) else {
        return json!({
            "claim_id": claim_id,
            "changed_dependencies": changed_dependencies,
            "disposition": "Unsupported",
            "reason_code": "UNSUPPORTED_FIXTURE_RULE",
            "rationale": "This family/claim combination has no explicit v0.1 development fixture rule."
        });
    };

    if selected.disposition == "PreservedByQualifiedEquivalence"
        && !witness_admitted(case, family, claim_id)
    {
        selected = rule(
            "RequiresPhysicalReverification",
            "QUALIFIED_ENVELOPE_WITNESS_NOT_ADMITTED",
            "A preservation disposition requires an admitted synthetic witness of the allowed type that explicitly covers this exact claim.",
        );
    }

    json!({
        "claim_id": claim_id,
        "changed_dependencies": changed_dependencies,
        "disposition": selected.disposition,
        "reason_code": selected.reason_code,
        "rationale": selected.rationale
    })
}

fn oracle_map(oracle: &Value) -> Result<BTreeMap<(String, String), (String, String)>, Vec<String>> {
    let mut errors = Vec::new();
    let mut expected = BTreeMap::new();
    let Some(rows) = array_at(oracle, &["dispositions"]) else {
        return Err(vec!["oracle_dispositions_missing".to_owned()]);
    };
    for row in rows {
        let case_id = string_at(row, &["case_id"]).unwrap_or("unknown_case");
        let Some(claims) = array_at(row, &["claims"]) else {
            errors.push(format!("oracle_claims_missing:{case_id}"));
            continue;
        };
        for tuple in claims {
            let claim_id = tuple
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("unknown_claim");
            let disposition = tuple
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("invalid_disposition");
            let reason = tuple
                .get(2)
                .and_then(Value::as_str)
                .unwrap_or("invalid_reason");
            if !RCQ_DISPOSITIONS.contains(&disposition) {
                errors.push(format!(
                    "oracle_disposition_invalid:{case_id}:{claim_id}:{disposition}"
                ));
            }
            let key = (case_id.to_owned(), claim_id.to_owned());
            if expected
                .insert(key.clone(), (disposition.to_owned(), reason.to_owned()))
                .is_some()
            {
                errors.push(format!("oracle_tuple_duplicate:{}:{}", key.0, key.1));
            }
        }
    }
    if errors.is_empty() {
        Ok(expected)
    } else {
        Err(sorted(errors))
    }
}

/// Evaluate only the finite public v0.1 fixture rules. It returns computed
/// results and a strict comparison to the separate public oracle.
pub fn evaluate_against_oracle(corpus: &Value, oracle: &Value) -> Result<Value, Vec<String>> {
    let invariants = corpus_invariants(corpus, oracle);
    if !invariants.is_empty() {
        return Err(invariants);
    }
    let mut structural_errors = Vec::new();
    let catalog = claim_catalog(corpus, &mut structural_errors);
    if !structural_errors.is_empty() {
        return Err(sorted(structural_errors));
    }
    let expected = oracle_map(oracle).map_err(|errors| errors)?;
    let mut cases = array_at(corpus, &["cases"]).cloned().unwrap_or_default();
    cases.sort_by_key(|case| string_at(case, &["case_id"]).unwrap_or("").to_owned());

    let mut results = Vec::new();
    let mut actual = BTreeMap::new();
    let mut disposition_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut critical_changed_dependency_preserved = 0_u64;
    let mut incomplete_case_claims = 0_u64;
    let mut agreement_count = 0_u64;

    for case in &cases {
        let case_id = string_at(case, &["case_id"]).unwrap_or("unknown_case");
        let family = string_at(case, &["mutation_family"]).unwrap_or("unknown_family");
        let mut claims = array_at(case, &["claims_to_evaluate"])
            .cloned()
            .unwrap_or_default();
        claims.sort_by_key(|claim| string_at(claim, &["claim_id"]).unwrap_or("").to_owned());
        let mut claim_results = Vec::new();

        for claim in &claims {
            let claim_id = string_at(claim, &["claim_id"]).unwrap_or("unknown_claim");
            let Some(catalog_claim) = catalog.get(claim_id) else {
                return Err(vec![format!(
                    "evaluated_claim_not_in_catalog:{case_id}:{claim_id}"
                )]);
            };
            let result = evaluate_claim(family, case, claim, catalog_claim);
            let disposition = string_at(&result, &["disposition"]).unwrap_or("Unsupported");
            let reason = string_at(&result, &["reason_code"]).unwrap_or("UNSPECIFIED");
            let key = (case_id.to_owned(), claim_id.to_owned());
            actual.insert(key.clone(), (disposition.to_owned(), reason.to_owned()));
            if expected
                .get(&key)
                .is_some_and(|tuple| tuple.0 == disposition && tuple.1 == reason)
            {
                agreement_count += 1;
            }
            *disposition_counts
                .entry(disposition.to_owned())
                .or_default() += 1;

            if bool_at(case, &["dependency_snapshot_complete"]) == Some(false) {
                incomplete_case_claims += 1;
            }
            let criticality = string_at(catalog_claim, &["criticality"]);
            let changed = array_at(&result, &["changed_dependencies"])
                .is_some_and(|values| !values.is_empty());
            if changed
                && criticality == Some("safety_critical")
                && matches!(
                    disposition,
                    "PreservedByExactIdentity"
                        | "PreservedByQualifiedEquivalence"
                        | "PreservedByExactRecomputation"
                )
            {
                critical_changed_dependency_preserved += 1;
            }
            claim_results.push(result);
        }
        results.push(json!({
            "case_id": case_id,
            "mutation_family": family,
            "dependency_snapshot_complete": case["dependency_snapshot_complete"],
            "claims": claim_results
        }));
    }

    let mut mismatches = Vec::new();
    for (key, expected_tuple) in &expected {
        match actual.get(key) {
            Some(actual_tuple) if actual_tuple == expected_tuple => {}
            Some(actual_tuple) => mismatches.push(json!({
                "case_id": key.0,
                "claim_id": key.1,
                "expected_disposition": expected_tuple.0,
                "expected_reason_code": expected_tuple.1,
                "actual_disposition": actual_tuple.0,
                "actual_reason_code": actual_tuple.1
            })),
            None => mismatches.push(json!({
                "case_id": key.0,
                "claim_id": key.1,
                "expected_disposition": expected_tuple.0,
                "expected_reason_code": expected_tuple.1,
                "actual_disposition": null,
                "actual_reason_code": null
            })),
        }
    }
    for key in actual.keys() {
        if !expected.contains_key(key) {
            mismatches.push(json!({
                "case_id": key.0,
                "claim_id": key.1,
                "expected_disposition": null,
                "expected_reason_code": null,
                "actual_disposition": actual[key].0,
                "actual_reason_code": actual[key].1
            }));
        }
    }
    mismatches.sort_by_key(|value| {
        (
            string_at(value, &["case_id"]).unwrap_or("").to_owned(),
            string_at(value, &["claim_id"]).unwrap_or("").to_owned(),
        )
    });
    let oracle_match = mismatches.is_empty() && agreement_count as usize == expected.len();

    let disposition_counts = disposition_counts
        .into_iter()
        .map(|(name, count)| json!({"disposition": name, "count": count}))
        .collect::<Vec<_>>();

    Ok(json!({
        "schema_version": RESULT_SCHEMA_VERSION,
        "evaluator_id": EVALUATOR_ID,
        "benchmark_id": string_at(corpus, &["benchmark_id"]).unwrap_or("unknown_benchmark"),
        "visibility": "public_development_fixture_only",
        "oracle_comparison": {
            "oracle_visibility": "public_development_oracle_not_hidden_holdout",
            "agreement_count": agreement_count,
            "expected_count": expected.len(),
            "exact_disposition_and_reason_agreement": oracle_match,
            "mismatches": mismatches
        },
        "summary": {
            "case_count": results.len(),
            "claim_result_count": actual.len(),
            "disposition_counts": disposition_counts,
            "safety_critical_changed_dependency_preserved_count": critical_changed_dependency_preserved,
            "incomplete_dependency_claim_count": incomplete_case_claims
        },
        "cases": results,
        "nonclaims": [
            "This evaluator implements only the finite public synthetic RCQ v0.1 fixture rules.",
            "The public oracle is development-visible and is not hidden, independent custody, or a scored holdout.",
            "Synthetic equivalence witnesses do not establish physical equivalence or evidence admissibility.",
            "No result establishes capability, safety, physical qualification, certification, standards conformity, or work authorization.",
            "Stable owner interfaces for generic change impact, dependency lineage, capability, physical trial evidence and evidence admissibility are not implemented by this crate."
        ]
    }))
}

pub fn validate_package(
    corpus: &Value,
    oracle: &Value,
    corpus_schema: &Value,
    oracle_schema: &Value,
    result_schema: &Value,
) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();
    for (name, schema) in [
        ("corpus", corpus_schema),
        ("oracle", oracle_schema),
        ("result", result_schema),
    ] {
        if !schema_is_valid(schema) {
            errors.push(format!("schema_meta_invalid:{name}"));
        }
    }
    errors.extend(
        validate_instance(corpus_schema, corpus)
            .into_iter()
            .map(|error| format!("corpus:{error}")),
    );
    errors.extend(
        validate_instance(oracle_schema, oracle)
            .into_iter()
            .map(|error| format!("oracle:{error}")),
    );
    errors.extend(corpus_invariants(corpus, oracle));
    if !errors.is_empty() {
        return Err(sorted(errors));
    }
    let mut result = evaluate_against_oracle(corpus, oracle)?;
    let canonical_corpus =
        serde_json::to_vec(corpus).map_err(|error| vec![format!("corpus_canonicalize:{error}")])?;
    let canonical_oracle =
        serde_json::to_vec(oracle).map_err(|error| vec![format!("oracle_canonicalize:{error}")])?;
    let canonical_result_schema = serde_json::to_vec(result_schema)
        .map_err(|error| vec![format!("result_schema_canonicalize:{error}")])?;
    result["input_digests"] = json!({
        "canonicalization": "serde_json-1.0.154-default-map-order-compact-utf8-no-trailing-newline-v1",
        "corpus_canonical_json_sha256": sha256_hex(&canonical_corpus),
        "oracle_canonical_json_sha256": sha256_hex(&canonical_oracle),
        "result_schema_canonical_json_sha256": sha256_hex(&canonical_result_schema)
    });
    errors.extend(
        validate_instance(result_schema, &result)
            .into_iter()
            .map(|error| format!("result:{error}")),
    );
    if !errors.is_empty() {
        Err(sorted(errors))
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (Value, Value, Value, Value, Value) {
        let base = repository_root().join(PACKAGE_DIR);
        let corpus =
            read_json(&base.join("robot-config-change-corpus-v0.1.json")).expect("corpus parses");
        let oracle = read_json(&base.join("development-oracle-v0.1.json")).expect("oracle parses");
        let corpus_schema =
            read_json(&base.join("schemas/robot-config-change-corpus-v0.1.schema.json"))
                .expect("corpus schema parses");
        let oracle_schema = read_json(&base.join("schemas/development-oracle-v0.1.schema.json"))
            .expect("oracle schema parses");
        let result_schema = read_json(&base.join("schemas/development-result-v0.1.schema.json"))
            .expect("result schema parses");
        (corpus, oracle, corpus_schema, oracle_schema, result_schema)
    }

    #[test]
    fn all_schemas_are_draft_2020_12_and_closed_instances_validate() {
        let (corpus, oracle, corpus_schema, oracle_schema, result_schema) = fixtures();
        assert_eq!(
            string_at(&corpus_schema, &["$schema"]),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
        assert_eq!(
            string_at(&oracle_schema, &["$schema"]),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
        assert_eq!(
            string_at(&result_schema, &["$schema"]),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
        assert!(
            validate_package(
                &corpus,
                &oracle,
                &corpus_schema,
                &oracle_schema,
                &result_schema
            )
            .is_ok()
        );
    }

    #[test]
    fn all_sixteen_public_cases_match_exact_disposition_and_reason_oracle() {
        let (corpus, oracle, corpus_schema, oracle_schema, result_schema) = fixtures();
        let result = validate_package(
            &corpus,
            &oracle,
            &corpus_schema,
            &oracle_schema,
            &result_schema,
        )
        .unwrap();
        assert_eq!(
            result["oracle_comparison"]["exact_disposition_and_reason_agreement"],
            true
        );
        assert_eq!(result["oracle_comparison"]["agreement_count"], 51);
        assert_eq!(result["oracle_comparison"]["expected_count"], 51);
        assert_eq!(result["summary"]["case_count"], 16);
        assert_eq!(result["summary"]["claim_result_count"], 51);
        assert_eq!(
            result["summary"]["safety_critical_changed_dependency_preserved_count"],
            0
        );
        assert_eq!(result["summary"]["incomplete_dependency_claim_count"], 4);
    }

    #[test]
    fn incomplete_dependency_snapshot_blocks_every_claim_including_unrelated_claims() {
        let (corpus, oracle, _, _, _) = fixtures();
        let result = evaluate_against_oracle(&corpus, &oracle).unwrap();
        let rcq016 = array_at(&result, &["cases"])
            .unwrap()
            .iter()
            .find(|case| string_at(case, &["case_id"]) == Some("RCQ-016"))
            .unwrap();
        for claim in array_at(rcq016, &["claims"]).unwrap() {
            assert_eq!(claim["disposition"], "UnknownImpact");
            assert_eq!(claim["reason_code"], "DEPENDENCY_CLOSURE_INCOMPLETE");
        }
    }

    #[test]
    fn descriptions_and_case_order_do_not_change_decisions() {
        let (corpus, oracle, _, _, _) = fixtures();
        let baseline = evaluate_against_oracle(&corpus, &oracle).unwrap();
        let mut changed = corpus.clone();
        let rows = changed["cases"].as_array_mut().unwrap();
        rows.reverse();
        for case in rows {
            case["description"] =
                Value::String("renamed product and scenario marketing prose".to_owned());
        }
        let replay = evaluate_against_oracle(&changed, &oracle).unwrap();
        assert_eq!(
            serde_json::to_vec(&baseline).unwrap(),
            serde_json::to_vec(&replay).unwrap()
        );
    }

    #[test]
    fn missing_equivalence_witness_cannot_preserve_a_safety_critical_claim() {
        let (mut corpus, oracle, _, _, _) = fixtures();
        let rcq005 = corpus["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|case| string_at(case, &["case_id"]) == Some("RCQ-005"))
            .unwrap();
        let witness = rcq005["equivalence_witnesses"]
            .as_array_mut()
            .unwrap()
            .first_mut()
            .unwrap();
        witness["covers_claims"] = json!(["mobility.surface_envelope", "stopping.distance"]);
        let result = evaluate_against_oracle(&corpus, &oracle).unwrap();
        assert_eq!(
            result["oracle_comparison"]["exact_disposition_and_reason_agreement"],
            false
        );
        let rcq005_result = array_at(&result, &["cases"])
            .unwrap()
            .iter()
            .find(|case| string_at(case, &["case_id"]) == Some("RCQ-005"))
            .unwrap();
        let payload = array_at(rcq005_result, &["claims"])
            .unwrap()
            .iter()
            .find(|claim| string_at(claim, &["claim_id"]) == Some("payload.handling"))
            .unwrap();
        assert_eq!(payload["disposition"], "RequiresPhysicalReverification");
        assert_eq!(
            payload["reason_code"],
            "QUALIFIED_ENVELOPE_WITNESS_NOT_ADMITTED"
        );
    }

    #[test]
    fn added_witness_claim_does_not_expand_the_allowed_witness_type_policy() {
        let (mut corpus, oracle, _, _, _) = fixtures();
        let rcq003 = corpus["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|case| string_at(case, &["case_id"]) == Some("RCQ-003"))
            .unwrap();
        rcq003["equivalence_witnesses"][0]["covers_claims"]
            .as_array_mut()
            .unwrap()
            .push(Value::String("process.output_quality".to_owned()));
        let result = evaluate_against_oracle(&corpus, &oracle).unwrap();
        let rcq003_result = array_at(&result, &["cases"])
            .unwrap()
            .iter()
            .find(|case| string_at(case, &["case_id"]) == Some("RCQ-003"))
            .unwrap();
        let process = array_at(rcq003_result, &["claims"])
            .unwrap()
            .iter()
            .find(|claim| string_at(claim, &["claim_id"]) == Some("process.output_quality"))
            .unwrap();
        assert_eq!(process["disposition"], "RequiresBenchReverification");
        assert_eq!(process["reason_code"], "PROCESS_TOOL_REPLACEMENT");
    }

    #[test]
    fn duplicate_or_missing_case_and_claim_references_fail_closed() {
        let (mut corpus, oracle, _, _, _) = fixtures();
        corpus["cases"][1]["case_id"] = Value::String("RCQ-001".to_owned());
        let errors = corpus_invariants(&corpus, &oracle);
        assert!(
            errors
                .iter()
                .any(|error| error == "case_id_duplicate:RCQ-001")
        );

        let (mut corpus, oracle, _, _, _) = fixtures();
        corpus["cases"][0]["claims_to_evaluate"][0]["claim_id"] =
            Value::String("unregistered.claim".to_owned());
        let errors = corpus_invariants(&corpus, &oracle);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("evaluated_claim_not_in_catalog"))
        );
    }

    #[test]
    fn oracle_cannot_omit_or_add_claim_rows() {
        let (corpus, mut oracle, _, _, _) = fixtures();
        oracle["dispositions"][0]["claims"]
            .as_array_mut()
            .unwrap()
            .pop();
        let errors = corpus_invariants(&corpus, &oracle);
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("oracle_claim_coverage_mismatch:"))
        );

        let (corpus, mut oracle, _, _, _) = fixtures();
        oracle["dispositions"][0]["claims"]
            .as_array_mut()
            .unwrap()
            .push(json!([
                "unexpected.claim",
                "UnknownImpact",
                "UNKNOWN",
                "fixture"
            ]));
        let errors = corpus_invariants(&corpus, &oracle);
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("oracle_claim_coverage_mismatch:"))
        );
    }

    #[test]
    fn result_is_deterministic_and_digest_inputs_are_stable() {
        let (corpus, oracle, _, _, _) = fixtures();
        let first = evaluate_against_oracle(&corpus, &oracle).unwrap();
        let second = evaluate_against_oracle(&corpus, &oracle).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(
            sha256_hex(&serde_json::to_vec(&corpus).unwrap()),
            sha256_hex(&serde_json::to_vec(&corpus).unwrap())
        );
    }
}
