// SPDX-License-Identifier: AGPL-3.0-or-later
//! Research-only pre-print input-admission contract.
//!
//! This module never executes a robot command, performs a coordinate transform,
//! authenticates an issuer, or mutates an authoritative site model.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SCHEMA_VERSION: &str = "construction-layout-input/0.1";
pub const VALIDATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InputManifest {
    pub schema_version: String,
    pub manifest_id: String,
    pub project_id: String,
    pub source_model: SourceModel,
    pub export: ModelExport,
    pub site_binding: SiteBinding,
    pub coordinate_frame: CoordinateFrame,
    pub inventory: FeatureInventory,
    pub control_validation: ControlValidation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceModel {
    pub model_id: String,
    pub revision: String,
    pub sha256: String,
    pub approval_reference: String,
    pub discipline_revisions: Vec<DisciplineRevision>,
    pub feature_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct DisciplineRevision {
    pub discipline_id: String,
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelExport {
    pub source_model_id: String,
    pub source_model_revision: String,
    pub source_model_sha256: String,
    pub export_id: String,
    pub sha256: String,
    pub exporter_id: String,
    pub exporter_version: String,
    pub exporter_config_sha256: String,
    pub discipline_revisions: Vec<DisciplineRevision>,
    pub feature_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SiteBinding {
    pub site_id: String,
    pub task_scope: String,
    pub work_package_revision: String,
    pub work_zone_id: String,
    pub worksite_epoch_sha256: String,
    pub approved_change_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinateFrame {
    pub source_frame_id: String,
    pub target_frame_id: String,
    pub horizontal_reference: String,
    pub vertical_datum: String,
    pub coordinate_epoch: Option<String>,
    pub units: String,
    pub axis_order: Vec<String>,
    pub handedness: String,
    pub transform_bundle_sha256: String,
    pub transform_resource_sha256s: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureInventory {
    pub expected_feature_ids: Vec<String>,
    pub exported_feature_ids: Vec<String>,
    pub declared_excluded_feature_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlValidation {
    pub survey_result_reference: String,
    pub survey_result_sha256: String,
    pub survey_adjustment_reference: String,
    pub survey_adjustment_sha256: String,
    pub fit_point_ids: Vec<String>,
    pub independent_check_points: Vec<CheckPoint>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckPoint {
    pub point_id: String,
    pub coordinate_frame_id: String,
    pub x_m: f64,
    pub y_m: f64,
    pub residual_x_mm: f64,
    pub residual_y_mm: f64,
    pub u95_mm: f64,
    pub independence_status: String,
    pub independence_assessment_reference: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPolicy {
    pub policy_id: String,
    pub project_id: String,
    pub source_model_id: String,
    pub site_id: String,
    pub task_scope: String,
    pub work_package_revision: String,
    pub work_zone_id: String,
    pub worksite_epoch_sha256: String,
    pub approved_change_sequence: u64,
    pub source_model_revision: String,
    pub source_model_sha256: String,
    pub approved_source_model_reference: String,
    pub expected_exporter_id: String,
    pub expected_exporter_version: String,
    pub expected_exporter_config_sha256: String,
    pub source_frame_id: String,
    pub target_frame_id: String,
    pub horizontal_reference: String,
    pub vertical_datum: String,
    pub coordinate_epoch_required: bool,
    pub coordinate_epoch: Option<String>,
    pub units: String,
    pub axis_order: Vec<String>,
    pub handedness: String,
    pub transform_bundle_sha256: String,
    pub transform_resource_sha256s: Vec<String>,
    pub discipline_revisions: Vec<DisciplineRevision>,
    pub critical_feature_ids: Vec<String>,
    pub minimum_check_points: usize,
    pub maximum_check_points: usize,
    pub minimum_spread_area_m2: f64,
    pub decision_margin_mm: f64,
    pub maximum_u95_to_margin_ratio: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    AdmissibleForNamedLayoutInput,
    Inadmissible,
    Indeterminate,
}

#[derive(Clone, Debug, Serialize)]
pub struct Evaluation {
    pub schema_version: String,
    pub manifest_id: String,
    pub manifest_sha256: String,
    pub policy_id: String,
    pub policy_sha256: String,
    pub validator_version: String,
    pub decision: Decision,
    pub reason_codes: Vec<String>,
    pub check_point_count: usize,
    pub maximum_observed_bound_mm: Option<f64>,
}

#[derive(Default)]
struct Findings {
    hard: BTreeSet<String>,
    unknown: BTreeSet<String>,
}

impl Findings {
    fn hard(&mut self, code: &str) {
        self.hard.insert(code.to_owned());
    }

    fn unknown(&mut self, code: &str) {
        self.unknown.insert(code.to_owned());
    }

    fn decision(&self) -> Decision {
        if !self.hard.is_empty() {
            Decision::Inadmissible
        } else if !self.unknown.is_empty() {
            Decision::Indeterminate
        } else {
            Decision::AdmissibleForNamedLayoutInput
        }
    }

    fn reasons(&self) -> Vec<String> {
        self.hard.union(&self.unknown).cloned().collect()
    }
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn unique_nonempty(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .all(|value| nonempty(value) && seen.insert(value))
}

fn sorted<T: Ord + Clone>(values: &[T]) -> Vec<T> {
    let mut result = values.to_vec();
    result.sort();
    result
}

fn same_set<T: Ord + Clone>(left: &[T], right: &[T]) -> bool {
    sorted(left) == sorted(right)
}

fn check_digest(value: &str, code: &str, findings: &mut Findings) {
    if !valid_sha256(value) {
        findings.unknown(code);
    }
}

fn check_policy_identity(manifest: &InputManifest, policy: &TrustedPolicy, out: &mut Findings) {
    if manifest.project_id != policy.project_id {
        out.hard("project_id_does_not_match_trusted_policy");
    }

    let site = &manifest.site_binding;
    for (actual, expected, code) in [
        (
            site.site_id.as_str(),
            policy.site_id.as_str(),
            "site_id_mismatch",
        ),
        (
            site.task_scope.as_str(),
            policy.task_scope.as_str(),
            "task_scope_mismatch",
        ),
        (
            site.work_package_revision.as_str(),
            policy.work_package_revision.as_str(),
            "work_package_revision_stale_or_mismatched",
        ),
        (
            site.work_zone_id.as_str(),
            policy.work_zone_id.as_str(),
            "work_zone_mismatch",
        ),
        (
            site.worksite_epoch_sha256.as_str(),
            policy.worksite_epoch_sha256.as_str(),
            "worksite_epoch_stale_or_mismatched",
        ),
    ] {
        if actual != expected {
            out.hard(code);
        }
    }

    if site.approved_change_sequence != policy.approved_change_sequence {
        out.hard("approved_change_sequence_stale_or_mismatched");
    }
}

fn check_source_and_export(manifest: &InputManifest, policy: &TrustedPolicy, out: &mut Findings) {
    let source = &manifest.source_model;
    let export = &manifest.export;

    if !nonempty(&source.model_id) {
        out.unknown("source_model_id_missing");
    }
    if source.model_id != policy.source_model_id {
        out.hard("source_model_id_mismatch");
    }
    if source.revision != policy.source_model_revision
        || source.sha256 != policy.source_model_sha256
    {
        out.hard("source_model_revision_or_digest_not_approved");
    }
    if export.source_model_id != source.model_id || export.source_model_sha256 != source.sha256 {
        out.hard("export_source_model_identity_or_digest_mismatch");
    }
    if export.source_model_revision != source.revision {
        out.hard("export_not_derived_from_manifest_source_revision");
    }
    if export.exporter_id != policy.expected_exporter_id {
        out.hard("exporter_identity_not_approved");
    }
    if export.exporter_version != policy.expected_exporter_version {
        out.hard("exporter_version_not_approved");
    }
    if export.exporter_config_sha256 != policy.expected_exporter_config_sha256 {
        out.hard("exporter_configuration_not_approved");
    }
    if source.approval_reference != policy.approved_source_model_reference {
        out.hard("source_model_approval_reference_not_approved");
    }
    if !nonempty(&export.exporter_version) || !nonempty(&export.export_id) {
        out.unknown("exporter_version_or_export_id_missing");
    }
    if !nonempty(&source.approval_reference) {
        out.unknown("source_model_approval_reference_missing");
    }

    check_digest(&source.sha256, "source_model_digest_malformed", out);
    check_digest(
        &export.source_model_sha256,
        "export_source_model_digest_malformed",
        out,
    );
    check_digest(&export.sha256, "export_digest_malformed", out);
    check_digest(
        &export.exporter_config_sha256,
        "exporter_configuration_digest_malformed",
        out,
    );

    if !unique_nonempty(
        &source
            .discipline_revisions
            .iter()
            .map(|v| v.discipline_id.clone())
            .collect::<Vec<_>>(),
    ) || !unique_nonempty(
        &export
            .discipline_revisions
            .iter()
            .map(|v| v.discipline_id.clone())
            .collect::<Vec<_>>(),
    ) || source
        .discipline_revisions
        .iter()
        .any(|v| !nonempty(&v.revision))
        || export
            .discipline_revisions
            .iter()
            .any(|v| !nonempty(&v.revision))
    {
        out.hard("discipline_revision_identifiers_missing_or_duplicated");
    }
    if !same_set(&source.discipline_revisions, &policy.discipline_revisions) {
        out.hard("source_model_discipline_revision_set_mismatch");
    }
    if !same_set(&export.discipline_revisions, &source.discipline_revisions) {
        out.hard("export_discipline_revision_set_differs_from_source");
    }
}

fn check_coordinate_frame(manifest: &InputManifest, policy: &TrustedPolicy, out: &mut Findings) {
    let frame = &manifest.coordinate_frame;
    for (actual, expected, code) in [
        (
            frame.source_frame_id.as_str(),
            policy.source_frame_id.as_str(),
            "source_frame_id_mismatch",
        ),
        (
            frame.target_frame_id.as_str(),
            policy.target_frame_id.as_str(),
            "target_frame_id_mismatch",
        ),
        (
            frame.horizontal_reference.as_str(),
            policy.horizontal_reference.as_str(),
            "horizontal_reference_mismatch",
        ),
        (
            frame.vertical_datum.as_str(),
            policy.vertical_datum.as_str(),
            "vertical_datum_mismatch",
        ),
        (
            frame.units.as_str(),
            policy.units.as_str(),
            "coordinate_units_mismatch",
        ),
        (
            frame.handedness.as_str(),
            policy.handedness.as_str(),
            "coordinate_handedness_mismatch",
        ),
        (
            frame.transform_bundle_sha256.as_str(),
            policy.transform_bundle_sha256.as_str(),
            "transform_bundle_not_approved",
        ),
    ] {
        if actual != expected {
            out.hard(code);
        }
    }

    if frame.axis_order != policy.axis_order {
        out.hard("coordinate_axis_order_mismatch");
    }
    if policy.coordinate_epoch_required
        && frame.coordinate_epoch.as_ref().is_none_or(|v| !nonempty(v))
    {
        out.unknown("required_coordinate_epoch_missing");
    }
    if let Some(expected_epoch) = &policy.coordinate_epoch {
        if frame.coordinate_epoch.as_ref() != Some(expected_epoch) {
            out.hard("coordinate_epoch_mismatch");
        }
    }
    if !same_set(
        &frame.transform_resource_sha256s,
        &policy.transform_resource_sha256s,
    ) {
        out.hard("transform_resource_set_mismatch");
    }
    for digest in std::iter::once(&frame.transform_bundle_sha256)
        .chain(frame.transform_resource_sha256s.iter())
    {
        check_digest(digest, "transform_digest_malformed", out);
    }
}

fn check_inventory(manifest: &InputManifest, policy: &TrustedPolicy, out: &mut Findings) {
    let source_ids = &manifest.source_model.feature_ids;
    let inventory = &manifest.inventory;
    let excluded = &inventory.declared_excluded_feature_ids;

    if !excluded.is_empty() {
        // V0.1 deliberately has no self-authorized feature exclusion mechanism.
        // A future version must bind exclusions to an approved policy and reason.
        out.hard("declared_feature_exclusions_not_supported_in_v0_1");
    }
    if !unique_nonempty(&policy.critical_feature_ids)
        || !unique_nonempty(source_ids)
        || !unique_nonempty(&manifest.export.feature_ids)
        || !unique_nonempty(&inventory.expected_feature_ids)
        || !unique_nonempty(&inventory.exported_feature_ids)
        || !unique_nonempty(excluded)
    {
        out.hard("feature_inventory_contains_empty_or_duplicate_ids");
        return;
    }

    if !same_set(
        &inventory.expected_feature_ids,
        &policy.critical_feature_ids,
    ) {
        out.hard("expected_feature_inventory_not_bound_to_approved_policy");
    }
    if !same_set(source_ids, &inventory.expected_feature_ids) {
        out.hard("source_model_feature_inventory_mismatch");
    }
    if !same_set(
        &manifest.export.feature_ids,
        &inventory.exported_feature_ids,
    ) {
        out.hard("export_feature_inventory_reference_mismatch");
    }

    let expected_export: BTreeSet<&String> = inventory.expected_feature_ids.iter().collect();
    let exported: BTreeSet<&String> = inventory.exported_feature_ids.iter().collect();
    if expected_export != exported {
        out.hard("exported_feature_inventory_missing_or_unexpected");
    }
}

fn maximum_triangle_area(points: &[CheckPoint]) -> f64 {
    let mut maximum = 0.0_f64;
    for (i, a) in points.iter().enumerate() {
        for (j, b) in points.iter().enumerate().skip(i + 1) {
            for c in points.iter().skip(j + 1) {
                let twice_area =
                    (b.x_m - a.x_m) * (c.y_m - a.y_m) - (b.y_m - a.y_m) * (c.x_m - a.x_m);
                maximum = maximum.max(twice_area.abs() * 0.5);
            }
        }
    }
    maximum
}

fn check_control_validation(
    control: &ControlValidation,
    policy: &TrustedPolicy,
    out: &mut Findings,
) -> Option<f64> {
    if !nonempty(&control.survey_result_reference)
        || !nonempty(&control.survey_adjustment_reference)
    {
        out.unknown("survey_evidence_reference_missing");
    }
    if !valid_sha256(&control.survey_result_sha256)
        || !valid_sha256(&control.survey_adjustment_sha256)
    {
        out.unknown("survey_evidence_digest_malformed");
    }
    if control.fit_point_ids.is_empty() || !unique_nonempty(&control.fit_point_ids) {
        out.unknown("fit_point_ids_empty_or_duplicated");
    }

    let points = &control.independent_check_points;
    if points.len() < policy.minimum_check_points {
        out.unknown("insufficient_independent_check_points");
    }
    if points.len() > policy.maximum_check_points {
        out.hard("check_point_count_exceeds_policy_limit");
        // Do not perform an unbounded cubic geometry sweep on attacker-controlled
        // input. The configured policy also has a hard maximum below.
        return None;
    }
    if points.is_empty() {
        out.unknown("independent_check_points_missing");
        return None;
    }

    let fit_ids: BTreeSet<&str> = control.fit_point_ids.iter().map(String::as_str).collect();
    let mut point_ids = BTreeSet::new();
    let mut maximum_bound = 0.0_f64;

    for point in points {
        if !nonempty(&point.point_id) || !point_ids.insert(point.point_id.as_str()) {
            out.hard("independent_check_point_id_empty_or_duplicated");
        }
        if point.coordinate_frame_id != policy.target_frame_id {
            out.hard("independent_check_point_coordinate_frame_mismatch");
        }
        if fit_ids.contains(point.point_id.as_str()) {
            out.hard("fit_point_reused_as_independent_check_point");
        }
        if point.independence_status != "independent_under_profile" {
            if matches!(
                point.independence_status.as_str(),
                "same_evidence_path" | "known_common_mode"
            ) {
                out.hard("independent_check_point_has_common_mode_reference");
            } else {
                out.unknown("independent_check_point_independence_not_established");
            }
        }
        if !nonempty(&point.independence_assessment_reference) {
            out.unknown("independence_assessment_reference_missing");
        }
        if !point.x_m.is_finite() || !point.y_m.is_finite() {
            out.unknown("check_point_coordinate_not_finite");
            continue;
        }
        if !point.u95_mm.is_finite() || point.u95_mm < 0.0 {
            out.unknown("check_point_uncertainty_invalid");
            continue;
        }
        if !point.residual_x_mm.is_finite() || !point.residual_y_mm.is_finite() {
            out.unknown("check_point_residual_not_finite");
            continue;
        }
        let residual_mm = point.residual_x_mm.hypot(point.residual_y_mm);
        if !residual_mm.is_finite() {
            out.unknown("check_point_residual_not_finite");
            continue;
        }
        let bound_mm = residual_mm + point.u95_mm;
        if !bound_mm.is_finite() {
            out.unknown("check_point_uncertainty_bound_overflow");
            continue;
        }
        maximum_bound = maximum_bound.max(bound_mm);
        if point.u95_mm > policy.maximum_u95_to_margin_ratio * policy.decision_margin_mm {
            out.unknown("reference_u95_exceeds_predeclared_fraction_of_margin");
        }
        if bound_mm > policy.decision_margin_mm {
            out.hard("independent_check_point_exceeds_decision_margin");
        }
    }

    let spread = maximum_triangle_area(points);
    if !spread.is_finite() {
        out.unknown("independent_check_point_spread_overflow");
    } else if spread < policy.minimum_spread_area_m2 {
        out.unknown("independent_check_point_geometry_insufficient");
    }
    Some(maximum_bound)
}

/// Evaluate a manifest against a policy supplied by the trusted project/site owner.
/// The manifest never supplies its own policy, decision, or reason codes.
pub fn evaluate_manifest(raw: &str, policy: &TrustedPolicy) -> Result<Evaluation, String> {
    let manifest: InputManifest = serde_json::from_str(raw)
        .map_err(|e| format!("invalid or non-conforming manifest: {e}"))?;

    if !valid_sha256(&policy.source_model_sha256)
        || !valid_sha256(&policy.expected_exporter_config_sha256)
        || !valid_sha256(&policy.worksite_epoch_sha256)
        || !valid_sha256(&policy.transform_bundle_sha256)
        || policy
            .transform_resource_sha256s
            .iter()
            .any(|digest| !valid_sha256(digest))
    {
        return Err("trusted policy contains malformed SHA-256 references".to_owned());
    }
    if !nonempty(&policy.policy_id)
        || !nonempty(&policy.project_id)
        || !nonempty(&policy.source_model_id)
        || !nonempty(&policy.site_id)
        || !nonempty(&policy.task_scope)
        || !nonempty(&policy.work_package_revision)
        || !nonempty(&policy.work_zone_id)
        || !nonempty(&policy.source_model_revision)
        || !nonempty(&policy.approved_source_model_reference)
        || !nonempty(&policy.expected_exporter_id)
        || !nonempty(&policy.expected_exporter_version)
        || !nonempty(&policy.source_frame_id)
        || !nonempty(&policy.target_frame_id)
        || !nonempty(&policy.horizontal_reference)
        || !nonempty(&policy.vertical_datum)
        || !nonempty(&policy.units)
        || !nonempty(&policy.handedness)
        || policy.axis_order.is_empty()
        || !unique_nonempty(&policy.axis_order)
        || policy.discipline_revisions.is_empty()
        || !unique_nonempty(
            &policy
                .discipline_revisions
                .iter()
                .map(|v| v.discipline_id.clone())
                .collect::<Vec<_>>(),
        )
        || policy
            .discipline_revisions
            .iter()
            .any(|v| !nonempty(&v.discipline_id) || !nonempty(&v.revision))
        || policy.critical_feature_ids.is_empty()
        || !unique_nonempty(&policy.critical_feature_ids)
        || !unique_nonempty(&policy.transform_resource_sha256s)
    {
        return Err(
            "trusted policy contains missing or duplicated required identity fields".to_owned(),
        );
    }
    if policy.coordinate_epoch_required
        && policy
            .coordinate_epoch
            .as_ref()
            .is_none_or(|v| !nonempty(v))
    {
        return Err("trusted policy requires a coordinate epoch but does not pin one".to_owned());
    }
    if policy
        .coordinate_epoch
        .as_ref()
        .is_some_and(|v| !nonempty(v))
    {
        return Err("trusted policy contains an empty coordinate epoch".to_owned());
    }
    if policy.minimum_check_points < 3
        || policy.maximum_check_points > 128
        || policy.maximum_check_points < policy.minimum_check_points
        || !policy.minimum_spread_area_m2.is_finite()
        || policy.minimum_spread_area_m2 <= 0.0
        || !policy.decision_margin_mm.is_finite()
        || policy.decision_margin_mm <= 0.0
        || !policy.maximum_u95_to_margin_ratio.is_finite()
        || policy.maximum_u95_to_margin_ratio <= 0.0
        || policy.maximum_u95_to_margin_ratio > 1.0
    {
        return Err("trusted policy contains invalid check-point or uncertainty limits".to_owned());
    }

    let mut findings = Findings::default();
    if manifest.schema_version != SCHEMA_VERSION {
        findings.hard("manifest_schema_version_unsupported");
    }
    if !nonempty(&manifest.manifest_id) {
        findings.unknown("manifest_id_missing");
    }

    check_policy_identity(&manifest, policy, &mut findings);
    check_source_and_export(&manifest, policy, &mut findings);
    check_coordinate_frame(&manifest, policy, &mut findings);
    check_inventory(&manifest, policy, &mut findings);
    let max_bound = check_control_validation(&manifest.control_validation, policy, &mut findings);

    let policy_bytes =
        serde_json::to_vec(policy).map_err(|e| format!("cannot serialize trusted policy: {e}"))?;
    Ok(Evaluation {
        schema_version: "construction-layout-input-evaluation/0.1".to_owned(),
        manifest_id: manifest.manifest_id,
        manifest_sha256: sha256(raw.as_bytes()),
        policy_id: policy.policy_id.clone(),
        policy_sha256: sha256(&policy_bytes),
        validator_version: VALIDATOR_VERSION.to_owned(),
        decision: findings.decision(),
        reason_codes: findings.reasons(),
        check_point_count: manifest.control_validation.independent_check_points.len(),
        maximum_observed_bound_mm: max_bound,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const POLICY: &str =
        include_str!("../../../../research/construction-layout-input-v0-1/policy.synthetic.json");
    const MANIFEST_SCHEMA: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/schema/layout-input-manifest-v0.1.schema.json"
    );
    const POLICY_SCHEMA: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/schema/trusted-policy-v0.1.schema.json"
    );
    const VALID: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/fixtures/valid/matched-input.json"
    );
    const WRONG_AXIS: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/fixtures/invalid/wrong-axis-order.json"
    );
    const STALE_REVISION: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/fixtures/invalid/stale-design-revision.json"
    );
    const REUSED_CHECK: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/fixtures/invalid/fit-point-reused-as-check-point.json"
    );
    const WRONG_FRAME_CHECK: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/fixtures/invalid/checkpoint-wrong-frame.json"
    );
    const FEATURE_EXCLUSION: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/fixtures/invalid/feature-exclusion-self-authorized.json"
    );
    const CHECK_OUTSIDE_MARGIN: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/fixtures/invalid/checkpoint-outside-margin.json"
    );
    const WRONG_VERTICAL_DATUM: &str = include_str!(
        "../../../../research/construction-layout-input-v0-1/fixtures/invalid/wrong-vertical-datum.json"
    );

    fn policy() -> TrustedPolicy {
        serde_json::from_str(POLICY).expect("valid synthetic policy")
    }

    #[test]
    fn published_schemas_and_policy_fixture_are_valid_json() {
        let manifest_schema: Value =
            serde_json::from_str(MANIFEST_SCHEMA).expect("manifest schema JSON");
        let policy_schema: Value =
            serde_json::from_str(POLICY_SCHEMA).expect("trusted policy schema JSON");
        let _: TrustedPolicy = serde_json::from_str(POLICY).expect("trusted policy JSON");
        assert_eq!(
            manifest_schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            policy_schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
    }

    fn assert_schema_covers_instance(schema: &Value, instance: &Value, path: &str) {
        if schema["type"] == "object" {
            let properties = schema["properties"]
                .as_object()
                .unwrap_or_else(|| panic!("{path}: schema object has no properties"));
            let object = instance
                .as_object()
                .unwrap_or_else(|| panic!("{path}: instance is not an object"));
            for required in schema["required"].as_array().into_iter().flatten() {
                let key = required.as_str().expect("required field name");
                assert!(
                    object.contains_key(key),
                    "{path}: missing required field {key}"
                );
            }
            for (key, value) in object {
                let child_schema = properties
                    .get(key)
                    .unwrap_or_else(|| panic!("{path}: field {key} is absent from schema"));
                assert_schema_covers_instance(child_schema, value, &format!("{path}/{key}"));
            }
        } else if schema["type"] == "array" {
            let items = &schema["items"];
            assert!(items.is_object(), "{path}: array schema has no item schema");
            for (index, value) in instance
                .as_array()
                .unwrap_or_else(|| panic!("{path}: instance is not an array"))
                .iter()
                .enumerate()
            {
                assert_schema_covers_instance(items, value, &format!("{path}/{index}"));
            }
        }
    }

    #[test]
    fn published_schemas_cover_every_rust_deserialized_field() {
        let manifest_schema: Value =
            serde_json::from_str(MANIFEST_SCHEMA).expect("manifest schema JSON");
        let policy_schema: Value =
            serde_json::from_str(POLICY_SCHEMA).expect("trusted policy schema JSON");
        let manifest: Value = serde_json::from_str(VALID).expect("valid manifest JSON");
        let policy_value: Value = serde_json::from_str(POLICY).expect("trusted policy JSON");

        assert_schema_covers_instance(&manifest_schema, &manifest, "manifest");
        assert_schema_covers_instance(&policy_schema, &policy_value, "trusted_policy");

        let export_required = manifest_schema["properties"]["export"]["required"]
            .as_array()
            .expect("export required array");
        for field in [
            "source_model_id",
            "source_model_revision",
            "source_model_sha256",
        ] {
            assert!(
                export_required.iter().any(|value| value == field),
                "export schema must require Rust field {field}"
            );
        }
    }

    #[test]
    fn matched_manifest_is_admissible_for_named_input_only() {
        let result = evaluate_manifest(VALID, &policy()).expect("valid fixture");
        assert_eq!(result.decision, Decision::AdmissibleForNamedLayoutInput);
        assert_eq!(result.reason_codes.len(), 0);
        assert_eq!(result.check_point_count, 3);
    }

    #[test]
    fn wrong_axis_order_is_inadmissible() {
        let result = evaluate_manifest(WRONG_AXIS, &policy()).expect("well-formed fixture");
        assert_eq!(result.decision, Decision::Inadmissible);
        assert!(
            result
                .reason_codes
                .contains(&"coordinate_axis_order_mismatch".to_owned())
        );
    }

    #[test]
    fn stale_design_revision_is_inadmissible() {
        let result = evaluate_manifest(STALE_REVISION, &policy()).expect("well-formed fixture");
        assert_eq!(result.decision, Decision::Inadmissible);
        assert!(
            result
                .reason_codes
                .contains(&"source_model_revision_or_digest_not_approved".to_owned())
        );
    }

    #[test]
    fn fitting_point_cannot_be_reused_as_independent_checkpoint() {
        let result = evaluate_manifest(REUSED_CHECK, &policy()).expect("well-formed fixture");
        assert_eq!(result.decision, Decision::Inadmissible);
        assert!(
            result
                .reason_codes
                .contains(&"fit_point_reused_as_independent_check_point".to_owned())
        );
    }

    #[test]
    fn missing_checkpoints_produce_indeterminate_not_admissible() {
        let mut value: Value = serde_json::from_str(VALID).expect("manifest json");
        value["control_validation"]["independent_check_points"] = serde_json::json!([]);
        let result = evaluate_manifest(
            &serde_json::to_string(&value).expect("manifest serializes"),
            &policy(),
        )
        .expect("well-formed manifest");
        assert_eq!(result.decision, Decision::Indeterminate);
    }

    #[test]
    fn uncertainty_wider_than_margin_prevents_admission() {
        let mut value: Value = serde_json::from_str(VALID).expect("manifest json");
        value["control_validation"]["independent_check_points"][0]["u95_mm"] =
            serde_json::json!(0.5);
        let result = evaluate_manifest(
            &serde_json::to_string(&value).expect("manifest serializes"),
            &policy(),
        )
        .expect("well-formed manifest");
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(
            result
                .reason_codes
                .contains(&"reference_u95_exceeds_predeclared_fraction_of_margin".to_owned())
        );
    }

    #[test]
    fn wrong_site_epoch_cannot_be_reused() {
        let mut value: Value = serde_json::from_str(VALID).expect("manifest json");
        value["site_binding"]["worksite_epoch_sha256"] = serde_json::json!("f".repeat(64));
        let result = evaluate_manifest(
            &serde_json::to_string(&value).expect("manifest serializes"),
            &policy(),
        )
        .expect("well-formed manifest");
        assert_eq!(result.decision, Decision::Inadmissible);
        assert!(
            result
                .reason_codes
                .contains(&"worksite_epoch_stale_or_mismatched".to_owned())
        );
    }

    #[test]
    fn author_supplied_decision_is_rejected_as_unknown_field() {
        let mut value: Value = serde_json::from_str(VALID).expect("manifest json");
        value["decision"] = serde_json::json!("admissible_for_named_layout_input");
        assert!(
            evaluate_manifest(
                &serde_json::to_string(&value).expect("manifest serializes"),
                &policy(),
            )
            .is_err()
        );
    }

    #[test]
    fn wrong_vertical_datum_is_inadmissible() {
        let result = evaluate_manifest(WRONG_VERTICAL_DATUM, &policy()).expect("fixture");
        assert_eq!(result.decision, Decision::Inadmissible);
        assert!(
            result
                .reason_codes
                .contains(&"vertical_datum_mismatch".to_owned())
        );
    }

    #[test]
    fn independent_checkpoint_must_use_the_approved_target_frame() {
        let result = evaluate_manifest(WRONG_FRAME_CHECK, &policy()).expect("fixture");
        assert_eq!(result.decision, Decision::Inadmissible);
        assert!(
            result
                .reason_codes
                .contains(&"independent_check_point_coordinate_frame_mismatch".to_owned())
        );
    }

    #[test]
    fn manifest_cannot_exclude_critical_features_without_a_policy_mechanism() {
        let result = evaluate_manifest(FEATURE_EXCLUSION, &policy()).expect("fixture");
        assert_eq!(result.decision, Decision::Inadmissible);
        assert!(
            result
                .reason_codes
                .contains(&"declared_feature_exclusions_not_supported_in_v0_1".to_owned())
        );
    }

    #[test]
    fn independent_checkpoint_outside_margin_is_inadmissible() {
        let result = evaluate_manifest(CHECK_OUTSIDE_MARGIN, &policy()).expect("fixture");
        assert_eq!(result.decision, Decision::Inadmissible);
        assert!(
            result
                .reason_codes
                .contains(&"independent_check_point_exceeds_decision_margin".to_owned())
        );
    }

    #[test]
    fn trusted_policy_cannot_request_unbounded_checkpoint_geometry_work() {
        let mut policy = policy();
        policy.maximum_check_points = 129;
        assert!(evaluate_manifest(VALID, &policy).is_err());
    }

    #[test]
    fn unverified_survey_receipt_is_indeterminate() {
        let mut value: Value = serde_json::from_str(VALID).expect("manifest json");
        value["control_validation"]["survey_result_sha256"] = serde_json::json!("not-a-digest");
        let result = evaluate_manifest(
            &serde_json::to_string(&value).expect("manifest serializes"),
            &policy(),
        )
        .expect("well-formed manifest");
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(
            result
                .reason_codes
                .contains(&"survey_evidence_digest_malformed".to_owned())
        );
    }

    #[test]
    fn no_check_point_count_is_reported_as_an_exact_denominator() {
        let result = evaluate_manifest(VALID, &policy()).expect("valid fixture");
        assert_eq!(result.check_point_count, 3);
        let bound = result.maximum_observed_bound_mm.expect("three checkpoints");
        assert!((bound - 0.2914213562373095).abs() < 1e-12);
    }
}
