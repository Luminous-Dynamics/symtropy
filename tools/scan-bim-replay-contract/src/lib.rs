// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifest {
    schema_version: String,
    replay_id: String,
    dataset: DatasetPolicy,
    split: SplitPolicy,
    scenes: Vec<SceneRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetPolicy {
    dataset_id: String,
    source_uri: String,
    use_class: DataUseClass,
    source_data_committed: bool,
    ci_artifacts_include_source_data: bool,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DataUseClass {
    Synthetic,
    ResearchOnlyNonRedistributable,
    OpenRedistributable,
    AccessRestricted,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitPolicy {
    development_group_ids: Vec<String>,
    evaluation_group_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneRecord {
    scene_id: String,
    building_group_id: String,
    capture_group_id: String,
    split_group_id: String,
    candidate_inputs: Vec<CandidateArtifact>,
    evaluator_artifacts: Vec<EvaluatorArtifact>,
    surface_strata: Vec<SurfaceStratum>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateArtifact {
    artifact_id: String,
    role: CandidateInputRole,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluatorArtifact {
    artifact_id: String,
    role: EvaluatorArtifactRole,
    sha256: String,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CandidateInputRole {
    PointCloudObservation,
    SensorMetadata,
    PriorAssetModel,
    RegistrationMetadata,
    CalibrationArtifact,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvaluatorArtifactRole {
    ReferenceGeometry,
    GroundTruthLabels,
    HoldoutAssignment,
    OfficialScoreTable,
    IndependentCheckpoints,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceStratum {
    stratum_id: String,
    area_m2: f64,
    visibility: VisibilityClass,
    observation: ObservationStatus,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum VisibilityClass {
    TheoreticallyVisible,
    SelfOccluded,
    ExternallyOccluded,
    OutsideSensorCoverage,
    Unknown,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ObservationStatus {
    Captured,
    NotCaptured,
    Unknown,
    Contradictory,
}

#[derive(Debug, Serialize)]
pub struct ReplayAuditReport {
    pub schema_version: String,
    pub replay_id: String,
    pub dataset_id: String,
    pub manifest_sha256: String,
    pub scene_count: usize,
    pub candidate_artifact_count: usize,
    pub evaluator_artifact_count: usize,
    pub scenes: Vec<SceneVisibilityAudit>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
pub struct SceneVisibilityAudit {
    pub scene_id: String,
    pub building_group_id: String,
    pub capture_group_id: String,
    pub split_group_id: String,
    pub total_classified_area_m2: f64,
    pub theoretically_visible_area_m2: f64,
    pub captured_visible_area_m2: f64,
    pub not_captured_visible_area_m2: f64,
    pub unresolved_visible_area_m2: f64,
    pub self_occluded_area_m2: f64,
    pub externally_occluded_area_m2: f64,
    pub outside_sensor_coverage_area_m2: f64,
    pub visibility_unknown_area_m2: f64,
    pub contradictory_area_m2: f64,
    pub captured_fraction_of_theoretically_visible_area: Option<f64>,
}

fn digest_is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_manifest(manifest: &ReplayManifest) -> Result<(), String> {
    if manifest.schema_version != "0.1" {
        return Err("unsupported schema_version; expected 0.1".to_owned());
    }
    if !valid_identifier(&manifest.replay_id) {
        return Err("replay_id must be non-empty".to_owned());
    }
    if !valid_identifier(&manifest.dataset.dataset_id)
        || (!manifest.dataset.source_uri.starts_with("https://")
            && !manifest.dataset.source_uri.starts_with("urn:"))
    {
        return Err("dataset_id must be non-empty and source_uri must use https:// or urn:".to_owned());
    }
    if manifest.dataset.source_data_committed {
        return Err("source dataset bytes must not be committed".to_owned());
    }
    if manifest.dataset.ci_artifacts_include_source_data {
        return Err("CI artifacts must not include source dataset bytes".to_owned());
    }
    if manifest.dataset.dataset_id == "ISPRS-INDOOR-MODELLING"
        && manifest.dataset.use_class != DataUseClass::ResearchOnlyNonRedistributable
    {
        return Err("ISPRS replay requires research_only_non_redistributable use class".to_owned());
    }

    let development: BTreeSet<&str> = manifest
        .split
        .development_group_ids
        .iter()
        .map(String::as_str)
        .collect();
    let evaluation: BTreeSet<&str> = manifest
        .split
        .evaluation_group_ids
        .iter()
        .map(String::as_str)
        .collect();

    if development.len() != manifest.split.development_group_ids.len()
        || evaluation.len() != manifest.split.evaluation_group_ids.len()
    {
        return Err("split group identifiers must be unique within each partition".to_owned());
    }
    if development.iter().any(|group| evaluation.contains(group)) {
        return Err("development and evaluation groups must be disjoint".to_owned());
    }

    let mut scene_ids = BTreeSet::new();
    let mut candidate_ids = BTreeSet::new();
    let mut evaluator_ids = BTreeSet::new();
    let mut candidate_digests = BTreeSet::new();
    let mut evaluator_digests = BTreeSet::new();
    let mut building_splits: BTreeMap<&str, &str> = BTreeMap::new();
    let mut capture_splits: BTreeMap<&str, &str> = BTreeMap::new();
    let mut tub1: Option<(&str, &str, &str)> = None;
    let mut tub2: Option<(&str, &str, &str)> = None;

    if manifest.scenes.is_empty() {
        return Err("manifest must contain at least one scene".to_owned());
    }

    for scene in &manifest.scenes {
        if !valid_identifier(&scene.scene_id)
            || !valid_identifier(&scene.building_group_id)
            || !valid_identifier(&scene.capture_group_id)
            || !valid_identifier(&scene.split_group_id)
        {
            return Err("scene and group identifiers must be non-empty".to_owned());
        }
        if !scene_ids.insert(scene.scene_id.as_str()) {
            return Err(format!("duplicate scene_id: {}", scene.scene_id));
        }
        if scene.split_group_id != scene.building_group_id {
            return Err(format!(
                "scene {} must use its building_group_id as split_group_id",
                scene.scene_id
            ));
        }
        let is_development = development.contains(scene.split_group_id.as_str());
        let is_evaluation = evaluation.contains(scene.split_group_id.as_str());
        if is_development == is_evaluation {
            return Err(format!(
                "scene {} must belong to exactly one declared split partition",
                scene.scene_id
            ));
        }
        if let Some(previous) = building_splits.insert(
            scene.building_group_id.as_str(),
            scene.split_group_id.as_str(),
        ) {
            if previous != scene.split_group_id.as_str() {
                return Err(format!(
                    "building {} crosses split groups",
                    scene.building_group_id
                ));
            }
        }
        if let Some(previous) = capture_splits.insert(
            scene.capture_group_id.as_str(),
            scene.split_group_id.as_str(),
        ) {
            if previous != scene.split_group_id.as_str() {
                return Err(format!(
                    "capture lineage {} crosses split groups",
                    scene.capture_group_id
                ));
            }
        }

        if scene.candidate_inputs.is_empty() {
            return Err(format!("scene {} has no candidate inputs", scene.scene_id));
        }
        if scene.evaluator_artifacts.is_empty() {
            return Err(format!("scene {} has no evaluator artifacts", scene.scene_id));
        }
        for artifact in &scene.candidate_inputs {
            let _role = artifact.role;
            if !valid_identifier(&artifact.artifact_id) || !digest_is_sha256(&artifact.sha256) {
                return Err(format!(
                    "candidate artifact in scene {} has invalid ID or SHA-256",
                    scene.scene_id
                ));
            }
            if !candidate_ids.insert(artifact.artifact_id.as_str()) {
                return Err(format!(
                    "duplicate candidate artifact_id: {}",
                    artifact.artifact_id
                ));
            }
            candidate_digests.insert(artifact.sha256.as_str());
        }
        for artifact in &scene.evaluator_artifacts {
            let _role = artifact.role;
            if !valid_identifier(&artifact.artifact_id) || !digest_is_sha256(&artifact.sha256) {
                return Err(format!(
                    "evaluator artifact in scene {} has invalid ID or SHA-256",
                    scene.scene_id
                ));
            }
            if !evaluator_ids.insert(artifact.artifact_id.as_str()) {
                return Err(format!(
                    "duplicate evaluator artifact_id: {}",
                    artifact.artifact_id
                ));
            }
            evaluator_digests.insert(artifact.sha256.as_str());
        }
        if scene.surface_strata.is_empty() {
            return Err(format!("scene {} has no surface visibility strata", scene.scene_id));
        }
        let mut stratum_ids = BTreeSet::new();
        for stratum in &scene.surface_strata {
            if !valid_identifier(&stratum.stratum_id)
                || !stratum_ids.insert(stratum.stratum_id.as_str())
            {
                return Err(format!(
                    "scene {} has empty or duplicate stratum_id",
                    scene.scene_id
                ));
            }
            if !stratum.area_m2.is_finite() || stratum.area_m2 <= 0.0 {
                return Err(format!(
                    "scene {} stratum {} must have finite positive area",
                    scene.scene_id, stratum.stratum_id
                ));
            }
            match stratum.visibility {
                VisibilityClass::TheoreticallyVisible => {}
                VisibilityClass::Unknown => {
                    if stratum.observation != ObservationStatus::Unknown {
                        return Err(format!(
                            "scene {} stratum {} has unknown visibility but non-unknown observation",
                            scene.scene_id, stratum.stratum_id
                        ));
                    }
                }
                VisibilityClass::SelfOccluded
                | VisibilityClass::ExternallyOccluded
                | VisibilityClass::OutsideSensorCoverage => {
                    if stratum.observation == ObservationStatus::Captured {
                        return Err(format!(
                            "scene {} stratum {} is marked captured despite excluded visibility",
                            scene.scene_id, stratum.stratum_id
                        ));
                    }
                }
            }
        }

        if scene.scene_id == "TUB1" {
            tub1 = Some((
                scene.building_group_id.as_str(),
                scene.capture_group_id.as_str(),
                scene.split_group_id.as_str(),
            ));
        } else if scene.scene_id == "TUB2" {
            tub2 = Some((
                scene.building_group_id.as_str(),
                scene.capture_group_id.as_str(),
                scene.split_group_id.as_str(),
            ));
        }
    }

    if candidate_ids.iter().any(|id| evaluator_ids.contains(*id)) {
        return Err("candidate and evaluator artifact identifiers must be disjoint".to_owned());
    }
    if candidate_digests
        .iter()
        .any(|digest| evaluator_digests.contains(*digest))
    {
        return Err("candidate and evaluator artifact SHA-256 digests must be disjoint".to_owned());
    }

    if manifest.dataset.dataset_id == "ISPRS-INDOOR-MODELLING" {
        if let (Some(first), Some(second)) = (tub1, tub2) {
            if first.0 != second.0 || first.2 != second.2 {
                return Err("TUB1 and TUB2 must share building and split groups".to_owned());
            }
            if first.1 == second.1 {
                return Err("TUB1 and TUB2 must retain distinct capture-group identifiers".to_owned());
            }
        } else {
            return Err("ISPRS manifest must include both TUB1 and TUB2".to_owned());
        }
    }

    Ok(())
}

fn summarize_scene(scene: &SceneRecord) -> SceneVisibilityAudit {
    let mut total = 0.0;
    let mut visible = 0.0;
    let mut captured = 0.0;
    let mut missing = 0.0;
    let mut unresolved = 0.0;
    let mut self_occluded = 0.0;
    let mut externally_occluded = 0.0;
    let mut outside = 0.0;
    let mut unknown = 0.0;
    let mut contradictory = 0.0;

    for stratum in &scene.surface_strata {
        let area = stratum.area_m2;
        total += area;
        match stratum.visibility {
            VisibilityClass::TheoreticallyVisible => {
                visible += area;
                match stratum.observation {
                    ObservationStatus::Captured => captured += area,
                    ObservationStatus::NotCaptured => missing += area,
                    ObservationStatus::Unknown => unresolved += area,
                    ObservationStatus::Contradictory => {
                        unresolved += area;
                        contradictory += area;
                    }
                }
            }
            VisibilityClass::SelfOccluded => self_occluded += area,
            VisibilityClass::ExternallyOccluded => externally_occluded += area,
            VisibilityClass::OutsideSensorCoverage => outside += area,
            VisibilityClass::Unknown => unknown += area,
        }
        if stratum.observation == ObservationStatus::Contradictory
            && stratum.visibility != VisibilityClass::TheoreticallyVisible
        {
            contradictory += area;
        }
    }

    SceneVisibilityAudit {
        scene_id: scene.scene_id.clone(),
        building_group_id: scene.building_group_id.clone(),
        capture_group_id: scene.capture_group_id.clone(),
        split_group_id: scene.split_group_id.clone(),
        total_classified_area_m2: total,
        theoretically_visible_area_m2: visible,
        captured_visible_area_m2: captured,
        not_captured_visible_area_m2: missing,
        unresolved_visible_area_m2: unresolved,
        self_occluded_area_m2: self_occluded,
        externally_occluded_area_m2: externally_occluded,
        outside_sensor_coverage_area_m2: outside,
        visibility_unknown_area_m2: unknown,
        contradictory_area_m2: contradictory,
        captured_fraction_of_theoretically_visible_area: (visible > 0.0)
            .then_some(captured / visible),
    }
}

pub fn audit_manifest_bytes(bytes: &[u8]) -> Result<ReplayAuditReport, String> {
    let manifest: ReplayManifest =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid manifest JSON: {error}"))?;
    validate_manifest(&manifest)?;

    let candidate_artifact_count = manifest
        .scenes
        .iter()
        .map(|scene| scene.candidate_inputs.len())
        .sum();
    let evaluator_artifact_count = manifest
        .scenes
        .iter()
        .map(|scene| scene.evaluator_artifacts.len())
        .sum();
    let scenes = manifest.scenes.iter().map(summarize_scene).collect();
    let digest = Sha256::digest(bytes);
    let manifest_sha256 = digest.iter().map(|byte| format!("{byte:02x}")).collect();

    Ok(ReplayAuditReport {
        schema_version: manifest.schema_version,
        replay_id: manifest.replay_id,
        dataset_id: manifest.dataset.dataset_id,
        manifest_sha256,
        scene_count: manifest.scenes.len(),
        candidate_artifact_count,
        evaluator_artifact_count,
        scenes,
        claim_ceiling: "METADATA_AND_VISIBILITY_ACCOUNTING_ONLY; no geometry reconstruction, official ISPRS scoring, OS sandboxing, or proof of organizational evaluator independence",
    })
}

#[cfg(test)]
mod tests {
    use super::{ReplayAuditReport, audit_manifest_bytes};
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../fixtures/synthetic-observability.json");

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture is valid JSON")
    }

    fn audit(value: &Value) -> Result<ReplayAuditReport, String> {
        audit_manifest_bytes(value.to_string().as_bytes())
    }

    #[test]
    fn accepts_synthetic_visibility_manifest() {
        let report = audit_manifest_bytes(FIXTURE.as_bytes()).expect("manifest should pass");
        assert_eq!(report.scene_count, 2);
        assert_eq!(report.candidate_artifact_count, 2);
        assert_eq!(report.evaluator_artifact_count, 2);
    }

    #[test]
    fn occluded_area_is_not_counted_as_visible_miss() {
        let report = audit_manifest_bytes(FIXTURE.as_bytes()).expect("manifest should pass");
        let first = &report.scenes[0];
        assert!((first.theoretically_visible_area_m2 - 10.0).abs() < 1e-9);
        assert!((first.captured_visible_area_m2 - 8.0).abs() < 1e-9);
        assert!((first.not_captured_visible_area_m2 - 2.0).abs() < 1e-9);
        assert!((first.self_occluded_area_m2 - 4.0).abs() < 1e-9);
        assert!((first.externally_occluded_area_m2 - 3.0).abs() < 1e-9);
        assert!((first.visibility_unknown_area_m2 - 1.0).abs() < 1e-9);
        assert!(
            (first
                .captured_fraction_of_theoretically_visible_area
                .expect("visible area is non-zero")
                - 0.8)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn rejects_candidate_evaluator_artifact_id_collision() {
        let mut value = fixture_value();
        let candidate_id = value["scenes"][0]["candidate_inputs"][0]["artifact_id"].clone();
        value["scenes"][0]["evaluator_artifacts"][0]["artifact_id"] = candidate_id;
        let error = audit(&value).expect_err("artifact namespaces must be disjoint");
        assert!(error.contains("identifiers must be disjoint"));
    }

    #[test]
    fn rejects_candidate_evaluator_digest_collision() {
        let mut value = fixture_value();
        let candidate_digest = value["scenes"][0]["candidate_inputs"][0]["sha256"].clone();
        value["scenes"][0]["evaluator_artifacts"][0]["sha256"] = candidate_digest;
        let error = audit(&value).expect_err("artifact bytes must not be shared");
        assert!(error.contains("SHA-256 digests must be disjoint"));
    }

    #[test]
    fn rejects_building_group_split_leakage() {
        let mut value = fixture_value();
        value["split"]["development_group_ids"] = serde_json::json!(["BUILDING-A"]);
        value["split"]["evaluation_group_ids"] = serde_json::json!(["BUILDING-A"]);
        let error = audit(&value).expect_err("split overlap must fail");
        assert!(error.contains("must be disjoint"));
    }

    #[test]
    fn rejects_unknown_visibility_mislabeled_as_captured() {
        let mut value = fixture_value();
        value["scenes"][0]["surface_strata"][4]["observation"] =
            serde_json::json!("captured");
        let error = audit(&value).expect_err("unknown visibility cannot be reported captured");
        assert!(error.contains("unknown visibility"));
    }

    #[test]
    fn rejects_unknown_fields() {
        let mut value = fixture_value();
        value["unexpected_reference_bounds"] = serde_json::json!([0.0, 1.0]);
        let error = audit(&value).expect_err("unknown manifest fields must fail closed");
        assert!(error.contains("unknown field"));
    }

    #[test]
    fn rejects_isprs_without_research_only_rights() {
        let mut value = fixture_value();
        value["dataset"]["dataset_id"] = serde_json::json!("ISPRS-INDOOR-MODELLING");
        value["dataset"]["use_class"] = serde_json::json!("open_redistributable");
        let error = audit(&value).expect_err("ISPRS rights and grouping must be checked");
        assert!(error.contains("research_only_non_redistributable"));
    }
}
