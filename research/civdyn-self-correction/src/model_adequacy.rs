// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Purpose-bounded model-adequacy diagnostics for CIVDYN.
//!
//! This module deliberately reports statistic/stratum results separately. It
//! does not produce a universal adequacy score, a posterior-predictive p-value,
//! or proof that a model is true. A passing result means only that no
//! discrepancy was detected by the declared finite reference for the
//! registered held-out targets.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SplitRole {
    Calibration,
    Validation,
    Challenge,
}

impl SplitRole {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Calibration => "calibration",
            Self::Validation => "validation",
            Self::Challenge => "challenge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndependentUnitKind {
    Participant,
    Household,
    Community,
    Region,
    Site,
    Network,
    Cohort,
    TimeBlock,
}

impl IndependentUnitKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Participant => "participant",
            Self::Household => "household",
            Self::Community => "community",
            Self::Region => "region",
            Self::Site => "site",
            Self::Network => "network",
            Self::Cohort => "cohort",
            Self::TimeBlock => "time_block",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatisticSpec {
    pub id: String,
    pub construct: String,
    pub measurement_unit: String,
    pub population: String,
    pub context: String,
    pub source_family: String,
    pub source_digest: String,
    pub independent_unit: IndependentUnitKind,
    pub role: SplitRole,
    pub primary: bool,
    pub required: bool,
    pub minimum_draws: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatisticRegistry {
    pub version: String,
    pub statistics: Vec<StatisticSpec>,
}

impl StatisticRegistry {
    pub fn validate(&self) -> Result<(), AdequacyError> {
        if self.version.trim().is_empty() || self.statistics.is_empty() {
            return Err(AdequacyError::InvalidRegistry);
        }

        let mut ids = BTreeSet::new();
        for statistic in &self.statistics {
            if statistic.id.trim().is_empty()
                || statistic.construct.trim().is_empty()
                || statistic.measurement_unit.trim().is_empty()
                || statistic.population.trim().is_empty()
                || statistic.context.trim().is_empty()
                || statistic.source_family.trim().is_empty()
                || statistic.source_digest.trim().is_empty()
                || statistic.minimum_draws == 0
            {
                return Err(AdequacyError::InvalidStatistic {
                    statistic_id: statistic.id.clone(),
                });
            }
            if statistic.primary && !statistic.required {
                return Err(AdequacyError::PrimaryStatisticMustBeRequired {
                    statistic_id: statistic.id.clone(),
                });
            }
            if !ids.insert(statistic.id.as_str()) {
                return Err(AdequacyError::DuplicateStatistic {
                    statistic_id: statistic.id.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&StatisticSpec> {
        self.statistics.iter().find(|statistic| statistic.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitManifest {
    pub id: String,
    pub independent_unit: IndependentUnitKind,
    pub calibration_data_digest: String,
    pub manifest_digest: String,
    pub calibration_units: Vec<String>,
    pub validation_units: Vec<String>,
    pub challenge_units: Vec<String>,
}

impl SplitManifest {
    pub fn validate(&self) -> Result<(), AdequacyError> {
        if self.id.trim().is_empty()
            || self.calibration_data_digest.trim().is_empty()
            || self.manifest_digest.trim().is_empty()
            || self.calibration_units.is_empty()
            || self.validation_units.is_empty()
        {
            return Err(AdequacyError::InvalidSplitManifest);
        }

        let mut seen = BTreeSet::new();
        for unit_id in self
            .calibration_units
            .iter()
            .chain(self.validation_units.iter())
            .chain(self.challenge_units.iter())
        {
            if unit_id.trim().is_empty() || !seen.insert(unit_id.as_str()) {
                return Err(AdequacyError::SplitUnitOverlap {
                    unit_id: unit_id.clone(),
                });
            }
        }
        Ok(())
    }

    fn units_for(&self, role: SplitRole) -> &[String] {
        match role {
            SplitRole::Calibration => &self.calibration_units,
            SplitRole::Validation => &self.validation_units,
            SplitRole::Challenge => &self.challenge_units,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservedStatistic {
    pub statistic_id: String,
    pub independent_unit_id: String,
    pub measurement_unit: String,
    pub source_digest: String,
    pub sample_size: usize,
    pub value: f64,
    pub measurement_standard_error: Option<f64>,
    pub observation_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictiveKind {
    Prior,
    Split,
}

impl PredictiveKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Prior => "prior",
            Self::Split => "split_predictive",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictiveDraw {
    pub kind: PredictiveKind,
    pub statistic_id: String,
    pub independent_unit_id: String,
    pub sample_size: usize,
    pub draw_index: u64,
    pub value: f64,
    pub model_digest: String,
    pub draw_set_digest: String,
    /// Present only for split-predictive draws, and must match the frozen
    /// calibration data identity in the split manifest.
    pub calibration_data_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticStatus {
    WithinReferenceInterval,
    OutsideReferenceInterval,
    NotAssessed,
}

impl StatisticStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::WithinReferenceInterval => "within_reference_interval",
            Self::OutsideReferenceInterval => "outside_reference_interval",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainStatus {
    PrimaryDiscrepancyDetected,
    NoPrimaryDiscrepancyDetected,
    NotAssessed,
}

impl DomainStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::PrimaryDiscrepancyDetected => "primary_discrepancy_detected",
            Self::NoPrimaryDiscrepancyDetected => "no_primary_discrepancy_detected",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatisticAssessment {
    pub statistic_id: String,
    pub source_family: String,
    pub source_digest: String,
    pub observation_digest: Option<String>,
    pub model_digest: Option<String>,
    pub draw_set_digest: Option<String>,
    pub independent_unit_id: String,
    pub measurement_unit: String,
    pub sample_size: usize,
    pub observed_value: Option<f64>,
    /// Kept separate from structural model discrepancy.
    pub measurement_standard_error: Option<f64>,
    pub lower_reference: Option<f64>,
    pub upper_reference: Option<f64>,
    /// Empirical mid-rank in [0, 1]. This is descriptive, not a p-value.
    pub predictive_rank: Option<f64>,
    pub draw_count: usize,
    pub status: StatisticStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageSummary {
    pub statistic_id: String,
    pub independent_unit: IndependentUnitKind,
    pub expected_independent_units: usize,
    pub assessed_independent_units: usize,
    pub not_assessed_independent_units: usize,
    pub covered_independent_units: usize,
    pub empirical_coverage: Option<f64>,
    pub wilson_lower_95: Option<f64>,
    pub wilson_upper_95: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdequacyAssessment {
    pub registry_version: String,
    pub split_manifest_id: String,
    pub split_manifest_digest: String,
    pub role: SplitRole,
    pub predictive_kind: PredictiveKind,
    pub central_mass: f64,
    pub domain_status: DomainStatus,
    pub required_evidence_complete: bool,
    pub statistics: Vec<StatisticAssessment>,
    /// Coverage is calculated across independent units, separately per statistic.
    pub coverage: Vec<CoverageSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdequacyError {
    InvalidRegistry,
    InvalidStatistic {
        statistic_id: String,
    },
    DuplicateStatistic {
        statistic_id: String,
    },
    PrimaryStatisticMustBeRequired {
        statistic_id: String,
    },
    InvalidSplitManifest,
    SplitUnitOverlap {
        unit_id: String,
    },
    InvalidIntervalMass,
    CalibrationPartitionCannotBeEvaluated,
    UnknownStatistic {
        statistic_id: String,
    },
    StatisticRoleMismatch {
        statistic_id: String,
    },
    UnitKindMismatch {
        statistic_id: String,
    },
    UnknownIndependentUnit {
        unit_id: String,
    },
    DuplicateObservation {
        statistic_id: String,
        unit_id: String,
    },
    InvalidObservation {
        statistic_id: String,
        unit_id: String,
    },
    InvalidDraw {
        statistic_id: String,
        unit_id: String,
    },
    DuplicateDraw {
        statistic_id: String,
        unit_id: String,
        draw_index: u64,
    },
    CalibrationDigestMismatch {
        statistic_id: String,
        unit_id: String,
    },
    MixedModelDrawSet {
        statistic_id: String,
        unit_id: String,
    },
}

impl fmt::Display for AdequacyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AdequacyError {}

/// Evaluate one declared held-out partition using sample-size-matched draws.
///
/// Prior-predictive evaluation requires draws with kind = Prior and no
/// calibration digest. Split-predictive evaluation requires draws conditioned
/// on exactly the calibration digest frozen in manifest.
///
/// The returned interval is an empirical central reference interval. The
/// predictive rank is a descriptive mid-rank. Neither is a universally
/// calibrated frequentist test; callers must not relabel it as an SPC p-value.
pub fn assess_predictive_partition(
    registry: &StatisticRegistry,
    manifest: &SplitManifest,
    observations: &[ObservedStatistic],
    draws: &[PredictiveDraw],
    role: SplitRole,
    kind: PredictiveKind,
    central_mass: f64,
) -> Result<AdequacyAssessment, AdequacyError> {
    registry.validate()?;
    manifest.validate()?;

    if role == SplitRole::Calibration {
        return Err(AdequacyError::CalibrationPartitionCannotBeEvaluated);
    }
    if !central_mass.is_finite() || central_mass <= 0.0 || central_mass >= 1.0 {
        return Err(AdequacyError::InvalidIntervalMass);
    }

    let expected_units = manifest.units_for(role);
    let expected_unit_set: BTreeSet<&str> = expected_units.iter().map(String::as_str).collect();

    let mut observed_by_key: BTreeMap<(String, String), &ObservedStatistic> = BTreeMap::new();
    for observation in observations {
        let statistic = registry.get(&observation.statistic_id).ok_or_else(|| {
            AdequacyError::UnknownStatistic {
                statistic_id: observation.statistic_id.clone(),
            }
        })?;
        if statistic.role != role {
            return Err(AdequacyError::StatisticRoleMismatch {
                statistic_id: observation.statistic_id.clone(),
            });
        }
        if statistic.independent_unit != manifest.independent_unit {
            return Err(AdequacyError::UnitKindMismatch {
                statistic_id: observation.statistic_id.clone(),
            });
        }
        if !expected_unit_set.contains(observation.independent_unit_id.as_str()) {
            return Err(AdequacyError::UnknownIndependentUnit {
                unit_id: observation.independent_unit_id.clone(),
            });
        }
        if observation.measurement_unit != statistic.measurement_unit
            || observation.source_digest != statistic.source_digest
            || observation.sample_size == 0
            || !observation.value.is_finite()
            || observation
                .measurement_standard_error
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            || observation.observation_digest.trim().is_empty()
        {
            return Err(AdequacyError::InvalidObservation {
                statistic_id: observation.statistic_id.clone(),
                unit_id: observation.independent_unit_id.clone(),
            });
        }
        let key = (
            observation.statistic_id.clone(),
            observation.independent_unit_id.clone(),
        );
        if observed_by_key.insert(key, observation).is_some() {
            return Err(AdequacyError::DuplicateObservation {
                statistic_id: observation.statistic_id.clone(),
                unit_id: observation.independent_unit_id.clone(),
            });
        }
    }

    let mut draw_indices = BTreeSet::new();
    let mut draw_set_identities: BTreeMap<(String, String, usize), (String, String)> =
        BTreeMap::new();
    for draw in draws.iter().filter(|draw| draw.kind == kind) {
        let statistic =
            registry
                .get(&draw.statistic_id)
                .ok_or_else(|| AdequacyError::UnknownStatistic {
                    statistic_id: draw.statistic_id.clone(),
                })?;
        if statistic.role != role {
            return Err(AdequacyError::StatisticRoleMismatch {
                statistic_id: draw.statistic_id.clone(),
            });
        }
        if statistic.independent_unit != manifest.independent_unit {
            return Err(AdequacyError::UnitKindMismatch {
                statistic_id: draw.statistic_id.clone(),
            });
        }
        if !expected_unit_set.contains(draw.independent_unit_id.as_str())
            || draw.sample_size == 0
            || !draw.value.is_finite()
            || draw.model_digest.trim().is_empty()
            || draw.draw_set_digest.trim().is_empty()
        {
            return Err(AdequacyError::InvalidDraw {
                statistic_id: draw.statistic_id.clone(),
                unit_id: draw.independent_unit_id.clone(),
            });
        }

        match kind {
            PredictiveKind::Prior if draw.calibration_data_digest.is_some() => {
                return Err(AdequacyError::InvalidDraw {
                    statistic_id: draw.statistic_id.clone(),
                    unit_id: draw.independent_unit_id.clone(),
                });
            }
            PredictiveKind::Split
                if draw.calibration_data_digest.as_deref()
                    != Some(manifest.calibration_data_digest.as_str()) =>
            {
                return Err(AdequacyError::CalibrationDigestMismatch {
                    statistic_id: draw.statistic_id.clone(),
                    unit_id: draw.independent_unit_id.clone(),
                });
            }
            _ => {}
        }

        let key = (
            draw.statistic_id.clone(),
            draw.independent_unit_id.clone(),
            draw.sample_size,
        );
        if !draw_indices.insert((
            draw.statistic_id.clone(),
            draw.independent_unit_id.clone(),
            draw.sample_size,
            draw.draw_index,
        )) {
            return Err(AdequacyError::DuplicateDraw {
                statistic_id: draw.statistic_id.clone(),
                unit_id: draw.independent_unit_id.clone(),
                draw_index: draw.draw_index,
            });
        }

        let identity = (draw.model_digest.clone(), draw.draw_set_digest.clone());
        if let Some(previous) = draw_set_identities.get(&key) {
            if previous != &identity {
                return Err(AdequacyError::MixedModelDrawSet {
                    statistic_id: draw.statistic_id.clone(),
                    unit_id: draw.independent_unit_id.clone(),
                });
            }
        } else {
            draw_set_identities.insert(key, identity);
        }
    }

    let mut results = Vec::new();
    for statistic in registry.statistics.iter().filter(|spec| spec.role == role) {
        let units = if statistic.required {
            expected_units
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        } else {
            observations
                .iter()
                .filter(|observation| observation.statistic_id == statistic.id)
                .map(|observation| observation.independent_unit_id.as_str())
                .collect::<Vec<_>>()
        };

        for unit_id in units {
            let key = (statistic.id.clone(), unit_id.to_string());
            let observation = observed_by_key.get(&key).copied();
            let Some(observation) = observation else {
                results.push(StatisticAssessment {
                    statistic_id: statistic.id.clone(),
                    source_family: statistic.source_family.clone(),
                    source_digest: statistic.source_digest.clone(),
                    observation_digest: None,
                    model_digest: None,
                    draw_set_digest: None,
                    independent_unit_id: unit_id.to_string(),
                    measurement_unit: statistic.measurement_unit.clone(),
                    sample_size: 0,
                    observed_value: None,
                    measurement_standard_error: None,
                    lower_reference: None,
                    upper_reference: None,
                    predictive_rank: None,
                    draw_count: 0,
                    status: StatisticStatus::NotAssessed,
                });
                continue;
            };

            let matching_draws: Vec<&PredictiveDraw> = draws
                .iter()
                .filter(|draw| {
                    draw.kind == kind
                        && draw.statistic_id == statistic.id
                        && draw.independent_unit_id == unit_id
                        && draw.sample_size == observation.sample_size
                })
                .collect();
            let model_digest = matching_draws.first().map(|draw| draw.model_digest.clone());
            let draw_set_digest = matching_draws
                .first()
                .map(|draw| draw.draw_set_digest.clone());
            let mut values: Vec<f64> = matching_draws.iter().map(|draw| draw.value).collect();
            values.sort_by(f64::total_cmp);

            if values.len() < statistic.minimum_draws {
                results.push(StatisticAssessment {
                    statistic_id: statistic.id.clone(),
                    source_family: statistic.source_family.clone(),
                    source_digest: statistic.source_digest.clone(),
                    observation_digest: Some(observation.observation_digest.clone()),
                    model_digest: model_digest.clone(),
                    draw_set_digest: draw_set_digest.clone(),
                    independent_unit_id: unit_id.to_string(),
                    measurement_unit: statistic.measurement_unit.clone(),
                    sample_size: observation.sample_size,
                    observed_value: Some(observation.value),
                    measurement_standard_error: observation.measurement_standard_error,
                    lower_reference: None,
                    upper_reference: None,
                    predictive_rank: None,
                    draw_count: values.len(),
                    status: StatisticStatus::NotAssessed,
                });
                continue;
            }

            let tail_mass = (1.0 - central_mass) / 2.0;
            let lower = quantile(&values, tail_mass);
            let upper = quantile(&values, 1.0 - tail_mass);
            let rank = mid_rank(&values, observation.value);
            let status = if observation.value >= lower && observation.value <= upper {
                StatisticStatus::WithinReferenceInterval
            } else {
                StatisticStatus::OutsideReferenceInterval
            };

            results.push(StatisticAssessment {
                statistic_id: statistic.id.clone(),
                source_family: statistic.source_family.clone(),
                source_digest: statistic.source_digest.clone(),
                observation_digest: Some(observation.observation_digest.clone()),
                model_digest,
                draw_set_digest,
                independent_unit_id: unit_id.to_string(),
                measurement_unit: statistic.measurement_unit.clone(),
                sample_size: observation.sample_size,
                observed_value: Some(observation.value),
                measurement_standard_error: observation.measurement_standard_error,
                lower_reference: Some(lower),
                upper_reference: Some(upper),
                predictive_rank: Some(rank),
                draw_count: values.len(),
                status,
            });
        }
    }

    let primary: Vec<&StatisticAssessment> = results
        .iter()
        .filter(|result| {
            registry
                .get(&result.statistic_id)
                .is_some_and(|spec| spec.primary)
        })
        .collect();
    let required_statistics: Vec<&StatisticSpec> = registry
        .statistics
        .iter()
        .filter(|spec| spec.role == role && spec.required)
        .collect();
    let required_evidence_complete = !required_statistics.is_empty()
        && !expected_units.is_empty()
        && required_statistics.iter().all(|statistic| {
            let statistic_results: Vec<&StatisticAssessment> = results
                .iter()
                .filter(|result| result.statistic_id == statistic.id)
                .collect();
            statistic_results.len() == expected_units.len()
                && statistic_results
                    .iter()
                    .all(|result| result.status != StatisticStatus::NotAssessed)
        });

    let domain_status = if primary
        .iter()
        .any(|result| result.status == StatisticStatus::OutsideReferenceInterval)
    {
        DomainStatus::PrimaryDiscrepancyDetected
    } else if primary.is_empty()
        || !required_evidence_complete
        || primary
            .iter()
            .any(|result| result.status == StatisticStatus::NotAssessed)
    {
        DomainStatus::NotAssessed
    } else {
        DomainStatus::NoPrimaryDiscrepancyDetected
    };

    let mut coverage = Vec::new();
    for statistic in registry.statistics.iter().filter(|spec| spec.role == role) {
        let statistic_results: Vec<&StatisticAssessment> = results
            .iter()
            .filter(|result| result.statistic_id == statistic.id)
            .collect();
        if statistic_results.is_empty() {
            continue;
        }

        let expected_units = if statistic.required {
            expected_units.len()
        } else {
            statistic_results.len()
        };
        let assessed_units = statistic_results
            .iter()
            .filter(|result| result.status != StatisticStatus::NotAssessed)
            .count();
        let not_assessed_units = expected_units.saturating_sub(assessed_units);
        let covered_units = statistic_results
            .iter()
            .filter(|result| result.status == StatisticStatus::WithinReferenceInterval)
            .count();
        let empirical_coverage =
            (assessed_units > 0).then_some(covered_units as f64 / assessed_units as f64);
        let (wilson_lower_95, wilson_upper_95) = if assessed_units > 0 {
            let (lower, upper) = wilson_interval_95(covered_units, assessed_units);
            (Some(lower), Some(upper))
        } else {
            (None, None)
        };

        coverage.push(CoverageSummary {
            statistic_id: statistic.id.clone(),
            independent_unit: statistic.independent_unit,
            expected_independent_units: expected_units,
            assessed_independent_units: assessed_units,
            not_assessed_independent_units: not_assessed_units,
            covered_independent_units: covered_units,
            empirical_coverage,
            wilson_lower_95,
            wilson_upper_95,
        });
    }

    Ok(AdequacyAssessment {
        registry_version: registry.version.clone(),
        split_manifest_id: manifest.id.clone(),
        split_manifest_digest: manifest.manifest_digest.clone(),
        role,
        predictive_kind: kind,
        central_mass,
        domain_status,
        required_evidence_complete,
        statistics: results,
        coverage,
    })
}

fn quantile(sorted_values: &[f64], probability: f64) -> f64 {
    debug_assert!(!sorted_values.is_empty());
    let position = probability.clamp(0.0, 1.0) * (sorted_values.len() - 1) as f64;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    if lower_index == upper_index {
        sorted_values[lower_index]
    } else {
        let weight = position - lower_index as f64;
        sorted_values[lower_index] * (1.0 - weight) + sorted_values[upper_index] * weight
    }
}

fn wilson_interval_95(successes: usize, total: usize) -> (f64, f64) {
    debug_assert!(total > 0 && successes <= total);
    let z = 1.959_963_984_540_054_f64;
    let n = total as f64;
    let proportion = successes as f64 / n;
    let z_squared = z * z;
    let denominator = 1.0 + z_squared / n;
    let center = (proportion + z_squared / (2.0 * n)) / denominator;
    let half_width =
        z * (proportion * (1.0 - proportion) / n + z_squared / (4.0 * n * n)).sqrt() / denominator;
    (
        (center - half_width).max(0.0),
        (center + half_width).min(1.0),
    )
}

fn mid_rank(sorted_values: &[f64], observed: f64) -> f64 {
    let less = sorted_values
        .iter()
        .take_while(|value| **value < observed)
        .count();
    let equal = sorted_values
        .iter()
        .filter(|value| **value == observed)
        .count();
    (less as f64 + 0.5 * equal as f64) / sorted_values.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, role: SplitRole, primary: bool) -> StatisticSpec {
        StatisticSpec {
            id: id.to_string(),
            construct: format!("construct-{id}"),
            measurement_unit: "fraction".to_string(),
            population: "declared-population".to_string(),
            context: "declared-context".to_string(),
            source_family: format!("source-{id}"),
            source_digest: format!("sha256-source-{id}"),
            independent_unit: IndependentUnitKind::Region,
            role,
            primary,
            required: true,
            minimum_draws: 5,
        }
    }

    fn registry() -> StatisticRegistry {
        StatisticRegistry {
            version: "test-v1".to_string(),
            statistics: vec![
                spec("service-rate", SplitRole::Validation, true),
                spec("joint-dependence", SplitRole::Validation, true),
                spec("held-out-tail", SplitRole::Challenge, true),
            ],
        }
    }

    fn manifest() -> SplitManifest {
        SplitManifest {
            id: "split-v1".to_string(),
            independent_unit: IndependentUnitKind::Region,
            calibration_data_digest: "sha256-calibration".to_string(),
            manifest_digest: "sha256-manifest".to_string(),
            calibration_units: vec!["region-cal".to_string()],
            validation_units: vec!["region-val".to_string()],
            challenge_units: vec!["region-challenge".to_string()],
        }
    }

    fn observation(statistic_id: &str, independent_unit_id: &str, value: f64) -> ObservedStatistic {
        ObservedStatistic {
            statistic_id: statistic_id.to_string(),
            independent_unit_id: independent_unit_id.to_string(),
            measurement_unit: "fraction".to_string(),
            source_digest: format!("sha256-source-{statistic_id}"),
            sample_size: 100,
            value,
            measurement_standard_error: Some(0.01),
            observation_digest: format!("sha256-observation-{statistic_id}-{independent_unit_id}"),
        }
    }

    fn draws(
        kind: PredictiveKind,
        statistic_id: &str,
        independent_unit_id: &str,
        values: &[f64],
    ) -> Vec<PredictiveDraw> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| PredictiveDraw {
                kind,
                statistic_id: statistic_id.to_string(),
                independent_unit_id: independent_unit_id.to_string(),
                sample_size: 100,
                draw_index: index as u64,
                value: *value,
                model_digest: "sha256-model".to_string(),
                draw_set_digest: format!("sha256-draw-set-{}", kind.name()),
                calibration_data_digest: match kind {
                    PredictiveKind::Prior => None,
                    PredictiveKind::Split => Some("sha256-calibration".to_string()),
                },
            })
            .collect()
    }

    #[test]
    fn registry_rejects_duplicate_statistic_identity() {
        let mut candidate = registry();
        candidate
            .statistics
            .push(spec("service-rate", SplitRole::Validation, true));
        assert!(matches!(
            candidate.validate(),
            Err(AdequacyError::DuplicateStatistic { .. })
        ));
    }

    #[test]
    fn registry_rejects_optional_primary_statistic() {
        let mut candidate = registry();
        candidate.statistics[0].required = false;
        assert!(matches!(
            candidate.validate(),
            Err(AdequacyError::PrimaryStatisticMustBeRequired { .. })
        ));
    }

    #[test]
    fn registry_rejects_missing_unit_or_source_identity() {
        let mut candidate = registry();
        candidate.statistics[0].measurement_unit.clear();
        assert!(matches!(
            candidate.validate(),
            Err(AdequacyError::InvalidStatistic { .. })
        ));
    }

    #[test]
    fn split_manifest_rejects_cross_partition_unit_leakage() {
        let mut candidate = manifest();
        candidate.challenge_units.push("region-val".to_string());
        assert!(matches!(
            candidate.validate(),
            Err(AdequacyError::SplitUnitOverlap { .. })
        ));
    }

    #[test]
    fn calibration_partition_cannot_be_mistaken_for_validation() {
        assert!(matches!(
            assess_predictive_partition(
                &registry(),
                &manifest(),
                &[],
                &[],
                SplitRole::Calibration,
                PredictiveKind::Split,
                0.9
            ),
            Err(AdequacyError::CalibrationPartitionCannotBeEvaluated)
        ));
    }

    #[test]
    fn prior_reachability_flags_a_target_outside_the_declared_reference() {
        let observations = vec![observation("service-rate", "region-val", 0.95)];
        let prior = draws(
            PredictiveKind::Prior,
            "service-rate",
            "region-val",
            &[0.10, 0.12, 0.14, 0.16, 0.18, 0.20, 0.22],
        );
        let assessment = assess_predictive_partition(
            &registry(),
            &manifest(),
            &observations,
            &prior,
            SplitRole::Validation,
            PredictiveKind::Prior,
            0.8,
        )
        .expect("valid prior-predictive reference");
        assert_eq!(
            assessment.statistics[0].status,
            StatisticStatus::OutsideReferenceInterval
        );
        assert_eq!(
            assessment.domain_status,
            DomainStatus::PrimaryDiscrepancyDetected
        );
    }

    #[test]
    fn prior_reachability_accepts_an_observation_inside_the_reference_interval() {
        let observations = vec![observation("service-rate", "region-val", 0.5)];
        let prior = draws(
            PredictiveKind::Prior,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9],
        );
        let assessment = assess_predictive_partition(
            &registry(),
            &manifest(),
            &observations,
            &prior,
            SplitRole::Validation,
            PredictiveKind::Prior,
            0.8,
        )
        .expect("valid prior-predictive reference");
        assert_eq!(
            assessment.statistics[0].status,
            StatisticStatus::WithinReferenceInterval
        );
        assert_eq!(
            assessment.statistics[0].observation_digest.as_deref(),
            Some("sha256-observation-service-rate-region-val")
        );
        assert_eq!(
            assessment.statistics[0].model_digest.as_deref(),
            Some("sha256-model")
        );
        assert_eq!(
            assessment.statistics[0].draw_set_digest.as_deref(),
            Some("sha256-draw-set-prior")
        );
    }

    #[test]
    fn split_predictive_check_detects_calibration_overfit_on_held_out_unit() {
        let observations = vec![observation("service-rate", "region-val", 0.95)];
        let split_draws = draws(
            PredictiveKind::Split,
            "service-rate",
            "region-val",
            &[0.10, 0.12, 0.14, 0.16, 0.18, 0.20, 0.22],
        );
        let assessment = assess_predictive_partition(
            &registry(),
            &manifest(),
            &observations,
            &split_draws,
            SplitRole::Validation,
            PredictiveKind::Split,
            0.8,
        )
        .expect("valid split-predictive reference");
        assert_eq!(
            assessment.statistics[0].status,
            StatisticStatus::OutsideReferenceInterval
        );
        assert_eq!(
            assessment.domain_status,
            DomainStatus::PrimaryDiscrepancyDetected
        );
    }

    #[test]
    fn split_predictive_requires_the_exact_calibration_digest() {
        let observations = vec![observation("service-rate", "region-val", 0.5)];
        let mut split_draws = draws(
            PredictiveKind::Split,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        split_draws[0].calibration_data_digest = Some("sha256-different-training-data".to_string());
        assert!(matches!(
            assess_predictive_partition(
                &registry(),
                &manifest(),
                &observations,
                &split_draws,
                SplitRole::Validation,
                PredictiveKind::Split,
                0.8
            ),
            Err(AdequacyError::CalibrationDigestMismatch { .. })
        ));
    }

    #[test]
    fn mismatched_sample_size_is_not_silently_pooled() {
        let observations = vec![observation("service-rate", "region-val", 0.5)];
        let mut split_draws = draws(
            PredictiveKind::Split,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        for draw in &mut split_draws {
            draw.sample_size = 50;
        }
        let assessment = assess_predictive_partition(
            &registry(),
            &manifest(),
            &observations,
            &split_draws,
            SplitRole::Validation,
            PredictiveKind::Split,
            0.8,
        )
        .expect("well-formed but unmatched predictive draws");
        assert_eq!(
            assessment.statistics[0].status,
            StatisticStatus::NotAssessed
        );
        assert_eq!(assessment.statistics[0].draw_count, 0);
        assert_eq!(assessment.domain_status, DomainStatus::NotAssessed);
    }

    #[test]
    fn measurement_unit_mismatch_fails_closed() {
        let mut observations = vec![observation("service-rate", "region-val", 0.5)];
        observations[0].measurement_unit = "percent".to_string();
        let prior = draws(
            PredictiveKind::Prior,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        assert!(matches!(
            assess_predictive_partition(
                &registry(),
                &manifest(),
                &observations,
                &prior,
                SplitRole::Validation,
                PredictiveKind::Prior,
                0.8
            ),
            Err(AdequacyError::InvalidObservation { .. })
        ));
    }

    #[test]
    fn observation_source_digest_must_match_the_registered_statistic() {
        let mut observations = vec![observation("service-rate", "region-val", 0.5)];
        observations[0].source_digest = "sha256-substituted-source".to_string();
        let prior = draws(
            PredictiveKind::Prior,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        assert!(matches!(
            assess_predictive_partition(
                &registry(),
                &manifest(),
                &observations,
                &prior,
                SplitRole::Validation,
                PredictiveKind::Prior,
                0.8
            ),
            Err(AdequacyError::InvalidObservation { .. })
        ));
    }

    #[test]
    fn validation_cannot_consume_a_challenge_partition_observation() {
        let observations = vec![observation("held-out-tail", "region-challenge", 0.5)];
        assert!(matches!(
            assess_predictive_partition(
                &registry(),
                &manifest(),
                &observations,
                &[],
                SplitRole::Validation,
                PredictiveKind::Prior,
                0.8
            ),
            Err(AdequacyError::StatisticRoleMismatch { .. })
        ));
    }

    #[test]
    fn required_statistics_missing_from_a_partition_are_not_assessed() {
        let assessment = assess_predictive_partition(
            &registry(),
            &manifest(),
            &[],
            &[],
            SplitRole::Validation,
            PredictiveKind::Split,
            0.9,
        )
        .expect("missing required evidence is a result, not a panic");
        assert_eq!(assessment.statistics.len(), 2);
        assert!(
            assessment
                .statistics
                .iter()
                .all(|result| { result.status == StatisticStatus::NotAssessed })
        );
        assert!(!assessment.required_evidence_complete);
        assert_eq!(assessment.domain_status, DomainStatus::NotAssessed);
    }

    #[test]
    fn missing_required_non_primary_statistic_blocks_domain_qualification() {
        let mut candidate = registry();
        candidate.statistics[1].primary = false;
        let observations = vec![observation("service-rate", "region-val", 0.3)];
        let split_draws = draws(
            PredictiveKind::Split,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        let assessment = assess_predictive_partition(
            &candidate,
            &manifest(),
            &observations,
            &split_draws,
            SplitRole::Validation,
            PredictiveKind::Split,
            0.8,
        )
        .expect("missing diagnostic evidence remains an assessment result");
        assert!(!assessment.required_evidence_complete);
        assert_eq!(assessment.domain_status, DomainStatus::NotAssessed);
    }

    #[test]
    fn duplicate_predictive_draw_identity_is_rejected() {
        let observations = vec![observation("service-rate", "region-val", 0.5)];
        let mut split_draws = draws(
            PredictiveKind::Split,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        split_draws[1].draw_index = split_draws[0].draw_index;
        assert!(matches!(
            assess_predictive_partition(
                &registry(),
                &manifest(),
                &observations,
                &split_draws,
                SplitRole::Validation,
                PredictiveKind::Split,
                0.8
            ),
            Err(AdequacyError::DuplicateDraw { .. })
        ));
    }

    #[test]
    fn sample_size_specific_draw_sets_can_coexist() {
        let observations = vec![observation("service-rate", "region-val", 0.3)];
        let mut sample_100 = draws(
            PredictiveKind::Split,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        let mut sample_50 = sample_100.clone();
        for draw in &mut sample_100 {
            draw.draw_set_digest = "draw-set-n100".to_string();
        }
        for draw in &mut sample_50 {
            draw.sample_size = 50;
            draw.draw_set_digest = "draw-set-n50".to_string();
        }
        sample_100.extend(sample_50);

        let assessment = assess_predictive_partition(
            &registry(),
            &manifest(),
            &observations,
            &sample_100,
            SplitRole::Validation,
            PredictiveKind::Split,
            0.8,
        )
        .expect("distinct sample-size draw sets are independently identified");
        assert_eq!(
            assessment.statistics[0].status,
            StatisticStatus::WithinReferenceInterval
        );
        assert_eq!(assessment.statistics[0].draw_count, 5);
    }

    #[test]
    fn mixed_model_or_draw_set_identity_is_rejected() {
        let observations = vec![observation("service-rate", "region-val", 0.5)];
        let mut split_draws = draws(
            PredictiveKind::Split,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        split_draws[2].model_digest = "sha256-another-model".to_string();
        assert!(matches!(
            assess_predictive_partition(
                &registry(),
                &manifest(),
                &observations,
                &split_draws,
                SplitRole::Validation,
                PredictiveKind::Split,
                0.8
            ),
            Err(AdequacyError::MixedModelDrawSet { .. })
        ));
    }

    #[test]
    fn tied_predictive_values_use_a_mid_rank() {
        let observations = vec![observation("service-rate", "region-val", 0.5)];
        let split_draws = draws(
            PredictiveKind::Split,
            "service-rate",
            "region-val",
            &[0.1, 0.5, 0.5, 0.5, 0.9],
        );
        let assessment = assess_predictive_partition(
            &registry(),
            &manifest(),
            &observations,
            &split_draws,
            SplitRole::Validation,
            PredictiveKind::Split,
            0.8,
        )
        .expect("valid tied draws");
        assert!((assessment.statistics[0].predictive_rank.unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn each_registered_statistic_remains_separate_without_composite_averaging() {
        let observations = vec![
            observation("service-rate", "region-val", 0.3),
            observation("joint-dependence", "region-val", 0.95),
        ];
        let mut split_draws = draws(
            PredictiveKind::Split,
            "service-rate",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        split_draws.extend(draws(
            PredictiveKind::Split,
            "joint-dependence",
            "region-val",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        ));
        let assessment = assess_predictive_partition(
            &registry(),
            &manifest(),
            &observations,
            &split_draws,
            SplitRole::Validation,
            PredictiveKind::Split,
            0.8,
        )
        .expect("valid independent statistic families");
        assert_eq!(assessment.statistics.len(), 2);
        assert_eq!(
            assessment.statistics[0].status,
            StatisticStatus::WithinReferenceInterval
        );
        assert_eq!(
            assessment.statistics[1].status,
            StatisticStatus::OutsideReferenceInterval
        );
        assert_eq!(
            assessment.domain_status,
            DomainStatus::PrimaryDiscrepancyDetected
        );
    }

    #[test]
    fn coverage_summary_uses_independent_units_and_reports_wilson_uncertainty() {
        let mut manifest = manifest();
        manifest.validation_units = (0..10).map(|index| format!("region-{index}")).collect();

        let registry = StatisticRegistry {
            version: "coverage-v1".to_string(),
            statistics: vec![spec("service-rate", SplitRole::Validation, true)],
        };

        let mut observations = Vec::new();
        let mut predictive = Vec::new();
        for (index, unit_id) in manifest.validation_units.iter().enumerate() {
            let observed_value = if index < 8 { 0.3 } else { 0.9 };
            observations.push(observation("service-rate", unit_id, observed_value));

            let mut unit_draws = draws(
                PredictiveKind::Split,
                "service-rate",
                unit_id,
                &[0.1, 0.2, 0.3, 0.4, 0.5],
            );
            predictive.append(&mut unit_draws);
        }

        let assessment = assess_predictive_partition(
            &registry,
            &manifest,
            &observations,
            &predictive,
            SplitRole::Validation,
            PredictiveKind::Split,
            0.8,
        )
        .expect("valid independent-unit coverage fixture");

        assert_eq!(assessment.coverage.len(), 1);
        let coverage = &assessment.coverage[0];
        assert_eq!(coverage.expected_independent_units, 10);
        assert_eq!(coverage.assessed_independent_units, 10);
        assert_eq!(coverage.not_assessed_independent_units, 0);
        assert_eq!(coverage.covered_independent_units, 8);
        assert!((coverage.empirical_coverage.unwrap() - 0.8).abs() < 1e-12);
        assert!(coverage.wilson_lower_95.unwrap() < 0.5);
        assert!(coverage.wilson_upper_95.unwrap() > 0.9);
    }

    #[test]
    fn challenge_partition_is_evaluated_only_when_explicitly_requested() {
        let observations = vec![observation("held-out-tail", "region-challenge", 0.8)];
        let challenge_draws = draws(
            PredictiveKind::Split,
            "held-out-tail",
            "region-challenge",
            &[0.1, 0.2, 0.3, 0.4, 0.5],
        );
        let assessment = assess_predictive_partition(
            &registry(),
            &manifest(),
            &observations,
            &challenge_draws,
            SplitRole::Challenge,
            PredictiveKind::Split,
            0.8,
        )
        .expect("explicit challenge evaluation");
        assert_eq!(assessment.role, SplitRole::Challenge);
        assert_eq!(
            assessment.domain_status,
            DomainStatus::PrimaryDiscrepancyDetected
        );
    }

    #[test]
    fn empty_challenge_partition_is_valid_but_has_no_evidence_to_assess() {
        let mut candidate = manifest();
        candidate.challenge_units.clear();
        candidate
            .validate()
            .expect("empty challenge set is permitted");
        let assessment = assess_predictive_partition(
            &registry(),
            &candidate,
            &[],
            &[],
            SplitRole::Challenge,
            PredictiveKind::Split,
            0.9,
        )
        .expect("empty challenge is not a failed validation");
        assert_eq!(assessment.domain_status, DomainStatus::NotAssessed);
    }
}
