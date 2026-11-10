// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Statistic-held-out, fresh-challenge qualification for model repair.
//!
//! This module does not perform repairs. It verifies that baseline and repaired
//! models are compared against the same untouched challenge observations, and
//! that challenge statistic IDs were excluded from the declared repair inputs.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::model_adequacy::{
    AdequacyAssessment, AdequacyError, DomainStatus, PredictiveKind, SplitManifest, SplitRole,
    StatisticRegistry, StatisticStatus,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RepairQualificationInput {
    pub baseline_model_digest: String,
    pub repaired_model_digest: String,
    pub repair_plan_digest: String,
    pub diagnostic_manifest_digest: String,
    /// Every statistic identifier made available to diagnosis/repair.
    pub diagnostic_statistic_ids: Vec<String>,
    /// Observation receipt digests made available to diagnosis/repair.
    pub diagnostic_observation_digests: Vec<String>,
    pub challenge_registry: StatisticRegistry,
    pub challenge_manifest: SplitManifest,
    pub baseline_assessment: AdequacyAssessment,
    pub repaired_assessment: AdequacyAssessment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairTransition {
    Improved,
    Regressed,
    PersistentDiscrepancy,
    UnchangedWithinReference,
    NotAssessed,
}

impl RepairTransition {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Improved => "improved",
            Self::Regressed => "regressed",
            Self::PersistentDiscrepancy => "persistent_discrepancy",
            Self::UnchangedWithinReference => "unchanged_within_reference",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairQualificationStatus {
    RestoredOnFreshChallenge,
    ImprovedButNotRestored,
    RegressionDetected,
    PersistentDiscrepancy,
    NoPrimaryDiscrepancyBeforeOrAfter,
    NotAssessed,
}

impl RepairQualificationStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::RestoredOnFreshChallenge => "restored_on_fresh_challenge",
            Self::ImprovedButNotRestored => "improved_but_not_restored",
            Self::RegressionDetected => "regression_detected",
            Self::PersistentDiscrepancy => "persistent_discrepancy",
            Self::NoPrimaryDiscrepancyBeforeOrAfter => "no_primary_discrepancy_before_or_after",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairStatisticTransition {
    pub statistic_id: String,
    pub independent_unit_id: String,
    pub primary: bool,
    pub required: bool,
    pub baseline_status: StatisticStatus,
    pub repaired_status: StatisticStatus,
    pub transition: RepairTransition,
    pub observation_digest: Option<String>,
    pub source_digest: String,
    pub sample_size: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepairQualificationReport {
    pub baseline_model_digest: String,
    pub repaired_model_digest: String,
    pub repair_plan_digest: String,
    pub diagnostic_manifest_digest: String,
    pub challenge_manifest_id: String,
    pub challenge_manifest_digest: String,
    pub calibration_data_digest: String,
    pub central_mass: f64,
    pub challenge_unit_count: usize,
    pub evidence_complete: bool,
    pub baseline_required_evidence_complete: bool,
    pub repaired_required_evidence_complete: bool,
    pub primary_improvement_count: usize,
    pub primary_regression_count: usize,
    pub primary_persistent_discrepancy_count: usize,
    pub primary_baseline_discrepancy_count: usize,
    pub primary_repaired_discrepancy_count: usize,
    pub status: RepairQualificationStatus,
    pub transitions: Vec<RepairStatisticTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AssessmentKey {
    statistic_id: String,
    independent_unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairQualificationError {
    Adequacy(AdequacyError),
    MissingIdentityDigest,
    BaselineAndRepairedModelMustDiffer,
    DiagnosticAndChallengeManifestMustDiffer,
    EmptyDiagnosticStatisticSet,
    EmptyDiagnosticStatisticId,
    DuplicateDiagnosticStatisticId { statistic_id: String },
    ChallengeStatisticWasUsedForRepair { statistic_id: String },
    EmptyDiagnosticObservationSet,
    EmptyDiagnosticObservationDigest,
    DuplicateDiagnosticObservationDigest { observation_digest: String },
    ChallengeObservationWasUsedForRepair { observation_digest: String },
    ChallengeMustUseSplitPredictiveAssessment,
    ChallengeManifestMismatch,
    ChallengeRegistryMismatch,
    ChallengeCentralMassMismatch,
    ChallengeAssessmentRoleMismatch,
    ChallengeRegistryMustContainOnlyChallengeStatistics,
    ChallengeIndependentUnitMismatch { statistic_id: String },
    EmptyChallengeUnits,
    AssessmentModelIdentityMismatch { model_digest: String },
    DuplicateAssessmentRow { statistic_id: String, unit_id: String },
    UnknownAssessmentStatistic { statistic_id: String },
    UnknownAssessmentUnit { statistic_id: String, unit_id: String },
    AssessmentStatisticIdentityMismatch { statistic_id: String },
    MissingRequiredAssessmentRow { statistic_id: String, unit_id: String },
    RequiredEvidenceCompletenessMismatch,
    DomainStatusMismatch,
    BaselineAndRepairedObservationMismatch { statistic_id: String, unit_id: String },
    BaselineAndRepairedChallengeVocabularyMismatch,
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

type AssessmentRows<'a> =
    BTreeMap<AssessmentKey, &'a crate::model_adequacy::StatisticAssessment>;

fn validate_assessment<'a>(
    assessment: &'a AdequacyAssessment,
    expected_model_digest: &str,
    registry: &StatisticRegistry,
    manifest: &SplitManifest,
) -> Result<(AssessmentRows<'a>, bool), RepairQualificationError> {
    if assessment.role != SplitRole::Challenge
        || assessment.predictive_kind != PredictiveKind::Split
    {
        return Err(RepairQualificationError::ChallengeMustUseSplitPredictiveAssessment);
    }
    if assessment.registry_version != registry.version {
        return Err(RepairQualificationError::ChallengeRegistryMismatch);
    }
    if assessment.split_manifest_id != manifest.id
        || assessment.split_manifest_digest != manifest.manifest_digest
    {
        return Err(RepairQualificationError::ChallengeManifestMismatch);
    }
    if assessment.central_mass <= 0.0
        || assessment.central_mass >= 1.0
        || !assessment.central_mass.is_finite()
    {
        return Err(RepairQualificationError::ChallengeCentralMassMismatch);
    }
    if assessment.role != SplitRole::Challenge {
        return Err(RepairQualificationError::ChallengeAssessmentRoleMismatch);
    }

    let challenge_units: BTreeSet<&str> =
        manifest.challenge_units.iter().map(String::as_str).collect();
    let mut rows = BTreeMap::new();

    for row in &assessment.statistics {
        let spec = registry.get(&row.statistic_id).ok_or_else(|| {
            RepairQualificationError::UnknownAssessmentStatistic {
                statistic_id: row.statistic_id.clone(),
            }
        })?;
        if !challenge_units.contains(row.independent_unit_id.as_str()) {
            return Err(RepairQualificationError::UnknownAssessmentUnit {
                statistic_id: row.statistic_id.clone(),
                unit_id: row.independent_unit_id.clone(),
            });
        }
        if spec.role != SplitRole::Challenge
            || spec.independent_unit != manifest.independent_unit
            || row.source_family != spec.source_family
            || row.source_digest != spec.source_digest
            || row.measurement_unit != spec.measurement_unit
        {
            return Err(RepairQualificationError::AssessmentStatisticIdentityMismatch {
                statistic_id: row.statistic_id.clone(),
            });
        }

        if let Some(receipt_model) = row.model_digest.as_deref() {
            if receipt_model != expected_model_digest {
                return Err(RepairQualificationError::AssessmentModelIdentityMismatch {
                    model_digest: receipt_model.to_string(),
                });
            }
        }

        if row.status != StatisticStatus::NotAssessed {
            let observed = row.observed_value;
            let lower = row.lower_reference;
            let upper = row.upper_reference;
            let consistent = row.model_digest.as_deref() == Some(expected_model_digest)
                && row.draw_set_digest.as_deref().is_some_and(|digest| !digest.trim().is_empty())
                && row.observation_digest.as_deref().is_some_and(|digest| !digest.trim().is_empty())
                && row.sample_size > 0
                && observed.is_some_and(f64::is_finite)
                && lower.is_some_and(f64::is_finite)
                && upper.is_some_and(f64::is_finite)
                && lower <= upper
                && row.predictive_rank.is_some_and(|rank| {
                    rank.is_finite() && (0.0..=1.0).contains(&rank)
                })
                && row.draw_count >= spec.minimum_draws
                && row.measurement_standard_error.is_none_or(|value| {
                    value.is_finite() && value >= 0.0
                })
                && match (row.status, observed, lower, upper) {
                    (
                        StatisticStatus::WithinReferenceInterval,
                        Some(value),
                        Some(low),
                        Some(high),
                    ) => value >= low && value <= high,
                    (
                        StatisticStatus::OutsideReferenceInterval,
                        Some(value),
                        Some(low),
                        Some(high),
                    ) => value < low || value > high,
                    _ => false,
                };
            if !consistent {
                return Err(RepairQualificationError::AssessmentStatisticIdentityMismatch {
                    statistic_id: row.statistic_id.clone(),
                });
            }
        }

        let key = AssessmentKey {
            statistic_id: row.statistic_id.clone(),
            independent_unit_id: row.independent_unit_id.clone(),
        };
        if rows.insert(key, row).is_some() {
            return Err(RepairQualificationError::DuplicateAssessmentRow {
                statistic_id: row.statistic_id.clone(),
                unit_id: row.independent_unit_id.clone(),
            });
        }
    }

    for spec in registry.statistics.iter().filter(|spec| spec.required) {
        for unit_id in &manifest.challenge_units {
            let key = AssessmentKey {
                statistic_id: spec.id.clone(),
                independent_unit_id: unit_id.clone(),
            };
            if !rows.contains_key(&key) {
                return Err(RepairQualificationError::MissingRequiredAssessmentRow {
                    statistic_id: spec.id.clone(),
                    unit_id: unit_id.clone(),
                });
            }
        }
    }

    let required_specs: Vec<_> = registry
        .statistics
        .iter()
        .filter(|spec| spec.required)
        .collect();
    let required_evidence_complete = !required_specs.is_empty()
        && !manifest.challenge_units.is_empty()
        && required_specs.iter().all(|spec| {
            manifest.challenge_units.iter().all(|unit_id| {
                rows.get(&AssessmentKey {
                    statistic_id: spec.id.clone(),
                    independent_unit_id: unit_id.clone(),
                })
                .is_some_and(|row| row.status != StatisticStatus::NotAssessed)
            })
        });

    if assessment.required_evidence_complete != required_evidence_complete {
        return Err(RepairQualificationError::RequiredEvidenceCompletenessMismatch);
    }

    let primary_specs: BTreeSet<&str> = registry
        .statistics
        .iter()
        .filter(|spec| spec.primary)
        .map(|spec| spec.id.as_str())
        .collect();
    let primary_rows: Vec<_> = rows
        .iter()
        .filter(|(key, _)| primary_specs.contains(key.statistic_id.as_str()))
        .map(|(_, row)| *row)
        .collect();
    let derived_domain_status = if primary_rows
        .iter()
        .any(|row| row.status == StatisticStatus::OutsideReferenceInterval)
    {
        DomainStatus::PrimaryDiscrepancyDetected
    } else if primary_rows.is_empty()
        || !required_evidence_complete
        || primary_rows
            .iter()
            .any(|row| row.status == StatisticStatus::NotAssessed)
    {
        DomainStatus::NotAssessed
    } else {
        DomainStatus::NoPrimaryDiscrepancyDetected
    };
    if assessment.domain_status != derived_domain_status {
        return Err(RepairQualificationError::DomainStatusMismatch);
    }

    Ok((rows, required_evidence_complete))
}

/// Qualify repair on a statistic-held-out Challenge partition.
///
/// The baseline and repaired model are evaluated against identical challenge
/// observations and reference protocol. Challenge statistic IDs are disjoint
/// from the diagnostics declared as inputs to the repair.
pub fn qualify_repair_on_fresh_challenge(
    input: &RepairQualificationInput,
) -> Result<RepairQualificationReport, RepairQualificationError> {
    for digest in [
        input.baseline_model_digest.as_str(),
        input.repaired_model_digest.as_str(),
        input.repair_plan_digest.as_str(),
        input.diagnostic_manifest_digest.as_str(),
    ] {
        if digest.trim().is_empty() {
            return Err(RepairQualificationError::MissingIdentityDigest);
        }
    }
    if input.baseline_model_digest == input.repaired_model_digest {
        return Err(RepairQualificationError::BaselineAndRepairedModelMustDiffer);
    }
    input.challenge_registry.validate()?;
    input.challenge_manifest.validate()?;
    if input.diagnostic_manifest_digest == input.challenge_manifest.manifest_digest {
        return Err(RepairQualificationError::DiagnosticAndChallengeManifestMustDiffer);
    }
    if input.challenge_manifest.challenge_units.is_empty() {
        return Err(RepairQualificationError::EmptyChallengeUnits);
    }
    if input.challenge_registry.statistics.is_empty()
        || input.challenge_registry.statistics.iter().any(|spec| spec.role != SplitRole::Challenge)
    {
        return Err(RepairQualificationError::ChallengeRegistryMustContainOnlyChallengeStatistics);
    }
    if input.challenge_registry.statistics.iter().any(|spec| {
        spec.independent_unit != input.challenge_manifest.independent_unit
    }) {
        let spec = input.challenge_registry.statistics.iter()
            .find(|spec| spec.independent_unit != input.challenge_manifest.independent_unit)
            .expect("mismatched statistic exists");
        return Err(RepairQualificationError::ChallengeIndependentUnitMismatch {
            statistic_id: spec.id.clone(),
        });
    }

    if input.diagnostic_statistic_ids.is_empty() {
        return Err(RepairQualificationError::EmptyDiagnosticStatisticSet);
    }
    let mut diagnostic_ids = BTreeSet::new();
    for statistic_id in &input.diagnostic_statistic_ids {
        if statistic_id.trim().is_empty() {
            return Err(RepairQualificationError::EmptyDiagnosticStatisticId);
        }
        if !diagnostic_ids.insert(statistic_id.as_str()) {
            return Err(RepairQualificationError::DuplicateDiagnosticStatisticId {
                statistic_id: statistic_id.clone(),
            });
        }
    }
    for spec in &input.challenge_registry.statistics {
        if diagnostic_ids.contains(spec.id.as_str()) {
            return Err(RepairQualificationError::ChallengeStatisticWasUsedForRepair {
                statistic_id: spec.id.clone(),
            });
        }
    }

    if input.diagnostic_observation_digests.is_empty() {
        return Err(RepairQualificationError::EmptyDiagnosticObservationSet);
    }
    let mut diagnostic_observations = BTreeSet::new();
    for observation_digest in &input.diagnostic_observation_digests {
        if observation_digest.trim().is_empty() {
            return Err(RepairQualificationError::EmptyDiagnosticObservationDigest);
        }
        if !diagnostic_observations.insert(observation_digest.as_str()) {
            return Err(RepairQualificationError::DuplicateDiagnosticObservationDigest {
                observation_digest: observation_digest.clone(),
            });
        }
    }

    let (baseline_rows, baseline_complete) = validate_assessment(
        &input.baseline_assessment,
        &input.baseline_model_digest,
        &input.challenge_registry,
        &input.challenge_manifest,
    )?;
    let (repaired_rows, repaired_complete) = validate_assessment(
        &input.repaired_assessment,
        &input.repaired_model_digest,
        &input.challenge_registry,
        &input.challenge_manifest,
    )?;

    if input.baseline_assessment.central_mass != input.repaired_assessment.central_mass {
        return Err(RepairQualificationError::ChallengeCentralMassMismatch);
    }
    if baseline_rows.keys().ne(repaired_rows.keys()) {
        return Err(RepairQualificationError::BaselineAndRepairedChallengeVocabularyMismatch);
    }

    let mut transitions = Vec::with_capacity(baseline_rows.len());
    let mut primary_improvement_count = 0;
    let mut primary_regression_count = 0;
    let mut primary_persistent_discrepancy_count = 0;
    let mut primary_baseline_discrepancy_count = 0;
    let mut primary_repaired_discrepancy_count = 0;

    for (key, baseline_row) in &baseline_rows {
        let repaired_row = repaired_rows.get(key).ok_or(
            RepairQualificationError::BaselineAndRepairedChallengeVocabularyMismatch,
        )?;
        if baseline_row.source_family != repaired_row.source_family
            || baseline_row.source_digest != repaired_row.source_digest
            || baseline_row.observation_digest != repaired_row.observation_digest
            || baseline_row.measurement_unit != repaired_row.measurement_unit
            || baseline_row.sample_size != repaired_row.sample_size
            || baseline_row.observed_value != repaired_row.observed_value
            || baseline_row.measurement_standard_error != repaired_row.measurement_standard_error
        {
            return Err(RepairQualificationError::BaselineAndRepairedObservationMismatch {
                statistic_id: key.statistic_id.clone(),
                unit_id: key.independent_unit_id.clone(),
            });
        }

        if let Some(observation_digest) = baseline_row.observation_digest.as_deref() {
            if diagnostic_observations.contains(observation_digest) {
                return Err(RepairQualificationError::ChallengeObservationWasUsedForRepair {
                    observation_digest: observation_digest.to_string(),
                });
            }
        }
        let spec = input.challenge_registry
            .get(&key.statistic_id)
            .expect("assessment key already validated against registry");
        let transition = match (baseline_row.status, repaired_row.status) {
            (StatisticStatus::OutsideReferenceInterval, StatisticStatus::WithinReferenceInterval) => {
                RepairTransition::Improved
            }
            (StatisticStatus::WithinReferenceInterval, StatisticStatus::OutsideReferenceInterval) => {
                RepairTransition::Regressed
            }
            (StatisticStatus::OutsideReferenceInterval, StatisticStatus::OutsideReferenceInterval) => {
                RepairTransition::PersistentDiscrepancy
            }
            (StatisticStatus::WithinReferenceInterval, StatisticStatus::WithinReferenceInterval) => {
                RepairTransition::UnchangedWithinReference
            }
            _ => RepairTransition::NotAssessed,
        };

        if spec.primary {
            if baseline_row.status == StatisticStatus::OutsideReferenceInterval {
                primary_baseline_discrepancy_count += 1;
            }
            if repaired_row.status == StatisticStatus::OutsideReferenceInterval {
                primary_repaired_discrepancy_count += 1;
            }
            match transition {
                RepairTransition::Improved => primary_improvement_count += 1,
                RepairTransition::Regressed => primary_regression_count += 1,
                RepairTransition::PersistentDiscrepancy => {
                    primary_persistent_discrepancy_count += 1
                }
                RepairTransition::UnchangedWithinReference | RepairTransition::NotAssessed => {}
            }
        }

        transitions.push(RepairStatisticTransition {
            statistic_id: key.statistic_id.clone(),
            independent_unit_id: key.independent_unit_id.clone(),
            primary: spec.primary,
            required: spec.required,
            baseline_status: baseline_row.status,
            repaired_status: repaired_row.status,
            transition,
            observation_digest: baseline_row.observation_digest.clone(),
            source_digest: baseline_row.source_digest.clone(),
            sample_size: baseline_row.sample_size,
        });
    }

    let evidence_complete = baseline_complete && repaired_complete;
    let status = if primary_regression_count > 0 {
        RepairQualificationStatus::RegressionDetected
    } else if !evidence_complete {
        RepairQualificationStatus::NotAssessed
    } else if primary_baseline_discrepancy_count > 0
        && primary_repaired_discrepancy_count == 0
    {
        RepairQualificationStatus::RestoredOnFreshChallenge
    } else if primary_improvement_count > 0 && primary_repaired_discrepancy_count > 0 {
        RepairQualificationStatus::ImprovedButNotRestored
    } else if primary_repaired_discrepancy_count > 0 {
        RepairQualificationStatus::PersistentDiscrepancy
    } else {
        RepairQualificationStatus::NoPrimaryDiscrepancyBeforeOrAfter
    };

    Ok(RepairQualificationReport {
        baseline_model_digest: input.baseline_model_digest.clone(),
        repaired_model_digest: input.repaired_model_digest.clone(),
        repair_plan_digest: input.repair_plan_digest.clone(),
        diagnostic_manifest_digest: input.diagnostic_manifest_digest.clone(),
        challenge_manifest_id: input.challenge_manifest.id.clone(),
        challenge_manifest_digest: input.challenge_manifest.manifest_digest.clone(),
        calibration_data_digest: input.challenge_manifest.calibration_data_digest.clone(),
        central_mass: input.baseline_assessment.central_mass,
        challenge_unit_count: input.challenge_manifest.challenge_units.len(),
        evidence_complete,
        baseline_required_evidence_complete: baseline_complete,
        repaired_required_evidence_complete: repaired_complete,
        primary_improvement_count,
        primary_regression_count,
        primary_persistent_discrepancy_count,
        primary_baseline_discrepancy_count,
        primary_repaired_discrepancy_count,
        status,
        transitions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_adequacy::{
        assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
        StatisticSpec,
    };

    const BASELINE_MODEL: &str = "sha256-baseline-model";
    const REPAIRED_MODEL: &str = "sha256-repaired-model";
    const CALIBRATION: &str = "sha256-repair-calibration";

    fn registry() -> StatisticRegistry {
        StatisticRegistry {
            version: "repair-challenge-registry-v1".to_string(),
            statistics: vec![StatisticSpec {
                id: "held-out-transitivity".to_string(),
                construct: "global network transitivity".to_string(),
                measurement_unit: "ratio".to_string(),
                population: "held-out-networks".to_string(),
                context: "fresh-network-topology-challenge".to_string(),
                source_family: "synthetic-network-fixture".to_string(),
                source_digest: "sha256-fresh-network-source".to_string(),
                independent_unit: IndependentUnitKind::Network,
                role: SplitRole::Challenge,
                primary: true,
                required: true,
                minimum_draws: 5,
            }],
        }
    }

    fn manifest() -> SplitManifest {
        SplitManifest {
            id: "fresh-network-challenge-v1".to_string(),
            independent_unit: IndependentUnitKind::Network,
            calibration_data_digest: CALIBRATION.to_string(),
            manifest_digest: "sha256-fresh-network-challenge-manifest".to_string(),
            calibration_units: vec!["calibration-network".to_string()],
            validation_units: vec!["reserved-validation-network".to_string()],
            challenge_units: vec!["network-a".to_string(), "network-b".to_string()],
        }
    }

    fn assessment(
        model_digest: &str,
        values_a: Option<[f64; 5]>,
        values_b: Option<[f64; 5]>,
    ) -> AdequacyAssessment {
        let registry = registry();
        let manifest = manifest();
        let observations = vec![
            ObservedStatistic {
                statistic_id: "held-out-transitivity".to_string(),
                independent_unit_id: "network-a".to_string(),
                measurement_unit: "ratio".to_string(),
                source_digest: "sha256-fresh-network-source".to_string(),
                sample_size: 1,
                value: 0.8,
                measurement_standard_error: Some(0.01),
                observation_digest: "sha256-network-a-observation".to_string(),
            },
            ObservedStatistic {
                statistic_id: "held-out-transitivity".to_string(),
                independent_unit_id: "network-b".to_string(),
                measurement_unit: "ratio".to_string(),
                source_digest: "sha256-fresh-network-source".to_string(),
                sample_size: 1,
                value: 0.2,
                measurement_standard_error: Some(0.01),
                observation_digest: "sha256-network-b-observation".to_string(),
            },
        ];
        let mut draws = Vec::new();
        for (unit_id, maybe_values) in [
            ("network-a", values_a),
            ("network-b", values_b),
        ] {
            if let Some(values) = maybe_values {
                for (index, value) in values.into_iter().enumerate() {
                    draws.push(PredictiveDraw {
                        kind: PredictiveKind::Split,
                        statistic_id: "held-out-transitivity".to_string(),
                        independent_unit_id: unit_id.to_string(),
                        sample_size: 1,
                        draw_index: index as u64,
                        value,
                        model_digest: model_digest.to_string(),
                        draw_set_digest: format!("sha256-{model_digest}-{unit_id}-draws"),
                        calibration_data_digest: Some(CALIBRATION.to_string()),
                    });
                }
            }
        }
        assess_predictive_partition(
            &registry,
            &manifest,
            &observations,
            &draws,
            SplitRole::Challenge,
            PredictiveKind::Split,
            0.9,
        )
        .expect("challenge assessment")
    }

    fn input(
        baseline: AdequacyAssessment,
        repaired: AdequacyAssessment,
    ) -> RepairQualificationInput {
        RepairQualificationInput {
            baseline_model_digest: BASELINE_MODEL.to_string(),
            repaired_model_digest: REPAIRED_MODEL.to_string(),
            repair_plan_digest: "sha256-repair-plan".to_string(),
            diagnostic_manifest_digest: "sha256-diagnostic-manifest".to_string(),
            diagnostic_statistic_ids: vec!["calibration-water-mean".to_string()],
            diagnostic_observation_digests: vec!["sha256-diagnostic-observation".to_string()],
            challenge_registry: registry(),
            challenge_manifest: manifest(),
            baseline_assessment: baseline,
            repaired_assessment: repaired,
        }
    }

    #[test]
    fn repair_improvement_does_not_hide_a_persistent_held_out_discrepancy() {
        let baseline = assessment(
            BASELINE_MODEL,
            Some([0.1, 0.2, 0.2, 0.3, 0.4]),
            Some([0.6, 0.7, 0.75, 0.8, 0.9]),
        );
        let repaired = assessment(
            REPAIRED_MODEL,
            Some([0.7, 0.75, 0.8, 0.85, 0.9]),
            Some([0.6, 0.7, 0.75, 0.8, 0.9]),
        );
        let report = qualify_repair_on_fresh_challenge(&input(baseline, repaired))
            .expect("repair challenge report");
        assert_eq!(
            report.status,
            RepairQualificationStatus::ImprovedButNotRestored
        );
        assert_eq!(report.primary_improvement_count, 1);
        assert_eq!(report.primary_persistent_discrepancy_count, 1);
        assert_eq!(report.primary_repaired_discrepancy_count, 1);
        assert!(report.evidence_complete);
    }

    #[test]
    fn fully_repaired_model_is_restored_only_on_complete_fresh_challenge() {
        let baseline = assessment(
            BASELINE_MODEL,
            Some([0.1, 0.2, 0.2, 0.3, 0.4]),
            Some([0.6, 0.7, 0.75, 0.8, 0.9]),
        );
        let repaired = assessment(
            REPAIRED_MODEL,
            Some([0.7, 0.75, 0.8, 0.85, 0.9]),
            Some([0.1, 0.15, 0.2, 0.25, 0.3]),
        );
        let report = qualify_repair_on_fresh_challenge(&input(baseline, repaired))
            .expect("restored challenge report");
        assert_eq!(report.status, RepairQualificationStatus::RestoredOnFreshChallenge);
        assert_eq!(report.primary_improvement_count, 2);
        assert_eq!(report.primary_repaired_discrepancy_count, 0);
    }

    #[test]
    fn new_primary_discrepancy_is_reported_as_regression() {
        let baseline = assessment(
            BASELINE_MODEL,
            Some([0.7, 0.75, 0.8, 0.85, 0.9]),
            Some([0.1, 0.15, 0.2, 0.25, 0.3]),
        );
        let repaired = assessment(
            REPAIRED_MODEL,
            Some([0.1, 0.2, 0.2, 0.3, 0.4]),
            Some([0.1, 0.15, 0.2, 0.25, 0.3]),
        );
        let report = qualify_repair_on_fresh_challenge(&input(baseline, repaired))
            .expect("regression report");
        assert_eq!(report.status, RepairQualificationStatus::RegressionDetected);
        assert_eq!(report.primary_regression_count, 1);
    }

    #[test]
    fn rejects_statistic_leakage_into_repair_diagnostics() {
        let mut candidate = input(
            assessment(BASELINE_MODEL, Some([0.1, 0.2, 0.2, 0.3, 0.4]), Some([0.1, 0.2, 0.2, 0.3, 0.4])),
            assessment(REPAIRED_MODEL, Some([0.7, 0.75, 0.8, 0.85, 0.9]), Some([0.7, 0.75, 0.8, 0.85, 0.9])),
        );
        candidate.diagnostic_statistic_ids.push("held-out-transitivity".to_string());
        assert!(matches!(
            qualify_repair_on_fresh_challenge(&candidate),
            Err(RepairQualificationError::ChallengeStatisticWasUsedForRepair { .. })
        ));
    }

    #[test]
    fn rejects_reuse_of_fresh_challenge_observation_digest_in_repair() {
        let baseline = assessment(
            BASELINE_MODEL,
            Some([0.1, 0.2, 0.2, 0.3, 0.4]),
            Some([0.1, 0.2, 0.2, 0.3, 0.4]),
        );
        let repaired = assessment(
            REPAIRED_MODEL,
            Some([0.7, 0.75, 0.8, 0.85, 0.9]),
            Some([0.7, 0.75, 0.8, 0.85, 0.9]),
        );
        let mut candidate = input(baseline, repaired);
        candidate.diagnostic_observation_digests =
            vec!["sha256-network-a-observation".to_string()];
        assert!(matches!(
            qualify_repair_on_fresh_challenge(&candidate),
            Err(RepairQualificationError::ChallengeObservationWasUsedForRepair { .. })
        ));
    }

    #[test]
    fn rejects_same_manifest_prior_checks_and_changed_challenge_observations() {
        let baseline = assessment(
            BASELINE_MODEL,
            Some([0.1, 0.2, 0.2, 0.3, 0.4]),
            Some([0.1, 0.2, 0.2, 0.3, 0.4]),
        );
        let repaired = assessment(
            REPAIRED_MODEL,
            Some([0.7, 0.75, 0.8, 0.85, 0.9]),
            Some([0.7, 0.75, 0.8, 0.85, 0.9]),
        );
        let mut same_manifest = input(baseline.clone(), repaired.clone());
        same_manifest.diagnostic_manifest_digest =
            same_manifest.challenge_manifest.manifest_digest.clone();
        assert!(matches!(
            qualify_repair_on_fresh_challenge(&same_manifest),
            Err(RepairQualificationError::DiagnosticAndChallengeManifestMustDiffer)
        ));

        let mut prior = input(baseline.clone(), repaired.clone());
        prior.repaired_assessment.predictive_kind = PredictiveKind::Prior;
        assert!(matches!(
            qualify_repair_on_fresh_challenge(&prior),
            Err(RepairQualificationError::ChallengeMustUseSplitPredictiveAssessment)
        ));

        let mut changed_observation = input(baseline, repaired);
        changed_observation.repaired_assessment.statistics[0].observation_digest =
            Some("sha256-substituted-observation".to_string());
        assert!(matches!(
            qualify_repair_on_fresh_challenge(&changed_observation),
            Err(RepairQualificationError::BaselineAndRepairedObservationMismatch { .. })
        ));
    }

    #[test]
    fn insufficient_repaired_evidence_cannot_qualify_restoration() {
        let baseline = assessment(
            BASELINE_MODEL,
            Some([0.1, 0.2, 0.2, 0.3, 0.4]),
            Some([0.6, 0.7, 0.75, 0.8, 0.9]),
        );
        let repaired = assessment(REPAIRED_MODEL, Some([0.7, 0.75, 0.8, 0.85, 0.9]), None);
        let report = qualify_repair_on_fresh_challenge(&input(baseline, repaired))
            .expect("underpowered challenge is reportable");
        assert!(!report.evidence_complete);
        assert_eq!(report.status, RepairQualificationStatus::NotAssessed);
        assert_eq!(report.transitions[1].repaired_status, StatisticStatus::NotAssessed);
    }
}
