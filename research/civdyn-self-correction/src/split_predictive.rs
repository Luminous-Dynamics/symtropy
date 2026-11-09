// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Split predictive checks over source-pinned held-out predictive draws.
//!
//! A single SPC compares a held-out statistic with predictive replicates
//! generated from a model fitted on the disjoint calibration partition.
//! Ties are randomized with an explicit deterministic seed. A divided SPC
//! tests whether single-SPC p-values across all declared independent units
//! are compatible with Uniform(0, 1), using a Monte Carlo KS reference.
//!
//! The second-stage Monte Carlo test is exact for its simulated iid
//! continuous Uniform null up to Monte Carlo error. Model-based single-SPC
//! p-values inherit the predictive/exchangeability and asymptotic assumptions
//! of SPC; this module does not claim those assumptions for arbitrary
//! dependent, hierarchical, network, or time-series observations.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

use crate::model_adequacy::{
    assess_predictive_partition, AdequacyError, IndependentUnitKind, ObservedStatistic,
    PredictiveDraw, PredictiveKind, SplitManifest, SplitRole, StatisticRegistry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpcTail {
    Lower,
    Upper,
}

impl SpcTail {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lower => "lower",
            Self::Upper => "upper",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SingleSpcResult {
    pub statistic_id: String,
    pub independent_unit_id: String,
    pub independent_unit_kind: IndependentUnitKind,
    pub measurement_unit: String,
    pub sample_size: usize,
    pub observed_value: f64,
    pub tail: SpcTail,
    pub predictive_p_value: f64,
    /// Randomized lower-tail rank before applying the requested tail direction.
    pub randomized_lower_tail_rank: f64,
    pub predictive_draw_count: usize,
    pub model_digest: String,
    pub draw_set_digest: String,
    pub source_digest: String,
    pub observation_digest: String,
    pub calibration_data_digest: String,
    pub split_manifest_digest: String,
    pub randomization_seed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpcUnitPValue {
    pub independent_unit_id: String,
    pub p_value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DividedSpcResult {
    pub statistic_id: String,
    pub model_digest: String,
    pub calibration_data_digest: String,
    pub split_manifest_digest: String,
    pub tail: SpcTail,
    pub independent_unit_kind: IndependentUnitKind,
    pub sample_size: usize,
    pub unit_p_values: Vec<SpcUnitPValue>,
    pub ks_statistic: f64,
    pub uniformity_p_value: f64,
    pub unit_count: usize,
    pub monte_carlo_replicates: usize,
    pub monte_carlo_seed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitPredictiveError {
    Adequacy(AdequacyError),
    CalibrationRoleNotAllowed,
    UnknownStatistic { statistic_id: String },
    UnknownIndependentUnit { unit_id: String },
    MissingObservation { statistic_id: String, unit_id: String },
    InsufficientPredictiveDraws {
        statistic_id: String,
        unit_id: String,
        required: usize,
        actual: usize,
    },
    InvalidSingleSpcReceipt { unit_id: String },
    TooFewIndependentUnits { actual: usize },
    MissingDeclaredIndependentUnits { expected: usize, actual: usize },
    DuplicateIndependentUnit { unit_id: String },
    MixedStatisticIdentities,
    MixedModelIdentities,
    MixedTailDirections,
    MixedCalibrationIdentities,
    MixedSplitManifestIdentities,
    MixedSourceIdentities,
    MixedMeasurementUnits,
    MixedSampleSizes,
    InvalidMonteCarloReplicates,
}

impl fmt::Display for SplitPredictiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SplitPredictiveError {}

impl From<AdequacyError> for SplitPredictiveError {
    fn from(value: AdequacyError) -> Self {
        Self::Adequacy(value)
    }
}

/// Compute one randomized single-SPC rank for a declared held-out statistic.
///
/// Predictive replicates must be conditioned on the exact calibration digest
/// in the split manifest. They must match the statistic, independent unit,
/// sample size, model identity and draw-set identity in the adequacy receipt.
pub fn single_split_predictive_check(
    registry: &StatisticRegistry,
    manifest: &SplitManifest,
    observations: &[ObservedStatistic],
    draws: &[PredictiveDraw],
    role: SplitRole,
    statistic_id: &str,
    independent_unit_id: &str,
    tail: SpcTail,
    seed: u64,
) -> Result<SingleSpcResult, SplitPredictiveError> {
    if role == SplitRole::Calibration {
        return Err(SplitPredictiveError::CalibrationRoleNotAllowed);
    }

    let statistic = registry.get(statistic_id).ok_or_else(|| {
        SplitPredictiveError::UnknownStatistic {
            statistic_id: statistic_id.to_string(),
        }
    })?;

    let expected_units = units_for(manifest, role);
    if !expected_units
        .iter()
        .any(|unit_id| unit_id == independent_unit_id)
    {
        return Err(SplitPredictiveError::UnknownIndependentUnit {
            unit_id: independent_unit_id.to_string(),
        });
    }

    let observation = observations
        .iter()
        .find(|row| {
            row.statistic_id == statistic_id
                && row.independent_unit_id == independent_unit_id
        })
        .ok_or_else(|| SplitPredictiveError::MissingObservation {
            statistic_id: statistic_id.to_string(),
            unit_id: independent_unit_id.to_string(),
        })?;

    // Reuse the adequacy validator for unit, sample-size, split and
    // calibration-digest checks. Its interval classification is not used as
    // the SPC statistic.
    let assessment = assess_predictive_partition(
        registry,
        manifest,
        observations,
        draws,
        role,
        PredictiveKind::Split,
        0.5,
    )?;

    let assessed = assessment
        .statistics
        .iter()
        .find(|row| {
            row.statistic_id == statistic_id
                && row.independent_unit_id == independent_unit_id
        })
        .ok_or_else(|| SplitPredictiveError::MissingObservation {
            statistic_id: statistic_id.to_string(),
            unit_id: independent_unit_id.to_string(),
        })?;

    let matching: Vec<&PredictiveDraw> = draws
        .iter()
        .filter(|draw| {
            draw.kind == PredictiveKind::Split
                && draw.statistic_id == statistic_id
                && draw.independent_unit_id == independent_unit_id
                && draw.sample_size == observation.sample_size
        })
        .collect();

    if matching.len() < statistic.minimum_draws {
        return Err(SplitPredictiveError::InsufficientPredictiveDraws {
            statistic_id: statistic_id.to_string(),
            unit_id: independent_unit_id.to_string(),
            required: statistic.minimum_draws,
            actual: matching.len(),
        });
    }

    let model_digest = assessed.model_digest.clone().ok_or_else(|| {
        SplitPredictiveError::InsufficientPredictiveDraws {
            statistic_id: statistic_id.to_string(),
            unit_id: independent_unit_id.to_string(),
            required: statistic.minimum_draws,
            actual: 0,
        }
    })?;
    let draw_set_digest = assessed.draw_set_digest.clone().ok_or_else(|| {
        SplitPredictiveError::InsufficientPredictiveDraws {
            statistic_id: statistic_id.to_string(),
            unit_id: independent_unit_id.to_string(),
            required: statistic.minimum_draws,
            actual: 0,
        }
    })?;

    let predictive_values: Vec<f64> = matching.iter().map(|draw| draw.value).collect();
    let less = predictive_values
        .iter()
        .filter(|value| **value < observation.value)
        .count();
    let equal = predictive_values
        .iter()
        .filter(|value| **value == observation.value)
        .count();

    // Randomized rank among the held-out observation and predictive
    // replicates. An open-interval U avoids exact 0/1 p-values.
    let unit_seed = seed
        ^ stable_hash(statistic_id.as_bytes())
        ^ stable_hash(independent_unit_id.as_bytes());
    let mut rng = SplitMix64::new(unit_seed);
    let tie_break = rng.next_open_unit();
    let randomized_lower_tail_rank =
        (less as f64 + tie_break * (equal as f64 + 1.0))
            / (predictive_values.len() as f64 + 1.0);
    let predictive_p_value = match tail {
        SpcTail::Lower => randomized_lower_tail_rank,
        SpcTail::Upper => 1.0 - randomized_lower_tail_rank,
    };

    Ok(SingleSpcResult {
        statistic_id: statistic_id.to_string(),
        independent_unit_id: independent_unit_id.to_string(),
        independent_unit_kind: manifest.independent_unit,
        measurement_unit: statistic.measurement_unit.clone(),
        sample_size: observation.sample_size,
        observed_value: observation.value,
        tail,
        predictive_p_value,
        randomized_lower_tail_rank,
        predictive_draw_count: predictive_values.len(),
        model_digest,
        draw_set_digest,
        source_digest: statistic.source_digest.clone(),
        observation_digest: observation.observation_digest.clone(),
        calibration_data_digest: manifest.calibration_data_digest.clone(),
        split_manifest_digest: manifest.manifest_digest.clone(),
        randomization_seed: unit_seed,
    })
}

/// Compute a divided-SPC uniformity check over every declared unit in a
/// frozen partition. Unit identities must cover the whole declared partition.
pub fn divided_split_predictive_check(
    manifest: &SplitManifest,
    role: SplitRole,
    single_checks: &[SingleSpcResult],
    monte_carlo_replicates: usize,
    seed: u64,
) -> Result<DividedSpcResult, SplitPredictiveError> {
    manifest
        .validate()
        .map_err(SplitPredictiveError::Adequacy)?;
    if role == SplitRole::Calibration {
        return Err(SplitPredictiveError::CalibrationRoleNotAllowed);
    }
    if monte_carlo_replicates == 0 {
        return Err(SplitPredictiveError::InvalidMonteCarloReplicates);
    }
    if single_checks.len() < 2 {
        return Err(SplitPredictiveError::TooFewIndependentUnits {
            actual: single_checks.len(),
        });
    }

    let expected_units = units_for(manifest, role);
    if single_checks.len() != expected_units.len() {
        return Err(SplitPredictiveError::MissingDeclaredIndependentUnits {
            expected: expected_units.len(),
            actual: single_checks.len(),
        });
    }

    let first = &single_checks[0];
    let mut seen = BTreeSet::new();
    for check in single_checks {
        if check.statistic_id != first.statistic_id {
            return Err(SplitPredictiveError::MixedStatisticIdentities);
        }
        if check.model_digest != first.model_digest {
            return Err(SplitPredictiveError::MixedModelIdentities);
        }
        if check.tail != first.tail {
            return Err(SplitPredictiveError::MixedTailDirections);
        }
        if check.source_digest != first.source_digest {
            return Err(SplitPredictiveError::MixedSourceIdentities);
        }
        if check.measurement_unit != first.measurement_unit {
            return Err(SplitPredictiveError::MixedMeasurementUnits);
        }
        if check.sample_size != first.sample_size {
            return Err(SplitPredictiveError::MixedSampleSizes);
        }
        if check.calibration_data_digest != manifest.calibration_data_digest {
            return Err(SplitPredictiveError::MixedCalibrationIdentities);
        }
        if check.split_manifest_digest != manifest.manifest_digest {
            return Err(SplitPredictiveError::MixedSplitManifestIdentities);
        }
        if check.independent_unit_kind != manifest.independent_unit
            || !check.predictive_p_value.is_finite()
            || check.predictive_p_value <= 0.0
            || check.predictive_p_value >= 1.0
            || check.model_digest.trim().is_empty()
            || check.draw_set_digest.trim().is_empty()
            || check.observation_digest.trim().is_empty()
            || check.source_digest.trim().is_empty()
        {
            return Err(SplitPredictiveError::InvalidSingleSpcReceipt {
                unit_id: check.independent_unit_id.clone(),
            });
        }
        if !expected_units
            .iter()
            .any(|unit_id| unit_id == &check.independent_unit_id)
        {
            return Err(SplitPredictiveError::UnknownIndependentUnit {
                unit_id: check.independent_unit_id.clone(),
            });
        }
        if !seen.insert(check.independent_unit_id.as_str()) {
            return Err(SplitPredictiveError::DuplicateIndependentUnit {
                unit_id: check.independent_unit_id.clone(),
            });
        }
    }
    if seen.len() != expected_units.len()
        || expected_units
            .iter()
            .any(|unit_id| !seen.contains(unit_id.as_str()))
    {
        return Err(SplitPredictiveError::MissingDeclaredIndependentUnits {
            expected: expected_units.len(),
            actual: seen.len(),
        });
    }

    let mut unit_p_values: Vec<SpcUnitPValue> = single_checks
        .iter()
        .map(|check| SpcUnitPValue {
            independent_unit_id: check.independent_unit_id.clone(),
            p_value: check.predictive_p_value,
        })
        .collect();
    unit_p_values.sort_by(|left, right| {
        left.independent_unit_id.cmp(&right.independent_unit_id)
    });

    let values: Vec<f64> = unit_p_values.iter().map(|row| row.p_value).collect();
    let ks_statistic = ks_uniform_statistic(&values);
    let mut rng = SplitMix64::new(seed ^ stable_hash(first.statistic_id.as_bytes()));
    let mut exceedances = 0usize;
    for _ in 0..monte_carlo_replicates {
        let reference: Vec<f64> = (0..values.len())
            .map(|_| rng.next_open_unit())
            .collect();
        let reference_ks = ks_uniform_statistic(&reference);
        if reference_ks >= ks_statistic {
            exceedances += 1;
        }
    }
    let uniformity_p_value =
        (exceedances as f64 + 1.0) / (monte_carlo_replicates as f64 + 1.0);

    Ok(DividedSpcResult {
        statistic_id: first.statistic_id.clone(),
        model_digest: first.model_digest.clone(),
        calibration_data_digest: manifest.calibration_data_digest.clone(),
        split_manifest_digest: manifest.manifest_digest.clone(),
        tail: first.tail,
        independent_unit_kind: manifest.independent_unit,
        sample_size: first.sample_size,
        unit_p_values,
        ks_statistic,
        uniformity_p_value,
        unit_count: values.len(),
        monte_carlo_replicates,
        monte_carlo_seed: seed,
    })
}

fn units_for(manifest: &SplitManifest, role: SplitRole) -> &[String] {
    match role {
        SplitRole::Calibration => &manifest.calibration_units,
        SplitRole::Validation => &manifest.validation_units,
        SplitRole::Challenge => &manifest.challenge_units,
    }
}

/// One-sample two-sided Kolmogorov-Smirnov distance to Uniform(0, 1).
pub fn ks_uniform_statistic(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 1.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len() as f64;
    let mut d_plus = 0.0_f64;
    let mut d_minus = 0.0_f64;
    for (index, value) in sorted.iter().enumerate() {
        let upper = (index + 1) as f64 / n;
        let lower = index as f64 / n;
        d_plus = d_plus.max(upper - value);
        d_minus = d_minus.max(value - lower);
    }
    d_plus.max(d_minus)
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    fn next_open_unit(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64 + 0.5) * (1.0 / 9_007_199_254_740_992.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_adequacy::{
        IndependentUnitKind, ObservedStatistic, PredictiveDraw, PredictiveKind, SplitManifest,
        SplitRole, StatisticSpec,
    };

    fn registry() -> StatisticRegistry {
        StatisticRegistry {
            version: "spc-test-v1".to_string(),
            statistics: vec![StatisticSpec {
                id: "basin-water-flow".to_string(),
                construct: "basin water flow".to_string(),
                measurement_unit: "m3_per_day".to_string(),
                population: "synthetic-regions".to_string(),
                context: "known-answer-spc".to_string(),
                source_family: "synthetic-water-source".to_string(),
                source_digest: "sha256-water-source".to_string(),
                independent_unit: IndependentUnitKind::Region,
                role: SplitRole::Validation,
                primary: true,
                required: true,
                minimum_draws: 5,
            }],
        }
    }

    fn manifest() -> SplitManifest {
        SplitManifest {
            id: "spc-split-v1".to_string(),
            independent_unit: IndependentUnitKind::Region,
            calibration_data_digest: "sha256-calibration".to_string(),
            manifest_digest: "sha256-manifest".to_string(),
            calibration_units: vec!["cal-region".to_string()],
            validation_units: vec![
                "region-a".to_string(),
                "region-b".to_string(),
                "region-c".to_string(),
            ],
            challenge_units: vec![],
        }
    }

    fn observations() -> Vec<ObservedStatistic> {
        vec![
            observation("region-a", 3.0),
            observation("region-b", 2.5),
            observation("region-c", 4.0),
        ]
    }

    fn observation(unit_id: &str, value: f64) -> ObservedStatistic {
        ObservedStatistic {
            statistic_id: "basin-water-flow".to_string(),
            independent_unit_id: unit_id.to_string(),
            measurement_unit: "m3_per_day".to_string(),
            source_digest: "sha256-water-source".to_string(),
            sample_size: 10,
            value,
            measurement_standard_error: Some(0.1),
            observation_digest: format!("sha256-observation-{unit_id}"),
        }
    }

    fn draws() -> Vec<PredictiveDraw> {
        let values = [1.0, 2.0, 2.5, 3.0, 4.0];
        let mut result = Vec::new();
        for unit in ["region-a", "region-b", "region-c"] {
            for (index, value) in values.iter().enumerate() {
                result.push(PredictiveDraw {
                    kind: PredictiveKind::Split,
                    statistic_id: "basin-water-flow".to_string(),
                    independent_unit_id: unit.to_string(),
                    sample_size: 10,
                    draw_index: index as u64,
                    value: *value,
                    model_digest: "sha256-model".to_string(),
                    draw_set_digest: format!("sha256-draws-{unit}"),
                    calibration_data_digest: Some("sha256-calibration".to_string()),
                });
            }
        }
        result
    }

    fn checks() -> Vec<SingleSpcResult> {
        let registry = registry();
        let manifest = manifest();
        let observations = observations();
        let draws = draws();
        ["region-a", "region-b", "region-c"]
            .into_iter()
            .map(|unit| {
                single_split_predictive_check(
                    &registry,
                    &manifest,
                    &observations,
                    &draws,
                    SplitRole::Validation,
                    "basin-water-flow",
                    unit,
                    SpcTail::Lower,
                    42,
                )
                .expect("valid single SPC")
            })
            .collect()
    }

    #[test]
    fn single_spc_randomizes_ties_and_replays_exactly() {
        let first = checks();
        let second = checks();
        assert_eq!(first, second);
        assert!(first.iter().all(|row| row.predictive_p_value > 0.0));
        assert!(first.iter().all(|row| row.predictive_p_value < 1.0));
        assert!(first.iter().all(|row| row.predictive_draw_count == 5));
        assert!(first.iter().all(|row| row.calibration_data_digest == "sha256-calibration"));
        assert!(first.iter().all(|row| row.split_manifest_digest == "sha256-manifest"));
    }

    #[test]
    fn upper_and_lower_tail_p_values_are_complements() {
        let registry = registry();
        let manifest = manifest();
        let observations = observations();
        let draws = draws();
        let lower = single_split_predictive_check(
            &registry, &manifest, &observations, &draws, SplitRole::Validation,
            "basin-water-flow", "region-a", SpcTail::Lower, 42
        ).expect("lower tail SPC");
        let upper = single_split_predictive_check(
            &registry, &manifest, &observations, &draws, SplitRole::Validation,
            "basin-water-flow", "region-a", SpcTail::Upper, 42
        ).expect("upper tail SPC");
        assert!((lower.predictive_p_value + upper.predictive_p_value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_calibration_partition_for_a_predictive_check() {
        assert!(matches!(
            single_split_predictive_check(
                &registry(), &manifest(), &observations(), &draws(), SplitRole::Calibration,
                "basin-water-flow", "region-a", SpcTail::Lower, 42
            ),
            Err(SplitPredictiveError::CalibrationRoleNotAllowed)
        ));
    }

    #[test]
    fn single_spc_rejects_predictive_draws_from_a_different_calibration_split() {
        let mut foreign_draws = draws();
        for row in &mut foreign_draws {
            row.calibration_data_digest = Some("sha256-foreign-calibration".to_string());
        }
        assert!(matches!(
            single_split_predictive_check(
                &registry(), &manifest(), &observations(), &foreign_draws,
                SplitRole::Validation, "basin-water-flow", "region-a", SpcTail::Lower, 42
            ),
            Err(SplitPredictiveError::Adequacy(
                AdequacyError::CalibrationDigestMismatch { .. }
            ))
        ));
    }

    #[test]
    fn rejects_insufficient_split_predictive_draws() {
        let mut short_draws = draws();
        short_draws.retain(|row| {
            row.independent_unit_id != "region-a" || row.draw_index < 2
        });
        assert!(matches!(
            single_split_predictive_check(
                &registry(), &manifest(), &observations(), &short_draws, SplitRole::Validation,
                "basin-water-flow", "region-a", SpcTail::Lower, 42
            ),
            Err(SplitPredictiveError::InsufficientPredictiveDraws { .. })
        ));
    }

    #[test]
    fn ks_uniform_statistic_matches_two_point_reference() {
        assert!((ks_uniform_statistic(&[0.25, 0.75]) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn divided_spc_uses_all_manifest_units_and_is_reproducible() {
        let checks = checks();
        let first = divided_split_predictive_check(
            &manifest(), SplitRole::Validation, &checks, 999, 2026
        ).expect("valid divided SPC");
        let second = divided_split_predictive_check(
            &manifest(), SplitRole::Validation, &checks, 999, 2026
        ).expect("replayed divided SPC");
        assert_eq!(first, second);
        assert_eq!(first.unit_count, 3);
        assert_eq!(first.sample_size, 10);
        assert_eq!(first.split_manifest_digest, "sha256-manifest");
        assert_eq!(first.monte_carlo_replicates, 999);
        assert!(first.uniformity_p_value > 0.0 && first.uniformity_p_value <= 1.0);
    }

    #[test]
    fn divided_spc_rejects_missing_declared_units() {
        let mut checks = checks();
        checks.pop();
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks, 999, 1),
            Err(SplitPredictiveError::MissingDeclaredIndependentUnits { .. })
        ));
    }

    #[test]
    fn divided_spc_rejects_duplicate_independent_units() {
        let mut checks = checks();
        checks[1].independent_unit_id = checks[0].independent_unit_id.clone();
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks, 999, 1),
            Err(SplitPredictiveError::MissingDeclaredIndependentUnits { .. })
                | Err(SplitPredictiveError::DuplicateIndependentUnit { .. })
        ));
    }

    #[test]
    fn divided_spc_rejects_mixed_model_statistic_or_tail() {
        let mut checks = checks();
        checks[1].model_digest = "sha256-other-model".to_string();
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks, 999, 1),
            Err(SplitPredictiveError::MixedModelIdentities)
        ));

        let mut checks = checks();
        checks[1].statistic_id = "other-statistic".to_string();
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks, 999, 1),
            Err(SplitPredictiveError::MixedStatisticIdentities)
        ));

        let mut checks = checks();
        checks[1].tail = SpcTail::Upper;
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks, 999, 1),
            Err(SplitPredictiveError::MixedTailDirections)
        ));
    }

    #[test]
    fn divided_spc_rejects_mixed_split_manifest_identity() {
        let mut checks = checks();
        checks[1].split_manifest_digest = "sha256-other-manifest".to_string();
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks, 999, 1),
            Err(SplitPredictiveError::MixedSplitManifestIdentities)
        ));
    }

    #[test]
    fn divided_spc_rejects_mixed_source_and_measurement_identities() {
        let mut checks = checks();
        checks[1].source_digest = "sha256-other-source".to_string();
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks, 999, 1),
            Err(SplitPredictiveError::MixedSourceIdentities)
        ));

        let mut checks = checks();
        checks[1].measurement_unit = "kWh".to_string();
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks, 999, 1),
            Err(SplitPredictiveError::MixedMeasurementUnits)
        ));
    }

    #[test]
    fn divided_spc_rejects_mixed_sample_sizes() {
        let mut checks = checks();
        checks[1].sample_size += 1;
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks, 999, 1),
            Err(SplitPredictiveError::MixedSampleSizes)
        ));
    }

    #[test]
    fn divided_spc_rejects_zero_monte_carlo_replicates() {
        assert!(matches!(
            divided_split_predictive_check(&manifest(), SplitRole::Validation, &checks(), 0, 1),
            Err(SplitPredictiveError::InvalidMonteCarloReplicates)
        ));
    }

    #[test]
    fn divided_spc_requires_at_least_two_independent_units() {
        let mut one_unit_manifest = manifest();
        one_unit_manifest.validation_units = vec!["region-a".to_string()];
        let one_check = vec![checks()[0].clone()];
        assert!(matches!(
            divided_split_predictive_check(
                &one_unit_manifest, SplitRole::Validation, &one_check, 999, 1
            ),
            Err(SplitPredictiveError::TooFewIndependentUnits { .. })
        ));
    }
}
