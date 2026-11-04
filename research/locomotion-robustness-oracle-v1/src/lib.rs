// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Matched, tail-aware evaluation of degraded-loop embodied locomotion.
//!
//! This crate keeps nominal task performance, worst-tail performance,
//! recovery/censoring and hard failures as separate scorecard dimensions.
//! It does not infer causality, authenticate external artifacts, bootstrap
//! confidence intervals, or determine whether a biological intervention is
//! ethically or experimentally feasible.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const MAX_EPISODES: usize = 200_000;
const MAX_MODELS: usize = 64;
const MAX_SCENARIOS: usize = 10_000;
const MAX_IDENTIFIER_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelFamily {
    PhenomenologicalController,
    RuleBasedController,
    LearnedController,
    ConnectomeCircuit,
    ConnectomeGroundedReflex,
    ConnectomeHybrid,
    MuscleActuatedController,
    ConnectomeMuscleHybrid,
    TorqueActuatedWholeBody,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelEvidenceTier {
    Synthetic,
    Phenomenological,
    ConnectomeConstrained,
    PerturbationGrounded,
    ReplicatedBiological,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvaluationTier {
    SyntheticHarness,
    InSilicoEmbodied,
    MeasuredIntervention,
    IndependentReplication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioFamily {
    FlatGround,
    ComplexTerrain,
    SensoryLoss,
    SensorNoise,
    SensorDelay,
    SensorDropout,
    CentralDriveIntervention,
    LocalReflexIntervention,
    LoadPerturbation,
    MechanicalPerturbation,
    MorphologyShift,
    CombinedIntervention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationReason {
    TargetReached,
    HorizonCompleted,
    Fell,
    TimedOut,
    NumericalFailure,
    PhysicsViolation,
    Aborted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStatus {
    /// No perturbation was applied in this episode.
    NotApplicable,
    /// Recovery was observed; latency is measured from perturbation onset.
    Recovered { latency_ticks: u64 },
    /// The full planned horizon elapsed without observing recovery.
    NotRecoveredByHorizon,
    /// The episode terminated before the planned horizon and recovery was absent.
    NotRecoveredBeforeTermination,
    /// Recovery was not measured; this is not equivalent to non-recovery.
    Unobserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MechanismRecoveryStatus {
    NotApplicable,
    Restored { latency_ticks: u64 },
    NotRestoredByHorizon,
    NotRestoredBeforeTermination,
    Unobserved,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobustnessProtocol {
    pub benchmark_id: String,
    pub protocol_sha256: String,
    pub split_sha256: String,
    pub scenario_catalog_sha256: String,
    pub scoring_plan_sha256: String,
    /// Preregistered schema for mechanism-state distance; None disables this measure.
    pub mechanism_state_schema_sha256: Option<String>,
    /// Lower-tail CVaR fraction, in basis points (1000 = worst 10%).
    pub lower_tail_fraction_basis_points: u16,
    pub minimum_episodes_per_scenario: usize,
    pub minimum_episodes_per_individual: usize,
    pub minimum_individuals_per_scenario: usize,
    pub minimum_paired_episodes: usize,
    /// Absolute tolerance used only to classify paired score ties.
    pub paired_score_tie_tolerance: f64,
    /// Preregistered normalized functional-state distance for task recovery.
    pub functional_recovery_tolerance: f64,
    /// Preregistered normalized mechanism-state distance for mechanism restoration.
    pub mechanism_recovery_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeRecord {
    pub model_id: String,
    pub model_family: ModelFamily,
    pub model_evidence_tier: ModelEvidenceTier,
    pub evaluation_tier: EvaluationTier,

    pub scenario_id: String,
    pub scenario_family: ScenarioFamily,
    /// Integer severity is meaningful only within its declared unit and family.
    pub perturbation_severity_microunits: u64,
    pub perturbation_unit: String,

    pub individual_id: String,
    pub trial_id: String,
    pub seed: u64,

    pub protocol_sha256: String,
    pub split_sha256: String,
    pub scoring_plan_sha256: String,
    pub scenario_sha256: String,
    pub initial_state_sha256: String,
    pub body_model_sha256: String,
    pub morphology_sha256: String,
    pub physics_sha256: String,
    pub environment_sha256: String,
    pub sensor_schema_sha256: String,
    pub solver_sha256: String,
    pub hardware_sha256: String,
    pub code_sha256: String,
    pub parameter_sha256: String,
    pub controller_config_sha256: String,
    pub connectome_sha256: Option<String>,
    pub biological_source_sha256: Option<String>,
    pub sensor_mapping_sha256: Option<String>,
    pub effector_mapping_sha256: Option<String>,
    pub muscle_mapping_sha256: Option<String>,
    pub raw_trajectory_sha256: String,
    pub metric_payload_sha256: String,
    pub run_receipt_sha256: String,
    pub measured_dataset_sha256: Option<String>,
    pub independent_replication_sha256: Option<String>,

    pub horizon_ticks: u64,
    pub completed_ticks: u64,
    pub control_compute_budget_micros: u64,
    pub training_budget_steps: u64,
    pub perturbation_onset_tick: Option<u64>,
    /// Task-level recovery, independent of mechanism restoration.
    pub recovery: RecoveryStatus,
    /// Whether the measured sensorimotor mechanism returned to its nominal regime.
    pub mechanism_recovery: MechanismRecoveryStatus,
    /// Normalized functional-state distance at the recovery assessment point.
    pub functional_state_deviation_from_nominal: Option<f64>,
    /// Schema for the measured neural/sensory/motor/effector mechanism state.
    pub mechanism_state_schema_sha256: Option<String>,
    /// Normalized mechanism-state distance at the recovery assessment point.
    pub mechanism_state_deviation_from_nominal: Option<f64>,

    /// Outcome score produced by the frozen scoring plan, in [0, 1].
    pub task_score: f64,
    pub success: bool,
    pub termination: TerminationReason,

    pub progress_distance_m: f64,
    pub body_pose_rmse_m: f64,
    pub joint_angle_rmse_rad: f64,
    pub slip_distance_m: f64,
    pub energy_j: f64,
    pub actuator_saturation_fraction: f64,

    pub fall_count: u32,
    pub numerical_failure_count: u32,
    pub physics_violation_count: u32,
    pub unclassified_failure_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RobustnessError {
    InvalidIdentifier(&'static str),
    InvalidDigest(&'static str),
    InvalidProtocol,
    InvalidProbability,
    InvalidMetric,
    InvalidOutcomeTermination,
    InvalidRecoveryRecord,
    MissingEvidenceArtifact(&'static str),
    EvidenceTierMismatch,
    ProtocolMismatch,
    ScenarioDefinitionMismatch,
    ModelDefinitionMismatch,
    DuplicateTrial,
    EmptyEpisodes,
    ResourceLimitExceeded,
    TooFewEpisodes {
        observed: usize,
        minimum: usize,
    },
    TooFewIndividualEpisodes {
        individual_id: String,
        observed: usize,
        minimum: usize,
    },
    TooFewIndividuals {
        observed: usize,
        minimum: usize,
    },
    ModelNotFound(String),
    PairSetMismatch,
    TooFewPairedEpisodes {
        observed: usize,
        minimum: usize,
    },
    NonFiniteSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndividualScorecard {
    pub individual_id: String,
    pub morphology_sha256: String,
    pub episode_count: usize,
    pub success_rate: f64,
    pub mean_task_score: f64,
    pub lower_tail_cvar_task_score: f64,
    pub episodes_with_numerical_failures: usize,
    pub episodes_with_physics_violations: usize,
    pub episodes_with_unclassified_failures: usize,
    pub functional_recovery_episodes: usize,
    pub mechanism_restoration_episodes: usize,
    pub compensation_without_mechanism_restoration_episodes: usize,
    pub unobserved_mechanism_recovery_episodes: usize,
    pub hard_failure_gate_passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioScorecard {
    pub model_id: String,
    pub model_family: ModelFamily,
    pub model_evidence_tier: ModelEvidenceTier,
    pub evaluation_tier: EvaluationTier,
    pub scenario_id: String,
    pub scenario_family: ScenarioFamily,
    pub perturbation_severity_microunits: u64,
    pub perturbation_unit: String,

    pub episode_count: usize,
    pub individual_count: usize,
    /// Episode-weighted success and score; heavily sampled individuals contribute more.
    pub success_rate: f64,
    pub mean_task_score: f64,
    pub lower_tail_cvar_task_score: f64,
    /// Equal-weight mean across individuals, independent of trials per individual.
    pub individual_balanced_success_rate: f64,
    pub individual_balanced_mean_task_score: f64,
    /// Lower-tail CVaR of equal-weight per-individual mean task scores.
    pub individual_balanced_lower_tail_cvar_task_score: f64,
    pub worst_individual_success_rate: f64,
    pub worst_success_individual_id: String,
    pub worst_individual_lower_tail_cvar_task_score: f64,
    pub worst_cvar_individual_id: String,
    /// Sorted by individual ID; different morphologies remain visible.
    pub individual_scorecards: Vec<IndividualScorecard>,

    pub mean_progress_distance_m: f64,
    pub mean_body_pose_rmse_m: f64,
    pub mean_joint_angle_rmse_rad: f64,
    pub median_slip_distance_m: f64,
    pub p95_slip_distance_m: f64,
    pub mean_energy_j: f64,
    pub mean_actuator_saturation_fraction: f64,

    pub recovery_applicable_episodes: usize,
    pub recovered_episodes: usize,
    pub not_recovered_by_horizon_episodes: usize,
    pub not_recovered_before_termination_episodes: usize,
    pub unobserved_recovery_episodes: usize,
    pub median_recovery_latency_ticks: Option<f64>,
    pub p95_recovery_latency_ticks: Option<u64>,
    pub mechanism_recovery_applicable_episodes: usize,
    pub mechanism_restored_episodes: usize,
    pub mechanism_not_restored_by_horizon_episodes: usize,
    pub mechanism_not_restored_before_termination_episodes: usize,
    pub unobserved_mechanism_recovery_episodes: usize,
    pub median_mechanism_recovery_latency_ticks: Option<f64>,
    pub p95_mechanism_recovery_latency_ticks: Option<u64>,
    /// Functional recovery without measured restoration of the nominal mechanism.
    pub compensation_without_mechanism_restoration_episodes: usize,
    /// Both task function and measured mechanism returned to their nominal regimes.
    pub joint_function_and_mechanism_recovery_episodes: usize,

    pub episodes_with_falls: usize,
    pub total_fall_count: u64,
    pub episodes_with_numerical_failures: usize,
    pub total_numerical_failure_count: u64,
    pub episodes_with_physics_violations: usize,
    pub total_physics_violation_count: u64,
    pub episodes_with_unclassified_failures: usize,
    pub total_unclassified_failure_count: u64,
    /// False whenever any numerical, physics or unclassified hard failure occurred.
    /// This is a promotion gate, not a task-score adjustment.
    pub hard_failure_gate_passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelWorstCase {
    pub model_id: String,
    pub scenario_count: usize,
    pub worst_success_rate: f64,
    pub worst_success_scenario_id: String,
    pub worst_lower_tail_cvar_task_score: f64,
    pub worst_cvar_scenario_id: String,
    pub total_numerical_failure_episodes: u64,
    pub total_physics_violation_episodes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobustnessReport {
    pub protocol_sha256: String,
    pub split_sha256: String,
    pub scoring_plan_sha256: String,
    /// Sorted by model_id, then scenario_id.
    pub by_scenario: Vec<ScenarioScorecard>,
    /// Sorted by model_id. Worst success and worst tail score remain distinct.
    pub model_worst_case: Vec<ModelWorstCase>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedModelComparison {
    pub candidate_model_id: String,
    pub baseline_model_id: String,
    pub matched_episode_count: usize,
    pub candidate_mean_task_score: f64,
    pub baseline_mean_task_score: f64,
    pub mean_paired_task_score_delta: f64,
    pub median_paired_task_score_delta: f64,
    pub candidate_wins: usize,
    pub ties: usize,
    pub candidate_losses: usize,
    pub candidate_only_successes: usize,
    pub baseline_only_successes: usize,
    pub candidate_numerical_failure_episodes: usize,
    pub baseline_numerical_failure_episodes: usize,
    pub candidate_physics_violation_episodes: usize,
    pub baseline_physics_violation_episodes: usize,
    pub candidate_functional_recovery_episodes: usize,
    pub baseline_functional_recovery_episodes: usize,
    pub candidate_mechanism_restoration_episodes: usize,
    pub baseline_mechanism_restoration_episodes: usize,
    pub candidate_compensation_without_mechanism_restoration_episodes: usize,
    pub baseline_compensation_without_mechanism_restoration_episodes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ModelScenarioKey {
    model_id: String,
    scenario_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelSignature {
    family: ModelFamily,
    evidence_tier: ModelEvidenceTier,
    code_sha256: String,
    parameter_sha256: String,
    controller_config_sha256: String,
    connectome_sha256: Option<String>,
    biological_source_sha256: Option<String>,
    sensor_mapping_sha256: Option<String>,
    effector_mapping_sha256: Option<String>,
    muscle_mapping_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScenarioSignature {
    family: ScenarioFamily,
    severity_microunits: u64,
    unit: String,
    scenario_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvaluationSignature {
    body_model_sha256: String,
    physics_sha256: String,
    environment_sha256: String,
    sensor_schema_sha256: String,
    solver_sha256: String,
    hardware_sha256: String,
    mechanism_state_schema_sha256: Option<String>,
    horizon_ticks: u64,
    control_compute_budget_micros: u64,
    training_budget_steps: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MatchedTrialKey {
    scenario_id: String,
    scenario_family: ScenarioFamily,
    perturbation_severity_microunits: u64,
    perturbation_unit: String,
    individual_id: String,
    trial_id: String,
    seed: u64,
    evaluation_tier: EvaluationTier,
    initial_state_sha256: String,
    body_model_sha256: String,
    morphology_sha256: String,
    physics_sha256: String,
    environment_sha256: String,
    sensor_schema_sha256: String,
    solver_sha256: String,
    hardware_sha256: String,
    mechanism_state_schema_sha256: Option<String>,
    scenario_sha256: String,
    horizon_ticks: u64,
    control_compute_budget_micros: u64,
    training_budget_steps: u64,
    perturbation_onset_tick: Option<u64>,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl RobustnessProtocol {
    pub fn validate(&self) -> Result<(), RobustnessError> {
        if !valid_identifier(&self.benchmark_id) {
            return Err(RobustnessError::InvalidIdentifier("benchmark_id"));
        }
        for (digest, label) in [
            (&self.protocol_sha256, "protocol"),
            (&self.split_sha256, "split"),
            (&self.scenario_catalog_sha256, "scenario_catalog"),
            (&self.scoring_plan_sha256, "scoring_plan"),
        ] {
            if !valid_sha256(digest) {
                return Err(RobustnessError::InvalidDigest(label));
            }
        }
        validate_optional_digest(
            &self.mechanism_state_schema_sha256,
            "protocol_mechanism_state_schema",
        )?;
        if self.lower_tail_fraction_basis_points == 0
            || self.lower_tail_fraction_basis_points > 10_000
            || self.minimum_episodes_per_scenario == 0
            || self.minimum_episodes_per_individual == 0
            || self.minimum_individuals_per_scenario == 0
            || self.minimum_paired_episodes == 0
            || !self.paired_score_tie_tolerance.is_finite()
            || self.paired_score_tie_tolerance < 0.0
            || self.paired_score_tie_tolerance > 1.0
            || !self.functional_recovery_tolerance.is_finite()
            || self.functional_recovery_tolerance < 0.0
            || !self.mechanism_recovery_tolerance.is_finite()
            || self.mechanism_recovery_tolerance < 0.0
        {
            return Err(RobustnessError::InvalidProtocol);
        }
        Ok(())
    }
}

fn is_connectome_family(family: ModelFamily) -> bool {
    matches!(
        family,
        ModelFamily::ConnectomeCircuit
            | ModelFamily::ConnectomeGroundedReflex
            | ModelFamily::ConnectomeHybrid
            | ModelFamily::ConnectomeMuscleHybrid
    )
}

fn validate_optional_digest(
    digest: &Option<String>,
    label: &'static str,
) -> Result<(), RobustnessError> {
    if digest.as_ref().is_some_and(|value| !valid_sha256(value)) {
        return Err(RobustnessError::InvalidDigest(label));
    }
    Ok(())
}

fn validate_episode(
    protocol: &RobustnessProtocol,
    record: &EpisodeRecord,
) -> Result<(), RobustnessError> {
    for (value, label) in [
        (&record.model_id, "model_id"),
        (&record.scenario_id, "scenario_id"),
        (&record.perturbation_unit, "perturbation_unit"),
        (&record.individual_id, "individual_id"),
        (&record.trial_id, "trial_id"),
    ] {
        if !valid_identifier(value) {
            return Err(RobustnessError::InvalidIdentifier(label));
        }
    }

    for (digest, label) in [
        (&record.protocol_sha256, "episode_protocol"),
        (&record.split_sha256, "episode_split"),
        (&record.scoring_plan_sha256, "episode_scoring_plan"),
        (&record.scenario_sha256, "scenario"),
        (&record.initial_state_sha256, "initial_state"),
        (&record.body_model_sha256, "body_model"),
        (&record.morphology_sha256, "morphology"),
        (&record.physics_sha256, "physics"),
        (&record.environment_sha256, "environment"),
        (&record.sensor_schema_sha256, "sensor_schema"),
        (&record.solver_sha256, "solver"),
        (&record.hardware_sha256, "hardware"),
        (&record.code_sha256, "code"),
        (&record.parameter_sha256, "parameters"),
        (&record.controller_config_sha256, "controller_config"),
        (&record.raw_trajectory_sha256, "raw_trajectory"),
        (&record.metric_payload_sha256, "metric_payload"),
        (&record.run_receipt_sha256, "run_receipt"),
    ] {
        if !valid_sha256(digest) {
            return Err(RobustnessError::InvalidDigest(label));
        }
    }
    validate_optional_digest(&record.connectome_sha256, "connectome")?;
    validate_optional_digest(&record.biological_source_sha256, "biological_source")?;
    validate_optional_digest(&record.sensor_mapping_sha256, "sensor_mapping")?;
    validate_optional_digest(&record.effector_mapping_sha256, "effector_mapping")?;
    validate_optional_digest(&record.muscle_mapping_sha256, "muscle_mapping")?;
    validate_optional_digest(
        &record.mechanism_state_schema_sha256,
        "mechanism_state_schema",
    )?;
    if record.mechanism_state_schema_sha256.is_some()
        && record.mechanism_state_schema_sha256 != protocol.mechanism_state_schema_sha256
    {
        return Err(RobustnessError::ProtocolMismatch);
    }
    validate_optional_digest(&record.measured_dataset_sha256, "measured_dataset")?;
    validate_optional_digest(
        &record.independent_replication_sha256,
        "independent_replication",
    )?;

    if record.protocol_sha256 != protocol.protocol_sha256
        || record.split_sha256 != protocol.split_sha256
        || record.scoring_plan_sha256 != protocol.scoring_plan_sha256
    {
        return Err(RobustnessError::ProtocolMismatch);
    }

    if is_connectome_family(record.model_family)
        && record.model_evidence_tier < ModelEvidenceTier::ConnectomeConstrained
    {
        return Err(RobustnessError::EvidenceTierMismatch);
    }
    if (is_connectome_family(record.model_family)
        || record.model_evidence_tier == ModelEvidenceTier::ConnectomeConstrained)
        && (record.connectome_sha256.is_none()
            || record.biological_source_sha256.is_none()
            || record.sensor_mapping_sha256.is_none()
            || record.effector_mapping_sha256.is_none())
    {
        return Err(RobustnessError::MissingEvidenceArtifact(
            "connectome_or_mapping",
        ));
    }
    if matches!(
        record.model_family,
        ModelFamily::MuscleActuatedController | ModelFamily::ConnectomeMuscleHybrid
    ) && record.muscle_mapping_sha256.is_none()
    {
        return Err(RobustnessError::MissingEvidenceArtifact("muscle_mapping"));
    }
    if record.model_evidence_tier >= ModelEvidenceTier::PerturbationGrounded
        && record.biological_source_sha256.is_none()
    {
        return Err(RobustnessError::MissingEvidenceArtifact(
            "biological_source",
        ));
    }
    if record.model_evidence_tier == ModelEvidenceTier::PerturbationGrounded
        && record.evaluation_tier < EvaluationTier::MeasuredIntervention
    {
        return Err(RobustnessError::EvidenceTierMismatch);
    }
    if record.model_evidence_tier == ModelEvidenceTier::ReplicatedBiological
        && record.independent_replication_sha256.is_none()
    {
        return Err(RobustnessError::MissingEvidenceArtifact(
            "independent_replication",
        ));
    }
    if record.model_evidence_tier == ModelEvidenceTier::ReplicatedBiological
        && record.evaluation_tier != EvaluationTier::IndependentReplication
    {
        return Err(RobustnessError::EvidenceTierMismatch);
    }
    if record.evaluation_tier >= EvaluationTier::MeasuredIntervention
        && record.measured_dataset_sha256.is_none()
    {
        return Err(RobustnessError::MissingEvidenceArtifact("measured_dataset"));
    }
    if record.evaluation_tier == EvaluationTier::IndependentReplication
        && record.independent_replication_sha256.is_none()
    {
        return Err(RobustnessError::MissingEvidenceArtifact(
            "independent_replication",
        ));
    }

    if record.horizon_ticks == 0
        || record.completed_ticks > record.horizon_ticks
        || record.control_compute_budget_micros == 0
    {
        return Err(RobustnessError::InvalidMetric);
    }

    let finite_nonnegative = [
        record.task_score,
        record.progress_distance_m,
        record.body_pose_rmse_m,
        record.joint_angle_rmse_rad,
        record.slip_distance_m,
        record.energy_j,
        record.actuator_saturation_fraction,
    ];
    if finite_nonnegative.iter().any(|value| !value.is_finite())
        || !(0.0..=1.0).contains(&record.task_score)
        || !(0.0..=1.0).contains(&record.actuator_saturation_fraction)
        || record.progress_distance_m < 0.0
        || record.body_pose_rmse_m < 0.0
        || record.joint_angle_rmse_rad < 0.0
        || record.slip_distance_m < 0.0
        || record.energy_j < 0.0
    {
        return Err(RobustnessError::InvalidMetric);
    }

    match record.termination {
        TerminationReason::TargetReached if !record.success => {
            return Err(RobustnessError::InvalidOutcomeTermination);
        }
        TerminationReason::Fell if record.success || record.fall_count == 0 => {
            return Err(RobustnessError::InvalidOutcomeTermination);
        }
        TerminationReason::TimedOut if record.success => {
            return Err(RobustnessError::InvalidOutcomeTermination);
        }
        TerminationReason::NumericalFailure
            if record.success || record.numerical_failure_count == 0 =>
        {
            return Err(RobustnessError::InvalidOutcomeTermination);
        }
        TerminationReason::PhysicsViolation
            if record.success || record.physics_violation_count == 0 =>
        {
            return Err(RobustnessError::InvalidOutcomeTermination);
        }
        TerminationReason::Aborted if record.success => {
            return Err(RobustnessError::InvalidOutcomeTermination);
        }
        _ => {}
    }

    match (record.perturbation_onset_tick, record.recovery) {
        (None, RecoveryStatus::NotApplicable) => {
            if record.functional_state_deviation_from_nominal.is_some()
                || record.mechanism_recovery != MechanismRecoveryStatus::NotApplicable
                || record.mechanism_state_schema_sha256.is_some()
                || record.mechanism_state_deviation_from_nominal.is_some()
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
        }
        (None, _) => return Err(RobustnessError::InvalidRecoveryRecord),
        (Some(onset), RecoveryStatus::NotApplicable) => {
            let _ = onset;
            return Err(RobustnessError::InvalidRecoveryRecord);
        }
        (Some(onset), RecoveryStatus::Recovered { latency_ticks }) => {
            if onset > record.completed_ticks
                || onset.saturating_add(latency_ticks) > record.completed_ticks
                || record
                    .functional_state_deviation_from_nominal
                    .is_none_or(|distance| {
                        !distance.is_finite()
                            || distance < 0.0
                            || distance > protocol.functional_recovery_tolerance
                    })
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
        }
        (Some(onset), RecoveryStatus::NotRecoveredByHorizon) => {
            if onset > record.completed_ticks
                || record.completed_ticks != record.horizon_ticks
                || record
                    .functional_state_deviation_from_nominal
                    .is_none_or(|distance| {
                        !distance.is_finite() || distance <= protocol.functional_recovery_tolerance
                    })
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
        }
        (Some(onset), RecoveryStatus::NotRecoveredBeforeTermination) => {
            if onset > record.completed_ticks
                || record.completed_ticks >= record.horizon_ticks
                || record
                    .functional_state_deviation_from_nominal
                    .is_none_or(|distance| {
                        !distance.is_finite() || distance <= protocol.functional_recovery_tolerance
                    })
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
        }
        (Some(onset), RecoveryStatus::Unobserved) => {
            if onset > record.completed_ticks
                || record.functional_state_deviation_from_nominal.is_some()
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
        }
    }

    let mechanism_assessed = match record.mechanism_recovery {
        MechanismRecoveryStatus::NotApplicable => {
            if record.perturbation_onset_tick.is_some()
                || record.mechanism_state_schema_sha256.is_some()
                || record.mechanism_state_deviation_from_nominal.is_some()
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
            false
        }
        MechanismRecoveryStatus::Restored { latency_ticks } => {
            let onset = record
                .perturbation_onset_tick
                .ok_or(RobustnessError::InvalidRecoveryRecord)?;
            if onset.saturating_add(latency_ticks) > record.completed_ticks
                || record
                    .mechanism_state_deviation_from_nominal
                    .is_none_or(|distance| {
                        !distance.is_finite()
                            || distance < 0.0
                            || distance > protocol.mechanism_recovery_tolerance
                    })
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
            true
        }
        MechanismRecoveryStatus::NotRestoredByHorizon => {
            if record.perturbation_onset_tick.is_none()
                || record.completed_ticks != record.horizon_ticks
                || record
                    .mechanism_state_deviation_from_nominal
                    .is_none_or(|distance| {
                        !distance.is_finite() || distance <= protocol.mechanism_recovery_tolerance
                    })
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
            true
        }
        MechanismRecoveryStatus::NotRestoredBeforeTermination => {
            if record.perturbation_onset_tick.is_none()
                || record.completed_ticks >= record.horizon_ticks
                || record
                    .mechanism_state_deviation_from_nominal
                    .is_none_or(|distance| {
                        !distance.is_finite() || distance <= protocol.mechanism_recovery_tolerance
                    })
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
            true
        }
        MechanismRecoveryStatus::Unobserved => {
            if record.perturbation_onset_tick.is_none()
                || record.mechanism_state_deviation_from_nominal.is_some()
                || record.mechanism_state_schema_sha256.is_some()
            {
                return Err(RobustnessError::InvalidRecoveryRecord);
            }
            false
        }
    };
    if mechanism_assessed
        && (record.mechanism_state_schema_sha256.is_none()
            || protocol.mechanism_state_schema_sha256.is_none()
            || record.mechanism_state_schema_sha256 != protocol.mechanism_state_schema_sha256)
    {
        return Err(RobustnessError::MissingEvidenceArtifact(
            "mechanism_state_schema",
        ));
    }

    Ok(())
}

fn stable_mean(values: &[f64]) -> Result<f64, RobustnessError> {
    if values.is_empty() {
        return Err(RobustnessError::NonFiniteSummary);
    }
    let divisor = values.len() as f64;
    let result = values.iter().map(|value| value / divisor).sum::<f64>();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(RobustnessError::NonFiniteSummary)
    }
}

fn median_f64(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some(values[middle - 1] * 0.5 + values[middle] * 0.5)
    } else {
        Some(values[middle])
    }
}

fn percentile_nearest_rank_u64(
    values: &mut [u64],
    numerator: usize,
    denominator: usize,
) -> Option<u64> {
    if values.is_empty() || denominator == 0 || numerator == 0 || numerator > denominator {
        return None;
    }
    values.sort_unstable();
    let rank = (values.len() * numerator).div_ceil(denominator);
    values.get(rank.saturating_sub(1)).copied()
}

fn percentile_nearest_rank_f64(
    values: &mut [f64],
    numerator: usize,
    denominator: usize,
) -> Option<f64> {
    if values.is_empty() || denominator == 0 || numerator == 0 || numerator > denominator {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let rank = (values.len() * numerator).div_ceil(denominator);
    values.get(rank.saturating_sub(1)).copied()
}

fn lower_tail_cvar(values: &[f64], fraction_basis_points: u16) -> Result<f64, RobustnessError> {
    if values.is_empty() || fraction_basis_points == 0 || fraction_basis_points > 10_000 {
        return Err(RobustnessError::NonFiniteSummary);
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let numerator = ordered.len() * usize::from(fraction_basis_points);
    let tail_count = (numerator.div_ceil(10_000)).max(1);
    stable_mean(&ordered[..tail_count])
}

fn score_scenario(
    protocol: &RobustnessProtocol,
    records: &[&EpisodeRecord],
) -> Result<ScenarioScorecard, RobustnessError> {
    let first = records.first().ok_or(RobustnessError::EmptyEpisodes)?;
    if records.len() < protocol.minimum_episodes_per_scenario {
        return Err(RobustnessError::TooFewEpisodes {
            observed: records.len(),
            minimum: protocol.minimum_episodes_per_scenario,
        });
    }

    let mut individuals = BTreeSet::new();
    let mut episodes_by_individual: BTreeMap<String, Vec<&EpisodeRecord>> = BTreeMap::new();
    let mut task_scores = Vec::with_capacity(records.len());
    let mut successes = 0usize;
    let mut progress = Vec::with_capacity(records.len());
    let mut pose_errors = Vec::with_capacity(records.len());
    let mut joint_errors = Vec::with_capacity(records.len());
    let mut slips = Vec::with_capacity(records.len());
    let mut energies = Vec::with_capacity(records.len());
    let mut saturation = Vec::with_capacity(records.len());
    let mut recovered_latency = Vec::new();

    let mut recovery_applicable = 0usize;
    let mut not_recovered_by_horizon = 0usize;
    let mut not_recovered_before_termination = 0usize;
    let mut recovery_unobserved = 0usize;
    let mut mechanism_recovery_applicable = 0usize;
    let mut mechanism_not_restored_by_horizon = 0usize;
    let mut mechanism_not_restored_before_termination = 0usize;
    let mut mechanism_recovery_unobserved = 0usize;
    let mut mechanism_restored_latency = Vec::new();
    let mut compensation_without_mechanism_restoration = 0usize;
    let mut joint_function_and_mechanism_recovery = 0usize;

    let mut episodes_with_falls = 0usize;
    let mut total_fall_count = 0_u64;
    let mut episodes_with_numerical_failures = 0usize;
    let mut total_numerical_failure_count = 0_u64;
    let mut episodes_with_physics_violations = 0usize;
    let mut total_physics_violation_count = 0_u64;
    let mut episodes_with_unclassified_failures = 0usize;
    let mut total_unclassified_failure_count = 0_u64;

    let signature = ScenarioSignature {
        family: first.scenario_family,
        severity_microunits: first.perturbation_severity_microunits,
        unit: first.perturbation_unit.clone(),
        scenario_sha256: first.scenario_sha256.clone(),
    };
    let first_model_signature = ModelSignature {
        family: first.model_family,
        evidence_tier: first.model_evidence_tier,
        code_sha256: first.code_sha256.clone(),
        parameter_sha256: first.parameter_sha256.clone(),
        controller_config_sha256: first.controller_config_sha256.clone(),
        connectome_sha256: first.connectome_sha256.clone(),
        biological_source_sha256: first.biological_source_sha256.clone(),
        sensor_mapping_sha256: first.sensor_mapping_sha256.clone(),
        effector_mapping_sha256: first.effector_mapping_sha256.clone(),
        muscle_mapping_sha256: first.muscle_mapping_sha256.clone(),
    };
    let first_evaluation_signature = EvaluationSignature {
        body_model_sha256: first.body_model_sha256.clone(),
        physics_sha256: first.physics_sha256.clone(),
        environment_sha256: first.environment_sha256.clone(),
        sensor_schema_sha256: first.sensor_schema_sha256.clone(),
        solver_sha256: first.solver_sha256.clone(),
        hardware_sha256: first.hardware_sha256.clone(),
        mechanism_state_schema_sha256: first.mechanism_state_schema_sha256.clone(),
        horizon_ticks: first.horizon_ticks,
        control_compute_budget_micros: first.control_compute_budget_micros,
        training_budget_steps: first.training_budget_steps,
    };

    for record in records {
        if record.model_id != first.model_id
            || record.scenario_id != first.scenario_id
            || (ScenarioSignature {
                family: record.scenario_family,
                severity_microunits: record.perturbation_severity_microunits,
                unit: record.perturbation_unit.clone(),
                scenario_sha256: record.scenario_sha256.clone(),
            }) != signature
            || (ModelSignature {
                family: record.model_family,
                evidence_tier: record.model_evidence_tier,
                code_sha256: record.code_sha256.clone(),
                parameter_sha256: record.parameter_sha256.clone(),
                controller_config_sha256: record.controller_config_sha256.clone(),
                connectome_sha256: record.connectome_sha256.clone(),
                biological_source_sha256: record.biological_source_sha256.clone(),
                sensor_mapping_sha256: record.sensor_mapping_sha256.clone(),
                effector_mapping_sha256: record.effector_mapping_sha256.clone(),
                muscle_mapping_sha256: record.muscle_mapping_sha256.clone(),
            }) != first_model_signature
            || record.evaluation_tier != first.evaluation_tier
            || (EvaluationSignature {
                body_model_sha256: record.body_model_sha256.clone(),
                physics_sha256: record.physics_sha256.clone(),
                environment_sha256: record.environment_sha256.clone(),
                sensor_schema_sha256: record.sensor_schema_sha256.clone(),
                solver_sha256: record.solver_sha256.clone(),
                hardware_sha256: record.hardware_sha256.clone(),
                mechanism_state_schema_sha256: record.mechanism_state_schema_sha256.clone(),
                horizon_ticks: record.horizon_ticks,
                control_compute_budget_micros: record.control_compute_budget_micros,
                training_budget_steps: record.training_budget_steps,
            }) != first_evaluation_signature
        {
            return Err(RobustnessError::ScenarioDefinitionMismatch);
        }

        individuals.insert(record.individual_id.as_str());
        episodes_by_individual
            .entry(record.individual_id.clone())
            .or_default()
            .push(record);
        task_scores.push(record.task_score);
        if record.success {
            successes += 1;
        }
        progress.push(record.progress_distance_m);
        pose_errors.push(record.body_pose_rmse_m);
        joint_errors.push(record.joint_angle_rmse_rad);
        slips.push(record.slip_distance_m);
        energies.push(record.energy_j);
        saturation.push(record.actuator_saturation_fraction);

        match record.recovery {
            RecoveryStatus::NotApplicable => {}
            RecoveryStatus::Recovered { latency_ticks } => {
                recovery_applicable += 1;
                recovered_latency.push(latency_ticks);
            }
            RecoveryStatus::NotRecoveredByHorizon => {
                recovery_applicable += 1;
                not_recovered_by_horizon += 1;
            }
            RecoveryStatus::NotRecoveredBeforeTermination => {
                recovery_applicable += 1;
                not_recovered_before_termination += 1;
            }
            RecoveryStatus::Unobserved => {
                recovery_applicable += 1;
                recovery_unobserved += 1;
            }
        }
        match record.mechanism_recovery {
            MechanismRecoveryStatus::NotApplicable => {}
            MechanismRecoveryStatus::Restored { latency_ticks } => {
                mechanism_recovery_applicable += 1;
                mechanism_restored_latency.push(latency_ticks);
                if matches!(record.recovery, RecoveryStatus::Recovered { .. }) {
                    joint_function_and_mechanism_recovery += 1;
                }
            }
            MechanismRecoveryStatus::NotRestoredByHorizon => {
                mechanism_recovery_applicable += 1;
                mechanism_not_restored_by_horizon += 1;
                if matches!(record.recovery, RecoveryStatus::Recovered { .. }) {
                    compensation_without_mechanism_restoration += 1;
                }
            }
            MechanismRecoveryStatus::NotRestoredBeforeTermination => {
                mechanism_recovery_applicable += 1;
                mechanism_not_restored_before_termination += 1;
                if matches!(record.recovery, RecoveryStatus::Recovered { .. }) {
                    compensation_without_mechanism_restoration += 1;
                }
            }
            MechanismRecoveryStatus::Unobserved => {
                mechanism_recovery_applicable += 1;
                mechanism_recovery_unobserved += 1;
            }
        }

        if record.fall_count > 0 {
            episodes_with_falls += 1;
            total_fall_count += u64::from(record.fall_count);
        }
        if record.numerical_failure_count > 0 {
            episodes_with_numerical_failures += 1;
            total_numerical_failure_count += u64::from(record.numerical_failure_count);
        }
        if record.physics_violation_count > 0 {
            episodes_with_physics_violations += 1;
            total_physics_violation_count += u64::from(record.physics_violation_count);
        }
        if record.unclassified_failure_count > 0 {
            episodes_with_unclassified_failures += 1;
            total_unclassified_failure_count += u64::from(record.unclassified_failure_count);
        }
    }

    if individuals.len() < protocol.minimum_individuals_per_scenario {
        return Err(RobustnessError::TooFewIndividuals {
            observed: individuals.len(),
            minimum: protocol.minimum_individuals_per_scenario,
        });
    }

    let success_rate = successes as f64 / records.len() as f64;
    let mut individual_scorecards = Vec::with_capacity(episodes_by_individual.len());
    let mut individual_success_rates = Vec::with_capacity(episodes_by_individual.len());
    let mut individual_mean_task_scores = Vec::with_capacity(episodes_by_individual.len());

    for (individual_id, individual_episodes) in episodes_by_individual {
        if individual_episodes.len() < protocol.minimum_episodes_per_individual {
            return Err(RobustnessError::TooFewIndividualEpisodes {
                individual_id,
                observed: individual_episodes.len(),
                minimum: protocol.minimum_episodes_per_individual,
            });
        }
        let first_individual_episode = individual_episodes
            .first()
            .ok_or(RobustnessError::NonFiniteSummary)?;
        let morphology_sha256 = first_individual_episode.morphology_sha256.clone();
        if individual_episodes
            .iter()
            .any(|episode| episode.morphology_sha256 != morphology_sha256)
        {
            return Err(RobustnessError::ScenarioDefinitionMismatch);
        }

        let individual_scores = individual_episodes
            .iter()
            .map(|episode| episode.task_score)
            .collect::<Vec<_>>();
        let individual_success_count = individual_episodes
            .iter()
            .filter(|episode| episode.success)
            .count();
        let individual_success_rate =
            individual_success_count as f64 / individual_episodes.len() as f64;
        let individual_mean_task_score = stable_mean(&individual_scores)?;
        let individual_lower_tail_cvar = lower_tail_cvar(
            &individual_scores,
            protocol.lower_tail_fraction_basis_points,
        )?;
        let episodes_with_numerical_failures = individual_episodes
            .iter()
            .filter(|episode| episode.numerical_failure_count > 0)
            .count();
        let episodes_with_physics_violations = individual_episodes
            .iter()
            .filter(|episode| episode.physics_violation_count > 0)
            .count();
        let episodes_with_unclassified_failures = individual_episodes
            .iter()
            .filter(|episode| episode.unclassified_failure_count > 0)
            .count();
        let functional_recovery_episodes = individual_episodes
            .iter()
            .filter(|episode| matches!(episode.recovery, RecoveryStatus::Recovered { .. }))
            .count();
        let mechanism_restoration_episodes = individual_episodes
            .iter()
            .filter(|episode| {
                matches!(
                    episode.mechanism_recovery,
                    MechanismRecoveryStatus::Restored { .. }
                )
            })
            .count();
        let compensation_without_mechanism_restoration_episodes = individual_episodes
            .iter()
            .filter(|episode| {
                matches!(episode.recovery, RecoveryStatus::Recovered { .. })
                    && matches!(
                        episode.mechanism_recovery,
                        MechanismRecoveryStatus::NotRestoredByHorizon
                            | MechanismRecoveryStatus::NotRestoredBeforeTermination
                    )
            })
            .count();
        let unobserved_mechanism_recovery_episodes = individual_episodes
            .iter()
            .filter(|episode| {
                matches!(
                    episode.mechanism_recovery,
                    MechanismRecoveryStatus::Unobserved
                )
            })
            .count();

        individual_success_rates.push(individual_success_rate);
        individual_mean_task_scores.push(individual_mean_task_score);
        individual_scorecards.push(IndividualScorecard {
            individual_id,
            morphology_sha256,
            episode_count: individual_episodes.len(),
            success_rate: individual_success_rate,
            mean_task_score: individual_mean_task_score,
            lower_tail_cvar_task_score: individual_lower_tail_cvar,
            episodes_with_numerical_failures,
            episodes_with_physics_violations,
            episodes_with_unclassified_failures,
            functional_recovery_episodes,
            mechanism_restoration_episodes,
            compensation_without_mechanism_restoration_episodes,
            unobserved_mechanism_recovery_episodes,
            hard_failure_gate_passed: episodes_with_numerical_failures == 0
                && episodes_with_physics_violations == 0
                && episodes_with_unclassified_failures == 0,
        });
    }

    let worst_success_individual = individual_scorecards
        .iter()
        .min_by(|left, right| left.success_rate.total_cmp(&right.success_rate))
        .ok_or(RobustnessError::NonFiniteSummary)?;
    let worst_cvar_individual = individual_scorecards
        .iter()
        .min_by(|left, right| {
            left.lower_tail_cvar_task_score
                .total_cmp(&right.lower_tail_cvar_task_score)
        })
        .ok_or(RobustnessError::NonFiniteSummary)?;
    let individual_balanced_success_rate = stable_mean(&individual_success_rates)?;
    let individual_balanced_mean_task_score = stable_mean(&individual_mean_task_scores)?;
    let individual_balanced_lower_tail_cvar_task_score = lower_tail_cvar(
        &individual_mean_task_scores,
        protocol.lower_tail_fraction_basis_points,
    )?;

    let mut slip_for_median = slips.clone();
    let median_slip_distance_m =
        median_f64(&mut slip_for_median).ok_or(RobustnessError::NonFiniteSummary)?;
    let mut slip_for_p95 = slips.clone();
    let p95_slip_distance_m = percentile_nearest_rank_f64(&mut slip_for_p95, 95, 100)
        .ok_or(RobustnessError::NonFiniteSummary)?;
    let mut recovery_for_median = recovered_latency.clone();
    let median_recovery_latency_ticks = if recovery_for_median.is_empty() {
        None
    } else {
        recovery_for_median.sort_unstable();
        let middle = recovery_for_median.len() / 2;
        Some(if recovery_for_median.len() % 2 == 0 {
            recovery_for_median[middle - 1] as f64 * 0.5 + recovery_for_median[middle] as f64 * 0.5
        } else {
            recovery_for_median[middle] as f64
        })
    };
    let mut recovery_for_p95 = recovered_latency.clone();
    let p95_recovery_latency_ticks = percentile_nearest_rank_u64(&mut recovery_for_p95, 95, 100);
    let mut mechanism_recovery_for_median = mechanism_restored_latency.clone();
    let median_mechanism_recovery_latency_ticks = if mechanism_recovery_for_median.is_empty() {
        None
    } else {
        mechanism_recovery_for_median.sort_unstable();
        let middle = mechanism_recovery_for_median.len() / 2;
        Some(if mechanism_recovery_for_median.len() % 2 == 0 {
            mechanism_recovery_for_median[middle - 1] as f64 * 0.5
                + mechanism_recovery_for_median[middle] as f64 * 0.5
        } else {
            mechanism_recovery_for_median[middle] as f64
        })
    };
    let mut mechanism_recovery_for_p95 = mechanism_restored_latency.clone();
    let p95_mechanism_recovery_latency_ticks =
        percentile_nearest_rank_u64(&mut mechanism_recovery_for_p95, 95, 100);

    Ok(ScenarioScorecard {
        model_id: first.model_id.clone(),
        model_family: first.model_family,
        model_evidence_tier: first.model_evidence_tier,
        evaluation_tier: first.evaluation_tier,
        scenario_id: first.scenario_id.clone(),
        scenario_family: first.scenario_family,
        perturbation_severity_microunits: first.perturbation_severity_microunits,
        perturbation_unit: first.perturbation_unit.clone(),
        episode_count: records.len(),
        individual_count: individuals.len(),
        success_rate,
        mean_task_score: stable_mean(&task_scores)?,
        lower_tail_cvar_task_score: lower_tail_cvar(
            &task_scores,
            protocol.lower_tail_fraction_basis_points,
        )?,
        individual_balanced_success_rate,
        individual_balanced_mean_task_score,
        individual_balanced_lower_tail_cvar_task_score,
        worst_individual_success_rate: worst_success_individual.success_rate,
        worst_success_individual_id: worst_success_individual.individual_id.clone(),
        worst_individual_lower_tail_cvar_task_score: worst_cvar_individual
            .lower_tail_cvar_task_score,
        worst_cvar_individual_id: worst_cvar_individual.individual_id.clone(),
        individual_scorecards,
        mean_progress_distance_m: stable_mean(&progress)?,
        mean_body_pose_rmse_m: stable_mean(&pose_errors)?,
        mean_joint_angle_rmse_rad: stable_mean(&joint_errors)?,
        median_slip_distance_m,
        p95_slip_distance_m,
        mean_energy_j: stable_mean(&energies)?,
        mean_actuator_saturation_fraction: stable_mean(&saturation)?,
        recovery_applicable_episodes: recovery_applicable,
        recovered_episodes: recovered_latency.len(),
        not_recovered_by_horizon_episodes: not_recovered_by_horizon,
        not_recovered_before_termination_episodes: not_recovered_before_termination,
        unobserved_recovery_episodes: recovery_unobserved,
        median_recovery_latency_ticks,
        p95_recovery_latency_ticks,
        mechanism_recovery_applicable_episodes: mechanism_recovery_applicable,
        mechanism_restored_episodes: mechanism_restored_latency.len(),
        mechanism_not_restored_by_horizon_episodes: mechanism_not_restored_by_horizon,
        mechanism_not_restored_before_termination_episodes:
            mechanism_not_restored_before_termination,
        unobserved_mechanism_recovery_episodes: mechanism_recovery_unobserved,
        median_mechanism_recovery_latency_ticks,
        p95_mechanism_recovery_latency_ticks,
        compensation_without_mechanism_restoration_episodes:
            compensation_without_mechanism_restoration,
        joint_function_and_mechanism_recovery_episodes: joint_function_and_mechanism_recovery,
        episodes_with_falls,
        total_fall_count,
        episodes_with_numerical_failures,
        total_numerical_failure_count,
        episodes_with_physics_violations,
        total_physics_violation_count,
        episodes_with_unclassified_failures,
        total_unclassified_failure_count,
        hard_failure_gate_passed: total_numerical_failure_count == 0
            && total_physics_violation_count == 0
            && total_unclassified_failure_count == 0,
    })
}

fn model_worst_cases(scores: &[ScenarioScorecard]) -> Result<Vec<ModelWorstCase>, RobustnessError> {
    let mut by_model: BTreeMap<&str, Vec<&ScenarioScorecard>> = BTreeMap::new();
    for score in scores {
        by_model.entry(&score.model_id).or_default().push(score);
    }

    let mut worst_cases = Vec::with_capacity(by_model.len());
    for (model_id, rows) in by_model {
        let worst_success = rows
            .iter()
            .min_by(|left, right| left.success_rate.total_cmp(&right.success_rate))
            .ok_or(RobustnessError::NonFiniteSummary)?;
        let worst_cvar = rows
            .iter()
            .min_by(|left, right| {
                left.lower_tail_cvar_task_score
                    .total_cmp(&right.lower_tail_cvar_task_score)
            })
            .ok_or(RobustnessError::NonFiniteSummary)?;

        worst_cases.push(ModelWorstCase {
            model_id: model_id.to_string(),
            scenario_count: rows.len(),
            worst_success_rate: worst_success.success_rate,
            worst_success_scenario_id: worst_success.scenario_id.clone(),
            worst_lower_tail_cvar_task_score: worst_cvar.lower_tail_cvar_task_score,
            worst_cvar_scenario_id: worst_cvar.scenario_id.clone(),
            total_numerical_failure_episodes: rows
                .iter()
                .map(|row| row.episodes_with_numerical_failures as u64)
                .sum(),
            total_physics_violation_episodes: rows
                .iter()
                .map(|row| row.episodes_with_physics_violations as u64)
                .sum(),
        });
    }
    Ok(worst_cases)
}

fn matched_key(record: &EpisodeRecord) -> MatchedTrialKey {
    MatchedTrialKey {
        scenario_id: record.scenario_id.clone(),
        scenario_family: record.scenario_family,
        perturbation_severity_microunits: record.perturbation_severity_microunits,
        perturbation_unit: record.perturbation_unit.clone(),
        individual_id: record.individual_id.clone(),
        trial_id: record.trial_id.clone(),
        seed: record.seed,
        evaluation_tier: record.evaluation_tier,
        initial_state_sha256: record.initial_state_sha256.clone(),
        body_model_sha256: record.body_model_sha256.clone(),
        morphology_sha256: record.morphology_sha256.clone(),
        physics_sha256: record.physics_sha256.clone(),
        environment_sha256: record.environment_sha256.clone(),
        sensor_schema_sha256: record.sensor_schema_sha256.clone(),
        solver_sha256: record.solver_sha256.clone(),
        hardware_sha256: record.hardware_sha256.clone(),
        mechanism_state_schema_sha256: record.mechanism_state_schema_sha256.clone(),
        scenario_sha256: record.scenario_sha256.clone(),
        horizon_ticks: record.horizon_ticks,
        control_compute_budget_micros: record.control_compute_budget_micros,
        training_budget_steps: record.training_budget_steps,
        perturbation_onset_tick: record.perturbation_onset_tick,
    }
}

/// Validate all receipts and compute separate per-scenario, per-model scorecards.
///
/// This function deliberately does not average across scenario families. Easy
/// nominal trials cannot dilute a hard sensory-loss or disturbance stratum.
pub fn analyze_robustness(
    protocol: &RobustnessProtocol,
    episodes: &[EpisodeRecord],
) -> Result<RobustnessReport, RobustnessError> {
    protocol.validate()?;
    if episodes.is_empty() {
        return Err(RobustnessError::EmptyEpisodes);
    }
    if episodes.len() > MAX_EPISODES {
        return Err(RobustnessError::ResourceLimitExceeded);
    }

    let mut model_ids = BTreeSet::new();
    let mut scenario_ids = BTreeSet::new();
    let mut seen_trials = BTreeSet::new();
    let mut model_signatures: BTreeMap<&str, ModelSignature> = BTreeMap::new();
    let mut scenario_signatures: BTreeMap<&str, ScenarioSignature> = BTreeMap::new();
    let mut groups: BTreeMap<ModelScenarioKey, Vec<&EpisodeRecord>> = BTreeMap::new();

    for episode in episodes {
        validate_episode(protocol, episode)?;
        model_ids.insert(episode.model_id.as_str());
        scenario_ids.insert(episode.scenario_id.as_str());

        let model_signature = ModelSignature {
            family: episode.model_family,
            evidence_tier: episode.model_evidence_tier,
            code_sha256: episode.code_sha256.clone(),
            parameter_sha256: episode.parameter_sha256.clone(),
            controller_config_sha256: episode.controller_config_sha256.clone(),
            connectome_sha256: episode.connectome_sha256.clone(),
            biological_source_sha256: episode.biological_source_sha256.clone(),
            sensor_mapping_sha256: episode.sensor_mapping_sha256.clone(),
            effector_mapping_sha256: episode.effector_mapping_sha256.clone(),
            muscle_mapping_sha256: episode.muscle_mapping_sha256.clone(),
        };
        if let Some(previous) = model_signatures.get(episode.model_id.as_str()) {
            if previous != &model_signature {
                return Err(RobustnessError::ModelDefinitionMismatch);
            }
        } else {
            model_signatures.insert(episode.model_id.as_str(), model_signature);
        }

        let scenario_signature = ScenarioSignature {
            family: episode.scenario_family,
            severity_microunits: episode.perturbation_severity_microunits,
            unit: episode.perturbation_unit.clone(),
            scenario_sha256: episode.scenario_sha256.clone(),
        };
        if let Some(previous) = scenario_signatures.get(episode.scenario_id.as_str()) {
            if previous != &scenario_signature {
                return Err(RobustnessError::ScenarioDefinitionMismatch);
            }
        } else {
            scenario_signatures.insert(episode.scenario_id.as_str(), scenario_signature);
        }

        if !seen_trials.insert((episode.model_id.as_str(), episode.trial_id.as_str())) {
            return Err(RobustnessError::DuplicateTrial);
        }

        groups
            .entry(ModelScenarioKey {
                model_id: episode.model_id.clone(),
                scenario_id: episode.scenario_id.clone(),
            })
            .or_default()
            .push(episode);
    }

    if model_ids.len() > MAX_MODELS || scenario_ids.len() > MAX_SCENARIOS {
        return Err(RobustnessError::ResourceLimitExceeded);
    }

    let mut by_scenario = Vec::with_capacity(groups.len());
    for records in groups.values() {
        by_scenario.push(score_scenario(protocol, records)?);
    }

    let model_worst_case = model_worst_cases(&by_scenario)?;
    Ok(RobustnessReport {
        protocol_sha256: protocol.protocol_sha256.clone(),
        split_sha256: protocol.split_sha256.clone(),
        scoring_plan_sha256: protocol.scoring_plan_sha256.clone(),
        by_scenario,
        model_worst_case,
    })
}

/// Compare two model IDs only on strictly matched scenario/individual/seed/state
/// episodes. A missing pair is a hard error; silently intersecting unmatched
/// runs would introduce selection bias.
pub fn compare_paired_models(
    protocol: &RobustnessProtocol,
    episodes: &[EpisodeRecord],
    candidate_model_id: &str,
    baseline_model_id: &str,
) -> Result<PairedModelComparison, RobustnessError> {
    protocol.validate()?;
    if !valid_identifier(candidate_model_id) {
        return Err(RobustnessError::InvalidIdentifier("candidate_model_id"));
    }
    if !valid_identifier(baseline_model_id) {
        return Err(RobustnessError::InvalidIdentifier("baseline_model_id"));
    }
    if candidate_model_id == baseline_model_id {
        return Err(RobustnessError::ModelDefinitionMismatch);
    }
    if episodes.is_empty() {
        return Err(RobustnessError::EmptyEpisodes);
    }

    let mut candidate = BTreeMap::new();
    let mut baseline = BTreeMap::new();
    let mut candidate_seen = false;
    let mut baseline_seen = false;
    let mut seen_model_trials = BTreeSet::new();
    let mut model_signatures: BTreeMap<String, ModelSignature> = BTreeMap::new();

    for episode in episodes {
        validate_episode(protocol, episode)?;
        if !seen_model_trials.insert((episode.model_id.clone(), episode.trial_id.clone())) {
            return Err(RobustnessError::DuplicateTrial);
        }
        let signature = ModelSignature {
            family: episode.model_family,
            evidence_tier: episode.model_evidence_tier,
            code_sha256: episode.code_sha256.clone(),
            parameter_sha256: episode.parameter_sha256.clone(),
            controller_config_sha256: episode.controller_config_sha256.clone(),
            connectome_sha256: episode.connectome_sha256.clone(),
            biological_source_sha256: episode.biological_source_sha256.clone(),
            sensor_mapping_sha256: episode.sensor_mapping_sha256.clone(),
            effector_mapping_sha256: episode.effector_mapping_sha256.clone(),
            muscle_mapping_sha256: episode.muscle_mapping_sha256.clone(),
        };
        if model_signatures
            .insert(episode.model_id.clone(), signature.clone())
            .is_some_and(|previous| previous != signature)
        {
            return Err(RobustnessError::ModelDefinitionMismatch);
        }
        let key = matched_key(episode);
        if episode.model_id == candidate_model_id {
            candidate_seen = true;
            if candidate.insert(key, episode).is_some() {
                return Err(RobustnessError::DuplicateTrial);
            }
        } else if episode.model_id == baseline_model_id {
            baseline_seen = true;
            if baseline.insert(key, episode).is_some() {
                return Err(RobustnessError::DuplicateTrial);
            }
        }
    }

    if !candidate_seen {
        return Err(RobustnessError::ModelNotFound(
            candidate_model_id.to_string(),
        ));
    }
    if !baseline_seen {
        return Err(RobustnessError::ModelNotFound(
            baseline_model_id.to_string(),
        ));
    }
    if candidate.len() != baseline.len() || candidate.keys().ne(baseline.keys()) {
        return Err(RobustnessError::PairSetMismatch);
    }
    if candidate.len() < protocol.minimum_paired_episodes {
        return Err(RobustnessError::TooFewPairedEpisodes {
            observed: candidate.len(),
            minimum: protocol.minimum_paired_episodes,
        });
    }

    let mut candidate_scores = Vec::with_capacity(candidate.len());
    let mut baseline_scores = Vec::with_capacity(baseline.len());
    let mut deltas = Vec::with_capacity(candidate.len());
    let mut candidate_wins = 0usize;
    let mut ties = 0usize;
    let mut candidate_losses = 0usize;
    let mut candidate_only_successes = 0usize;
    let mut baseline_only_successes = 0usize;
    let mut candidate_numerical_failure_episodes = 0usize;
    let mut baseline_numerical_failure_episodes = 0usize;
    let mut candidate_physics_violation_episodes = 0usize;
    let mut baseline_physics_violation_episodes = 0usize;
    let mut candidate_functional_recovery_episodes = 0usize;
    let mut baseline_functional_recovery_episodes = 0usize;
    let mut candidate_mechanism_restoration_episodes = 0usize;
    let mut baseline_mechanism_restoration_episodes = 0usize;
    let mut candidate_compensation_without_mechanism_restoration_episodes = 0usize;
    let mut baseline_compensation_without_mechanism_restoration_episodes = 0usize;

    for (key, candidate_episode) in &candidate {
        let baseline_episode = baseline.get(key).ok_or(RobustnessError::PairSetMismatch)?;
        candidate_scores.push(candidate_episode.task_score);
        baseline_scores.push(baseline_episode.task_score);
        let delta = candidate_episode.task_score - baseline_episode.task_score;
        deltas.push(delta);

        if delta > protocol.paired_score_tie_tolerance {
            candidate_wins += 1;
        } else if delta < -protocol.paired_score_tie_tolerance {
            candidate_losses += 1;
        } else {
            ties += 1;
        }

        if candidate_episode.success && !baseline_episode.success {
            candidate_only_successes += 1;
        }
        if baseline_episode.success && !candidate_episode.success {
            baseline_only_successes += 1;
        }
        if candidate_episode.numerical_failure_count > 0 {
            candidate_numerical_failure_episodes += 1;
        }
        if baseline_episode.numerical_failure_count > 0 {
            baseline_numerical_failure_episodes += 1;
        }
        if candidate_episode.physics_violation_count > 0 {
            candidate_physics_violation_episodes += 1;
        }
        if baseline_episode.physics_violation_count > 0 {
            baseline_physics_violation_episodes += 1;
        }
        if matches!(candidate_episode.recovery, RecoveryStatus::Recovered { .. }) {
            candidate_functional_recovery_episodes += 1;
        }
        if matches!(baseline_episode.recovery, RecoveryStatus::Recovered { .. }) {
            baseline_functional_recovery_episodes += 1;
        }
        if matches!(
            candidate_episode.mechanism_recovery,
            MechanismRecoveryStatus::Restored { .. }
        ) {
            candidate_mechanism_restoration_episodes += 1;
        }
        if matches!(
            baseline_episode.mechanism_recovery,
            MechanismRecoveryStatus::Restored { .. }
        ) {
            baseline_mechanism_restoration_episodes += 1;
        }
        if matches!(candidate_episode.recovery, RecoveryStatus::Recovered { .. })
            && matches!(
                candidate_episode.mechanism_recovery,
                MechanismRecoveryStatus::NotRestoredByHorizon
                    | MechanismRecoveryStatus::NotRestoredBeforeTermination
            )
        {
            candidate_compensation_without_mechanism_restoration_episodes += 1;
        }
        if matches!(baseline_episode.recovery, RecoveryStatus::Recovered { .. })
            && matches!(
                baseline_episode.mechanism_recovery,
                MechanismRecoveryStatus::NotRestoredByHorizon
                    | MechanismRecoveryStatus::NotRestoredBeforeTermination
            )
        {
            baseline_compensation_without_mechanism_restoration_episodes += 1;
        }
    }

    let mut deltas_for_median = deltas.clone();
    let median_paired_task_score_delta =
        median_f64(&mut deltas_for_median).ok_or(RobustnessError::NonFiniteSummary)?;

    Ok(PairedModelComparison {
        candidate_model_id: candidate_model_id.to_string(),
        baseline_model_id: baseline_model_id.to_string(),
        matched_episode_count: candidate.len(),
        candidate_mean_task_score: stable_mean(&candidate_scores)?,
        baseline_mean_task_score: stable_mean(&baseline_scores)?,
        mean_paired_task_score_delta: stable_mean(&deltas)?,
        median_paired_task_score_delta,
        candidate_wins,
        ties,
        candidate_losses,
        candidate_only_successes,
        baseline_only_successes,
        candidate_numerical_failure_episodes,
        baseline_numerical_failure_episodes,
        candidate_physics_violation_episodes,
        baseline_physics_violation_episodes,
        candidate_functional_recovery_episodes,
        baseline_functional_recovery_episodes,
        candidate_mechanism_restoration_episodes,
        baseline_mechanism_restoration_episodes,
        candidate_compensation_without_mechanism_restoration_episodes,
        baseline_compensation_without_mechanism_restoration_episodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn protocol() -> RobustnessProtocol {
        RobustnessProtocol {
            benchmark_id: "loco-robust-v1".to_string(),
            protocol_sha256: digest('a'),
            split_sha256: digest('b'),
            scenario_catalog_sha256: digest('c'),
            scoring_plan_sha256: digest('d'),
            mechanism_state_schema_sha256: Some(digest('f')),
            lower_tail_fraction_basis_points: 1000,
            minimum_episodes_per_scenario: 2,
            minimum_episodes_per_individual: 1,
            minimum_individuals_per_scenario: 2,
            minimum_paired_episodes: 2,
            paired_score_tie_tolerance: 1.0e-9,
            functional_recovery_tolerance: 0.1,
            mechanism_recovery_tolerance: 0.1,
        }
    }

    fn episode(model_id: &str, trial: &str, individual: &str, score: f64) -> EpisodeRecord {
        EpisodeRecord {
            model_id: model_id.to_string(),
            model_family: ModelFamily::ConnectomeHybrid,
            model_evidence_tier: ModelEvidenceTier::ConnectomeConstrained,
            evaluation_tier: EvaluationTier::InSilicoEmbodied,
            scenario_id: "front-feCO-dropout-v1".to_string(),
            scenario_family: ScenarioFamily::SensorDropout,
            perturbation_severity_microunits: 500_000,
            perturbation_unit: "ppm".to_string(),
            individual_id: individual.to_string(),
            trial_id: trial.to_string(),
            seed: trial.bytes().map(u64::from).sum(),
            protocol_sha256: digest('a'),
            split_sha256: digest('b'),
            scoring_plan_sha256: digest('d'),
            scenario_sha256: digest('e'),
            initial_state_sha256: digest('f'),
            body_model_sha256: digest('1'),
            morphology_sha256: digest('2'),
            physics_sha256: digest('3'),
            environment_sha256: digest('4'),
            sensor_schema_sha256: digest('5'),
            solver_sha256: digest('6'),
            hardware_sha256: digest('7'),
            code_sha256: digest('8'),
            parameter_sha256: digest('9'),
            controller_config_sha256: if model_id == "candidate" {
                digest('a')
            } else {
                digest('b')
            },
            connectome_sha256: Some(digest('f')),
            biological_source_sha256: Some(digest('1')),
            sensor_mapping_sha256: Some(digest('2')),
            effector_mapping_sha256: Some(digest('3')),
            muscle_mapping_sha256: None,
            raw_trajectory_sha256: digest('c'),
            metric_payload_sha256: digest('d'),
            run_receipt_sha256: digest('e'),
            measured_dataset_sha256: None,
            independent_replication_sha256: None,
            horizon_ticks: 1000,
            completed_ticks: 1000,
            control_compute_budget_micros: 50,
            training_budget_steps: 10_000,
            perturbation_onset_tick: Some(100),
            recovery: RecoveryStatus::Recovered { latency_ticks: 120 },
            mechanism_recovery: MechanismRecoveryStatus::Restored { latency_ticks: 150 },
            functional_state_deviation_from_nominal: Some(0.05),
            mechanism_state_schema_sha256: Some(digest('f')),
            mechanism_state_deviation_from_nominal: Some(0.04),
            task_score: score,
            success: true,
            termination: TerminationReason::HorizonCompleted,
            progress_distance_m: 0.01,
            body_pose_rmse_m: 0.001,
            joint_angle_rmse_rad: 0.01,
            slip_distance_m: 0.0001,
            energy_j: 0.02,
            actuator_saturation_fraction: 0.1,
            fall_count: 0,
            numerical_failure_count: 0,
            physics_violation_count: 0,
            unclassified_failure_count: 0,
        }
    }

    #[test]
    fn lower_tail_cvar_keeps_rare_catastrophic_episodes_visible() {
        let mut episodes = Vec::new();
        for index in 0..9 {
            episodes.push(episode(
                "candidate",
                &format!("trial-{index}"),
                &format!("fly-{index}"),
                0.95,
            ));
        }
        episodes.push(episode("candidate", "trial-9", "fly-9", 0.05));
        let mut protocol = protocol();
        protocol.minimum_episodes_per_scenario = 10;
        protocol.minimum_individuals_per_scenario = 10;
        let report = analyze_robustness(&protocol, &episodes).expect("scorecard");
        let row = &report.by_scenario[0];
        assert!((row.mean_task_score - 0.86).abs() < 1.0e-12);
        assert!((row.lower_tail_cvar_task_score - 0.05).abs() < 1.0e-12);
    }

    #[test]
    fn individual_balancing_prevents_over_sampled_individual_dominance() {
        let mut episodes = Vec::new();
        for index in 0..9 {
            episodes.push(episode(
                "candidate",
                &format!("fly-a-trial-{index}"),
                "fly-a",
                0.9,
            ));
        }
        let mut under_sampled = episode("candidate", "fly-b-trial-1", "fly-b", 0.1);
        under_sampled.success = false;
        episodes.push(under_sampled);

        let mut protocol = protocol();
        protocol.minimum_episodes_per_scenario = 10;
        let report = analyze_robustness(&protocol, &episodes).expect("scorecard");
        let row = &report.by_scenario[0];

        assert!((row.mean_task_score - 0.82).abs() < 1.0e-12);
        assert!((row.individual_balanced_mean_task_score - 0.5).abs() < 1.0e-12);
        assert!((row.individual_balanced_lower_tail_cvar_task_score - 0.1).abs() < 1.0e-12);
        assert_eq!(row.individual_scorecards.len(), 2);
        assert_eq!(row.worst_success_individual_id, "fly-b");
        assert_eq!(row.worst_cvar_individual_id, "fly-b");
        assert_eq!(row.individual_scorecards[0].morphology_sha256, digest('2'));
    }

    #[test]
    fn recovery_that_does_not_occur_is_right_censored_not_imputed() {
        let mut episodes = vec![
            episode("candidate", "trial-1", "fly-1", 0.8),
            episode("candidate", "trial-2", "fly-2", 0.7),
        ];
        episodes[0].recovery = RecoveryStatus::Recovered { latency_ticks: 100 };
        episodes[1].recovery = RecoveryStatus::NotRecoveredByHorizon;
        episodes[1].functional_state_deviation_from_nominal = Some(0.4);
        let report = analyze_robustness(&protocol(), &episodes).expect("scorecard");
        let row = &report.by_scenario[0];
        assert_eq!(row.recovered_episodes, 1);
        assert_eq!(row.not_recovered_by_horizon_episodes, 1);
        assert_eq!(row.median_recovery_latency_ticks, Some(100.0));
        assert_eq!(row.p95_recovery_latency_ticks, Some(100));
    }

    #[test]
    fn recovery_latency_is_not_fabricated_for_unobserved_channels() {
        let mut episodes = vec![
            episode("candidate", "trial-1", "fly-1", 0.8),
            episode("candidate", "trial-2", "fly-2", 0.7),
        ];
        episodes[0].recovery = RecoveryStatus::Unobserved;
        episodes[1].recovery = RecoveryStatus::Unobserved;
        episodes[0].functional_state_deviation_from_nominal = None;
        episodes[1].functional_state_deviation_from_nominal = None;
        episodes[0].mechanism_recovery = MechanismRecoveryStatus::Unobserved;
        episodes[1].mechanism_recovery = MechanismRecoveryStatus::Unobserved;
        episodes[0].mechanism_state_schema_sha256 = None;
        episodes[1].mechanism_state_schema_sha256 = None;
        episodes[0].mechanism_state_deviation_from_nominal = None;
        episodes[1].mechanism_state_deviation_from_nominal = None;
        let mut unobserved_protocol = protocol();
        unobserved_protocol.mechanism_state_schema_sha256 = None;
        let report = analyze_robustness(&unobserved_protocol, &episodes).expect("scorecard");
        assert_eq!(report.by_scenario[0].median_recovery_latency_ticks, None);
        assert_eq!(report.by_scenario[0].unobserved_recovery_episodes, 2);
        assert_eq!(
            report.by_scenario[0].unobserved_mechanism_recovery_episodes,
            2
        );
    }

    #[test]
    fn worst_success_and_worst_cvar_strata_remain_separate() {
        let mut flat = vec![
            episode("candidate", "flat-1", "fly-1", 0.9),
            episode("candidate", "flat-2", "fly-2", 0.9),
        ];
        for record in &mut flat {
            record.scenario_id = "flat-v1".to_string();
            record.scenario_family = ScenarioFamily::FlatGround;
            record.scenario_sha256 = digest('a');
            record.perturbation_severity_microunits = 0;
            record.perturbation_unit = "none".to_string();
            record.perturbation_onset_tick = None;
            record.recovery = RecoveryStatus::NotApplicable;
            record.mechanism_recovery = MechanismRecoveryStatus::NotApplicable;
            record.functional_state_deviation_from_nominal = None;
            record.mechanism_state_schema_sha256 = None;
            record.mechanism_state_deviation_from_nominal = None;
        }
        flat[0].success = false;
        let mut stress = vec![
            episode("candidate", "stress-1", "fly-1", 0.2),
            episode("candidate", "stress-2", "fly-2", 0.2),
        ];
        for record in &mut stress {
            record.success = true;
            record.termination = TerminationReason::HorizonCompleted;
            record.task_score = 0.1;
        }
        let mut combined = flat;
        combined.extend(stress);
        let report = analyze_robustness(&protocol(), &combined).expect("scorecard");
        let worst = &report.model_worst_case[0];
        assert_eq!(worst.worst_success_scenario_id, "flat-v1");
        assert_eq!(worst.worst_cvar_scenario_id, "front-feCO-dropout-v1");
        assert_eq!(worst.scenario_count, 2);
    }

    #[test]
    fn paired_comparison_requires_identical_initial_state_and_physics() {
        let mut episodes = vec![
            episode("candidate", "trial-1", "fly-1", 0.9),
            episode("baseline", "trial-1", "fly-1", 0.6),
            episode("candidate", "trial-2", "fly-2", 0.8),
            episode("baseline", "trial-2", "fly-2", 0.7),
        ];
        let result = compare_paired_models(&protocol(), &episodes, "candidate", "baseline")
            .expect("paired comparison");
        assert_eq!(result.matched_episode_count, 2);
        assert_eq!(result.candidate_wins, 2);
        assert!((result.mean_paired_task_score_delta - 0.2).abs() < 1.0e-12);

        episodes[3].physics_sha256 = digest('8');
        assert_eq!(
            compare_paired_models(&protocol(), &episodes, "candidate", "baseline"),
            Err(RobustnessError::PairSetMismatch)
        );
    }

    #[test]
    fn paired_comparison_separates_task_score_from_mechanism_restoration() {
        let mut episodes = vec![
            episode("candidate", "trial-1", "fly-1", 0.95),
            episode("baseline", "trial-1", "fly-1", 0.85),
            episode("candidate", "trial-2", "fly-2", 0.95),
            episode("baseline", "trial-2", "fly-2", 0.85),
        ];
        for candidate_index in [0, 2] {
            episodes[candidate_index].mechanism_recovery =
                MechanismRecoveryStatus::NotRestoredByHorizon;
            episodes[candidate_index].mechanism_state_deviation_from_nominal = Some(0.3);
        }

        let result = compare_paired_models(&protocol(), &episodes, "candidate", "baseline")
            .expect("paired comparison");
        assert_eq!(result.candidate_wins, 2);
        assert_eq!(result.candidate_functional_recovery_episodes, 2);
        assert_eq!(result.baseline_functional_recovery_episodes, 2);
        assert_eq!(result.candidate_mechanism_restoration_episodes, 0);
        assert_eq!(result.baseline_mechanism_restoration_episodes, 2);
        assert_eq!(
            result.candidate_compensation_without_mechanism_restoration_episodes,
            2
        );
        assert_eq!(
            result.baseline_compensation_without_mechanism_restoration_episodes,
            0
        );
    }

    #[test]
    fn paired_comparison_rejects_missing_pairs_instead_of_intersecting() {
        let episodes = vec![
            episode("candidate", "trial-1", "fly-1", 0.9),
            episode("baseline", "trial-1", "fly-1", 0.6),
            episode("candidate", "trial-2", "fly-2", 0.8),
        ];
        assert_eq!(
            compare_paired_models(&protocol(), &episodes, "candidate", "baseline"),
            Err(RobustnessError::PairSetMismatch)
        );
    }

    #[test]
    fn hard_numerical_and_physics_failures_are_reported_separately() {
        let mut numerical = episode("candidate", "trial-1", "fly-1", 0.0);
        numerical.success = false;
        numerical.termination = TerminationReason::NumericalFailure;
        numerical.numerical_failure_count = 1;
        let mut physics = episode("candidate", "trial-2", "fly-2", 0.0);
        physics.success = false;
        physics.termination = TerminationReason::PhysicsViolation;
        physics.physics_violation_count = 3;
        let report = analyze_robustness(&protocol(), &[numerical, physics]).expect("report");
        let row = &report.by_scenario[0];
        assert_eq!(row.episodes_with_numerical_failures, 1);
        assert_eq!(row.total_physics_violation_count, 3);
        assert_eq!(row.success_rate, 0.0);
        assert!(!row.hard_failure_gate_passed);
    }

    #[test]
    fn functional_recovery_without_mechanism_restoration_is_reported_as_compensation() {
        let mut episodes = vec![
            episode("candidate", "trial-1", "fly-1", 0.9),
            episode("candidate", "trial-2", "fly-2", 0.8),
        ];
        episodes[0].mechanism_recovery = MechanismRecoveryStatus::NotRestoredByHorizon;
        episodes[0].mechanism_state_deviation_from_nominal = Some(0.3);

        let report = analyze_robustness(&protocol(), &episodes).expect("scorecard");
        let row = &report.by_scenario[0];
        assert_eq!(row.recovered_episodes, 2);
        assert_eq!(row.mechanism_restored_episodes, 1);
        assert_eq!(row.mechanism_not_restored_by_horizon_episodes, 1);
        assert_eq!(row.compensation_without_mechanism_restoration_episodes, 1);
        assert_eq!(row.joint_function_and_mechanism_recovery_episodes, 1);
        assert_eq!(
            row.individual_scorecards[0].compensation_without_mechanism_restoration_episodes,
            1
        );
        assert_eq!(
            row.individual_scorecards[1].mechanism_restoration_episodes,
            1
        );
    }

    #[test]
    fn rejects_mechanism_state_schema_drift_from_the_preregistered_protocol() {
        let episodes = vec![
            episode("candidate", "trial-1", "fly-1", 0.9),
            episode("candidate", "trial-2", "fly-2", 0.8),
        ];
        let mut protocol = protocol();
        protocol.mechanism_state_schema_sha256 = Some(digest('9'));
        assert_eq!(
            analyze_robustness(&protocol, &episodes),
            Err(RobustnessError::ProtocolMismatch)
        );
    }

    #[test]
    fn rejects_mechanism_restoration_without_a_within_tolerance_state_measure() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.mechanism_state_deviation_from_nominal = Some(0.4);
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::InvalidRecoveryRecord)
        );
    }

    #[test]
    fn rejects_functional_non_recovery_within_declared_recovery_tolerance() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.recovery = RecoveryStatus::NotRecoveredByHorizon;
        invalid.functional_state_deviation_from_nominal = Some(0.05);
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::InvalidRecoveryRecord)
        );
    }

    #[test]
    fn rejects_mixed_physics_within_one_scenario_scorecard() {
        let first = episode("candidate", "trial-1", "fly-1", 0.9);
        let mut second = episode("candidate", "trial-2", "fly-2", 0.8);
        second.physics_sha256 = digest('8');
        assert_eq!(
            analyze_robustness(&protocol(), &[first, second]),
            Err(RobustnessError::ScenarioDefinitionMismatch)
        );
    }

    #[test]
    fn rejects_successful_episode_with_numerical_failure_termination() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.termination = TerminationReason::NumericalFailure;
        invalid.numerical_failure_count = 1;
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::InvalidOutcomeTermination)
        );
    }

    #[test]
    fn rejects_duplicate_model_trial_identity() {
        let first = episode("candidate", "trial-1", "fly-1", 0.9);
        let second = episode("candidate", "trial-1", "fly-2", 0.8);
        assert_eq!(
            analyze_robustness(&protocol(), &[first, second]),
            Err(RobustnessError::DuplicateTrial)
        );
    }

    #[test]
    fn rejects_connectome_model_claim_without_connectome_evidence() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.model_evidence_tier = ModelEvidenceTier::Phenomenological;
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::EvidenceTierMismatch)
        );
    }

    #[test]
    fn connectome_model_requires_all_mapping_artifacts() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.sensor_mapping_sha256 = None;
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::MissingEvidenceArtifact(
                "connectome_or_mapping"
            ))
        );
    }

    #[test]
    fn muscle_actuated_model_requires_a_muscle_mapping_artifact() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.model_family = ModelFamily::MuscleActuatedController;
        invalid.model_evidence_tier = ModelEvidenceTier::Phenomenological;
        invalid.muscle_mapping_sha256 = None;
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::MissingEvidenceArtifact("muscle_mapping"))
        );
    }

    #[test]
    fn perturbation_grounded_non_connectome_model_requires_biological_source_and_data() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.model_family = ModelFamily::LearnedController;
        invalid.model_evidence_tier = ModelEvidenceTier::PerturbationGrounded;
        invalid.connectome_sha256 = None;
        invalid.sensor_mapping_sha256 = None;
        invalid.effector_mapping_sha256 = None;
        invalid.measured_dataset_sha256 = Some(digest('9'));
        invalid.evaluation_tier = EvaluationTier::MeasuredIntervention;
        invalid.biological_source_sha256 = None;
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::MissingEvidenceArtifact(
                "biological_source"
            ))
        );
    }

    #[test]
    fn rejects_model_provenance_changes_between_episodes() {
        let first = episode("candidate", "trial-1", "fly-1", 0.9);
        let mut second = episode("candidate", "trial-2", "fly-2", 0.8);
        second.connectome_sha256 = Some(digest('8'));
        assert_eq!(
            analyze_robustness(&protocol(), &[first, second]),
            Err(RobustnessError::ModelDefinitionMismatch)
        );
    }

    #[test]
    fn rejects_recovery_latency_after_episode_completion() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.recovery = RecoveryStatus::Recovered {
            latency_ticks: 2_000,
        };
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::InvalidRecoveryRecord)
        );
    }

    #[test]
    fn rejects_protocol_digest_mismatch() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.scoring_plan_sha256 = digest('9');
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::ProtocolMismatch)
        );
    }

    #[test]
    fn rejects_missing_measured_data_for_measured_evaluation() {
        let mut invalid = episode("candidate", "trial-1", "fly-1", 0.9);
        invalid.evaluation_tier = EvaluationTier::MeasuredIntervention;
        assert_eq!(
            analyze_robustness(&protocol(), &[invalid]),
            Err(RobustnessError::MissingEvidenceArtifact("measured_dataset"))
        );
    }

    #[test]
    fn rejects_individual_below_declared_episode_replicates() {
        let episodes = vec![
            episode("candidate", "trial-1", "fly-1", 0.9),
            episode("candidate", "trial-2", "fly-2", 0.8),
        ];
        let mut protocol = protocol();
        protocol.minimum_episodes_per_individual = 2;
        assert_eq!(
            analyze_robustness(&protocol, &episodes),
            Err(RobustnessError::TooFewIndividualEpisodes {
                individual_id: "fly-1".to_string(),
                observed: 1,
                minimum: 2,
            })
        );
    }

    #[test]
    fn rejects_too_few_individuals_in_a_scenario() {
        let episodes = vec![
            episode("candidate", "trial-1", "fly-1", 0.9),
            episode("candidate", "trial-2", "fly-1", 0.8),
        ];
        assert_eq!(
            analyze_robustness(&protocol(), &episodes),
            Err(RobustnessError::TooFewIndividuals {
                observed: 1,
                minimum: 2,
            })
        );
    }
}
