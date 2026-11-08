// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

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
    relative_path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluatorArtifact {
    artifact_id: String,
    role: EvaluatorArtifactRole,
    relative_path: String,
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
pub struct ArtifactVerificationSummary {
    pub candidate_verified: usize,
    pub evaluator_verified: usize,
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
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
        return Err(
            "dataset_id must be non-empty and source_uri must use https:// or urn:".to_owned(),
        );
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
        ) && previous != scene.split_group_id.as_str()
        {
            return Err(format!(
                "building {} crosses split groups",
                scene.building_group_id
            ));
        }
        if let Some(previous) = capture_splits.insert(
            scene.capture_group_id.as_str(),
            scene.split_group_id.as_str(),
        ) && previous != scene.split_group_id.as_str()
        {
            return Err(format!(
                "capture lineage {} crosses split groups",
                scene.capture_group_id
            ));
        }

        if scene.candidate_inputs.is_empty() {
            return Err(format!("scene {} has no candidate inputs", scene.scene_id));
        }
        if scene.evaluator_artifacts.is_empty() {
            return Err(format!(
                "scene {} has no evaluator artifacts",
                scene.scene_id
            ));
        }
        let has_point_cloud_observation = scene
            .candidate_inputs
            .iter()
            .any(|artifact| artifact.role == CandidateInputRole::PointCloudObservation);
        if !has_point_cloud_observation {
            return Err(format!(
                "scene {} requires at least one point_cloud_observation candidate input",
                scene.scene_id
            ));
        }
        for artifact in &scene.candidate_inputs {
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
        let has_reference_geometry = scene
            .evaluator_artifacts
            .iter()
            .any(|artifact| artifact.role == EvaluatorArtifactRole::ReferenceGeometry);
        if !has_reference_geometry {
            return Err(format!(
                "scene {} requires at least one reference_geometry evaluator artifact",
                scene.scene_id
            ));
        }
        for artifact in &scene.evaluator_artifacts {
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
            return Err(format!(
                "scene {} has no surface visibility strata",
                scene.scene_id
            ));
        }
        let mut stratum_ids = BTreeSet::new();
        let mut total_surface_area = 0.0_f64;
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
            total_surface_area += stratum.area_m2;
            if !total_surface_area.is_finite() {
                return Err(format!(
                    "scene {} classified surface area overflows f64",
                    scene.scene_id
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
                return Err(
                    "TUB1 and TUB2 must retain distinct capture-group identifiers".to_owned(),
                );
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

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("{label} root cannot be canonicalized: {error}"))?;
    if !canonical.is_dir() {
        return Err(format!("{label} root is not a directory"));
    }
    Ok(canonical)
}

fn resolve_file_beneath_root(
    root: &Path,
    relative_path: &str,
    label: &str,
    artifact_id: &str,
) -> Result<PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{label} artifact {artifact_id} has an unsafe relative_path"
        ));
    }

    let components: Vec<_> = relative.components().collect();
    let mut current = root.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(format!(
                "{label} artifact {artifact_id} has an unsafe path component"
            ));
        };
        current.push(name);
        let metadata = fs::symlink_metadata(&current).map_err(|error| {
            format!("{label} artifact {artifact_id} path component is unavailable: {error}")
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "{label} artifact {artifact_id} path contains a symlink"
            ));
        }
        if index + 1 < components.len() && !metadata.is_dir() {
            return Err(format!(
                "{label} artifact {artifact_id} has a non-directory path component"
            ));
        }
    }

    let canonical = fs::canonicalize(&current).map_err(|error| {
        format!("{label} artifact {artifact_id} cannot be canonicalized: {error}")
    })?;
    if !canonical.starts_with(root) {
        return Err(format!(
            "{label} artifact {artifact_id} resolves outside its declared root"
        ));
    }
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("{label} artifact {artifact_id} metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err(format!(
            "{label} artifact {artifact_id} is not a regular file"
        ));
    }
    Ok(canonical)
}

fn sha256_file(path: &Path, label: &str, artifact_id: &str) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("{label} artifact {artifact_id} cannot be opened: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|error| format!("{label} artifact {artifact_id} read failed: {error}"))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let digest = hasher.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Recomputes declared artifact digests from local files beneath separate roots.
///
/// This is integrity verification only. It does not sandbox a candidate process or
/// prove independent evaluator custody. Standard-library path checks reject observed
/// symlinks and path traversal, but this is not a race-free OS-level openat/O_NOFOLLOW
/// sandbox against a concurrently malicious process.
pub fn verify_manifest_artifact_files(
    bytes: &[u8],
    candidate_root: &Path,
    evaluator_root: &Path,
) -> Result<ArtifactVerificationSummary, String> {
    let manifest: ReplayManifest =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid manifest JSON: {error}"))?;
    validate_manifest(&manifest)?;

    let candidate_root = canonical_directory(candidate_root, "candidate")?;
    let evaluator_root = canonical_directory(evaluator_root, "evaluator")?;
    if candidate_root == evaluator_root
        || candidate_root.starts_with(&evaluator_root)
        || evaluator_root.starts_with(&candidate_root)
    {
        return Err(
            "candidate and evaluator roots must be distinct and non-overlapping".to_owned(),
        );
    }

    let mut candidate_verified = 0;
    let mut evaluator_verified = 0;
    for scene in &manifest.scenes {
        for artifact in &scene.candidate_inputs {
            let path = resolve_file_beneath_root(
                &candidate_root,
                &artifact.relative_path,
                "candidate",
                &artifact.artifact_id,
            )?;
            let actual = sha256_file(&path, "candidate", &artifact.artifact_id)?;
            if actual != artifact.sha256 {
                return Err(format!(
                    "candidate artifact {} SHA-256 mismatch: declared {}, actual {}",
                    artifact.artifact_id, artifact.sha256, actual
                ));
            }
            candidate_verified += 1;
        }
        for artifact in &scene.evaluator_artifacts {
            let path = resolve_file_beneath_root(
                &evaluator_root,
                &artifact.relative_path,
                "evaluator",
                &artifact.artifact_id,
            )?;
            let actual = sha256_file(&path, "evaluator", &artifact.artifact_id)?;
            if actual != artifact.sha256 {
                return Err(format!(
                    "evaluator artifact {} SHA-256 mismatch: declared {}, actual {}",
                    artifact.artifact_id, artifact.sha256, actual
                ));
            }
            evaluator_verified += 1;
        }
    }

    Ok(ArtifactVerificationSummary {
        candidate_verified,
        evaluator_verified,
    })
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

    fn synthetic_data_roots() -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("data");
        (root.join("candidate"), root.join("evaluator"))
    }

    #[test]
    fn verifies_exact_candidate_and_evaluator_file_bytes() {
        let (candidate, evaluator) = synthetic_data_roots();
        let summary =
            super::verify_manifest_artifact_files(FIXTURE.as_bytes(), &candidate, &evaluator)
                .expect("synthetic artifact bytes should match the declared hashes");
        assert_eq!(summary.candidate_verified, 2);
        assert_eq!(summary.evaluator_verified, 2);
    }

    #[test]
    fn rejects_artifact_path_traversal() {
        let mut value = fixture_value();
        value["scenes"][0]["candidate_inputs"][0]["relative_path"] =
            serde_json::json!("../evaluator/room-a-reference.bin");
        let (candidate, evaluator) = synthetic_data_roots();
        let error = super::verify_manifest_artifact_files(
            value.to_string().as_bytes(),
            &candidate,
            &evaluator,
        )
        .expect_err("parent traversal must be rejected");
        assert!(error.contains("unsafe relative_path"));
    }

    #[test]
    fn rejects_artifact_digest_mismatch_against_file_bytes() {
        let mut value = fixture_value();
        value["scenes"][0]["candidate_inputs"][0]["sha256"] =
            serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000");
        let (candidate, evaluator) = synthetic_data_roots();
        let error = super::verify_manifest_artifact_files(
            value.to_string().as_bytes(),
            &candidate,
            &evaluator,
        )
        .expect_err("a syntactically valid but false digest must be rejected");
        assert!(error.contains("SHA-256 mismatch"));
    }

    #[test]
    fn rejects_candidate_and_evaluator_roots_that_overlap() {
        let (candidate, _) = synthetic_data_roots();
        let error =
            super::verify_manifest_artifact_files(FIXTURE.as_bytes(), &candidate, &candidate)
                .expect_err("candidate and evaluator roots must not alias");
        assert!(error.contains("distinct and non-overlapping"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_candidate_artifact_path() {
        use std::os::unix::fs::symlink;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temp = std::env::temp_dir().join(format!(
            "scan-bim-replay-symlink-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let candidate = temp.join("candidate");
        let evaluator = temp.join("evaluator");
        std::fs::create_dir_all(&candidate).expect("candidate root");
        std::fs::create_dir_all(&evaluator).expect("evaluator root");
        std::fs::copy(
            synthetic_data_roots().0.join("room-a-scan.bin"),
            candidate.join("room-a-scan.bin"),
        )
        .expect("copy candidate artifact");
        let evaluator_source = synthetic_data_roots().1.join("room-a-reference.bin");
        std::fs::copy(&evaluator_source, evaluator.join("room-a-reference.bin"))
            .expect("copy evaluator artifact");
        symlink(&evaluator_source, candidate.join("escape.bin")).expect("create escape symlink");

        let mut value = fixture_value();
        value["scenes"][0]["candidate_inputs"][0]["relative_path"] =
            serde_json::json!("escape.bin");
        let error = super::verify_manifest_artifact_files(
            value.to_string().as_bytes(),
            &candidate,
            &evaluator,
        )
        .expect_err("symlinked candidate artifacts must be rejected");
        assert!(error.contains("path contains a symlink"));
        let _ = std::fs::remove_dir_all(&temp);
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
    fn rejects_committed_source_dataset_bytes() {
        let mut value = fixture_value();
        value["dataset"]["source_data_committed"] = serde_json::json!(true);
        let error = audit(&value).expect_err("source bytes must stay outside the repository");
        assert!(error.contains("source dataset bytes must not be committed"));
    }

    #[test]
    fn rejects_source_dataset_bytes_in_ci_artifacts() {
        let mut value = fixture_value();
        value["dataset"]["ci_artifacts_include_source_data"] = serde_json::json!(true);
        let error = audit(&value).expect_err("CI artifacts must not contain source dataset bytes");
        assert!(error.contains("CI artifacts must not include source dataset bytes"));
    }

    #[test]
    fn rejects_reference_derived_bounds_in_candidate_inputs() {
        let mut value = fixture_value();
        value["scenes"][0]["candidate_inputs"][0]["full_object_bounds"] =
            serde_json::json!([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        let error =
            audit(&value).expect_err("candidate input schema must reject target-derived bounds");
        assert!(error.contains("unknown field"));
    }

    #[test]
    fn rejects_unsupported_source_uri_scheme() {
        let mut value = fixture_value();
        value["dataset"]["source_uri"] = serde_json::json!("file:///restricted/scan.ply");
        let error = audit(&value).expect_err("manifest cannot embed local source paths");
        assert!(error.contains("source_uri must use https:// or urn:"));
    }

    #[test]
    fn rejects_duplicate_scene_ids() {
        let mut value = fixture_value();
        value["scenes"][1]["scene_id"] = serde_json::json!("ROOM-A");
        let error = audit(&value).expect_err("scene identifiers must be unique");
        assert!(error.contains("duplicate scene_id"));
    }

    #[test]
    fn rejects_uppercase_sha256_encoding() {
        let mut value = fixture_value();
        value["scenes"][0]["candidate_inputs"][0]["sha256"] = serde_json::json!("A".repeat(64));
        let error = audit(&value).expect_err("SHA-256 must use canonical lowercase hex");
        assert!(error.contains("invalid ID or SHA-256"));
    }

    #[test]
    fn requires_candidate_point_cloud_observation() {
        let mut value = fixture_value();
        value["scenes"][0]["candidate_inputs"][0]["role"] = serde_json::json!("sensor_metadata");
        let error = audit(&value).expect_err("candidate scene needs a point-cloud observation");
        assert!(error.contains("point_cloud_observation"));
    }

    #[test]
    fn requires_evaluator_reference_geometry() {
        let mut value = fixture_value();
        value["scenes"][0]["evaluator_artifacts"][0]["role"] =
            serde_json::json!("ground_truth_labels");
        let error = audit(&value).expect_err("scene needs evaluator reference geometry");
        assert!(error.contains("reference_geometry"));
    }

    #[test]
    fn rejects_overflowing_surface_area_total() {
        let mut value = fixture_value();
        value["scenes"][0]["surface_strata"][0]["area_m2"] = serde_json::json!(f64::MAX);
        value["scenes"][0]["surface_strata"][1]["area_m2"] = serde_json::json!(f64::MAX);
        let error = audit(&value).expect_err("finite individual areas can still overflow in sum");
        assert!(error.contains("overflows f64"));
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
        value["scenes"][0]["surface_strata"][4]["observation"] = serde_json::json!("captured");
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
    fn rejects_capture_lineage_crossing_split_groups() {
        let mut value = fixture_value();
        let capture_id = value["scenes"][0]["capture_group_id"].clone();
        value["scenes"][1]["capture_group_id"] = capture_id;
        let error = audit(&value).expect_err("capture lineage cannot cross splits");
        assert!(error.contains("capture lineage"));
    }

    fn isprs_pair_fixture() -> Value {
        let mut value = fixture_value();
        value["dataset"]["dataset_id"] = serde_json::json!("ISPRS-INDOOR-MODELLING");
        value["dataset"]["use_class"] = serde_json::json!("research_only_non_redistributable");
        value["split"]["evaluation_group_ids"] = serde_json::json!(["TUB-BUILDING"]);
        for (index, scene_id, capture_id) in
            [(0, "TUB1", "TUB1-CAPTURE"), (1, "TUB2", "TUB2-CAPTURE")]
        {
            value["scenes"][index]["scene_id"] = serde_json::json!(scene_id);
            value["scenes"][index]["building_group_id"] = serde_json::json!("TUB-BUILDING");
            value["scenes"][index]["split_group_id"] = serde_json::json!("TUB-BUILDING");
            value["scenes"][index]["capture_group_id"] = serde_json::json!(capture_id);
        }
        value
    }

    #[test]
    fn accepts_tub1_tub2_as_distinct_captures_of_one_building() {
        let value = isprs_pair_fixture();
        let report = audit(&value).expect("TUB1 and TUB2 should share one building group");
        assert_eq!(report.scene_count, 2);
    }

    #[test]
    fn rejects_tub1_tub2_as_different_building_groups() {
        let mut value = isprs_pair_fixture();
        value["scenes"][1]["building_group_id"] = serde_json::json!("OTHER-BUILDING");
        value["scenes"][1]["split_group_id"] = serde_json::json!("OTHER-BUILDING");
        value["split"]["evaluation_group_ids"] =
            serde_json::json!(["TUB-BUILDING", "OTHER-BUILDING"]);
        let error = audit(&value).expect_err("TUB scenes are one physical building");
        assert!(error.contains("TUB1 and TUB2 must share"));
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
