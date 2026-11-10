// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multi-level predictive interval calibration profiles for CIVDYN.
//!
//! This module consumes source-pinned split-predictive adequacy assessments at
//! multiple nominal central masses. It reports coverage and sharpness separately
//! for each registered statistic and never emits a pooled calibration score.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::model_adequacy::{
    AdequacyAssessment, AdequacyError, DomainStatus, IndependentUnitKind, PredictiveKind,
    SplitManifest, SplitRole, StatisticRegistry, StatisticStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationPointStatus {
    UndercoverageDetected,
    OvercoverageDetected,
    CompatibleWithNominal,
    NotAssessed,
}

impl CalibrationPointStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::UndercoverageDetected => "undercoverage_detected",
            Self::OvercoverageDetected => "overcoverage_detected",
            Self::CompatibleWithNominal => "compatible_with_nominal",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationClaimCeiling {
    CalibrationDiscrepancyDetected,
    NoDiscrepancyDetectedAtDeclaredLevels,
    NotAssessed,
}

impl CalibrationClaimCeiling {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CalibrationDiscrepancyDetected => "calibration_discrepancy_detected",
            Self::NoDiscrepancyDetectedAtDeclaredLevels => {
                "no_discrepancy_detected_at_declared_levels"
            }
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationProfileEvidence {
    /// Expected exact identity of the frozen predictive model.
    pub model_digest: String,
    pub registry: StatisticRegistry,
    pub manifest: SplitManifest,
    /// At least two split-predictive assessments, sorted by increasing central mass.
    pub assessments: Vec<AdequacyAssessment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationProfilePoint {
    pub statistic_id: String,
    pub construct: String,
    pub measurement_unit: String,
    pub population: String,
    pub context: String,
    pub source_family: String,
    pub source_digest: String,
    pub independent_unit: IndependentUnitKind,
    pub primary: bool,
    pub required: bool,
    pub central_mass: f64,
    pub nominal_coverage: f64,
    pub expected_independent_units: usize,
    pub assessed_independent_units: usize,
    pub covered_independent_units: usize,
    pub not_assessed_independent_units: usize,
    pub empirical_coverage: Option<f64>,
    pub signed_calibration_gap: Option<f64>,
    /// Pointwise, unadjusted 95% Wilson interval; not a simultaneous band.
    pub wilson_lower_95: Option<f64>,
    pub wilson_upper_95: Option<f64>,
    /// Mean interval width in this statistic's native measurement unit.
    /// Calculated over assessed units only.
    pub mean_interval_width: Option<f64>,
    pub status: CalibrationPointStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationProfileAssessment {
    pub registry_version: String,
    pub model_digest: String,
    pub split_manifest_id: String,
    pub split_manifest_digest: String,
    pub calibration_data_digest: String,
    pub role: SplitRole,
    pub predictive_kind: PredictiveKind,
    pub independent_unit: IndependentUnitKind,
    pub central_masses: Vec<f64>,
    pub evidence_complete: bool,
    pub claim_ceiling: CalibrationClaimCeiling,
    pub points: Vec<CalibrationProfilePoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalibrationProfileError {
    Adequacy(AdequacyError),
    MissingModelDigest,
    MissingAssessments,
    InvalidRole,
    PriorPredictiveNotAllowed,
    TooFewCentralMasses,
    InvalidCentralMass,
    CentralMassOrderOrDuplicate,
    RegistryVersionMismatch,
    AssessmentRoleMismatch,
    AssessmentRegistryMismatch,
    AssessmentManifestMismatch,
    StatisticRoleMismatch { statistic_id: String },
    StatisticUnitKindMismatch { statistic_id: String },
    NoPrimaryStatistic,
    UnknownStatistic { statistic_id: String },
    UnknownIndependentUnit { statistic_id: String, unit_id: String },
    DuplicateStatisticRow { statistic_id: String, unit_id: String },
    MissingRequiredStatisticRow { statistic_id: String, unit_id: String },
    RowRosterMismatch { statistic_id: String, unit_id: String },
    StatisticIdentityMismatch { statistic_id: String, unit_id: String },
    ModelDigestMismatch { statistic_id: String, unit_id: String },
    DrawSetDigestMismatch { statistic_id: String, unit_id: String },
    ObservationIdentityMismatch { statistic_id: String, unit_id: String },
    InvalidStatisticAssessment { statistic_id: String, unit_id: String },
    DuplicateCoverageSummary { statistic_id: String },
    MissingCoverageSummary { statistic_id: String },
    UnknownCoverageStatistic { statistic_id: String },
    CoverageSummaryMismatch { statistic_id: String },
    DomainStatusMismatch,
}

impl fmt::Display for CalibrationProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CalibrationProfileError {}

impl From<AdequacyError> for CalibrationProfileError {
    fn from(value: AdequacyError) -> Self {
        Self::Adequacy(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservationIdentity {
    observation_digest: Option<String>,
    sample_size: usize,
    observed_bits: Option<u64>,
    measurement_standard_error_bits: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PredictiveIdentity {
    model_digest: Option<String>,
    draw_set_digest: Option<String>,
}

fn units_for(manifest: &SplitManifest, role: SplitRole) -> &[String] {
    match role {
        SplitRole::Calibration => &manifest.calibration_units,
        SplitRole::Validation => &manifest.validation_units,
        SplitRole::Challenge => &manifest.challenge_units,
    }
}

fn close_enough(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-10 * left.abs().max(right.abs()).max(1.0)
}

fn wilson_interval_95(successes: usize, total: usize) -> (f64, f64) {
    let z = 1.959_963_984_540_054_f64;
    let n = total as f64;
    let proportion = successes as f64 / n;
    let z_squared = z * z;
    let denominator = 1.0 + z_squared / n;
    let center = (proportion + z_squared / (2.0 * n)) / denominator;
    let half_width =
        z * (proportion * (1.0 - proportion) / n + z_squared / (4.0 * n * n)).sqrt()
            / denominator;
    (
        (center - half_width).max(0.0),
        (center + half_width).min(1.0),
    )
}

fn validate_assessment_domain_status(
    assessment: &AdequacyAssessment,
    registry: &StatisticRegistry,
) -> Result<(), CalibrationProfileError> {
    let mut has_primary = false;
    let mut has_primary_discrepancy = false;
    let mut required_complete = true;

    for spec in registry.statistics.iter().filter(|spec| spec.role == assessment.role) {
        let rows: Vec<_> = assessment
            .statistics
            .iter()
            .filter(|row| row.statistic_id == spec.id)
            .collect();
        if spec.required && rows.iter().any(|row| row.status == StatisticStatus::NotAssessed) {
            required_complete = false;
        }
        if spec.primary {
            has_primary = true;
            has_primary_discrepancy |= rows
                .iter()
                .any(|row| row.status == StatisticStatus::OutsideReferenceInterval);
            if rows.is_empty()
                || rows.iter().any(|row| row.status == StatisticStatus::NotAssessed)
            {
                required_complete = false;
            }
        }
    }

    let expected = if has_primary_discrepancy {
        DomainStatus::PrimaryDiscrepancyDetected
    } else if !has_primary || !required_complete {
        DomainStatus::NotAssessed
    } else {
        DomainStatus::NoPrimaryDiscrepancyDetected
    };

    if expected != assessment.domain_status
        || assessment.required_evidence_complete != required_complete
    {
        return Err(CalibrationProfileError::DomainStatusMismatch);
    }
    Ok(())
}

/// Build a calibration profile from multiple exact-head split-predictive
/// adequacy assessments that reuse the same observations and predictive draws.
pub fn assess_calibration_profile(
    evidence: &CalibrationProfileEvidence,
) -> Result<CalibrationProfileAssessment, CalibrationProfileError> {
    evidence.registry.validate()?;
    evidence.manifest.validate()?;

    if evidence.model_digest.trim().is_empty() {
        return Err(CalibrationProfileError::MissingModelDigest);
    }
    if evidence.assessments.is_empty() {
        return Err(CalibrationProfileError::MissingAssessments);
    }
    if evidence.assessments.len() < 2 {
        return Err(CalibrationProfileError::TooFewCentralMasses);
    }
    let role = evidence.assessments[0].role;
    if role == SplitRole::Calibration {
        return Err(CalibrationProfileError::InvalidRole);
    }
    if evidence.assessments[0].predictive_kind != PredictiveKind::Split {
        return Err(CalibrationProfileError::PriorPredictiveNotAllowed);
    }
    let independent_unit = evidence.manifest.independent_unit;
    let expected_units = units_for(&evidence.manifest, role);
    if expected_units.is_empty() {
        return Err(CalibrationProfileError::InvalidRole);
    }
    if evidence.registry.statistics.iter().any(|spec| spec.role != role) {
        let spec = evidence.registry.statistics.iter().find(|spec| spec.role != role)
            .expect("mismatching statistic exists");
        return Err(CalibrationProfileError::StatisticRoleMismatch {
            statistic_id: spec.id.clone(),
        });
    }
    if let Some(spec) = evidence.registry.statistics.iter()
        .find(|spec| spec.independent_unit != independent_unit)
    {
        return Err(CalibrationProfileError::StatisticUnitKindMismatch {
            statistic_id: spec.id.clone(),
        });
    }
    if !evidence.registry.statistics.iter().any(|spec| spec.primary) {
        return Err(CalibrationProfileError::NoPrimaryStatistic);
    }

    let mut central_masses = Vec::with_capacity(evidence.assessments.len());
    for (index, assessment) in evidence.assessments.iter().enumerate() {
        if assessment.role != role {
            return Err(CalibrationProfileError::AssessmentRoleMismatch);
        }
        if assessment.predictive_kind != PredictiveKind::Split {
            return Err(CalibrationProfileError::PriorPredictiveNotAllowed);
        }
        if assessment.registry_version != evidence.registry.version {
            return Err(CalibrationProfileError::AssessmentRegistryMismatch);
        }
        if assessment.split_manifest_id != evidence.manifest.id
            || assessment.split_manifest_digest != evidence.manifest.manifest_digest
        {
            return Err(CalibrationProfileError::AssessmentManifestMismatch);
        }
        if !assessment.central_mass.is_finite()
            || assessment.central_mass <= 0.0
            || assessment.central_mass >= 1.0
        {
            return Err(CalibrationProfileError::InvalidCentralMass);
        }
        if index > 0 && assessment.central_mass <= evidence.assessments[index - 1].central_mass {
            return Err(CalibrationProfileError::CentralMassOrderOrDuplicate);
        }
        validate_assessment_domain_status(assessment, &evidence.registry)?;
        central_masses.push(assessment.central_mass);
    }

    let mut baseline_rows = BTreeMap::<(String, String), (&crate::model_adequacy::StatisticAssessment, PredictiveIdentity, ObservationIdentity)>::new();
    let mut baseline_rosters = BTreeMap::<String, BTreeSet<String>>::new();

    for spec in &evidence.registry.statistics {
        let mut roster = BTreeSet::new();
        for row in evidence.assessments[0]
            .statistics
            .iter()
            .filter(|row| row.statistic_id == spec.id)
        {
            if !roster.insert(row.independent_unit_id.clone()) {
                return Err(CalibrationProfileError::DuplicateStatisticRow {
                    statistic_id: spec.id.clone(),
                    unit_id: row.independent_unit_id.clone(),
                });
            }
        }
        if spec.required {
            for unit in expected_units {
                if !roster.contains(unit) {
                    return Err(CalibrationProfileError::MissingRequiredStatisticRow {
                        statistic_id: spec.id.clone(),
                        unit_id: unit.clone(),
                    });
                }
            }
            if roster.len() != expected_units.len() {
                return Err(CalibrationProfileError::RowRosterMismatch {
                    statistic_id: spec.id.clone(),
                    unit_id: String::new(),
                });
            }
        }
        if roster.is_empty() && spec.required {
            return Err(CalibrationProfileError::MissingRequiredStatisticRow {
                statistic_id: spec.id.clone(),
                unit_id: String::new(),
            });
        }
        baseline_rosters.insert(spec.id.clone(), roster);
    }

    for assessment in &evidence.assessments {
        let mut coverage_seen = BTreeSet::new();
        for summary in &assessment.coverage {
            if evidence.registry.get(&summary.statistic_id).is_none() {
                return Err(CalibrationProfileError::UnknownCoverageStatistic {
                    statistic_id: summary.statistic_id.clone(),
                });
            }
            if !coverage_seen.insert(summary.statistic_id.as_str()) {
                return Err(CalibrationProfileError::DuplicateCoverageSummary {
                    statistic_id: summary.statistic_id.clone(),
                });
            }
        }

        let mut seen = BTreeSet::new();
        for row in &assessment.statistics {
            let spec = evidence.registry.get(&row.statistic_id).ok_or_else(|| {
                CalibrationProfileError::UnknownStatistic {
                    statistic_id: row.statistic_id.clone(),
                }
            })?;
            if !expected_units.iter().any(|unit| unit == &row.independent_unit_id) {
                return Err(CalibrationProfileError::UnknownIndependentUnit {
                    statistic_id: row.statistic_id.clone(),
                    unit_id: row.independent_unit_id.clone(),
                });
            }
            if row.source_family != spec.source_family
                || row.source_digest != spec.source_digest
                || row.measurement_unit != spec.measurement_unit
            {
                return Err(CalibrationProfileError::StatisticIdentityMismatch {
                    statistic_id: row.statistic_id.clone(),
                    unit_id: row.independent_unit_id.clone(),
                });
            }
            let key = (row.statistic_id.clone(), row.independent_unit_id.clone());
            if !seen.insert(key.clone()) {
                return Err(CalibrationProfileError::DuplicateStatisticRow {
                    statistic_id: row.statistic_id.clone(),
                    unit_id: row.independent_unit_id.clone(),
                });
            }
        }

        for spec in &evidence.registry.statistics {
            let roster = &baseline_rosters[&spec.id];
            let current: BTreeSet<String> = assessment
                .statistics
                .iter()
                .filter(|row| row.statistic_id == spec.id)
                .map(|row| row.independent_unit_id.clone())
                .collect();
            if &current != roster {
                return Err(CalibrationProfileError::RowRosterMismatch {
                    statistic_id: spec.id.clone(),
                    unit_id: String::new(),
                });
            }
        }
    }

    let first = &evidence.assessments[0];
    for row in &first.statistics {
        let identity = PredictiveIdentity {
            model_digest: row.model_digest.clone(),
            draw_set_digest: row.draw_set_digest.clone(),
        };
        if row.model_digest.as_deref().is_some_and(|digest| digest != evidence.model_digest) {
            return Err(CalibrationProfileError::ModelDigestMismatch {
                statistic_id: row.statistic_id.clone(),
                unit_id: row.independent_unit_id.clone(),
            });
        }
        if row.status != StatisticStatus::NotAssessed
            && (row.model_digest.as_deref() != Some(evidence.model_digest.as_str())
                || row.draw_set_digest.as_deref().is_none_or(str::is_empty))
        {
            return Err(CalibrationProfileError::InvalidStatisticAssessment {
                statistic_id: row.statistic_id.clone(),
                unit_id: row.independent_unit_id.clone(),
            });
        }
        let observation = ObservationIdentity {
            observation_digest: row.observation_digest.clone(),
            sample_size: row.sample_size,
            observed_bits: row.observed_value.map(f64::to_bits),
            measurement_standard_error_bits: row.measurement_standard_error.map(f64::to_bits),
        };
        baseline_rows.insert(
            (row.statistic_id.clone(), row.independent_unit_id.clone()),
            (row, identity, observation),
        );
    }

    for assessment in evidence.assessments.iter().skip(1) {
        for row in &assessment.statistics {
            let key = (row.statistic_id.clone(), row.independent_unit_id.clone());
            let Some((baseline, predictive_identity, observation_identity)) =
                baseline_rows.get(&key)
            else {
                return Err(CalibrationProfileError::RowRosterMismatch {
                    statistic_id: row.statistic_id.clone(),
                    unit_id: row.independent_unit_id.clone(),
                });
            };
            let current_predictive = PredictiveIdentity {
                model_digest: row.model_digest.clone(),
                draw_set_digest: row.draw_set_digest.clone(),
            };
            if &current_predictive != predictive_identity {
                if row.model_digest != baseline.model_digest {
                    return Err(CalibrationProfileError::ModelDigestMismatch {
                        statistic_id: row.statistic_id.clone(),
                        unit_id: row.independent_unit_id.clone(),
                    });
                }
                return Err(CalibrationProfileError::DrawSetDigestMismatch {
                    statistic_id: row.statistic_id.clone(),
                    unit_id: row.independent_unit_id.clone(),
                });
            }
            if row.model_digest.as_deref().is_some_and(|digest| digest != evidence.model_digest) {
                return Err(CalibrationProfileError::ModelDigestMismatch {
                    statistic_id: row.statistic_id.clone(),
                    unit_id: row.independent_unit_id.clone(),
                });
            }
            let current_observation = ObservationIdentity {
                observation_digest: row.observation_digest.clone(),
                sample_size: row.sample_size,
                observed_bits: row.observed_value.map(f64::to_bits),
                measurement_standard_error_bits: row.measurement_standard_error.map(f64::to_bits),
            };
            if &current_observation != observation_identity {
                return Err(CalibrationProfileError::ObservationIdentityMismatch {
                    statistic_id: row.statistic_id.clone(),
                    unit_id: row.independent_unit_id.clone(),
                });
            }
        }
    }

    let mut points = Vec::new();
    let mut evidence_complete = true;
    let mut primary_discrepancy = false;

    for assessment in &evidence.assessments {
        let central_mass = assessment.central_mass;
        for spec in &evidence.registry.statistics {
            let roster = &baseline_rosters[&spec.id];
            if roster.is_empty() {
                continue;
            }
            let rows: Vec<_> = roster.iter()
                .filter_map(|unit| {
                    assessment.statistics.iter().find(|row| {
                        row.statistic_id == spec.id && row.independent_unit_id == *unit
                    })
                })
                .collect();

            let expected_count = rows.len();
            let assessed_rows: Vec<_> = rows
                .iter()
                .copied()
                .filter(|row| row.status != StatisticStatus::NotAssessed)
                .collect();
            let covered_count = assessed_rows
                .iter()
                .filter(|row| row.status == StatisticStatus::WithinReferenceInterval)
                .count();
            let not_assessed_count = expected_count.saturating_sub(assessed_rows.len());

            for row in &assessed_rows {
                let (Some(observed), Some(lower), Some(upper)) =
                    (row.observed_value, row.lower_reference, row.upper_reference)
                else {
                    return Err(CalibrationProfileError::InvalidStatisticAssessment {
                        statistic_id: spec.id.clone(),
                        unit_id: row.independent_unit_id.clone(),
                    });
                };
                if !observed.is_finite()
                    || !lower.is_finite()
                    || !upper.is_finite()
                    || lower > upper
                    || row.sample_size == 0
                    || row.draw_count < spec.minimum_draws
                    || row.predictive_rank.is_none_or(|rank| {
                        !rank.is_finite() || !(0.0..=1.0).contains(&rank)
                    })
                    || row.model_digest.as_deref() != Some(evidence.model_digest.as_str())
                    || row.draw_set_digest.as_deref().is_none_or(str::is_empty)
                    || row.observation_digest.as_deref().is_none_or(str::is_empty)
                    || row.measurement_standard_error.is_some_and(|value| {
                        !value.is_finite() || value < 0.0
                    })
                {
                    return Err(CalibrationProfileError::InvalidStatisticAssessment {
                        statistic_id: spec.id.clone(),
                        unit_id: row.independent_unit_id.clone(),
                    });
                }
                let is_covered = observed >= lower && observed <= upper;
                if (row.status == StatisticStatus::WithinReferenceInterval) != is_covered {
                    return Err(CalibrationProfileError::InvalidStatisticAssessment {
                        statistic_id: spec.id.clone(),
                        unit_id: row.independent_unit_id.clone(),
                    });
                }
            }
            for row in rows.iter().filter(|row| row.status == StatisticStatus::NotAssessed) {
                if row.lower_reference.is_some()
                    || row.upper_reference.is_some()
                    || row.predictive_rank.is_some()
                {
                    return Err(CalibrationProfileError::InvalidStatisticAssessment {
                        statistic_id: spec.id.clone(),
                        unit_id: row.independent_unit_id.clone(),
                    });
                }
            }

            let coverage_summary = assessment.coverage.iter()
                .filter(|summary| summary.statistic_id == spec.id)
                .collect::<Vec<_>>();
            if coverage_summary.len() > 1 {
                return Err(CalibrationProfileError::DuplicateCoverageSummary {
                    statistic_id: spec.id.clone(),
                });
            }
            let summary = coverage_summary.first().ok_or_else(|| {
                CalibrationProfileError::MissingCoverageSummary {
                    statistic_id: spec.id.clone(),
                }
            })?;
            if summary.independent_unit != spec.independent_unit
                || summary.expected_independent_units != expected_count
                || summary.assessed_independent_units != assessed_rows.len()
                || summary.not_assessed_independent_units != not_assessed_count
                || summary.covered_independent_units != covered_count
            {
                return Err(CalibrationProfileError::CoverageSummaryMismatch {
                    statistic_id: spec.id.clone(),
                });
            }

            let empirical_coverage = if assessed_rows.is_empty() {
                None
            } else {
                Some(covered_count as f64 / assessed_rows.len() as f64)
            };
            let wilson = if assessed_rows.is_empty() {
                None
            } else {
                Some(wilson_interval_95(covered_count, assessed_rows.len()))
            };
            let coverage_summary_matches = match (
                summary.empirical_coverage,
                empirical_coverage,
                summary.wilson_lower_95,
                summary.wilson_upper_95,
                wilson,
            ) {
                (None, None, None, None, None) => true,
                (
                    Some(reported),
                    Some(computed),
                    Some(reported_lower),
                    Some(reported_upper),
                    Some((computed_lower, computed_upper)),
                ) => {
                    close_enough(reported, computed)
                        && close_enough(reported_lower, computed_lower)
                        && close_enough(reported_upper, computed_upper)
                }
                _ => false,
            };
            if !coverage_summary_matches {
                return Err(CalibrationProfileError::CoverageSummaryMismatch {
                    statistic_id: spec.id.clone(),
                });
            }

            let mean_interval_width = if assessed_rows.is_empty() {
                None
            } else {
                let total_width: f64 = assessed_rows
                    .iter()
                    .map(|row| {
                        row.upper_reference.expect("assessed upper interval")
                            - row.lower_reference.expect("assessed lower interval")
                    })
                    .sum();
                if !total_width.is_finite() {
                    return Err(CalibrationProfileError::InvalidStatisticAssessment {
                        statistic_id: spec.id.clone(),
                        unit_id: String::new(),
                    });
                }
                Some(total_width / assessed_rows.len() as f64)
            };
            let nominal = central_mass;
            let status = match wilson {
                None => CalibrationPointStatus::NotAssessed,
                Some((_, upper)) if upper < nominal => {
                    CalibrationPointStatus::UndercoverageDetected
                }
                Some((lower, _)) if lower > nominal => {
                    CalibrationPointStatus::OvercoverageDetected
                }
                Some(_) if not_assessed_count > 0 => CalibrationPointStatus::NotAssessed,
                Some(_) => CalibrationPointStatus::CompatibleWithNominal,
            };
            if spec.primary
                && matches!(
                    status,
                    CalibrationPointStatus::UndercoverageDetected
                        | CalibrationPointStatus::OvercoverageDetected
                )
            {
                primary_discrepancy = true;
            }
            if status == CalibrationPointStatus::NotAssessed {
                evidence_complete = false;
            }

            points.push(CalibrationProfilePoint {
                statistic_id: spec.id.clone(),
                construct: spec.construct.clone(),
                measurement_unit: spec.measurement_unit.clone(),
                population: spec.population.clone(),
                context: spec.context.clone(),
                source_family: spec.source_family.clone(),
                source_digest: spec.source_digest.clone(),
                independent_unit: spec.independent_unit,
                primary: spec.primary,
                required: spec.required,
                central_mass,
                nominal_coverage: nominal,
                expected_independent_units: expected_count,
                assessed_independent_units: assessed_rows.len(),
                covered_independent_units: covered_count,
                not_assessed_independent_units: not_assessed_count,
                empirical_coverage,
                signed_calibration_gap: empirical_coverage.map(|coverage| coverage - nominal),
                wilson_lower_95: wilson.map(|interval| interval.0),
                wilson_upper_95: wilson.map(|interval| interval.1),
                mean_interval_width,
                status,
            });
        }
    }

    let claim_ceiling = if primary_discrepancy {
        CalibrationClaimCeiling::CalibrationDiscrepancyDetected
    } else if !evidence_complete {
        CalibrationClaimCeiling::NotAssessed
    } else {
        CalibrationClaimCeiling::NoDiscrepancyDetectedAtDeclaredLevels
    };

    Ok(CalibrationProfileAssessment {
        registry_version: evidence.registry.version.clone(),
        model_digest: evidence.model_digest.clone(),
        split_manifest_id: evidence.manifest.id.clone(),
        split_manifest_digest: evidence.manifest.manifest_digest.clone(),
        calibration_data_digest: evidence.manifest.calibration_data_digest.clone(),
        role,
        predictive_kind: PredictiveKind::Split,
        independent_unit,
        central_masses,
        evidence_complete,
        claim_ceiling,
        points,
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_adequacy::{
        assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
        StatisticSpec,
    };

    const MODEL: &str = "sha256-calibration-profile-model";
    const CALIBRATION: &str = "sha256-calibration-profile-calibration";
    const MASSES: [f64; 4] = [0.50, 0.80, 0.90, 0.95];

    fn registry() -> StatisticRegistry {
        StatisticRegistry {
            version: "calibration-profile-v1".to_string(),
            statistics: vec![
                StatisticSpec {
                    id: "service-coverage-group-a".to_string(),
                    construct: "essential service coverage group A".to_string(),
                    measurement_unit: "coverage_fraction".to_string(),
                    population: "group-a".to_string(),
                    context: "synthetic-calibration-fixture".to_string(),
                    source_family: "synthetic-service-measurements".to_string(),
                    source_digest: "sha256-calibration-observations".to_string(),
                    independent_unit: IndependentUnitKind::Region,
                    role: SplitRole::Validation,
                    primary: true,
                    required: true,
                    minimum_draws: 5,
                },
                StatisticSpec {
                    id: "service-coverage-group-b".to_string(),
                    construct: "essential service coverage group B".to_string(),
                    measurement_unit: "coverage_fraction".to_string(),
                    population: "group-b".to_string(),
                    context: "synthetic-calibration-fixture".to_string(),
                    source_family: "synthetic-service-measurements".to_string(),
                    source_digest: "sha256-calibration-observations".to_string(),
                    independent_unit: IndependentUnitKind::Region,
                    role: SplitRole::Validation,
                    primary: true,
                    required: true,
                    minimum_draws: 101,
                },
            ],
        }
    }

    fn manifest() -> SplitManifest {
        SplitManifest {
            id: "calibration-profile-split-v1".to_string(),
            independent_unit: IndependentUnitKind::Region,
            calibration_data_digest: CALIBRATION.to_string(),
            manifest_digest: "sha256-calibration-profile-manifest".to_string(),
            calibration_units: vec!["calibration-region".to_string()],
            validation_units: (0..200)
                .map(|index| format!("region-{index:03}"))
                .collect(),
            challenge_units: Vec::new(),
        }
    }

    fn group_a_ranks() -> Vec<f64> {
        let mut ranks = Vec::with_capacity(200);
        ranks.extend(std::iter::repeat_n(0.50, 80));
        ranks.extend(std::iter::repeat_n(0.15, 20));
        ranks.extend(std::iter::repeat_n(0.85, 20));
        ranks.extend(std::iter::repeat_n(0.075, 10));
        ranks.extend(std::iter::repeat_n(0.925, 10));
        ranks.extend(std::iter::repeat_n(0.0375, 10));
        ranks.extend(std::iter::repeat_n(0.9625, 10));
        ranks.extend(std::iter::repeat_n(0.01, 20));
        ranks.extend(std::iter::repeat_n(0.99, 20));
        assert_eq!(ranks.len(), 200);
        ranks
    }

    fn group_b_ranks() -> Vec<f64> {
        let mut ranks = Vec::with_capacity(200);
        ranks.extend(std::iter::repeat_n(0.50, 120));
        ranks.extend(std::iter::repeat_n(0.15, 30));
        ranks.extend(std::iter::repeat_n(0.85, 30));
        ranks.extend(std::iter::repeat_n(0.075, 8));
        ranks.extend(std::iter::repeat_n(0.925, 8));
        ranks.extend(std::iter::repeat_n(0.0375, 1));
        ranks.extend(std::iter::repeat_n(0.9625, 1));
        ranks.extend(std::iter::repeat_n(0.01, 1));
        ranks.extend(std::iter::repeat_n(0.99, 1));
        assert_eq!(ranks.len(), 200);
        ranks
    }

    fn evidence() -> CalibrationProfileEvidence {
        let registry = registry();
        let manifest = manifest();
        let ranks = [
            ("service-coverage-group-a", group_a_ranks()),
            ("service-coverage-group-b", group_b_ranks()),
        ];
        let mut observations = Vec::with_capacity(400);
        let mut draws = Vec::with_capacity(2_000);

        for (statistic_id, statistic_ranks) in &ranks {
            for (index, rank) in statistic_ranks.iter().enumerate() {
                let unit_id = format!("region-{index:03}");
                observations.push(ObservedStatistic {
                    statistic_id: (*statistic_id).to_string(),
                    independent_unit_id: unit_id.clone(),
                    measurement_unit: "coverage_fraction".to_string(),
                    source_digest: "sha256-calibration-observations".to_string(),
                    sample_size: 5,
                    value: rank * 100.0,
                    measurement_standard_error: Some(0.01),
                    observation_digest: format!("sha256-observation-{statistic_id}-{unit_id}"),
                });
                for (draw_index, draw_value) in [0.0, 25.0, 50.0, 75.0, 100.0].iter().enumerate() {
                    draws.push(PredictiveDraw {
                        kind: PredictiveKind::Split,
                        statistic_id: (*statistic_id).to_string(),
                        independent_unit_id: unit_id.clone(),
                        sample_size: 101,
                        draw_index: draw_index as u64,
                        value: *draw_value,
                        model_digest: MODEL.to_string(),
                        draw_set_digest: format!("sha256-draws-{statistic_id}-{unit_id}"),
                        calibration_data_digest: Some(CALIBRATION.to_string()),
                    });
                }
            }
        }

        let assessments = MASSES
            .iter()
            .map(|central_mass| {
                assess_predictive_partition(
                    &registry,
                    &manifest,
                    &observations,
                    &draws,
                    SplitRole::Validation,
                    PredictiveKind::Split,
                    *central_mass,
                )
                .expect("valid synthetic split-predictive calibration assessment")
            })
            .collect();

        CalibrationProfileEvidence {
            model_digest: MODEL.to_string(),
            registry,
            manifest,
            assessments,
        }
    }

    #[test]
    fn detects_opposite_conditional_calibration_failures_at_all_nominal_levels() {
        let report = assess_calibration_profile(&evidence()).expect("calibration profile");
        assert_eq!(report.claim_ceiling, CalibrationClaimCeiling::CalibrationDiscrepancyDetected);
        assert!(report.evidence_complete);
        assert_eq!(report.points.len(), 8);
        assert_eq!(report.central_masses, MASSES.to_vec());

        let group_a: Vec<_> = report.points.iter()
            .filter(|point| point.statistic_id == "service-coverage-group-a")
            .collect();
        let group_b: Vec<_> = report.points.iter()
            .filter(|point| point.statistic_id == "service-coverage-group-b")
            .collect();
        assert_eq!(group_a.len(), 4);
        assert_eq!(group_b.len(), 4);

        let expected_a = [0.40, 0.60, 0.70, 0.80];
        let expected_b = [0.60, 0.90, 0.98, 0.99];
        for (index, point) in group_a.iter().enumerate() {
            assert_eq!(point.expected_independent_units, 200);
            assert_eq!(point.assessed_independent_units, 200);
            assert_eq!(point.not_assessed_independent_units, 0);
            assert_eq!(point.status, CalibrationPointStatus::UndercoverageDetected);
            assert!((point.empirical_coverage.expect("coverage") - expected_a[index]).abs() < 1e-12);
            assert!(point.wilson_lower_95.is_some());
            assert!(point.wilson_upper_95.is_some());
        }
        for (index, point) in group_b.iter().enumerate() {
            assert_eq!(point.expected_independent_units, 200);
            assert_eq!(point.assessed_independent_units, 200);
            assert_eq!(point.not_assessed_independent_units, 0);
            assert_eq!(point.status, CalibrationPointStatus::OvercoverageDetected);
            assert!((point.empirical_coverage.expect("coverage") - expected_b[index]).abs() < 1e-12);
            assert!(point.wilson_lower_95.is_some());
            assert!(point.wilson_upper_95.is_some());
        }

        let widths: Vec<_> = group_a.iter()
            .map(|point| point.mean_interval_width.expect("sharpness"))
            .collect();
        assert!((widths[0] - 50.0).abs() < 1e-10);
        assert!((widths[1] - 80.0).abs() < 1e-10);
        assert!((widths[2] - 90.0).abs() < 1e-10);
        assert!((widths[3] - 95.0).abs() < 1e-10);
    }

    #[test]
    fn rejects_changed_draw_set_between_nominal_levels() {
        let mut candidate = evidence();
        candidate.assessments[2].statistics[0].draw_set_digest =
            Some("sha256-different-draws".to_string());
        assert!(matches!(
            assess_calibration_profile(&candidate),
            Err(CalibrationProfileError::DrawSetDigestMismatch { .. })
        ));
    }

    #[test]
    fn rejects_changed_model_between_nominal_levels() {
        let mut candidate = evidence();
        candidate.assessments[1].statistics[0].model_digest =
            Some("sha256-adapted-model".to_string());
        assert!(matches!(
            assess_calibration_profile(&candidate),
            Err(CalibrationProfileError::ModelDigestMismatch { .. })
        ));
    }

    #[test]
    fn rejects_changed_observation_between_nominal_levels() {
        let mut candidate = evidence();
        candidate.assessments[1].statistics[0].observed_value = Some(42.0);
        assert!(matches!(
            assess_calibration_profile(&candidate),
            Err(CalibrationProfileError::ObservationIdentityMismatch { .. })
        ));
    }

    #[test]
    fn rejects_tampered_coverage_summary() {
        let mut candidate = evidence();
        candidate.assessments[1].coverage[0].covered_independent_units += 1;
        assert!(matches!(
            assess_calibration_profile(&candidate),
            Err(CalibrationProfileError::CoverageSummaryMismatch { .. })
        ));
    }

    #[test]
    fn rejects_prior_predictive_and_duplicate_central_masses() {
        let mut prior = evidence();
        prior.assessments[1].predictive_kind = PredictiveKind::Prior;
        assert_eq!(
            assess_calibration_profile(&prior).unwrap_err(),
            CalibrationProfileError::PriorPredictiveNotAllowed
        );

        let mut duplicate = evidence();
        duplicate.assessments[2].central_mass = duplicate.assessments[1].central_mass;
        assert_eq!(
            assess_calibration_profile(&duplicate).unwrap_err(),
            CalibrationProfileError::CentralMassOrderOrDuplicate
        );
    }

    #[test]
    fn missing_required_unit_is_rejected() {
        let mut candidate = evidence();
        candidate.assessments[2].statistics.retain(|row| {
            !(row.statistic_id == "service-coverage-group-a"
                && row.independent_unit_id == "region-000")
        });
        assert!(matches!(
            assess_calibration_profile(&candidate),
            Err(CalibrationProfileError::RowRosterMismatch { .. })
                | Err(CalibrationProfileError::MissingRequiredStatisticRow { .. })
        ));
    }
}
