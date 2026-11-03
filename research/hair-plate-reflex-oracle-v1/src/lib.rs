// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-qualified hair-plate inventory and blind held-out reflex evaluation.
//!
//! The contract deliberately distinguishes observed evidence, explicit absence
//! within a declared search scope, and unknown evidence. Unknown is never
//! converted into a negative label.
//!
//! This crate validates/evaluates supplied artifacts. It does not fetch,
//! authenticate, interpret or establish the biological truth of source data.

#![forbid(unsafe_code)]

const MAX_RECORDS: usize = 1024;
const MAX_POINTS: usize = 16_384;
const MAX_NEURONS_PER_PLATE: usize = 4096;
const MAX_TARGETS_PER_PLATE: usize = 65_536;
const MAX_TRAIN_PLATES: usize = 1024;
const MAX_SOURCE_ARTIFACTS: usize = 16_384;
const MAX_ID_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Leg {
    Front,
    Middle,
    Hind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BodySide {
    Left,
    Right,
    Midline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Segment {
    Thorax,
    Coxa,
    Trochanter,
    Femur,
    Tibia,
    Tarsus,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Joint {
    ThoraxCoxa,
    CoxaTrochanter,
    TrochanterFemur,
    FemurTibia,
    TibiaTarsus,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceUnknownReason {
    NotMeasured,
    NotReconstructed,
    RegistrationUncertain,
    AnnotationPending,
    SourceUnavailable,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Evidence<T> {
    Observed {
        value: T,
        source_sha256: String,
    },
    /// Absence is asserted only within this explicit, digest-bound search scope.
    ExplicitlyAbsent {
        search_receipt_sha256: String,
        scope_id: String,
    },
    /// Unknown is missing/ambiguous evidence, not a negative observation.
    Unknown {
        reason: EvidenceUnknownReason,
        detail: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynapseSign {
    Excitatory,
    Inhibitory,
    Modulatory,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MovementDirection {
    Anterior,
    Posterior,
    Dorsal,
    Ventral,
    Flexion,
    Extension,
    Abduction,
    Adduction,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionKind {
    OptogeneticActivation,
    OptogeneticSilencing,
    GeneticActivation,
    GeneticSilencing,
    MechanicalReplay,
    PassiveKinematicReplay,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Morphology {
    pub hair_count: u16,
    pub contact_surface_normal: [f64; 3],
    pub orientation_axis: [f64; 3],
    pub geometry_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TuningPoint {
    pub joint_angle_rad: f64,
    pub normalized_response: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensoryTuning {
    /// Measured joint angle at the preregistered response threshold.
    pub joint_limit_angle_rad: f64,
    pub threshold_definition_id: String,
    pub response_points: Vec<TuningPoint>,
    pub response_unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectomeOutput {
    pub target_neuron_id: String,
    pub target_cell_type: String,
    pub synapse_count: u32,
    pub sign: SynapseSign,
    pub anatomical_region: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflexOutcome {
    pub intervention: InterventionKind,
    pub movement_direction: MovementDirection,
    pub response_latency_ms: f64,
    pub response_magnitude: f64,
    pub behavioural_context_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HairPlateRecord {
    pub plate_id: String,
    pub taxon_id: String,
    pub leg: Leg,
    pub body_side: BodySide,
    pub segment: Segment,
    pub joint: Joint,

    pub record_sha256: String,
    pub morphology: Evidence<Morphology>,
    pub afferent_neuron_ids: Evidence<Vec<String>>,
    pub sensory_tuning: Evidence<SensoryTuning>,
    pub connectome_outputs: Evidence<Vec<ConnectomeOutput>>,
    pub reflex_outcomes: Evidence<Vec<ReflexOutcome>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceArtifactRole {
    Morphology,
    AfferentIdentity,
    SensoryTuning,
    ConnectomeOutput,
    ReflexOutcome,
    SearchReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCustodyMode {
    LocalBytesDigestVerified,
    ProviderChecksumOnly,
    VersionedRemoteMaterialization,
    DerivedFromSourceArtifacts,
    SyntheticFixture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceArtifact {
    pub artifact_id: String,
    pub source_uri: String,
    pub source_version_id: String,
    pub media_type: String,
    pub license_spdx: Option<String>,
    pub sha256: String,
    pub roles: Vec<SourceArtifactRole>,
    /// Declared provenance mode. This crate does not verify the underlying bytes.
    pub custody_mode: SourceCustodyMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HairPlateInventory {
    pub inventory_id: String,
    pub dataset_id: String,
    pub dataset_version: String,
    pub inventory_sha256: String,
    pub source_artifacts: Vec<SourceArtifact>,
    pub records: Vec<HairPlateRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictedMotorTarget {
    pub target_neuron_id: String,
    pub predicted_sign: SynapseSign,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HairPlatePrediction {
    pub plate_id: String,
    pub model_sha256: String,
    pub training_split_sha256: String,
    pub heldout_split_sha256: String,
    pub prediction_sha256: String,
    pub training_plate_ids: Vec<String>,
    /// Must match the measured tuning-curve threshold definition.
    pub predicted_threshold_definition_id: Option<String>,
    pub predicted_joint_limit_angle_rad: Option<f64>,
    /// Reflex predictions are conditional on this exact intervention and assay context.
    pub reflex_intervention: Option<InterventionKind>,
    pub reflex_context_id: Option<String>,
    /// Exact experiment/scope identity used when a complete assay found no reflex.
    pub reflex_assay_scope_id: Option<String>,
    pub predicted_reflex_direction: Option<MovementDirection>,
    pub predicted_motor_targets: Vec<PredictedMotorTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnscorableReason {
    UnknownEvidence,
    ExplicitlyAbsent,
    PredictionNotProvided,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotorTargetMetrics {
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub sign_evaluated_targets: usize,
    pub correct_sign_predictions: usize,
    pub sign_accuracy: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HairPlateEvaluation {
    pub plate_id: String,
    pub joint_limit_presence_correct: Option<bool>,
    pub joint_limit_presence_unscorable: Option<UnscorableReason>,
    pub joint_limit_absolute_error_rad: Option<f64>,
    pub joint_limit_unscorable: Option<UnscorableReason>,
    pub reflex_presence_correct: Option<bool>,
    pub reflex_presence_unscorable: Option<UnscorableReason>,
    pub reflex_direction_correct: Option<bool>,
    pub reflex_direction_unscorable: Option<UnscorableReason>,
    pub motor_target_metrics: Option<MotorTargetMetrics>,
    pub motor_targets_unscorable: Option<UnscorableReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleError {
    InvalidIdentifier(&'static str),
    InvalidDigest(&'static str),
    InvalidEvidence(&'static str),
    InvalidMorphology,
    InvalidTuningCurve,
    InvalidConnectomeOutput,
    InvalidReflexOutcome,
    EvidenceScopeMismatch,
    DuplicateIdentity(&'static str),
    EmptyInventory,
    ResourceLimitExceeded,
    PlateIdentityMismatch,
    PredictionIdentityMismatch,
    TrainingLeakage,
    UnknownPlate(String),
    CrossTaxonTrainingPlate(String),
    InvalidSourceArtifact,
    MissingSourceArtifact {
        sha256: String,
        role: SourceArtifactRole,
    },
    InvalidPrediction,
    NonFiniteMetric,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_source_uri(value: &str) -> bool {
    value.starts_with("https://")
        && value.len() <= 2048
        && !value.bytes().any(|byte| byte.is_ascii_whitespace())
}

fn valid_media_type(value: &str) -> bool {
    let mut parts = value.split('/');
    let Some(top) = parts.next() else {
        return false;
    };
    let Some(sub) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && !top.is_empty()
        && !sub.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'.' | b'-')
        })
}

impl SourceArtifact {
    pub fn validate(&self) -> Result<(), OracleError> {
        if !valid_identifier(&self.artifact_id)
            || !valid_identifier(&self.source_version_id)
            || self.license_spdx.as_ref().is_some_and(|license| !valid_identifier(license))
        {
            return Err(OracleError::InvalidSourceArtifact);
        }
        if !valid_source_uri(&self.source_uri)
            || !valid_media_type(&self.media_type)
            || !valid_sha256(&self.sha256)
            || self.roles.is_empty()
        {
            return Err(OracleError::InvalidSourceArtifact);
        }
        let mut roles = std::collections::BTreeSet::new();
        for role in &self.roles {
            if !roles.insert(*role) {
                return Err(OracleError::InvalidSourceArtifact);
            }
        }
        Ok(())
    }
}

fn source_artifact_has_role(
    artifacts: &[SourceArtifact],
    sha256: &str,
    required_role: SourceArtifactRole,
) -> bool {
    artifacts.iter().any(|artifact| {
        artifact.sha256 == sha256 && artifact.roles.contains(&required_role)
    })
}

fn require_evidence_artifact<T>(
    evidence: &Evidence<T>,
    artifacts: &[SourceArtifact],
    role: SourceArtifactRole,
) -> Result<(), OracleError> {
    let required = match evidence {
        Evidence::Observed { source_sha256, .. } => Some((source_sha256, role)),
        Evidence::ExplicitlyAbsent {
            search_receipt_sha256,
            ..
        } => Some((search_receipt_sha256, SourceArtifactRole::SearchReceipt)),
        Evidence::Unknown { .. } => None,
    };
    if let Some((sha256, required_role)) = required {
        if !source_artifact_has_role(artifacts, sha256, required_role) {
            return Err(OracleError::MissingSourceArtifact {
                sha256: sha256.clone(),
                role: required_role,
            });
        }
    }
    Ok(())
}

fn validate_evidence<T>(
    evidence: &Evidence<T>,
    label: &'static str,
    validate_value: impl FnOnce(&T) -> Result<(), OracleError>,
) -> Result<(), OracleError> {
    match evidence {
        Evidence::Observed {
            value,
            source_sha256,
        } => {
            if !valid_sha256(source_sha256) {
                return Err(OracleError::InvalidDigest(label));
            }
            validate_value(value)
        }
        Evidence::ExplicitlyAbsent {
            search_receipt_sha256,
            scope_id,
        } => {
            if !valid_sha256(search_receipt_sha256) {
                return Err(OracleError::InvalidDigest(label));
            }
            if !valid_identifier(scope_id) {
                return Err(OracleError::InvalidIdentifier(label));
            }
            Ok(())
        }
        Evidence::Unknown { detail, .. } => {
            if detail.trim().is_empty() || detail.len() > 512 {
                return Err(OracleError::InvalidEvidence(label));
            }
            Ok(())
        }
    }
}

fn vector_norm(vector: [f64; 3]) -> Result<f64, OracleError> {
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(OracleError::InvalidMorphology);
    }
    let norm = vector
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= f64::EPSILON {
        return Err(OracleError::InvalidMorphology);
    }
    Ok(norm)
}

fn validate_morphology(value: &Morphology) -> Result<(), OracleError> {
    if value.hair_count == 0 || !valid_sha256(&value.geometry_sha256) {
        return Err(OracleError::InvalidMorphology);
    }
    let normal_norm = vector_norm(value.contact_surface_normal)?;
    let axis_norm = vector_norm(value.orientation_axis)?;
    if (normal_norm - 1.0).abs() > 1.0e-3 || (axis_norm - 1.0).abs() > 1.0e-3 {
        return Err(OracleError::InvalidMorphology);
    }
    Ok(())
}

fn validate_tuning(value: &SensoryTuning) -> Result<(), OracleError> {
    if !value.joint_limit_angle_rad.is_finite()
        || value.joint_limit_angle_rad.abs() > std::f64::consts::TAU
        || !valid_identifier(&value.threshold_definition_id)
        || !valid_identifier(&value.response_unit)
        || value.response_points.len() < 2
        || value.response_points.len() > MAX_POINTS
    {
        return Err(OracleError::InvalidTuningCurve);
    }
    let mut previous_angle = None;
    for point in &value.response_points {
        if !point.joint_angle_rad.is_finite()
            || !point.normalized_response.is_finite()
            || point.joint_angle_rad.abs() > std::f64::consts::TAU
            || !(0.0..=1.0).contains(&point.normalized_response)
            || previous_angle.is_some_and(|previous| point.joint_angle_rad <= previous)
        {
            return Err(OracleError::InvalidTuningCurve);
        }
        previous_angle = Some(point.joint_angle_rad);
    }
    Ok(())
}

fn validate_connectome_outputs(value: &[ConnectomeOutput]) -> Result<(), OracleError> {
    if value.is_empty() || value.len() > MAX_TARGETS_PER_PLATE {
        return Err(OracleError::InvalidConnectomeOutput);
    }
    let mut identities = std::collections::BTreeSet::new();
    for target in value {
        if !valid_identifier(&target.target_neuron_id)
            || !valid_identifier(&target.target_cell_type)
            || !valid_identifier(&target.anatomical_region)
            || target.synapse_count == 0
            || !identities.insert(target.target_neuron_id.as_str())
        {
            return Err(OracleError::InvalidConnectomeOutput);
        }
    }
    Ok(())
}

fn validate_reflex_outcomes(value: &[ReflexOutcome]) -> Result<(), OracleError> {
    if value.is_empty() || value.len() > 256 {
        return Err(OracleError::InvalidReflexOutcome);
    }
    for outcome in value {
        if !outcome.response_latency_ms.is_finite()
            || outcome.response_latency_ms < 0.0
            || !outcome.response_magnitude.is_finite()
            || outcome.response_magnitude < 0.0
            || !valid_identifier(&outcome.behavioural_context_id)
        {
            return Err(OracleError::InvalidReflexOutcome);
        }
    }
    Ok(())
}

impl HairPlateRecord {
    pub fn validate(&self) -> Result<(), OracleError> {
        if !valid_identifier(&self.plate_id) {
            return Err(OracleError::InvalidIdentifier("plate_id"));
        }
        if !valid_identifier(&self.taxon_id) {
            return Err(OracleError::InvalidIdentifier("taxon_id"));
        }
        if !valid_sha256(&self.record_sha256) {
            return Err(OracleError::InvalidDigest("record"));
        }

        validate_evidence(&self.morphology, "morphology", validate_morphology)?;
        validate_evidence(&self.afferent_neuron_ids, "afferents", |ids| {
            if ids.is_empty() || ids.len() > MAX_NEURONS_PER_PLATE {
                return Err(OracleError::InvalidEvidence("afferents"));
            }
            let mut seen = std::collections::BTreeSet::new();
            for id in ids {
                if !valid_identifier(id) || !seen.insert(id.as_str()) {
                    return Err(OracleError::DuplicateIdentity("afferent_neuron_id"));
                }
            }
            Ok(())
        })?;
        validate_evidence(&self.sensory_tuning, "sensory_tuning", validate_tuning)?;
        validate_evidence(
            &self.connectome_outputs,
            "connectome_outputs",
            |outputs| validate_connectome_outputs(outputs),
        )?;
        validate_evidence(
            &self.reflex_outcomes,
            "reflex_outcomes",
            |outcomes| validate_reflex_outcomes(outcomes),
        )?;
        Ok(())
    }
}

impl HairPlateInventory {
    /// Validate that a blind prediction's held-out and training plates are
    /// members of this exact inventory and have the same declared taxon.
    pub fn validate_prediction(
        &self,
        prediction: &HairPlatePrediction,
    ) -> Result<(), OracleError> {
        self.validate()?;
        prediction.validate()?;

        let records = self
            .records
            .iter()
            .map(|record| (record.plate_id.as_str(), record))
            .collect::<std::collections::BTreeMap<_, _>>();
        let heldout = records
            .get(prediction.plate_id.as_str())
            .ok_or_else(|| OracleError::UnknownPlate(prediction.plate_id.clone()))?;

        for training_plate_id in &prediction.training_plate_ids {
            let training = records
                .get(training_plate_id.as_str())
                .ok_or_else(|| OracleError::UnknownPlate(training_plate_id.clone()))?;
            if training.taxon_id != heldout.taxon_id {
                return Err(OracleError::CrossTaxonTrainingPlate(
                    training_plate_id.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), OracleError> {
        if !valid_identifier(&self.inventory_id)
            || !valid_identifier(&self.dataset_id)
            || !valid_identifier(&self.dataset_version)
        {
            return Err(OracleError::InvalidIdentifier("inventory"));
        }
        if !valid_sha256(&self.inventory_sha256) {
            return Err(OracleError::InvalidDigest("inventory"));
        }
        if self.records.is_empty() {
            return Err(OracleError::EmptyInventory);
        }
        if self.records.len() > MAX_RECORDS
            || self.source_artifacts.is_empty()
            || self.source_artifacts.len() > MAX_SOURCE_ARTIFACTS
        {
            return Err(OracleError::ResourceLimitExceeded);
        }

        let mut artifact_ids = std::collections::BTreeSet::new();
        let mut artifact_digests = std::collections::BTreeSet::new();
        for artifact in &self.source_artifacts {
            artifact.validate()?;
            if !artifact_ids.insert(artifact.artifact_id.as_str())
                || !artifact_digests.insert(artifact.sha256.as_str())
            {
                return Err(OracleError::DuplicateIdentity("source_artifact"));
            }
        }

        let mut plate_ids = std::collections::BTreeSet::new();
        for record in &self.records {
            record.validate()?;
            if !plate_ids.insert(record.plate_id.as_str()) {
                return Err(OracleError::DuplicateIdentity("plate_id"));
            }

            require_evidence_artifact(
                &record.morphology,
                &self.source_artifacts,
                SourceArtifactRole::Morphology,
            )?;
            if let Evidence::Observed { value, .. } = &record.morphology {
                if !source_artifact_has_role(
                    &self.source_artifacts,
                    &value.geometry_sha256,
                    SourceArtifactRole::Morphology,
                ) {
                    return Err(OracleError::MissingSourceArtifact {
                        sha256: value.geometry_sha256.clone(),
                        role: SourceArtifactRole::Morphology,
                    });
                }
            }
            require_evidence_artifact(
                &record.afferent_neuron_ids,
                &self.source_artifacts,
                SourceArtifactRole::AfferentIdentity,
            )?;
            require_evidence_artifact(
                &record.sensory_tuning,
                &self.source_artifacts,
                SourceArtifactRole::SensoryTuning,
            )?;
            require_evidence_artifact(
                &record.connectome_outputs,
                &self.source_artifacts,
                SourceArtifactRole::ConnectomeOutput,
            )?;
            require_evidence_artifact(
                &record.reflex_outcomes,
                &self.source_artifacts,
                SourceArtifactRole::ReflexOutcome,
            )?;
        }
        Ok(())
    }
}

impl HairPlatePrediction {
    pub fn validate(&self) -> Result<(), OracleError> {
        if !valid_identifier(&self.plate_id) {
            return Err(OracleError::InvalidIdentifier("plate_id"));
        }
        for (digest, label) in [
            (&self.model_sha256, "model"),
            (&self.training_split_sha256, "training_split"),
            (&self.heldout_split_sha256, "heldout_split"),
            (&self.prediction_sha256, "prediction"),
        ] {
            if !valid_sha256(digest) {
                return Err(OracleError::InvalidDigest(label));
            }
        }
        if self.training_split_sha256 == self.heldout_split_sha256
            || self.training_plate_ids.is_empty()
            || self.training_plate_ids.len() > MAX_TRAIN_PLATES
            || self.predicted_motor_targets.len() > MAX_TARGETS_PER_PLATE
        {
            return Err(OracleError::InvalidPrediction);
        }

        let mut training_ids = std::collections::BTreeSet::new();
        for id in &self.training_plate_ids {
            if !valid_identifier(id) || !training_ids.insert(id.as_str()) {
                return Err(OracleError::DuplicateIdentity("training_plate_id"));
            }
        }
        if training_ids.contains(self.plate_id.as_str()) {
            return Err(OracleError::TrainingLeakage);
        }

        if self
            .predicted_joint_limit_angle_rad
            .is_some_and(|angle| !angle.is_finite() || angle.abs() > std::f64::consts::TAU)
            || self
                .predicted_threshold_definition_id
                .as_ref()
                .is_some_and(|id| !valid_identifier(id))
            || self.predicted_joint_limit_angle_rad.is_some()
                != self.predicted_threshold_definition_id.is_some()
            || self
                .reflex_context_id
                .as_ref()
                .is_some_and(|id| !valid_identifier(id))
            || self.reflex_intervention.is_some() != self.reflex_context_id.is_some()
            || self.reflex_intervention.is_some() != self.reflex_assay_scope_id.is_some()
            || self
                .reflex_assay_scope_id
                .as_ref()
                .is_some_and(|id| !valid_identifier(id))
            || self.predicted_reflex_direction.is_some()
                && self.reflex_intervention.is_none()
        {
            return Err(OracleError::InvalidPrediction);
        }

        let mut target_ids = std::collections::BTreeSet::new();
        for target in &self.predicted_motor_targets {
            if !valid_identifier(&target.target_neuron_id)
                || !target.confidence.is_finite()
                || !(0.0..=1.0).contains(&target.confidence)
                || !target_ids.insert(target.target_neuron_id.as_str())
                || target.predicted_sign == SynapseSign::Unknown
            {
                return Err(OracleError::InvalidPrediction);
            }
        }
        Ok(())
    }
}

fn motor_target_metrics(
    predicted: &[PredictedMotorTarget],
    truth: &[ConnectomeOutput],
) -> Result<MotorTargetMetrics, OracleError> {
    let predicted_by_id = predicted
        .iter()
        .map(|target| (target.target_neuron_id.as_str(), target))
        .collect::<std::collections::BTreeMap<_, _>>();
    let truth_by_id = truth
        .iter()
        .map(|target| (target.target_neuron_id.as_str(), target))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut true_positives = 0usize;
    let mut false_positives = 0usize;
    let mut false_negatives = 0usize;
    let mut sign_evaluated_targets = 0usize;
    let mut correct_sign_predictions = 0usize;

    for (target_id, prediction) in &predicted_by_id {
        if let Some(observation) = truth_by_id.get(target_id) {
            true_positives += 1;
            if observation.sign != SynapseSign::Unknown {
                sign_evaluated_targets += 1;
                if prediction.predicted_sign == observation.sign {
                    correct_sign_predictions += 1;
                }
            }
        } else {
            false_positives += 1;
        }
    }
    for target_id in truth_by_id.keys() {
        if !predicted_by_id.contains_key(target_id) {
            false_negatives += 1;
        }
    }

    let precision = if predicted_by_id.is_empty() {
        Some(if truth_by_id.is_empty() { 1.0 } else { 0.0 })
    } else {
        Some(true_positives as f64 / predicted_by_id.len() as f64)
    };
    let recall = if truth_by_id.is_empty() {
        None
    } else {
        Some(true_positives as f64 / truth_by_id.len() as f64)
    };
    let f1 = recall.map(|recall_value| {
        let precision_value = precision.unwrap_or(0.0);
        let denominator = precision_value + recall_value;
        if denominator <= f64::EPSILON {
            0.0
        } else {
            2.0 * precision_value * recall_value / denominator
        }
    });
    let sign_accuracy = if sign_evaluated_targets == 0 {
        None
    } else {
        Some(correct_sign_predictions as f64 / sign_evaluated_targets as f64)
    };

    if precision.is_some_and(|value| !value.is_finite())
        || recall.is_some_and(|value| !value.is_finite())
        || f1.is_some_and(|value| !value.is_finite())
        || sign_accuracy.is_some_and(|value| !value.is_finite())
    {
        return Err(OracleError::NonFiniteMetric);
    }

    Ok(MotorTargetMetrics {
        true_positives,
        false_positives,
        false_negatives,
        precision,
        recall,
        f1,
        sign_evaluated_targets,
        correct_sign_predictions,
        sign_accuracy,
    })
}

pub fn evaluate_prediction(
    record: &HairPlateRecord,
    prediction: &HairPlatePrediction,
) -> Result<HairPlateEvaluation, OracleError> {
    record.validate()?;
    prediction.validate()?;
    if record.plate_id != prediction.plate_id {
        return Err(OracleError::PredictionIdentityMismatch);
    }

    let (joint_limit_presence_correct, joint_limit_presence_unscorable) =
        match &record.sensory_tuning {
            Evidence::Observed { .. } => (
                Some(prediction.predicted_joint_limit_angle_rad.is_some()),
                None,
            ),
            Evidence::ExplicitlyAbsent { .. } => (
                Some(prediction.predicted_joint_limit_angle_rad.is_none()),
                None,
            ),
            Evidence::Unknown { .. } => (
                None,
                Some(UnscorableReason::UnknownEvidence),
            ),
        };

    let (joint_limit_absolute_error_rad, joint_limit_unscorable) =
        match (
            &record.sensory_tuning,
            prediction.predicted_joint_limit_angle_rad,
            prediction.predicted_threshold_definition_id.as_deref(),
        ) {
            (Evidence::Observed { value, .. }, Some(predicted), Some(threshold_id))
                if threshold_id == value.threshold_definition_id =>
            {
                let error = (predicted - value.joint_limit_angle_rad).abs();
                if !error.is_finite() {
                    return Err(OracleError::NonFiniteMetric);
                }
                (Some(error), None)
            }
            (Evidence::Unknown { .. }, _, _) => {
                (None, Some(UnscorableReason::UnknownEvidence))
            }
            (Evidence::ExplicitlyAbsent { .. }, _, _) => {
                (None, Some(UnscorableReason::ExplicitlyAbsent))
            }
            (Evidence::Observed { .. }, Some(_), Some(_)) => {
                return Err(OracleError::InvalidPrediction);
            }
            (Evidence::Observed { .. }, _, _) => {
                (None, Some(UnscorableReason::PredictionNotProvided))
            }
        };

    let reflex_scope = match (
        prediction.reflex_intervention,
        prediction.reflex_context_id.as_deref(),
        prediction.reflex_assay_scope_id.as_deref(),
    ) {
        (Some(intervention), Some(context_id), Some(scope_id)) => {
            Some((intervention, context_id, scope_id))
        }
        (None, None, None) => None,
        _ => return Err(OracleError::InvalidPrediction),
    };

    let observed_scoped_reflexes = match (&record.reflex_outcomes, reflex_scope) {
        (Evidence::Observed { value, .. }, Some((intervention, context_id, _))) => Some(
            value
                .iter()
                .filter(|outcome| {
                    outcome.intervention == intervention
                        && outcome.behavioural_context_id == context_id
                })
                .collect::<Vec<_>>(),
        ),
        _ => None,
    };

    let (reflex_presence_correct, reflex_presence_unscorable) =
        match (&record.reflex_outcomes, reflex_scope) {
            (Evidence::Observed { .. }, Some(_)) => {
                match observed_scoped_reflexes.as_ref() {
                    Some(outcomes) if !outcomes.is_empty() => (
                        Some(prediction.predicted_reflex_direction.is_some()),
                        None,
                    ),
                    _ => (None, Some(UnscorableReason::UnknownEvidence)),
                }
            }
            (
                Evidence::ExplicitlyAbsent { scope_id, .. },
                Some((_, _, predicted_scope_id)),
            ) => {
                if scope_id != predicted_scope_id {
                    return Err(OracleError::EvidenceScopeMismatch);
                }
                (
                    Some(prediction.predicted_reflex_direction.is_none()),
                    None,
                )
            }
            (Evidence::Unknown { .. }, _) => (
                None,
                Some(UnscorableReason::UnknownEvidence),
            ),
            (Evidence::ExplicitlyAbsent { .. }, None) => (
                None,
                Some(UnscorableReason::PredictionNotProvided),
            ),
            (Evidence::Observed { .. }, None) => (
                None,
                Some(UnscorableReason::PredictionNotProvided),
            ),
        };

    let (reflex_direction_correct, reflex_direction_unscorable) =
        match (
            &record.reflex_outcomes,
            observed_scoped_reflexes.as_ref(),
            prediction.predicted_reflex_direction,
        ) {
            (Evidence::Observed { .. }, Some(outcomes), Some(predicted))
                if !outcomes.is_empty() =>
            {
                let directions = outcomes
                    .iter()
                    .map(|outcome| outcome.movement_direction)
                    .collect::<std::collections::BTreeSet<_>>();
                if directions.len() == 1 {
                    (Some(directions.contains(&predicted)), None)
                } else {
                    (None, Some(UnscorableReason::UnknownEvidence))
                }
            }
            (Evidence::Unknown { .. }, _, _) => (
                None,
                Some(UnscorableReason::UnknownEvidence),
            ),
            (Evidence::ExplicitlyAbsent { .. }, _, _) => (
                None,
                Some(UnscorableReason::ExplicitlyAbsent),
            ),
            (Evidence::Observed { .. }, Some(_), None) => (
                None,
                Some(UnscorableReason::PredictionNotProvided),
            ),
            (Evidence::Observed { .. }, _, _) => (
                None,
                Some(UnscorableReason::PredictionNotProvided),
            ),
        };

    let (motor_target_metrics, motor_targets_unscorable) =
        match &record.connectome_outputs {
            Evidence::Observed { value, .. } => {
                (Some(motor_target_metrics(&prediction.predicted_motor_targets, value)?), None)
            }
            Evidence::ExplicitlyAbsent { .. } => (
                Some(motor_target_metrics(&prediction.predicted_motor_targets, &[])?),
                None,
            ),
            Evidence::Unknown { .. } => (None, Some(UnscorableReason::UnknownEvidence)),
        };

    Ok(HairPlateEvaluation {
        plate_id: record.plate_id.clone(),
        joint_limit_presence_correct,
        joint_limit_presence_unscorable,
        joint_limit_absolute_error_rad,
        joint_limit_unscorable,
        reflex_presence_correct,
        reflex_presence_unscorable,
        reflex_direction_correct,
        reflex_direction_unscorable,
        motor_target_metrics,
        motor_targets_unscorable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn tuning() -> SensoryTuning {
        SensoryTuning {
            joint_limit_angle_rad: 0.8,
            threshold_definition_id: "response-over-0p5-v1".to_string(),
            response_points: vec![
                TuningPoint {
                    joint_angle_rad: 0.2,
                    normalized_response: 0.1,
                },
                TuningPoint {
                    joint_angle_rad: 0.5,
                    normalized_response: 0.3,
                },
                TuningPoint {
                    joint_angle_rad: 0.8,
                    normalized_response: 0.9,
                },
            ],
            response_unit: "normalized-calcium-df-f".to_string(),
        }
    }

    fn record() -> HairPlateRecord {
        HairPlateRecord {
            plate_id: "front-coxa-hp8".to_string(),
            taxon_id: "drosophila-melanogaster".to_string(),
            leg: Leg::Front,
            body_side: BodySide::Left,
            segment: Segment::Coxa,
            joint: Joint::ThoraxCoxa,
            record_sha256: digest('a'),
            morphology: Evidence::Observed {
                value: Morphology {
                    hair_count: 8,
                    contact_surface_normal: [0.0, 0.0, 1.0],
                    orientation_axis: [1.0, 0.0, 0.0],
                    geometry_sha256: digest('b'),
                },
                source_sha256: digest('c'),
            },
            afferent_neuron_ids: Evidence::Observed {
                value: vec!["CxHP8-a".to_string(), "CxHP8-b".to_string()],
                source_sha256: digest('d'),
            },
            sensory_tuning: Evidence::Observed {
                value: tuning(),
                source_sha256: digest('e'),
            },
            connectome_outputs: Evidence::Observed {
                value: vec![
                    ConnectomeOutput {
                        target_neuron_id: "motor-posterior-1".to_string(),
                        target_cell_type: "motor-neuron".to_string(),
                        synapse_count: 24,
                        sign: SynapseSign::Excitatory,
                        anatomical_region: "front-leg-neuropil".to_string(),
                    },
                    ConnectomeOutput {
                        target_neuron_id: "premotor-anterior-1".to_string(),
                        target_cell_type: "premotor-neuron".to_string(),
                        synapse_count: 18,
                        sign: SynapseSign::Inhibitory,
                        anatomical_region: "front-leg-neuropil".to_string(),
                    },
                ],
                source_sha256: digest('f'),
            },
            reflex_outcomes: Evidence::Observed {
                value: vec![ReflexOutcome {
                    intervention: InterventionKind::OptogeneticActivation,
                    movement_direction: MovementDirection::Posterior,
                    response_latency_ms: 120.0,
                    response_magnitude: 0.7,
                    behavioural_context_id: "standing-v1".to_string(),
                }],
                source_sha256: digest('1'),
            },
        }
    }

    fn prediction() -> HairPlatePrediction {
        HairPlatePrediction {
            plate_id: "front-coxa-hp8".to_string(),
            model_sha256: digest('2'),
            training_split_sha256: digest('3'),
            heldout_split_sha256: digest('4'),
            prediction_sha256: digest('5'),
            training_plate_ids: vec!["front-coxa-hp3".to_string()],
            predicted_threshold_definition_id: Some("response-over-0p5-v1".to_string()),
            predicted_joint_limit_angle_rad: Some(0.75),
            reflex_intervention: Some(InterventionKind::OptogeneticActivation),
            reflex_context_id: Some("standing-v1".to_string()),
            reflex_assay_scope_id: Some("activation-standing-v1".to_string()),
            predicted_reflex_direction: Some(MovementDirection::Posterior),
            predicted_motor_targets: vec![
                PredictedMotorTarget {
                    target_neuron_id: "motor-posterior-1".to_string(),
                    predicted_sign: SynapseSign::Excitatory,
                    confidence: 0.9,
                },
                PredictedMotorTarget {
                    target_neuron_id: "premotor-anterior-1".to_string(),
                    predicted_sign: SynapseSign::Inhibitory,
                    confidence: 0.8,
                },
            ],
        }
    }

    #[test]
    fn validates_source_qualified_plate_inventory() {
        let inventory = HairPlateInventory {
            inventory_id: "drosophila-hair-plate-v1".to_string(),
            dataset_id: "fanc-v1-2-1".to_string(),
            dataset_version: "1.2.1".to_string(),
            inventory_sha256: digest('6'),
            records: vec![record()],
        };
        assert_eq!(inventory.validate(), Ok(()));
    }

    #[test]
    fn unknown_evidence_is_not_explicit_absence() {
        let mut input = record();
        input.connectome_outputs = Evidence::Unknown {
            reason: EvidenceUnknownReason::NotReconstructed,
            detail: "targets are not yet annotated".to_string(),
        };
        let input_prediction = prediction();
        let result = evaluate_prediction(&input, &input_prediction).expect("evaluation");
        assert_eq!(result.motor_target_metrics, None);
        assert_eq!(
            result.motor_targets_unscorable,
            Some(UnscorableReason::UnknownEvidence)
        );
    }

    #[test]
    fn explicit_absence_is_a_scored_empty_target_set_not_unknown() {
        let mut input = record();
        input.connectome_outputs = Evidence::ExplicitlyAbsent {
            search_receipt_sha256: digest('7'),
            scope_id: "complete-front-leg-motor-output-search-v1".to_string(),
        };
        let result = evaluate_prediction(&input, &prediction()).expect("evaluation");
        let metrics = result.motor_target_metrics.expect("known negative set");
        assert_eq!(metrics.true_positives, 0);
        assert_eq!(metrics.false_positives, 2);
        assert_eq!(metrics.false_negatives, 0);
        assert_eq!(metrics.precision, Some(0.0));
        assert_eq!(metrics.recall, None);
    }

    #[test]
    fn evaluates_known_answer_joint_limit_reflex_and_motor_signs() {
        let result = evaluate_prediction(&record(), &prediction()).expect("evaluation");
        assert!((result.joint_limit_absolute_error_rad.unwrap_or(f64::NAN) - 0.05).abs() < 1.0e-12);
        assert_eq!(result.reflex_direction_correct, Some(true));
        let targets = result.motor_target_metrics.expect("target metrics");
        assert_eq!(targets.true_positives, 2);
        assert_eq!(targets.false_positives, 0);
        assert_eq!(targets.false_negatives, 0);
        assert_eq!(targets.f1, Some(1.0));
        assert_eq!(targets.sign_accuracy, Some(1.0));
    }

    #[test]
    fn explicitly_absent_tuning_and_reflex_are_scored_as_negative_predictions() {
        let mut input = record();
        input.sensory_tuning = Evidence::ExplicitlyAbsent {
            search_receipt_sha256: digest('7'),
            scope_id: "complete-front-coxa-tuning-search-v1".to_string(),
        };
        input.reflex_outcomes = Evidence::ExplicitlyAbsent {
            search_receipt_sha256: digest('8'),
            scope_id: "activation-standing-v1".to_string(),
        };
        let mut absent_prediction = prediction();
        absent_prediction.predicted_threshold_definition_id = None;
        absent_prediction.predicted_joint_limit_angle_rad = None;
        absent_prediction.predicted_reflex_direction = None;

        let absent_result =
            evaluate_prediction(&input, &absent_prediction).expect("absent case");
        assert_eq!(absent_result.joint_limit_presence_correct, Some(true));
        assert_eq!(absent_result.reflex_presence_correct, Some(true));

        let false_positive_result =
            evaluate_prediction(&input, &prediction()).expect("false positive case");
        assert_eq!(false_positive_result.joint_limit_presence_correct, Some(false));
        assert_eq!(false_positive_result.reflex_presence_correct, Some(false));
        assert_eq!(
            false_positive_result.joint_limit_unscorable,
            Some(UnscorableReason::ExplicitlyAbsent)
        );
        assert_eq!(
            false_positive_result.reflex_direction_unscorable,
            Some(UnscorableReason::ExplicitlyAbsent)
        );
    }

    #[test]
    fn unknown_tuning_and_reflex_remain_unscorable_not_negative() {
        let mut input = record();
        input.sensory_tuning = Evidence::Unknown {
            reason: EvidenceUnknownReason::NotMeasured,
            detail: "not measured in this plate".to_string(),
        };
        input.reflex_outcomes = Evidence::Unknown {
            reason: EvidenceUnknownReason::NotMeasured,
            detail: "no reflex assay available".to_string(),
        };
        let result = evaluate_prediction(&input, &prediction()).expect("unknown case");
        assert_eq!(result.joint_limit_presence_correct, None);
        assert_eq!(result.reflex_presence_correct, None);
        assert_eq!(
            result.joint_limit_presence_unscorable,
            Some(UnscorableReason::UnknownEvidence)
        );
        assert_eq!(
            result.reflex_presence_unscorable,
            Some(UnscorableReason::UnknownEvidence)
        );
    }

    #[test]
    fn inventory_validates_heldout_and_training_membership() {
        let mut heldout = record();
        let mut training = record();
        training.plate_id = "front-coxa-hp3".to_string();
        training.record_sha256 = digest('9');

        let inventory = HairPlateInventory {
            inventory_id: "drosophila-hair-plate-v1".to_string(),
            dataset_id: "fanc-v1-2-1".to_string(),
            dataset_version: "1.2.1".to_string(),
            inventory_sha256: digest('6'),
            records: vec![heldout, training],
        };
        assert_eq!(inventory.validate_prediction(&prediction()), Ok(()));
    }

    #[test]
    fn inventory_rejects_cross_taxon_training_membership() {
        let heldout = record();
        let mut training = record();
        training.plate_id = "other-taxon-plate-1".to_string();
        training.taxon_id = "drosophila-simulans".to_string();
        training.record_sha256 = digest('9');

        let inventory = HairPlateInventory {
            inventory_id: "mixed-taxon-hair-plates-v1".to_string(),
            dataset_id: "fanc-v1-2-1".to_string(),
            dataset_version: "1.2.1".to_string(),
            inventory_sha256: digest('6'),
            records: vec![heldout, training],
        };
        assert_eq!(
            inventory.validate_prediction(&prediction()),
            Err(OracleError::CrossTaxonTrainingPlate(
                "other-taxon-plate-1".to_string()
            ))
        );
    }

    #[test]
    fn inventory_rejects_training_plate_missing_from_the_pinned_inventory() {
        let inventory = HairPlateInventory {
            inventory_id: "drosophila-hair-plate-v1".to_string(),
            dataset_id: "fanc-v1-2-1".to_string(),
            dataset_version: "1.2.1".to_string(),
            inventory_sha256: digest('6'),
            records: vec![record()],
        };
        assert_eq!(
            inventory.validate_prediction(&prediction()),
            Err(OracleError::UnknownPlate("front-coxa-hp3".to_string()))
        );
    }

    #[test]
    fn rejects_joint_limit_prediction_with_a_different_threshold_definition() {
        let mut input_prediction = prediction();
        input_prediction.predicted_threshold_definition_id =
            Some("different-threshold-v2".to_string());
        assert_eq!(
            evaluate_prediction(&record(), &input_prediction),
            Err(OracleError::InvalidPrediction)
        );
    }

    #[test]
    fn reflex_direction_is_scored_only_for_the_declared_assay_context() {
        let mut input = record();
        if let Evidence::Observed { value, .. } = &mut input.reflex_outcomes {
            value.push(ReflexOutcome {
                intervention: InterventionKind::OptogeneticSilencing,
                movement_direction: MovementDirection::Anterior,
                response_latency_ms: 150.0,
                response_magnitude: 0.4,
                behavioural_context_id: "walking-v1".to_string(),
            });
        }
        let result = evaluate_prediction(&input, &prediction()).expect("scoped reflex");
        assert_eq!(result.reflex_presence_correct, Some(true));
        assert_eq!(result.reflex_direction_correct, Some(true));
    }

    #[test]
    fn observed_outcomes_do_not_turn_an_unmeasured_context_into_negative_evidence() {
        let input = record();
        let mut different_context = prediction();
        different_context.reflex_intervention = Some(InterventionKind::OptogeneticSilencing);
        different_context.reflex_context_id = Some("walking-v1".to_string());
        different_context.reflex_assay_scope_id = Some("silencing-walking-v1".to_string());
        let result =
            evaluate_prediction(&input, &different_context).expect("unmeasured assay scope");
        assert_eq!(result.reflex_presence_correct, None);
        assert_eq!(
            result.reflex_presence_unscorable,
            Some(UnscorableReason::UnknownEvidence)
        );
    }

    #[test]
    fn explicit_reflex_absence_requires_exact_assay_scope_match() {
        let mut input = record();
        input.reflex_outcomes = Evidence::ExplicitlyAbsent {
            search_receipt_sha256: digest('8'),
            scope_id: "activation-standing-v1".to_string(),
        };
        let mut invalid = prediction();
        invalid.reflex_assay_scope_id = Some("different-assay-scope".to_string());
        assert_eq!(
            evaluate_prediction(&input, &invalid),
            Err(OracleError::EvidenceScopeMismatch)
        );
    }

    #[test]
    fn rejects_reflex_direction_prediction_without_intervention_and_context() {
        let mut invalid = prediction();
        invalid.reflex_intervention = None;
        invalid.reflex_context_id = None;
        invalid.reflex_assay_scope_id = None;
        assert_eq!(invalid.validate(), Err(OracleError::InvalidPrediction));
    }

    #[test]
    fn rejects_prediction_trained_on_the_held_out_plate() {
        let mut invalid = prediction();
        invalid.training_plate_ids.push(invalid.plate_id.clone());
        assert_eq!(
            invalid.validate(),
            Err(OracleError::TrainingLeakage)
        );
    }

    #[test]
    fn rejects_non_monotonic_tuning_curve() {
        let mut invalid = record();
        if let Evidence::Observed { value, .. } = &mut invalid.sensory_tuning {
            value.response_points[2].joint_angle_rad = 0.4;
        }
        assert_eq!(
            invalid.validate(),
            Err(OracleError::InvalidTuningCurve)
        );
    }

    #[test]
    fn rejects_duplicate_afferent_or_target_identities() {
        let mut invalid = record();
        invalid.afferent_neuron_ids = Evidence::Observed {
            value: vec!["same-neuron".to_string(), "same-neuron".to_string()],
            source_sha256: digest('d'),
        };
        assert_eq!(
            invalid.validate(),
            Err(OracleError::DuplicateIdentity("afferent_neuron_id"))
        );

        let mut target_duplicate = record();
        if let Evidence::Observed { value, .. } = &mut target_duplicate.connectome_outputs {
            value[1].target_neuron_id = value[0].target_neuron_id.clone();
        }
        assert_eq!(
            target_duplicate.validate(),
            Err(OracleError::InvalidConnectomeOutput)
        );
    }

    #[test]
    fn rejects_invalid_morphology_axes_and_artifact_digests() {
        let mut invalid = record();
        if let Evidence::Observed { value, .. } = &mut invalid.morphology {
            value.orientation_axis = [0.0, 0.0, 0.0];
        }
        assert_eq!(invalid.validate(), Err(OracleError::InvalidMorphology));

        let mut invalid_digest = record();
        invalid_digest.record_sha256 = "not-a-digest".to_string();
        assert_eq!(
            invalid_digest.validate(),
            Err(OracleError::InvalidDigest("record"))
        );
    }

    #[test]
    fn unknown_fields_require_a_reason_and_do_not_silently_score() {
        let mut invalid = record();
        invalid.sensory_tuning = Evidence::Unknown {
            reason: EvidenceUnknownReason::NotMeasured,
            detail: "  ".to_string(),
        };
        assert_eq!(
            invalid.validate(),
            Err(OracleError::InvalidEvidence("sensory_tuning"))
        );
        let result = evaluate_prediction(&invalid, &prediction()).expect_err("invalid evidence");
        assert_eq!(result, OracleError::InvalidEvidence("sensory_tuning"));
    }
}
