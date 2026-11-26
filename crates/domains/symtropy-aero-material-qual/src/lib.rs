// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Semantic checks for AERO-MATERIAL-PROCESS-QUAL-001 v0.1.
//!
//! This evaluator checks the referential and partition contract represented by
//! the synthetic case. It does not implement full JSON Schema Draft 2020-12
//! validation, measurement validity, material statistics, physical models,
//! design allowables, part acceptance, airworthiness, or certification.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub mod evidence_binding_v0_2;
pub mod evidence_gap_packet_v0_1;
pub mod observer_coverage_v0_1;
pub mod vector_watermark_v0_1;

const PROFILE_ID: &str = "aero-material-process-qual-v0.1";
const CLAIM_CEILING: &str = "synthetic_contract_test_only";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Finding {
    pub code: String,
    pub entity_id: String,
    pub detail: String,
}

impl Finding {
    fn new(code: &str, entity_id: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            entity_id: entity_id.to_owned(),
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvaluationReport {
    pub evaluator_profile: String,
    pub case_id: Option<String>,
    pub schema_validation_status: String,
    pub structurally_complete: bool,
    pub structural_status: String,
    pub process_domain_status: String,
    pub model_applicability_status: String,
    pub evidence_sufficiency_status: String,
    pub decision_status: String,
    pub physical_qualification_status: String,
    pub entity_counts: BTreeMap<String, usize>,
    pub observation_status_counts: BTreeMap<String, usize>,
    pub violations: Vec<Finding>,
    pub blockers: Vec<Finding>,
    pub capability_gaps: Vec<String>,
}

fn field_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn push_finding(findings: &mut Vec<Finding>, code: &str, entity_id: &str, detail: &str) {
    findings.push(Finding::new(code, entity_id, detail));
}

fn required_array<'a>(root: &'a Value, path: &str, findings: &mut Vec<Finding>) -> Vec<&'a Value> {
    let mut current = root;
    for segment in path.split('.') {
        let Some(next) = current.get(segment) else {
            push_finding(
                findings,
                "MISSING_COLLECTION",
                path,
                "Required collection path is absent.",
            );
            return Vec::new();
        };
        current = next;
    }
    match current.as_array() {
        Some(values) => values.iter().collect(),
        None => {
            push_finding(
                findings,
                "INVALID_COLLECTION",
                path,
                "Required collection is not an array.",
            );
            Vec::new()
        }
    }
}

fn index_by_id<'a>(
    rows: &[&'a Value],
    kind: &str,
    findings: &mut Vec<Finding>,
) -> BTreeMap<String, &'a Value> {
    let mut index = BTreeMap::new();
    for (position, row) in rows.iter().enumerate() {
        let Some(id) = field_str(row, "id") else {
            push_finding(
                findings,
                "MISSING_ID",
                &format!("{kind}[{position}]"),
                "Entity requires a string id.",
            );
            continue;
        };
        if id.trim().is_empty() {
            push_finding(
                findings,
                "BLANK_ID",
                &format!("{kind}[{position}]"),
                "Entity id must not be blank.",
            );
            continue;
        }
        if id != id.trim() {
            push_finding(
                findings,
                "NON_CANONICAL_ID",
                id,
                "Entity id has leading or trailing whitespace.",
            );
            continue;
        }
        if index.contains_key(id) {
            push_finding(
                findings,
                "DUPLICATE_ID",
                id,
                &format!("Duplicate identity in {kind}."),
            );
        } else {
            index.insert(id.to_owned(), *row);
        }
    }
    index
}

fn check_foreign_key(
    rows: &[&Value],
    owner_kind: &str,
    foreign_field: &str,
    target_kind: &str,
    target_index: &BTreeMap<String, &Value>,
    findings: &mut Vec<Finding>,
) {
    for (position, row) in rows.iter().enumerate() {
        let owner_id = field_str(row, "id")
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{owner_kind}[{position}]"));
        match field_str(row, foreign_field) {
            None => push_finding(
                findings,
                "MISSING_REFERENCE",
                &owner_id,
                &format!("Missing {foreign_field} reference to {target_kind}."),
            ),
            Some(target_id) if !target_index.contains_key(target_id) => push_finding(
                findings,
                "DANGLING_REFERENCE",
                &owner_id,
                &format!("{foreign_field} references unknown {target_kind} identity {target_id}."),
            ),
            Some(_) => {}
        }
    }
}

fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|left, right| {
        (&left.code, &left.entity_id, &left.detail).cmp(&(
            &right.code,
            &right.entity_id,
            &right.detail,
        ))
    });
}

fn allowed_grouping_level(level: &str) -> bool {
    matches!(
        level,
        "material_lot" | "process_batch" | "panel" | "test_campaign" | "configuration"
    )
}

/// Evaluate the v0.1 semantic contract.
///
/// The return value is deterministic for record-order permutations. A
/// structurally complete report is not a decision approval: this profile is
/// explicitly synthetic-only and always reports the real-world decision gate
/// as ineligible.
pub fn evaluate_case(case: &Value) -> EvaluationReport {
    let mut violations = Vec::new();
    let mut blockers = Vec::new();

    let case_id = field_str(case, "case_id").map(str::to_owned);
    if case_id.as_deref().is_none_or(|id| id.trim().is_empty()) {
        push_finding(
            &mut violations,
            "MISSING_OR_BLANK_CASE_ID",
            "case",
            "The semantic contract requires a non-empty case_id.",
        );
    }
    if field_str(case, "profile_id") != Some(PROFILE_ID) {
        push_finding(
            &mut violations,
            "PROFILE_ID_MISMATCH",
            case_id.as_deref().unwrap_or("case"),
            "Expected the v0.1 AERO material/process profile identifier.",
        );
    }
    if case.get("synthetic_only").and_then(Value::as_bool) != Some(true)
        || field_str(case, "claim_ceiling") != Some(CLAIM_CEILING)
        || case
            .pointer("/known_truth/not_a_physical_model")
            .and_then(Value::as_bool)
            != Some(true)
    {
        push_finding(
            &mut violations,
            "CLAIM_CEILING_VIOLATION",
            case_id.as_deref().unwrap_or("case"),
            "v0.1 must remain explicitly synthetic and must not be represented as a physical model.",
        );
    }

    let lots = required_array(case, "hierarchy.material_lots", &mut violations);
    let batches = required_array(case, "hierarchy.process_batches", &mut violations);
    let panels = required_array(case, "hierarchy.panels", &mut violations);
    let specimens = required_array(case, "hierarchy.specimens", &mut violations);
    let campaigns = required_array(case, "hierarchy.test_campaigns", &mut violations);
    let configurations = required_array(case, "hierarchy.configurations", &mut violations);
    let observations = required_array(case, "observations", &mut violations);
    let assignments = required_array(case, "partition_contract.assignments", &mut violations);

    for (kind, rows) in [
        ("material_lots", lots.as_slice()),
        ("process_batches", batches.as_slice()),
        ("panels", panels.as_slice()),
        ("specimens", specimens.as_slice()),
        ("test_campaigns", campaigns.as_slice()),
        ("configurations", configurations.as_slice()),
        ("observations", observations.as_slice()),
        ("partition_assignments", assignments.as_slice()),
    ] {
        if rows.is_empty() {
            push_finding(
                &mut violations,
                "EMPTY_REQUIRED_COLLECTION",
                kind,
                "The v0.1 semantic contract requires this collection to contain at least one record.",
            );
        }
    }

    let lot_index = index_by_id(&lots, "material_lots", &mut violations);
    let batch_index = index_by_id(&batches, "process_batches", &mut violations);
    let panel_index = index_by_id(&panels, "panels", &mut violations);
    let specimen_index = index_by_id(&specimens, "specimens", &mut violations);
    let campaign_index = index_by_id(&campaigns, "test_campaigns", &mut violations);
    let configuration_index = index_by_id(&configurations, "configurations", &mut violations);
    let _observation_index = index_by_id(&observations, "observations", &mut violations);

    check_foreign_key(
        &batches,
        "process_batch",
        "material_lot_id",
        "material_lot",
        &lot_index,
        &mut violations,
    );
    check_foreign_key(
        &panels,
        "panel",
        "process_batch_id",
        "process_batch",
        &batch_index,
        &mut violations,
    );
    check_foreign_key(
        &specimens,
        "specimen",
        "panel_id",
        "panel",
        &panel_index,
        &mut violations,
    );
    check_foreign_key(
        &specimens,
        "specimen",
        "configuration_id",
        "configuration",
        &configuration_index,
        &mut violations,
    );
    check_foreign_key(
        &observations,
        "observation",
        "specimen_id",
        "specimen",
        &specimen_index,
        &mut violations,
    );
    check_foreign_key(
        &observations,
        "observation",
        "campaign_id",
        "test_campaign",
        &campaign_index,
        &mut violations,
    );
    check_foreign_key(
        &assignments,
        "partition_assignment",
        "specimen_id",
        "specimen",
        &specimen_index,
        &mut violations,
    );

    let mut observation_status_counts = BTreeMap::from([
        ("observed".to_owned(), 0usize),
        ("censored".to_owned(), 0usize),
        ("excluded".to_owned(), 0usize),
        ("missing".to_owned(), 0usize),
    ]);
    let mut campaigns_by_specimen: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for observation in &observations {
        let observation_id = field_str(observation, "id").unwrap_or("observation");
        let specimen_id = field_str(observation, "specimen_id");
        let campaign_id = field_str(observation, "campaign_id");
        if let (Some(specimen_id), Some(campaign_id)) = (specimen_id, campaign_id) {
            campaigns_by_specimen
                .entry(specimen_id.to_owned())
                .or_default()
                .insert(campaign_id.to_owned());
        }

        if field_str(observation, "quantity") != Some("response_index")
            || field_str(observation, "unit") != Some("synthetic_index")
        {
            push_finding(
                &mut violations,
                "NON_SYNTHETIC_QUANTITY",
                observation_id,
                "v0.1 observations must use response_index / synthetic_index.",
            );
        }

        match field_str(observation, "status") {
            Some("observed") => {
                *observation_status_counts
                    .get_mut("observed")
                    .expect("observed counter exists") += 1;
                if observation
                    .get("value_index")
                    .and_then(Value::as_f64)
                    .is_none()
                    || observation
                        .get("standard_uncertainty_index")
                        .and_then(Value::as_f64)
                        .is_none_or(|value| value < 0.0)
                {
                    push_finding(
                        &mut violations,
                        "INVALID_OBSERVED_VALUE",
                        observation_id,
                        "Observed records require a numeric value and non-negative standard uncertainty.",
                    );
                }
            }
            Some("censored") => {
                *observation_status_counts
                    .get_mut("censored")
                    .expect("censored counter exists") += 1;
                if observation
                    .get("censoring_limit_index")
                    .and_then(Value::as_f64)
                    .is_none()
                    || field_str(observation, "reason_code").is_none()
                {
                    push_finding(
                        &mut violations,
                        "INCOMPLETE_CENSORED_RECORD",
                        observation_id,
                        "Censored records require a numeric limit and reason code.",
                    );
                }
            }
            Some("excluded") => {
                *observation_status_counts
                    .get_mut("excluded")
                    .expect("excluded counter exists") += 1;
                if field_str(observation, "reason_code").is_none() {
                    push_finding(
                        &mut violations,
                        "EXCLUSION_WITHOUT_REASON",
                        observation_id,
                        "Excluded records must retain a reason code.",
                    );
                }
            }
            Some("missing") => {
                *observation_status_counts
                    .get_mut("missing")
                    .expect("missing counter exists") += 1;
                if field_str(observation, "reason_code").is_none() {
                    push_finding(
                        &mut violations,
                        "MISSINGNESS_WITHOUT_REASON",
                        observation_id,
                        "Missing records must retain a reason code.",
                    );
                }
            }
            _ => push_finding(
                &mut violations,
                "UNKNOWN_OBSERVATION_STATUS",
                observation_id,
                "Observation status is outside the v0.1 state set.",
            ),
        }
    }

    for campaign in &campaigns {
        let campaign_id = field_str(campaign, "id").unwrap_or("campaign");
        match field_str(campaign, "calibration_state") {
            Some("active") => {}
            Some("expired") | Some("unknown") => push_finding(
                &mut blockers,
                "CALIBRATION_NOT_DECLARED_ACTIVE",
                campaign_id,
                "The campaign's declared calibration state is expired or unknown; linked observations are not eligible for a positive evidence claim.",
            ),
            _ => push_finding(
                &mut violations,
                "UNKNOWN_CALIBRATION_STATE",
                campaign_id,
                "Calibration state is outside the v0.1 state set.",
            ),
        }
    }

    let mut out_of_domain_batches = 0usize;
    let mut unknown_domain_batches = 0usize;
    for batch in &batches {
        let batch_id = field_str(batch, "id").unwrap_or("process_batch");
        match field_str(batch, "process_state") {
            Some("in_domain") => {}
            Some("out_of_domain") => {
                out_of_domain_batches += 1;
                push_finding(
                    &mut blockers,
                    "PROCESS_OUT_OF_DOMAIN",
                    batch_id,
                    "Out-of-domain process state cannot be promoted by a favourable response value.",
                );
            }
            Some("unknown") => {
                unknown_domain_batches += 1;
                push_finding(
                    &mut blockers,
                    "PROCESS_DOMAIN_UNKNOWN",
                    batch_id,
                    "Process-domain applicability is unknown.",
                );
            }
            _ => push_finding(
                &mut violations,
                "UNKNOWN_PROCESS_STATE",
                batch_id,
                "Process state is outside the v0.1 state set.",
            ),
        }
    }

    let mut assignment_by_specimen = BTreeMap::new();
    for (position, assignment) in assignments.iter().enumerate() {
        let owner_id = field_str(assignment, "specimen_id").unwrap_or("partition_assignment");
        let Some(specimen_id) = field_str(assignment, "specimen_id") else {
            continue;
        };
        let Some(partition) = field_str(assignment, "partition") else {
            push_finding(
                &mut violations,
                "MISSING_PARTITION",
                &format!("{owner_id}[{position}]"),
                "Assignment requires train, holdout, or challenge.",
            );
            continue;
        };
        if !matches!(partition, "train" | "holdout" | "challenge") {
            push_finding(
                &mut violations,
                "UNKNOWN_PARTITION",
                specimen_id,
                "Partition is outside train / holdout / challenge.",
            );
            continue;
        }
        if assignment_by_specimen
            .insert(specimen_id.to_owned(), partition.to_owned())
            .is_some()
        {
            push_finding(
                &mut violations,
                "DUPLICATE_SPECIMEN_ASSIGNMENT",
                specimen_id,
                "A specimen must have exactly one partition assignment.",
            );
        }
    }

    for specimen_id in specimen_index.keys() {
        if !assignment_by_specimen.contains_key(specimen_id) {
            push_finding(
                &mut violations,
                "UNASSIGNED_SPECIMEN",
                specimen_id,
                "Every declared specimen must have exactly one partition assignment.",
            );
        }
    }

    let grouping_levels = case
        .pointer("/partition_contract/grouping_levels")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if grouping_levels.is_empty() {
        push_finding(
            &mut violations,
            "EMPTY_GROUPING_CONTRACT",
            "partition_contract",
            "At least one grouping level is required.",
        );
    }
    let mut seen_levels = BTreeSet::new();
    for level in &grouping_levels {
        if !allowed_grouping_level(level) {
            push_finding(
                &mut violations,
                "UNKNOWN_GROUPING_LEVEL",
                level,
                "Grouping level is not supported by the v0.1 contract.",
            );
        }
        if !seen_levels.insert(level.clone()) {
            push_finding(
                &mut violations,
                "DUPLICATE_GROUPING_LEVEL",
                level,
                "Each grouping level must be declared only once.",
            );
        }
    }

    // Build group -> partition memberships from the hierarchy, never from
    // caller-supplied group labels. Test campaigns are derived from linked
    // observations, so multiple observations from one specimen remain grouped.
    let mut group_partitions: BTreeMap<(String, String), String> = BTreeMap::new();
    for (specimen_id, partition) in &assignment_by_specimen {
        let Some(specimen) = specimen_index.get(specimen_id) else {
            continue;
        };
        let panel_id = field_str(specimen, "panel_id");
        let panel = panel_id.and_then(|id| panel_index.get(id).copied());
        let batch_id = panel.and_then(|value| field_str(value, "process_batch_id"));
        let batch = batch_id.and_then(|id| batch_index.get(id).copied());
        let lot_id = batch.and_then(|value| field_str(value, "material_lot_id"));
        let configuration_id = field_str(specimen, "configuration_id");
        let campaign_ids = campaigns_by_specimen.get(specimen_id);

        for level in &grouping_levels {
            let group_ids: Vec<String> = match level.as_str() {
                "material_lot" => lot_id.map(str::to_owned).into_iter().collect(),
                "process_batch" => batch_id.map(str::to_owned).into_iter().collect(),
                "panel" => panel_id.map(str::to_owned).into_iter().collect(),
                "configuration" => configuration_id.map(str::to_owned).into_iter().collect(),
                "test_campaign" => campaign_ids
                    .map(|ids| ids.iter().cloned().collect())
                    .unwrap_or_default(),
                _ => Vec::new(),
            };

            if group_ids.is_empty() && allowed_grouping_level(level) {
                push_finding(
                    &mut violations,
                    "UNRESOLVED_GROUP_MEMBERSHIP",
                    specimen_id,
                    &format!(
                        "Cannot derive declared {level} grouping membership from the lineage graph."
                    ),
                );
            }

            for group_id in group_ids {
                let key = (level.clone(), group_id.clone());
                if let Some(existing_partition) = group_partitions.get(&key) {
                    if existing_partition != partition {
                        push_finding(
                            &mut violations,
                            "SPLIT_GROUP_OVERLAP",
                            &group_id,
                            &format!(
                                "{level} group appears in both {existing_partition} and {partition}."
                            ),
                        );
                    }
                } else {
                    group_partitions.insert(key, partition.clone());
                }
            }
        }
    }

    let mut entity_counts = BTreeMap::new();
    entity_counts.insert("material_lots".to_owned(), lots.len());
    entity_counts.insert("process_batches".to_owned(), batches.len());
    entity_counts.insert("panels".to_owned(), panels.len());
    entity_counts.insert("specimens".to_owned(), specimens.len());
    entity_counts.insert("test_campaigns".to_owned(), campaigns.len());
    entity_counts.insert("configurations".to_owned(), configurations.len());
    entity_counts.insert("observations".to_owned(), observations.len());
    entity_counts.insert("partition_assignments".to_owned(), assignments.len());

    let structurally_complete = violations.is_empty();
    let structural_status = if structurally_complete {
        "STRUCTURALLY_COMPLETE"
    } else {
        "STRUCTURAL_DEFECTS"
    }
    .to_owned();

    let process_domain_status = if batches.is_empty() {
        "INDETERMINATE_NO_PROCESS_BATCHES"
    } else {
        match (out_of_domain_batches > 0, unknown_domain_batches > 0) {
            (true, true) => "MIXED_OUT_OF_DOMAIN_AND_UNKNOWN",
            (true, false) => "PARTIALLY_OUT_OF_DOMAIN",
            (false, true) => "INDETERMINATE",
            (false, false) => "DECLARED_IN_DOMAIN",
        }
    }
    .to_owned();

    sort_findings(&mut violations);
    sort_findings(&mut blockers);

    EvaluationReport {
        evaluator_profile: "aero-material-process-evaluator-v0.1".to_owned(),
        case_id,
        schema_validation_status: "NOT_PERFORMED_BY_THIS_SEMANTIC_EVALUATOR".to_owned(),
        structurally_complete,
        structural_status,
        process_domain_status,
        model_applicability_status: "NOT_EVALUATED_V0_1".to_owned(),
        evidence_sufficiency_status: "NOT_EVALUATED_V0_1".to_owned(),
        decision_status: "NOT_ELIGIBLE_SYNTHETIC_ONLY".to_owned(),
        physical_qualification_status: "OUT_OF_SCOPE_NOT_CLAIMED".to_owned(),
        entity_counts,
        observation_status_counts,
        violations,
        blockers,
        capability_gaps: vec![
            "INDEPENDENT_SPECIMEN_SOURCE_MATERIAL_IDENTITY_NOT_REPRESENTED".to_owned(),
            "INDEPENDENT_SPECIMEN_DESIGN_CONFIGURATION_ASSERTION_NOT_REPRESENTED".to_owned(),
            "CALIBRATION_VALIDITY_INTERVAL_AND_MEASUREMENT_TIMESTAMP_NOT_REPRESENTED".to_owned(),
            "SOURCE_ARTIFACT_DIGEST_AND_BYTE_LENGTH_ARE_OWNED_BY_MYCELIX_PACKET_LAYER".to_owned(),
            "COUPON_TO_ELEMENT_JOINT_SUBCOMPONENT_TRANSFER_EVIDENCE_NOT_REPRESENTED".to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{evaluate_case, field_str};
    use serde_json::Value;

    const FIXTURE: &str = include_str!(
        "../../../../docs/research/aviation/examples/synthetic_material_process_case_v0_1.json"
    );

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("synthetic fixture parses")
    }

    fn has_code(report: &super::EvaluationReport, code: &str) -> bool {
        report.violations.iter().any(|finding| finding.code == code)
            || report.blockers.iter().any(|finding| finding.code == code)
    }

    #[test]
    fn fixture_is_structurally_complete_but_never_decision_eligible() {
        let report = evaluate_case(&fixture());
        assert!(report.structurally_complete, "{:?}", report.violations);
        assert_eq!(report.structural_status, "STRUCTURALLY_COMPLETE");
        assert_eq!(report.process_domain_status, "PARTIALLY_OUT_OF_DOMAIN");
        assert_eq!(report.model_applicability_status, "NOT_EVALUATED_V0_1");
        assert_eq!(report.evidence_sufficiency_status, "NOT_EVALUATED_V0_1");
        assert_eq!(report.decision_status, "NOT_ELIGIBLE_SYNTHETIC_ONLY");
        assert_eq!(
            report.physical_qualification_status,
            "OUT_OF_SCOPE_NOT_CLAIMED"
        );
        assert_eq!(report.entity_counts["specimens"], 7);
        assert_eq!(report.observation_status_counts["observed"], 7);
        assert_eq!(report.observation_status_counts["censored"], 1);
        assert_eq!(report.observation_status_counts["excluded"], 1);
        assert_eq!(report.observation_status_counts["missing"], 1);
        assert!(has_code(&report, "PROCESS_OUT_OF_DOMAIN"));
        assert!(has_code(&report, "CALIBRATION_NOT_DECLARED_ACTIVE"));
        assert_eq!(
            report.schema_validation_status,
            "NOT_PERFORMED_BY_THIS_SEMANTIC_EVALUATOR"
        );
    }

    #[test]
    fn duplicate_specimen_identity_is_rejected() {
        let mut case = fixture();
        case["hierarchy"]["specimens"][1]["id"] = case["hierarchy"]["specimens"][0]["id"].clone();
        let report = evaluate_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_code(&report, "DUPLICATE_ID"));
    }

    #[test]
    fn dangling_material_lot_reference_is_rejected() {
        let mut case = fixture();
        case["hierarchy"]["process_batches"][0]["material_lot_id"] =
            Value::String("LOT-DOES-NOT-EXIST".to_owned());
        let report = evaluate_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_code(&report, "DANGLING_REFERENCE"));
    }

    #[test]
    fn split_overlap_is_derived_from_hierarchy_not_input_group_labels() {
        let mut case = fixture();
        let assignments = case["partition_contract"]["assignments"]
            .as_array_mut()
            .expect("assignments array");
        let assignment = assignments
            .iter_mut()
            .find(|entry| entry["specimen_id"] == "SPEC-C1")
            .expect("SPEC-C1 assignment");
        assignment["partition"] = Value::String("train".to_owned());

        let report = evaluate_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_code(&report, "SPLIT_GROUP_OVERLAP"));
    }

    #[test]
    fn missing_specimen_assignment_is_rejected() {
        let mut case = fixture();
        case["partition_contract"]["assignments"]
            .as_array_mut()
            .expect("assignments array")
            .retain(|entry| entry["specimen_id"] != "SPEC-D1");
        let report = evaluate_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_code(&report, "UNASSIGNED_SPECIMEN"));
    }

    #[test]
    fn non_active_calibration_state_is_a_blocker_not_a_positive_claim() {
        let mut case = fixture();
        case["hierarchy"]["test_campaigns"][0]["calibration_state"] =
            Value::String("unknown".to_owned());
        let report = evaluate_case(&case);
        assert!(report.structurally_complete);
        assert!(has_code(&report, "CALIBRATION_NOT_DECLARED_ACTIVE"));
        assert_eq!(report.decision_status, "NOT_ELIGIBLE_SYNTHETIC_ONLY");
    }

    #[test]
    fn mixed_unknown_and_out_of_domain_process_is_not_collapsed() {
        let mut case = fixture();
        case["hierarchy"]["process_batches"][0]["process_state"] =
            Value::String("unknown".to_owned());
        let report = evaluate_case(&case);
        assert_eq!(
            report.process_domain_status,
            "MIXED_OUT_OF_DOMAIN_AND_UNKNOWN"
        );
        assert!(has_code(&report, "PROCESS_DOMAIN_UNKNOWN"));
        assert!(has_code(&report, "PROCESS_OUT_OF_DOMAIN"));
    }

    #[test]
    fn record_order_permutations_produce_identical_reports() {
        let case = fixture();
        let baseline = evaluate_case(&case);
        let mut permuted = case;
        for path in [
            "/hierarchy/material_lots",
            "/hierarchy/process_batches",
            "/hierarchy/panels",
            "/hierarchy/specimens",
            "/hierarchy/test_campaigns",
            "/hierarchy/configurations",
            "/observations",
            "/partition_contract/assignments",
        ] {
            permuted
                .pointer_mut(path)
                .and_then(Value::as_array_mut)
                .expect("fixture collection")
                .reverse();
        }
        assert_eq!(baseline, evaluate_case(&permuted));
    }

    const SCHEMA: &str = include_str!(
        "../../../../docs/research/aviation/schemas/aero_material_process_case_v0_1.schema.json"
    );
    const MUTATION_ORACLE: &str = include_str!(
        "../../../../docs/research/aviation/examples/adversarial_material_process_mutations_v0_1.json"
    );
    const AERO_BASALT_RESOURCE_MAP: &str = include_str!(
        "../../../../docs/research/aviation/schemas/aero_basalt_resource_map_v0_1.json"
    );

    #[test]
    fn independent_mutation_oracle_matches_expected_dispositions() {
        let corpus: Value = serde_json::from_str(MUTATION_ORACLE).expect("mutation oracle parses");
        assert_eq!(
            corpus.get("synthetic_only").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            corpus
                .get("oracle_authored_independently_of_evaluator")
                .and_then(Value::as_bool),
            Some(true)
        );
        let cases = corpus
            .get("cases")
            .and_then(Value::as_array)
            .expect("mutation cases array");
        assert_eq!(cases.len(), 12);

        for mutation in cases {
            let case_id = field_str(mutation, "case_id").expect("case id");
            let operation = field_str(mutation, "operation").expect("mutation operation");
            let pointer = field_str(mutation, "pointer").expect("mutation pointer");
            let mut case = fixture();

            match operation {
                "set" => {
                    let target = case
                        .pointer_mut(pointer)
                        .unwrap_or_else(|| panic!("{case_id}: mutation pointer exists"));
                    *target = mutation
                        .get("value")
                        .expect("set mutation has value")
                        .clone();
                }
                "append" => {
                    let target = case
                        .pointer_mut(pointer)
                        .and_then(Value::as_array_mut)
                        .unwrap_or_else(|| panic!("{case_id}: append target is an array"));
                    target.push(
                        mutation
                            .get("value")
                            .expect("append mutation has value")
                            .clone(),
                    );
                }
                other => panic!("{case_id}: unsupported mutation operation {other}"),
            }

            let report = evaluate_case(&case);
            assert_eq!(
                report.structurally_complete,
                mutation["expected_structurally_complete"]
                    .as_bool()
                    .expect("expected structural status"),
                "{case_id}: structural disposition"
            );
            assert_eq!(
                report.process_domain_status,
                mutation["expected_process_domain_status"]
                    .as_str()
                    .expect("expected process-domain status"),
                "{case_id}: process-domain disposition"
            );

            let expected_violations: std::collections::BTreeSet<String> =
                mutation["expected_violation_codes"]
                    .as_array()
                    .expect("expected violation codes")
                    .iter()
                    .map(|code| code.as_str().expect("violation code string").to_owned())
                    .collect();
            let actual_violations: std::collections::BTreeSet<String> = report
                .violations
                .iter()
                .map(|finding| finding.code.clone())
                .collect();
            assert_eq!(
                actual_violations, expected_violations,
                "{case_id}: exact violation-code oracle"
            );

            let expected_blockers: std::collections::BTreeSet<String> =
                mutation["expected_blocker_codes"]
                    .as_array()
                    .expect("expected blocker codes")
                    .iter()
                    .map(|code| code.as_str().expect("blocker code string").to_owned())
                    .collect();
            let actual_blockers: std::collections::BTreeSet<String> = report
                .blockers
                .iter()
                .map(|finding| finding.code.clone())
                .collect();
            assert_eq!(
                actual_blockers, expected_blockers,
                "{case_id}: exact blocker-code oracle"
            );
        }
    }

    #[test]
    fn empty_core_collections_fail_closed() {
        let mut case = fixture();
        case["hierarchy"]["material_lots"] = Value::Array(Vec::new());
        case["hierarchy"]["process_batches"] = Value::Array(Vec::new());
        case["hierarchy"]["panels"] = Value::Array(Vec::new());
        case["hierarchy"]["specimens"] = Value::Array(Vec::new());
        case["hierarchy"]["test_campaigns"] = Value::Array(Vec::new());
        case["hierarchy"]["configurations"] = Value::Array(Vec::new());
        case["observations"] = Value::Array(Vec::new());
        case["partition_contract"]["assignments"] = Value::Array(Vec::new());

        let report = evaluate_case(&case);
        assert!(!report.structurally_complete);
        assert_eq!(
            report.process_domain_status,
            "INDETERMINATE_NO_PROCESS_BATCHES"
        );
        assert!(has_code(&report, "EMPTY_REQUIRED_COLLECTION"));
    }

    #[test]
    fn aviation_basalt_resource_map_is_consistent_with_generic_contract() {
        let mapping: Value = serde_json::from_str(AERO_BASALT_RESOURCE_MAP)
            .expect("aviation BASALT resource map parses");
        assert_eq!(
            mapping
                .pointer("/mapping_profile/id")
                .and_then(Value::as_str),
            Some("aero-basalt-resource-map-v0.1")
        );
        assert_eq!(
            mapping
                .pointer("/mapping_profile/synthetic_only")
                .and_then(Value::as_bool),
            Some(true)
        );

        let resource_classes = mapping
            .get("resource_classes")
            .and_then(Value::as_array)
            .expect("resource classes");
        assert!(!resource_classes.is_empty());
        let valid_change_kinds = [
            "source_change",
            "dependency_change",
            "toolchain_change",
            "configuration_change",
            "task_contract_change",
            "environment_change",
            "session_change",
            "other",
        ];
        let mut class_names = std::collections::BTreeSet::new();
        let mut resource_templates = std::collections::BTreeSet::new();

        for resource in resource_classes {
            let class_name = field_str(resource, "class").expect("resource class name");
            let template =
                field_str(resource, "resource_id_template").expect("resource ID template");
            let change_kind =
                field_str(resource, "default_change_kind").expect("BASALT change kind");
            assert!(
                class_names.insert(class_name.to_owned()),
                "duplicate {class_name}"
            );
            assert!(
                template.starts_with("aero:") && template.ends_with(":{immutable_id}"),
                "non-canonical resource template {template}"
            );
            assert!(
                resource_templates.insert(template.to_owned()),
                "duplicate resource ID template {template}"
            );
            assert!(
                valid_change_kinds.contains(&change_kind),
                "unsupported BASALT change kind {change_kind}"
            );
            assert!(
                field_str(resource, "owner").is_some_and(|owner| !owner.trim().is_empty()),
                "{class_name}: owner required"
            );
            assert!(
                field_str(resource, "digest_rule").is_some_and(|rule| !rule.trim().is_empty()),
                "{class_name}: digest rule required"
            );
        }

        let evidence_classes = mapping
            .get("evidence_classes")
            .and_then(Value::as_array)
            .expect("evidence classes");
        assert!(!evidence_classes.is_empty());
        let valid_evidence_kinds = [
            "build",
            "test",
            "typecheck",
            "lint",
            "security",
            "scope",
            "regression",
            "other",
        ];
        let mut check_ids = std::collections::BTreeSet::new();

        for evidence in evidence_classes {
            let check_id = field_str(evidence, "check_id").expect("check id");
            let kind = field_str(evidence, "basalt_evidence_kind").expect("BASALT evidence kind");
            assert!(
                check_id.starts_with("aero."),
                "non-aviation check ID {check_id}"
            );
            assert!(
                check_ids.insert(check_id.to_owned()),
                "duplicate check ID {check_id}"
            );
            assert!(
                valid_evidence_kinds.contains(&kind),
                "unsupported BASALT evidence kind {kind}"
            );

            let expected_resources = evidence
                .get("expected_resource_classes")
                .and_then(Value::as_array)
                .expect("expected resource classes");
            assert!(!expected_resources.is_empty());
            for expected in expected_resources {
                let expected = expected.as_str().expect("resource class string");
                assert!(
                    class_names.contains(expected),
                    "{check_id}: unknown resource class {expected}"
                );
            }
        }
    }

    #[test]
    fn schema_document_is_parseable_and_pins_draft_2020_12() {
        let schema: Value = serde_json::from_str(SCHEMA).expect("schema is valid JSON");
        assert_eq!(
            schema.get("$schema").and_then(Value::as_str),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
    }

    #[test]
    fn unknown_configuration_reference_is_rejected() {
        let mut case = fixture();
        case["hierarchy"]["specimens"][0]["configuration_id"] =
            Value::String("CONFIG-UNKNOWN".to_owned());
        let report = evaluate_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_code(&report, "DANGLING_REFERENCE"));
    }

    #[test]
    fn unknown_partition_is_rejected() {
        let mut case = fixture();
        case["partition_contract"]["assignments"][0]["partition"] =
            Value::String("validation".to_owned());
        let report = evaluate_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_code(&report, "UNKNOWN_PARTITION"));
    }

    #[test]
    fn independent_configuration_assertion_is_reported_as_a_capability_gap() {
        let report = evaluate_case(&fixture());
        assert!(report.capability_gaps.iter().any(|gap| {
            gap == "INDEPENDENT_SPECIMEN_DESIGN_CONFIGURATION_ASSERTION_NOT_REPRESENTED"
        }));
    }

    #[test]
    fn unsupported_independent_lot_identity_is_reported_as_a_capability_gap() {
        let report = evaluate_case(&fixture());
        assert!(
            report.capability_gaps.iter().any(|gap| {
                gap == "INDEPENDENT_SPECIMEN_SOURCE_MATERIAL_IDENTITY_NOT_REPRESENTED"
            })
        );
    }
}
