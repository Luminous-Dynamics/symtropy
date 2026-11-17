//! Group-aware split-conformal coverage and selective-risk audit.
//!
//! The calibration unit is an independent deployment group (for example a
//! contact session or physical sensor article), not an adjacent frame. A
//! group-max standardized residual produces a simultaneous interval for every
//! sample and output within a new exchangeable group.
//!
//! This module is an evaluator-side statistical audit. It does not change the
//! deployed predictor, certify physical traceability, or guarantee coverage
//! under arbitrary distribution shift. The declared target population and
//! identity partitions are assumptions that must be justified by the campaign.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use super::wrench_consistency::ReferenceIndependenceDisposition;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum CoverageIdentityDimension {
    SensorFamily,
    SensorArticle,
    ManufacturingLot,
    SkinMaterialLot,
    HostMount,
    ReadoutChain,
    ProbeGeometry,
    SurfaceMaterial,
    ContactSession,
    OperatingEnvelopeCell,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CoverageIdentity {
    pub dimension: CoverageIdentityDimension,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CoveragePartition {
    Calibration,
    Validation,
    FinalTest,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CoverageObservation {
    pub sample_id: String,
    pub prediction: Vec<f64>,
    /// Evaluator-side truth; never an inference-time input.
    pub observed_truth: Vec<f64>,
    /// Positive marginal uncertainty scale matching each output component.
    pub standard_uncertainty: Vec<f64>,
    pub uncertainty_method_digest: String,
    pub operating_envelope_cell_id: String,
    pub inside_qualified_envelope: bool,
    pub reference_independence: ReferenceIndependenceDisposition,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CoverageGroup {
    pub group_id: String,
    /// Declared exchangeability population (e.g. sensor model + material regime).
    pub target_population_digest: String,
    pub partition: CoveragePartition,
    pub identities: Vec<CoverageIdentity>,
    pub observations: Vec<CoverageObservation>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CoverageAuditConfig {
    pub target_population_digest: String,
    pub uncertainty_method_digest: String,
    pub miscoverage_alpha: f64,
    pub physical_error_tolerance: f64,
    /// Identity dimensions that must be new between calibration and test groups.
    pub required_heldout_dimensions: Vec<CoverageIdentityDimension>,
    /// Dimensions that must remain fixed for this in-domain coverage claim.
    pub required_shared_dimensions: Vec<CoverageIdentityDimension>,
    /// Maximum interval half-width thresholds for selective risk/coverage.
    pub maximum_half_width_thresholds: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CoverageAuditDisposition {
    CoverageAudited,
    InsufficientCalibrationGroups,
    InvalidManifest,
    IdentityLeakageDetected,
    TargetPopulationShift,
    NoEligibleFinalTestGroups,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TestGroupDisposition {
    Covered,
    CoverageMiss,
    AbstainedOutOfEnvelope,
    AbstainedReferenceUnresolved,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TestGroupCoverage {
    pub group_id: String,
    pub disposition: TestGroupDisposition,
    pub maximum_standardized_nonconformity: Option<f64>,
    pub maximum_interval_half_width: Option<f64>,
    pub sample_count: usize,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SelectiveRiskCoveragePoint {
    pub maximum_half_width_threshold: f64,
    pub accepted_group_count: usize,
    pub accepted_sample_count: usize,
    pub accepted_group_coverage: Option<f64>,
    pub empirical_selective_risk: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CoverageAuditReport {
    pub disposition: CoverageAuditDisposition,
    pub target_population_digest: String,
    pub uncertainty_method_digest: String,
    pub miscoverage_alpha: f64,
    pub calibration_group_count: usize,
    pub validation_group_count: usize,
    pub final_test_group_count: usize,
    pub eligible_final_test_group_count: usize,
    pub conformal_quantile: Option<f64>,
    pub simultaneous_group_coverage: Option<f64>,
    pub simultaneous_group_coverage_lower_95: Option<f64>,
    pub simultaneous_group_coverage_upper_95: Option<f64>,
    pub covered_group_count: usize,
    pub missed_group_count: usize,
    pub abstained_out_of_envelope_group_count: usize,
    pub abstained_reference_unresolved_group_count: usize,
    pub mean_interval_half_width: Option<f64>,
    pub worst_test_group_nonconformity: Option<f64>,
    pub groups: Vec<TestGroupCoverage>,
    pub selective_risk_coverage: Vec<SelectiveRiskCoveragePoint>,
    pub leakage_findings: Vec<String>,
    pub target_shift_findings: Vec<String>,
    pub reasons: Vec<String>,
    pub claim_class: &'static str,
}

fn unique_dimensions(values: &[CoverageIdentityDimension], name: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(*value) {
            return Err(format!("{name} contains duplicate identity dimension {value:?}"));
        }
    }
    Ok(())
}

fn validate_config(config: &CoverageAuditConfig) -> Result<(), String> {
    for (name, value) in [
        ("target_population_digest", config.target_population_digest.as_str()),
        ("uncertainty_method_digest", config.uncertainty_method_digest.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("coverage config field {name} is missing"));
        }
    }
    if !config.miscoverage_alpha.is_finite()
        || config.miscoverage_alpha <= 0.0
        || config.miscoverage_alpha >= 1.0
        || !config.physical_error_tolerance.is_finite()
        || config.physical_error_tolerance < 0.0
    {
        return Err("miscoverage alpha must lie in (0,1) and physical error tolerance must be finite/nonnegative".to_owned());
    }
    if config.maximum_half_width_thresholds.is_empty()
        || config.maximum_half_width_thresholds.iter().any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("selective risk/coverage thresholds must be nonempty, finite and positive".to_owned());
    }
    unique_dimensions(&config.required_heldout_dimensions, "required_heldout_dimensions")?;
    unique_dimensions(&config.required_shared_dimensions, "required_shared_dimensions")?;
    if config.required_heldout_dimensions.iter().any(|dimension| {
        config.required_shared_dimensions.contains(dimension)
    }) {
        return Err("an identity dimension cannot be both held-out and required-shared".to_owned());
    }
    Ok(())
}

fn validate_group(
    group: &CoverageGroup,
    config: &CoverageAuditConfig,
) -> Vec<String> {
    let mut errors = Vec::new();
    if group.group_id.trim().is_empty() || group.target_population_digest.trim().is_empty() {
        errors.push("coverage group ID and target-population digest must be nonempty".to_owned());
    }
    if group.observations.is_empty() {
        errors.push(format!("coverage group {} has no observations", group.group_id));
    }
    let mut identities = BTreeMap::new();
    for identity in &group.identities {
        if identity.value.trim().is_empty() {
            errors.push(format!("group {} has an empty {:?} identity", group.group_id, identity.dimension));
        }
        if identities.insert(identity.dimension, &identity.value).is_some() {
            errors.push(format!("group {} repeats identity dimension {:?}", group.group_id, identity.dimension));
        }
    }
    for dimension in config.required_heldout_dimensions.iter().chain(&config.required_shared_dimensions) {
        if !identities.contains_key(dimension) {
            errors.push(format!("group {} is missing required identity dimension {dimension:?}", group.group_id));
        }
    }

    let mut sample_ids = BTreeSet::new();
    for observation in &group.observations {
        if observation.sample_id.trim().is_empty() || !sample_ids.insert(&observation.sample_id) {
            errors.push(format!("group {} contains an empty or duplicate sample ID", group.group_id));
        }
        if observation.prediction.is_empty()
            || observation.prediction.len() != observation.observed_truth.len()
            || observation.prediction.len() != observation.standard_uncertainty.len()
        {
            errors.push(format!("sample {} prediction/truth/uncertainty dimensions do not match", observation.sample_id));
        }
        if observation.prediction.iter().chain(&observation.observed_truth).any(|value| !value.is_finite())
            || observation.standard_uncertainty.iter().any(|value| !value.is_finite() || *value <= 0.0)
        {
            errors.push(format!("sample {} has non-finite prediction/truth or nonpositive uncertainty", observation.sample_id));
        }
        if observation.uncertainty_method_digest != config.uncertainty_method_digest {
            errors.push(format!("sample {} uses a different uncertainty method digest", observation.sample_id));
        }
        if observation.operating_envelope_cell_id.trim().is_empty()
            || observation.reference_ancestry_receipt_id.trim().is_empty()
            || observation.common_cause_analysis_receipt_id.trim().is_empty()
        {
            errors.push(format!("sample {} is missing operating-envelope or reference evidence identity", observation.sample_id));
        }
    }
    errors
}

fn group_nonconformity(group: &CoverageGroup) -> Result<f64, String> {
    let mut maximum = 0.0_f64;
    for observation in &group.observations {
        for index in 0..observation.prediction.len() {
            let score = (observation.observed_truth[index] - observation.prediction[index]).abs()
                / observation.standard_uncertainty[index];
            if !score.is_finite() {
                return Err(format!("sample {} produced non-finite standardized residual", observation.sample_id));
            }
            maximum = maximum.max(score);
        }
    }
    Ok(maximum)
}

fn group_half_width(group: &CoverageGroup, quantile: f64) -> f64 {
    group.observations
        .iter()
        .flat_map(|observation| observation.standard_uncertainty.iter())
        .map(|sigma| quantile * sigma)
        .fold(0.0_f64, f64::max)
}

fn group_has_reference_conflict(group: &CoverageGroup) -> bool {
    group.observations.iter().any(|observation| {
        observation.reference_independence != ReferenceIndependenceDisposition::IndependentlyCharacterized
    })
}

fn group_is_out_of_envelope(group: &CoverageGroup) -> bool {
    group.observations.iter().any(|observation| !observation.inside_qualified_envelope)
}

fn group_exceeds_physical_tolerance(group: &CoverageGroup, tolerance: f64) -> bool {
    group.observations.iter().any(|observation| {
        observation.prediction.iter().zip(&observation.observed_truth)
            .any(|(prediction, truth)| (prediction - truth).abs() > tolerance)
    })
}

fn wilson_interval(successes: usize, trials: usize) -> Option<(f64, f64)> {
    if trials == 0 {
        return None;
    }
    let z = 1.959_963_984_540_054_f64;
    let n = trials as f64;
    let observed = successes as f64 / n;
    let denominator = 1.0 + z * z / n;
    let center = (observed + z * z / (2.0 * n)) / denominator;
    let half = z
        * ((observed * (1.0 - observed) / n + z * z / (4.0 * n * n)).sqrt())
        / denominator;
    Some(((center - half).max(0.0), (center + half).min(1.0)))
}

fn identity_value<'a>(
    group: &'a CoverageGroup,
    dimension: CoverageIdentityDimension,
) -> Option<&'a str> {
    group.identities.iter()
        .find(|identity| identity.dimension == dimension)
        .map(|identity| identity.value.as_str())
}

/// Audit group-level split conformal intervals and selective risk/coverage.
/// Calibration scores use only Calibration groups. Final-test truth is used
/// only after the quantile is frozen; Validation groups do not tune q here.
pub fn audit_group_conformal_coverage(
    groups: &[CoverageGroup],
    config: &CoverageAuditConfig,
) -> CoverageAuditReport {
    let mut reasons = Vec::new();
    let mut leakage_findings = Vec::new();
    let mut target_shift_findings = Vec::new();
    if let Err(error) = validate_config(config) {
        reasons.push(error);
    }
    let mut group_ids = BTreeSet::new();
    let mut sample_ids = BTreeSet::new();
    for group in groups {
        if !group_ids.insert(group.group_id.clone()) {
            reasons.push(format!("duplicate group ID {}", group.group_id));
        }
        for observation in &group.observations {
            if !sample_ids.insert(observation.sample_id.clone()) {
                reasons.push(format!("sample ID {} crosses group boundaries", observation.sample_id));
            }
        }
        reasons.extend(validate_group(group, config));
        if group.target_population_digest != config.target_population_digest {
            target_shift_findings.push(format!(
                "group {} target population {} differs from declared population {}",
                group.group_id, group.target_population_digest, config.target_population_digest
            ));
        }
    }

    let calibration: Vec<_> = groups.iter()
        .filter(|group| group.partition == CoveragePartition::Calibration)
        .collect();
    let validation_count = groups.iter()
        .filter(|group| group.partition == CoveragePartition::Validation)
        .count();
    let test: Vec<_> = groups.iter()
        .filter(|group| group.partition == CoveragePartition::FinalTest)
        .collect();

    for dimension in &config.required_heldout_dimensions {
        let mut values = BTreeMap::<String, BTreeSet<CoveragePartition>>::new();
        for group in groups {
            if let Some(value) = identity_value(group, *dimension) {
                values.entry(value.to_owned()).or_default().insert(group.partition);
            }
        }
        for (value, partitions) in values {
            if partitions.len() > 1 {
                leakage_findings.push(format!(
                    "held-out identity {dimension:?}={value} appears in multiple partitions: {partitions:?}"
                ));
            }
        }
    }

    for dimension in &config.required_shared_dimensions {
        let values: BTreeSet<_> = groups.iter()
            .filter_map(|group| identity_value(group, *dimension))
            .collect();
        if values.len() > 1 {
            target_shift_findings.push(format!(
                "required-shared identity dimension {dimension:?} varies across groups: {values:?}"
            ));
        }
    }

    let base_report = |disposition, quantile, group_reports, coverage, coverage_lower, coverage_upper, covered, missed,
        eligible, out_of_domain, ref_unresolved, mean_half_width, worst_score, selective| CoverageAuditReport {
        disposition,
        target_population_digest: config.target_population_digest.clone(),
        uncertainty_method_digest: config.uncertainty_method_digest.clone(),
        miscoverage_alpha: config.miscoverage_alpha,
        calibration_group_count: calibration.len(),
        validation_group_count: validation_count,
        final_test_group_count: test.len(),
        eligible_final_test_group_count: eligible,
        conformal_quantile: quantile,
        simultaneous_group_coverage: coverage,
        simultaneous_group_coverage_lower_95: coverage_lower,
        simultaneous_group_coverage_upper_95: coverage_upper,
        covered_group_count: covered,
        missed_group_count: missed,
        abstained_out_of_envelope_group_count: out_of_domain,
        abstained_reference_unresolved_group_count: ref_unresolved,
        mean_interval_half_width: mean_half_width,
        worst_test_group_nonconformity: worst_score,
        groups: group_reports,
        selective_risk_coverage: selective,
        leakage_findings: leakage_findings.clone(),
        target_shift_findings: target_shift_findings.clone(),
        reasons: reasons.clone(),
        claim_class: "synthetic_group_conformal_coverage_audit_only",
    };

    if !reasons.is_empty() {
        return base_report(CoverageAuditDisposition::InvalidManifest, None, Vec::new(), None, None, None, 0, 0, 0, 0, 0, None, None, Vec::new());
    }
    if !leakage_findings.is_empty() {
        return base_report(CoverageAuditDisposition::IdentityLeakageDetected, None, Vec::new(), None, None, None, 0, 0, 0, 0, 0, None, None, Vec::new());
    }
    if !target_shift_findings.is_empty() {
        return base_report(CoverageAuditDisposition::TargetPopulationShift, None, Vec::new(), None, None, None, 0, 0, 0, 0, 0, None, None, Vec::new());
    }
    if test.is_empty() {
        return base_report(CoverageAuditDisposition::NoEligibleFinalTestGroups, None, Vec::new(), None, None, None, 0, 0, 0, 0, 0, None, None, Vec::new());
    }
    if calibration.is_empty() {
        reasons.push("no independent calibration groups are available".to_owned());
        return base_report(CoverageAuditDisposition::InsufficientCalibrationGroups, None, Vec::new(), None, None, None, 0, 0, 0, 0, 0, None, None, Vec::new());
    }

    let mut calibration_scores = Vec::with_capacity(calibration.len());
    for group in &calibration {
        if group_is_out_of_envelope(group) || group_has_reference_conflict(group) {
            reasons.push(format!(
                "calibration group {} is outside the qualified envelope or has unresolved reference independence",
                group.group_id
            ));
            return base_report(CoverageAuditDisposition::InvalidManifest, None, Vec::new(), None, None, None, 0, 0, 0, 0, 0, None, None, Vec::new());
        }
        match group_nonconformity(group) {
            Ok(score) => calibration_scores.push(score),
            Err(error) => {
                reasons.push(error);
                return base_report(CoverageAuditDisposition::InvalidManifest, None, Vec::new(), None, None, None, 0, 0, 0, 0, 0, None, None, Vec::new());
            }
        }
    }
    calibration_scores.sort_by(f64::total_cmp);
    let one_based_rank = ((calibration.len() as f64 + 1.0) * (1.0 - config.miscoverage_alpha)).ceil() as usize;
    if one_based_rank == 0 || one_based_rank > calibration_scores.len() {
        reasons.push(format!(
            "{} independent calibration groups cannot support alpha={:.4}; finite-sample quantile rank is {}",
            calibration.len(), config.miscoverage_alpha, one_based_rank
        ));
        return base_report(CoverageAuditDisposition::InsufficientCalibrationGroups, None, Vec::new(), None, None, None, 0, 0, 0, 0, 0, None, None, Vec::new());
    }
    let quantile = calibration_scores[one_based_rank - 1];

    let mut group_reports = Vec::with_capacity(test.len());
    let mut covered_count = 0;
    let mut missed_count = 0;
    let mut out_of_domain_count = 0;
    let mut reference_unresolved_count = 0;
    let mut widths = Vec::new();
    let mut worst_score: Option<f64> = None;

    for group in &test {
        if group_is_out_of_envelope(group) {
            out_of_domain_count += 1;
            group_reports.push(TestGroupCoverage {
                group_id: group.group_id.clone(),
                disposition: TestGroupDisposition::AbstainedOutOfEnvelope,
                maximum_standardized_nonconformity: None,
                maximum_interval_half_width: None,
                sample_count: group.observations.len(),
            });
            continue;
        }
        if group_has_reference_conflict(group) {
            reference_unresolved_count += 1;
            group_reports.push(TestGroupCoverage {
                group_id: group.group_id.clone(),
                disposition: TestGroupDisposition::AbstainedReferenceUnresolved,
                maximum_standardized_nonconformity: None,
                maximum_interval_half_width: None,
                sample_count: group.observations.len(),
            });
            continue;
        }
        let score = match group_nonconformity(group) {
            Ok(score) => score,
            Err(error) => {
                reasons.push(error);
                return base_report(CoverageAuditDisposition::InvalidManifest, Some(quantile), group_reports, None, covered_count, missed_count, out_of_domain_count, reference_unresolved_count, None, worst_score, Vec::new());
            }
        };
        worst_score = Some(worst_score.map_or(score, |current| current.max(score)));
        let width = group_half_width(group, quantile);
        widths.push(width);
        let disposition = if score <= quantile {
            covered_count += 1;
            TestGroupDisposition::Covered
        } else {
            missed_count += 1;
            TestGroupDisposition::CoverageMiss
        };
        group_reports.push(TestGroupCoverage {
            group_id: group.group_id.clone(),
            disposition,
            maximum_standardized_nonconformity: Some(score),
            maximum_interval_half_width: Some(width),
            sample_count: group.observations.len(),
        });
    }

    let eligible_count = covered_count + missed_count;
    let coverage = (eligible_count > 0).then(|| covered_count as f64 / eligible_count as f64);
    let (coverage_lower, coverage_upper) = wilson_interval(covered_count, eligible_count)
        .map_or((None, None), |(lower, upper)| (Some(lower), Some(upper)));
    let mean_half_width = (!widths.is_empty()).then(|| widths.iter().sum::<f64>() / widths.len() as f64);

    let mut selective = Vec::new();
    for threshold in &config.maximum_half_width_thresholds {
        let eligible_groups: Vec<_> = test.iter().filter(|group| {
            !group_is_out_of_envelope(group)
                && !group_has_reference_conflict(group)
                && group_half_width(group, quantile) <= *threshold
        }).collect();
        let accepted_count = eligible_groups.len();
        let risky_count = eligible_groups.iter()
            .filter(|group| group_exceeds_physical_tolerance(group, config.physical_error_tolerance))
            .count();
        let accepted_samples = eligible_groups.iter()
            .map(|group| group.observations.len())
            .sum::<usize>();
        selective.push(SelectiveRiskCoveragePoint {
            maximum_half_width_threshold: *threshold,
            accepted_group_count: accepted_count,
            accepted_sample_count: accepted_samples,
            accepted_group_coverage: (eligible_count > 0)
                .then(|| accepted_count as f64 / eligible_count as f64),
            empirical_selective_risk: (accepted_count > 0)
                .then(|| risky_count as f64 / accepted_count as f64),
        });
    }

    if eligible_count == 0 {
        reasons.push("all final-test groups were abstained; no eligible group remains for an empirical coverage audit".to_owned());
        return base_report(
            CoverageAuditDisposition::NoEligibleFinalTestGroups,
            Some(quantile),
            group_reports,
            None,
            None,
            None,
            covered_count,
            missed_count,
            eligible_count,
            out_of_domain_count,
            reference_unresolved_count,
            mean_half_width,
            worst_score,
            selective,
        );
    }

    base_report(
        CoverageAuditDisposition::CoverageAudited,
        Some(quantile),
        group_reports,
        coverage,
        coverage_lower,
        coverage_upper,
        covered_count,
        missed_count,
        eligible_count,
        out_of_domain_count,
        reference_unresolved_count,
        mean_half_width,
        worst_score,
        selective,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> CoverageAuditConfig {
        CoverageAuditConfig {
            target_population_digest: "population:resistive-r1-room-temperature".to_owned(),
            uncertainty_method_digest: "uncertainty:method-r1".to_owned(),
            miscoverage_alpha: 0.2,
            physical_error_tolerance: 0.25,
            required_heldout_dimensions: vec![CoverageIdentityDimension::ContactSession],
            required_shared_dimensions: vec![
                CoverageIdentityDimension::SensorFamily,
                CoverageIdentityDimension::SensorArticle,
                CoverageIdentityDimension::HostMount,
                CoverageIdentityDimension::OperatingEnvelopeCell,
            ],
            maximum_half_width_thresholds: vec![0.05, 0.15, 0.5, 1.0],
        }
    }

    fn group(id: &str, partition: CoveragePartition, error: f64) -> CoverageGroup {
        CoverageGroup {
            group_id: format!("group-{id}"),
            target_population_digest: "population:resistive-r1-room-temperature".to_owned(),
            partition,
            identities: vec![
                CoverageIdentity { dimension: CoverageIdentityDimension::SensorFamily, value: "resistive".to_owned() },
                CoverageIdentity { dimension: CoverageIdentityDimension::SensorArticle, value: "article-01".to_owned() },
                CoverageIdentity { dimension: CoverageIdentityDimension::ManufacturingLot, value: "lot-01".to_owned() },
                CoverageIdentity { dimension: CoverageIdentityDimension::SkinMaterialLot, value: "skin-lot-01".to_owned() },
                CoverageIdentity { dimension: CoverageIdentityDimension::HostMount, value: "finger-mount-r1".to_owned() },
                CoverageIdentity { dimension: CoverageIdentityDimension::ReadoutChain, value: "readout-r1".to_owned() },
                CoverageIdentity { dimension: CoverageIdentityDimension::ProbeGeometry, value: "tip-2mm".to_owned() },
                CoverageIdentity { dimension: CoverageIdentityDimension::SurfaceMaterial, value: "silicone-target".to_owned() },
                CoverageIdentity { dimension: CoverageIdentityDimension::ContactSession, value: format!("session-{id}") },
                CoverageIdentity { dimension: CoverageIdentityDimension::OperatingEnvelopeCell, value: "cell-room-20c".to_owned() },
            ],
            observations: vec![
                CoverageObservation {
                    sample_id: format!("sample-{id}-a"),
                    prediction: vec![0.0, 0.0, 0.0],
                    observed_truth: vec![error, -error, error * 0.5],
                    standard_uncertainty: vec![1.0, 1.0, 1.0],
                    uncertainty_method_digest: "uncertainty:method-r1".to_owned(),
                    operating_envelope_cell_id: "cell-room-20c".to_owned(),
                    inside_qualified_envelope: true,
                    reference_independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
                    reference_ancestry_receipt_id: format!("reference-ancestry-{id}"),
                    common_cause_analysis_receipt_id: format!("reference-cca-{id}"),
                }
            ],
        }
    }

    #[test]
    fn group_max_conformal_quantile_covers_simultaneous_outputs_on_heldout_groups() {
        let mut groups: Vec<_> = (0..20)
            .map(|index| group(&format!("cal-{index:02}"), CoveragePartition::Calibration, 0.1 + index as f64 * 0.001))
            .collect();
        groups.push(group("test-a", CoveragePartition::FinalTest, 0.12));
        groups.push(group("test-b", CoveragePartition::FinalTest, 0.4));
        let report = audit_group_conformal_coverage(&groups, &config());
        assert_eq!(report.disposition, CoverageAuditDisposition::CoverageAudited);
        assert_eq!(report.calibration_group_count, 20);
        assert!(report.conformal_quantile.is_some());
        assert_eq!(report.final_test_group_count, 2);
        assert_eq!(report.covered_group_count + report.missed_group_count, 2);
        assert!(report.simultaneous_group_coverage.is_some());
        assert_eq!(report.groups.len(), 2);
    }

    #[test]
    fn insufficient_independent_calibration_groups_never_emit_finite_quantile() {
        let groups = vec![
            group("cal-a", CoveragePartition::Calibration, 0.1),
            group("cal-b", CoveragePartition::Calibration, 0.1),
            group("test-a", CoveragePartition::FinalTest, 0.1),
        ];
        let mut cfg = config();
        cfg.miscoverage_alpha = 0.05;
        let report = audit_group_conformal_coverage(&groups, &cfg);
        assert_eq!(report.disposition, CoverageAuditDisposition::InsufficientCalibrationGroups);
        assert!(report.conformal_quantile.is_none());
    }

    #[test]
    fn session_identity_reuse_is_detected_as_group_leakage() {
        let mut calibration = group("cal-a", CoveragePartition::Calibration, 0.1);
        let mut test = group("test-a", CoveragePartition::FinalTest, 0.1);
        test.identities.iter_mut()
            .find(|identity| identity.dimension == CoverageIdentityDimension::ContactSession)
            .unwrap().value = calibration.identities.iter()
                .find(|identity| identity.dimension == CoverageIdentityDimension::ContactSession)
                .unwrap().value.clone();
        let report = audit_group_conformal_coverage(&[calibration, test], &config());
        assert_eq!(report.disposition, CoverageAuditDisposition::IdentityLeakageDetected);
        assert!(!report.leakage_findings.is_empty());
    }

    #[test]
    fn changed_host_or_population_suspends_the_coverage_claim() {
        let calibration = group("cal-a", CoveragePartition::Calibration, 0.1);
        let mut test = group("test-a", CoveragePartition::FinalTest, 0.1);
        test.identities.iter_mut()
            .find(|identity| identity.dimension == CoverageIdentityDimension::HostMount)
            .unwrap().value = "new-host".to_owned();
        let report = audit_group_conformal_coverage(&[calibration.clone(), test], &config());
        assert_eq!(report.disposition, CoverageAuditDisposition::TargetPopulationShift);

        let mut shifted = group("test-b", CoveragePartition::FinalTest, 0.1);
        shifted.target_population_digest = "population:new-sensor-family".to_owned();
        let report = audit_group_conformal_coverage(&[calibration, shifted], &config());
        assert_eq!(report.disposition, CoverageAuditDisposition::TargetPopulationShift);
    }

    #[test]
    fn out_of_envelope_and_unresolved_reference_groups_are_abstained() {
        let calibration = group("cal-a", CoveragePartition::Calibration, 0.1);
        let mut out_of_domain = group("test-a", CoveragePartition::FinalTest, 0.1);
        out_of_domain.observations[0].inside_qualified_envelope = false;
        let mut shared_reference = group("test-b", CoveragePartition::FinalTest, 0.1);
        shared_reference.observations[0].reference_independence =
            ReferenceIndependenceDisposition::SharedCommonCause;
        let report = audit_group_conformal_coverage(&[calibration, out_of_domain, shared_reference], &config());
        assert_eq!(report.disposition, CoverageAuditDisposition::NoEligibleFinalTestGroups);
        assert_eq!(report.abstained_out_of_envelope_group_count, 1);
        assert_eq!(report.abstained_reference_unresolved_group_count, 1);
        assert_eq!(report.covered_group_count + report.missed_group_count, 0);
    }

    #[test]
    fn duplicate_samples_and_invalid_uncertainty_fail_closed() {
        let calibration = group("cal-a", CoveragePartition::Calibration, 0.1);
        let mut duplicate = group("test-a", CoveragePartition::FinalTest, 0.1);
        duplicate.observations[0].sample_id = "sample-cal-a-a".to_owned();
        let report = audit_group_conformal_coverage(&[calibration, duplicate], &config());
        assert_eq!(report.disposition, CoverageAuditDisposition::InvalidManifest);

        let mut invalid = group("cal-b", CoveragePartition::Calibration, 0.1);
        invalid.observations[0].standard_uncertainty[1] = 0.0;
        let report = audit_group_conformal_coverage(&[invalid], &config());
        assert_eq!(report.disposition, CoverageAuditDisposition::InvalidManifest);
    }
}
