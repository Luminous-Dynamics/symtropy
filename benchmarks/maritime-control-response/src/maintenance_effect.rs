#![forbid(unsafe_code)]

//! Known-truth blocked-randomized maintenance-effect benchmark.
//!
//! This is a P0 causal-estimation reference for randomized synthetic or
//! controlled test-bench experiments. It deliberately rejects observational
//! assignment: target-trial emulation, time-varying confounding, transport and
//! sensitivity bounds belong to later protocol versions.
//!
//! Estimation uses only observed outcomes and the preregistered blocked
//! allocation. The potential-outcome oracle is a separate scoring input and is
//! never passed to the estimator. A passing synthetic test is not marine-engine
//! causal evidence.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MAINTENANCE_EFFECT_SCHEMA: &str = "symtropy-marine-maintenance-effect-v0.1";
pub const MAINTENANCE_EFFECT_ORACLE_SCHEMA: &str = "symtropy-marine-maintenance-effect-oracle-v0.1";
const MAX_EXACT_RANDOMIZATION_UNITS: usize = 16;
const TIME_TOLERANCE_SECONDS: f64 = 1e-9;
const VARIANCE_TOLERANCE: f64 = 1e-12;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentMethod {
    BlockRandomized,
    Observational,
    DeterministicNonRandomized,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssignedArm {
    Treatment,
    Control,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeDirection {
    HigherIsBetter,
    LowerIsBetter,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementQuality {
    Valid,
    Missing,
    Stale,
    Rejected,
    Conflicted,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationStatus {
    Verified,
    Unknown,
    Expired,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StratumAllocation {
    pub stratum_id: String,
    pub planned_treatment_count: usize,
    pub planned_control_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RandomizedUnitObservation {
    /// The independent randomization unit (e.g. one engine/asset), not a row or window.
    pub unit_id: String,
    pub stratum_id: String,
    pub assigned_arm: AssignedArm,
    pub assignment_evidence_sha256: String,
    pub assignment_available_at_seconds: f64,
    pub baseline_value: f64,
    pub baseline_unit: String,
    pub baseline_observed_at_seconds: f64,
    pub baseline_available_at_seconds: f64,
    pub baseline_evidence_sha256: String,
    pub baseline_source_id: String,
    pub baseline_measurement_principle_id: String,
    pub baseline_dependency_domain_id: String,
    pub baseline_quality: MeasurementQuality,
    pub baseline_calibration_status: CalibrationStatus,
    pub baseline_calibration_record_sha256: String,
    pub baseline_calibration_valid_until_seconds: f64,
    pub baseline_uncertainty_abs: f64,
    pub observed_outcome_value: f64,
    pub outcome_unit: String,
    pub outcome_observed_at_seconds: f64,
    pub outcome_available_at_seconds: f64,
    pub outcome_evidence_sha256: String,
    pub outcome_source_id: String,
    pub outcome_measurement_principle_id: String,
    pub outcome_dependency_domain_id: String,
    pub outcome_quality: MeasurementQuality,
    pub outcome_calibration_status: CalibrationStatus,
    pub outcome_calibration_record_sha256: String,
    pub outcome_calibration_valid_until_seconds: f64,
    /// Deterministic absolute-error bound from the measurement/calibration contract,
    /// not a standard deviation and not folded into the sampling standard error.
    pub outcome_uncertainty_abs: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BlockedRandomizedMaintenanceTrial {
    pub schema_version: String,
    pub trial_id: String,
    pub assignment_method: AssignmentMethod,
    pub treatment_label: String,
    pub control_label: String,
    pub outcome_metric_id: String,
    pub outcome_unit: String,
    pub outcome_measurement_principle_id: String,
    pub outcome_direction: OutcomeDirection,
    pub assignment_at_seconds: f64,
    pub followup_horizon_seconds: f64,
    pub analysis_as_of_seconds: f64,
    pub confidence_level: f64,
    /// Commitment to the allocation/assignment plan frozen before outcomes.
    pub assignment_plan_sha256: String,
    pub assignment_audit_ref: String,
    pub stratum_allocations: Vec<StratumAllocation>,
    pub observations: Vec<RandomizedUnitObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UnitPotentialOutcomes {
    pub unit_id: String,
    pub potential_outcome_treatment: f64,
    pub potential_outcome_control: f64,
    pub oracle_record_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SyntheticPotentialOutcomeOracle {
    pub schema_version: String,
    pub trial_id: String,
    pub oracle_dataset_sha256: String,
    pub outcomes: Vec<UnitPotentialOutcomes>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct RandomizedMaintenanceEffectEstimate {
    pub schema_version: String,
    pub trial_id: String,
    pub treatment_label: String,
    pub control_label: String,
    pub outcome_metric_id: String,
    pub outcome_unit: String,
    pub estimand: String,
    pub estimator: String,
    /// Raw difference: treatment mean minus control mean, weighted by stratum size.
    pub ate_treatment_minus_control: f64,
    /// Reoriented so positive always means improvement under the declared outcome direction.
    pub benefit_oriented_effect: f64,
    pub standard_error: f64,
    pub confidence_level: f64,
    /// Normal-approximation interval; its small-sample limitations remain explicit.
    pub confidence_interval_treatment_minus_control: [f64; 2],
    pub confidence_interval_benefit_oriented: [f64; 2],
    /// Weighted worst-case measurement error bound for the treatment-control contrast.
    pub measurement_error_bound_abs: f64,
    /// Sampling interval expanded by the deterministic measurement-error bound.
    pub measurement_robust_interval_treatment_minus_control: [f64; 2],
    pub measurement_robust_interval_benefit_oriented: [f64; 2],
    /// Two-sided exact Fisher randomization p-value under the sharp null.
    pub exact_randomization_p_value: f64,
    pub independent_randomization_units: usize,
    pub stratum_count: usize,
    pub baseline_adjustment_used: bool,
    pub potential_outcome_oracle_used_for_estimation: bool,
    pub maintenance_authorized: bool,
    pub physical_restoration_authorized: bool,
    pub operating_restriction_clearance_authorized: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SyntheticOracleScore {
    pub trial_id: String,
    pub estimated_ate: f64,
    pub true_finite_population_ate: f64,
    pub estimation_error: f64,
    pub absolute_bias: f64,
    pub interval_contains_truth: bool,
    pub oracle_used_for_estimation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceEffectError {
    InvalidSchemaVersion,
    InvalidIdentity,
    InvalidDigest,
    NonRandomizedDesign,
    InvalidTimestamp,
    InvalidConfidenceLevel,
    InvalidAllocation(String),
    DuplicateStratum(String),
    UnknownStratum(String),
    DuplicateUnit(String),
    InvalidObservation(String),
    BaselineNotAvailableAtAssignment(String),
    OutcomeUnavailableAtAnalysis(String),
    FollowupHorizonNotMet(String),
    UnitOrMetricMismatch(String),
    StratumAllocationMismatch(String),
    PositivityFailure(String),
    InsufficientWithinArmReplication(String),
    TooManyRandomizationUnits(usize),
    RandomizationSpaceEmpty,
    InvalidOracle,
    EstimateMismatch,
    OracleUnitMismatch(String),
    OracleObservedOutcomeMismatch(String),
    Serialization,
}

impl fmt::Display for MaintenanceEffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchemaVersion => {
                write!(formatter, "unexpected maintenance-effect schema version")
            }
            Self::InvalidIdentity => write!(formatter, "maintenance-effect identity is empty"),
            Self::InvalidDigest => write!(
                formatter,
                "maintenance-effect digest is not lowercase SHA-256"
            ),
            Self::NonRandomizedDesign => write!(
                formatter,
                "P0 estimator accepts blocked randomized assignment only"
            ),
            Self::InvalidTimestamp => write!(formatter, "maintenance-effect timestamp is invalid"),
            Self::InvalidConfidenceLevel => {
                write!(formatter, "confidence level must be 0.90, 0.95 or 0.99")
            }
            Self::InvalidAllocation(id) => write!(formatter, "invalid blocked allocation: {id}"),
            Self::DuplicateStratum(id) => {
                write!(formatter, "duplicate randomization stratum: {id}")
            }
            Self::UnknownStratum(id) => {
                write!(formatter, "observation refers to unknown stratum: {id}")
            }
            Self::DuplicateUnit(id) => write!(formatter, "duplicate randomization unit: {id}"),
            Self::InvalidObservation(id) => {
                write!(formatter, "invalid observation or provenance: {id}")
            }
            Self::BaselineNotAvailableAtAssignment(id) => write!(
                formatter,
                "baseline was not available at assignment time: {id}"
            ),
            Self::OutcomeUnavailableAtAnalysis(id) => write!(
                formatter,
                "outcome was not available at analysis time: {id}"
            ),
            Self::FollowupHorizonNotMet(id) => write!(
                formatter,
                "outcome does not meet the preregistered follow-up horizon: {id}"
            ),
            Self::UnitOrMetricMismatch(id) => write!(
                formatter,
                "outcome metric or unit does not match the trial: {id}"
            ),
            Self::StratumAllocationMismatch(id) => write!(
                formatter,
                "observed arm counts do not match the frozen stratum allocation: {id}"
            ),
            Self::PositivityFailure(id) => write!(
                formatter,
                "both randomized arms are not represented in stratum: {id}"
            ),
            Self::InsufficientWithinArmReplication(id) => write!(
                formatter,
                "at least two independent units per arm are required for variance in stratum: {id}"
            ),
            Self::TooManyRandomizationUnits(count) => write!(
                formatter,
                "exact randomization enumeration is bounded to 16 units; got {count}"
            ),
            Self::RandomizationSpaceEmpty => write!(
                formatter,
                "no assignment satisfies the frozen blocked allocation"
            ),
            Self::InvalidOracle => write!(
                formatter,
                "synthetic potential-outcome oracle is incomplete or invalid"
            ),
            Self::EstimateMismatch => write!(
                formatter,
                "supplied effect estimate does not match the observed trial estimator output"
            ),
            Self::OracleUnitMismatch(id) => write!(
                formatter,
                "oracle unit set differs from randomized trial: {id}"
            ),
            Self::OracleObservedOutcomeMismatch(id) => write!(
                formatter,
                "observed outcome does not match the selected oracle potential outcome: {id}"
            ),
            Self::Serialization => write!(formatter, "failed to serialize maintenance-effect data"),
        }
    }
}

impl std::error::Error for MaintenanceEffectError {}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn confidence_critical_value(confidence_level: f64) -> Result<f64, MaintenanceEffectError> {
    if (confidence_level - 0.90).abs() <= VARIANCE_TOLERANCE {
        Ok(1.644_853_626_951_472_2)
    } else if (confidence_level - 0.95).abs() <= VARIANCE_TOLERANCE {
        Ok(1.959_963_984_540_054)
    } else if (confidence_level - 0.99).abs() <= VARIANCE_TOLERANCE {
        Ok(2.575_829_303_548_900_4)
    } else {
        Err(MaintenanceEffectError::InvalidConfidenceLevel)
    }
}

impl BlockedRandomizedMaintenanceTrial {
    pub fn validate(&self) -> Result<(), MaintenanceEffectError> {
        if self.schema_version != MAINTENANCE_EFFECT_SCHEMA {
            return Err(MaintenanceEffectError::InvalidSchemaVersion);
        }
        if self.assignment_method != AssignmentMethod::BlockRandomized {
            return Err(MaintenanceEffectError::NonRandomizedDesign);
        }
        if self.trial_id.trim().is_empty()
            || self.treatment_label.trim().is_empty()
            || self.control_label.trim().is_empty()
            || self.outcome_metric_id.trim().is_empty()
            || self.outcome_unit.trim().is_empty()
            || self.assignment_audit_ref.trim().is_empty()
        {
            return Err(MaintenanceEffectError::InvalidIdentity);
        }
        if !is_sha256_hex(&self.assignment_plan_sha256) {
            return Err(MaintenanceEffectError::InvalidDigest);
        }
        if !self.assignment_at_seconds.is_finite()
            || !self.followup_horizon_seconds.is_finite()
            || !self.analysis_as_of_seconds.is_finite()
            || self.assignment_at_seconds < 0.0
            || self.followup_horizon_seconds <= 0.0
            || self.analysis_as_of_seconds
                < self.assignment_at_seconds + self.followup_horizon_seconds
        {
            return Err(MaintenanceEffectError::InvalidTimestamp);
        }
        confidence_critical_value(self.confidence_level)?;

        if self.stratum_allocations.is_empty() || self.observations.is_empty() {
            return Err(MaintenanceEffectError::InvalidAllocation(
                "trial requires strata and observations".into(),
            ));
        }

        let mut allocations = BTreeMap::<String, (usize, usize)>::new();
        for allocation in &self.stratum_allocations {
            if allocation.stratum_id.trim().is_empty()
                || allocation.planned_treatment_count < 2
                || allocation.planned_control_count < 2
            {
                return Err(MaintenanceEffectError::InvalidAllocation(
                    allocation.stratum_id.clone(),
                ));
            }
            if allocations
                .insert(
                    allocation.stratum_id.clone(),
                    (
                        allocation.planned_treatment_count,
                        allocation.planned_control_count,
                    ),
                )
                .is_some()
            {
                return Err(MaintenanceEffectError::DuplicateStratum(
                    allocation.stratum_id.clone(),
                ));
            }
        }

        let mut units = BTreeSet::new();
        let mut observed_counts = BTreeMap::<String, (usize, usize)>::new();
        for observation in &self.observations {
            if observation.unit_id.trim().is_empty() {
                return Err(MaintenanceEffectError::InvalidIdentity);
            }
            if !units.insert(observation.unit_id.clone()) {
                return Err(MaintenanceEffectError::DuplicateUnit(
                    observation.unit_id.clone(),
                ));
            }
            if !allocations.contains_key(&observation.stratum_id) {
                return Err(MaintenanceEffectError::UnknownStratum(
                    observation.stratum_id.clone(),
                ));
            }
            if !is_sha256_hex(&observation.assignment_evidence_sha256)
                || !is_sha256_hex(&observation.baseline_evidence_sha256)
                || !is_sha256_hex(&observation.baseline_calibration_record_sha256)
                || !is_sha256_hex(&observation.outcome_evidence_sha256)
                || !is_sha256_hex(&observation.outcome_calibration_record_sha256)
                || observation.baseline_source_id.trim().is_empty()
                || observation.baseline_measurement_principle_id.trim().is_empty()
                || observation.baseline_dependency_domain_id.trim().is_empty()
                || observation.outcome_source_id.trim().is_empty()
                || observation.outcome_measurement_principle_id.trim().is_empty()
                || observation.outcome_dependency_domain_id.trim().is_empty()
                || observation.baseline_unit.trim().is_empty()
                || observation.outcome_unit.trim().is_empty()
                || !observation.baseline_value.is_finite()
                || !observation.observed_outcome_value.is_finite()
                || !observation.baseline_uncertainty_abs.is_finite()
                || observation.baseline_uncertainty_abs < 0.0
                || !observation.outcome_uncertainty_abs.is_finite()
                || observation.outcome_uncertainty_abs < 0.0
                || !observation.assignment_available_at_seconds.is_finite()
                || !observation.baseline_observed_at_seconds.is_finite()
                || !observation.baseline_available_at_seconds.is_finite()
                || !observation.baseline_calibration_valid_until_seconds.is_finite()
                || !observation.outcome_observed_at_seconds.is_finite()
                || !observation.outcome_available_at_seconds.is_finite()
                || !observation.outcome_calibration_valid_until_seconds.is_finite()
                || observation.baseline_quality != MeasurementQuality::Valid
                || observation.outcome_quality != MeasurementQuality::Valid
                || observation.baseline_calibration_status != CalibrationStatus::Verified
                || observation.outcome_calibration_status != CalibrationStatus::Verified
                || observation.baseline_calibration_valid_until_seconds
                    < observation.baseline_available_at_seconds
                || observation.outcome_calibration_valid_until_seconds
                    < observation.outcome_available_at_seconds
                || observation.outcome_measurement_principle_id
                    != self.outcome_measurement_principle_id
            {
                return Err(MaintenanceEffectError::InvalidObservation(
                    observation.unit_id.clone(),
                ));
            }
            if observation.assignment_available_at_seconds < 0.0
                || observation.assignment_available_at_seconds
                    > self.assignment_at_seconds + TIME_TOLERANCE_SECONDS
            {
                return Err(MaintenanceEffectError::InvalidObservation(
                    observation.unit_id.clone(),
                ));
            }
            if observation.baseline_observed_at_seconds < 0.0
                || observation.baseline_available_at_seconds
                    < observation.baseline_observed_at_seconds
                || observation.baseline_observed_at_seconds
                    >= self.assignment_at_seconds - TIME_TOLERANCE_SECONDS
                || observation.baseline_available_at_seconds
                    > self.assignment_at_seconds + TIME_TOLERANCE_SECONDS
            {
                return Err(MaintenanceEffectError::BaselineNotAvailableAtAssignment(
                    observation.unit_id.clone(),
                ));
            }
            if observation.outcome_observed_at_seconds < 0.0
                || observation.outcome_available_at_seconds < 0.0
                || observation.outcome_observed_at_seconds
                    < self.assignment_at_seconds + self.followup_horizon_seconds
                        - TIME_TOLERANCE_SECONDS
            {
                return Err(MaintenanceEffectError::FollowupHorizonNotMet(
                    observation.unit_id.clone(),
                ));
            }
            if observation.outcome_available_at_seconds < observation.outcome_observed_at_seconds
                || observation.outcome_available_at_seconds
                    > self.analysis_as_of_seconds + TIME_TOLERANCE_SECONDS
            {
                return Err(MaintenanceEffectError::OutcomeUnavailableAtAnalysis(
                    observation.unit_id.clone(),
                ));
            }
            if observation.baseline_unit != self.outcome_unit
                || observation.outcome_unit != self.outcome_unit
            {
                return Err(MaintenanceEffectError::UnitOrMetricMismatch(
                    observation.unit_id.clone(),
                ));
            }
            let entry = observed_counts
                .entry(observation.stratum_id.clone())
                .or_insert((0, 0));
            match observation.assigned_arm {
                AssignedArm::Treatment => entry.0 += 1,
                AssignedArm::Control => entry.1 += 1,
            }
        }

        for (stratum_id, expected) in &allocations {
            let observed = observed_counts.get(stratum_id).copied().unwrap_or((0, 0));
            if observed.0 == 0 || observed.1 == 0 {
                return Err(MaintenanceEffectError::PositivityFailure(
                    stratum_id.clone(),
                ));
            }
            if observed.0 < 2 || observed.1 < 2 {
                return Err(MaintenanceEffectError::InsufficientWithinArmReplication(
                    stratum_id.clone(),
                ));
            }
            if observed != *expected {
                return Err(MaintenanceEffectError::StratumAllocationMismatch(
                    stratum_id.clone(),
                ));
            }
        }
        if observed_counts.len() != allocations.len() {
            return Err(MaintenanceEffectError::InvalidAllocation(
                "unexpected observed stratum".into(),
            ));
        }
        Ok(())
    }
}

fn arm_means_for_stratum(
    observations: &[&RandomizedUnitObservation],
    stratum_id: &str,
    treatment_mask: u64,
    unit_indices: &BTreeMap<String, usize>,
) -> Option<(f64, f64)> {
    let mut treatment_sum = 0.0;
    let mut control_sum = 0.0;
    let mut treatment_count = 0usize;
    let mut control_count = 0usize;
    for observation in observations {
        if observation.stratum_id != stratum_id {
            continue;
        }
        let bit = 1u64 << unit_indices[&observation.unit_id];
        let assigned_treatment = (treatment_mask & bit) != 0;
        if assigned_treatment {
            treatment_sum += observation.observed_outcome_value;
            treatment_count += 1;
        } else {
            control_sum += observation.observed_outcome_value;
            control_count += 1;
        }
    }
    if treatment_count == 0 || control_count == 0 {
        return None;
    }
    Some((
        treatment_sum / treatment_count as f64,
        control_sum / control_count as f64,
    ))
}

fn stratified_difference_in_means(
    trial: &BlockedRandomizedMaintenanceTrial,
    treatment_mask: u64,
    unit_indices: &BTreeMap<String, usize>,
) -> Option<f64> {
    let refs = trial.observations.iter().collect::<Vec<_>>();
    let total_count = refs.len() as f64;
    let mut weighted_effect = 0.0;
    for allocation in &trial.stratum_allocations {
        let (treatment_mean, control_mean) =
            arm_means_for_stratum(&refs, &allocation.stratum_id, treatment_mask, unit_indices)?;
        let stratum_count =
            (allocation.planned_treatment_count + allocation.planned_control_count) as f64;
        weighted_effect += (stratum_count / total_count) * (treatment_mean - control_mean);
    }
    Some(weighted_effect)
}

fn sample_variance(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    Some(
        values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (values.len() - 1) as f64,
    )
}

fn exact_randomization_p_value(
    trial: &BlockedRandomizedMaintenanceTrial,
    observed_effect: f64,
) -> Result<f64, MaintenanceEffectError> {
    let count = trial.observations.len();
    if count > MAX_EXACT_RANDOMIZATION_UNITS {
        return Err(MaintenanceEffectError::TooManyRandomizationUnits(count));
    }
    let unit_indices = trial
        .observations
        .iter()
        .enumerate()
        .map(|(index, observation)| (observation.unit_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let observed_mask = trial
        .observations
        .iter()
        .enumerate()
        .filter(|(_, observation)| observation.assigned_arm == AssignedArm::Treatment)
        .fold(0u64, |mask, (index, _)| mask | (1u64 << index));

    let mut valid_assignments = 0usize;
    let mut extreme_assignments = 0usize;
    let assignment_space = 1u64 << count;
    for mask in 0..assignment_space {
        let mut valid = true;
        for allocation in &trial.stratum_allocations {
            let mut assigned_treatment = 0usize;
            for observation in &trial.observations {
                if observation.stratum_id == allocation.stratum_id {
                    let bit = 1u64 << unit_indices[&observation.unit_id];
                    if (mask & bit) != 0 {
                        assigned_treatment += 1;
                    }
                }
            }
            if assigned_treatment != allocation.planned_treatment_count {
                valid = false;
                break;
            }
        }
        if !valid {
            continue;
        }
        let effect = stratified_difference_in_means(trial, mask, &unit_indices)
            .ok_or(MaintenanceEffectError::RandomizationSpaceEmpty)?;
        valid_assignments += 1;
        if effect.abs() + VARIANCE_TOLERANCE >= observed_effect.abs() {
            extreme_assignments += 1;
        }
    }
    if valid_assignments == 0 {
        return Err(MaintenanceEffectError::RandomizationSpaceEmpty);
    }
    // The observed allocation is part of the exhaustive reference distribution.
    let _ = observed_mask;
    Ok(extreme_assignments as f64 / valid_assignments as f64)
}

/// Estimate the stratified finite-population treatment effect from observed outcomes only.
/// Baseline values are validated as pre-assignment but are not adjusted in this v0.1 estimator.
pub fn estimate_blocked_randomized_maintenance_effect(
    trial: &BlockedRandomizedMaintenanceTrial,
) -> Result<RandomizedMaintenanceEffectEstimate, MaintenanceEffectError> {
    trial.validate()?;
    let total_count = trial.observations.len() as f64;
    let mut ate = 0.0;
    let mut variance = 0.0;
    let mut measurement_error_bound_abs = 0.0;
    for allocation in &trial.stratum_allocations {
        let treatment = trial
            .observations
            .iter()
            .filter(|item| {
                item.stratum_id == allocation.stratum_id
                    && item.assigned_arm == AssignedArm::Treatment
            })
            .map(|item| item.observed_outcome_value)
            .collect::<Vec<_>>();
        let control = trial
            .observations
            .iter()
            .filter(|item| {
                item.stratum_id == allocation.stratum_id
                    && item.assigned_arm == AssignedArm::Control
            })
            .map(|item| item.observed_outcome_value)
            .collect::<Vec<_>>();
        let treatment_variance = sample_variance(&treatment).ok_or_else(|| {
            MaintenanceEffectError::InsufficientWithinArmReplication(allocation.stratum_id.clone())
        })?;
        let control_variance = sample_variance(&control).ok_or_else(|| {
            MaintenanceEffectError::InsufficientWithinArmReplication(allocation.stratum_id.clone())
        })?;
        let treatment_mean = treatment.iter().sum::<f64>() / treatment.len() as f64;
        let control_mean = control.iter().sum::<f64>() / control.len() as f64;
        let stratum_count = (treatment.len() + control.len()) as f64;
        let weight = stratum_count / total_count;
        ate += weight * (treatment_mean - control_mean);
        variance += weight.powi(2)
            * (treatment_variance / treatment.len() as f64
                + control_variance / control.len() as f64);

        let treatment_measurement_bound = trial
            .observations
            .iter()
            .filter(|item| {
                item.stratum_id == allocation.stratum_id
                    && item.assigned_arm == AssignedArm::Treatment
            })
            .map(|item| item.outcome_uncertainty_abs)
            .sum::<f64>()
            / treatment.len() as f64;
        let control_measurement_bound = trial
            .observations
            .iter()
            .filter(|item| {
                item.stratum_id == allocation.stratum_id
                    && item.assigned_arm == AssignedArm::Control
            })
            .map(|item| item.outcome_uncertainty_abs)
            .sum::<f64>()
            / control.len() as f64;
        measurement_error_bound_abs +=
            weight * (treatment_measurement_bound + control_measurement_bound);
    }
    let standard_error = variance.sqrt();
    let critical = confidence_critical_value(trial.confidence_level)?;
    let interval = [
        ate - critical * standard_error,
        ate + critical * standard_error,
    ];
    let robust_interval = [
        interval[0] - measurement_error_bound_abs,
        interval[1] + measurement_error_bound_abs,
    ];
    let benefit_interval = match trial.outcome_direction {
        OutcomeDirection::HigherIsBetter => interval,
        OutcomeDirection::LowerIsBetter => [-interval[1], -interval[0]],
    };
    let robust_benefit_interval = match trial.outcome_direction {
        OutcomeDirection::HigherIsBetter => robust_interval,
        OutcomeDirection::LowerIsBetter => [-robust_interval[1], -robust_interval[0]],
    };
    let benefit_oriented_effect = match trial.outcome_direction {
        OutcomeDirection::HigherIsBetter => ate,
        OutcomeDirection::LowerIsBetter => -ate,
    };
    let unit_indices = trial
        .observations
        .iter()
        .enumerate()
        .map(|(index, item)| (item.unit_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let observed_mask = trial
        .observations
        .iter()
        .enumerate()
        .filter(|(_, item)| item.assigned_arm == AssignedArm::Treatment)
        .fold(0u64, |mask, (index, _)| mask | (1u64 << index));
    let exact_observed = stratified_difference_in_means(trial, observed_mask, &unit_indices)
        .ok_or(MaintenanceEffectError::RandomizationSpaceEmpty)?;
    let randomization_p = exact_randomization_p_value(trial, exact_observed)?;

    Ok(RandomizedMaintenanceEffectEstimate {
        schema_version: MAINTENANCE_EFFECT_SCHEMA.to_owned(),
        trial_id: trial.trial_id.clone(),
        treatment_label: trial.treatment_label.clone(),
        control_label: trial.control_label.clone(),
        outcome_metric_id: trial.outcome_metric_id.clone(),
        outcome_unit: trial.outcome_unit.clone(),
        estimand: "finite_population_average_treatment_effect_for_the_randomized_eligible_units"
            .into(),
        estimator: "stratum_weighted_difference_in_means_with_neyman_conservative_variance".into(),
        ate_treatment_minus_control: ate,
        benefit_oriented_effect,
        standard_error,
        confidence_level: trial.confidence_level,
        confidence_interval_treatment_minus_control: interval,
        confidence_interval_benefit_oriented: benefit_interval,
        measurement_error_bound_abs,
        measurement_robust_interval_treatment_minus_control: robust_interval,
        measurement_robust_interval_benefit_oriented: robust_benefit_interval,
        exact_randomization_p_value: randomization_p,
        independent_randomization_units: trial.observations.len(),
        stratum_count: trial.stratum_allocations.len(),
        baseline_adjustment_used: false,
        potential_outcome_oracle_used_for_estimation: false,
        maintenance_authorized: false,
        physical_restoration_authorized: false,
        operating_restriction_clearance_authorized: false,
    })
}

/// Score an estimate against an oracle that is separate from the estimator input.
/// This function is only appropriate for synthetic or controlled known-truth experiments.
pub fn score_synthetic_effect_against_oracle(
    trial: &BlockedRandomizedMaintenanceTrial,
    estimate: &RandomizedMaintenanceEffectEstimate,
    oracle: &SyntheticPotentialOutcomeOracle,
) -> Result<SyntheticOracleScore, MaintenanceEffectError> {
    trial.validate()?;
    if oracle.schema_version != MAINTENANCE_EFFECT_ORACLE_SCHEMA
        || oracle.trial_id != trial.trial_id
        || !is_sha256_hex(&oracle.oracle_dataset_sha256)
        || estimate.trial_id != trial.trial_id
    {
        return Err(MaintenanceEffectError::InvalidOracle);
    }
    let recomputed = estimate_blocked_randomized_maintenance_effect(trial)?;
    if (estimate.ate_treatment_minus_control - recomputed.ate_treatment_minus_control).abs()
        > VARIANCE_TOLERANCE
        || (estimate.standard_error - recomputed.standard_error).abs() > VARIANCE_TOLERANCE
        || estimate.confidence_interval_treatment_minus_control
            != recomputed.confidence_interval_treatment_minus_control
        || (estimate.measurement_error_bound_abs - recomputed.measurement_error_bound_abs).abs()
            > VARIANCE_TOLERANCE
        || estimate.measurement_robust_interval_treatment_minus_control
            != recomputed.measurement_robust_interval_treatment_minus_control
        || estimate.exact_randomization_p_value != recomputed.exact_randomization_p_value
    {
        return Err(MaintenanceEffectError::EstimateMismatch);
    }
    let mut oracle_map = BTreeMap::new();
    for outcome in &oracle.outcomes {
        if outcome.unit_id.trim().is_empty()
            || !outcome.potential_outcome_treatment.is_finite()
            || !outcome.potential_outcome_control.is_finite()
            || !is_sha256_hex(&outcome.oracle_record_sha256)
            || oracle_map
                .insert(outcome.unit_id.clone(), outcome)
                .is_some()
        {
            return Err(MaintenanceEffectError::InvalidOracle);
        }
    }
    if oracle_map.len() != trial.observations.len() {
        return Err(MaintenanceEffectError::InvalidOracle);
    }
    let mut true_effect_sum = 0.0;
    for observation in &trial.observations {
        let Some(potential) = oracle_map.get(&observation.unit_id) else {
            return Err(MaintenanceEffectError::OracleUnitMismatch(
                observation.unit_id.clone(),
            ));
        };
        let expected = match observation.assigned_arm {
            AssignedArm::Treatment => potential.potential_outcome_treatment,
            AssignedArm::Control => potential.potential_outcome_control,
        };
        if (expected - observation.observed_outcome_value).abs() > VARIANCE_TOLERANCE {
            return Err(MaintenanceEffectError::OracleObservedOutcomeMismatch(
                observation.unit_id.clone(),
            ));
        }
        true_effect_sum +=
            potential.potential_outcome_treatment - potential.potential_outcome_control;
    }
    let truth = true_effect_sum / trial.observations.len() as f64;
    let error = estimate.ate_treatment_minus_control - truth;
    Ok(SyntheticOracleScore {
        trial_id: trial.trial_id.clone(),
        estimated_ate: estimate.ate_treatment_minus_control,
        true_finite_population_ate: truth,
        estimation_error: error,
        absolute_bias: error.abs(),
        interval_contains_truth: estimate.confidence_interval_treatment_minus_control[0] <= truth
            && truth <= estimate.confidence_interval_treatment_minus_control[1],
        oracle_used_for_estimation: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn fixture() -> (
        BlockedRandomizedMaintenanceTrial,
        SyntheticPotentialOutcomeOracle,
    ) {
        let mut observations = Vec::new();
        let mut oracle = Vec::new();
        let effects = [2.0, 4.0, 6.0];
        for (stratum_index, effect) in effects.iter().enumerate() {
            let stratum_id = format!("load-band-{stratum_index}");
            let baseline = [8.0, 10.0, 12.0, 14.0];
            for (unit_index, control_outcome) in baseline.iter().enumerate() {
                let unit_id = format!("asset-{stratum_index}-{unit_index}");
                let assigned_arm = if unit_index == 0 || unit_index == 3 {
                    AssignedArm::Treatment
                } else {
                    AssignedArm::Control
                };
                let treatment_outcome = control_outcome + effect;
                let observed = match assigned_arm {
                    AssignedArm::Treatment => treatment_outcome,
                    AssignedArm::Control => *control_outcome,
                };
                let digest_base = (stratum_index * 4 + unit_index) * 4 + 1;
                let assignment_digest = format!("{digest_base:064x}");
                let baseline_digest = format!("{:064x}", digest_base + 1);
                let outcome_digest = format!("{:064x}", digest_base + 2);
                let oracle_digest = format!("{:064x}", digest_base + 3);
                observations.push(RandomizedUnitObservation {
                    unit_id: unit_id.clone(),
                    stratum_id: stratum_id.clone(),
                    assigned_arm,
                    assignment_evidence_sha256: assignment_digest,
                    assignment_available_at_seconds: 100.0,
                    baseline_value: 5.0 + unit_index as f64,
                    baseline_unit: "normalized_capability".into(),
                    baseline_observed_at_seconds: 80.0,
                    baseline_available_at_seconds: 90.0,
                    baseline_evidence_sha256: baseline_digest,
                    baseline_source_id: format!("baseline-source-{unit_id}"),
                    baseline_measurement_principle_id: "calibrated_shaft_torque".into(),
                    baseline_dependency_domain_id: format!("baseline-daq-{unit_id}"),
                    baseline_quality: MeasurementQuality::Valid,
                    baseline_calibration_status: CalibrationStatus::Verified,
                    baseline_calibration_record_sha256: format!("{:064x}", digest_base + 4),
                    baseline_calibration_valid_until_seconds: 1000.0,
                    baseline_uncertainty_abs: 0.1,
                    observed_outcome_value: observed,
                    outcome_unit: "normalized_capability".into(),
                    outcome_observed_at_seconds: 210.0,
                    outcome_available_at_seconds: 220.0,
                    outcome_evidence_sha256: outcome_digest,
                    outcome_source_id: format!("outcome-source-{unit_id}"),
                    outcome_measurement_principle_id: "calibrated_shaft_torque".into(),
                    outcome_dependency_domain_id: format!("outcome-daq-{unit_id}"),
                    outcome_quality: MeasurementQuality::Valid,
                    outcome_calibration_status: CalibrationStatus::Verified,
                    outcome_calibration_record_sha256: format!("{:064x}", digest_base + 5),
                    outcome_calibration_valid_until_seconds: 1000.0,
                    outcome_uncertainty_abs: 0.25,
                });
                oracle.push(UnitPotentialOutcomes {
                    unit_id,
                    potential_outcome_treatment: treatment_outcome,
                    potential_outcome_control: *control_outcome,
                    oracle_record_sha256: oracle_digest,
                });
            }
        }
        (
            BlockedRandomizedMaintenanceTrial {
                schema_version: MAINTENANCE_EFFECT_SCHEMA.into(),
                trial_id: "synthetic-randomized-repair-v1".into(),
                assignment_method: AssignmentMethod::BlockRandomized,
                treatment_label: "physical_component_repair".into(),
                control_label: "matched_deferred_control".into(),
                outcome_metric_id: "normalized_propulsion_capability".into(),
                outcome_unit: "normalized_capability".into(),
                outcome_measurement_principle_id: "calibrated_shaft_torque".into(),
                outcome_direction: OutcomeDirection::HigherIsBetter,
                assignment_at_seconds: 100.0,
                followup_horizon_seconds: 100.0,
                analysis_as_of_seconds: 250.0,
                confidence_level: 0.95,
                assignment_plan_sha256: digest('c'),
                assignment_audit_ref: "synthetic-randomization-plan-001".into(),
                stratum_allocations: effects
                    .iter()
                    .enumerate()
                    .map(|(index, _)| StratumAllocation {
                        stratum_id: format!("load-band-{index}"),
                        planned_treatment_count: 2,
                        planned_control_count: 2,
                    })
                    .collect(),
                observations,
            },
            SyntheticPotentialOutcomeOracle {
                schema_version: MAINTENANCE_EFFECT_ORACLE_SCHEMA.into(),
                trial_id: "synthetic-randomized-repair-v1".into(),
                oracle_dataset_sha256: digest('d'),
                outcomes: oracle,
            },
        )
    }

    #[test]
    fn blocked_randomized_estimator_recovers_known_finite_population_effect() {
        let (trial, oracle) = fixture();
        let estimate = estimate_blocked_randomized_maintenance_effect(&trial).unwrap();
        let score = score_synthetic_effect_against_oracle(&trial, &estimate, &oracle).unwrap();
        assert!((estimate.ate_treatment_minus_control - 4.0).abs() < 1e-9);
        assert!((score.true_finite_population_ate - 4.0).abs() < 1e-9);
        assert!(score.absolute_bias < 1e-9);
        assert!(!estimate.potential_outcome_oracle_used_for_estimation);
        assert!(!score.oracle_used_for_estimation);
        assert!((0.0..=1.0).contains(&estimate.exact_randomization_p_value));
    }

    #[test]
    fn lower_is_better_outcome_reorients_benefit_without_changing_raw_ate() {
        let (mut trial, _) = fixture();
        trial.outcome_direction = OutcomeDirection::LowerIsBetter;
        let estimate = estimate_blocked_randomized_maintenance_effect(&trial).unwrap();
        assert_eq!(estimate.ate_treatment_minus_control, 4.0);
        assert_eq!(estimate.benefit_oriented_effect, -4.0);
    }

    #[test]
    fn observational_assignments_are_rejected_by_the_randomized_p0_estimator() {
        let (mut trial, _) = fixture();
        trial.assignment_method = AssignmentMethod::Observational;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::NonRandomizedDesign)
        );
    }

    #[test]
    fn future_baseline_evidence_cannot_be_used_for_randomization_eligibility() {
        let (mut trial, _) = fixture();
        trial.observations[0].baseline_available_at_seconds = 101.0;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::BaselineNotAvailableAtAssignment(
                trial.observations[0].unit_id.clone()
            ))
        );
    }

    #[test]
    fn future_outcome_evidence_is_not_used_before_analysis_availability() {
        let (mut trial, _) = fixture();
        trial.observations[0].outcome_available_at_seconds = 251.0;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::OutcomeUnavailableAtAnalysis(
                trial.observations[0].unit_id.clone()
            ))
        );
    }

    #[test]
    fn post_treatment_outcome_before_followup_horizon_is_rejected() {
        let (mut trial, _) = fixture();
        trial.observations[0].outcome_observed_at_seconds = 150.0;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::FollowupHorizonNotMet(
                trial.observations[0].unit_id.clone()
            ))
        );
    }

    #[test]
    fn every_randomization_stratum_requires_both_arms_and_two_units_each() {
        let (mut trial, _) = fixture();
        trial.stratum_allocations[0].planned_treatment_count = 3;
        trial.stratum_allocations[0].planned_control_count = 1;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::InvalidAllocation(
                "load-band-0".into()
            ))
        );
    }

    #[test]
    fn exact_randomization_is_bounded_and_fails_closed_above_sixteen_units() {
        let (mut trial, _) = fixture();
        for index in 0..5 {
            let mut extra = trial.observations[index].clone();
            extra.unit_id = format!("extra-{index}");
            extra.stratum_id = "overflow-stratum".into();
            trial.observations.push(extra);
        }
        trial.stratum_allocations.push(StratumAllocation {
            stratum_id: "overflow-stratum".into(),
            planned_treatment_count: 3,
            planned_control_count: 2,
        });
        trial.observations[12].assigned_arm = AssignedArm::Treatment;
        trial.observations[13].assigned_arm = AssignedArm::Treatment;
        trial.observations[14].assigned_arm = AssignedArm::Treatment;
        trial.observations[15].assigned_arm = AssignedArm::Control;
        trial.observations[16].assigned_arm = AssignedArm::Control;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::TooManyRandomizationUnits(17))
        );
    }

    #[test]
    fn assignment_evidence_available_after_assignment_time_is_rejected() {
        let (mut trial, _) = fixture();
        trial.observations[0].assignment_available_at_seconds = 101.0;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::InvalidObservation(
                trial.observations[0].unit_id.clone()
            ))
        );
    }

    #[test]
    fn baseline_availability_cannot_precede_its_observation_time() {
        let (mut trial, _) = fixture();
        trial.observations[0].baseline_available_at_seconds = 70.0;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::BaselineNotAvailableAtAssignment(
                trial.observations[0].unit_id.clone()
            ))
        );
    }

    #[test]
    fn calibration_validity_and_outcome_quality_are_hard_gates() {
        let (mut trial, _) = fixture();
        trial.observations[0].outcome_calibration_valid_until_seconds = 219.0;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::InvalidObservation(
                trial.observations[0].unit_id.clone()
            ))
        );

        let (mut stale_trial, _) = fixture();
        stale_trial.observations[0].outcome_quality = MeasurementQuality::Stale;
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&stale_trial),
            Err(MaintenanceEffectError::InvalidObservation(
                stale_trial.observations[0].unit_id.clone()
            ))
        );
    }

    #[test]
    fn outcome_measurement_principle_must_match_the_frozen_trial_contract() {
        let (mut trial, _) = fixture();
        trial.observations[0].outcome_measurement_principle_id =
            "uncalibrated_alternate_principle".into();
        assert_eq!(
            estimate_blocked_randomized_maintenance_effect(&trial),
            Err(MaintenanceEffectError::InvalidObservation(
                trial.observations[0].unit_id.clone()
            ))
        );
    }

    #[test]
    fn deterministic_measurement_error_bound_expands_interval_without_inflating_sampling_se() {
        let (mut trial, _) = fixture();
        let with_measurement_bound =
            estimate_blocked_randomized_maintenance_effect(&trial).unwrap();
        for observation in &mut trial.observations {
            observation.outcome_uncertainty_abs = 0.0;
        }
        let without_measurement_bound =
            estimate_blocked_randomized_maintenance_effect(&trial).unwrap();

        assert!((with_measurement_bound.measurement_error_bound_abs - 0.5).abs() < 1e-12);
        assert_eq!(with_measurement_bound.standard_error, without_measurement_bound.standard_error);
        assert!(
            with_measurement_bound.measurement_robust_interval_treatment_minus_control[0]
                < with_measurement_bound.confidence_interval_treatment_minus_control[0]
        );
        assert!(
            with_measurement_bound.measurement_robust_interval_treatment_minus_control[1]
                > with_measurement_bound.confidence_interval_treatment_minus_control[1]
        );
    }

    #[test]
    fn changing_only_unobserved_potential_outcomes_changes_scoring_not_estimation() {
        let (trial, oracle) = fixture();
        let estimate = estimate_blocked_randomized_maintenance_effect(&trial).unwrap();
        let original_score =
            score_synthetic_effect_against_oracle(&trial, &estimate, &oracle).unwrap();
        let mut counterfactual_oracle = oracle;
        for (observation, potential) in trial
            .observations
            .iter()
            .zip(counterfactual_oracle.outcomes.iter_mut())
        {
            if observation.assigned_arm == AssignedArm::Treatment {
                potential.potential_outcome_control += 10.0;
            }
        }
        let recomputed_estimate = estimate_blocked_randomized_maintenance_effect(&trial).unwrap();
        let counterfactual_score = score_synthetic_effect_against_oracle(
            &trial,
            &recomputed_estimate,
            &counterfactual_oracle,
        )
        .unwrap();
        assert_eq!(
            estimate.ate_treatment_minus_control,
            recomputed_estimate.ate_treatment_minus_control
        );
        assert_ne!(
            original_score.true_finite_population_ate,
            counterfactual_score.true_finite_population_ate
        );
        assert!(!recomputed_estimate.potential_outcome_oracle_used_for_estimation);
    }

    #[test]
    fn tampered_oracle_is_rejected_and_never_changes_the_point_estimate() {
        let (trial, mut oracle) = fixture();
        let estimate = estimate_blocked_randomized_maintenance_effect(&trial).unwrap();
        oracle.outcomes[0].potential_outcome_treatment += 1.0;
        assert_eq!(
            score_synthetic_effect_against_oracle(&trial, &estimate, &oracle),
            Err(MaintenanceEffectError::OracleObservedOutcomeMismatch(
                trial.observations[0].unit_id.clone()
            ))
        );
        let estimate_again = estimate_blocked_randomized_maintenance_effect(&trial).unwrap();
        assert_eq!(
            estimate.ate_treatment_minus_control,
            estimate_again.ate_treatment_minus_control
        );
    }

    #[test]
    fn exact_randomization_reference_distribution_contains_observed_assignment() {
        let (trial, _) = fixture();
        let estimate = estimate_blocked_randomized_maintenance_effect(&trial).unwrap();
        assert!(estimate.exact_randomization_p_value > 0.0);
    }

    #[test]
    fn protocol_and_case_manifest_keep_randomization_limits_machine_readable() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../maintenance-effect-v0.1.json")).unwrap();
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../maintenance-effect-cases-v0.1.json")).unwrap();
        assert_eq!(protocol["schema_version"], MAINTENANCE_EFFECT_SCHEMA);
        assert_eq!(protocol["status"], "protocol_only_not_executed");
        assert_eq!(protocol["estimator"]["exact_test_limit_units"], 16);
        assert!(
            protocol["hard_gates"].as_array().unwrap().len() >= 29
        );
        assert!(
            protocol["estimator"]["measurement_provenance"]
                .as_object()
                .is_some()
        );
        assert_eq!(
            corpus["schema_version"],
            "symtropy-marine-maintenance-effect-cases-v0.1"
        );
        assert_eq!(corpus["cases"].as_array().unwrap().len(), 14);
    }
}
