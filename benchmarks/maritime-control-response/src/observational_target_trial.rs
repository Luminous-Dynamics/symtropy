#![forbid(unsafe_code)]

//! Structural target-trial emulation contract for observational maintenance data.
//!
//! This P0 module validates the design and refuses to estimate a causal effect.
//! It checks aligned time zero, treatment versions, baseline-only covariates,
//! recorded propensity overlap, follow-up/censoring state and measurement
//! provenance. Passing means only "structurally eligible under declared
//! assumptions"; it does not establish exchangeability or authenticate the DAG.

use crate::maintenance_effect::{CalibrationStatus, MeasurementQuality};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const OBSERVATIONAL_TARGET_TRIAL_SCHEMA: &str =
    "symtropy-marine-observational-target-trial-v0.1";
pub const OBSERVATIONAL_TARGET_TRIAL_ORACLE_SCHEMA: &str =
    "symtropy-marine-observational-target-trial-oracle-v0.1";
const TIME_TOLERANCE_SECONDS: f64 = 1e-9;
const NUMERIC_TOLERANCE: f64 = 1e-12;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationalAssignmentMethod {
    ObservationalTargetTrial,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrategyArm {
    RepairStrategy,
    ComparatorStrategy,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CovariateTemporalRole {
    Baseline,
    PostTreatment,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FollowupStatus {
    Complete,
    LostToFollowup,
    CompetingEvent,
    PreventiveReplacement,
    AdministrativeCensoring,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TargetTrialStratum {
    pub stratum_id: String,
    pub minimum_repair_units: usize,
    pub minimum_comparator_units: usize,
    /// Minimum intersection width of the treatment-propensity ranges
    /// observed in the two strategy arms.
    pub minimum_propensity_overlap: f64,
    pub minimum_effective_sample_size_per_arm: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservationalTargetTrialProtocol {
    pub schema_version: String,
    pub trial_id: String,
    pub assignment_method: ObservationalAssignmentMethod,
    pub target_population_ref: String,
    pub eligibility_rule_sha256: String,
    pub analysis_plan_sha256: String,
    pub preregistration_record_ref: String,
    pub treatment_strategy_version_id: String,
    pub comparator_strategy_version_id: String,
    pub outcome_metric_id: String,
    pub outcome_unit: String,
    pub outcome_measurement_principle_id: String,
    pub causal_graph_sha256: String,
    pub propensity_model_sha256: String,
    pub propensity_fit_data_sha256: String,
    pub propensity_fit_covariate_ids: Vec<String>,
    pub propensity_floor: f64,
    pub propensity_ceiling: f64,
    pub time_zero_tolerance_seconds: f64,
    pub followup_horizon_seconds: f64,
    pub analysis_as_of_seconds: f64,
    pub baseline_covariate_ids: Vec<String>,
    pub strata: Vec<TargetTrialStratum>,
    pub exchangeability_review_ref: String,
    pub treatment_version_review_ref: String,
    pub censoring_review_ref: String,
    pub interference_review_ref: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BaselineCovariateMeasurement {
    pub covariate_id: String,
    pub temporal_role: CovariateTemporalRole,
    pub value: f64,
    pub unit: String,
    pub observed_at_seconds: f64,
    pub available_at_seconds: f64,
    pub source_id: String,
    pub evidence_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TargetTrialOutcome {
    pub metric_id: String,
    pub value: f64,
    pub unit: String,
    pub observed_at_seconds: f64,
    pub available_at_seconds: f64,
    pub source_id: String,
    pub evidence_sha256: String,
    pub measurement_principle_id: String,
    pub dependency_domain_id: String,
    pub quality: MeasurementQuality,
    pub calibration_status: CalibrationStatus,
    pub calibration_record_sha256: String,
    pub calibration_valid_from_seconds: f64,
    pub calibration_valid_until_seconds: f64,
    pub uncertainty_abs: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservationalTargetTrialUnit {
    pub unit_id: String,
    pub asset_id: String,
    pub engine_id: String,
    pub campaign_id: String,
    pub stratum_id: String,
    pub assigned_arm: StrategyArm,
    pub strategy_version_id: String,
    pub strategy_record_ref: String,
    pub strategy_record_sha256: String,
    pub eligibility_at_seconds: f64,
    pub assignment_at_seconds: f64,
    pub followup_start_seconds: f64,
    /// Probability of assignment to the repair strategy, estimated from
    /// pre-treatment covariates only and bound to the protocol model/data digests.
    pub propensity_repair: f64,
    pub baseline_covariates: Vec<BaselineCovariateMeasurement>,
    pub followup_status: FollowupStatus,
    pub censoring_at_seconds: Option<f64>,
    pub concurrent_intervention_ids: Vec<String>,
    pub outcome: Option<TargetTrialOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservationalTargetTrialDataset {
    pub schema_version: String,
    pub protocol: ObservationalTargetTrialProtocol,
    pub units: Vec<ObservationalTargetTrialUnit>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetTrialDesignDisposition {
    StructurallyEligibleUnderDeclaredAssumptions,
    NotIdentified,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetTrialNotIdentifiedReason {
    TimeZeroMisaligned,
    MissingBaselineCovariate,
    PostTreatmentCovariateLeakage,
    TreatmentVersionMismatch,
    PositivityFailure,
    InsufficientEffectiveSampleSize,
    PropensityOutsideDeclaredSupport,
    FollowupIncomplete,
    CensoringRequiresModel,
    ConcurrentInterventionUnresolved,
    OutcomeMissing,
    OutcomeNotInFollowupWindow,
    OutcomeEvidenceUnavailable,
    OutcomeMeasurementContractMismatch,
    AssumptionReviewReferenceMissing,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct StratumOverlapReport {
    pub stratum_id: String,
    pub repair_units: usize,
    pub comparator_units: usize,
    pub repair_propensity_min: Option<f64>,
    pub repair_propensity_max: Option<f64>,
    pub comparator_propensity_min: Option<f64>,
    pub comparator_propensity_max: Option<f64>,
    pub propensity_overlap_width: f64,
    pub repair_effective_sample_size: f64,
    pub comparator_effective_sample_size: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ObservationalTargetTrialReport {
    pub schema_version: String,
    pub trial_id: String,
    pub disposition: TargetTrialDesignDisposition,
    pub not_identified_reasons: Vec<TargetTrialNotIdentifiedReason>,
    pub unit_count: usize,
    pub repair_units: usize,
    pub comparator_units: usize,
    pub complete_followup_units: usize,
    pub censored_or_competing_units: usize,
    pub stratum_reports: Vec<StratumOverlapReport>,
    pub analysis_plan_sha256: String,
    pub causal_graph_sha256: String,
    pub propensity_model_sha256: String,
    pub propensity_fit_data_sha256: String,
    pub dataset_sha256: String,
    pub causal_effect_estimated: bool,
    pub maintenance_authorized: bool,
    pub physical_restoration_authorized: bool,
    pub operating_restriction_clearance_authorized: bool,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UnitPotentialOutcomes {
    pub unit_id: String,
    pub potential_outcome_repair: f64,
    pub potential_outcome_comparator: f64,
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
pub struct UnadjustedAssociationScore {
    pub trial_id: String,
    pub observed_repair_minus_comparator: f64,
    pub true_finite_population_ate: f64,
    pub association_bias: f64,
    pub sign_reversal: bool,
    pub oracle_used_for_association: bool,
    pub causal_effect_estimated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetTrialValidationError {
    InvalidSchemaVersion,
    InvalidIdentity,
    InvalidDigest,
    InvalidProtocol(String),
    DuplicateUnit(String),
    UnknownStratum(String),
    InvalidTimestamp(String),
    InvalidCovariate(String),
    InvalidPropensity(String),
    InvalidOutcome(String),
    InvalidOracle,
    OracleUnitMismatch(String),
    OracleObservedOutcomeMismatch(String),
    Serialization,
}

impl fmt::Display for TargetTrialValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchemaVersion => write!(formatter, "unexpected observational target-trial schema version"),
            Self::InvalidIdentity => write!(formatter, "observational target-trial identity is incomplete"),
            Self::InvalidDigest => write!(formatter, "observational target-trial digest is not lowercase SHA-256"),
            Self::InvalidProtocol(id) => write!(formatter, "invalid target-trial protocol: {id}"),
            Self::DuplicateUnit(id) => write!(formatter, "duplicate independent target-trial unit: {id}"),
            Self::UnknownStratum(id) => write!(formatter, "unit belongs to an undeclared target-trial stratum: {id}"),
            Self::InvalidTimestamp(id) => write!(formatter, "invalid target-trial timestamp for unit: {id}"),
            Self::InvalidCovariate(id) => write!(formatter, "invalid baseline covariate evidence for unit: {id}"),
            Self::InvalidPropensity(id) => write!(formatter, "invalid treatment propensity for unit: {id}"),
            Self::InvalidOutcome(id) => write!(formatter, "invalid outcome measurement evidence for unit: {id}"),
            Self::InvalidOracle => write!(formatter, "synthetic target-trial potential-outcome oracle is invalid"),
            Self::OracleUnitMismatch(id) => write!(formatter, "oracle unit set differs from target-trial cohort: {id}"),
            Self::OracleObservedOutcomeMismatch(id) => write!(formatter, "observed outcome does not match assigned oracle potential outcome: {id}"),
            Self::Serialization => write!(formatter, "failed to serialize target-trial evidence"),
        }
    }
}

impl std::error::Error for TargetTrialValidationError {}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_serializable<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<String, TargetTrialValidationError> {
    let bytes = serde_json::to_vec(value).map_err(|_| TargetTrialValidationError::Serialization)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn effective_sample_size(weights: &[f64]) -> f64 {
    let sum = weights.iter().sum::<f64>();
    let sum_squares = weights.iter().map(|weight| weight * weight).sum::<f64>();
    if sum_squares <= 0.0 {
        0.0
    } else {
        (sum * sum) / sum_squares
    }
}

fn sorted_min_max(values: &[f64]) -> (Option<f64>, Option<f64>) {
    let minimum = values.iter().copied().reduce(f64::min);
    let maximum = values.iter().copied().reduce(f64::max);
    (minimum, maximum)
}

impl ObservationalTargetTrialProtocol {
    pub fn validate(&self) -> Result<(), TargetTrialValidationError> {
        if self.schema_version != OBSERVATIONAL_TARGET_TRIAL_SCHEMA {
            return Err(TargetTrialValidationError::InvalidSchemaVersion);
        }
        if self.trial_id.trim().is_empty()
            || self.target_population_ref.trim().is_empty()
            || self.treatment_strategy_version_id.trim().is_empty()
            || self.comparator_strategy_version_id.trim().is_empty()
            || self.outcome_metric_id.trim().is_empty()
            || self.outcome_unit.trim().is_empty()
            || self.outcome_measurement_principle_id.trim().is_empty()
        {
            return Err(TargetTrialValidationError::InvalidIdentity);
        }
        if self.treatment_strategy_version_id == self.comparator_strategy_version_id {
            return Err(TargetTrialValidationError::InvalidProtocol(
                "treatment and comparator strategy versions must differ".into(),
            ));
        }
        for digest in [
            &self.eligibility_rule_sha256,
            &self.analysis_plan_sha256,
            &self.causal_graph_sha256,
            &self.propensity_model_sha256,
            &self.propensity_fit_data_sha256,
        ] {
            if !is_sha256_hex(digest) {
                return Err(TargetTrialValidationError::InvalidDigest);
            }
        }
        if self.preregistration_record_ref.trim().is_empty() {
            return Err(TargetTrialValidationError::InvalidIdentity);
        }
        if !self.propensity_floor.is_finite()
            || !self.propensity_ceiling.is_finite()
            || self.propensity_floor <= 0.0
            || self.propensity_ceiling >= 1.0
            || self.propensity_floor >= self.propensity_ceiling
            || !self.time_zero_tolerance_seconds.is_finite()
            || self.time_zero_tolerance_seconds < 0.0
            || !self.followup_horizon_seconds.is_finite()
            || self.followup_horizon_seconds <= 0.0
            || !self.analysis_as_of_seconds.is_finite()
            || self.analysis_as_of_seconds < self.followup_horizon_seconds
        {
            return Err(TargetTrialValidationError::InvalidProtocol(
                "time-zero, follow-up horizon or analysis-as-of is invalid".into(),
            ));
        }
        if self.baseline_covariate_ids.is_empty() || self.strata.is_empty() {
            return Err(TargetTrialValidationError::InvalidProtocol(
                "protocol requires declared baseline covariates and strata".into(),
            ));
        }
        let mut covariates = BTreeSet::new();
        for id in &self.baseline_covariate_ids {
            if id.trim().is_empty() || !covariates.insert(id) {
                return Err(TargetTrialValidationError::InvalidProtocol(
                    "baseline covariate IDs must be non-empty and unique".into(),
                ));
            }
        }
        let mut propensity_covariates = BTreeSet::new();
        for id in &self.propensity_fit_covariate_ids {
            if id.trim().is_empty() || !propensity_covariates.insert(id.as_str()) {
                return Err(TargetTrialValidationError::InvalidProtocol(
                    "propensity-fit covariate IDs must be non-empty and unique".into(),
                ));
            }
        }
        if propensity_covariates != covariates.iter().map(|id| id.as_str()).collect() {
            return Err(TargetTrialValidationError::InvalidProtocol(
                "propensity model must use exactly the declared pre-treatment baseline covariates".into(),
            ));
        }

        let mut strata = BTreeSet::new();
        for stratum in &self.strata {
            if stratum.stratum_id.trim().is_empty()
                || !strata.insert(stratum.stratum_id.as_str())
                || stratum.minimum_repair_units < 2
                || stratum.minimum_comparator_units < 2
                || !stratum.minimum_propensity_overlap.is_finite()
                || !(0.0..=1.0).contains(&stratum.minimum_propensity_overlap)
                || !stratum.minimum_effective_sample_size_per_arm.is_finite()
                || stratum.minimum_effective_sample_size_per_arm < 1.0
            {
                return Err(TargetTrialValidationError::InvalidProtocol(
                    stratum.stratum_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

impl ObservationalTargetTrialDataset {
    pub fn validate(&self) -> Result<(), TargetTrialValidationError> {
        if self.schema_version != OBSERVATIONAL_TARGET_TRIAL_SCHEMA {
            return Err(TargetTrialValidationError::InvalidSchemaVersion);
        }
        self.protocol.validate()?;
        if self.units.is_empty() {
            return Err(TargetTrialValidationError::InvalidProtocol(
                "cohort is empty".into(),
            ));
        }
        let declared_strata = self
            .protocol
            .strata
            .iter()
            .map(|stratum| stratum.stratum_id.as_str())
            .collect::<BTreeSet<_>>();
        let declared_covariates = self
            .protocol
            .baseline_covariate_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut unit_ids = BTreeSet::new();

        for unit in &self.units {
            if unit.unit_id.trim().is_empty()
                || unit.asset_id.trim().is_empty()
                || unit.engine_id.trim().is_empty()
                || unit.campaign_id.trim().is_empty()
                || unit.strategy_version_id.trim().is_empty()
                || unit.strategy_record_ref.trim().is_empty()
                || !is_sha256_hex(&unit.strategy_record_sha256)
            {
                return Err(TargetTrialValidationError::InvalidIdentity);
            }
            if !unit_ids.insert(unit.unit_id.as_str()) {
                return Err(TargetTrialValidationError::DuplicateUnit(
                    unit.unit_id.clone(),
                ));
            }
            if !declared_strata.contains(unit.stratum_id.as_str()) {
                return Err(TargetTrialValidationError::UnknownStratum(
                    unit.stratum_id.clone(),
                ));
            }
            if !unit.eligibility_at_seconds.is_finite()
                || !unit.assignment_at_seconds.is_finite()
                || !unit.followup_start_seconds.is_finite()
                || unit.eligibility_at_seconds < 0.0
                || unit.assignment_at_seconds < 0.0
                || unit.followup_start_seconds < 0.0
            {
                return Err(TargetTrialValidationError::InvalidTimestamp(
                    unit.unit_id.clone(),
                ));
            }
            if !unit.propensity_repair.is_finite()
                || !(0.0..=1.0).contains(&unit.propensity_repair)
            {
                return Err(TargetTrialValidationError::InvalidPropensity(
                    unit.unit_id.clone(),
                ));
            }
            let expected_version = match unit.assigned_arm {
                StrategyArm::RepairStrategy => {
                    self.protocol.treatment_strategy_version_id.as_str()
                }
                StrategyArm::ComparatorStrategy => {
                    self.protocol.comparator_strategy_version_id.as_str()
                }
            };
            let mut unit_covariates = BTreeSet::new();
            for covariate in &unit.baseline_covariates {
                if covariate.covariate_id.trim().is_empty()
                    || !unit_covariates.insert(covariate.covariate_id.as_str())
                    || !declared_covariates.contains(covariate.covariate_id.as_str())
                    || covariate.unit.trim().is_empty()
                    || covariate.source_id.trim().is_empty()
                    || !is_sha256_hex(&covariate.evidence_sha256)
                    || !covariate.value.is_finite()
                    || !covariate.observed_at_seconds.is_finite()
                    || !covariate.available_at_seconds.is_finite()
                    || covariate.observed_at_seconds < 0.0
                    || covariate.available_at_seconds < covariate.observed_at_seconds
                {
                    return Err(TargetTrialValidationError::InvalidCovariate(
                        unit.unit_id.clone(),
                    ));
                }
            }

            if let Some(outcome) = &unit.outcome {
                if outcome.metric_id.trim().is_empty()
                    || outcome.unit.trim().is_empty()
                    || outcome.source_id.trim().is_empty()
                    || outcome.measurement_principle_id.trim().is_empty()
                    || outcome.dependency_domain_id.trim().is_empty()
                    || !is_sha256_hex(&outcome.evidence_sha256)
                    || !is_sha256_hex(&outcome.calibration_record_sha256)
                    || !outcome.value.is_finite()
                    || !outcome.observed_at_seconds.is_finite()
                    || !outcome.available_at_seconds.is_finite()
                    || !outcome.calibration_valid_from_seconds.is_finite()
                    || !outcome.calibration_valid_until_seconds.is_finite()
                    || outcome.calibration_valid_from_seconds < 0.0
                    || outcome.calibration_valid_until_seconds < outcome.calibration_valid_from_seconds
                    || !outcome.uncertainty_abs.is_finite()
                    || outcome.uncertainty_abs < 0.0
                {
                    return Err(TargetTrialValidationError::InvalidOutcome(
                        unit.unit_id.clone(),
                    ));
                }
            }
            if let Some(censoring_at) = unit.censoring_at_seconds {
                if !censoring_at.is_finite() || censoring_at < 0.0 {
                    return Err(TargetTrialValidationError::InvalidTimestamp(
                        unit.unit_id.clone(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, TargetTrialValidationError> {
        let mut canonical = self.clone();
        canonical.units.sort_by(|left, right| left.unit_id.cmp(&right.unit_id));
        for unit in &mut canonical.units {
            unit.baseline_covariates
                .sort_by(|left, right| left.covariate_id.cmp(&right.covariate_id));
        }
        canonical
            .protocol
            .baseline_covariate_ids
            .sort();
        canonical
            .protocol
            .propensity_fit_covariate_ids
            .sort();
        canonical
            .protocol
            .strata
            .sort_by(|left, right| left.stratum_id.cmp(&right.stratum_id));
        sha256_serializable(
            b"symtropy-marine-observational-target-trial-dataset-v1\0",
            &canonical,
        )
    }
}

/// Validate structural eligibility for a future target-trial estimator.
/// A "structurally eligible" result is not an identified effect and contains
/// no causal estimate.
pub fn evaluate_observational_target_trial_design(
    dataset: &ObservationalTargetTrialDataset,
) -> Result<ObservationalTargetTrialReport, TargetTrialValidationError> {
    dataset.validate()?;
    let protocol = &dataset.protocol;
    let dataset_sha256 = dataset.digest()?;
    let mut reasons = BTreeSet::new();

    let mut repair_units = 0usize;
    let mut comparator_units = 0usize;
    let mut complete_followup_units = 0usize;
    let mut censored_or_competing_units = 0usize;
    let mut stratum_rows = Vec::new();

    let expected_covariates = protocol
        .baseline_covariate_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for unit in &dataset.units {
        let expected_version = match unit.assigned_arm {
            StrategyArm::RepairStrategy => protocol.treatment_strategy_version_id.as_str(),
            StrategyArm::ComparatorStrategy => protocol.comparator_strategy_version_id.as_str(),
        };
        if unit.strategy_version_id != expected_version {
            reasons.insert(TargetTrialNotIdentifiedReason::TreatmentVersionMismatch);
        }
        let unit_time_zero = unit.eligibility_at_seconds;
        if (unit.assignment_at_seconds - unit_time_zero).abs()
            > protocol.time_zero_tolerance_seconds
            || (unit.followup_start_seconds - unit_time_zero).abs()
                > protocol.time_zero_tolerance_seconds
        {
            reasons.insert(TargetTrialNotIdentifiedReason::TimeZeroMisaligned);
        }
        if unit.propensity_repair < protocol.propensity_floor
            || unit.propensity_repair > protocol.propensity_ceiling
        {
            reasons.insert(TargetTrialNotIdentifiedReason::PropensityOutsideDeclaredSupport);
        }

        let observed_covariates = unit
            .baseline_covariates
            .iter()
            .map(|covariate| covariate.covariate_id.as_str())
            .collect::<BTreeSet<_>>();
        if observed_covariates != expected_covariates {
            reasons.insert(TargetTrialNotIdentifiedReason::MissingBaselineCovariate);
        }
        for covariate in &unit.baseline_covariates {
            if covariate.temporal_role == CovariateTemporalRole::PostTreatment
                || covariate.observed_at_seconds
                    >= unit_time_zero - TIME_TOLERANCE_SECONDS
                || covariate.available_at_seconds
                    > unit_time_zero + TIME_TOLERANCE_SECONDS
            {
                reasons.insert(TargetTrialNotIdentifiedReason::PostTreatmentCovariateLeakage);
            }
        }

        if !unit.concurrent_intervention_ids.is_empty() {
            reasons.insert(TargetTrialNotIdentifiedReason::ConcurrentInterventionUnresolved);
        }
        match unit.followup_status {
            FollowupStatus::Complete => {
                if unit.censoring_at_seconds.is_some() {
                    reasons.insert(TargetTrialNotIdentifiedReason::FollowupIncomplete);
                    reasons.insert(TargetTrialNotIdentifiedReason::CensoringRequiresModel);
                }
                complete_followup_units += 1;
                let Some(outcome) = &unit.outcome else {
                    reasons.insert(TargetTrialNotIdentifiedReason::OutcomeMissing);
                    continue;
                };
                if outcome.metric_id != protocol.outcome_metric_id
                    || outcome.unit != protocol.outcome_unit
                    || outcome.measurement_principle_id
                        != protocol.outcome_measurement_principle_id
                    || outcome.quality != MeasurementQuality::Valid
                    || outcome.calibration_status != CalibrationStatus::Verified
                {
                    reasons.insert(
                        TargetTrialNotIdentifiedReason::OutcomeMeasurementContractMismatch,
                    );
                }
                if outcome.observed_at_seconds
                    < unit_time_zero + protocol.followup_horizon_seconds
                        - TIME_TOLERANCE_SECONDS
                {
                    reasons.insert(TargetTrialNotIdentifiedReason::OutcomeNotInFollowupWindow);
                }
                if outcome.available_at_seconds < outcome.observed_at_seconds
                    || outcome.available_at_seconds
                        > protocol.analysis_as_of_seconds + TIME_TOLERANCE_SECONDS
                    || outcome.calibration_valid_from_seconds
                        > outcome.observed_at_seconds + TIME_TOLERANCE_SECONDS
                    || outcome.calibration_valid_until_seconds
                        < outcome.available_at_seconds
                {
                    reasons.insert(TargetTrialNotIdentifiedReason::OutcomeEvidenceUnavailable);
                }
            }
            _ => {
                censored_or_competing_units += 1;
                reasons.insert(TargetTrialNotIdentifiedReason::FollowupIncomplete);
                reasons.insert(TargetTrialNotIdentifiedReason::CensoringRequiresModel);
                let censoring_time_valid = unit.censoring_at_seconds.is_some_and(|time| {
                    time >= unit.followup_start_seconds - TIME_TOLERANCE_SECONDS
                        && time <= protocol.analysis_as_of_seconds + TIME_TOLERANCE_SECONDS
                });
                if !censoring_time_valid {
                    reasons.insert(TargetTrialNotIdentifiedReason::CensoringRequiresModel);
                }
            }
        }
    }

    let unit_count = dataset.units.len();
    let mut total_repair = 0usize;
    let mut total_comparator = 0usize;
    for stratum in &protocol.strata {
        let rows = dataset
            .units
            .iter()
            .filter(|unit| unit.stratum_id == stratum.stratum_id)
            .collect::<Vec<_>>();
        let repair = rows
            .iter()
            .filter(|unit| unit.assigned_arm == StrategyArm::RepairStrategy)
            .collect::<Vec<_>>();
        let comparator = rows
            .iter()
            .filter(|unit| unit.assigned_arm == StrategyArm::ComparatorStrategy)
            .collect::<Vec<_>>();
        total_repair += repair.len();
        total_comparator += comparator.len();
        repair_units += repair.len();
        comparator_units += comparator.len();

        let repair_propensities = repair
            .iter()
            .map(|unit| unit.propensity_repair)
            .collect::<Vec<_>>();
        let comparator_propensities = comparator
            .iter()
            .map(|unit| unit.propensity_repair)
            .collect::<Vec<_>>();
        let (repair_min, repair_max) = sorted_min_max(&repair_propensities);
        let (comparator_min, comparator_max) = sorted_min_max(&comparator_propensities);
        let overlap_low = repair_min
            .zip(comparator_min)
            .map(|(left, right)| left.max(right));
        let overlap_high = repair_max
            .zip(comparator_max)
            .map(|(left, right)| left.min(right));
        let overlap_width = overlap_low
            .zip(overlap_high)
            .map(|(low, high)| (high - low).max(0.0))
            .unwrap_or(0.0);

        let repair_weights = repair_propensities
            .iter()
            .map(|propensity| 1.0 / propensity.max(1e-12))
            .collect::<Vec<_>>();
        let comparator_weights = comparator_propensities
            .iter()
            .map(|propensity| 1.0 / (1.0 - propensity).max(1e-12))
            .collect::<Vec<_>>();
        let repair_ess = effective_sample_size(&repair_weights);
        let comparator_ess = effective_sample_size(&comparator_weights);

        if repair.len() < stratum.minimum_repair_units
            || comparator.len() < stratum.minimum_comparator_units
            || overlap_width + NUMERIC_TOLERANCE < stratum.minimum_propensity_overlap
        {
            reasons.insert(TargetTrialNotIdentifiedReason::PositivityFailure);
        }
        if repair_ess + NUMERIC_TOLERANCE < stratum.minimum_effective_sample_size_per_arm
            || comparator_ess + NUMERIC_TOLERANCE
                < stratum.minimum_effective_sample_size_per_arm
        {
            reasons.insert(TargetTrialNotIdentifiedReason::InsufficientEffectiveSampleSize);
        }

        stratum_rows.push(StratumOverlapReport {
            stratum_id: stratum.stratum_id.clone(),
            repair_units: repair.len(),
            comparator_units: comparator.len(),
            repair_propensity_min: repair_min,
            repair_propensity_max: repair_max,
            comparator_propensity_min: comparator_min,
            comparator_propensity_max: comparator_max,
            propensity_overlap_width: overlap_width,
            repair_effective_sample_size: repair_ess,
            comparator_effective_sample_size: comparator_ess,
        });
    }

    if total_repair == 0 || total_comparator == 0 {
        reasons.insert(TargetTrialNotIdentifiedReason::PositivityFailure);
    }
    if protocol.exchangeability_review_ref.trim().is_empty()
        || protocol.treatment_version_review_ref.trim().is_empty()
        || protocol.censoring_review_ref.trim().is_empty()
        || protocol.interference_review_ref.trim().is_empty()
    {
        reasons.insert(TargetTrialNotIdentifiedReason::AssumptionReviewReferenceMissing);
    }

    let not_identified_reasons = reasons.into_iter().collect::<Vec<_>>();
    let disposition = if not_identified_reasons.is_empty() {
        TargetTrialDesignDisposition::StructurallyEligibleUnderDeclaredAssumptions
    } else {
        TargetTrialDesignDisposition::NotIdentified
    };

    Ok(ObservationalTargetTrialReport {
        schema_version: OBSERVATIONAL_TARGET_TRIAL_SCHEMA.to_owned(),
        trial_id: protocol.trial_id.clone(),
        disposition,
        not_identified_reasons,
        unit_count,
        repair_units,
        comparator_units,
        complete_followup_units,
        censored_or_competing_units,
        stratum_reports: stratum_rows,
        analysis_plan_sha256: protocol.analysis_plan_sha256.clone(),
        causal_graph_sha256: protocol.causal_graph_sha256.clone(),
        propensity_model_sha256: protocol.propensity_model_sha256.clone(),
        propensity_fit_data_sha256: protocol.propensity_fit_data_sha256.clone(),
        dataset_sha256,
        causal_effect_estimated: false,
        maintenance_authorized: false,
        physical_restoration_authorized: false,
        operating_restriction_clearance_authorized: false,
        claim_ceiling: "Structural target-trial validation only. This report does not estimate a causal effect or establish exchangeability, positivity beyond declared coarsened strata, correct causal graph specification, censoring exchangeability, source authenticity or transport.".into(),
    })
}

/// Score the naive observational difference in means against a separate oracle.
/// This is an association negative control, not a causal estimator. Its purpose is
/// to demonstrate confounding by indication and to keep the known truth out of
/// the estimator inputs.
pub fn score_unadjusted_observational_association(
    dataset: &ObservationalTargetTrialDataset,
    oracle: &SyntheticPotentialOutcomeOracle,
) -> Result<UnadjustedAssociationScore, TargetTrialValidationError> {
    dataset.validate()?;
    if oracle.schema_version != OBSERVATIONAL_TARGET_TRIAL_ORACLE_SCHEMA
        || oracle.trial_id != dataset.protocol.trial_id
        || !is_sha256_hex(&oracle.oracle_dataset_sha256)
    {
        return Err(TargetTrialValidationError::InvalidOracle);
    }
    let mut potential_by_unit = BTreeMap::new();
    for potential in &oracle.outcomes {
        if potential.unit_id.trim().is_empty()
            || !potential.potential_outcome_repair.is_finite()
            || !potential.potential_outcome_comparator.is_finite()
            || !is_sha256_hex(&potential.oracle_record_sha256)
            || potential_by_unit.insert(potential.unit_id.as_str(), potential).is_some()
        {
            return Err(TargetTrialValidationError::InvalidOracle);
        }
    }
    if potential_by_unit.len() != dataset.units.len() {
        return Err(TargetTrialValidationError::InvalidOracle);
    }

    let mut repair_sum = 0.0;
    let mut comparator_sum = 0.0;
    let mut repair_count = 0usize;
    let mut comparator_count = 0usize;
    let mut true_effect_sum = 0.0;
    for unit in &dataset.units {
        let Some(potential) = potential_by_unit.get(unit.unit_id.as_str()) else {
            return Err(TargetTrialValidationError::OracleUnitMismatch(
                unit.unit_id.clone(),
            ));
        };
        let Some(outcome) = &unit.outcome else {
            return Err(TargetTrialValidationError::InvalidOutcome(
                unit.unit_id.clone(),
            ));
        };
        let observed = match unit.assigned_arm {
            StrategyArm::RepairStrategy => potential.potential_outcome_repair,
            StrategyArm::ComparatorStrategy => potential.potential_outcome_comparator,
        };
        if (observed - outcome.value).abs() > NUMERIC_TOLERANCE {
            return Err(TargetTrialValidationError::OracleObservedOutcomeMismatch(
                unit.unit_id.clone(),
            ));
        }
        match unit.assigned_arm {
            StrategyArm::RepairStrategy => {
                repair_sum += outcome.value;
                repair_count += 1;
            }
            StrategyArm::ComparatorStrategy => {
                comparator_sum += outcome.value;
                comparator_count += 1;
            }
        }
        true_effect_sum +=
            potential.potential_outcome_repair - potential.potential_outcome_comparator;
    }
    if repair_count == 0 || comparator_count == 0 {
        return Err(TargetTrialValidationError::InvalidOracle);
    }
    let observed_difference =
        repair_sum / repair_count as f64 - comparator_sum / comparator_count as f64;
    let true_ate = true_effect_sum / dataset.units.len() as f64;
    let bias = observed_difference - true_ate;
    Ok(UnadjustedAssociationScore {
        trial_id: dataset.protocol.trial_id.clone(),
        observed_repair_minus_comparator: observed_difference,
        true_finite_population_ate: true_ate,
        association_bias: bias,
        sign_reversal: observed_difference * true_ate < 0.0,
        oracle_used_for_association: false,
        causal_effect_estimated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn digest_from_u64(value: u64) -> String {
        format!("{value:064x}")
    }

    fn fixture() -> (
        ObservationalTargetTrialDataset,
        SyntheticPotentialOutcomeOracle,
    ) {
        let protocol = ObservationalTargetTrialProtocol {
            schema_version: OBSERVATIONAL_TARGET_TRIAL_SCHEMA.into(),
            trial_id: "synthetic-confounding-by-indication-v1".into(),
            assignment_method: ObservationalAssignmentMethod::ObservationalTargetTrial,
            target_population_ref: "synthetic-eligible-engine-population".into(),
            eligibility_rule_sha256: digest('a'),
            analysis_plan_sha256: digest('b'),
            preregistration_record_ref: "synthetic-preregistration://trial-001".into(),
            treatment_strategy_version_id: "repair-v2-complete".into(),
            comparator_strategy_version_id: "deferred-maintenance-v1".into(),
            outcome_metric_id: "normalized_propulsion_capability".into(),
            outcome_unit: "normalized_capability".into(),
            outcome_measurement_principle_id: "calibrated_shaft_torque".into(),
            causal_graph_sha256: digest('c'),
            propensity_model_sha256: digest('d'),
            propensity_fit_data_sha256: digest('e'),
            propensity_fit_covariate_ids: vec![
                "baseline_severity".into(),
                "load_band_score".into(),
            ],
            propensity_floor: 0.05,
            propensity_ceiling: 0.95,
            time_zero_tolerance_seconds: 0.01,
            followup_horizon_seconds: 100.0,
            analysis_as_of_seconds: 300.0,
            baseline_covariate_ids: vec!["baseline_severity".into(), "load_band_score".into()],
            strata: vec![
                TargetTrialStratum {
                    stratum_id: "low_load".into(),
                    minimum_repair_units: 2,
                    minimum_comparator_units: 2,
                    minimum_propensity_overlap: 0.01,
                    minimum_effective_sample_size_per_arm: 1.9,
                },
                TargetTrialStratum {
                    stratum_id: "high_load".into(),
                    minimum_repair_units: 2,
                    minimum_comparator_units: 2,
                    minimum_propensity_overlap: 0.01,
                    minimum_effective_sample_size_per_arm: 1.9,
                },
            ],
            exchangeability_review_ref: "review://exchangeability-001".into(),
            treatment_version_review_ref: "review://treatment-versions-001".into(),
            censoring_review_ref: "review://censoring-001".into(),
            interference_review_ref: "review://interference-001".into(),
        };
        let mut units = Vec::new();
        let mut oracle = Vec::new();

        for (stratum_index, stratum_id) in ["low_load", "high_load"].iter().enumerate() {
            // Within each coarse stratum, severity still influences treatment
            // choice and outcomes. This leaves residual confounding by indication
            // even though both arms are represented and coarse propensity ranges overlap.
            for unit_index in 0..4 {
                let id = format!("asset-{stratum_index}-{unit_index}");
                let arm = if unit_index < 2 {
                    StrategyArm::RepairStrategy
                } else {
                    StrategyArm::ComparatorStrategy
                };
                let severity = match (stratum_index, unit_index) {
                    (0, 0) => 95.0,
                    (0, 1) => 85.0,
                    (0, 2) => 60.0,
                    (0, 3) => 55.0,
                    (1, 0) => 120.0,
                    (1, 1) => 110.0,
                    (1, 2) => 80.0,
                    _ => 75.0,
                };
                let base_outcome = 110.0 - severity * 0.5;
                let repair_effect = 5.0;
                let potential_repair = base_outcome + repair_effect;
                let potential_comparator = base_outcome;
                let observed = match arm {
                    StrategyArm::RepairStrategy => potential_repair,
                    StrategyArm::ComparatorStrategy => potential_comparator,
                };
                let propensity = match unit_index {
                    0 => 0.55,
                    1 => 0.52,
                    2 => 0.54,
                    _ => 0.50,
                };
                let suffix = (stratum_index * 4 + unit_index) as u64;
                units.push(ObservationalTargetTrialUnit {
                    unit_id: id.clone(),
                    asset_id: id.clone(),
                    engine_id: format!("engine-{stratum_index}"),
                    campaign_id: format!("campaign-{stratum_index}"),
                    stratum_id: stratum_id.to_string(),
                    assigned_arm: arm,
                    strategy_version_id: match arm {
                        StrategyArm::RepairStrategy => "repair-v2-complete".into(),
                        StrategyArm::ComparatorStrategy => "deferred-maintenance-v1".into(),
                    },
                    strategy_record_ref: format!("work-order-record://{id}"),
                    strategy_record_sha256: digest_from_u64(60_000 + suffix),
                    eligibility_at_seconds: 100.0,
                    assignment_at_seconds: 100.0,
                    followup_start_seconds: 100.0,
                    propensity_repair: propensity,
                    propensity_evidence_sha256: digest_from_u64(5_000 + suffix),
                    propensity_available_at_seconds: 95.0,
                    baseline_covariates: vec![
                        BaselineCovariateMeasurement {
                            covariate_id: "baseline_severity".into(),
                            temporal_role: CovariateTemporalRole::Baseline,
                            value: severity,
                            unit: "normalized_severity".into(),
                            observed_at_seconds: 90.0,
                            available_at_seconds: 95.0,
                            source_id: format!("severity-source-{id}"),
                            evidence_sha256: digest_from_u64(10_000 + suffix),
                        },
                        BaselineCovariateMeasurement {
                            covariate_id: "load_band_score".into(),
                            temporal_role: CovariateTemporalRole::Baseline,
                            value: 0.25 + stratum_index as f64,
                            unit: "normalized_load".into(),
                            observed_at_seconds: 90.0,
                            available_at_seconds: 95.0,
                            source_id: format!("load-source-{id}"),
                            evidence_sha256: digest_from_u64(20_000 + suffix),
                        },
                    ],
                    followup_status: FollowupStatus::Complete,
                    censoring_at_seconds: None,
                    concurrent_intervention_ids: Vec::new(),
                    outcome: Some(TargetTrialOutcome {
                        metric_id: "normalized_propulsion_capability".into(),
                        value: observed,
                        unit: "normalized_capability".into(),
                        observed_at_seconds: 210.0,
                        available_at_seconds: 220.0,
                        source_id: format!("outcome-source-{id}"),
                        evidence_sha256: digest_from_u64(30_000 + suffix),
                        measurement_principle_id: "calibrated_shaft_torque".into(),
                        dependency_domain_id: format!("outcome-daq-{id}"),
                        quality: MeasurementQuality::Valid,
                        calibration_status: CalibrationStatus::Verified,
                        calibration_record_sha256: digest_from_u64(40_000 + suffix),
                        calibration_valid_from_seconds: 0.0,
                        calibration_valid_until_seconds: 1_000.0,
                        uncertainty_abs: 0.25,
                    }),
                });
                oracle.push(UnitPotentialOutcomes {
                    unit_id: id,
                    potential_outcome_repair: potential_repair,
                    potential_outcome_comparator: potential_comparator,
                    oracle_record_sha256: digest_from_u64(50_000 + suffix),
                });
            }
        }

        (
            ObservationalTargetTrialDataset {
                schema_version: OBSERVATIONAL_TARGET_TRIAL_SCHEMA.into(),
                protocol,
                units,
            },
            SyntheticPotentialOutcomeOracle {
                schema_version: OBSERVATIONAL_TARGET_TRIAL_ORACLE_SCHEMA.into(),
                trial_id: "synthetic-confounding-by-indication-v1".into(),
                oracle_dataset_sha256: digest('f'),
                outcomes: oracle,
            },
        )
    }

    #[test]
    fn coarse_target_trial_design_can_pass_without_becoming_a_causal_estimate() {
        let (dataset, _) = fixture();
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert_eq!(
            report.disposition,
            TargetTrialDesignDisposition::StructurallyEligibleUnderDeclaredAssumptions
        );
        assert_eq!(report.unit_count, 8);
        assert_eq!(report.repair_units, 4);
        assert_eq!(report.comparator_units, 4);
        assert!(!report.causal_effect_estimated);
        assert!(!report.maintenance_authorized);
        assert_eq!(report.not_identified_reasons.len(), 0);
    }

    #[test]
    fn each_asset_can_have_its_own_aligned_time_zero() {
        let (mut dataset, _) = fixture();
        for unit in dataset.units.iter_mut().filter(|unit| unit.stratum_id == "high_load") {
            unit.eligibility_at_seconds += 10.0;
            unit.assignment_at_seconds += 10.0;
            unit.followup_start_seconds += 10.0;
            for covariate in &mut unit.baseline_covariates {
                covariate.observed_at_seconds += 10.0;
                covariate.available_at_seconds += 10.0;
            }
            if let Some(outcome) = &mut unit.outcome {
                outcome.observed_at_seconds += 10.0;
                outcome.available_at_seconds += 10.0;
            }
        }
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert_eq!(
            report.disposition,
            TargetTrialDesignDisposition::StructurallyEligibleUnderDeclaredAssumptions
        );
    }

    #[test]
    fn propensity_model_using_post_treatment_covariates_is_rejected() {
        let (mut dataset, _) = fixture();
        dataset.protocol.propensity_fit_covariate_ids.push("post_repair_temperature".into());
        assert_eq!(
            dataset.protocol.validate(),
            Err(TargetTrialValidationError::InvalidProtocol(
                "propensity model must use exactly the declared pre-treatment baseline covariates".into()
            ))
        );
    }

    #[test]
    fn propensity_outside_preregistered_support_is_not_identified() {
        let (mut dataset, _) = fixture();
        dataset.units[0].propensity_repair = 0.99;
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::PropensityOutsideDeclaredSupport
        ));
    }

    #[test]
    fn unadjusted_association_can_reverse_the_sign_of_the_known_repair_effect() {
        let (dataset, oracle) = fixture();
        let score = score_unadjusted_observational_association(&dataset, &oracle).unwrap();
        assert!(score.true_finite_population_ate > 0.0);
        assert!(score.observed_repair_minus_comparator < 0.0);
        assert!(score.sign_reversal);
        assert!(!score.oracle_used_for_association);
        assert!(!score.causal_effect_estimated);
    }

    #[test]
    fn misaligned_time_zero_is_not_identified() {
        let (mut dataset, _) = fixture();
        dataset.units[0].followup_start_seconds = 105.0;
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert_eq!(report.disposition, TargetTrialDesignDisposition::NotIdentified);
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::TimeZeroMisaligned
        ));
    }

    #[test]
    fn missing_baseline_covariate_is_not_identified() {
        let (mut dataset, _) = fixture();
        dataset.units[0].baseline_covariates.pop();
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::MissingBaselineCovariate
        ));
    }

    #[test]
    fn post_treatment_covariate_leakage_is_not_identified() {
        let (mut dataset, _) = fixture();
        dataset.units[0].baseline_covariates[0].temporal_role =
            CovariateTemporalRole::PostTreatment;
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::PostTreatmentCovariateLeakage
        ));
    }

    #[test]
    fn missing_arm_in_a_stratum_is_a_positivity_failure() {
        let (mut dataset, _) = fixture();
        for unit in &mut dataset.units {
            if unit.stratum_id == "high_load" {
                unit.assigned_arm = StrategyArm::RepairStrategy;
                unit.strategy_version_id = "repair-v2-complete".into();
            }
        }
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::PositivityFailure
        ));
    }

    #[test]
    fn incomplete_followup_is_not_silently_treated_as_non_failure() {
        let (mut dataset, _) = fixture();
        dataset.units[0].followup_status = FollowupStatus::PreventiveReplacement;
        dataset.units[0].censoring_at_seconds = Some(180.0);
        dataset.units[0].outcome = None;
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::FollowupIncomplete
        ));
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::CensoringRequiresModel
        ));
    }

    #[test]
    fn missing_strategy_record_digest_is_rejected() {
        let (mut dataset, _) = fixture();
        dataset.units[0].strategy_record_sha256.clear();
        assert_eq!(
            dataset.validate(),
            Err(TargetTrialValidationError::InvalidIdentity)
        );
    }

    #[test]
    fn calibration_expiring_before_outcome_availability_is_not_identified() {
        let (mut dataset, _) = fixture();
        dataset.units[0]
            .outcome
            .as_mut()
            .unwrap()
            .calibration_valid_until_seconds = 219.0;
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::OutcomeEvidenceUnavailable
        ));
    }

    #[test]
    fn treatment_version_mismatch_returns_not_identified_reason() {
        let (mut dataset, _) = fixture();
        dataset.units[0].strategy_version_id = "temporary-repair-v0".into();
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::TreatmentVersionMismatch
        ));
    }

    #[test]
    fn missing_assumption_review_reference_returns_not_identified_reason() {
        let (mut dataset, _) = fixture();
        dataset.protocol.exchangeability_review_ref.clear();
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::AssumptionReviewReferenceMissing
        ));
    }

    #[test]
    fn complete_followup_cannot_also_claim_a_censoring_time() {
        let (mut dataset, _) = fixture();
        dataset.units[0].censoring_at_seconds = Some(180.0);
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::FollowupIncomplete
        ));
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::CensoringRequiresModel
        ));
    }

    #[test]
    fn concurrent_intervention_blocks_single_repair_attribution() {
        let (mut dataset, _) = fixture();
        dataset.units[0].concurrent_intervention_ids.push("controller-tune".into());
        let report = evaluate_observational_target_trial_design(&dataset).unwrap();
        assert!(report.not_identified_reasons.contains(
            &TargetTrialNotIdentifiedReason::ConcurrentInterventionUnresolved
        ));
    }

    #[test]
    fn target_trial_manifest_keeps_association_and_identification_distinct() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../observational-target-trial-v0.1.json")).unwrap();
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../observational-target-trial-cases-v0.1.json"))
                .unwrap();
        assert_eq!(protocol["schema_version"], OBSERVATIONAL_TARGET_TRIAL_SCHEMA);
        assert_eq!(protocol["status"], "protocol_only_not_executed");
        assert_eq!(protocol["scope"]["causal_estimates_enabled"], false);
        assert!(protocol["hard_gates"].as_array().unwrap().len() >= 28);
        assert_eq!(protocol["cases"].as_array().unwrap().len(), 15);
        assert_eq!(corpus["cases"].as_array().unwrap().len(), 15);
    }
}
