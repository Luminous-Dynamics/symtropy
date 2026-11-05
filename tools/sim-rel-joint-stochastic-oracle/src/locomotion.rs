//! Source/evidence-aware locomotion adapter for the joint stochastic oracle.
//!
//! The adapter compiles stable locomotion-domain identifiers into the exact
//! finite joint kernel over mechanism, physical state, sensor state, action,
//! observation and hazard. It is a schema/adapter layer: it does not infer
//! transition probabilities from anatomy or claim biological validation.

use std::collections::{BTreeMap, BTreeSet};

use super::{JointKernel, JointMass, JointOutcome, KernelRow, Rational};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SourceKind {
    SyntheticFixture,
    SimulationCode,
    Connectome,
    NeuralMeasurement,
    KinematicMeasurement,
    BehaviouralMeasurement,
    ExperimentalProtocol,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EvidenceClass {
    SyntheticFixture,
    InSilicoGenerated,
    ConnectomeDerived,
    PreprintMeasured,
    PeerReviewedMeasured,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EndpointRole {
    Presynaptic,
    Postsynaptic,
    UnknownSide,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct UnknownEndpoint {
    pub raw_endpoint_id: String,
    pub role: EndpointRole,
    pub missingness_reason: String,
}

/// Source identity is separate from model identity and from causal evidence.
/// A connectome-derived source supports anatomical constraints, not a measured
/// causal effect. A source digest is a declared SHA-256 pin; the acquisition
/// pipeline must verify the actual bytes against it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceManifest {
    pub id: String,
    pub kind: SourceKind,
    pub evidence_class: EvidenceClass,
    pub citation_or_source_uri: String,
    pub dataset_id: Option<String>,
    /// Exact release, materialization, or immutable archive version.
    pub release_or_materialization: Option<String>,
    pub sha256: Option<String>,
    /// For connectome sources, the caller must state that unknown endpoints
    /// have been inventoried, even when the list is empty.
    pub unknown_endpoint_inventory_complete: bool,
    pub unknown_endpoints: Vec<UnknownEndpoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDigestPin {
    pub source_manifest_id: String,
    pub expected_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MechanismFamily {
    CentralRhythmGenerator,
    DistributedLocalSensorimotorLoops,
    CentralPlusLocalHybrid,
    ReflexOnly,
    MechanicsOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ClaimCeiling {
    SyntheticQualification,
    InSilicoReproduction,
    AnatomicalConnectomeConstrained,
    PreprintMeasuredNeuralReproduction,
    PeerReviewedMeasuredNeuralReproduction,
    PeerReviewedMeasuredBehaviourReproduction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MechanismHypothesis {
    pub id: String,
    pub family: MechanismFamily,
    pub source_manifest_ids: Vec<String>,
    pub claim_ceiling: ClaimCeiling,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum GaitPhase {
    Stance,
    Swing,
    Transition,
    Uncoordinated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BehavioralContext {
    Walking,
    Grooming,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ContactState {
    Supported,
    Slipping,
    Unloaded,
    Airborne,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LoadState {
    Reduced,
    Nominal,
    Elevated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PhysicalCarryover {
    Baseline,
    Loaded,
    Fatigued,
    Adapted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum JointPositionBin {
    Flexed,
    Neutral,
    Extended,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum JointVelocityBin {
    Retracting,
    Stationary,
    Protracting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ActuatorForceBin {
    Low,
    Nominal,
    High,
}

/// Bounded, typed mechanical state for one anatomical leg. These bins are
/// benchmark abstractions, not calibrated physical units.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LegMechanicalState {
    pub gait_phase: GaitPhase,
    pub contact_state: ContactState,
    pub load_state: LoadState,
    pub joint_position: JointPositionBin,
    pub joint_velocity: JointVelocityBin,
    pub actuator_force: ActuatorForceBin,
}

pub const fn leg_mechanics(
    gait_phase: GaitPhase,
    contact_state: ContactState,
    load_state: LoadState,
    joint_position: JointPositionBin,
    joint_velocity: JointVelocityBin,
    actuator_force: ActuatorForceBin,
) -> LegMechanicalState {
    LegMechanicalState {
        gait_phase,
        contact_state,
        load_state,
        joint_position,
        joint_velocity,
        actuator_force,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalState {
    pub id: String,
    pub behavioral_context: BehavioralContext,
    /// Whole-body scenario labels retained for grouping and compatibility.
    /// Per-leg mechanics in the legs field are authoritative for limb-level dynamics.
    pub gait_phase: GaitPhase,
    pub contact_state: ContactState,
    pub load_state: LoadState,
    pub carryover: PhysicalCarryover,
    pub legs: [LegMechanicalState; 6],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ChannelState {
    Intact,
    Gated,
    Attenuated,
    Stale,
    Missing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AcquisitionState {
    Calibrated,
    PositiveOffset,
    NegativeOffset,
    Drifting,
    Noisy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PresynapticMovementInhibition {
    Disinhibited,
    Engaged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LegId {
    FrontLeft,
    FrontRight,
    MiddleLeft,
    MiddleRight,
    HindLeft,
    HindRight,
}

impl LegId {
    pub const ALL: [Self; 6] = [
        Self::FrontLeft,
        Self::FrontRight,
        Self::MiddleLeft,
        Self::MiddleRight,
        Self::HindLeft,
        Self::HindRight,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::FrontLeft => 0,
            Self::FrontRight => 1,
            Self::MiddleLeft => 2,
            Self::MiddleRight => 3,
            Self::HindLeft => 4,
            Self::HindRight => 5,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SensorState {
    pub id: String,
    /// Position-encoding FeCO afferent channel (claw-like).
    pub position_afferent: ChannelState,
    /// Movement-encoding FeCO afferent channel (hook-like).
    pub movement_afferent: ChannelState,
    /// Vibration-sensitive afferent channel (club-like).
    pub vibration_afferent: ChannelState,
    pub contact_feedback: ChannelState,
    /// Acquisition/instrument state is distinct from biological afferent gain.
    pub acquisition: AcquisitionState,
    /// Neural presynaptic inhibition of movement-encoding FeCO axons,
    /// represented per leg because descending modulation is leg-specific.
    /// This is distinct from axon loss/gating and acquisition faults.
    pub presynaptic_movement_inhibition: [PresynapticMovementInhibition; 6],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum InterventionKind {
    ActiveMovementProbe,
    PassiveMovementProbe,
    PositionAfferentPerturbation,
    MovementAfferentGate,
    VibrationAfferentPerturbation,
    DescendingPathwayModulation,
    ContactLoadPerturbation,
    AcquisitionReferenceCalibration,
    Washout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterventionAction {
    pub id: String,
    pub kind: InterventionKind,
    pub protocol_id: String,
    pub protocol_source_manifest_id: String,
    pub cost_units: u64,
    /// Required for active/passive movement probes and descending modulation.
    pub target_leg: Option<LegId>,
    /// Required only for DescendingPathwayModulation; identifies the intended
    /// presynaptic inhibition state, not a generic sensory-channel gate.
    pub presynaptic_inhibition_target: Option<PresynapticMovementInhibition>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ObservationKind {
    NeuralMotorRhythm,
    PositionFeedback,
    MovementFeedback,
    VibrationResponse,
    JointCorrection,
    InterlegCoordination,
    ContactTransition,
    AcquisitionReference,
    WashoutResponse,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    pub id: String,
    pub kind: ObservationKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialJointMass {
    pub hypothesis_id: String,
    pub physical_state_id: String,
    pub sensor_state_id: String,
    pub mass: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionOutcome {
    pub probability: Rational,
    pub next_physical_state_id: String,
    pub next_sensor_state_id: String,
    /// None is a campaign hazard/terminal event in the joint oracle.
    pub observation_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointTransitionRow {
    pub hypothesis_id: String,
    pub physical_state_id: String,
    pub sensor_state_id: String,
    pub action_id: String,
    /// Sources supporting this row's transition likelihood, not just its
    /// mechanism label or intervention protocol.
    pub source_manifest_ids: Vec<String>,
    pub claim_ceiling: ClaimCeiling,
    pub outcomes: Vec<TransitionOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocomotionKernelSpec {
    pub source_manifests: Vec<SourceManifest>,
    pub source_pins: Vec<SourceDigestPin>,
    pub hypotheses: Vec<MechanismHypothesis>,
    pub physical_states: Vec<PhysicalState>,
    pub sensor_states: Vec<SensorState>,
    pub actions: Vec<InterventionAction>,
    pub observations: Vec<Observation>,
    pub initial_belief: Vec<InitialJointMass>,
    pub transition_rows: Vec<JointTransitionRow>,
    pub cost_unit_id: String,
    /// Pre-registered minimum delivered-protocol fidelity, as an exact probability.
    pub minimum_delivery_fidelity: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledLocomotionKernel {
    pub kernel: JointKernel,
    pub source_manifests: Vec<SourceManifest>,
    pub hypotheses: Vec<MechanismHypothesis>,
    pub physical_states: Vec<PhysicalState>,
    pub sensor_states: Vec<SensorState>,
    pub actions: Vec<InterventionAction>,
    pub observations: Vec<Observation>,
    /// Canonically ordered source-bearing rows used to compile the kernel.
    pub transition_rows: Vec<JointTransitionRow>,
    pub cost_unit_id: String,
    pub minimum_delivery_fidelity: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdapterError {
    EmptyIdentifier(&'static str),
    DuplicateIdentifier {
        kind: &'static str,
        id: String,
    },
    UnknownIdentifier {
        kind: &'static str,
        id: String,
    },
    InvalidSha256 {
        source_id: String,
    },
    SensorSemanticIdMismatch {
        sensor_state_id: String,
    },
    SourcePinMismatch {
        source_id: String,
    },
    UnpinnedNonSyntheticSource {
        source_id: String,
    },
    MissingConnectomeSnapshot {
        source_id: String,
    },
    MissingMeasurementSnapshot {
        source_id: String,
    },
    IncompleteUnknownEndpointInventory {
        source_id: String,
    },
    InvalidUnknownEndpoint {
        source_id: String,
        raw_endpoint_id: String,
    },
    InvalidEvidenceClaim {
        hypothesis_id: String,
        claim: ClaimCeiling,
    },
    MissingHypothesisSource {
        hypothesis_id: String,
    },
    MissingTransitionSource {
        hypothesis_id: String,
        action_id: String,
    },
    DuplicateTransitionSource {
        hypothesis_id: String,
        action_id: String,
        source_id: String,
    },
    MissingProtocolSource {
        action_id: String,
    },
    InvalidActionCost {
        action_id: String,
    },
    ActionSemanticsViolation {
        action_id: String,
        reason: String,
    },
    InvalidPresynapticInterventionTarget {
        action_id: String,
    },
    InconsistentCostUnits,
    IncompleteTransitionRow {
        hypothesis_id: String,
        physical_state_id: String,
        sensor_state_id: String,
        action_id: String,
    },
    DuplicateTransitionRow {
        hypothesis_id: String,
        physical_state_id: String,
        sensor_state_id: String,
        action_id: String,
    },
    InvalidKernel(String),
    InvalidTrialReceipt(&'static str),
    InvalidDeliveryFidelityThreshold,
    InvalidConditioningStateDigest,
    DuplicateSampleId(String),
    AssignmentDeliveryMismatch {
        assigned_action_id: String,
        expected_protocol_id: String,
        delivered_protocol_id: String,
    },
    EvidenceSourceMismatch {
        source_id: String,
        declared: EvidenceClass,
        source: EvidenceClass,
    },
    AnatomicalEvidenceCannotScoreTrial,
    UnknownAssignedAction(String),
}

fn nonempty(value: &str, field: &'static str) -> Result<(), AdapterError> {
    if value.trim().is_empty() {
        Err(AdapterError::EmptyIdentifier(field))
    } else {
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn index_ids<T, F>(
    values: &[T],
    kind: &'static str,
    id_of: F,
) -> Result<BTreeMap<String, usize>, AdapterError>
where
    F: Fn(&T) -> &str,
{
    let mut result = BTreeMap::new();
    for (index, value) in values.iter().enumerate() {
        let id = id_of(value);
        nonempty(id, kind)?;
        if result.insert(id.to_owned(), index).is_some() {
            return Err(AdapterError::DuplicateIdentifier {
                kind,
                id: id.to_owned(),
            });
        }
    }
    Ok(result)
}

fn lookup(
    indices: &BTreeMap<String, usize>,
    kind: &'static str,
    id: &str,
) -> Result<usize, AdapterError> {
    indices
        .get(id)
        .copied()
        .ok_or_else(|| AdapterError::UnknownIdentifier {
            kind,
            id: id.to_owned(),
        })
}

fn validate_source(source: &SourceManifest) -> Result<(), AdapterError> {
    nonempty(&source.id, "source_manifest_id")?;
    nonempty(&source.citation_or_source_uri, "citation_or_source_uri")?;

    if source.kind == SourceKind::Connectome {
        if source.dataset_id.as_deref().is_none_or(str::is_empty)
            || source
                .release_or_materialization
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(AdapterError::MissingConnectomeSnapshot {
                source_id: source.id.clone(),
            });
        }
        if !source.unknown_endpoint_inventory_complete {
            return Err(AdapterError::IncompleteUnknownEndpointInventory {
                source_id: source.id.clone(),
            });
        }
        if source.evidence_class != EvidenceClass::ConnectomeDerived {
            return Err(AdapterError::InvalidEvidenceClaim {
                hypothesis_id: source.id.clone(),
                claim: ClaimCeiling::AnatomicalConnectomeConstrained,
            });
        }
    }

    match source.evidence_class {
        EvidenceClass::SyntheticFixture => {}
        EvidenceClass::InSilicoGenerated => {
            if source.kind != SourceKind::SimulationCode {
                return Err(AdapterError::InvalidEvidenceClaim {
                    hypothesis_id: source.id.clone(),
                    claim: ClaimCeiling::InSilicoReproduction,
                });
            }
        }
        EvidenceClass::ConnectomeDerived => {
            if source.kind != SourceKind::Connectome {
                return Err(AdapterError::InvalidEvidenceClaim {
                    hypothesis_id: source.id.clone(),
                    claim: ClaimCeiling::AnatomicalConnectomeConstrained,
                });
            }
        }
        EvidenceClass::PreprintMeasured | EvidenceClass::PeerReviewedMeasured => {
            if source.dataset_id.as_deref().is_none_or(str::is_empty)
                || source
                    .release_or_materialization
                    .as_deref()
                    .is_none_or(str::is_empty)
            {
                return Err(AdapterError::MissingMeasurementSnapshot {
                    source_id: source.id.clone(),
                });
            }
            if !matches!(
                source.kind,
                SourceKind::NeuralMeasurement
                    | SourceKind::KinematicMeasurement
                    | SourceKind::BehaviouralMeasurement
            ) {
                return Err(AdapterError::InvalidEvidenceClaim {
                    hypothesis_id: source.id.clone(),
                    claim: if source.evidence_class == EvidenceClass::PreprintMeasured {
                        ClaimCeiling::PreprintMeasuredNeuralReproduction
                    } else {
                        ClaimCeiling::PeerReviewedMeasuredBehaviourReproduction
                    },
                });
            }
        }
    }

    if let Some(digest) = &source.sha256 {
        if !valid_sha256(digest) {
            return Err(AdapterError::InvalidSha256 {
                source_id: source.id.clone(),
            });
        }
    } else if source.evidence_class != EvidenceClass::SyntheticFixture {
        return Err(AdapterError::InvalidSha256 {
            source_id: source.id.clone(),
        });
    }

    let mut endpoint_ids = BTreeSet::new();
    for endpoint in &source.unknown_endpoints {
        nonempty(&endpoint.raw_endpoint_id, "unknown_raw_endpoint_id")?;
        nonempty(
            &endpoint.missingness_reason,
            "unknown_endpoint_missingness_reason",
        )?;
        if !endpoint_ids.insert(endpoint.raw_endpoint_id.clone()) {
            return Err(AdapterError::InvalidUnknownEndpoint {
                source_id: source.id.clone(),
                raw_endpoint_id: endpoint.raw_endpoint_id.clone(),
            });
        }
    }
    Ok(())
}

fn source_supports_claim(source: &SourceManifest, claim: ClaimCeiling) -> bool {
    match claim {
        ClaimCeiling::SyntheticQualification => {
            source.evidence_class == EvidenceClass::SyntheticFixture
        }
        ClaimCeiling::InSilicoReproduction => {
            source.evidence_class == EvidenceClass::InSilicoGenerated
        }
        ClaimCeiling::AnatomicalConnectomeConstrained => {
            source.kind == SourceKind::Connectome
                && source.evidence_class == EvidenceClass::ConnectomeDerived
        }
        ClaimCeiling::PreprintMeasuredNeuralReproduction => {
            source.kind == SourceKind::NeuralMeasurement
                && source.evidence_class == EvidenceClass::PreprintMeasured
        }
        ClaimCeiling::PeerReviewedMeasuredNeuralReproduction => {
            source.kind == SourceKind::NeuralMeasurement
                && source.evidence_class == EvidenceClass::PeerReviewedMeasured
        }
        ClaimCeiling::PeerReviewedMeasuredBehaviourReproduction => {
            matches!(
                source.kind,
                SourceKind::KinematicMeasurement | SourceKind::BehaviouralMeasurement
            ) && source.evidence_class == EvidenceClass::PeerReviewedMeasured
        }
    }
}

fn sensor_state_semantic_id(sensor: &SensorState) -> String {
    format!(
        "p{:?}-m{:?}-v{:?}-c{:?}-a{:?}-i{:?}",
        sensor.position_afferent,
        sensor.movement_afferent,
        sensor.vibration_afferent,
        sensor.contact_feedback,
        sensor.acquisition,
        sensor.presynaptic_movement_inhibition
    )
}

fn presynaptic_legs_except_unchanged(
    current: &SensorState,
    next: &SensorState,
    target_leg: LegId,
) -> bool {
    LegId::ALL
        .into_iter()
        .filter(|leg| *leg != target_leg)
        .all(|leg| {
            current.presynaptic_movement_inhibition[leg.index()]
                == next.presynaptic_movement_inhibition[leg.index()]
        })
}

fn sensor_channels_unchanged(current: &SensorState, next: &SensorState) -> bool {
    current.position_afferent == next.position_afferent
        && current.movement_afferent == next.movement_afferent
        && current.vibration_afferent == next.vibration_afferent
        && current.contact_feedback == next.contact_feedback
        && current.acquisition == next.acquisition
        && current.presynaptic_movement_inhibition == next.presynaptic_movement_inhibition
}

fn sensor_afferent_channels_unchanged(current: &SensorState, next: &SensorState) -> bool {
    current.position_afferent == next.position_afferent
        && current.movement_afferent == next.movement_afferent
        && current.vibration_afferent == next.vibration_afferent
        && current.contact_feedback == next.contact_feedback
}

fn sensor_modalities_unchanged(current: &SensorState, next: &SensorState) -> bool {
    current.position_afferent == next.position_afferent
        && current.movement_afferent == next.movement_afferent
        && current.vibration_afferent == next.vibration_afferent
        && current.contact_feedback == next.contact_feedback
        && current.presynaptic_movement_inhibition == next.presynaptic_movement_inhibition
}

fn validate_action_transition_semantics(
    action: &InterventionAction,
    current_physical: &PhysicalState,
    current_sensor: &SensorState,
    next_physical: &PhysicalState,
    next_sensor: &SensorState,
) -> Result<(), AdapterError> {
    let preserve_non_target_channels =
        |position_target: bool, movement_target: bool, vibration_target: bool| {
            (position_target || current_sensor.position_afferent == next_sensor.position_afferent)
                && (movement_target
                    || current_sensor.movement_afferent == next_sensor.movement_afferent)
                && (vibration_target
                    || current_sensor.vibration_afferent == next_sensor.vibration_afferent)
                && current_sensor.contact_feedback == next_sensor.contact_feedback
                && current_sensor.acquisition == next_sensor.acquisition
                && current_sensor.presynaptic_movement_inhibition
                    == next_sensor.presynaptic_movement_inhibition
        };

    let reason = match action.kind {
        InterventionKind::PositionAfferentPerturbation => {
            let perturbed = matches!(
                next_sensor.position_afferent,
                ChannelState::Gated | ChannelState::Attenuated
            );
            (!perturbed || !preserve_non_target_channels(true, false, false))
                .then_some("position perturbation must affect the position channel only")
        }
        InterventionKind::MovementAfferentGate => {
            (next_sensor.movement_afferent != ChannelState::Gated
                || !preserve_non_target_channels(false, true, false))
            .then_some("movement gate must gate movement feedback only")
        }
        InterventionKind::VibrationAfferentPerturbation => {
            let perturbed = matches!(
                next_sensor.vibration_afferent,
                ChannelState::Gated | ChannelState::Attenuated
            );
            (!perturbed || !preserve_non_target_channels(false, false, true))
                .then_some("vibration perturbation must affect the vibration channel only")
        }
        InterventionKind::AcquisitionReferenceCalibration => {
            (!sensor_modalities_unchanged(current_sensor, next_sensor)
                || next_sensor.acquisition != AcquisitionState::Calibrated
                || current_physical != next_physical)
                .then_some(
                    "reference calibration must change acquisition only and preserve physical state",
                )
        }
        InterventionKind::ContactLoadPerturbation => {
            (!sensor_channels_unchanged(current_sensor, next_sensor)
                || (current_physical.load_state == next_physical.load_state
                    && current_physical.contact_state == next_physical.contact_state))
            .then_some(
                "contact/load perturbation must change contact or load without changing sensor state",
            )
        }
        InterventionKind::Washout => {
            (next_physical.carryover != PhysicalCarryover::Baseline
                || !sensor_channels_unchanged(current_sensor, next_sensor))
            .then_some("washout must clear physical carryover while preserving sensor state")
        }
        InterventionKind::ActiveMovementProbe => {
            let expected = match current_physical.behavioral_context {
                BehavioralContext::Walking | BehavioralContext::Grooming => {
                    PresynapticMovementInhibition::Engaged
                }
                BehavioralContext::Other => PresynapticMovementInhibition::Disinhibited,
            };
            let target_matches = action.target_leg.is_some_and(|leg| {
                next_sensor.presynaptic_movement_inhibition[leg.index()] == expected
                    && presynaptic_legs_except_unchanged(current_sensor, next_sensor, leg)
            });
            (!target_matches
                || !sensor_afferent_channels_unchanged(current_sensor, next_sensor)
                || current_sensor.acquisition != next_sensor.acquisition)
            .then_some(
                "active probe must apply context-specific presynaptic inhibition to its target leg only",
            )
        }
        InterventionKind::PassiveMovementProbe => {
            let target_is_disinhibited = action.target_leg.is_some_and(|leg| {
                next_sensor.presynaptic_movement_inhibition[leg.index()]
                    == PresynapticMovementInhibition::Disinhibited
                    && presynaptic_legs_except_unchanged(current_sensor, next_sensor, leg)
            });
            (!target_is_disinhibited
                || !sensor_afferent_channels_unchanged(current_sensor, next_sensor)
                || current_sensor.acquisition != next_sensor.acquisition)
            .then_some(
                "passive movement must leave its target leg presynaptically disinhibited and preserve other legs",
            )
        }
        InterventionKind::DescendingPathwayModulation => {
            let target_matches = action.target_leg.is_some_and(|leg| {
                action.presynaptic_inhibition_target
                    == Some(next_sensor.presynaptic_movement_inhibition[leg.index()])
                    && presynaptic_legs_except_unchanged(current_sensor, next_sensor, leg)
            });
            (!target_matches
                || !sensor_afferent_channels_unchanged(current_sensor, next_sensor)
                || current_sensor.acquisition != next_sensor.acquisition)
            .then_some(
                "descending modulation must set its declared presynaptic target leg and state only",
            )
        }
    };

    if let Some(reason) = reason {
        return Err(AdapterError::ActionSemanticsViolation {
            action_id: action.id.clone(),
            reason: reason.to_owned(),
        });
    }
    Ok(())
}

impl LocomotionKernelSpec {
    /// Compile independent of caller storage order. Stable IDs define canonical
    /// indices so reordered but semantically identical manifests yield the same
    /// exact kernel and provenance object.
    pub fn compile(&self) -> Result<CompiledLocomotionKernel, AdapterError> {
        let mut canonical = self.clone();
        canonical.sort_by_semantic_id();
        canonical.compile_ordered()
    }

    fn sort_by_semantic_id(&mut self) {
        self.source_manifests
            .sort_by(|left, right| left.id.cmp(&right.id));
        for source in &mut self.source_manifests {
            source.unknown_endpoints.sort();
        }
        self.source_pins
            .sort_by(|left, right| left.source_manifest_id.cmp(&right.source_manifest_id));
        self.hypotheses
            .sort_by(|left, right| left.id.cmp(&right.id));
        for hypothesis in &mut self.hypotheses {
            hypothesis.source_manifest_ids.sort();
        }
        for row in &mut self.transition_rows {
            row.source_manifest_ids.sort();
        }
        self.physical_states
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.sensor_states
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.actions.sort_by(|left, right| left.id.cmp(&right.id));
        self.observations
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.initial_belief.sort_by(|left, right| {
            (
                &left.hypothesis_id,
                &left.physical_state_id,
                &left.sensor_state_id,
            )
                .cmp(&(
                    &right.hypothesis_id,
                    &right.physical_state_id,
                    &right.sensor_state_id,
                ))
        });
        self.transition_rows.sort_by(|left, right| {
            (
                &left.hypothesis_id,
                &left.physical_state_id,
                &left.sensor_state_id,
                &left.action_id,
            )
                .cmp(&(
                    &right.hypothesis_id,
                    &right.physical_state_id,
                    &right.sensor_state_id,
                    &right.action_id,
                ))
        });
    }

    fn compile_ordered(&self) -> Result<CompiledLocomotionKernel, AdapterError> {
        nonempty(&self.cost_unit_id, "cost_unit_id")?;
        if self.minimum_delivery_fidelity.is_zero()
            || !self.minimum_delivery_fidelity.is_probability()
        {
            return Err(AdapterError::InvalidDeliveryFidelityThreshold);
        }
        if self.hypotheses.is_empty()
            || self.physical_states.is_empty()
            || self.sensor_states.is_empty()
            || self.actions.is_empty()
            || self.observations.is_empty()
        {
            return Err(AdapterError::InvalidKernel(
                "hypotheses, physical states, sensor states, actions and observations must be non-empty"
                    .to_owned(),
            ));
        }

        let source_indices = index_ids(&self.source_manifests, "source_manifest_id", |item| {
            &item.id
        })?;
        let hypothesis_indices = index_ids(&self.hypotheses, "hypothesis_id", |item| &item.id)?;
        let physical_indices =
            index_ids(&self.physical_states, "physical_state_id", |item| &item.id)?;
        let sensor_indices = index_ids(&self.sensor_states, "sensor_state_id", |item| &item.id)?;
        for sensor in &self.sensor_states {
            if sensor.id != sensor_state_semantic_id(sensor) {
                return Err(AdapterError::SensorSemanticIdMismatch {
                    sensor_state_id: sensor.id.clone(),
                });
            }
        }
        let action_indices = index_ids(&self.actions, "action_id", |item| &item.id)?;
        let observation_indices = index_ids(&self.observations, "observation_id", |item| &item.id)?;

        for source in &self.source_manifests {
            validate_source(source)?;
        }

        let mut pins_seen = BTreeSet::new();
        for pin in &self.source_pins {
            let source_index = lookup(
                &source_indices,
                "source_manifest_id",
                &pin.source_manifest_id,
            )?;
            nonempty(&pin.expected_sha256, "expected_source_sha256")?;
            if !valid_sha256(&pin.expected_sha256) {
                return Err(AdapterError::InvalidSha256 {
                    source_id: pin.source_manifest_id.clone(),
                });
            }
            if !pins_seen.insert(pin.source_manifest_id.clone()) {
                return Err(AdapterError::DuplicateIdentifier {
                    kind: "source_digest_pin",
                    id: pin.source_manifest_id.clone(),
                });
            }
            if self.source_manifests[source_index].sha256.as_deref()
                != Some(pin.expected_sha256.as_str())
            {
                return Err(AdapterError::SourcePinMismatch {
                    source_id: pin.source_manifest_id.clone(),
                });
            }
        }
        for source in &self.source_manifests {
            if source.evidence_class != EvidenceClass::SyntheticFixture
                && !pins_seen.contains(&source.id)
            {
                return Err(AdapterError::UnpinnedNonSyntheticSource {
                    source_id: source.id.clone(),
                });
            }
        }

        for hypothesis in &self.hypotheses {
            if hypothesis.source_manifest_ids.is_empty() {
                return Err(AdapterError::MissingHypothesisSource {
                    hypothesis_id: hypothesis.id.clone(),
                });
            }
            let mut referenced = BTreeSet::new();
            let mut supports_claim = false;
            for source_id in &hypothesis.source_manifest_ids {
                if !referenced.insert(source_id.clone()) {
                    return Err(AdapterError::DuplicateIdentifier {
                        kind: "hypothesis_source_reference",
                        id: source_id.clone(),
                    });
                }
                let index = lookup(&source_indices, "source_manifest_id", source_id)?;
                supports_claim |=
                    source_supports_claim(&self.source_manifests[index], hypothesis.claim_ceiling);
            }
            if !supports_claim {
                return Err(AdapterError::InvalidEvidenceClaim {
                    hypothesis_id: hypothesis.id.clone(),
                    claim: hypothesis.claim_ceiling,
                });
            }
        }

        for action in &self.actions {
            let target_leg_valid = match action.kind {
                InterventionKind::ActiveMovementProbe
                | InterventionKind::PassiveMovementProbe
                | InterventionKind::DescendingPathwayModulation
                | InterventionKind::ContactLoadPerturbation => action.target_leg.is_some(),
                _ => action.target_leg.is_none(),
            };
            let presynaptic_target_valid = match action.kind {
                InterventionKind::DescendingPathwayModulation => {
                    action.presynaptic_inhibition_target.is_some()
                }
                _ => action.presynaptic_inhibition_target.is_none(),
            };
            if !target_leg_valid || !presynaptic_target_valid {
                return Err(AdapterError::InvalidPresynapticInterventionTarget {
                    action_id: action.id.clone(),
                });
            }
            nonempty(&action.protocol_id, "protocol_id")?;
            if action.cost_units == 0 {
                return Err(AdapterError::InvalidActionCost {
                    action_id: action.id.clone(),
                });
            }
            let protocol_source_index = lookup(
                &source_indices,
                "protocol_source_manifest_id",
                &action.protocol_source_manifest_id,
            )
            .map_err(|_| AdapterError::MissingProtocolSource {
                action_id: action.id.clone(),
            })?;
            let protocol_source = &self.source_manifests[protocol_source_index];
            if !(matches!(
                protocol_source.kind,
                SourceKind::ExperimentalProtocol | SourceKind::SyntheticFixture
            ) || (protocol_source.kind == SourceKind::SimulationCode
                && protocol_source.evidence_class == EvidenceClass::InSilicoGenerated))
            {
                return Err(AdapterError::MissingProtocolSource {
                    action_id: action.id.clone(),
                });
            }
        }

        let mut initial_belief = Vec::with_capacity(self.initial_belief.len());
        let mut initial_keys = BTreeSet::new();
        for point in &self.initial_belief {
            let hypothesis = lookup(&hypothesis_indices, "hypothesis_id", &point.hypothesis_id)?;
            let physical_state = lookup(
                &physical_indices,
                "physical_state_id",
                &point.physical_state_id,
            )?;
            let sensor_state = lookup(&sensor_indices, "sensor_state_id", &point.sensor_state_id)?;
            if !initial_keys.insert((hypothesis, physical_state, sensor_state)) {
                return Err(AdapterError::InvalidKernel(
                    "initial belief contains a duplicate joint tuple".to_owned(),
                ));
            }
            initial_belief.push(JointMass {
                hypothesis,
                physical_state,
                sensor_state,
                mass: point.mass,
            });
        }
        initial_belief.sort();

        let mut indexed_rows = BTreeMap::new();
        for row in &self.transition_rows {
            let key = (
                lookup(&hypothesis_indices, "hypothesis_id", &row.hypothesis_id)?,
                lookup(
                    &physical_indices,
                    "physical_state_id",
                    &row.physical_state_id,
                )?,
                lookup(&sensor_indices, "sensor_state_id", &row.sensor_state_id)?,
                lookup(&action_indices, "action_id", &row.action_id)?,
            );
            if indexed_rows.insert(key, row).is_some() {
                return Err(AdapterError::DuplicateTransitionRow {
                    hypothesis_id: row.hypothesis_id.clone(),
                    physical_state_id: row.physical_state_id.clone(),
                    sensor_state_id: row.sensor_state_id.clone(),
                    action_id: row.action_id.clone(),
                });
            }
            if row.source_manifest_ids.is_empty() {
                return Err(AdapterError::MissingTransitionSource {
                    hypothesis_id: row.hypothesis_id.clone(),
                    action_id: row.action_id.clone(),
                });
            }
            let mut row_sources = BTreeSet::new();
            let mut supports_claim = false;
            for source_id in &row.source_manifest_ids {
                if !row_sources.insert(source_id.clone()) {
                    return Err(AdapterError::DuplicateTransitionSource {
                        hypothesis_id: row.hypothesis_id.clone(),
                        action_id: row.action_id.clone(),
                        source_id: source_id.clone(),
                    });
                }
                let source_index =
                    lookup(&source_indices, "transition_source_manifest_id", source_id)?;
                supports_claim |=
                    source_supports_claim(&self.source_manifests[source_index], row.claim_ceiling);
            }
            if !supports_claim {
                return Err(AdapterError::InvalidEvidenceClaim {
                    hypothesis_id: row.hypothesis_id.clone(),
                    claim: row.claim_ceiling,
                });
            }
        }

        let mut rows = Vec::with_capacity(
            self.hypotheses.len()
                * self.physical_states.len()
                * self.sensor_states.len()
                * self.actions.len(),
        );
        for hypothesis in 0..self.hypotheses.len() {
            for physical_state in 0..self.physical_states.len() {
                for sensor_state in 0..self.sensor_states.len() {
                    for action in 0..self.actions.len() {
                        let key = (hypothesis, physical_state, sensor_state, action);
                        let row = indexed_rows.get(&key).ok_or_else(|| {
                            AdapterError::IncompleteTransitionRow {
                                hypothesis_id: self.hypotheses[hypothesis].id.clone(),
                                physical_state_id: self.physical_states[physical_state].id.clone(),
                                sensor_state_id: self.sensor_states[sensor_state].id.clone(),
                                action_id: self.actions[action].id.clone(),
                            }
                        })?;
                        let mut outcomes = Vec::with_capacity(row.outcomes.len());
                        for outcome in &row.outcomes {
                            let next_physical_state = lookup(
                                &physical_indices,
                                "physical_state_id",
                                &outcome.next_physical_state_id,
                            )?;
                            let next_sensor_state = lookup(
                                &sensor_indices,
                                "sensor_state_id",
                                &outcome.next_sensor_state_id,
                            )?;
                            validate_action_transition_semantics(
                                &self.actions[action],
                                &self.physical_states[physical_state],
                                &self.sensor_states[sensor_state],
                                &self.physical_states[next_physical_state],
                                &self.sensor_states[next_sensor_state],
                            )?;
                            outcomes.push(JointOutcome {
                                probability: outcome.probability,
                                next_physical_state,
                                next_sensor_state,
                                observation: outcome
                                    .observation_id
                                    .as_deref()
                                    .map(|id| lookup(&observation_indices, "observation_id", id))
                                    .transpose()?,
                            });
                        }
                        rows.push(KernelRow {
                            hypothesis,
                            physical_state,
                            sensor_state,
                            action,
                            outcomes,
                        });
                    }
                }
            }
        }
        if indexed_rows.len() != rows.len() {
            return Err(AdapterError::InvalidKernel(
                "transition table contains rows outside the canonical joint space".to_owned(),
            ));
        }

        let kernel = JointKernel {
            hypothesis_ids: self.hypotheses.iter().map(|item| item.id.clone()).collect(),
            physical_state_count: self.physical_states.len(),
            sensor_state_count: self.sensor_states.len(),
            action_ids: self.actions.iter().map(|item| item.id.clone()).collect(),
            observation_ids: self
                .observations
                .iter()
                .map(|item| item.id.clone())
                .collect(),
            action_costs: self.actions.iter().map(|item| item.cost_units).collect(),
            initial_belief,
            rows,
        };
        kernel.validate().map_err(AdapterError::InvalidKernel)?;

        Ok(CompiledLocomotionKernel {
            kernel,
            source_manifests: self.source_manifests.clone(),
            hypotheses: self.hypotheses.clone(),
            physical_states: self.physical_states.clone(),
            sensor_states: self.sensor_states.clone(),
            actions: self.actions.clone(),
            observations: self.observations.clone(),
            transition_rows: self.transition_rows.clone(),
            cost_unit_id: self.cost_unit_id.clone(),
            minimum_delivery_fidelity: self.minimum_delivery_fidelity,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocomotionTrialReceipt {
    pub sample_id: String,
    pub assigned_action_id: String,
    pub delivered_protocol_id: String,
    pub delivery_verified: bool,
    pub delivery_fidelity: Rational,
    /// SHA-256 of the complete pre-outcome history used for this trial's forecast.
    pub conditioning_state_digest: String,
    pub animal_or_preparation_id: Option<String>,
    pub session_id: String,
    pub source_manifest_id: String,
    pub evidence_class: EvidenceClass,
}

impl CompiledLocomotionKernel {
    /// Validate protocol assignment separately from actual delivery. A mismatch
    /// is retained as a delivery-failure record and cannot update a causal model.
    pub fn validate_trial_receipt(
        &self,
        receipt: &LocomotionTrialReceipt,
    ) -> Result<(), AdapterError> {
        nonempty(&receipt.sample_id, "sample_id")?;
        nonempty(&receipt.assigned_action_id, "assigned_action_id")?;
        nonempty(&receipt.delivered_protocol_id, "delivered_protocol_id")?;
        nonempty(&receipt.session_id, "session_id")?;
        if !receipt.delivery_fidelity.is_probability()
            || receipt.delivery_fidelity < self.minimum_delivery_fidelity
        {
            return Err(AdapterError::InvalidTrialReceipt(
                "delivered protocol fidelity is below the pre-registered minimum",
            ));
        }
        if !valid_sha256(&receipt.conditioning_state_digest) {
            return Err(AdapterError::InvalidConditioningStateDigest);
        }
        let action = self
            .actions
            .iter()
            .find(|action| action.id == receipt.assigned_action_id)
            .ok_or_else(|| {
                AdapterError::UnknownAssignedAction(receipt.assigned_action_id.clone())
            })?;
        if !receipt.delivery_verified {
            return Err(AdapterError::InvalidTrialReceipt(
                "delivered protocol was not independently verified",
            ));
        }
        if action.protocol_id != receipt.delivered_protocol_id {
            return Err(AdapterError::AssignmentDeliveryMismatch {
                assigned_action_id: receipt.assigned_action_id.clone(),
                expected_protocol_id: action.protocol_id.clone(),
                delivered_protocol_id: receipt.delivered_protocol_id.clone(),
            });
        }
        let source = self
            .source_manifests
            .iter()
            .find(|source| source.id == receipt.source_manifest_id)
            .ok_or_else(|| AdapterError::UnknownIdentifier {
                kind: "trial_source_manifest_id",
                id: receipt.source_manifest_id.clone(),
            })?;
        if source.evidence_class != receipt.evidence_class {
            return Err(AdapterError::EvidenceSourceMismatch {
                source_id: source.id.clone(),
                declared: receipt.evidence_class,
                source: source.evidence_class,
            });
        }
        if receipt.evidence_class == EvidenceClass::ConnectomeDerived {
            return Err(AdapterError::AnatomicalEvidenceCannotScoreTrial);
        }
        match receipt.evidence_class {
            EvidenceClass::SyntheticFixture | EvidenceClass::InSilicoGenerated => {}
            _ => {
                let individual = receipt.animal_or_preparation_id.as_deref().ok_or(
                    AdapterError::InvalidTrialReceipt(
                        "measured trial lacks animal/preparation replication identity",
                    ),
                )?;
                nonempty(individual, "animal_or_preparation_id")?;
            }
        }
        Ok(())
    }

    /// Validate a batch of trial receipts while rejecting duplicate sample IDs.
    /// Repeated measurements within an animal/session remain allowed; callers
    /// must provide a new complete-history digest for each sequential look.
    pub fn validate_trial_receipts(
        &self,
        receipts: &[LocomotionTrialReceipt],
    ) -> Result<(), AdapterError> {
        let mut sample_ids = BTreeSet::new();
        for receipt in receipts {
            if !sample_ids.insert(receipt.sample_id.clone()) {
                return Err(AdapterError::DuplicateSampleId(receipt.sample_id.clone()));
            }
            self.validate_trial_receipt(receipt)?;
        }
        Ok(())
    }

    pub fn row_index(
        &self,
        hypothesis_id: &str,
        physical_state_id: &str,
        sensor_state_id: &str,
        action_id: &str,
    ) -> Result<usize, AdapterError> {
        let hypothesis = self
            .hypotheses
            .iter()
            .position(|item| item.id == hypothesis_id)
            .ok_or_else(|| AdapterError::UnknownIdentifier {
                kind: "hypothesis_id",
                id: hypothesis_id.to_owned(),
            })?;
        let physical = self
            .physical_states
            .iter()
            .position(|item| item.id == physical_state_id)
            .ok_or_else(|| AdapterError::UnknownIdentifier {
                kind: "physical_state_id",
                id: physical_state_id.to_owned(),
            })?;
        let sensor = self
            .sensor_states
            .iter()
            .position(|item| item.id == sensor_state_id)
            .ok_or_else(|| AdapterError::UnknownIdentifier {
                kind: "sensor_state_id",
                id: sensor_state_id.to_owned(),
            })?;
        let action = self
            .actions
            .iter()
            .position(|item| item.id == action_id)
            .ok_or_else(|| AdapterError::UnknownIdentifier {
                kind: "action_id",
                id: action_id.to_owned(),
            })?;
        Ok(
            (((hypothesis * self.physical_states.len()) + physical) * self.sensor_states.len()
                + sensor)
                * self.actions.len()
                + action,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::rank_by_one_step_eig;
    use crate::{PolicyPlan, SolverConfig, optimize_joint_kernel};

    fn sha(fill: char) -> String {
        std::iter::repeat_n(fill, 64).collect()
    }

    fn synthetic_source(id: &str, kind: SourceKind) -> SourceManifest {
        SourceManifest {
            id: id.to_owned(),
            kind,
            evidence_class: EvidenceClass::SyntheticFixture,
            citation_or_source_uri: format!("synthetic://locomotion/{id}"),
            dataset_id: None,
            release_or_materialization: None,
            sha256: Some(sha('a')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        }
    }

    fn sensor_states() -> Vec<SensorState> {
        let channels = [ChannelState::Intact, ChannelState::Gated];
        let vibration = [ChannelState::Intact, ChannelState::Attenuated];
        let acquisition = [
            AcquisitionState::Calibrated,
            AcquisitionState::PositiveOffset,
        ];
        let mut states = Vec::new();
        for position in channels {
            for movement in channels {
                for vib in vibration {
                    for contact in channels {
                        for acquisition_state in acquisition {
                            for inhibition_mask in 0_u8..64 {
                                let presynaptic_movement_inhibition =
                                    std::array::from_fn(|leg_index| {
                                        if inhibition_mask & (1 << leg_index) == 0 {
                                            PresynapticMovementInhibition::Disinhibited
                                        } else {
                                            PresynapticMovementInhibition::Engaged
                                        }
                                    });
                                let mut sensor = SensorState {
                                    id: String::new(),
                                    position_afferent: position,
                                    movement_afferent: movement,
                                    vibration_afferent: vib,
                                    contact_feedback: contact,
                                    acquisition: acquisition_state,
                                    presynaptic_movement_inhibition,
                                };
                                sensor.id = sensor_state_semantic_id(&sensor);
                                states.push(sensor);
                            }
                        }
                    }
                }
            }
        }
        states
    }

    fn source_spec() -> Vec<SourceManifest> {
        vec![
            synthetic_source("synthetic-models", SourceKind::SyntheticFixture),
            synthetic_source("synthetic-protocols", SourceKind::SyntheticFixture),
        ]
    }

    fn sensor_index(states: &[SensorState], target: &SensorState) -> usize {
        states.iter().position(|state| state == target).unwrap()
    }

    fn spec() -> LocomotionKernelSpec {
        let hypotheses = vec![
            MechanismHypothesis {
                id: "central-rhythm-model".to_owned(),
                family: MechanismFamily::CentralRhythmGenerator,
                source_manifest_ids: vec!["synthetic-models".to_owned()],
                claim_ceiling: ClaimCeiling::SyntheticQualification,
            },
            MechanismHypothesis {
                id: "distributed-local-loop-model".to_owned(),
                family: MechanismFamily::DistributedLocalSensorimotorLoops,
                source_manifest_ids: vec!["synthetic-models".to_owned()],
                claim_ceiling: ClaimCeiling::SyntheticQualification,
            },
        ];
        let mut physical_states = vec![
            PhysicalState {
                id: "stance-nominal".to_owned(),
                behavioral_context: BehavioralContext::Walking,
                gait_phase: GaitPhase::Stance,
                contact_state: ContactState::Supported,
                load_state: LoadState::Nominal,
                carryover: PhysicalCarryover::Baseline,
                legs: [
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Extended, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Swing, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Protracting, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Swing, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Protracting, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Elevated, JointPositionBin::Extended, JointVelocityBin::Retracting, ActuatorForceBin::High),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Swing, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Protracting, ActuatorForceBin::Low),
                ],
            },
            PhysicalState {
                id: "swing-loaded".to_owned(),
                behavioral_context: BehavioralContext::Walking,
                gait_phase: GaitPhase::Swing,
                contact_state: ContactState::Slipping,
                load_state: LoadState::Elevated,
                carryover: PhysicalCarryover::Loaded,
                legs: [
                    leg_mechanics(GaitPhase::Swing, ContactState::Slipping, LoadState::Elevated, JointPositionBin::Flexed, JointVelocityBin::Retracting, ActuatorForceBin::High),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Extended, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Extended, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Swing, ContactState::Unloaded, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Protracting, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Swing, ContactState::Unloaded, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Protracting, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Elevated, JointPositionBin::Extended, JointVelocityBin::Retracting, ActuatorForceBin::High),
                ],
            },
            PhysicalState {
                id: "grooming-supported".to_owned(),
                behavioral_context: BehavioralContext::Grooming,
                gait_phase: GaitPhase::Transition,
                contact_state: ContactState::Supported,
                load_state: LoadState::Nominal,
                carryover: PhysicalCarryover::Baseline,
                legs: [
                    leg_mechanics(GaitPhase::Transition, ContactState::Unloaded, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Protracting, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Transition, ContactState::Unloaded, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Retracting, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Elevated, JointPositionBin::Extended, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Elevated, JointPositionBin::Extended, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                ],
            },
            PhysicalState {
                id: "grooming-unloaded".to_owned(),
                behavioral_context: BehavioralContext::Grooming,
                gait_phase: GaitPhase::Transition,
                contact_state: ContactState::Unloaded,
                load_state: LoadState::Reduced,
                carryover: PhysicalCarryover::Loaded,
                legs: [
                    leg_mechanics(GaitPhase::Transition, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Extended, JointVelocityBin::Protracting, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Transition, ContactState::Unloaded, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Retracting, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Elevated, JointPositionBin::Extended, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Elevated, JointPositionBin::Extended, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                ],
            },
            PhysicalState {
                id: "other-supported".to_owned(),
                behavioral_context: BehavioralContext::Other,
                gait_phase: GaitPhase::Transition,
                contact_state: ContactState::Supported,
                load_state: LoadState::Nominal,
                carryover: PhysicalCarryover::Baseline,
                legs: [
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                    leg_mechanics(GaitPhase::Stance, ContactState::Supported, LoadState::Nominal, JointPositionBin::Neutral, JointVelocityBin::Stationary, ActuatorForceBin::Nominal),
                ],
            },
            PhysicalState {
                id: "other-unloaded".to_owned(),
                behavioral_context: BehavioralContext::Other,
                gait_phase: GaitPhase::Transition,
                contact_state: ContactState::Airborne,
                load_state: LoadState::Reduced,
                carryover: PhysicalCarryover::Loaded,
                legs: [
                    leg_mechanics(GaitPhase::Transition, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Stationary, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Transition, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Stationary, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Transition, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Stationary, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Transition, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Stationary, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Transition, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Stationary, ActuatorForceBin::Low),
                    leg_mechanics(GaitPhase::Transition, ContactState::Airborne, LoadState::Reduced, JointPositionBin::Flexed, JointVelocityBin::Stationary, ActuatorForceBin::Low),
                ],
            },
        ];
        // For each behavioral/physical archetype add a paired fixture in which
        // only the declared front-left leg's contact, load and actuation differ.
        let base_physical_states = physical_states.clone();
        for mut state in base_physical_states {
            state.id.push_str("-front-left-load-perturbed");
            state.legs[LegId::FrontLeft.index()] = leg_mechanics(
                state.legs[LegId::FrontLeft.index()].gait_phase,
                ContactState::Slipping,
                LoadState::Elevated,
                JointPositionBin::Extended,
                JointVelocityBin::Retracting,
                ActuatorForceBin::High,
            );
            physical_states.push(state);
        }
        let sensor_states = sensor_states();
        let actions = vec![
            InterventionAction {
                id: "active-movement-probe".to_owned(),
                kind: InterventionKind::ActiveMovementProbe,
                protocol_id: "protocol-active-probe".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: Some(LegId::FrontLeft),
                presynaptic_inhibition_target: None,
            },
            InterventionAction {
                id: "acquisition-reference-calibration".to_owned(),
                kind: InterventionKind::AcquisitionReferenceCalibration,
                protocol_id: "protocol-reference-calibration".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: None,
                presynaptic_inhibition_target: None,
            },
            InterventionAction {
                id: "movement-afferent-gate".to_owned(),
                kind: InterventionKind::MovementAfferentGate,
                protocol_id: "protocol-movement-gate".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: None,
                presynaptic_inhibition_target: None,
            },
            InterventionAction {
                id: "position-afferent-perturbation".to_owned(),
                kind: InterventionKind::PositionAfferentPerturbation,
                protocol_id: "protocol-position-perturbation".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: None,
                presynaptic_inhibition_target: None,
            },
            InterventionAction {
                id: "vibration-afferent-perturbation".to_owned(),
                kind: InterventionKind::VibrationAfferentPerturbation,
                protocol_id: "protocol-vibration-perturbation".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: None,
                presynaptic_inhibition_target: None,
            },
            InterventionAction {
                id: "contact-load-perturbation".to_owned(),
                kind: InterventionKind::ContactLoadPerturbation,
                protocol_id: "protocol-contact-load".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: Some(LegId::FrontLeft),
                presynaptic_inhibition_target: None,
            },
            InterventionAction {
                id: "passive-movement-probe".to_owned(),
                kind: InterventionKind::PassiveMovementProbe,
                protocol_id: "protocol-passive-probe".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: Some(LegId::FrontLeft),
                presynaptic_inhibition_target: None,
            },
            InterventionAction {
                id: "descending-inhibition-engage".to_owned(),
                kind: InterventionKind::DescendingPathwayModulation,
                protocol_id: "protocol-descending-engage".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: Some(LegId::FrontLeft),
                presynaptic_inhibition_target: Some(PresynapticMovementInhibition::Engaged),
            },
            InterventionAction {
                id: "descending-inhibition-release".to_owned(),
                kind: InterventionKind::DescendingPathwayModulation,
                protocol_id: "protocol-descending-release".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: Some(LegId::FrontLeft),
                presynaptic_inhibition_target: Some(PresynapticMovementInhibition::Disinhibited),
            },
            InterventionAction {
                id: "descending-inhibition-engage-front-right".to_owned(),
                kind: InterventionKind::DescendingPathwayModulation,
                protocol_id: "protocol-descending-engage-front-right".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: Some(LegId::FrontRight),
                presynaptic_inhibition_target: Some(PresynapticMovementInhibition::Engaged),
            },
            InterventionAction {
                id: "descending-inhibition-release-front-right".to_owned(),
                kind: InterventionKind::DescendingPathwayModulation,
                protocol_id: "protocol-descending-release-front-right".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: Some(LegId::FrontRight),
                presynaptic_inhibition_target: Some(PresynapticMovementInhibition::Disinhibited),
            },
            InterventionAction {
                id: "washout-carryover".to_owned(),
                kind: InterventionKind::Washout,
                protocol_id: "protocol-washout".to_owned(),
                protocol_source_manifest_id: "synthetic-protocols".to_owned(),
                cost_units: 1,
                target_leg: None,
                presynaptic_inhibition_target: None,
            },
        ];
        let observations = vec![
            Observation {
                id: "signal-low".to_owned(),
                kind: ObservationKind::NeuralMotorRhythm,
            },
            Observation {
                id: "signal-high".to_owned(),
                kind: ObservationKind::NeuralMotorRhythm,
            },
            Observation {
                id: "reference-calibrated".to_owned(),
                kind: ObservationKind::AcquisitionReference,
            },
            Observation {
                id: "movement-gated".to_owned(),
                kind: ObservationKind::MovementFeedback,
            },
            Observation {
                id: "position-perturbed".to_owned(),
                kind: ObservationKind::PositionFeedback,
            },
            Observation {
                id: "vibration-perturbed".to_owned(),
                kind: ObservationKind::VibrationResponse,
            },
            Observation {
                id: "load-shifted".to_owned(),
                kind: ObservationKind::ContactTransition,
            },
            Observation {
                id: "washout-cleared".to_owned(),
                kind: ObservationKind::WashoutResponse,
            },
            Observation {
                id: "movement-presynaptic-inhibited".to_owned(),
                kind: ObservationKind::MovementFeedback,
            },
            Observation {
                id: "movement-presynaptic-released".to_owned(),
                kind: ObservationKind::MovementFeedback,
            },
        ];

        let mut initial_belief =
            vec![
                InitialJointMass {
                    hypothesis_id: hypotheses[0].id.clone(),
                    physical_state_id: physical_states[0].id.clone(),
                    sensor_state_id: sensor_states
                        .iter()
                        .find(|sensor| {
                            sensor.position_afferent == ChannelState::Intact
                                && sensor.movement_afferent == ChannelState::Intact
                                && sensor.vibration_afferent == ChannelState::Intact
                                && sensor.contact_feedback == ChannelState::Intact
                                && sensor.acquisition == AcquisitionState::PositiveOffset
                                && sensor.presynaptic_movement_inhibition.iter().all(|state| {
                                    *state == PresynapticMovementInhibition::Disinhibited
                                })
                        })
                        .unwrap()
                        .id
                        .clone(),
                    mass: Rational::new(1, 2).unwrap(),
                },
                InitialJointMass {
                    hypothesis_id: hypotheses[1].id.clone(),
                    physical_state_id: physical_states[1].id.clone(),
                    sensor_state_id: sensor_states
                        .iter()
                        .find(|sensor| {
                            sensor.position_afferent == ChannelState::Intact
                                && sensor.movement_afferent == ChannelState::Intact
                                && sensor.vibration_afferent == ChannelState::Intact
                                && sensor.contact_feedback == ChannelState::Intact
                                && sensor.acquisition == AcquisitionState::Calibrated
                                && sensor.presynaptic_movement_inhibition.iter().all(|state| {
                                    *state == PresynapticMovementInhibition::Disinhibited
                                })
                        })
                        .unwrap()
                        .id
                        .clone(),
                    mass: Rational::new(1, 2).unwrap(),
                },
            ];
        initial_belief.reverse();

        let mut transition_rows = Vec::new();
        for hypothesis in &hypotheses {
            for physical in &physical_states {
                for sensor in &sensor_states {
                    for action in &actions {
                        let mut next_physical = physical.id.clone();
                        let mut next_sensor = sensor.clone();
                        let observation_id = match action.kind {
                            InterventionKind::ActiveMovementProbe => {
                                let target_leg = action
                                    .target_leg
                                    .expect("validated active probe has a target leg");
                                let target_state = match physical.behavioral_context {
                                    BehavioralContext::Walking | BehavioralContext::Grooming => {
                                        PresynapticMovementInhibition::Engaged
                                    }
                                    BehavioralContext::Other => {
                                        PresynapticMovementInhibition::Disinhibited
                                    }
                                };
                                next_sensor.presynaptic_movement_inhibition[target_leg.index()] =
                                    target_state;
                                next_sensor.id = sensor_state_semantic_id(&next_sensor);
                                let physical_bit = usize::from(physical.id == "swing-loaded");
                                let bias_bit = usize::from(
                                    sensor.acquisition == AcquisitionState::PositiveOffset,
                                );
                                if physical_bit ^ bias_bit == 0 {
                                    "signal-low"
                                } else {
                                    "signal-high"
                                }
                            }
                            InterventionKind::PassiveMovementProbe => {
                                let target_leg = action
                                    .target_leg
                                    .expect("validated passive probe has a target leg");
                                next_sensor.presynaptic_movement_inhibition[target_leg.index()] =
                                    PresynapticMovementInhibition::Disinhibited;
                                next_sensor.id = sensor_state_semantic_id(&next_sensor);
                                let physical_bit = usize::from(physical.id == "swing-loaded");
                                let bias_bit = usize::from(
                                    sensor.acquisition == AcquisitionState::PositiveOffset,
                                );
                                if physical_bit ^ bias_bit == 0 {
                                    "signal-low"
                                } else {
                                    "signal-high"
                                }
                            }
                            InterventionKind::AcquisitionReferenceCalibration => {
                                next_sensor.acquisition = AcquisitionState::Calibrated;
                                next_sensor.id = sensor_state_semantic_id(&next_sensor);
                                "reference-calibrated"
                            }
                            InterventionKind::MovementAfferentGate => {
                                next_sensor.movement_afferent = ChannelState::Gated;
                                next_sensor.id = sensor_state_semantic_id(&next_sensor);
                                "movement-gated"
                            }
                            InterventionKind::PositionAfferentPerturbation => {
                                next_sensor.position_afferent = ChannelState::Gated;
                                next_sensor.id = sensor_state_semantic_id(&next_sensor);
                                "position-perturbed"
                            }
                            InterventionKind::VibrationAfferentPerturbation => {
                                next_sensor.vibration_afferent = ChannelState::Attenuated;
                                next_sensor.id = sensor_state_semantic_id(&next_sensor);
                                "vibration-perturbed"
                            }
                            InterventionKind::ContactLoadPerturbation => {
                                const SUFFIX: &str = "-front-left-load-perturbed";
                                next_physical = if let Some(base) = physical.id.strip_suffix(SUFFIX) {
                                    base.to_owned()
                                } else {
                                    format!("{}{}", physical.id, SUFFIX)
                                };
                                "load-shifted"
                            }
                            InterventionKind::DescendingPathwayModulation => {
                                let target_leg = action
                                    .target_leg
                                    .expect("validated descending modulation has a target leg");
                                let target_state = action
                                    .presynaptic_inhibition_target
                                    .expect("validated descending modulation has a target state");
                                next_sensor.presynaptic_movement_inhibition[target_leg.index()] =
                                    target_state;
                                next_sensor.id = sensor_state_semantic_id(&next_sensor);
                                match target_state {
                                    PresynapticMovementInhibition::Engaged => {
                                        "movement-presynaptic-inhibited"
                                    }
                                    PresynapticMovementInhibition::Disinhibited => {
                                        "movement-presynaptic-released"
                                    }
                                }
                            }
                            InterventionKind::Washout => {
                                next_physical = if physical.carryover == PhysicalCarryover::Baseline
                                {
                                    physical.id.clone()
                                } else {
                                    match physical.behavioral_context {
                                        BehavioralContext::Walking => "stance-nominal",
                                        BehavioralContext::Grooming => "grooming-supported",
                                        BehavioralContext::Other => "other-supported",
                                    }
                                    .to_owned()
                                };
                                "washout-cleared"
                            }
                        };
                        let _next_sensor_index = sensor_index(&sensor_states, &next_sensor);
                        let outcomes = if matches!(
                            action.kind,
                            InterventionKind::ActiveMovementProbe
                                | InterventionKind::PassiveMovementProbe
                        ) {
                            // A deliberately noisy synthetic readout. The acquisition
                            // confound makes both initial hypotheses produce the same
                            // observation distribution before calibration. After
                            // calibration, the physical state separates the likelihoods.
                            let high_probability = if observation_id == "signal-high" {
                                Rational::new(3, 4).unwrap()
                            } else {
                                Rational::new(1, 4).unwrap()
                            };
                            let low_probability =
                                Rational::one().checked_sub(high_probability).unwrap();
                            vec![
                                TransitionOutcome {
                                    probability: high_probability,
                                    next_physical_state_id: next_physical.clone(),
                                    next_sensor_state_id: next_sensor.id.clone(),
                                    observation_id: Some("signal-high".to_owned()),
                                },
                                TransitionOutcome {
                                    probability: low_probability,
                                    next_physical_state_id: next_physical.clone(),
                                    next_sensor_state_id: next_sensor.id.clone(),
                                    observation_id: Some("signal-low".to_owned()),
                                },
                            ]
                        } else {
                            vec![TransitionOutcome {
                                probability: Rational::one(),
                                next_physical_state_id: next_physical,
                                next_sensor_state_id: next_sensor.id,
                                observation_id: Some(observation_id.to_owned()),
                            }]
                        };
                        transition_rows.push(JointTransitionRow {
                            hypothesis_id: hypothesis.id.clone(),
                            physical_state_id: physical.id.clone(),
                            sensor_state_id: sensor.id.clone(),
                            action_id: action.id.clone(),
                            source_manifest_ids: vec!["synthetic-models".to_owned()],
                            claim_ceiling: ClaimCeiling::SyntheticQualification,
                            outcomes,
                        });
                    }
                }
            }
        }

        LocomotionKernelSpec {
            source_manifests: source_spec(),
            source_pins: vec![],
            hypotheses,
            physical_states,
            sensor_states,
            actions,
            observations,
            initial_belief,
            transition_rows,
            cost_unit_id: "synthetic-experiment-credit".to_owned(),
            minimum_delivery_fidelity: Rational::new(9, 10).unwrap(),
        }
    }

    #[test]
    fn compiles_canonical_joint_kernel_and_preserves_unknown_endpoint_records() {
        let mut input = spec();
        input.source_manifests[0]
            .unknown_endpoints
            .push(UnknownEndpoint {
                raw_endpoint_id: "raw-untyped-endpoint-17".to_owned(),
                role: EndpointRole::UnknownSide,
                missingness_reason: "source snapshot does not identify this endpoint".to_owned(),
            });
        let compiled = input.compile().unwrap();
        assert_eq!(
            compiled.source_manifests[0].unknown_endpoints[0].raw_endpoint_id,
            "raw-untyped-endpoint-17"
        );
        assert_eq!(compiled.kernel.rows.len(), 2 * 6 * 2048 * 12);
        assert_eq!(compiled.transition_rows.len(), compiled.kernel.rows.len());
        compiled.kernel.validate().unwrap();
    }

    #[test]
    fn movement_probe_rows_conserve_exact_noisy_observation_mass() {
        let compiled = spec().compile().unwrap();
        let sensor = compiled
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::PositiveOffset
                    && state
                        .presynaptic_movement_inhibition
                        .iter()
                        .all(|value| *value == PresynapticMovementInhibition::Disinhibited)
            })
            .unwrap();
        let row = compiled
            .row_index(
                "central-rhythm-model",
                "stance-nominal",
                &sensor.id,
                "active-movement-probe",
            )
            .unwrap();
        let outcomes = &compiled.kernel.rows[row].outcomes;
        assert_eq!(outcomes.len(), 2);
        assert_eq!(
            outcomes
                .iter()
                .map(|outcome| outcome.probability)
                .try_fold(Rational::zero(), |sum, probability| sum
                    .checked_add(probability))
                .unwrap(),
            Rational::one()
        );
        assert_eq!(
            outcomes
                .iter()
                .map(|outcome| outcome.probability)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([Rational::new(1, 4).unwrap(), Rational::new(3, 4).unwrap()])
        );
    }

    #[test]
    fn transition_rows_must_carry_source_provenance() {
        let mut input = spec();
        input.transition_rows[0].source_manifest_ids.clear();
        assert!(matches!(
            input.compile(),
            Err(AdapterError::MissingTransitionSource { .. })
        ));
    }

    #[test]
    fn transition_likelihood_claim_cannot_exceed_its_source_evidence() {
        let mut input = spec();
        input.transition_rows[0].claim_ceiling =
            ClaimCeiling::PeerReviewedMeasuredNeuralReproduction;
        assert!(matches!(
            input.compile(),
            Err(AdapterError::InvalidEvidenceClaim {
                claim: ClaimCeiling::PeerReviewedMeasuredNeuralReproduction,
                ..
            })
        ));
    }

    #[test]
    fn compiled_adapter_preserves_row_level_transition_provenance() {
        let compiled = spec().compile().unwrap();
        assert_eq!(compiled.transition_rows.len(), compiled.kernel.rows.len());
        assert!(compiled.transition_rows.iter().all(|row| {
            row.source_manifest_ids == vec!["synthetic-models".to_owned()]
                && row.claim_ceiling == ClaimCeiling::SyntheticQualification
        }));
    }

    #[test]
    fn sensor_semantic_ids_must_match_typed_channel_and_modulation_fields() {
        let mut input = spec();
        input.sensor_states[0].id.push_str("-wrong");
        assert!(matches!(
            input.compile(),
            Err(AdapterError::SensorSemanticIdMismatch { .. })
        ));
    }

    #[test]
    fn semantic_ids_make_compilation_invariant_to_input_storage_order() {
        let original_spec = spec();
        let original = original_spec.compile().unwrap();

        let mut reordered_spec = original_spec;
        reordered_spec.source_manifests.reverse();
        reordered_spec.source_pins.reverse();
        reordered_spec.hypotheses.reverse();
        for hypothesis in &mut reordered_spec.hypotheses {
            hypothesis.source_manifest_ids.reverse();
        }
        reordered_spec.physical_states.reverse();
        reordered_spec.sensor_states.reverse();
        reordered_spec.actions.reverse();
        reordered_spec.observations.reverse();
        reordered_spec.initial_belief.reverse();
        reordered_spec.transition_rows.reverse();
        let reordered = reordered_spec.compile().unwrap();

        assert_eq!(original, reordered);
        assert_eq!(
            original.kernel.hypothesis_ids,
            vec![
                "central-rhythm-model".to_owned(),
                "distributed-local-loop-model".to_owned()
            ]
        );
    }

    #[test]
    fn movement_gate_does_not_change_position_or_vibration_channels() {
        let compiled = spec().compile().unwrap();
        let original = compiled
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::Calibrated
            })
            .unwrap();
        let row_index = compiled
            .row_index(
                "central-rhythm-model",
                "stance-nominal",
                &original.id,
                "movement-afferent-gate",
            )
            .unwrap();
        let next =
            &compiled.sensor_states[compiled.kernel.rows[row_index].outcomes[0].next_sensor_state];
        assert_eq!(next.movement_afferent, ChannelState::Gated);
        assert_eq!(next.position_afferent, ChannelState::Intact);
        assert_eq!(next.vibration_afferent, ChannelState::Intact);
        assert_eq!(next.contact_feedback, ChannelState::Intact);
    }

    #[test]
    fn position_and_vibration_interventions_remain_modality_specific() {
        let compiled = spec().compile().unwrap();
        let original = compiled
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::Calibrated
            })
            .unwrap();
        let position_row = compiled
            .row_index(
                "central-rhythm-model",
                "stance-nominal",
                &original.id,
                "position-afferent-perturbation",
            )
            .unwrap();
        let position_next = &compiled.sensor_states
            [compiled.kernel.rows[position_row].outcomes[0].next_sensor_state];
        assert_eq!(position_next.position_afferent, ChannelState::Gated);
        assert_eq!(position_next.movement_afferent, ChannelState::Intact);
        assert_eq!(position_next.vibration_afferent, ChannelState::Intact);

        let vibration_row = compiled
            .row_index(
                "central-rhythm-model",
                "stance-nominal",
                &original.id,
                "vibration-afferent-perturbation",
            )
            .unwrap();
        let vibration_next = &compiled.sensor_states
            [compiled.kernel.rows[vibration_row].outcomes[0].next_sensor_state];
        assert_eq!(vibration_next.vibration_afferent, ChannelState::Attenuated);
        assert_eq!(vibration_next.position_afferent, ChannelState::Intact);
        assert_eq!(vibration_next.movement_afferent, ChannelState::Intact);
    }

    #[test]
    fn active_and_passive_probes_have_distinct_presynaptic_states_without_channel_loss() {
        let compiled = spec().compile().unwrap();
        let initial = compiled
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::Calibrated
                    && state
                        .presynaptic_movement_inhibition
                        .iter()
                        .all(|inhibition| {
                            *inhibition == PresynapticMovementInhibition::Disinhibited
                        })
            })
            .unwrap();

        let active_row = compiled
            .row_index(
                "central-rhythm-model",
                "stance-nominal",
                &initial.id,
                "active-movement-probe",
            )
            .unwrap();
        let passive_row = compiled
            .row_index(
                "central-rhythm-model",
                "stance-nominal",
                &initial.id,
                "passive-movement-probe",
            )
            .unwrap();
        let active_next =
            &compiled.sensor_states[compiled.kernel.rows[active_row].outcomes[0].next_sensor_state];
        let passive_next = &compiled.sensor_states
            [compiled.kernel.rows[passive_row].outcomes[0].next_sensor_state];

        assert_eq!(
            active_next.presynaptic_movement_inhibition[LegId::FrontLeft.index()],
            PresynapticMovementInhibition::Engaged
        );
        assert_eq!(
            passive_next.presynaptic_movement_inhibition[LegId::FrontLeft.index()],
            PresynapticMovementInhibition::Disinhibited
        );
        for leg in LegId::ALL
            .into_iter()
            .filter(|leg| *leg != LegId::FrontLeft)
        {
            assert_eq!(
                active_next.presynaptic_movement_inhibition[leg.index()],
                PresynapticMovementInhibition::Disinhibited
            );
            assert_eq!(
                passive_next.presynaptic_movement_inhibition[leg.index()],
                PresynapticMovementInhibition::Disinhibited
            );
        }
        for next in [active_next, passive_next] {
            assert_eq!(next.position_afferent, ChannelState::Intact);
            assert_eq!(next.movement_afferent, ChannelState::Intact);
            assert_eq!(next.vibration_afferent, ChannelState::Intact);
            assert_eq!(next.acquisition, AcquisitionState::Calibrated);
        }
    }

    #[test]
    fn active_presynaptic_modulation_is_walking_grooming_context_dependent() {
        let compiled = spec().compile().unwrap();
        let sensor = compiled
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::Calibrated
                    && state
                        .presynaptic_movement_inhibition
                        .iter()
                        .all(|value| *value == PresynapticMovementInhibition::Disinhibited)
            })
            .unwrap();

        for (physical_id, expected) in [
            ("stance-nominal", PresynapticMovementInhibition::Engaged),
            ("grooming-supported", PresynapticMovementInhibition::Engaged),
            (
                "other-supported",
                PresynapticMovementInhibition::Disinhibited,
            ),
        ] {
            let row = compiled
                .row_index(
                    "central-rhythm-model",
                    physical_id,
                    &sensor.id,
                    "active-movement-probe",
                )
                .unwrap();
            let next =
                &compiled.sensor_states[compiled.kernel.rows[row].outcomes[0].next_sensor_state];
            assert_eq!(
                next.presynaptic_movement_inhibition[LegId::FrontLeft.index()],
                expected,
                "unexpected presynaptic state in context {physical_id}"
            );
            for leg in LegId::ALL
                .into_iter()
                .filter(|leg| *leg != LegId::FrontLeft)
            {
                assert_eq!(
                    next.presynaptic_movement_inhibition[leg.index()],
                    PresynapticMovementInhibition::Disinhibited
                );
            }
            assert_eq!(next.position_afferent, ChannelState::Intact);
            assert_eq!(next.movement_afferent, ChannelState::Intact);
            assert_eq!(next.vibration_afferent, ChannelState::Intact);
        }
    }

    #[test]
    fn passive_probe_remains_disinhibited_in_walking_and_grooming_contexts() {
        let compiled = spec().compile().unwrap();
        let sensor = compiled
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::Calibrated
                    && state
                        .presynaptic_movement_inhibition
                        .iter()
                        .all(|value| *value == PresynapticMovementInhibition::Disinhibited)
            })
            .unwrap();
        for physical_id in ["stance-nominal", "grooming-supported", "other-supported"] {
            let row = compiled
                .row_index(
                    "central-rhythm-model",
                    physical_id,
                    &sensor.id,
                    "passive-movement-probe",
                )
                .unwrap();
            let next =
                &compiled.sensor_states[compiled.kernel.rows[row].outcomes[0].next_sensor_state];
            assert!(
                next.presynaptic_movement_inhibition
                    .iter()
                    .all(|value| *value == PresynapticMovementInhibition::Disinhibited)
            );
        }
    }

    #[test]
    fn washout_preserves_behavioral_context_while_clearing_carryover() {
        let compiled = spec().compile().unwrap();
        for (physical_id, expected_context, expected_state) in [
            ("swing-loaded", BehavioralContext::Walking, "stance-nominal"),
            (
                "grooming-unloaded",
                BehavioralContext::Grooming,
                "grooming-supported",
            ),
            (
                "other-unloaded",
                BehavioralContext::Other,
                "other-supported",
            ),
        ] {
            let sensor = compiled
                .sensor_states
                .iter()
                .find(|state| {
                    state.position_afferent == ChannelState::Intact
                        && state.movement_afferent == ChannelState::Intact
                        && state.vibration_afferent == ChannelState::Intact
                        && state.contact_feedback == ChannelState::Intact
                        && state.acquisition == AcquisitionState::PositiveOffset
                        && state
                            .presynaptic_movement_inhibition
                            .iter()
                            .all(|value| *value == PresynapticMovementInhibition::Disinhibited)
                })
                .unwrap();
            let row = compiled
                .row_index(
                    "central-rhythm-model",
                    physical_id,
                    &sensor.id,
                    "washout-carryover",
                )
                .unwrap();
            let outcome = &compiled.kernel.rows[row].outcomes[0];
            let next_physical = &compiled.physical_states[outcome.next_physical_state];
            let next_sensor = &compiled.sensor_states[outcome.next_sensor_state];
            assert_eq!(next_physical.id, expected_state);
            assert_eq!(next_physical.behavioral_context, expected_context);
            assert_eq!(next_physical.carryover, PhysicalCarryover::Baseline);
            assert_eq!(next_sensor, sensor);
        }
    }

    #[test]
    fn descending_modulation_uses_explicit_presynaptic_target_only() {
        let compiled = spec().compile().unwrap();
        let initial = compiled
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::Calibrated
                    && state
                        .presynaptic_movement_inhibition
                        .iter()
                        .all(|inhibition| {
                            *inhibition == PresynapticMovementInhibition::Disinhibited
                        })
            })
            .unwrap();
        for (action_id, target_leg, expected) in [
            (
                "descending-inhibition-engage",
                LegId::FrontLeft,
                PresynapticMovementInhibition::Engaged,
            ),
            (
                "descending-inhibition-release",
                LegId::FrontLeft,
                PresynapticMovementInhibition::Disinhibited,
            ),
            (
                "descending-inhibition-engage-front-right",
                LegId::FrontRight,
                PresynapticMovementInhibition::Engaged,
            ),
            (
                "descending-inhibition-release-front-right",
                LegId::FrontRight,
                PresynapticMovementInhibition::Disinhibited,
            ),
        ] {
            let row = compiled
                .row_index(
                    "central-rhythm-model",
                    "stance-nominal",
                    &initial.id,
                    action_id,
                )
                .unwrap();
            let next =
                &compiled.sensor_states[compiled.kernel.rows[row].outcomes[0].next_sensor_state];
            assert_eq!(
                next.presynaptic_movement_inhibition[target_leg.index()],
                expected
            );
            for leg in LegId::ALL.into_iter().filter(|leg| *leg != target_leg) {
                assert_eq!(
                    next.presynaptic_movement_inhibition[leg.index()],
                    PresynapticMovementInhibition::Disinhibited
                );
            }
            assert_eq!(next.movement_afferent, ChannelState::Intact);
            assert_eq!(next.position_afferent, ChannelState::Intact);
            assert_eq!(next.vibration_afferent, ChannelState::Intact);
            assert_eq!(next.acquisition, AcquisitionState::Calibrated);
        }
    }

    #[test]
    fn movement_channel_gate_does_not_change_presynaptic_inhibition_state() {
        let compiled = spec().compile().unwrap();
        for inhibition in [
            PresynapticMovementInhibition::Disinhibited,
            PresynapticMovementInhibition::Engaged,
        ] {
            let sensor = compiled
                .sensor_states
                .iter()
                .find(|state| {
                    state.position_afferent == ChannelState::Intact
                        && state.movement_afferent == ChannelState::Intact
                        && state.vibration_afferent == ChannelState::Intact
                        && state.contact_feedback == ChannelState::Intact
                        && state.acquisition == AcquisitionState::Calibrated
                        && state.presynaptic_movement_inhibition[LegId::FrontLeft.index()]
                            == inhibition
                        && LegId::ALL
                            .into_iter()
                            .filter(|leg| *leg != LegId::FrontLeft)
                            .all(|leg| {
                                state.presynaptic_movement_inhibition[leg.index()]
                                    == PresynapticMovementInhibition::Disinhibited
                            })
                })
                .unwrap();
            let row = compiled
                .row_index(
                    "central-rhythm-model",
                    "stance-nominal",
                    &sensor.id,
                    "movement-afferent-gate",
                )
                .unwrap();
            let next =
                &compiled.sensor_states[compiled.kernel.rows[row].outcomes[0].next_sensor_state];
            assert_eq!(next.movement_afferent, ChannelState::Gated);
            assert_eq!(
                next.presynaptic_movement_inhibition,
                sensor.presynaptic_movement_inhibition
            );
        }
    }

    #[test]
    fn non_descending_actions_cannot_declare_presynaptic_control_targets() {
        let mut input = spec();
        let action = input
            .actions
            .iter_mut()
            .find(|action| action.id == "movement-afferent-gate")
            .unwrap();
        action.target_leg = Some(LegId::FrontLeft);
        assert!(matches!(
            input.compile(),
            Err(AdapterError::InvalidPresynapticInterventionTarget { action_id })
                if action_id == "movement-afferent-gate"
        ));
    }

    #[test]
    fn descending_action_requires_an_explicit_target() {
        let mut input = spec();
        let action = input
            .actions
            .iter_mut()
            .find(|action| action.id == "descending-inhibition-engage")
            .unwrap();
        action.target_leg = None;
        action.presynaptic_inhibition_target = None;
        assert!(matches!(
            input.compile(),
            Err(AdapterError::InvalidPresynapticInterventionTarget { action_id })
                if action_id == "descending-inhibition-engage"
        ));
    }

    #[test]
    fn compiler_rejects_cross_modal_effect_in_a_movement_gate_row() {
        let mut input = spec();
        let base_sensor = input
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::Calibrated
            })
            .unwrap()
            .id
            .clone();
        let wrong_target = input
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Gated
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::Calibrated
            })
            .unwrap()
            .id
            .clone();
        let row = input
            .transition_rows
            .iter_mut()
            .find(|row| {
                row.hypothesis_id == "central-rhythm-model"
                    && row.physical_state_id == "stance-nominal"
                    && row.sensor_state_id == base_sensor
                    && row.action_id == "movement-afferent-gate"
            })
            .unwrap();
        row.outcomes[0].next_sensor_state_id = wrong_target;
        assert!(matches!(
            input.compile(),
            Err(AdapterError::ActionSemanticsViolation { action_id, .. })
                if action_id == "movement-afferent-gate"
        ));
    }

    #[test]
    fn washout_clears_physical_carryover_without_clearing_sensor_state() {
        let compiled = spec().compile().unwrap();
        let sensor = compiled
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::PositiveOffset
            })
            .unwrap();
        let row_index = compiled
            .row_index(
                "central-rhythm-model",
                "swing-loaded",
                &sensor.id,
                "washout-carryover",
            )
            .unwrap();
        let outcome = &compiled.kernel.rows[row_index].outcomes[0];
        let next_physical = &compiled.physical_states[outcome.next_physical_state];
        let next_sensor = &compiled.sensor_states[outcome.next_sensor_state];
        assert_eq!(next_physical.carryover, PhysicalCarryover::Baseline);
        assert_eq!(next_sensor, sensor);
    }

    #[test]
    fn physical_load_perturbation_does_not_reset_sensor_drift() {
        let compiled = spec().compile().unwrap();
        let sensor = compiled
            .sensor_states
            .iter()
            .find(|state| {
                state.position_afferent == ChannelState::Intact
                    && state.movement_afferent == ChannelState::Intact
                    && state.vibration_afferent == ChannelState::Intact
                    && state.contact_feedback == ChannelState::Intact
                    && state.acquisition == AcquisitionState::PositiveOffset
            })
            .unwrap();
        let row_index = compiled
            .row_index(
                "central-rhythm-model",
                "stance-nominal",
                &sensor.id,
                "contact-load-perturbation",
            )
            .unwrap();
        let outcome = &compiled.kernel.rows[row_index].outcomes[0];
        let next_physical = &compiled.physical_states[outcome.next_physical_state];
        let next_sensor = &compiled.sensor_states[outcome.next_sensor_state];
        assert_eq!(next_physical.id, "swing-loaded");
        assert_eq!(next_physical.behavioral_context, BehavioralContext::Walking);
        assert_eq!(next_physical.load_state, LoadState::Elevated);
        assert_eq!(next_sensor.acquisition, AcquisitionState::PositiveOffset);
        assert_eq!(next_sensor, sensor);
    }

    #[test]
    fn joint_optimizer_resolves_synthetic_sensor_physical_confounding() {
        let compiled = spec().compile().unwrap();
        let scores =
            rank_by_one_step_eig(&compiled.kernel, &compiled.kernel.initial_belief).unwrap();
        assert!(
            scores
                .iter()
                .find(|score| score.action_id == "active-movement-probe")
                .unwrap()
                .expected_information_gain_nats
                .abs()
                < 1.0e-12
        );
        assert!(
            scores
                .iter()
                .find(|score| score.action_id == "acquisition-reference-calibration")
                .unwrap()
                .expected_information_gain_nats
                .abs()
                < 1.0e-12
        );

        let exact = optimize_joint_kernel(
            &compiled.kernel,
            &SolverConfig {
                horizon: 2,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        let identifying = exact
            .frontier
            .iter()
            .find(|point| {
                point.correct_identification_probability == Rational::new(3, 4).unwrap()
                    && point.false_identification_probability == Rational::new(1, 4).unwrap()
                    && point.hazard_probability == Rational::zero()
                    && point.abstention_probability == Rational::zero()
            })
            .expect("reference calibration should make the noisy synthetic readout informative");
        match &identifying.plan {
            PolicyPlan::Action { action, branches } => {
                assert_eq!(
                    compiled.kernel.action_ids[*action],
                    "acquisition-reference-calibration"
                );
                assert_eq!(branches.len(), 1);
                match branches[0].plan.as_ref() {
                    PolicyPlan::Action {
                        action: next_action,
                        ..
                    } => {
                        assert_eq!(
                            compiled.kernel.action_ids[*next_action],
                            "active-movement-probe"
                        );
                    }
                    other => panic!("expected follow-up measurement, got {other:?}"),
                }
            }
            other => panic!("expected reference action first, got {other:?}"),
        }
    }

    #[test]
    fn measured_sources_require_dataset_and_release_identity() {
        let mut input = spec_with_measured_source();
        let source = input
            .source_manifests
            .iter_mut()
            .find(|source| source.id == "peer-reviewed-neural-source")
            .unwrap();
        source.dataset_id = None;
        assert!(matches!(
            input.compile(),
            Err(AdapterError::MissingMeasurementSnapshot { source_id })
                if source_id == "peer-reviewed-neural-source"
        ));
    }

    #[test]
    fn in_silico_evidence_requires_a_simulation_code_source() {
        let mut input = spec();
        input.source_manifests.push(SourceManifest {
            id: "in-silico-neural-source".to_owned(),
            kind: SourceKind::NeuralMeasurement,
            evidence_class: EvidenceClass::InSilicoGenerated,
            citation_or_source_uri: "https://example.org/synthetic-simulation".to_owned(),
            dataset_id: Some("simulated-recordings".to_owned()),
            release_or_materialization: Some("sim-v1".to_owned()),
            sha256: Some(sha('8')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        });
        input.source_pins.push(SourceDigestPin {
            source_manifest_id: "in-silico-neural-source".to_owned(),
            expected_sha256: sha('8'),
        });
        input.hypotheses[0]
            .source_manifest_ids
            .push("in-silico-neural-source".to_owned());
        input.hypotheses[0].claim_ceiling = ClaimCeiling::InSilicoReproduction;
        assert!(matches!(
            input.compile(),
            Err(AdapterError::InvalidEvidenceClaim {
                claim: ClaimCeiling::InSilicoReproduction,
                ..
            })
        ));
    }

    #[test]
    fn source_materialization_and_digest_pins_are_fail_closed() {
        let mut input = spec();
        input.source_manifests.push(SourceManifest {
            id: "fanc-connectome-v840".to_owned(),
            kind: SourceKind::Connectome,
            evidence_class: EvidenceClass::ConnectomeDerived,
            citation_or_source_uri: "https://doi.org/10.1038/s41586-025-09554-2".to_owned(),
            dataset_id: Some("FANC".to_owned()),
            release_or_materialization: Some("CAVE-v840".to_owned()),
            sha256: Some(sha('b')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        });
        input.source_pins.push(SourceDigestPin {
            source_manifest_id: "fanc-connectome-v840".to_owned(),
            expected_sha256: sha('b'),
        });
        input.hypotheses[0]
            .source_manifest_ids
            .push("fanc-connectome-v840".to_owned());
        input.hypotheses[0].claim_ceiling = ClaimCeiling::AnatomicalConnectomeConstrained;
        assert!(input.compile().is_ok());

        input
            .source_manifests
            .iter_mut()
            .find(|source| source.id == "fanc-connectome-v840")
            .unwrap()
            .sha256 = Some(sha('c'));
        assert!(matches!(
            input.compile(),
            Err(AdapterError::SourcePinMismatch { .. })
        ));
    }

    #[test]
    fn every_non_synthetic_source_requires_a_sha256_pin() {
        let mut input = spec();
        input.source_manifests.push(SourceManifest {
            id: "connectome-unpinned".to_owned(),
            kind: SourceKind::Connectome,
            evidence_class: EvidenceClass::ConnectomeDerived,
            citation_or_source_uri: "doi:synthetic-connectome-source".to_owned(),
            dataset_id: Some("fixture-connectome".to_owned()),
            release_or_materialization: Some("fixture-v1".to_owned()),
            sha256: Some(sha('e')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        });
        input.hypotheses[0]
            .source_manifest_ids
            .push("connectome-unpinned".to_owned());
        input.hypotheses[0].claim_ceiling = ClaimCeiling::AnatomicalConnectomeConstrained;
        assert!(matches!(
            input.compile(),
            Err(AdapterError::UnpinnedNonSyntheticSource { source_id })
                if source_id == "connectome-unpinned"
        ));
    }

    #[test]
    fn trial_evidence_tier_must_match_its_source_manifest() {
        let mut input = spec();
        input.source_manifests.push(SourceManifest {
            id: "peer-reviewed-neural-source".to_owned(),
            kind: SourceKind::NeuralMeasurement,
            evidence_class: EvidenceClass::PeerReviewedMeasured,
            citation_or_source_uri: "https://example.org/synthetic-measurement-fixture".to_owned(),
            dataset_id: Some("synthetic-recording-set".to_owned()),
            release_or_materialization: Some("fixture-v1".to_owned()),
            sha256: Some(sha('7')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        });
        input.source_pins.push(SourceDigestPin {
            source_manifest_id: "peer-reviewed-neural-source".to_owned(),
            expected_sha256: sha('7'),
        });
        let compiled = input.compile().unwrap();
        let receipt = LocomotionTrialReceipt {
            sample_id: "measured-sample-2".to_owned(),
            assigned_action_id: "movement-afferent-gate".to_owned(),
            delivered_protocol_id: "protocol-movement-gate".to_owned(),
            delivery_verified: true,
            delivery_fidelity: Rational::one(),
            conditioning_state_digest: sha('a'),
            animal_or_preparation_id: Some("animal-1".to_owned()),
            session_id: "session-1".to_owned(),
            source_manifest_id: "synthetic-protocols".to_owned(),
            evidence_class: EvidenceClass::PeerReviewedMeasured,
        };
        assert!(matches!(
            compiled.validate_trial_receipt(&receipt),
            Err(AdapterError::EvidenceSourceMismatch { .. })
        ));
    }

    fn spec_with_measured_source() -> LocomotionKernelSpec {
        let mut input = spec();
        input.source_manifests.push(SourceManifest {
            id: "peer-reviewed-neural-source".to_owned(),
            kind: SourceKind::NeuralMeasurement,
            evidence_class: EvidenceClass::PeerReviewedMeasured,
            citation_or_source_uri: "https://example.org/synthetic-measurement-fixture".to_owned(),
            dataset_id: Some("synthetic-recording-set".to_owned()),
            release_or_materialization: Some("fixture-v1".to_owned()),
            sha256: Some(sha('7')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        });
        input.source_pins.push(SourceDigestPin {
            source_manifest_id: "peer-reviewed-neural-source".to_owned(),
            expected_sha256: sha('7'),
        });
        input
    }

    fn measured_receipt(sample_id: &str, history_digest: char) -> LocomotionTrialReceipt {
        LocomotionTrialReceipt {
            sample_id: sample_id.to_owned(),
            assigned_action_id: "movement-afferent-gate".to_owned(),
            delivered_protocol_id: "protocol-movement-gate".to_owned(),
            delivery_verified: true,
            delivery_fidelity: Rational::one(),
            conditioning_state_digest: sha(history_digest),
            animal_or_preparation_id: Some("animal-1".to_owned()),
            session_id: "session-1".to_owned(),
            source_manifest_id: "peer-reviewed-neural-source".to_owned(),
            evidence_class: EvidenceClass::PeerReviewedMeasured,
        }
    }

    #[test]
    fn receipt_delivery_fidelity_must_meet_preregistered_minimum() {
        let compiled = spec().compile().unwrap();
        let mut receipt = LocomotionTrialReceipt {
            sample_id: "low-fidelity-sample".to_owned(),
            assigned_action_id: "movement-afferent-gate".to_owned(),
            delivered_protocol_id: "protocol-movement-gate".to_owned(),
            delivery_verified: true,
            delivery_fidelity: Rational::new(4, 5).unwrap(),
            conditioning_state_digest: sha('a'),
            animal_or_preparation_id: None,
            session_id: "synthetic-session".to_owned(),
            source_manifest_id: "synthetic-protocols".to_owned(),
            evidence_class: EvidenceClass::SyntheticFixture,
        };
        assert!(matches!(
            compiled.validate_trial_receipt(&receipt),
            Err(AdapterError::InvalidTrialReceipt(
                "delivered protocol fidelity is below the pre-registered minimum"
            ))
        ));
        receipt.delivery_fidelity = Rational::new(19, 20).unwrap();
        assert!(compiled.validate_trial_receipt(&receipt).is_ok());
    }

    #[test]
    fn receipt_requires_a_valid_pre_outcome_history_digest() {
        let compiled = spec_with_measured_source().compile().unwrap();
        let mut receipt = measured_receipt("sample-history-test", 'a');
        receipt.conditioning_state_digest = "history-hash-not-sha256".to_owned();
        assert_eq!(
            compiled.validate_trial_receipt(&receipt),
            Err(AdapterError::InvalidConditioningStateDigest)
        );
    }

    #[test]
    fn repeated_cluster_samples_are_allowed_but_duplicate_sample_ids_are_rejected() {
        let compiled = spec_with_measured_source().compile().unwrap();
        let first = measured_receipt("sample-1", 'a');
        let second = measured_receipt("sample-2", 'b');
        assert!(
            compiled
                .validate_trial_receipts(&[first.clone(), second.clone()])
                .is_ok()
        );
        assert_eq!(
            compiled.validate_trial_receipts(&[first.clone(), first]),
            Err(AdapterError::DuplicateSampleId("sample-1".to_owned()))
        );
        assert_eq!(second.animal_or_preparation_id, Some("animal-1".to_owned()));
        assert_eq!(second.session_id, "session-1");
        assert_ne!(
            second.conditioning_state_digest,
            measured_receipt("sample-3", 'a').conditioning_state_digest
        );
    }

    #[test]
    fn connectome_anatomy_cannot_be_substituted_for_trial_measurements() {
        let mut input = spec();
        input.source_manifests.push(SourceManifest {
            id: "connectome-v888".to_owned(),
            kind: SourceKind::Connectome,
            evidence_class: EvidenceClass::ConnectomeDerived,
            citation_or_source_uri: "doi:synthetic-connectome-source".to_owned(),
            dataset_id: Some("BANC".to_owned()),
            release_or_materialization: Some("CAVE-v888".to_owned()),
            sha256: Some(sha('f')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        });
        input.source_pins.push(SourceDigestPin {
            source_manifest_id: "connectome-v888".to_owned(),
            expected_sha256: sha('f'),
        });
        let compiled = input.compile().unwrap();
        let receipt = LocomotionTrialReceipt {
            sample_id: "anatomy-not-outcome".to_owned(),
            assigned_action_id: "movement-afferent-gate".to_owned(),
            delivered_protocol_id: "protocol-movement-gate".to_owned(),
            delivery_verified: true,
            delivery_fidelity: Rational::one(),
            conditioning_state_digest: sha('a'),
            animal_or_preparation_id: None,
            session_id: "session-1".to_owned(),
            source_manifest_id: "connectome-v888".to_owned(),
            evidence_class: EvidenceClass::ConnectomeDerived,
        };
        assert_eq!(
            compiled.validate_trial_receipt(&receipt),
            Err(AdapterError::AnatomicalEvidenceCannotScoreTrial)
        );
    }

    #[test]
    fn source_snapshot_cannot_omit_unknown_endpoint_inventory() {
        let mut input = spec();
        input.source_manifests.push(SourceManifest {
            id: "connectome-no-inventory".to_owned(),
            kind: SourceKind::Connectome,
            evidence_class: EvidenceClass::ConnectomeDerived,
            citation_or_source_uri: "doi:synthetic-connectome-source".to_owned(),
            dataset_id: Some("fixture-connectome".to_owned()),
            release_or_materialization: Some("fixture-v1".to_owned()),
            sha256: Some(sha('1')),
            unknown_endpoint_inventory_complete: false,
            unknown_endpoints: vec![],
        });
        input.source_pins.push(SourceDigestPin {
            source_manifest_id: "connectome-no-inventory".to_owned(),
            expected_sha256: sha('1'),
        });
        input.hypotheses[0]
            .source_manifest_ids
            .push("connectome-no-inventory".to_owned());
        input.hypotheses[0].claim_ceiling = ClaimCeiling::AnatomicalConnectomeConstrained;
        assert!(matches!(
            input.compile(),
            Err(AdapterError::IncompleteUnknownEndpointInventory { .. })
        ));
    }

    #[test]
    fn anatomy_alone_cannot_support_a_measured_neural_claim() {
        let mut input = spec();
        input.source_manifests.push(SourceManifest {
            id: "connectome-only".to_owned(),
            kind: SourceKind::Connectome,
            evidence_class: EvidenceClass::ConnectomeDerived,
            citation_or_source_uri: "doi:synthetic-connectome-source".to_owned(),
            dataset_id: Some("fixture-connectome".to_owned()),
            release_or_materialization: Some("fixture-v1".to_owned()),
            sha256: Some(sha('d')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        });
        input.source_pins.push(SourceDigestPin {
            source_manifest_id: "connectome-only".to_owned(),
            expected_sha256: sha('d'),
        });
        input.hypotheses[0]
            .source_manifest_ids
            .push("connectome-only".to_owned());
        input.hypotheses[0].claim_ceiling = ClaimCeiling::PeerReviewedMeasuredNeuralReproduction;
        assert!(matches!(
            input.compile(),
            Err(AdapterError::InvalidEvidenceClaim { .. })
        ));
    }

    #[test]
    fn assignment_and_delivered_protocol_are_not_conflated() {
        let compiled = spec().compile().unwrap();
        let receipt = LocomotionTrialReceipt {
            sample_id: "synthetic-sample-1".to_owned(),
            assigned_action_id: "movement-afferent-gate".to_owned(),
            delivered_protocol_id: "protocol-position-perturbation".to_owned(),
            delivery_verified: true,
            delivery_fidelity: Rational::one(),
            conditioning_state_digest: sha('a'),
            animal_or_preparation_id: None,
            session_id: "synthetic-session-1".to_owned(),
            source_manifest_id: "synthetic-protocols".to_owned(),
            evidence_class: EvidenceClass::SyntheticFixture,
        };
        assert!(matches!(
            compiled.validate_trial_receipt(&receipt),
            Err(AdapterError::AssignmentDeliveryMismatch { .. })
        ));
    }

    #[test]
    fn measured_receipts_require_an_independent_replication_unit() {
        let mut input = spec();
        input.source_manifests.push(SourceManifest {
            id: "peer-reviewed-neural-source".to_owned(),
            kind: SourceKind::NeuralMeasurement,
            evidence_class: EvidenceClass::PeerReviewedMeasured,
            citation_or_source_uri: "https://example.org/synthetic-measurement-fixture".to_owned(),
            dataset_id: Some("synthetic-recording-set".to_owned()),
            release_or_materialization: Some("fixture-v1".to_owned()),
            sha256: Some(sha('7')),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        });
        input.source_pins.push(SourceDigestPin {
            source_manifest_id: "peer-reviewed-neural-source".to_owned(),
            expected_sha256: sha('7'),
        });
        let compiled = input.compile().unwrap();
        let receipt = LocomotionTrialReceipt {
            sample_id: "measured-sample-1".to_owned(),
            assigned_action_id: "movement-afferent-gate".to_owned(),
            delivered_protocol_id: "protocol-movement-gate".to_owned(),
            delivery_verified: true,
            delivery_fidelity: Rational::one(),
            conditioning_state_digest: sha('a'),
            animal_or_preparation_id: None,
            session_id: "session-1".to_owned(),
            source_manifest_id: "peer-reviewed-neural-source".to_owned(),
            evidence_class: EvidenceClass::PeerReviewedMeasured,
        };
        assert_eq!(
            compiled.validate_trial_receipt(&receipt),
            Err(AdapterError::InvalidTrialReceipt(
                "measured trial lacks animal/preparation replication identity"
            ))
        );
    }
}
