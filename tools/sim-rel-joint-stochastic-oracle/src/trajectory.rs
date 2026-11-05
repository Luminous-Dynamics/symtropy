//! Source-locked locomotion trajectory receipts.
//!
//! This module validates trajectory identity, source/evidence scope, units,
//! coordinate frames, replicate identity and time ordering before a downstream
//! estimator fits transition factors. It deliberately does not fit CPDs or
//! treat frames as independent biological replicates.

use std::collections::{BTreeMap, BTreeSet};

use super::locomotion::{
    source_supports_claim, valid_sha256, validate_source, AdapterError, ClaimCeiling,
    EvidenceClass, InterventionAction, LegId, SourceDigestPin, SourceKind, SourceManifest,
};
use super::Rational;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AxisLabel {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CoordinateHandedness {
    RightHanded,
    LeftHanded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoordinateFrame {
    pub id: String,
    pub handedness: CoordinateHandedness,
    /// Ordered physical axes; order and handedness are part of the schema.
    pub axis_order: [AxisLabel; 3],
    pub origin_definition: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MeasurementUnit {
    Radian,
    Degree,
    Meter,
    Millimeter,
    MeterPerSecond,
    RadianPerSecond,
    Newton,
    NewtonMeter,
    Hertz,
    Millivolt,
    Dimensionless,
    Categorical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MeasurementKind {
    GaitPhase,
    JointAngle,
    JointTranslation,
    JointVelocity,
    ContactState,
    ContactForce,
    ActuatorForce,
    ActuatorTorque,
    PositionAfferent,
    MovementAfferent,
    VibrationAfferent,
    NeuralActivity,
    SensorSignal,
    ExternalLoad,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrajectoryVariable {
    pub id: String,
    pub kind: MeasurementKind,
    pub unit: MeasurementUnit,
    pub coordinate_frame_id: Option<String>,
    pub leg_id: Option<LegId>,
    /// Sources supporting the declared measurement and its interpretation.
    pub source_manifest_ids: Vec<String>,
    pub claim_ceiling: ClaimCeiling,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameRate {
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrajectoryRecordingManifest {
    pub id: String,
    pub frame_rate: FrameRate,
    pub coordinate_frames: Vec<CoordinateFrame>,
    pub variables: Vec<TrajectoryVariable>,
    pub source_manifests: Vec<SourceManifest>,
    pub source_pins: Vec<SourceDigestPin>,
    pub actions: Vec<InterventionAction>,
    /// Preregistered minimum fidelity for a delivered intervention.
    pub minimum_delivery_fidelity: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplicationUnit {
    /// Biological replication unit. Sessions and frames are nested within it.
    AnimalOrPreparation(String),
    /// In-silico unit. Never count it as an animal/preparation.
    SimulationInstance(String),
}

impl ReplicationUnit {
    pub fn id(&self) -> &str {
        match self {
            Self::AnimalOrPreparation(id) | Self::SimulationInstance(id) => id,
        }
    }

    pub fn is_biological(&self) -> bool {
        matches!(self, Self::AnimalOrPreparation(_))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimulatorProfile {
    pub simulator_id: String,
    pub release: String,
    pub code_commit_sha: String,
    pub physics_engine: String,
    pub physics_engine_version: String,
    pub physics_timestep_ns: u64,
    pub control_period_ns: u64,
    pub solver_tolerance: f64,
    pub morphology_sha256: String,
    pub terrain_sha256: String,
    /// Pinned code and asset sources used by this simulator run.
    pub simulator_source_manifest_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MeasurementValue {
    Scalar(f64),
    Category(String),
    /// Missingness is explicit and must not be silently converted to zero.
    Missing { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrajectoryFrame {
    /// Source frame index. Gaps are allowed; duplicate or decreasing indices are not.
    pub frame_index: u64,
    pub timestamp_ns: u64,
    /// Exactly one value or explicit missingness entry per schema variable.
    pub values: BTreeMap<String, MeasurementValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrajectoryRunReceipt {
    pub recording_manifest_id: String,
    pub sample_id: String,
    pub session_id: String,
    pub run_id: String,
    pub replication_unit: ReplicationUnit,
    pub source_manifest_id: String,
    /// Digest of the acquired trajectory payload, distinct from archive custody.
    pub trajectory_payload_sha256: String,
    pub claim_ceiling: ClaimCeiling,
    pub assigned_action_id: String,
    pub delivered_protocol_id: String,
    pub delivery_verified: bool,
    pub delivery_fidelity: Rational,
    /// SHA-256 of the complete pre-outcome history used by the forecast.
    pub conditioning_state_digest: String,
    pub simulator_profile: Option<SimulatorProfile>,
    pub frames: Vec<TrajectoryFrame>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrajectoryError {
    EmptyIdentifier(&'static str),
    DuplicateIdentifier { kind: &'static str, id: String },
    UnknownIdentifier { kind: &'static str, id: String },
    SourceValidation(AdapterError),
    InvalidSourcePin { source_id: String },
    SourcePinMismatch { source_id: String },
    UnpinnedNonSyntheticSource { source_id: String },
    InvalidClaimCeiling { subject_id: String, claim: ClaimCeiling },
    MissingVariableSource { variable_id: String },
    InvalidVariableUnit { variable_id: String, kind: MeasurementKind, unit: MeasurementUnit },
    MissingCoordinateFrame { variable_id: String },
    UnexpectedCoordinateFrame { variable_id: String },
    MissingLegIdentity { variable_id: String },
    UnexpectedLegIdentity { variable_id: String },
    InvalidFrameRate,
    InvalidDeliveryFidelityThreshold,
    InvalidDeliveryFidelity,
    InvalidConditioningStateDigest,
    InvalidTrajectoryDigest,
    InvalidSimulatorProfile(&'static str),
    SimulatorProfileOnMeasuredData,
    MissingSimulatorProfile,
    ReplicationUnitEvidenceMismatch,
    ConnectomeCannotScoreTrajectory,
    AssignmentDeliveryMismatch {
        assigned_action_id: String,
        expected_protocol_id: String,
        delivered_protocol_id: String,
    },
    InvalidDeliveryReceipt(&'static str),
    EmptyTrajectory,
    NonMonotonicFrameIndex,
    NonMonotonicTimestamp,
    FrameSchemaMismatch { frame_index: u64 },
    InvalidMeasurement { frame_index: u64, variable_id: String },
    DuplicateSampleId(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledTrajectoryRecordingManifest {
    pub manifest: TrajectoryRecordingManifest,
}

fn nonempty(value: &str, field: &'static str) -> Result<(), TrajectoryError> {
    if value.trim().is_empty() {
        Err(TrajectoryError::EmptyIdentifier(field))
    } else {
        Ok(())
    }
}

fn valid_git_sha(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn requires_coordinate_frame(kind: MeasurementKind) -> bool {
    matches!(
        kind,
        MeasurementKind::JointAngle
            | MeasurementKind::JointTranslation
            | MeasurementKind::JointVelocity
            | MeasurementKind::ContactForce
            | MeasurementKind::ActuatorForce
            | MeasurementKind::ActuatorTorque
    )
}

fn requires_leg_identity(kind: MeasurementKind) -> bool {
    matches!(
        kind,
        MeasurementKind::JointAngle
            | MeasurementKind::JointTranslation
            | MeasurementKind::JointVelocity
            | MeasurementKind::ContactState
            | MeasurementKind::ContactForce
            | MeasurementKind::ActuatorForce
            | MeasurementKind::ActuatorTorque
            | MeasurementKind::PositionAfferent
            | MeasurementKind::MovementAfferent
            | MeasurementKind::VibrationAfferent
    )
}

fn unit_matches_kind(kind: MeasurementKind, unit: MeasurementUnit) -> bool {
    match kind {
        MeasurementKind::GaitPhase | MeasurementKind::ContactState => {
            unit == MeasurementUnit::Categorical
        }
        MeasurementKind::JointAngle => {
            matches!(unit, MeasurementUnit::Radian | MeasurementUnit::Degree)
        }
        MeasurementKind::JointTranslation => {
            matches!(unit, MeasurementUnit::Meter | MeasurementUnit::Millimeter)
        }
        MeasurementKind::JointVelocity => matches!(
            unit,
            MeasurementUnit::MeterPerSecond | MeasurementUnit::RadianPerSecond
        ),
        MeasurementKind::ContactForce | MeasurementKind::ActuatorForce | MeasurementKind::ExternalLoad => {
            unit == MeasurementUnit::Newton
        }
        MeasurementKind::ActuatorTorque => unit == MeasurementUnit::NewtonMeter,
        MeasurementKind::NeuralActivity => {
            matches!(unit, MeasurementUnit::Hertz | MeasurementUnit::Millivolt)
        }
        MeasurementKind::PositionAfferent
        | MeasurementKind::MovementAfferent
        | MeasurementKind::VibrationAfferent
        | MeasurementKind::SensorSignal => matches!(
            unit,
            MeasurementUnit::Hertz | MeasurementUnit::Millivolt | MeasurementUnit::Dimensionless
        ),
        MeasurementKind::Other => true,
    }
}

fn validate_coordinate_frame(frame: &CoordinateFrame) -> Result<(), TrajectoryError> {
    nonempty(&frame.id, "coordinate_frame_id")?;
    nonempty(&frame.origin_definition, "coordinate_frame_origin")?;
    let axes = frame.axis_order.into_iter().collect::<BTreeSet<_>>();
    if axes.len() != 3 {
        return Err(TrajectoryError::UnexpectedCoordinateFrame {
            variable_id: frame.id.clone(),
        });
    }
    Ok(())
}

impl TrajectoryRecordingManifest {
    /// Canonicalize all identifier-bearing collections, then validate the
    /// immutable schema/source contract before any trajectory payload is read.
    pub fn compile(&self) -> Result<CompiledTrajectoryRecordingManifest, TrajectoryError> {
        let mut manifest = self.clone();
        nonempty(&manifest.id, "recording_manifest_id")?;
        if manifest.frame_rate.numerator == 0 || manifest.frame_rate.denominator == 0 {
            return Err(TrajectoryError::InvalidFrameRate);
        }
        if manifest.minimum_delivery_fidelity.is_zero()
            || !manifest.minimum_delivery_fidelity.is_probability()
        {
            return Err(TrajectoryError::InvalidDeliveryFidelityThreshold);
        }

        manifest
            .coordinate_frames
            .sort_by(|left, right| left.id.cmp(&right.id));
        manifest.variables.sort_by(|left, right| left.id.cmp(&right.id));
        manifest
            .source_manifests
            .sort_by(|left, right| left.id.cmp(&right.id));
        manifest
            .source_pins
            .sort_by(|left, right| left.source_manifest_id.cmp(&right.source_manifest_id));
        manifest.actions.sort_by(|left, right| left.id.cmp(&right.id));
        for variable in &mut manifest.variables {
            variable.source_manifest_ids.sort();
        }

        let mut coordinate_ids = BTreeSet::new();
        for frame in &manifest.coordinate_frames {
            validate_coordinate_frame(frame)?;
            if !coordinate_ids.insert(frame.id.clone()) {
                return Err(TrajectoryError::DuplicateIdentifier {
                    kind: "coordinate_frame_id",
                    id: frame.id.clone(),
                });
            }
        }

        let mut source_ids = BTreeSet::new();
        for source in &manifest.source_manifests {
            validate_source(source).map_err(TrajectoryError::SourceValidation)?;
            if !source_ids.insert(source.id.clone()) {
                return Err(TrajectoryError::DuplicateIdentifier {
                    kind: "source_manifest_id",
                    id: source.id.clone(),
                });
            }
        }

        let mut pins = BTreeSet::new();
        for pin in &manifest.source_pins {
            nonempty(&pin.source_manifest_id, "source_manifest_id")?;
            if !valid_sha256(&pin.expected_sha256) {
                return Err(TrajectoryError::InvalidSourcePin {
                    source_id: pin.source_manifest_id.clone(),
                });
            }
            if !pins.insert(pin.source_manifest_id.clone()) {
                return Err(TrajectoryError::DuplicateIdentifier {
                    kind: "source_digest_pin",
                    id: pin.source_manifest_id.clone(),
                });
            }
            let source = manifest
                .source_manifests
                .iter()
                .find(|source| source.id == pin.source_manifest_id)
                .ok_or_else(|| TrajectoryError::UnknownIdentifier {
                    kind: "source_manifest_id",
                    id: pin.source_manifest_id.clone(),
                })?;
            if source.sha256.as_deref() != Some(pin.expected_sha256.as_str()) {
                return Err(TrajectoryError::SourcePinMismatch {
                    source_id: pin.source_manifest_id.clone(),
                });
            }
        }
        for source in &manifest.source_manifests {
            if source.evidence_class != EvidenceClass::SyntheticFixture
                && !pins.contains(&source.id)
            {
                return Err(TrajectoryError::UnpinnedNonSyntheticSource {
                    source_id: source.id.clone(),
                });
            }
        }

        let mut variable_ids = BTreeSet::new();
        for variable in &manifest.variables {
            nonempty(&variable.id, "trajectory_variable_id")?;
            if !variable_ids.insert(variable.id.clone()) {
                return Err(TrajectoryError::DuplicateIdentifier {
                    kind: "trajectory_variable_id",
                    id: variable.id.clone(),
                });
            }
            if !unit_matches_kind(variable.kind, variable.unit) {
                return Err(TrajectoryError::InvalidVariableUnit {
                    variable_id: variable.id.clone(),
                    kind: variable.kind,
                    unit: variable.unit,
                });
            }
            if requires_coordinate_frame(variable.kind) {
                let frame_id = variable.coordinate_frame_id.as_deref().ok_or_else(|| {
                    TrajectoryError::MissingCoordinateFrame {
                        variable_id: variable.id.clone(),
                    }
                })?;
                if !coordinate_ids.contains(frame_id) {
                    return Err(TrajectoryError::UnknownIdentifier {
                        kind: "coordinate_frame_id",
                        id: frame_id.to_owned(),
                    });
                }
            } else if variable.coordinate_frame_id.is_some() {
                return Err(TrajectoryError::UnexpectedCoordinateFrame {
                    variable_id: variable.id.clone(),
                });
            }
            if requires_leg_identity(variable.kind) && variable.leg_id.is_none() {
                return Err(TrajectoryError::MissingLegIdentity {
                    variable_id: variable.id.clone(),
                });
            }
            if !requires_leg_identity(variable.kind) && variable.leg_id.is_some() {
                return Err(TrajectoryError::UnexpectedLegIdentity {
                    variable_id: variable.id.clone(),
                });
            }
            if variable.source_manifest_ids.is_empty() {
                return Err(TrajectoryError::MissingVariableSource {
                    variable_id: variable.id.clone(),
                });
            }
            let mut source_support = false;
            for source_id in &variable.source_manifest_ids {
                let source = manifest
                    .source_manifests
                    .iter()
                    .find(|source| source.id == *source_id)
                    .ok_or_else(|| TrajectoryError::UnknownIdentifier {
                        kind: "variable_source_manifest_id",
                        id: source_id.clone(),
                    })?;
                source_support |= source_supports_claim(source, variable.claim_ceiling);
            }
            if !source_support {
                return Err(TrajectoryError::InvalidClaimCeiling {
                    subject_id: variable.id.clone(),
                    claim: variable.claim_ceiling,
                });
            }
        }
        if manifest.variables.is_empty() {
            return Err(TrajectoryError::FrameSchemaMismatch { frame_index: 0 });
        }

        let mut action_ids = BTreeSet::new();
        for action in &manifest.actions {
            nonempty(&action.id, "action_id")?;
            nonempty(&action.protocol_id, "protocol_id")?;
            if action.cost_units == 0 {
                return Err(TrajectoryError::InvalidDeliveryReceipt(
                    "action has zero declared cost",
                ));
            }
            if !action_ids.insert(action.id.clone()) {
                return Err(TrajectoryError::DuplicateIdentifier {
                    kind: "action_id",
                    id: action.id.clone(),
                });
            }
            let protocol_source = manifest
                .source_manifests
                .iter()
                .find(|source| source.id == action.protocol_source_manifest_id)
                .ok_or_else(|| TrajectoryError::UnknownIdentifier {
                    kind: "protocol_source_manifest_id",
                    id: action.protocol_source_manifest_id.clone(),
                })?;
            let protocol_source_valid = matches!(
                protocol_source.kind,
                SourceKind::ExperimentalProtocol | SourceKind::SyntheticFixture
            ) || (protocol_source.kind == SourceKind::SimulationCode
                && protocol_source.evidence_class == EvidenceClass::InSilicoGenerated);
            if !protocol_source_valid {
                return Err(TrajectoryError::InvalidDeliveryReceipt(
                    "protocol source is not an experimental protocol or pinned simulation source",
                ));
            }
        }
        if manifest.actions.is_empty() {
            return Err(TrajectoryError::InvalidDeliveryReceipt(
                "recording manifest declares no actions",
            ));
        }

        Ok(CompiledTrajectoryRecordingManifest { manifest })
    }
}

impl CompiledTrajectoryRecordingManifest {
    pub fn validate_receipt(
        &self,
        receipt: &TrajectoryRunReceipt,
    ) -> Result<(), TrajectoryError> {
        nonempty(&receipt.recording_manifest_id, "recording_manifest_id")?;
        nonempty(&receipt.sample_id, "sample_id")?;
        nonempty(&receipt.session_id, "session_id")?;
        nonempty(&receipt.run_id, "run_id")?;
        nonempty(receipt.replication_unit.id(), "replication_unit_id")?;
        nonempty(&receipt.assigned_action_id, "assigned_action_id")?;
        nonempty(&receipt.delivered_protocol_id, "delivered_protocol_id")?;

        if receipt.recording_manifest_id != self.manifest.id {
            return Err(TrajectoryError::UnknownIdentifier {
                kind: "recording_manifest_id",
                id: receipt.recording_manifest_id.clone(),
            });
        }
        if !valid_sha256(&receipt.trajectory_payload_sha256) {
            return Err(TrajectoryError::InvalidTrajectoryDigest);
        }
        if !valid_sha256(&receipt.conditioning_state_digest) {
            return Err(TrajectoryError::InvalidConditioningStateDigest);
        }
        if !receipt.delivery_fidelity.is_probability()
            || receipt.delivery_fidelity < self.manifest.minimum_delivery_fidelity
        {
            return Err(TrajectoryError::InvalidDeliveryFidelity);
        }
        if !receipt.delivery_verified {
            return Err(TrajectoryError::InvalidDeliveryReceipt(
                "delivered protocol was not independently verified",
            ));
        }

        let action = self
            .manifest
            .actions
            .iter()
            .find(|action| action.id == receipt.assigned_action_id)
            .ok_or_else(|| TrajectoryError::UnknownIdentifier {
                kind: "assigned_action_id",
                id: receipt.assigned_action_id.clone(),
            })?;
        if action.protocol_id != receipt.delivered_protocol_id {
            return Err(TrajectoryError::AssignmentDeliveryMismatch {
                assigned_action_id: receipt.assigned_action_id.clone(),
                expected_protocol_id: action.protocol_id.clone(),
                delivered_protocol_id: receipt.delivered_protocol_id.clone(),
            });
        }

        let source = self
            .manifest
            .source_manifests
            .iter()
            .find(|source| source.id == receipt.source_manifest_id)
            .ok_or_else(|| TrajectoryError::UnknownIdentifier {
                kind: "trajectory_source_manifest_id",
                id: receipt.source_manifest_id.clone(),
            })?;
        if !source_supports_claim(source, receipt.claim_ceiling) {
            return Err(TrajectoryError::InvalidClaimCeiling {
                subject_id: receipt.sample_id.clone(),
                claim: receipt.claim_ceiling,
            });
        }
        if source.evidence_class == EvidenceClass::ConnectomeDerived {
            return Err(TrajectoryError::ConnectomeCannotScoreTrajectory);
        }
        for variable in &self.manifest.variables {
            if !variable
                .source_manifest_ids
                .contains(&receipt.source_manifest_id)
            {
                return Err(TrajectoryError::UnknownIdentifier {
                    kind: "trajectory_variable_source_binding",
                    id: format!("{}:{}", variable.id, receipt.source_manifest_id),
                });
            }
        }

        match source.evidence_class {
            EvidenceClass::SyntheticFixture | EvidenceClass::InSilicoGenerated => {
                if !matches!(
                    receipt.replication_unit,
                    ReplicationUnit::SimulationInstance(_)
                ) {
                    return Err(TrajectoryError::ReplicationUnitEvidenceMismatch);
                }
                let profile = receipt
                    .simulator_profile
                    .as_ref()
                    .ok_or(TrajectoryError::MissingSimulatorProfile)?;
                validate_simulator_profile(profile, &self.manifest.source_manifests)?;
            }
            EvidenceClass::PreprintMeasured | EvidenceClass::PeerReviewedMeasured => {
                if !matches!(
                    receipt.replication_unit,
                    ReplicationUnit::AnimalOrPreparation(_)
                ) {
                    return Err(TrajectoryError::ReplicationUnitEvidenceMismatch);
                }
                if receipt.simulator_profile.is_some() {
                    return Err(TrajectoryError::SimulatorProfileOnMeasuredData);
                }
            }
            EvidenceClass::ConnectomeDerived => {
                return Err(TrajectoryError::ConnectomeCannotScoreTrajectory);
            }
        }

        validate_frames(&self.manifest.variables, &receipt.frames)
    }

    /// Validate a batch without treating repeated frames or sessions as new
    /// independent biological replication units. Unique sample IDs are required;
    /// animal/preparation and session identities are deliberately retained.
    pub fn validate_batch(
        &self,
        receipts: &[TrajectoryRunReceipt],
    ) -> Result<(), TrajectoryError> {
        let mut sample_ids = BTreeSet::new();
        for receipt in receipts {
            if !sample_ids.insert(receipt.sample_id.clone()) {
                return Err(TrajectoryError::DuplicateSampleId(
                    receipt.sample_id.clone(),
                ));
            }
            self.validate_receipt(receipt)?;
        }
        Ok(())
    }

    pub fn biological_replication_units(
        &self,
        receipts: &[TrajectoryRunReceipt],
    ) -> BTreeSet<String> {
        receipts
            .iter()
            .filter_map(|receipt| match &receipt.replication_unit {
                ReplicationUnit::AnimalOrPreparation(id) => Some(id.clone()),
                ReplicationUnit::SimulationInstance(_) => None,
            })
            .collect()
    }
}

fn validate_simulator_profile(
    profile: &SimulatorProfile,
    sources: &[SourceManifest],
) -> Result<(), TrajectoryError> {
    nonempty(&profile.simulator_id, "simulator_id")?;
    nonempty(&profile.release, "simulator_release")?;
    nonempty(&profile.physics_engine, "physics_engine")?;
    nonempty(&profile.physics_engine_version, "physics_engine_version")?;
    if !valid_git_sha(&profile.code_commit_sha) {
        return Err(TrajectoryError::InvalidSimulatorProfile(
            "simulator code commit is not a 40- or 64-character hexadecimal SHA",
        ));
    }
    if profile.physics_timestep_ns == 0 || profile.control_period_ns == 0 {
        return Err(TrajectoryError::InvalidSimulatorProfile(
            "physics timestep and control period must be positive",
        ));
    }
    if !profile.solver_tolerance.is_finite() || profile.solver_tolerance <= 0.0 {
        return Err(TrajectoryError::InvalidSimulatorProfile(
            "solver tolerance must be finite and positive",
        ));
    }
    if !valid_sha256(&profile.morphology_sha256) || !valid_sha256(&profile.terrain_sha256) {
        return Err(TrajectoryError::InvalidSimulatorProfile(
            "morphology and terrain assets require SHA-256 digests",
        ));
    }
    if profile.simulator_source_manifest_ids.is_empty() {
        return Err(TrajectoryError::InvalidSimulatorProfile(
            "simulator profile lacks source manifest IDs",
        ));
    }
    for source_id in &profile.simulator_source_manifest_ids {
        let source = sources
            .iter()
            .find(|source| source.id == *source_id)
            .ok_or_else(|| TrajectoryError::UnknownIdentifier {
                kind: "simulator_source_manifest_id",
                id: source_id.clone(),
            })?;
        if source.kind != SourceKind::SimulationCode
            || source.evidence_class != EvidenceClass::InSilicoGenerated
            || source.sha256.is_none()
        {
            return Err(TrajectoryError::InvalidSimulatorProfile(
                "simulator code/assets must be source-pinned in-silico manifests",
            ));
        }
    }
    Ok(())
}

fn validate_frames(
    variables: &[TrajectoryVariable],
    frames: &[TrajectoryFrame],
) -> Result<(), TrajectoryError> {
    if frames.is_empty() {
        return Err(TrajectoryError::EmptyTrajectory);
    }
    let variable_ids = variables
        .iter()
        .map(|variable| variable.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut previous_index = None;
    let mut previous_timestamp = None;
    for frame in frames {
        if previous_index.is_some_and(|previous| frame.frame_index <= previous) {
            return Err(TrajectoryError::NonMonotonicFrameIndex);
        }
        if previous_timestamp.is_some_and(|previous| frame.timestamp_ns <= previous) {
            return Err(TrajectoryError::NonMonotonicTimestamp);
        }
        previous_index = Some(frame.frame_index);
        previous_timestamp = Some(frame.timestamp_ns);

        if frame.values.len() != variable_ids.len()
            || frame.values.keys().any(|id| !variable_ids.contains(id.as_str()))
        {
            return Err(TrajectoryError::FrameSchemaMismatch {
                frame_index: frame.frame_index,
            });
        }
        for variable in variables {
            let value = frame.values.get(&variable.id).ok_or_else(|| {
                TrajectoryError::FrameSchemaMismatch {
                    frame_index: frame.frame_index,
                }
            })?;
            match value {
                MeasurementValue::Scalar(number) => {
                    if variable.unit == MeasurementUnit::Categorical || !number.is_finite() {
                        return Err(TrajectoryError::InvalidMeasurement {
                            frame_index: frame.frame_index,
                            variable_id: variable.id.clone(),
                        });
                    }
                }
                MeasurementValue::Category(category) => {
                    if variable.unit != MeasurementUnit::Categorical || category.trim().is_empty()
                    {
                        return Err(TrajectoryError::InvalidMeasurement {
                            frame_index: frame.frame_index,
                            variable_id: variable.id.clone(),
                        });
                    }
                }
                MeasurementValue::Missing { reason } => {
                    if reason.trim().is_empty() {
                        return Err(TrajectoryError::InvalidMeasurement {
                            frame_index: frame.frame_index,
                            variable_id: variable.id.clone(),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(fill: char) -> String {
        std::iter::repeat_n(fill, 64).collect()
    }

    fn commit(fill: char) -> String {
        std::iter::repeat_n(fill, 40).collect()
    }

    fn simulation_source() -> SourceManifest {
        SourceManifest {
            id: "flygym-2.1.1-source".to_owned(),
            kind: SourceKind::SimulationCode,
            evidence_class: EvidenceClass::InSilicoGenerated,
            citation_or_source_uri: "https://github.com/NeLy-EPFL/flygym/releases/tag/v2.1.1"
                .to_owned(),
            dataset_id: Some("FlyGym-2.1.1".to_owned()),
            release_or_materialization: Some("2.1.1".to_owned()),
            sha256: Some(sha('a')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        }
    }

    fn measured_source() -> SourceManifest {
        SourceManifest {
            id: "measured-kinematics-v1".to_owned(),
            kind: SourceKind::KinematicMeasurement,
            evidence_class: EvidenceClass::PeerReviewedMeasured,
            citation_or_source_uri: "https://example.org/locked-trajectory-fixture".to_owned(),
            dataset_id: Some("synthetic-measurement-fixture".to_owned()),
            release_or_materialization: Some("fixture-v1".to_owned()),
            sha256: Some(sha('b')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        }
    }

    fn action(source_id: &str) -> InterventionAction {
        InterventionAction {
            id: "passive-front-left-probe".to_owned(),
            kind: super::super::locomotion::InterventionKind::PassiveMovementProbe,
            protocol_id: "protocol-passive-front-left".to_owned(),
            protocol_source_manifest_id: source_id.to_owned(),
            cost_units: 1,
            target_leg: Some(LegId::FrontLeft),
            presynaptic_inhibition_target: None,
        }
    }

    fn variables(source_id: &str, claim: ClaimCeiling) -> Vec<TrajectoryVariable> {
        vec![
            TrajectoryVariable {
                id: "front-left-joint-angle".to_owned(),
                kind: MeasurementKind::JointAngle,
                unit: MeasurementUnit::Radian,
                coordinate_frame_id: Some("front-left-joint-frame".to_owned()),
                leg_id: Some(LegId::FrontLeft),
                source_manifest_ids: vec![source_id.to_owned()],
                claim_ceiling: claim,
            },
            TrajectoryVariable {
                id: "front-left-contact".to_owned(),
                kind: MeasurementKind::ContactState,
                unit: MeasurementUnit::Categorical,
                coordinate_frame_id: None,
                leg_id: Some(LegId::FrontLeft),
                source_manifest_ids: vec![source_id.to_owned()],
                claim_ceiling: claim,
            },
            TrajectoryVariable {
                id: "whole-body-gait-phase".to_owned(),
                kind: MeasurementKind::GaitPhase,
                unit: MeasurementUnit::Categorical,
                coordinate_frame_id: None,
                leg_id: None,
                source_manifest_ids: vec![source_id.to_owned()],
                claim_ceiling: claim,
            },
        ]
    }

    fn recording_manifest(source: SourceManifest, claim: ClaimCeiling) -> TrajectoryRecordingManifest {
        let source_id = source.id.clone();
        let pins = source.sha256.as_ref().map(|sha256| {
            vec![SourceDigestPin {
                source_manifest_id: source_id.clone(),
                expected_sha256: sha256.clone(),
            }]
        }).unwrap_or_default();
        TrajectoryRecordingManifest {
            id: "flygym-front-left-walk-v1".to_owned(),
            frame_rate: FrameRate { numerator: 60, denominator: 1 },
            coordinate_frames: vec![CoordinateFrame {
                id: "front-left-joint-frame".to_owned(),
                handedness: CoordinateHandedness::RightHanded,
                axis_order: [AxisLabel::X, AxisLabel::Y, AxisLabel::Z],
                origin_definition: "front-left joint centre".to_owned(),
            }],
            variables: variables(&source_id, claim),
            source_manifests: vec![source],
            source_pins: pins,
            actions: vec![action(&source_id)],
            minimum_delivery_fidelity: Rational::new(9, 10).unwrap(),
        }
    }

    fn simulator_profile(source_id: &str) -> SimulatorProfile {
        SimulatorProfile {
            simulator_id: "FlyGym".to_owned(),
            release: "2.1.1".to_owned(),
            code_commit_sha: commit('c'),
            physics_engine: "MuJoCo".to_owned(),
            physics_engine_version: "3.16".to_owned(),
            physics_timestep_ns: 1_000_000,
            control_period_ns: 10_000_000,
            solver_tolerance: 1.0e-5,
            morphology_sha256: sha('d'),
            terrain_sha256: sha('e'),
            simulator_source_manifest_ids: vec![source_id.to_owned()],
        }
    }

    fn frame(index: u64, timestamp_ns: u64, angle: f64) -> TrajectoryFrame {
        TrajectoryFrame {
            frame_index: index,
            timestamp_ns,
            values: BTreeMap::from([
                ("front-left-joint-angle".to_owned(), MeasurementValue::Scalar(angle)),
                (
                    "front-left-contact".to_owned(),
                    MeasurementValue::Category("loaded".to_owned()),
                ),
                (
                    "whole-body-gait-phase".to_owned(),
                    MeasurementValue::Category("stance".to_owned()),
                ),
            ]),
        }
    }

    fn simulation_receipt() -> TrajectoryRunReceipt {
        TrajectoryRunReceipt {
            recording_manifest_id: "flygym-front-left-walk-v1".to_owned(),
            sample_id: "sim-sample-1".to_owned(),
            session_id: "rollout-1".to_owned(),
            run_id: "run-1".to_owned(),
            replication_unit: ReplicationUnit::SimulationInstance("flygym-world-001".to_owned()),
            source_manifest_id: "flygym-2.1.1-source".to_owned(),
            trajectory_payload_sha256: sha('f'),
            claim_ceiling: ClaimCeiling::InSilicoReproduction,
            assigned_action_id: "passive-front-left-probe".to_owned(),
            delivered_protocol_id: "protocol-passive-front-left".to_owned(),
            delivery_verified: true,
            delivery_fidelity: Rational::new(19, 20).unwrap(),
            conditioning_state_digest: sha('1'),
            simulator_profile: Some(simulator_profile("flygym-2.1.1-source")),
            frames: vec![frame(0, 10_000_000, 0.1), frame(1, 20_000_000, 0.2)],
        }
    }

    #[test]
    fn simulator_trajectory_receipt_compiles_without_biological_replication_credit() {
        let contract = recording_manifest(
            simulation_source(),
            ClaimCeiling::InSilicoReproduction,
        )
        .compile()
        .unwrap();
        let receipt = simulation_receipt();
        contract.validate_receipt(&receipt).unwrap();
        assert!(contract.biological_replication_units(std::slice::from_ref(&receipt)).is_empty());
    }

    #[test]
    fn measured_trajectory_requires_animal_or_preparation_identity() {
        let contract = recording_manifest(
            measured_source(),
            ClaimCeiling::PeerReviewedMeasuredBehaviourReproduction,
        )
        .compile()
        .unwrap();
        let mut receipt = simulation_receipt();
        receipt.source_manifest_id = "measured-kinematics-v1".to_owned();
        receipt.claim_ceiling = ClaimCeiling::PeerReviewedMeasuredBehaviourReproduction;
        receipt.replication_unit = ReplicationUnit::SimulationInstance("sim-only".to_owned());
        receipt.simulator_profile = None;
        assert_eq!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::ReplicationUnitEvidenceMismatch)
        );
        receipt.replication_unit =
            ReplicationUnit::AnimalOrPreparation("animal-or-preparation-1".to_owned());
        assert!(contract.validate_receipt(&receipt).is_ok());
    }

    #[test]
    fn simulator_only_source_cannot_support_measured_behaviour_claim() {
        let contract = recording_manifest(
            simulation_source(),
            ClaimCeiling::PeerReviewedMeasuredBehaviourReproduction,
        );
        assert!(matches!(
            contract.compile(),
            Err(TrajectoryError::InvalidClaimCeiling { .. })
        ));
    }

    #[test]
    fn duplicate_sample_ids_are_rejected_but_distinct_samples_keep_the_same_cluster() {
        let contract = recording_manifest(
            measured_source(),
            ClaimCeiling::PeerReviewedMeasuredBehaviourReproduction,
        ).compile().unwrap();
        let mut first = simulation_receipt();
        first.source_manifest_id = "measured-kinematics-v1".to_owned();
        first.claim_ceiling = ClaimCeiling::PeerReviewedMeasuredBehaviourReproduction;
        first.replication_unit = ReplicationUnit::AnimalOrPreparation("animal-1".to_owned());
        first.simulator_profile = None;
        let mut second = first.clone();
        second.sample_id = "measured-sample-2".to_owned();
        second.run_id = "run-2".to_owned();
        assert!(contract.validate_batch(&[first.clone(), second.clone()]).is_ok());
        assert_eq!(
            contract.validate_batch(&[first.clone(), first]),
            Err(TrajectoryError::DuplicateSampleId("sim-sample-1".to_owned()))
        );
        assert_eq!(
            contract.biological_replication_units(&[second]),
            BTreeSet::from(["animal-1".to_owned()])
        );
    }

    #[test]
    fn delivered_protocol_mismatch_and_low_fidelity_are_rejected() {
        let contract = recording_manifest(
            simulation_source(),
            ClaimCeiling::InSilicoReproduction,
        ).compile().unwrap();
        let mut receipt = simulation_receipt();
        receipt.delivered_protocol_id = "a-different-protocol".to_owned();
        assert!(matches!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::AssignmentDeliveryMismatch { .. })
        ));
        receipt.delivered_protocol_id = "protocol-passive-front-left".to_owned();
        receipt.delivery_fidelity = Rational::new(4, 5).unwrap();
        assert_eq!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::InvalidDeliveryFidelity)
        );
    }

    #[test]
    fn invalid_timestamp_order_and_duplicate_frame_indices_are_rejected() {
        let contract = recording_manifest(
            simulation_source(),
            ClaimCeiling::InSilicoReproduction,
        ).compile().unwrap();
        let mut receipt = simulation_receipt();
        receipt.frames = vec![frame(0, 20_000_000, 0.1), frame(1, 10_000_000, 0.2)];
        assert_eq!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::NonMonotonicTimestamp)
        );
        receipt.frames = vec![frame(1, 10_000_000, 0.1), frame(1, 20_000_000, 0.2)];
        assert_eq!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::NonMonotonicFrameIndex)
        );
    }

    #[test]
    fn unknown_fields_and_implicit_missing_values_are_rejected() {
        let contract = recording_manifest(
            simulation_source(),
            ClaimCeiling::InSilicoReproduction,
        ).compile().unwrap();
        let mut receipt = simulation_receipt();
        receipt.frames[0].values.remove("front-left-contact");
        assert_eq!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::FrameSchemaMismatch { frame_index: 0 })
        );
        receipt.frames[0].values.insert(
            "front-left-contact".to_owned(),
            MeasurementValue::Missing {
                reason: "contact sensor dropped this frame".to_owned(),
            },
        );
        assert!(contract.validate_receipt(&receipt).is_ok());
    }

    #[test]
    fn empty_missingness_reason_and_nonfinite_measurements_are_rejected() {
        let contract = recording_manifest(
            simulation_source(),
            ClaimCeiling::InSilicoReproduction,
        ).compile().unwrap();
        let mut receipt = simulation_receipt();
        receipt.frames[0].values.insert(
            "front-left-contact".to_owned(),
            MeasurementValue::Missing {
                reason: "  ".to_owned(),
            },
        );
        assert!(matches!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::InvalidMeasurement { .. })
        ));
        receipt.frames[0].values.insert(
            "front-left-joint-angle".to_owned(),
            MeasurementValue::Scalar(f64::NAN),
        );
        assert!(matches!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::InvalidMeasurement { .. })
        ));
    }

    #[test]
    fn source_payload_and_history_digests_are_required_to_be_sha256_shaped() {
        let contract = recording_manifest(
            simulation_source(),
            ClaimCeiling::InSilicoReproduction,
        ).compile().unwrap();
        let mut receipt = simulation_receipt();
        receipt.trajectory_payload_sha256 = "md5-not-sha256".to_owned();
        assert_eq!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::InvalidTrajectoryDigest)
        );
        receipt.trajectory_payload_sha256 = sha('f');
        receipt.conditioning_state_digest = "unknown-history".to_owned();
        assert_eq!(
            contract.validate_receipt(&receipt),
            Err(TrajectoryError::InvalidConditioningStateDigest)
        );
    }

    #[test]
    fn source_digest_pin_drift_fails_manifest_compilation() {
        let mut manifest = recording_manifest(
            simulation_source(),
            ClaimCeiling::InSilicoReproduction,
        );
        manifest.source_pins[0].expected_sha256 = sha('9');
        assert!(matches!(
            manifest.compile(),
            Err(TrajectoryError::SourcePinMismatch { .. })
        ));
    }

    #[test]
    fn storage_order_permutations_compile_to_identical_trajectory_contracts() {
        let original = recording_manifest(
            simulation_source(),
            ClaimCeiling::InSilicoReproduction,
        ).compile().unwrap();
        let mut reordered = recording_manifest(
            simulation_source(),
            ClaimCeiling::InSilicoReproduction,
        );
        reordered.variables.reverse();
        reordered.source_manifests.reverse();
        reordered.source_pins.reverse();
        reordered.actions.reverse();
        reordered.coordinate_frames.reverse();
        let reordered = reordered.compile().unwrap();
        assert_eq!(original, reordered);
    }
}
