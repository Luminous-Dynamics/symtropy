// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub mod metrics;
pub mod residual_uncertainty;
pub mod sampling;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifest {
    schema_version: String,
    replay_id: String,
    dataset: DatasetPolicy,
    split: SplitPolicy,
    candidate_run: CandidateRun,
    candidate_runtime_artifacts: Vec<CandidateArtifact>,
    tolerance_profile: ToleranceProfile,
    sampling_profile: SamplingProfile,
    metric_profile: MetricProfile,
    candidate_outputs: Vec<CandidateOutputArtifact>,
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
    coordinate_frame: CoordinateFrame,
    candidate_inputs: Vec<CandidateArtifact>,
    evaluator_artifacts: Vec<EvaluatorArtifact>,
    surface_strata: Vec<SurfaceStratum>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateRun {
    pipeline_id: String,
    source_commit_sha: String,
    toolchain: String,
    command: Vec<String>,
    executable_artifact_id: String,
    configuration_artifact_id: String,
    model_artifact_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinateFrame {
    frame_id: String,
    crs_id: String,
    axis_order: AxisOrder,
    handedness: Handedness,
    linear_unit: LinearUnit,
    vertical_datum_id: String,
    transform_artifact_id: String,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AxisOrder {
    Xyz,
    EastNorthUp,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Handedness {
    Right,
    Left,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum LinearUnit {
    Metre,
    Millimetre,
    FootInternational,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToleranceProfile {
    profile_id: String,
    requirement_revision: String,
    surface_distance_limit_m: f64,
    decision_policy: DecisionPolicy,
    uncertainty_model_id: Option<String>,
    coverage_probability: Option<f64>,
    coverage_factor: Option<f64>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DecisionPolicy {
    ReportOnly,
    HardLimit,
    StatisticalInterval,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SamplingProfile {
    profile_id: String,
    reference_surface_spacing_m: f64,
    scan_voxel_size_m: f64,
    distance_cutoff_m: f64,
    normalization: NormalizationMode,
    surface_sampling_algorithm: String,
    surface_sampling_seed: u64,
    surface_sampling_max_samples: usize,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum NormalizationMode {
    PartialInputOnly,
    CandidateBoundsOnly,
    ReferenceDerivedFullBounds,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetricProfile {
    suite_id: String,
    version: String,
    scope: MetricScope,
    definition_authority: MetricDefinitionAuthority,
    accuracy_direction: AccuracyDirection,
    sample_strategy: AccuracySampleStrategy,
    correspondence_policy: MetricCorrespondencePolicy,
    surface_buffer_m: f64,
    accuracy_cutoff_m: f64,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MetricScope {
    WallOnly,
    AllElements,
    StructuralElements,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AccuracyDirection {
    ReferenceToCandidate,
    CandidateToReference,
    BidirectionalDiagnostic,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MetricDefinitionAuthority {
    OfficialResultsPage,
    EvaluationFramework2018,
    FinalResultsPaper2021,
    BidirectionalDiagnostic,
    ProjectDefined,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AccuracySampleStrategy {
    ReferenceSurfaceSamples,
    SourceSurfaceVertices,
    SourceSurfaceSamples,
    BothDirectionsDiagnostic,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MetricCorrespondencePolicy {
    EuclideanClosestPointOnTriangle,
    OrthogonalProjectionInsideTriangle,
    UnresolvedUntilReferenceMethodReproduction,
    BidirectionalDiagnostic,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateOutputArtifact {
    artifact_id: String,
    scene_id: String,
    role: CandidateOutputRole,
    relative_path: String,
    sha256: String,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CandidateOutputRole {
    IfcModel,
    GeometryModel,
    ElementGraph,
    CandidateMetrics,
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
    CandidateExecutable,
    CandidateConfiguration,
    ModelWeights,
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
    pub candidate_runtime_verified: usize,
    pub candidate_input_verified: usize,
    pub candidate_output_verified: usize,
    pub evaluator_verified: usize,
}

#[derive(Debug, Serialize)]
pub struct CandidateRuntimeReceipt {
    pub artifact_id: String,
    pub role: String,
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct CandidateOutputReceipt {
    pub artifact_id: String,
    pub scene_id: String,
    pub role: String,
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct ReplayAuditReport {
    pub schema_version: String,
    pub replay_id: String,
    pub dataset_id: String,
    pub pipeline_id: String,
    pub source_commit_sha: String,
    pub tolerance_profile_id: String,
    pub requirement_revision: String,
    pub sampling_profile_id: String,
    pub surface_sampling_algorithm: String,
    pub surface_sampling_seed: u64,
    pub surface_sampling_max_samples: usize,
    pub metric_suite_id: String,
    pub metric_suite_version: String,
    pub toolchain: String,
    pub command: Vec<String>,
    pub executable_sha256: String,
    pub configuration_sha256: String,
    pub model_sha256: Option<String>,
    pub candidate_runtime_artifacts: Vec<CandidateRuntimeReceipt>,
    pub candidate_outputs: Vec<CandidateOutputReceipt>,
    pub tolerance_distance_limit_m: f64,
    pub tolerance_decision_policy: String,
    pub tolerance_uncertainty_model_id: Option<String>,
    pub tolerance_coverage_probability: Option<f64>,
    pub tolerance_coverage_factor: Option<f64>,
    pub reference_surface_spacing_m: f64,
    pub scan_voxel_size_m: f64,
    pub distance_cutoff_m: f64,
    pub normalization_mode: String,
    pub metric_scope: String,
    pub metric_definition_authority: String,
    pub metric_accuracy_direction: String,
    pub metric_accuracy_sample_strategy: String,
    pub metric_correspondence_policy: String,
    pub surface_buffer_m: f64,
    pub accuracy_cutoff_m: f64,
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
    pub coordinate_frame_id: String,
    pub crs_id: String,
    pub axis_order: String,
    pub handedness: String,
    pub linear_unit: String,
    pub linear_unit_to_metre_factor: f64,
    pub vertical_datum_id: String,
    pub transform_artifact_id: String,
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

fn axis_order_label(value: AxisOrder) -> &'static str {
    match value {
        AxisOrder::Xyz => "xyz",
        AxisOrder::EastNorthUp => "east_north_up",
    }
}

fn handedness_label(value: Handedness) -> &'static str {
    match value {
        Handedness::Right => "right",
        Handedness::Left => "left",
    }
}

fn linear_unit_label(value: LinearUnit) -> &'static str {
    match value {
        LinearUnit::Metre => "metre",
        LinearUnit::Millimetre => "millimetre",
        LinearUnit::FootInternational => "foot_international",
    }
}

fn linear_unit_to_metre_factor(value: LinearUnit) -> f64 {
    match value {
        LinearUnit::Metre => 1.0,
        LinearUnit::Millimetre => 0.001,
        LinearUnit::FootInternational => 0.3048,
    }
}

fn candidate_input_role_label(value: CandidateInputRole) -> &'static str {
    match value {
        CandidateInputRole::PointCloudObservation => "point_cloud_observation",
        CandidateInputRole::SensorMetadata => "sensor_metadata",
        CandidateInputRole::PriorAssetModel => "prior_asset_model",
        CandidateInputRole::RegistrationMetadata => "registration_metadata",
        CandidateInputRole::CalibrationArtifact => "calibration_artifact",
        CandidateInputRole::CandidateExecutable => "candidate_executable",
        CandidateInputRole::CandidateConfiguration => "candidate_configuration",
        CandidateInputRole::ModelWeights => "model_weights",
    }
}

fn candidate_output_role_label(value: CandidateOutputRole) -> &'static str {
    match value {
        CandidateOutputRole::IfcModel => "ifc_model",
        CandidateOutputRole::GeometryModel => "geometry_model",
        CandidateOutputRole::ElementGraph => "element_graph",
        CandidateOutputRole::CandidateMetrics => "candidate_metrics",
    }
}

fn decision_policy_label(value: DecisionPolicy) -> &'static str {
    match value {
        DecisionPolicy::ReportOnly => "report_only",
        DecisionPolicy::HardLimit => "hard_limit",
        DecisionPolicy::StatisticalInterval => "statistical_interval",
    }
}

fn normalization_mode_label(value: NormalizationMode) -> &'static str {
    match value {
        NormalizationMode::PartialInputOnly => "partial_input_only",
        NormalizationMode::CandidateBoundsOnly => "candidate_bounds_only",
        NormalizationMode::ReferenceDerivedFullBounds => "reference_derived_full_bounds",
    }
}

fn metric_scope_label(value: MetricScope) -> &'static str {
    match value {
        MetricScope::WallOnly => "wall_only",
        MetricScope::AllElements => "all_elements",
        MetricScope::StructuralElements => "structural_elements",
    }
}

fn accuracy_sample_strategy_label(value: AccuracySampleStrategy) -> &'static str {
    match value {
        AccuracySampleStrategy::ReferenceSurfaceSamples => "reference_surface_samples",
        AccuracySampleStrategy::SourceSurfaceVertices => "source_surface_vertices",
        AccuracySampleStrategy::SourceSurfaceSamples => "source_surface_samples",
        AccuracySampleStrategy::BothDirectionsDiagnostic => "both_directions_diagnostic",
    }
}

fn metric_correspondence_policy_label(value: MetricCorrespondencePolicy) -> &'static str {
    match value {
        MetricCorrespondencePolicy::EuclideanClosestPointOnTriangle => {
            "euclidean_closest_point_on_triangle"
        }
        MetricCorrespondencePolicy::OrthogonalProjectionInsideTriangle => {
            "orthogonal_projection_inside_triangle"
        }
        MetricCorrespondencePolicy::UnresolvedUntilReferenceMethodReproduction => {
            "unresolved_until_reference_method_reproduction"
        }
        MetricCorrespondencePolicy::BidirectionalDiagnostic => "bidirectional_diagnostic",
    }
}

fn metric_definition_authority_label(value: MetricDefinitionAuthority) -> &'static str {
    match value {
        MetricDefinitionAuthority::OfficialResultsPage => "official_results_page",
        MetricDefinitionAuthority::EvaluationFramework2018 => "evaluation_framework2018",
        MetricDefinitionAuthority::FinalResultsPaper2021 => "final_results_paper_2021",
        MetricDefinitionAuthority::BidirectionalDiagnostic => "bidirectional_diagnostic",
        MetricDefinitionAuthority::ProjectDefined => "project_defined",
    }
}

fn accuracy_direction_label(value: AccuracyDirection) -> &'static str {
    match value {
        AccuracyDirection::ReferenceToCandidate => "reference_to_candidate",
        AccuracyDirection::CandidateToReference => "candidate_to_reference",
        AccuracyDirection::BidirectionalDiagnostic => "bidirectional_diagnostic",
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_manifest(manifest: &ReplayManifest) -> Result<(), String> {
    if manifest.schema_version != "0.3" {
        return Err("unsupported schema_version; expected 0.3".to_owned());
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

    let run = &manifest.candidate_run;
    if !valid_identifier(&run.pipeline_id)
        || !valid_identifier(&run.toolchain)
        || !valid_identifier(&run.executable_artifact_id)
        || !valid_identifier(&run.configuration_artifact_id)
    {
        return Err("candidate run identity fields must be non-empty".to_owned());
    }
    if run.source_commit_sha.len() != 40
        || !run
            .source_commit_sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(
            "candidate source_commit_sha must be 40 lowercase hexadecimal characters".to_owned(),
        );
    }
    if run.command.is_empty() || run.command.iter().any(|part| !valid_identifier(part)) {
        return Err("candidate command must contain non-empty arguments".to_owned());
    }

    if !valid_identifier(&manifest.tolerance_profile.profile_id)
        || !valid_identifier(&manifest.tolerance_profile.requirement_revision)
        || !manifest
            .tolerance_profile
            .surface_distance_limit_m
            .is_finite()
        || manifest.tolerance_profile.surface_distance_limit_m <= 0.0
    {
        return Err("tolerance profile requires an ID, revision and finite positive distance limit in metres".to_owned());
    }
    let tolerance = &manifest.tolerance_profile;
    match tolerance.decision_policy {
        DecisionPolicy::StatisticalInterval => {
            let valid_model = tolerance
                .uncertainty_model_id
                .as_deref()
                .is_some_and(valid_identifier);
            let valid_probability = tolerance
                .coverage_probability
                .is_some_and(|value| value.is_finite() && value > 0.0 && value < 1.0);
            let valid_factor = tolerance
                .coverage_factor
                .is_some_and(|value| value.is_finite() && value > 0.0);
            if !valid_model || !valid_probability || !valid_factor {
                return Err(
                    "statistical_interval requires uncertainty_model_id, coverage_probability in (0,1), and a finite positive coverage_factor"
                        .to_owned(),
                );
            }
        }
        DecisionPolicy::ReportOnly | DecisionPolicy::HardLimit => {
            if tolerance.uncertainty_model_id.is_some()
                || tolerance.coverage_probability.is_some()
                || tolerance.coverage_factor.is_some()
            {
                return Err(
                    "uncertainty model, coverage probability and coverage factor are only valid for statistical_interval"
                        .to_owned(),
                );
            }
        }
    }

    if !valid_identifier(&manifest.sampling_profile.profile_id)
        || !manifest
            .sampling_profile
            .reference_surface_spacing_m
            .is_finite()
        || manifest.sampling_profile.reference_surface_spacing_m <= 0.0
        || !manifest.sampling_profile.scan_voxel_size_m.is_finite()
        || manifest.sampling_profile.scan_voxel_size_m <= 0.0
        || !manifest.sampling_profile.distance_cutoff_m.is_finite()
        || manifest.sampling_profile.distance_cutoff_m <= 0.0
    {
        return Err("sampling profile values must be finite and positive in metres".to_owned());
    }
    match manifest.sampling_profile.normalization {
        NormalizationMode::PartialInputOnly | NormalizationMode::CandidateBoundsOnly => {}
        NormalizationMode::ReferenceDerivedFullBounds => {
            return Err(
                "reference-derived full-object bounds are forbidden in candidate normalization"
                    .to_owned(),
            );
        }
    }
    if manifest.sampling_profile.surface_sampling_algorithm != sampling::SAMPLING_ALGORITHM_ID {
        return Err(format!(
            "unsupported surface_sampling_algorithm; expected {}",
            sampling::SAMPLING_ALGORITHM_ID
        ));
    }
    if manifest.sampling_profile.surface_sampling_max_samples == 0
        || manifest.sampling_profile.surface_sampling_max_samples > 10_000_000
    {
        return Err("surface_sampling_max_samples must be between 1 and 10000000".to_owned());
    }
    if !valid_identifier(&manifest.metric_profile.suite_id)
        || !valid_identifier(&manifest.metric_profile.version)
        || !manifest.metric_profile.surface_buffer_m.is_finite()
        || manifest.metric_profile.surface_buffer_m <= 0.0
        || !manifest.metric_profile.accuracy_cutoff_m.is_finite()
        || manifest.metric_profile.accuracy_cutoff_m <= 0.0
    {
        return Err(
            "metric profile must declare a version and finite positive metre thresholds".to_owned(),
        );
    }
    if manifest.metric_profile.suite_id == "isprs_indoor_modelling_wall" {
        let authority_matches_definition = match manifest.metric_profile.definition_authority {
            MetricDefinitionAuthority::OfficialResultsPage => {
                manifest.metric_profile.accuracy_direction
                    == AccuracyDirection::ReferenceToCandidate
                    && manifest.metric_profile.sample_strategy
                        == AccuracySampleStrategy::ReferenceSurfaceSamples
                    && manifest.metric_profile.correspondence_policy
                        == MetricCorrespondencePolicy::EuclideanClosestPointOnTriangle
            }
            MetricDefinitionAuthority::EvaluationFramework2018 => {
                manifest.metric_profile.accuracy_direction
                    == AccuracyDirection::CandidateToReference
                    && manifest.metric_profile.sample_strategy
                        == AccuracySampleStrategy::SourceSurfaceVertices
                    && manifest.metric_profile.correspondence_policy
                        == MetricCorrespondencePolicy::OrthogonalProjectionInsideTriangle
            }
            MetricDefinitionAuthority::FinalResultsPaper2021 => {
                manifest.metric_profile.accuracy_direction
                    == AccuracyDirection::CandidateToReference
                    && manifest.metric_profile.sample_strategy
                        == AccuracySampleStrategy::SourceSurfaceSamples
                    && manifest.metric_profile.correspondence_policy
                        == MetricCorrespondencePolicy::UnresolvedUntilReferenceMethodReproduction
            }
            MetricDefinitionAuthority::BidirectionalDiagnostic => {
                manifest.metric_profile.accuracy_direction
                    == AccuracyDirection::BidirectionalDiagnostic
                    && manifest.metric_profile.sample_strategy
                        == AccuracySampleStrategy::BothDirectionsDiagnostic
                    && manifest.metric_profile.correspondence_policy
                        == MetricCorrespondencePolicy::BidirectionalDiagnostic
            }
            MetricDefinitionAuthority::ProjectDefined => false,
        };
        if manifest.metric_profile.scope != MetricScope::WallOnly || !authority_matches_definition {
            return Err(
                "ISPRS wall metric profile must pair wall_only scope, accuracy direction, sample strategy and correspondence policy with definition_authority"
                    .to_owned(),
            );
        }
    } else if manifest.metric_profile.definition_authority
        != MetricDefinitionAuthority::ProjectDefined
    {
        return Err(
            "non-ISPRS metric suites must use project_defined authority; published benchmark authorities are reserved for their named suites"
                .to_owned(),
        );
    }

    if manifest.candidate_runtime_artifacts.is_empty() {
        return Err("candidate_runtime_artifacts must not be empty".to_owned());
    }
    if manifest.candidate_outputs.is_empty() {
        return Err("candidate_outputs must not be empty".to_owned());
    }

    let mut runtime_ids = BTreeSet::new();
    let mut has_executable = false;
    let mut has_configuration = false;
    let mut has_model = false;
    for artifact in &manifest.candidate_runtime_artifacts {
        if !valid_identifier(&artifact.artifact_id)
            || !digest_is_sha256(&artifact.sha256)
            || !runtime_ids.insert(artifact.artifact_id.as_str())
        {
            return Err(format!(
                "candidate runtime artifact {} has invalid or duplicate identity/digest",
                artifact.artifact_id
            ));
        }
        match artifact.role {
            CandidateInputRole::CandidateExecutable => has_executable = true,
            CandidateInputRole::CandidateConfiguration => has_configuration = true,
            CandidateInputRole::ModelWeights => has_model = true,
            _ => {
                return Err(format!(
                    "candidate runtime artifact {} has a non-runtime role",
                    artifact.artifact_id
                ));
            }
        }
    }
    let executable = manifest
        .candidate_runtime_artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == run.executable_artifact_id);
    if !has_executable
        || executable
            .is_none_or(|artifact| artifact.role != CandidateInputRole::CandidateExecutable)
    {
        return Err("candidate executable_artifact_id must reference a candidate_executable runtime artifact".to_owned());
    }
    let configuration = manifest
        .candidate_runtime_artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == run.configuration_artifact_id);
    if !has_configuration
        || configuration
            .is_none_or(|artifact| artifact.role != CandidateInputRole::CandidateConfiguration)
    {
        return Err("candidate configuration_artifact_id must reference a candidate_configuration runtime artifact".to_owned());
    }
    if let Some(model_id) = &run.model_artifact_id {
        if !has_model
            || !manifest.candidate_runtime_artifacts.iter().any(|artifact| {
                artifact.artifact_id == *model_id
                    && artifact.role == CandidateInputRole::ModelWeights
            })
        {
            return Err(
                "candidate model_artifact_id must reference a model_weights runtime artifact"
                    .to_owned(),
            );
        }
    } else if has_model {
        return Err("unreferenced model_weights runtime artifact is not permitted".to_owned());
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
    for artifact in &manifest.candidate_runtime_artifacts {
        if !candidate_ids.insert(artifact.artifact_id.as_str()) {
            return Err(format!(
                "duplicate candidate artifact_id: {}",
                artifact.artifact_id
            ));
        }
        candidate_digests.insert(artifact.sha256.as_str());
    }
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
        let _frame_labels = (
            axis_order_label(scene.coordinate_frame.axis_order),
            handedness_label(scene.coordinate_frame.handedness),
            linear_unit_label(scene.coordinate_frame.linear_unit),
        );
        if !valid_identifier(&scene.coordinate_frame.frame_id)
            || !valid_identifier(&scene.coordinate_frame.crs_id)
            || !valid_identifier(&scene.coordinate_frame.vertical_datum_id)
        {
            return Err(format!(
                "scene {} coordinate frame requires frame_id, crs_id and vertical_datum_id",
                scene.scene_id
            ));
        }
        if !valid_identifier(&scene.coordinate_frame.transform_artifact_id) {
            return Err(format!(
                "scene {} coordinate frame requires transform_artifact_id",
                scene.scene_id
            ));
        }
        if !scene.candidate_inputs.iter().any(|artifact| {
            artifact.artifact_id == scene.coordinate_frame.transform_artifact_id
                && artifact.role == CandidateInputRole::RegistrationMetadata
        }) {
            return Err(format!(
                "scene {} transform_artifact_id must reference a registration_metadata candidate input",
                scene.scene_id
            ));
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

    let mut output_ids = BTreeSet::new();
    let mut ifc_output_scenes = BTreeSet::new();
    let mut graph_output_scenes = BTreeSet::new();
    for output in &manifest.candidate_outputs {
        if !valid_identifier(&output.artifact_id)
            || !valid_identifier(&output.scene_id)
            || !digest_is_sha256(&output.sha256)
            || !output_ids.insert(output.artifact_id.as_str())
        {
            return Err(format!(
                "candidate output {} has invalid or duplicate identity/digest",
                output.artifact_id
            ));
        }
        if !scene_ids.contains(output.scene_id.as_str()) {
            return Err(format!(
                "candidate output {} references unknown scene {}",
                output.artifact_id, output.scene_id
            ));
        }
        match output.role {
            CandidateOutputRole::IfcModel => {
                ifc_output_scenes.insert(output.scene_id.as_str());
            }
            CandidateOutputRole::ElementGraph => {
                graph_output_scenes.insert(output.scene_id.as_str());
            }
            CandidateOutputRole::GeometryModel | CandidateOutputRole::CandidateMetrics => {}
        }
    }
    for scene in &manifest.scenes {
        if !ifc_output_scenes.contains(scene.scene_id.as_str()) {
            return Err(format!(
                "scene {} requires at least one ifc_model candidate output",
                scene.scene_id
            ));
        }
        if !graph_output_scenes.contains(scene.scene_id.as_str()) {
            return Err(format!(
                "scene {} requires at least one element_graph candidate output",
                scene.scene_id
            ));
        }
    }
    for output in &manifest.candidate_outputs {
        if !candidate_ids.insert(output.artifact_id.as_str()) {
            return Err(format!(
                "candidate output artifact_id collides with another candidate artifact: {}",
                output.artifact_id
            ));
        }
        candidate_digests.insert(output.sha256.as_str());
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
        coordinate_frame_id: scene.coordinate_frame.frame_id.clone(),
        crs_id: scene.coordinate_frame.crs_id.clone(),
        axis_order: axis_order_label(scene.coordinate_frame.axis_order).to_owned(),
        handedness: handedness_label(scene.coordinate_frame.handedness).to_owned(),
        linear_unit: linear_unit_label(scene.coordinate_frame.linear_unit).to_owned(),
        linear_unit_to_metre_factor: linear_unit_to_metre_factor(
            scene.coordinate_frame.linear_unit,
        ),
        vertical_datum_id: scene.coordinate_frame.vertical_datum_id.clone(),
        transform_artifact_id: scene.coordinate_frame.transform_artifact_id.clone(),
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

    let mut candidate_runtime_verified = 0;
    let mut candidate_input_verified = 0;
    let mut candidate_output_verified = 0;
    let mut evaluator_verified = 0;
    let mut candidate_paths = BTreeSet::new();
    let mut evaluator_paths: BTreeMap<&str, &str> = BTreeMap::new();

    for artifact in &manifest.candidate_runtime_artifacts {
        if !candidate_paths.insert(artifact.relative_path.as_str()) {
            return Err(format!(
                "duplicate candidate relative_path: {}",
                artifact.relative_path
            ));
        }
        let path = resolve_file_beneath_root(
            &candidate_root,
            &artifact.relative_path,
            "candidate runtime",
            &artifact.artifact_id,
        )?;
        let actual = sha256_file(&path, "candidate runtime", &artifact.artifact_id)?;
        if actual != artifact.sha256 {
            return Err(format!(
                "candidate runtime artifact {} SHA-256 mismatch: declared {}, actual {}",
                artifact.artifact_id, artifact.sha256, actual
            ));
        }
        candidate_runtime_verified += 1;
    }

    for output in &manifest.candidate_outputs {
        if !candidate_paths.insert(output.relative_path.as_str()) {
            return Err(format!(
                "duplicate candidate relative_path: {}",
                output.relative_path
            ));
        }
        let path = resolve_file_beneath_root(
            &candidate_root,
            &output.relative_path,
            "candidate output",
            &output.artifact_id,
        )?;
        let actual = sha256_file(&path, "candidate output", &output.artifact_id)?;
        if actual != output.sha256 {
            return Err(format!(
                "candidate output {} SHA-256 mismatch: declared {}, actual {}",
                output.artifact_id, output.sha256, actual
            ));
        }
        candidate_output_verified += 1;
    }

    for scene in &manifest.scenes {
        for artifact in &scene.candidate_inputs {
            if !candidate_paths.insert(artifact.relative_path.as_str()) {
                return Err(format!(
                    "duplicate candidate relative_path: {}",
                    artifact.relative_path
                ));
            }
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
            candidate_input_verified += 1;
        }
        for artifact in &scene.evaluator_artifacts {
            if let Some(previous_digest) =
                evaluator_paths.insert(artifact.relative_path.as_str(), artifact.sha256.as_str())
                && previous_digest != artifact.sha256.as_str()
            {
                return Err(format!(
                    "evaluator path {} is declared with conflicting SHA-256 digests",
                    artifact.relative_path
                ));
            }
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

    let candidate_verified =
        candidate_runtime_verified + candidate_input_verified + candidate_output_verified;
    Ok(ArtifactVerificationSummary {
        candidate_verified,
        candidate_runtime_verified,
        candidate_input_verified,
        candidate_output_verified,
        evaluator_verified,
    })
}

pub fn audit_manifest_bytes(bytes: &[u8]) -> Result<ReplayAuditReport, String> {
    let manifest: ReplayManifest =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid manifest JSON: {error}"))?;
    validate_manifest(&manifest)?;

    let candidate_artifact_count: usize = manifest
        .scenes
        .iter()
        .map(|scene| scene.candidate_inputs.len())
        .sum();
    let evaluator_artifact_count: usize = manifest
        .scenes
        .iter()
        .map(|scene| scene.evaluator_artifacts.len())
        .sum();
    let scenes: Vec<SceneVisibilityAudit> = manifest.scenes.iter().map(summarize_scene).collect();
    let digest = Sha256::digest(bytes);
    let manifest_sha256: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();

    let executable_sha256 = manifest
        .candidate_runtime_artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == manifest.candidate_run.executable_artifact_id)
        .map(|artifact| artifact.sha256.clone())
        .ok_or("validated executable artifact disappeared")?;
    let configuration_sha256 = manifest
        .candidate_runtime_artifacts
        .iter()
        .find(|artifact| artifact.artifact_id == manifest.candidate_run.configuration_artifact_id)
        .map(|artifact| artifact.sha256.clone())
        .ok_or("validated configuration artifact disappeared")?;
    let model_sha256 = manifest
        .candidate_run
        .model_artifact_id
        .as_ref()
        .and_then(|model_id| {
            manifest
                .candidate_runtime_artifacts
                .iter()
                .find(|artifact| artifact.artifact_id == *model_id)
        })
        .map(|artifact| artifact.sha256.clone());

    let candidate_runtime_receipts = manifest
        .candidate_runtime_artifacts
        .iter()
        .map(|artifact| CandidateRuntimeReceipt {
            artifact_id: artifact.artifact_id.clone(),
            role: candidate_input_role_label(artifact.role).to_owned(),
            relative_path: artifact.relative_path.clone(),
            sha256: artifact.sha256.clone(),
        })
        .collect();
    let candidate_output_receipts = manifest
        .candidate_outputs
        .iter()
        .map(|artifact| CandidateOutputReceipt {
            artifact_id: artifact.artifact_id.clone(),
            scene_id: artifact.scene_id.clone(),
            role: candidate_output_role_label(artifact.role).to_owned(),
            relative_path: artifact.relative_path.clone(),
            sha256: artifact.sha256.clone(),
        })
        .collect();

    Ok(ReplayAuditReport {
        schema_version: manifest.schema_version,
        replay_id: manifest.replay_id,
        dataset_id: manifest.dataset.dataset_id,
        pipeline_id: manifest.candidate_run.pipeline_id,
        source_commit_sha: manifest.candidate_run.source_commit_sha,
        tolerance_profile_id: manifest.tolerance_profile.profile_id,
        requirement_revision: manifest.tolerance_profile.requirement_revision,
        sampling_profile_id: manifest.sampling_profile.profile_id,
        surface_sampling_algorithm: manifest.sampling_profile.surface_sampling_algorithm,
        surface_sampling_seed: manifest.sampling_profile.surface_sampling_seed,
        surface_sampling_max_samples: manifest.sampling_profile.surface_sampling_max_samples,
        metric_suite_id: manifest.metric_profile.suite_id,
        metric_suite_version: manifest.metric_profile.version,
        toolchain: manifest.candidate_run.toolchain,
        command: manifest.candidate_run.command,
        executable_sha256,
        configuration_sha256,
        model_sha256,
        candidate_runtime_artifacts: candidate_runtime_receipts,
        candidate_outputs: candidate_output_receipts,
        tolerance_distance_limit_m: manifest.tolerance_profile.surface_distance_limit_m,
        tolerance_decision_policy: decision_policy_label(
            manifest.tolerance_profile.decision_policy,
        )
        .to_owned(),
        tolerance_uncertainty_model_id: manifest.tolerance_profile.uncertainty_model_id,
        tolerance_coverage_probability: manifest.tolerance_profile.coverage_probability,
        tolerance_coverage_factor: manifest.tolerance_profile.coverage_factor,
        reference_surface_spacing_m: manifest.sampling_profile.reference_surface_spacing_m,
        scan_voxel_size_m: manifest.sampling_profile.scan_voxel_size_m,
        distance_cutoff_m: manifest.sampling_profile.distance_cutoff_m,
        normalization_mode: normalization_mode_label(manifest.sampling_profile.normalization)
            .to_owned(),
        metric_scope: metric_scope_label(manifest.metric_profile.scope).to_owned(),
        metric_definition_authority: metric_definition_authority_label(
            manifest.metric_profile.definition_authority,
        )
        .to_owned(),
        metric_accuracy_direction: accuracy_direction_label(
            manifest.metric_profile.accuracy_direction,
        )
        .to_owned(),
        metric_accuracy_sample_strategy: accuracy_sample_strategy_label(
            manifest.metric_profile.sample_strategy,
        )
        .to_owned(),
        metric_correspondence_policy: metric_correspondence_policy_label(
            manifest.metric_profile.correspondence_policy,
        )
        .to_owned(),
        surface_buffer_m: manifest.metric_profile.surface_buffer_m,
        accuracy_cutoff_m: manifest.metric_profile.accuracy_cutoff_m,
        manifest_sha256,
        scene_count: manifest.scenes.len(),
        candidate_artifact_count: candidate_artifact_count
            + manifest.candidate_runtime_artifacts.len()
            + manifest.candidate_outputs.len(),
        evaluator_artifact_count,
        scenes,
        claim_ceiling: "METADATA_LOCAL_ARTIFACT_INTEGRITY_AND_VISIBILITY_ACCOUNTING_ONLY; no candidate execution, geometry reconstruction, official ISPRS scoring, OS sandboxing, or proof of organizational evaluator independence",
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
        assert_eq!(summary.candidate_verified, 13);
        assert_eq!(summary.candidate_runtime_verified, 3);
        assert_eq!(summary.candidate_input_verified, 4);
        assert_eq!(summary.candidate_output_verified, 6);
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
        let evaluator_source = synthetic_data_roots().1.join("room-a-reference.bin");
        symlink(&evaluator_source, candidate.join("escape.bin")).expect("create escape symlink");

        let mut value = fixture_value();
        value["candidate_runtime_artifacts"][0]["relative_path"] = serde_json::json!("escape.bin");
        let error = super::verify_manifest_artifact_files(
            value.to_string().as_bytes(),
            &candidate,
            &evaluator,
        )
        .expect_err("symlinked candidate runtime artifacts must be rejected");
        assert!(error.contains("path contains a symlink"));
        let _ = std::fs::remove_dir_all(&temp);
    }
    #[test]
    fn rejects_unknown_surface_sampling_algorithm() {
        let mut value = fixture_value();
        value["sampling_profile"]["surface_sampling_algorithm"] =
            serde_json::json!("unversioned_random_points");
        let error = audit(&value).expect_err("sampler identity must be versioned and closed");
        assert!(error.contains("unsupported surface_sampling_algorithm"));
    }

    #[test]
    fn rejects_zero_or_unbounded_surface_sampling_budget() {
        let mut value = fixture_value();
        value["sampling_profile"]["surface_sampling_max_samples"] = serde_json::json!(0);
        let zero = audit(&value).expect_err("zero sample budget is invalid");
        assert!(zero.contains("surface_sampling_max_samples"));
        value["sampling_profile"]["surface_sampling_max_samples"] = serde_json::json!(10_000_001);
        let excessive = audit(&value).expect_err("sample budget must be bounded");
        assert!(excessive.contains("surface_sampling_max_samples"));
    }

    #[test]
    fn rejects_reference_derived_normalization_bounds() {
        let mut value = fixture_value();
        value["sampling_profile"]["normalization"] =
            serde_json::json!("reference_derived_full_bounds");
        let error = audit(&value).expect_err("reference-derived bounds leak evaluator knowledge");
        assert!(error.contains("reference-derived full-object bounds are forbidden"));
    }

    #[test]
    fn rejects_direction_mismatch_for_official_results_page_profile() {
        let mut value = isprs_pair_fixture();
        value["metric_profile"]["suite_id"] = serde_json::json!("isprs_indoor_modelling_wall");
        value["metric_profile"]["scope"] = serde_json::json!("wall_only");
        value["metric_profile"]["definition_authority"] =
            serde_json::json!("official_results_page");
        value["metric_profile"]["accuracy_direction"] = serde_json::json!("candidate_to_reference");
        let error = audit(&value).expect_err("direction must match selected definition authority");
        assert!(error.contains("definition_authority"));
    }

    #[test]
    fn accepts_published_2021_paper_direction_as_a_separate_named_profile() {
        let mut value = isprs_pair_fixture();
        value["metric_profile"]["suite_id"] = serde_json::json!("isprs_indoor_modelling_wall");
        value["metric_profile"]["scope"] = serde_json::json!("wall_only");
        value["metric_profile"]["definition_authority"] =
            serde_json::json!("final_results_paper2021");
        value["metric_profile"]["accuracy_direction"] = serde_json::json!("candidate_to_reference");
        value["metric_profile"]["sample_strategy"] = serde_json::json!("source_surface_samples");
        value["metric_profile"]["correspondence_policy"] =
            serde_json::json!("unresolved_until_reference_method_reproduction");
        let report =
            audit(&value).expect("paper-specific profile remains explicitly method-unresolved");
        assert_eq!(report.metric_accuracy_direction, "candidate_to_reference");
        assert_eq!(
            report.metric_definition_authority,
            "final_results_paper_2021"
        );
        assert_eq!(
            report.metric_accuracy_sample_strategy,
            "source_surface_samples"
        );
        assert_eq!(
            report.metric_correspondence_policy,
            "unresolved_until_reference_method_reproduction"
        );
    }

    #[test]
    fn accepts_2018_evaluation_framework_vertex_to_reference_definition() {
        let mut value = isprs_pair_fixture();
        value["metric_profile"]["suite_id"] = serde_json::json!("isprs_indoor_modelling_wall");
        value["metric_profile"]["scope"] = serde_json::json!("wall_only");
        value["metric_profile"]["definition_authority"] =
            serde_json::json!("evaluation_framework2018");
        value["metric_profile"]["accuracy_direction"] = serde_json::json!("candidate_to_reference");
        value["metric_profile"]["sample_strategy"] = serde_json::json!("source_surface_vertices");
        value["metric_profile"]["correspondence_policy"] =
            serde_json::json!("orthogonal_projection_inside_triangle");
        let report = audit(&value).expect(
            "2018 framework's source vertices and projection rule must be separately recorded",
        );
        assert_eq!(
            report.metric_accuracy_sample_strategy,
            "source_surface_vertices"
        );
        assert_eq!(
            report.metric_correspondence_policy,
            "orthogonal_projection_inside_triangle"
        );
    }

    #[test]
    fn accepts_bidirectional_isprs_diagnostics_without_mislabeling_them_official() {
        let mut value = isprs_pair_fixture();
        value["metric_profile"]["suite_id"] = serde_json::json!("isprs_indoor_modelling_wall");
        value["metric_profile"]["scope"] = serde_json::json!("wall_only");
        value["metric_profile"]["definition_authority"] =
            serde_json::json!("bidirectional_diagnostic");
        value["metric_profile"]["accuracy_direction"] =
            serde_json::json!("bidirectional_diagnostic");
        value["metric_profile"]["sample_strategy"] =
            serde_json::json!("both_directions_diagnostic");
        value["metric_profile"]["correspondence_policy"] =
            serde_json::json!("bidirectional_diagnostic");
        let report = audit(&value).expect("both directed distances may be retained as diagnostics");
        assert_eq!(report.metric_accuracy_direction, "bidirectional_diagnostic");
        assert_eq!(
            report.metric_accuracy_sample_strategy,
            "both_directions_diagnostic"
        );
        assert_eq!(
            report.metric_correspondence_policy,
            "bidirectional_diagnostic"
        );
    }

    #[test]
    fn accepts_official_isprs_metric_direction_and_scope() {
        let mut value = isprs_pair_fixture();
        value["metric_profile"]["suite_id"] = serde_json::json!("isprs_indoor_modelling_wall");
        value["metric_profile"]["scope"] = serde_json::json!("wall_only");
        value["metric_profile"]["definition_authority"] =
            serde_json::json!("official_results_page");
        value["metric_profile"]["accuracy_direction"] = serde_json::json!("reference_to_candidate");
        let report = audit(&value).expect("official profile direction and wall scope are valid");
        assert_eq!(report.metric_suite_id, "isprs_indoor_modelling_wall");
        assert_eq!(
            report.metric_correspondence_policy,
            "euclidean_closest_point_on_triangle"
        );
    }

    #[test]
    fn rejects_official_results_page_when_plane_projection_policy_is_used() {
        let mut value = isprs_pair_fixture();
        value["metric_profile"]["suite_id"] = serde_json::json!("isprs_indoor_modelling_wall");
        value["metric_profile"]["scope"] = serde_json::json!("wall_only");
        value["metric_profile"]["definition_authority"] =
            serde_json::json!("official_results_page");
        value["metric_profile"]["accuracy_direction"] = serde_json::json!("reference_to_candidate");
        value["metric_profile"]["sample_strategy"] = serde_json::json!("reference_surface_samples");
        value["metric_profile"]["correspondence_policy"] =
            serde_json::json!("orthogonal_projection_inside_triangle");
        let error = audit(&value)
            .expect_err("official closest-surface scoring must not use plane-only correspondence");
        assert!(error.contains("correspondence policy"));
    }

    #[test]
    fn rejects_invalid_candidate_source_commit_identity() {
        let mut value = fixture_value();
        value["candidate_run"]["source_commit_sha"] = serde_json::json!("not-a-commit");
        let error = audit(&value).expect_err("candidate source commit must be pinned");
        assert!(error.contains("40 lowercase hexadecimal"));
    }

    #[test]
    fn rejects_missing_executable_artifact_reference() {
        let mut value = fixture_value();
        value["candidate_run"]["executable_artifact_id"] =
            serde_json::json!("UNDECLARED-EXECUTABLE");
        let error = audit(&value).expect_err("run identity must bind the executable bytes");
        assert!(error.contains("executable_artifact_id must reference"));
    }

    #[test]
    fn rejects_scene_without_declared_registration_transform() {
        let mut value = fixture_value();
        value["scenes"][0]["coordinate_frame"]["transform_artifact_id"] =
            serde_json::json!("MISSING-TRANSFORM");
        let error = audit(&value).expect_err("frame must bind registration evidence");
        assert!(error.contains("transform_artifact_id must reference a registration_metadata"));
    }

    #[test]
    fn reports_millimetre_to_metre_conversion_factor() {
        let mut value = fixture_value();
        value["scenes"][0]["coordinate_frame"]["linear_unit"] = serde_json::json!("millimetre");
        let report = audit(&value).expect("millimetre frame is supported explicitly");
        assert_eq!(report.scenes[0].linear_unit_to_metre_factor, 0.001);
    }

    #[test]
    fn rejects_duplicate_candidate_artifact_paths() {
        let mut value = fixture_value();
        let input_path = value["scenes"][0]["candidate_inputs"][0]["relative_path"].clone();
        value["scenes"][0]["candidate_inputs"][1]["relative_path"] = input_path;
        let (candidate, evaluator) = synthetic_data_roots();
        let error = super::verify_manifest_artifact_files(
            value.to_string().as_bytes(),
            &candidate,
            &evaluator,
        )
        .expect_err("candidate artifact paths must be unambiguous");
        assert!(error.contains("duplicate candidate relative_path"));
    }

    #[test]
    fn rejects_unknown_coordinate_units() {
        let mut value = fixture_value();
        value["scenes"][0]["coordinate_frame"]["linear_unit"] = serde_json::json!("yards");
        let error = audit(&value).expect_err("coordinate units are a closed vocabulary");
        assert!(error.contains("unknown variant"));
    }

    #[test]
    fn rejects_non_positive_tolerance() {
        let mut value = fixture_value();
        value["tolerance_profile"]["surface_distance_limit_m"] = serde_json::json!(0.0);
        let error = audit(&value).expect_err("tolerance must be positive");
        assert!(error.contains("finite positive distance limit"));
    }

    #[test]
    fn rejects_non_isprs_suite_claiming_published_metric_authority() {
        let mut value = fixture_value();
        value["metric_profile"]["definition_authority"] =
            serde_json::json!("official_results_page");
        let error = audit(&value).expect_err("published authority cannot label a synthetic suite");
        assert!(error.contains("non-ISPRS metric suites must use project_defined authority"));
    }

    #[test]
    fn rejects_statistical_interval_without_uncertainty_model_and_coverage() {
        let mut value = fixture_value();
        value["tolerance_profile"]["decision_policy"] = serde_json::json!("statistical_interval");
        let error = audit(&value).expect_err("statistical decisions require explicit assumptions");
        assert!(error.contains("requires uncertainty_model_id"));
    }

    #[test]
    fn accepts_statistical_interval_with_explicit_model_and_coverage() {
        let mut value = fixture_value();
        value["tolerance_profile"]["decision_policy"] = serde_json::json!("statistical_interval");
        value["tolerance_profile"]["uncertainty_model_id"] =
            serde_json::json!("synthetic-gaussian-normal-error-v1");
        value["tolerance_profile"]["coverage_probability"] = serde_json::json!(0.95);
        value["tolerance_profile"]["coverage_factor"] = serde_json::json!(1.96);
        let report =
            audit(&value).expect("statistical profile has explicit uncertainty assumptions");
        assert_eq!(
            report.tolerance_uncertainty_model_id.as_deref(),
            Some("synthetic-gaussian-normal-error-v1")
        );
        assert_eq!(report.tolerance_coverage_probability, Some(0.95));
        assert_eq!(report.tolerance_coverage_factor, Some(1.96));
    }

    #[test]
    fn allows_shared_evaluator_reference_path_with_identical_digest() {
        let mut value = isprs_pair_fixture();
        let shared_path = value["scenes"][0]["evaluator_artifacts"][0]["relative_path"].clone();
        let shared_digest = value["scenes"][0]["evaluator_artifacts"][0]["sha256"].clone();
        value["scenes"][1]["evaluator_artifacts"][0]["relative_path"] = shared_path;
        value["scenes"][1]["evaluator_artifacts"][0]["sha256"] = shared_digest;

        let (candidate, evaluator) = synthetic_data_roots();
        let summary = super::verify_manifest_artifact_files(
            value.to_string().as_bytes(),
            &candidate,
            &evaluator,
        )
        .expect("distinct acquisition records may share one physical reference artifact");
        assert_eq!(summary.evaluator_verified, 2);
    }

    #[test]
    fn rejects_shared_evaluator_reference_path_with_conflicting_digest() {
        let mut value = isprs_pair_fixture();
        let shared_path = value["scenes"][0]["evaluator_artifacts"][0]["relative_path"].clone();
        value["scenes"][1]["evaluator_artifacts"][0]["relative_path"] = shared_path;
        value["scenes"][1]["evaluator_artifacts"][0]["sha256"] =
            serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000");

        let (candidate, evaluator) = synthetic_data_roots();
        let error = super::verify_manifest_artifact_files(
            value.to_string().as_bytes(),
            &candidate,
            &evaluator,
        )
        .expect_err("one evaluator file cannot have multiple declared digests");
        assert!(error.contains("conflicting SHA-256 digests"));
    }

    #[test]
    fn rejects_candidate_output_with_wrong_scene_binding() {
        let mut value = fixture_value();
        value["candidate_outputs"][0]["scene_id"] = serde_json::json!("SCENE-NOT-IN-MANIFEST");
        let error = audit(&value).expect_err("output must bind to a declared scene");
        assert!(error.contains("references unknown scene"));
    }

    #[test]
    fn requires_ifc_output_for_every_scene() {
        let mut value = fixture_value();
        value["candidate_outputs"]
            .as_array_mut()
            .expect("output array")
            .retain(|output| output["scene_id"] != "ROOM-B" || output["role"] != "ifc_model");
        let error = audit(&value).expect_err("every scene needs a candidate IFC output");
        assert!(error.contains("requires at least one ifc_model"));
    }

    #[test]
    fn requires_element_graph_output_for_every_scene() {
        let mut value = fixture_value();
        value["candidate_outputs"]
            .as_array_mut()
            .expect("output array")
            .retain(|output| output["scene_id"] != "ROOM-B" || output["role"] != "element_graph");
        let error = audit(&value).expect_err("every scene needs a candidate element graph");
        assert!(error.contains("requires at least one element_graph"));
    }

    #[test]
    fn rejects_candidate_output_bytes_that_do_not_match_digest() {
        let mut value = fixture_value();
        value["candidate_outputs"][0]["sha256"] =
            serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000");
        let (candidate, evaluator) = synthetic_data_roots();
        let error = super::verify_manifest_artifact_files(
            value.to_string().as_bytes(),
            &candidate,
            &evaluator,
        )
        .expect_err("candidate outputs must be hashed just like input artifacts");
        assert!(error.contains("candidate output ROOM-A-IFC SHA-256 mismatch"));
    }

    #[test]
    fn accepts_synthetic_visibility_manifest() {
        let report = audit_manifest_bytes(FIXTURE.as_bytes()).expect("manifest should pass");
        assert_eq!(report.scene_count, 2);
        assert_eq!(report.candidate_artifact_count, 13);
        assert_eq!(report.evaluator_artifact_count, 2);
        assert_eq!(report.pipeline_id, "synthetic-scan-bim-candidate-v0.2");
        assert_eq!(report.metric_suite_version, "0.3");
        assert_eq!(report.surface_sampling_algorithm, "uniform_area_halton_v1");
        assert_eq!(report.surface_sampling_seed, 42);
        assert_eq!(report.surface_sampling_max_samples, 100_000);
        assert_eq!(report.metric_accuracy_direction, "reference_to_candidate");
        assert_eq!(report.metric_definition_authority, "project_defined");
        assert_eq!(
            report.metric_accuracy_sample_strategy,
            "reference_surface_samples"
        );
        assert_eq!(
            report.metric_correspondence_policy,
            "euclidean_closest_point_on_triangle"
        );
        assert_eq!(report.tolerance_distance_limit_m, 0.05);
        assert_eq!(report.scenes[0].linear_unit, "metre");
        assert_eq!(report.scenes[0].linear_unit_to_metre_factor, 1.0);
        assert_eq!(report.scenes[0].transform_artifact_id, "ROOM-A-TRANSFORM");
        assert_eq!(report.candidate_runtime_artifacts.len(), 3);
        assert_eq!(report.candidate_outputs.len(), 6);
        assert_eq!(
            report.candidate_outputs[0].sha256,
            "1388812a1750215452fc4dea388e113d754231c3bd9e9ff4932917da6cef3698"
        );
        assert_eq!(report.candidate_outputs[0].role, "ifc_model");
        assert_eq!(report.command[0], "scan-bim-candidate");
        assert_eq!(report.executable_sha256.len(), 64);
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
        for output in value["candidate_outputs"]
            .as_array_mut()
            .expect("candidate outputs array")
        {
            let scene_id = output["scene_id"].as_str().expect("scene ID");
            output["scene_id"] = serde_json::json!(match scene_id {
                "ROOM-A" => "TUB1",
                "ROOM-B" => "TUB2",
                other => other,
            });
        }
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
