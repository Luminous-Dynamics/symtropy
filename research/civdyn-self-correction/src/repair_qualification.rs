// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Paired fresh-challenge qualification of model repairs.
//!
//! The original and repaired model must be evaluated on the same challenge
//! observations. Diagnostic/repair data must be disjoint from challenge
//! sources and observations. Interval-width budgets prevent uncertainty
//! inflation from masquerading as restored adequacy.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::model_adequacy::{
    AdequacyAssessment, AdequacyError, DomainStatus, IndependentUnitKind, PredictiveKind,
    SplitManifest, SplitRole, StatisticRegistry, StatisticStatus,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RepairEvidence {
    pub domain_id: String,
    /// Exact model identity expected in all assessed draw rows.
    pub model_digest: String,
    pub registry: StatisticRegistry,
    pub manifest: SplitManifest,
    pub assessment: AdequacyAssessment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairRowOutcome {
    Maintained,
    CorrectedOnFreshChallenge,
    ResidualDiscrepancy,
    Regression,
    WidthBudgetExceeded,
    NotAssessed,
}

impl RepairRowOutcome {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Maintained => "maintained",
            Self::CorrectedOnFreshChallenge => "corrected_on_fresh_challenge",
            Self::ResidualDiscrepancy => "residual_discrepancy",
            Self::Regression => "regression",
            Self::WidthBudgetExceeded => "width_budget_exceeded",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairClaimCeiling {
    RepairRestoredAdequacyOnFreshChallenge,
    NoFreshChallengeImprovementDemonstrated,
    ResidualDiscrepancy,
    RegressionDetected,
    WidthBudgetExceeded,
    NotAssessed,
}

impl RepairClaimCeiling {
    pub const fn name(self) -> &'static str {
        match self {
            Self::RepairRestoredAdequacyOnFreshChallenge => {
                "repair_restored_adequacy_on_fresh_challenge"
            }
            Self::NoFreshChallengeImprovementDemonstrated => {
                "no_fresh_challenge_improvement_demonstrated"
            }
            Self::ResidualDiscrepancy => "residual_discrepancy",
            Self::RegressionDetected => "regression_detected",
            Self::WidthBudgetExceeded => "width_budget_exceeded",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepairUnitResult {
    pub statistic_id: String,
    pub independent_unit_id: String,
    pub primary: bool,
    pub required: bool,
    pub baseline_status: StatisticStatus,
    pub repaired_status: StatisticStatus,
    pub baseline_interval_width: Option<f64>,
    pub repaired_interval_width: Option<f64>,
    pub interval_width_ratio: Option<f64>,
    pub outcome: RepairRowOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepairQualificationReport {
    pub initial_model_digest: String,
    pub repaired_model_digest: String,
    pub initial_calibration_data_digest: String,
    pub repair_training_data_digest: String,
    pub diagnostic_split_manifest_digest: String,
    pub baseline_challenge_manifest_digest: String,
    pub repaired_challenge_manifest_digest: String,
    pub repair_action_digest: String,
    pub protocol_digest: String,
    pub central_mass: f64,
    pub maximum_interval_width_ratio: f64,
    pub diagnostic_primary_discrepancies: usize,
    pub corrected_primary_statistics: usize,
    pub residual_primary_discrepancies: usize,
    pub regressed_primary_statistics: usize,
    pub width_budget_failures: usize,
    pub required_evidence_complete: bool,
    pub claim_ceiling: RepairClaimCeiling,
    pub results: Vec<RepairUnitResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatisticSignature {
    id: String,
    construct: String,
    measurement_unit: String,
    primary: bool,
    required: bool,
    minimum_draws: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct ValidatedEvidence {
    rows: BTreeMap<(String, String), crate::model_adequacy::StatisticAssessment>,
    required_evidence_complete: bool,
    primary_discrepancies: usize,
    central_mass: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairQualificationError {
    Adequacy(AdequacyError),
    InvalidDomainId,
    InvalidModelIdentity,
    InvalidRepairIdentity,
    SameInitialAndRepairedModel,
    InvalidWidthBudget,
    InvalidCentralMass,
    WrongAssessmentRole { domain_id: String },
    PriorPredictiveEvidence { domain_id: String },
    RegistryVersionMismatch { domain_id: String },
    StatisticSchemaMismatch { domain_id: String, statistic_id: String },
    AssessmentManifestMismatch { domain_id: String },
    AssessmentRegistryMismatch { domain_id: String },
    AssessmentRoleMismatch { domain_id: String },
    ModelIdentityMismatch { domain_id: String },
    CalibrationLineageMismatch,
    RepairTrainingDataMismatch,
    ChallengeUnitSetMismatch,
    ChallengeIndependentUnitMismatch,
    ChallengeCentralMassMismatch,
    ChallengeObservationMismatch { statistic_id: String, unit_id: String },
    DiagnosticChallengeSourceOverlap { source_digest: String },
    ChallengeRepairDataLeakage { digest: String },
    DiagnosticEvidenceNotRepairable,
    DuplicateAssessmentRow { statistic_id: String, unit_id: String },
    UnknownAssessmentStatistic { statistic_id: String },
    UnknownAssessmentUnit { statistic_id: String, unit_id: String },
    AssessmentStatisticIdentityMismatch { statistic_id: String, unit_id: String },
    RequiredEvidenceCompletenessMismatch { domain_id: String },
    DomainStatusMismatch { domain_id: String },
    MissingRequiredEvidence,
}

impl fmt::Display for RepairQualificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RepairQualificationError {}

impl From<AdequacyError> for RepairQualificationError {
    fn from(value: AdequacyError) -> Self {
        Self::Adequacy(value)
    }
}

fn semantic_schema(registry: &StatisticRegistry) -> Vec<StatisticSignature> {
    let mut schema: Vec<_> = registry
        .statistics
        .iter()
        .map(|spec| StatisticSignature {
            id: spec.id.clone(),
            construct: spec.construct.clone(),
            measurement_unit: spec.measurement_unit.clone(),
            primary: spec.primary,
            required: spec.required,
            minimum_draws: spec.minimum_draws,
        })
        .collect();
    schema.sort_by(|left, right| left.id.cmp(&right.id));
    schema
}

fn units_for(manifest: &SplitManifest, role: SplitRole) -> &[String] {
    match role {
        SplitRole::Calibration => &manifest.calibration_units,
        SplitRole::Validation => &manifest.validation_units,
        SplitRole::Challenge => &manifest.challenge_units,
    }
}

fn validate_evidence(
    evidence: &RepairEvidence,
    expected_role: SplitRole,
    expected_model_digest: &str,
    expected_registry_version: &str,
    expected_schema: &[StatisticSignature],
    expected_central_mass: Option<f64>,
) -> Result<ValidatedEvidence, RepairQualificationError> {
    evidence.registry.validate()?;
    evidence.manifest.validate()?;

    if evidence.domain_id.trim().is_empty() {
        return Err(RepairQualificationError::InvalidDomainId);
    }
    if evidence.model_digest.trim().is_empty() {
        return Err(RepairQualificationError::InvalidModelIdentity);
    }
    if evidence.assessment.role != expected_role {
        return Err(RepairQualificationError::WrongAssessmentRole {
            domain_id: evidence.domain_id.clone(),
        });
    }
    if evidence.assessment.predictive_kind != PredictiveKind::Split {
        return Err(RepairQualificationError::PriorPredictiveEvidence {
            domain_id: evidence.domain_id.clone(),
        });
    }
    if !evidence.assessment.central_mass.is_finite()
        || evidence.assessment.central_mass <= 0.0
        || evidence.assessment.central_mass >= 1.0
    {
        return Err(RepairQualificationError::InvalidCentralMass);
    }
    if expected_central_mass.is_some_and(|value| value != evidence.assessment.central_mass) {
        return Err(RepairQualificationError::ChallengeCentralMassMismatch);
    }
    if evidence.registry.version != expected_registry_version {
        return Err(RepairQualificationError::RegistryVersionMismatch {
            domain_id: evidence.domain_id.clone(),
        });
    }
    if semantic_schema(&evidence.registry) != expected_schema {
        let statistic_id = evidence
            .registry
            .statistics
            .first()
            .map(|spec| spec.id.clone())
            .unwrap_or_default();
        return Err(RepairQualificationError::StatisticSchemaMismatch {
            domain_id: evidence.domain_id.clone(),
            statistic_id,
        });
    }
    if evidence.assessment.registry_version != evidence.registry.version {
        return Err(RepairQualificationError::AssessmentRegistryMismatch {
            domain_id: evidence.domain_id.clone(),
        });
    }
    if evidence.assessment.split_manifest_id != evidence.manifest.id
        || evidence.assessment.split_manifest_digest != evidence.manifest.manifest_digest
    {
        return Err(RepairQualificationError::AssessmentManifestMismatch {
            domain_id: evidence.domain_id.clone(),
        });
    }
    if evidence.assessment.role
        != evidence
            .registry
            .statistics
            .first()
            .map(|spec| spec.role)
            .unwrap_or(SplitRole::Calibration)
    {
        return Err(RepairQualificationError::AssessmentRoleMismatch {
            domain_id: evidence.domain_id.clone(),
        });
    }
    if evidence
        .registry
        .statistics
        .iter()
        .any(|spec| spec.role != expected_role || spec.independent_unit != evidence.manifest.independent_unit)
    {
        let spec = evidence
            .registry
            .statistics
            .iter()
            .find(|spec| {
                spec.role != expected_role || spec.independent_unit != evidence.manifest.independent_unit
            })
            .expect("predicate found a mismatched statistic");
        return Err(RepairQualificationError::AssessmentStatisticIdentityMismatch {
            statistic_id: spec.id.clone(),
            unit_id: String::new(),
        });
    }

    let units = units_for(&evidence.manifest, expected_role);
    if units.is_empty() {
        return Err(RepairQualificationError::MissingRequiredEvidence);
    }
    let unit_set: BTreeSet<&str> = units.iter().map(String::as_str).collect();
    let mut rows = BTreeMap::new();

    for row in &evidence.assessment.statistics {
        let spec = evidence.registry.get(&row.statistic_id).ok_or_else(|| {
            RepairQualificationError::UnknownAssessmentStatistic {
                statistic_id: row.statistic_id.clone(),
            }
        })?;
        if !unit_set.contains(row.independent_unit_id.as_str()) {
            return Err(RepairQualificationError::UnknownAssessmentUnit {
                statistic_id: row.statistic_id.clone(),
                unit_id: row.independent_unit_id.clone(),
            });
        }
        if row.source_family != spec.source_family
            || row.source_digest != spec.source_digest
            || row.measurement_unit != spec.measurement_unit
        {
            return Err(RepairQualificationError::AssessmentStatisticIdentityMismatch {
                statistic_id: row.statistic_id.clone(),
                unit_id: row.independent_unit_id.clone(),
            });
        }
        if row
            .model_digest
            .as_deref()
            .is_some_and(|digest| digest != expected_model_digest)
        {
            return Err(RepairQualificationError::ModelIdentityMismatch {
                domain_id: evidence.domain_id.clone(),
            });
        }

        if row.status != StatisticStatus::NotAssessed {
            let valid = row.model_digest.as_deref() == Some(expected_model_digest)
                && row.draw_set_digest.as_deref().is_some_and(|value| !value.trim().is_empty())
                && row.observation_digest.as_deref().is_some_and(|value| !value.trim().is_empty())
                && row.sample_size > 0
                && row.observed_value.is_some_and(f64::is_finite)
                && row.lower_reference.is_some_and(f64::is_finite)
                && row.upper_reference.is_some_and(f64::is_finite)
                && row.lower_reference <= row.upper_reference
                && row.predictive_rank.is_some_and(|rank| {
                    rank.is_finite() && (0.0..=1.0).contains(&rank)
                })
                && row.draw_count >= spec.minimum_draws
                && row.measurement_standard_error.is_none_or(|value| {
                    value.is_finite() && value >= 0.0
                })
                && match (
                    row.status,
                    row.observed_value,
                    row.lower_reference,
                    row.upper_reference,
                ) {
                    (
                        StatisticStatus::WithinReferenceInterval,
                        Some(observed),
                        Some(lower),
                        Some(upper),
                    ) => observed >= lower && observed <= upper,
                    (
                        StatisticStatus::OutsideReferenceInterval,
                        Some(observed),
                        Some(lower),
                        Some(upper),
                    ) => observed < lower || observed > upper,
                    _ => false,
                };
            if !valid {
                return Err(RepairQualificationError::AssessmentStatisticIdentityMismatch {
                    statistic_id: row.statistic_id.clone(),
                    unit_id: row.independent_unit_id.clone(),
                });
            }
        }

        let key = (row.statistic_id.clone(), row.independent_unit_id.clone());
        if rows.insert(key, row.clone()).is_some() {
            return Err(RepairQualificationError::DuplicateAssessmentRow {
                statistic_id: row.statistic_id.clone(),
                unit_id: row.independent_unit_id.clone(),
            });
        }
    }

    let required_specs: Vec<_> = evidence
        .registry
        .statistics
        .iter()
        .filter(|spec| spec.required && spec.role == expected_role)
        .collect();
    let mut required_complete = !required_specs.is_empty();
    for spec in &required_specs {
        for unit_id in units {
            let assessed = rows
                .get(&(spec.id.clone(), unit_id.clone()))
                .is_some_and(|row| row.status != StatisticStatus::NotAssessed);
            required_complete &= assessed;
        }
    }
    if evidence.assessment.required_evidence_complete != required_complete {
        return Err(RepairQualificationError::RequiredEvidenceCompletenessMismatch {
            domain_id: evidence.domain_id.clone(),
        });
    }

    let primary_specs: BTreeSet<&str> = evidence
        .registry
        .statistics
        .iter()
        .filter(|spec| spec.primary)
        .map(|spec| spec.id.as_str())
        .collect();
    let primary_rows: Vec<_> = rows
        .iter()
        .filter(|((statistic_id, _), _)| primary_specs.contains(statistic_id.as_str()))
        .map(|(_, row)| row)
        .collect();
    let primary_discrepancies = primary_rows
        .iter()
        .filter(|row| row.status == StatisticStatus::OutsideReferenceInterval)
        .count();
    let primary_fully_assessed = !primary_specs.is_empty()
        && primary_rows.len() == primary_specs.len() * units.len()
        && primary_rows
            .iter()
            .all(|row| row.status != StatisticStatus::NotAssessed);

    let derived_domain_status = if primary_discrepancies > 0 {
        DomainStatus::PrimaryDiscrepancyDetected
    } else if !required_complete || !primary_fully_assessed {
        DomainStatus::NotAssessed
    } else {
        DomainStatus::NoPrimaryDiscrepancyDetected
    };
    if evidence.assessment.domain_status != derived_domain_status {
        return Err(RepairQualificationError::DomainStatusMismatch {
            domain_id: evidence.domain_id.clone(),
        });
    }

    Ok(ValidatedEvidence {
        rows,
        required_evidence_complete: required_complete,
        primary_discrepancies,
        central_mass: evidence.assessment.central_mass,
    })
}

fn interval_width(
    row: &crate::model_adequacy::StatisticAssessment,
) -> Option<f64> {
    Some(row.upper_reference? - row.lower_reference?)
}

/// Qualify a repair using a diagnostic split and paired baseline/repaired
/// evaluations on exactly the same fresh challenge observations.
pub fn qualify_repair(
    diagnostic: &RepairEvidence,
    baseline_challenge: &RepairEvidence,
    repaired_challenge: &RepairEvidence,
    initial_calibration_data_digest: &str,
    repair_training_data_digest: &str,
    repair_training_source_digests: &[String],
    repair_training_observation_digests: &[String],
    repair_action_digest: &str,
    protocol_digest: &str,
    maximum_interval_width_ratio: f64,
) -> Result<RepairQualificationReport, RepairQualificationError> {
    if initial_calibration_data_digest.trim().is_empty()
        || repair_training_data_digest.trim().is_empty()
        || repair_action_digest.trim().is_empty()
        || protocol_digest.trim().is_empty()
    {
        return Err(RepairQualificationError::InvalidRepairIdentity);
    }
    if !maximum_interval_width_ratio.is_finite() || maximum_interval_width_ratio < 1.0 {
        return Err(RepairQualificationError::InvalidWidthBudget);
    }
    if diagnostic.model_digest.trim().is_empty()
        || baseline_challenge.model_digest.trim().is_empty()
        || repaired_challenge.model_digest.trim().is_empty()
    {
        return Err(RepairQualificationError::InvalidModelIdentity);
    }
    if baseline_challenge.model_digest != diagnostic.model_digest {
        return Err(RepairQualificationError::ModelIdentityMismatch {
            domain_id: baseline_challenge.domain_id.clone(),
        });
    }
    if repaired_challenge.model_digest == diagnostic.model_digest {
        return Err(RepairQualificationError::SameInitialAndRepairedModel);
    }
    if diagnostic.manifest.calibration_data_digest != initial_calibration_data_digest
        || baseline_challenge.manifest.calibration_data_digest != initial_calibration_data_digest
    {
        return Err(RepairQualificationError::CalibrationLineageMismatch);
    }
    if repaired_challenge.manifest.calibration_data_digest != repair_training_data_digest {
        return Err(RepairQualificationError::RepairTrainingDataMismatch);
    }

    let expected_schema = semantic_schema(&diagnostic.registry);
    let registry_version = diagnostic.registry.version.as_str();
    let diagnostic_result = validate_evidence(
        diagnostic,
        SplitRole::Validation,
        &diagnostic.model_digest,
        registry_version,
        &expected_schema,
        None,
    )?;
    if diagnostic_result.central_mass != baseline_challenge.assessment.central_mass
        || diagnostic_result.central_mass != repaired_challenge.assessment.central_mass
    {
        return Err(RepairQualificationError::ChallengeCentralMassMismatch);
    }
    if diagnostic_result.primary_discrepancies == 0
        || !diagnostic_result.required_evidence_complete
    {
        return Err(RepairQualificationError::DiagnosticEvidenceNotRepairable);
    }

    let baseline_result = validate_evidence(
        baseline_challenge,
        SplitRole::Challenge,
        &diagnostic.model_digest,
        registry_version,
        &expected_schema,
        Some(diagnostic_result.central_mass),
    )?;
    let repaired_result = validate_evidence(
        repaired_challenge,
        SplitRole::Challenge,
        &repaired_challenge.model_digest,
        registry_version,
        &expected_schema,
        Some(diagnostic_result.central_mass),
    )?;

    if baseline_challenge.manifest.independent_unit != repaired_challenge.manifest.independent_unit {
        return Err(RepairQualificationError::ChallengeIndependentUnitMismatch);
    }
    if baseline_challenge.manifest.challenge_units != repaired_challenge.manifest.challenge_units {
        return Err(RepairQualificationError::ChallengeUnitSetMismatch);
    }
    if baseline_challenge.manifest.manifest_digest == repaired_challenge.manifest.manifest_digest
        || baseline_challenge.manifest.id == repaired_challenge.manifest.id
    {
        return Err(RepairQualificationError::ChallengeUnitSetMismatch);
    }

    let diagnostic_sources: BTreeSet<&str> = diagnostic
        .registry
        .statistics
        .iter()
        .filter(|spec| spec.required)
        .map(|spec| spec.source_digest.as_str())
        .collect();
    let training_sources: BTreeSet<&str> = repair_training_source_digests
        .iter()
        .map(String::as_str)
        .collect();
    let training_observations: BTreeSet<&str> = repair_training_observation_digests
        .iter()
        .map(String::as_str)
        .collect();
    let challenge_sources: BTreeSet<&str> = baseline_challenge
        .registry
        .statistics
        .iter()
        .filter(|spec| spec.required)
        .map(|spec| spec.source_digest.as_str())
        .collect();
    let challenge_observations: BTreeSet<&str> = baseline_result
        .rows
        .values()
        .filter_map(|row| row.observation_digest.as_deref())
        .collect();

    if let Some(source_digest) = diagnostic_sources
        .iter()
        .find(|digest| !training_sources.contains(**digest))
    {
        return Err(RepairQualificationError::DiagnosticChallengeSourceOverlap {
            source_digest: (*source_digest).to_string(),
        });
    }
    if let Some(digest) = challenge_sources
        .intersection(&training_sources)
        .next()
    {
        return Err(RepairQualificationError::ChallengeRepairDataLeakage {
            digest: (*digest).to_string(),
        });
    }
    if let Some(digest) = challenge_observations
        .intersection(&training_observations)
        .next()
    {
        return Err(RepairQualificationError::ChallengeRepairDataLeakage {
            digest: (*digest).to_string(),
        });
    }

    let mut results = Vec::new();
    let mut corrected_primary_statistics = 0;
    let mut residual_primary_discrepancies = 0;
    let mut regressed_primary_statistics = 0;
    let mut width_budget_failures = 0;
    let mut required_evidence_complete = baseline_result.required_evidence_complete
        && repaired_result.required_evidence_complete;

    for spec in &baseline_challenge.registry.statistics {
        let units = if spec.required {
            baseline_challenge.manifest.challenge_units.as_slice()
        } else {
            &[]
        };
        for unit_id in units {
            let key = (spec.id.clone(), unit_id.clone());
            let baseline_row = baseline_result.rows.get(&key);
            let repaired_row = repaired_result.rows.get(&key);
            let Some((baseline_row, repaired_row)) = baseline_row.zip(repaired_row) else {
                required_evidence_complete = false;
                results.push(RepairUnitResult {
                    statistic_id: spec.id.clone(),
                    independent_unit_id: unit_id.clone(),
                    primary: spec.primary,
                    required: spec.required,
                    baseline_status: StatisticStatus::NotAssessed,
                    repaired_status: StatisticStatus::NotAssessed,
                    baseline_interval_width: None,
                    repaired_interval_width: None,
                    interval_width_ratio: None,
                    outcome: RepairRowOutcome::NotAssessed,
                });
                continue;
            };

            if baseline_row.source_digest != repaired_row.source_digest
                || baseline_row.observation_digest != repaired_row.observation_digest
                || baseline_row.observed_value != repaired_row.observed_value
                || baseline_row.sample_size != repaired_row.sample_size
                || baseline_row.measurement_unit != repaired_row.measurement_unit
                || baseline_row.measurement_standard_error != repaired_row.measurement_standard_error
            {
                return Err(RepairQualificationError::ChallengeObservationMismatch {
                    statistic_id: spec.id.clone(),
                    unit_id: unit_id.clone(),
                });
            }

            let baseline_width = interval_width(baseline_row);
            let repaired_width = interval_width(repaired_row);
            let interval_width_ratio = baseline_width
                .filter(|width| *width > 0.0)
                .zip(repaired_width)
                .map(|(baseline, repaired)| repaired / baseline);
            let width_exceeded = match (baseline_width, repaired_width) {
                (Some(baseline), Some(repaired)) => {
                    repaired > baseline * maximum_interval_width_ratio + f64::EPSILON
                }
                _ => false,
            };

            let row_outcome = if baseline_row.status == StatisticStatus::NotAssessed
                || repaired_row.status == StatisticStatus::NotAssessed
            {
                if spec.required {
                    required_evidence_complete = false;
                }
                RepairRowOutcome::NotAssessed
            } else if width_exceeded {
                width_budget_failures += usize::from(spec.primary);
                RepairRowOutcome::WidthBudgetExceeded
            } else {
                match (baseline_row.status, repaired_row.status) {
                    (
                        StatisticStatus::OutsideReferenceInterval,
                        StatisticStatus::WithinReferenceInterval,
                    ) => {
                        corrected_primary_statistics += usize::from(spec.primary);
                        RepairRowOutcome::CorrectedOnFreshChallenge
                    }
                    (
                        StatisticStatus::WithinReferenceInterval,
                        StatisticStatus::OutsideReferenceInterval,
                    ) => {
                        regressed_primary_statistics += usize::from(spec.primary);
                        RepairRowOutcome::Regression
                    }
                    (_, StatisticStatus::OutsideReferenceInterval) => {
                        residual_primary_discrepancies += usize::from(spec.primary);
                        RepairRowOutcome::ResidualDiscrepancy
                    }
                    (
                        StatisticStatus::WithinReferenceInterval,
                        StatisticStatus::WithinReferenceInterval,
                    ) => RepairRowOutcome::Maintained,
                    _ => RepairRowOutcome::NotAssessed,
                }
            };

            results.push(RepairUnitResult {
                statistic_id: spec.id.clone(),
                independent_unit_id: unit_id.clone(),
                primary: spec.primary,
                required: spec.required,
                baseline_status: baseline_row.status,
                repaired_status: repaired_row.status,
                baseline_interval_width: baseline_width,
                repaired_interval_width: repaired_width,
                interval_width_ratio,
                outcome: row_outcome,
            });
        }
    }

    let has_primary_regression = regressed_primary_statistics > 0;
    let has_primary_residual = residual_primary_discrepancies > 0;
    let has_primary_width_failure = width_budget_failures > 0;
    let claim_ceiling = if has_primary_regression {
        RepairClaimCeiling::RegressionDetected
    } else if has_primary_residual {
        RepairClaimCeiling::ResidualDiscrepancy
    } else if has_primary_width_failure {
        RepairClaimCeiling::WidthBudgetExceeded
    } else if !required_evidence_complete {
        RepairClaimCeiling::NotAssessed
    } else if corrected_primary_statistics > 0 {
        RepairClaimCeiling::RepairRestoredAdequacyOnFreshChallenge
    } else {
        RepairClaimCeiling::NoFreshChallengeImprovementDemonstrated
    };

    Ok(RepairQualificationReport {
        initial_model_digest: diagnostic.model_digest.clone(),
        repaired_model_digest: repaired_challenge.model_digest.clone(),
        initial_calibration_data_digest: initial_calibration_data_digest.to_string(),
        repair_training_data_digest: repair_training_data_digest.to_string(),
        diagnostic_split_manifest_digest: diagnostic.manifest.manifest_digest.clone(),
        baseline_challenge_manifest_digest: baseline_challenge.manifest.manifest_digest.clone(),
        repaired_challenge_manifest_digest: repaired_challenge.manifest.manifest_digest.clone(),
        repair_action_digest: repair_action_digest.to_string(),
        protocol_digest: protocol_digest.to_string(),
        central_mass: diagnostic_result.central_mass,
        maximum_interval_width_ratio,
        diagnostic_primary_discrepancies: diagnostic_result.primary_discrepancies,
        corrected_primary_statistics,
        residual_primary_discrepancies,
        regressed_primary_statistics,
        width_budget_failures,
        required_evidence_complete,
        claim_ceiling,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_adequacy::{
        assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
        StatisticSpec,
    };

    const INITIAL_MODEL: &str = "sha256-initial-model";
    const REPAIRED_MODEL: &str = "sha256-repaired-model";
    const INITIAL_TRAINING: &str = "sha256-initial-training";
    const REPAIR_TRAINING: &str = "sha256-repair-training";
    const CENTRAL_MASS: f64 = 0.9;

    #[derive(Clone)]
    struct MetricFixture {
        id: &'static str,
        unit: &'static str,
        observed: f64,
        draws: Vec<f64>,
    }

    fn evidence(
        domain_id: &str,
        role: SplitRole,
        model_digest: &str,
        calibration_digest: &str,
        source_digest: &str,
        observation_digest_prefix: &str,
        metrics: &[MetricFixture],
    ) -> RepairEvidence {
        let independent_unit = IndependentUnitKind::Region;
        let unit_id = match role {
            SplitRole::Validation => "development-region",
            SplitRole::Challenge => "sealed-challenge-region",
            SplitRole::Calibration => unreachable!(),
        };
        let statistic_specs = metrics
            .iter()
            .map(|metric| StatisticSpec {
                id: metric.id.to_string(),
                construct: format!("construct-{}", metric.id),
                measurement_unit: metric.unit.to_string(),
                population: "synthetic-households".to_string(),
                context: "repair-known-answer".to_string(),
                source_family: "synthetic-repair-source".to_string(),
                source_digest: source_digest.to_string(),
                independent_unit,
                role,
                primary: true,
                required: true,
                minimum_draws: metric.draws.len(),
            })
            .collect();
        let registry = StatisticRegistry {
            version: "repair-registry-v1".to_string(),
            statistics: statistic_specs,
        };
        let manifest = SplitManifest {
            id: format!("{domain_id}-split"),
            independent_unit,
            calibration_data_digest: calibration_digest.to_string(),
            manifest_digest: format!("sha256-{domain_id}-manifest"),
            calibration_units: vec![format!("{domain_id}-calibration-unit")],
            validation_units: vec![if role == SplitRole::Validation {
                unit_id.to_string()
            } else {
                format!("{domain_id}-validation-reserved")
            }],
            challenge_units: if role == SplitRole::Challenge {
                vec![unit_id.to_string()]
            } else {
                Vec::new()
            },
        };
        let observations: Vec<_> = metrics
            .iter()
            .map(|metric| ObservedStatistic {
                statistic_id: metric.id.to_string(),
                independent_unit_id: unit_id.to_string(),
                measurement_unit: metric.unit.to_string(),
                source_digest: source_digest.to_string(),
                sample_size: 100,
                value: metric.observed,
                measurement_standard_error: Some(0.01),
                observation_digest: format!("{observation_digest_prefix}-{}", metric.id),
            })
            .collect();
        let draws: Vec<_> = metrics
            .iter()
            .flat_map(|metric| {
                metric
                    .draws
                    .iter()
                    .enumerate()
                    .map(move |(index, value)| PredictiveDraw {
                        kind: PredictiveKind::Split,
                        statistic_id: metric.id.to_string(),
                        independent_unit_id: unit_id.to_string(),
                        sample_size: 100,
                        draw_index: index as u64,
                        value: *value,
                        model_digest: model_digest.to_string(),
                        draw_set_digest: format!("{domain_id}-draws-{}", metric.id),
                        calibration_data_digest: Some(calibration_digest.to_string()),
                    })
            })
            .collect();
        let assessment = assess_predictive_partition(
            &registry,
            &manifest,
            &observations,
            &draws,
            role,
            PredictiveKind::Split,
            CENTRAL_MASS,
        )
        .expect("valid synthetic repair evidence");
        RepairEvidence {
            domain_id: domain_id.to_string(),
            model_digest: model_digest.to_string(),
            registry,
            manifest,
            assessment,
        }
    }

    fn metric(id: &'static str, observed: f64, draws: &[f64]) -> MetricFixture {
        MetricFixture {
            id,
            unit: "coverage_fraction",
            observed,
            draws: draws.to_vec(),
        }
    }

    fn scenario(
        diagnostic_metrics: &[MetricFixture],
        baseline_metrics: &[MetricFixture],
        repaired_metrics: &[MetricFixture],
    ) -> (
        RepairEvidence,
        RepairEvidence,
        RepairEvidence,
        Vec<String>,
        Vec<String>,
    ) {
        let diagnostic = evidence(
            "diagnostic",
            SplitRole::Validation,
            INITIAL_MODEL,
            INITIAL_TRAINING,
            "sha256-diagnostic-source",
            "sha256-diagnostic-observation",
            diagnostic_metrics,
        );
        let baseline = evidence(
            "baseline",
            SplitRole::Challenge,
            INITIAL_MODEL,
            INITIAL_TRAINING,
            "sha256-fresh-challenge-source",
            "sha256-fresh-challenge-observation",
            baseline_metrics,
        );
        let repaired = evidence(
            "repaired",
            SplitRole::Challenge,
            REPAIRED_MODEL,
            REPAIR_TRAINING,
            "sha256-fresh-challenge-source",
            "sha256-fresh-challenge-observation",
            repaired_metrics,
        );
        (
            diagnostic,
            baseline,
            repaired,
            vec!["sha256-diagnostic-source".to_string()],
            diagnostic_metrics
                .iter()
                .map(|metric| format!("sha256-diagnostic-observation-{}", metric.id))
                .collect(),
        )
    }

    fn qualify(
        diagnostic: &RepairEvidence,
        baseline: &RepairEvidence,
        repaired: &RepairEvidence,
        training_sources: &[String],
        training_observations: &[String],
        max_width_ratio: f64,
    ) -> Result<RepairQualificationReport, RepairQualificationError> {
        qualify_repair(
            diagnostic,
            baseline,
            repaired,
            INITIAL_TRAINING,
            REPAIR_TRAINING,
            training_sources,
            training_observations,
            "sha256-repair-action",
            "sha256-repair-protocol",
            max_width_ratio,
        )
    }

    #[test]
    fn correctly_repaired_primary_discrepancy_must_pass_fresh_paired_challenge() {
        let diagnostic = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let baseline = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let repaired = vec![metric("essential-service", 0.9, &[0.85, 0.88, 0.90, 0.92, 0.95])];
        let (diag, before, after, sources, observations) =
            scenario(&diagnostic, &baseline, &repaired);
        let report = qualify(&diag, &before, &after, &sources, &observations, 1.0)
            .expect("fresh challenge repair qualification");
        assert_eq!(
            report.claim_ceiling,
            RepairClaimCeiling::RepairRestoredAdequacyOnFreshChallenge
        );
        assert_eq!(report.corrected_primary_statistics, 1);
        assert_eq!(report.residual_primary_discrepancies, 0);
        assert_eq!(report.regressed_primary_statistics, 0);
        assert_eq!(
            report.results[0].outcome,
            RepairRowOutcome::CorrectedOnFreshChallenge
        );
    }

    #[test]
    fn post_repair_residual_is_not_hidden_by_diagnostic_improvement() {
        let diagnostic = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let baseline = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let repaired = vec![metric("essential-service", 0.9, &[0.50, 0.52, 0.54, 0.56, 0.58])];
        let (diag, before, after, sources, observations) =
            scenario(&diagnostic, &baseline, &repaired);
        let report = qualify(&diag, &before, &after, &sources, &observations, 1.0)
            .expect("residual discrepancy report");
        assert_eq!(report.claim_ceiling, RepairClaimCeiling::ResidualDiscrepancy);
        assert_eq!(report.residual_primary_discrepancies, 1);
    }

    #[test]
    fn previously_adequate_primary_metric_regression_blocks_repair_pass() {
        let diagnostic = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let baseline = vec![metric("essential-service", 0.5, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let repaired = vec![metric("essential-service", 0.5, &[0.90, 0.92, 0.94, 0.96, 0.98])];
        let (diag, before, after, sources, observations) =
            scenario(&diagnostic, &baseline, &repaired);
        let report = qualify(&diag, &before, &after, &sources, &observations, 1.0)
            .expect("regression report");
        assert_eq!(report.claim_ceiling, RepairClaimCeiling::RegressionDetected);
        assert_eq!(report.regressed_primary_statistics, 1);
    }

    #[test]
    fn interval_widening_cannot_masquerade_as_restored_adequacy() {
        let diagnostic = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let baseline = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let repaired = vec![metric("essential-service", 0.9, &[0.30, 0.50, 0.70, 0.90, 1.10])];
        let (diag, before, after, sources, observations) =
            scenario(&diagnostic, &baseline, &repaired);
        let report = qualify(&diag, &before, &after, &sources, &observations, 1.0)
            .expect("width-budget report");
        assert_eq!(report.claim_ceiling, RepairClaimCeiling::WidthBudgetExceeded);
        assert_eq!(report.width_budget_failures, 1);
        assert_eq!(report.results[0].repaired_status, StatisticStatus::WithinReferenceInterval);
    }

    #[test]
    fn held_out_statistics_are_not_averaged_away_by_a_targeted_repair() {
        let diagnostic = vec![
            metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55]),
            metric("network-connectivity", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55]),
        ];
        let baseline = diagnostic.clone();
        let repaired = vec![
            metric("essential-service", 0.9, &[0.85, 0.88, 0.90, 0.92, 0.95]),
            metric("network-connectivity", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55]),
        ];
        let (diag, before, after, sources, observations) =
            scenario(&diagnostic, &baseline, &repaired);
        let report = qualify(&diag, &before, &after, &sources, &observations, 1.0)
            .expect("multi-statistic repair report");
        assert_eq!(report.corrected_primary_statistics, 1);
        assert_eq!(report.residual_primary_discrepancies, 1);
        assert_eq!(report.claim_ceiling, RepairClaimCeiling::ResidualDiscrepancy);
    }

    #[test]
    fn no_fresh_challenge_improvement_is_not_attributed_to_the_repair() {
        let diagnostic = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let baseline = vec![metric("essential-service", 0.5, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let repaired = vec![metric("essential-service", 0.5, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let (diag, before, after, sources, observations) =
            scenario(&diagnostic, &baseline, &repaired);
        let report = qualify(&diag, &before, &after, &sources, &observations, 1.0)
            .expect("no improvement report");
        assert_eq!(
            report.claim_ceiling,
            RepairClaimCeiling::NoFreshChallengeImprovementDemonstrated
        );
        assert_eq!(report.corrected_primary_statistics, 0);
        assert_eq!(report.results[0].outcome, RepairRowOutcome::Maintained);
    }

    #[test]
    fn challenge_observation_or_source_reuse_in_repair_training_is_rejected() {
        let diagnostic = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let baseline = diagnostic.clone();
        let repaired = vec![metric("essential-service", 0.9, &[0.85, 0.88, 0.90, 0.92, 0.95])];
        let (diag, before, after, mut sources, observations) =
            scenario(&diagnostic, &baseline, &repaired);
        sources.push("sha256-fresh-challenge-source".to_string());
        assert!(matches!(
            qualify(&diag, &before, &after, &sources, &observations, 1.0),
            Err(RepairQualificationError::ChallengeRepairDataLeakage { .. })
        ));
    }

    #[test]
    fn baseline_and_repaired_models_must_use_identical_challenge_observations() {
        let diagnostic = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let baseline = diagnostic.clone();
        let repaired = vec![metric("essential-service", 0.8, &[0.75, 0.78, 0.80, 0.82, 0.85])];
        let (diag, before, after, sources, observations) =
            scenario(&diagnostic, &baseline, &repaired);
        assert!(matches!(
            qualify(&diag, &before, &after, &sources, &observations, 1.0),
            Err(RepairQualificationError::ChallengeObservationMismatch { .. })
        ));
    }

    #[test]
    fn no_op_model_digest_and_invalid_width_budget_are_rejected() {
        let diagnostic = vec![metric("essential-service", 0.9, &[0.45, 0.48, 0.50, 0.52, 0.55])];
        let baseline = diagnostic.clone();
        let repaired = baseline.clone();
        let (diag, before, after, sources, observations) =
            scenario(&diagnostic, &baseline, &repaired);
        let mut same_model = after.clone();
        same_model.model_digest = INITIAL_MODEL.to_string();
        assert_eq!(
            qualify(&diag, &before, &same_model, &sources, &observations, 1.0).unwrap_err(),
            RepairQualificationError::SameInitialAndRepairedModel
        );
        assert_eq!(
            qualify(&diag, &before, &after, &sources, &observations, 0.9).unwrap_err(),
            RepairQualificationError::InvalidWidthBudget
        );
    }
}
