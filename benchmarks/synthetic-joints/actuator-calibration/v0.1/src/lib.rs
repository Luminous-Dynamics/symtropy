#![forbid(unsafe_code)]

//! Grouped split-conformal calibration for actuator prediction intervals.
//!
//! One independent physical specimen is the exchangeability unit. Repeated
//! trajectory samples are aggregated using the maximum interval miss per
//! specimen; they are never counted as additional calibration specimens.
//!
//! The result is a marginal group-level calibration diagnostic under the
//! declared exchangeability and fixed-model assumptions. It is not a
//! conditional guarantee under manufacturer/architecture/condition shift.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementEvidenceClass {
    IndependentPhysical,
    SourceAuthorPhysical,
    VendorPhysical,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationStatus {
    Calibrated,
    InsufficientCalibrationGroups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupTestDisposition {
    Covered,
    Uncovered,
    OutOfDomain,
    NotEvaluated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationPlan {
    pub model_digest: String,
    pub training_split_digest: String,
    pub calibration_split_digest: String,
    pub test_split_digest: String,
    pub mission_set_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub domain_digest: String,
    pub alpha: f64,
    pub evidence_class: MeasurementEvidenceClass,
    /// Physical specimen IDs used to train or fit the base model, when known.
    pub training_specimen_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictionObservation {
    pub sample_id: String,
    /// Index into the frozen mission trajectory; indices must be 0..N-1.
    pub sample_index: usize,
    pub predicted_lower_torque_nm: f64,
    pub predicted_upper_torque_nm: f64,
    pub observed_torque_nm: f64,
    pub in_domain: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecimenPredictions {
    pub specimen_id: String,
    pub manufacturer_id: String,
    pub actuator_model_id: String,
    pub architecture_id: String,
    pub evidence_class: MeasurementEvidenceClass,
    pub evidence_reference: String,
    pub measurement_method: String,
    pub calibration_digest: String,
    pub observations: Vec<PredictionObservation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictionDataset {
    pub split_digest: String,
    pub model_digest: String,
    pub training_split_digest: String,
    pub calibration_split_digest: String,
    pub test_split_digest: String,
    pub mission_set_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub domain_digest: String,
    pub specimens: Vec<SpecimenPredictions>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationScore {
    pub specimen_id: String,
    pub manufacturer_id: String,
    pub actuator_model_id: String,
    pub architecture_id: String,
    pub maximum_interval_miss_nm: f64,
    pub evidence_class: MeasurementEvidenceClass,
    pub evidence_reference: String,
    pub measurement_method: String,
    pub calibration_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupTestAssessment {
    pub specimen_id: String,
    pub manufacturer_id: String,
    pub actuator_model_id: String,
    pub architecture_id: String,
    pub disposition: GroupTestDisposition,
    pub observation_count: usize,
    pub maximum_interval_miss_nm: Option<f64>,
    pub evidence_class: MeasurementEvidenceClass,
    pub evidence_reference: String,
    pub measurement_method: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationReport {
    pub status: CalibrationStatus,
    pub model_digest: String,
    pub training_split_digest: String,
    pub calibration_split_digest: String,
    pub test_split_digest: String,
    pub mission_set_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub domain_digest: String,
    pub target_miscoverage_alpha: f64,
    pub calibration_specimen_count: usize,
    /// One-based finite-sample order-statistic rank.
    pub order_statistic_rank: usize,
    pub correction_q_nm: Option<f64>,
    pub test_specimen_count: usize,
    pub in_domain_test_specimen_count: usize,
    pub covered_test_specimen_count: usize,
    pub uncovered_test_specimen_count: usize,
    pub out_of_domain_test_specimen_count: usize,
    pub empirical_group_coverage: Option<f64>,
    /// Descriptive 95% Wilson interval for observed test-group coverage.
    pub wilson_95_lower: Option<f64>,
    pub wilson_95_upper: Option<f64>,
    pub mean_raw_interval_width_nm: Option<f64>,
    pub mean_adjusted_interval_width_nm: Option<f64>,
    pub calibration_scores: Vec<CalibrationScore>,
    pub test_assessments: Vec<GroupTestAssessment>,
    pub interpretation: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn invalid(message: impl Into<String>) -> String {
    message.into()
}

fn validate_plan(plan: &CalibrationPlan) -> Result<(), String> {
    if !nonempty(&plan.model_digest)
        || !nonempty(&plan.training_split_digest)
        || !nonempty(&plan.calibration_split_digest)
        || !nonempty(&plan.test_split_digest)
        || !nonempty(&plan.mission_set_digest)
        || !nonempty(&plan.system_boundary_digest)
        || !nonempty(&plan.operating_condition_digest)
        || !nonempty(&plan.domain_digest)
    {
        return Err(invalid("model, split, mission and domain digests must be nonempty"));
    }

    let split_digests = [
        plan.training_split_digest.as_str(),
        plan.calibration_split_digest.as_str(),
        plan.test_split_digest.as_str(),
    ];
    if split_digests[0] == split_digests[1]
        || split_digests[0] == split_digests[2]
        || split_digests[1] == split_digests[2]
    {
        return Err(invalid("training, calibration and test split digests must be distinct"));
    }

    if !plan.alpha.is_finite() || !(0.0..1.0).contains(&plan.alpha) {
        return Err(invalid("miscoverage alpha must be finite and strictly between zero and one"));
    }
    if plan.evidence_class == MeasurementEvidenceClass::Unknown {
        return Err(invalid("calibration evidence class cannot be Unknown"));
    }

    let mut training_ids = std::collections::BTreeSet::new();
    for specimen_id in &plan.training_specimen_ids {
        if !nonempty(specimen_id) || !training_ids.insert(specimen_id.as_str()) {
            return Err(invalid("training specimen IDs must be nonempty and unique"));
        }
    }
    Ok(())
}

fn validate_dataset(
    dataset: &PredictionDataset,
    expected_split_digest: &str,
    plan: &CalibrationPlan,
    is_calibration: bool,
    used_specimen_ids: &mut std::collections::BTreeSet<String>,
    used_sample_ids: &mut std::collections::BTreeSet<String>,
) -> Result<usize, String> {
    if dataset.split_digest != expected_split_digest {
        return Err(invalid("dataset split digest does not match the frozen plan"));
    }
    if dataset.model_digest != plan.model_digest
        || dataset.mission_set_digest != plan.mission_set_digest
        || dataset.system_boundary_digest != plan.system_boundary_digest
        || dataset.operating_condition_digest != plan.operating_condition_digest
        || dataset.domain_digest != plan.domain_digest
    {
        return Err(invalid("dataset model, mission or domain digest does not match the frozen plan"));
    }
    if dataset.specimens.is_empty() {
        return Err(invalid("calibration and test datasets must contain specimens"));
    }

    let training_ids: std::collections::BTreeSet<&str> =
        plan.training_specimen_ids.iter().map(String::as_str).collect();
    let mut sample_count_per_specimen = None;

    for specimen in &dataset.specimens {
        if !nonempty(&specimen.specimen_id)
            || !nonempty(&specimen.manufacturer_id)
            || !nonempty(&specimen.actuator_model_id)
            || !nonempty(&specimen.architecture_id)
            || !nonempty(&specimen.evidence_reference)
            || !nonempty(&specimen.measurement_method)
            || !nonempty(&specimen.calibration_digest)
        {
            return Err(invalid("specimen identity and measurement provenance fields must be nonempty"));
        }
        if specimen.evidence_class != plan.evidence_class {
            return Err(invalid("specimen measurement evidence class differs from the frozen calibration lane"));
        }
        if specimen.evidence_class == MeasurementEvidenceClass::Unknown {
            return Err(invalid("unknown evidence cannot be used for calibration or held-out coverage"));
        }
        if !used_specimen_ids.insert(specimen.specimen_id.clone()) {
            return Err(invalid(format!("specimen leakage or duplicate specimen ID: {}", specimen.specimen_id)));
        }
        if training_ids.contains(specimen.specimen_id.as_str()) {
            return Err(invalid(format!("specimen {} overlaps the training split", specimen.specimen_id)));
        }
        if specimen.observations.is_empty() {
            return Err(invalid(format!("specimen {} has no trajectory observations", specimen.specimen_id)));
        }

        match sample_count_per_specimen {
            None => sample_count_per_specimen = Some(specimen.observations.len()),
            Some(expected) if expected != specimen.observations.len() => {
                return Err(invalid("all specimen groups must use the same frozen trajectory sample count"));
            }
            Some(_) => {}
        }

        for (expected_index, observation) in specimen.observations.iter().enumerate() {
            if !nonempty(&observation.sample_id) || observation.sample_index != expected_index {
                return Err(invalid(format!(
                    "specimen {} observations must have nonempty sample IDs and contiguous ordered indices",
                    specimen.specimen_id
                )));
            }
            if !used_sample_ids.insert(observation.sample_id.clone()) {
                return Err(invalid(format!("duplicate sample ID: {}", observation.sample_id)));
            }
            if !observation.predicted_lower_torque_nm.is_finite()
                || !observation.predicted_upper_torque_nm.is_finite()
                || !observation.observed_torque_nm.is_finite()
            {
                return Err(invalid(format!("specimen {} has a non-finite torque observation", specimen.specimen_id)));
            }
            if observation.predicted_lower_torque_nm > observation.predicted_upper_torque_nm {
                return Err(invalid(format!("specimen {} has an inverted prediction interval", specimen.specimen_id)));
            }
            if is_calibration && !observation.in_domain {
                return Err(invalid(format!(
                    "calibration specimen {} contains an out-of-domain observation",
                    specimen.specimen_id
                )));
            }
        }
    }

    Ok(sample_count_per_specimen.unwrap_or(0))
}

fn specimen_nonconformity(specimen: &SpecimenPredictions) -> f64 {
    specimen
        .observations
        .iter()
        .map(|observation| {
            (observation.predicted_lower_torque_nm - observation.observed_torque_nm)
                .max(observation.observed_torque_nm - observation.predicted_upper_torque_nm)
                .max(0.0)
        })
        .fold(0.0_f64, f64::max)
}

fn wilson_95(successes: usize, trials: usize) -> Option<(f64, f64)> {
    if trials == 0 {
        return None;
    }
    const Z: f64 = 1.959_963_984_540_054;
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = Z * Z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half_width = Z * ((p * (1.0 - p) / n) + (z2 / (4.0 * n * n))).sqrt() / denominator;
    Some(((center - half_width).max(0.0), (center + half_width).min(1.0)))
}

/// Calibrates a fixed base model using specimen-grouped split conformal
/// expansion, then evaluates full-specimen coverage on a disjoint test set.
///
/// One specimen contributes exactly one calibration score: the maximum
/// interval miss across all trajectory observations. This targets marginal
/// coverage of an entire sampled specimen trajectory under exchangeability.
pub fn calibrate_grouped_intervals(
    plan: &CalibrationPlan,
    calibration: &PredictionDataset,
    test: &PredictionDataset,
) -> Result<CalibrationReport, String> {
    validate_plan(plan)?;

    if calibration.split_digest == test.split_digest {
        return Err(invalid("calibration and test datasets must have distinct split identities"));
    }

    let mut specimen_ids = std::collections::BTreeSet::new();
    let mut sample_ids = std::collections::BTreeSet::new();
    let calibration_sample_count = validate_dataset(
        calibration,
        &plan.calibration_split_digest,
        plan,
        true,
        &mut specimen_ids,
        &mut sample_ids,
    )?;
    let test_sample_count = validate_dataset(
        test,
        &plan.test_split_digest,
        plan,
        false,
        &mut specimen_ids,
        &mut sample_ids,
    )?;
    if calibration_sample_count != test_sample_count {
        return Err(invalid("calibration and test specimens must use the same frozen trajectory sample count"));
    }

    let calibration_count = calibration.specimens.len();
    let order_statistic_rank =
        (((calibration_count + 1) as f64) * (1.0 - plan.alpha)).ceil() as usize;

    let mut calibration_scores: Vec<CalibrationScore> = calibration
        .specimens
        .iter()
        .map(|specimen| CalibrationScore {
            specimen_id: specimen.specimen_id.clone(),
            manufacturer_id: specimen.manufacturer_id.clone(),
            actuator_model_id: specimen.actuator_model_id.clone(),
            architecture_id: specimen.architecture_id.clone(),
            maximum_interval_miss_nm: specimen_nonconformity(specimen),
            evidence_class: specimen.evidence_class,
            evidence_reference: specimen.evidence_reference.clone(),
            measurement_method: specimen.measurement_method.clone(),
            calibration_digest: specimen.calibration_digest.clone(),
        })
        .collect();
    calibration_scores.sort_by(|left, right| {
        left.maximum_interval_miss_nm
            .total_cmp(&right.maximum_interval_miss_nm)
            .then_with(|| left.specimen_id.cmp(&right.specimen_id))
    });

    let Some(correction_q_nm) = calibration_scores
        .get(order_statistic_rank.saturating_sub(1))
        .filter(|_| order_statistic_rank > 0)
        .map(|score| score.maximum_interval_miss_nm)
    else {
        let test_assessments = test
            .specimens
            .iter()
            .map(|specimen| GroupTestAssessment {
                specimen_id: specimen.specimen_id.clone(),
                manufacturer_id: specimen.manufacturer_id.clone(),
                actuator_model_id: specimen.actuator_model_id.clone(),
                architecture_id: specimen.architecture_id.clone(),
                disposition: GroupTestDisposition::NotEvaluated,
                observation_count: specimen.observations.len(),
                maximum_interval_miss_nm: None,
                evidence_class: specimen.evidence_class,
                evidence_reference: specimen.evidence_reference.clone(),
                measurement_method: specimen.measurement_method.clone(),
            })
            .collect();

        return Ok(CalibrationReport {
            status: CalibrationStatus::InsufficientCalibrationGroups,
            model_digest: plan.model_digest.clone(),
            training_split_digest: plan.training_split_digest.clone(),
            calibration_split_digest: plan.calibration_split_digest.clone(),
            test_split_digest: plan.test_split_digest.clone(),
            mission_set_digest: plan.mission_set_digest.clone(),
            system_boundary_digest: plan.system_boundary_digest.clone(),
            operating_condition_digest: plan.operating_condition_digest.clone(),
            domain_digest: plan.domain_digest.clone(),
            target_miscoverage_alpha: plan.alpha,
            calibration_specimen_count: calibration_count,
            order_statistic_rank,
            correction_q_nm: None,
            test_specimen_count: test.specimens.len(),
            in_domain_test_specimen_count: 0,
            covered_test_specimen_count: 0,
            uncovered_test_specimen_count: 0,
            out_of_domain_test_specimen_count: 0,
            empirical_group_coverage: None,
            wilson_95_lower: None,
            wilson_95_upper: None,
            mean_raw_interval_width_nm: None,
            mean_adjusted_interval_width_nm: None,
            calibration_scores,
            test_assessments,
            interpretation: "Insufficient independent calibration specimens for the requested finite-sample rank. No coverage statement or test pass is issued.".to_owned(),
        });
    };

    let mut test_assessments = Vec::with_capacity(test.specimens.len());
    let mut in_domain_test_specimen_count = 0usize;
    let mut covered_test_specimen_count = 0usize;
    let mut uncovered_test_specimen_count = 0usize;
    let mut out_of_domain_test_specimen_count = 0usize;
    let mut raw_interval_width_total = 0.0;
    let mut adjusted_interval_width_total = 0.0;
    let mut evaluated_observation_count = 0usize;

    for specimen in &test.specimens {
        let out_of_domain = specimen.observations.iter().any(|observation| !observation.in_domain);
        if out_of_domain {
            out_of_domain_test_specimen_count += 1;
            test_assessments.push(GroupTestAssessment {
                specimen_id: specimen.specimen_id.clone(),
                manufacturer_id: specimen.manufacturer_id.clone(),
                actuator_model_id: specimen.actuator_model_id.clone(),
                architecture_id: specimen.architecture_id.clone(),
                disposition: GroupTestDisposition::OutOfDomain,
                observation_count: specimen.observations.len(),
                maximum_interval_miss_nm: None,
                evidence_class: specimen.evidence_class,
                evidence_reference: specimen.evidence_reference.clone(),
                measurement_method: specimen.measurement_method.clone(),
            });
            continue;
        }

        in_domain_test_specimen_count += 1;
        let mut group_covered = true;
        let mut maximum_miss = 0.0_f64;

        for observation in &specimen.observations {
            let adjusted_lower = observation.predicted_lower_torque_nm - correction_q_nm;
            let adjusted_upper = observation.predicted_upper_torque_nm + correction_q_nm;
            let miss = (adjusted_lower - observation.observed_torque_nm)
                .max(observation.observed_torque_nm - adjusted_upper)
                .max(0.0);
            maximum_miss = maximum_miss.max(miss);
            if miss > 0.0 {
                group_covered = false;
            }
            raw_interval_width_total +=
                observation.predicted_upper_torque_nm - observation.predicted_lower_torque_nm;
            adjusted_interval_width_total += adjusted_upper - adjusted_lower;
            evaluated_observation_count += 1;
        }

        let disposition = if group_covered {
            covered_test_specimen_count += 1;
            GroupTestDisposition::Covered
        } else {
            uncovered_test_specimen_count += 1;
            GroupTestDisposition::Uncovered
        };
        test_assessments.push(GroupTestAssessment {
            specimen_id: specimen.specimen_id.clone(),
            manufacturer_id: specimen.manufacturer_id.clone(),
            actuator_model_id: specimen.actuator_model_id.clone(),
            architecture_id: specimen.architecture_id.clone(),
            disposition,
            observation_count: specimen.observations.len(),
            maximum_interval_miss_nm: Some(maximum_miss),
            evidence_class: specimen.evidence_class,
            evidence_reference: specimen.evidence_reference.clone(),
            measurement_method: specimen.measurement_method.clone(),
        });
    }

    let empirical_group_coverage = if in_domain_test_specimen_count == 0 {
        None
    } else {
        Some(covered_test_specimen_count as f64 / in_domain_test_specimen_count as f64)
    };
    let (wilson_95_lower, wilson_95_upper) =
        wilson_95(covered_test_specimen_count, in_domain_test_specimen_count)
            .map_or((None, None), |(lower, upper)| (Some(lower), Some(upper)));

    let mean_raw_interval_width_nm = if evaluated_observation_count == 0 {
        None
    } else {
        Some(raw_interval_width_total / evaluated_observation_count as f64)
    };
    let mean_adjusted_interval_width_nm = if evaluated_observation_count == 0 {
        None
    } else {
        Some(adjusted_interval_width_total / evaluated_observation_count as f64)
    };

    Ok(CalibrationReport {
        status: CalibrationStatus::Calibrated,
        model_digest: plan.model_digest.clone(),
        training_split_digest: plan.training_split_digest.clone(),
        calibration_split_digest: plan.calibration_split_digest.clone(),
        test_split_digest: plan.test_split_digest.clone(),
        mission_set_digest: plan.mission_set_digest.clone(),
        system_boundary_digest: plan.system_boundary_digest.clone(),
        operating_condition_digest: plan.operating_condition_digest.clone(),
        domain_digest: plan.domain_digest.clone(),
        target_miscoverage_alpha: plan.alpha,
        calibration_specimen_count: calibration_count,
        order_statistic_rank,
        correction_q_nm: Some(correction_q_nm),
        test_specimen_count: test.specimens.len(),
        in_domain_test_specimen_count,
        covered_test_specimen_count,
        uncovered_test_specimen_count,
        out_of_domain_test_specimen_count,
        empirical_group_coverage,
        wilson_95_lower,
        wilson_95_upper,
        mean_raw_interval_width_nm,
        mean_adjusted_interval_width_nm,
        calibration_scores,
        test_assessments,
        interpretation: "Observed group-level test coverage is descriptive. The marginal split-conformal statement relies on exchangeability of independent specimen groups, a fixed base model, frozen splits and the stated mission/domain; it is not a conditional guarantee under distribution shift.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "model-digest-v1";
    const TRAIN: &str = "train-split-v1";
    const CAL: &str = "calibration-split-v1";
    const TEST: &str = "test-split-v1";
    const MISSION: &str = "frozen-mission-v1";
    const DOMAIN: &str = "48V-25C-load-domain-v1";

    fn plan(alpha: f64) -> CalibrationPlan {
        CalibrationPlan {
            model_digest: MODEL.to_owned(),
            training_split_digest: TRAIN.to_owned(),
            calibration_split_digest: CAL.to_owned(),
            test_split_digest: TEST.to_owned(),
            mission_set_digest: MISSION.to_owned(),
            system_boundary_digest: "installed-joint-boundary-v1".to_owned(),
            operating_condition_digest: "48V-25C-cooling-A".to_owned(),
            domain_digest: DOMAIN.to_owned(),
            alpha,
            evidence_class: MeasurementEvidenceClass::IndependentPhysical,
            training_specimen_ids: vec!["train-specimen-1".to_owned()],
        }
    }

    fn group(
        specimen_id: &str,
        manufacturer: &str,
        model: &str,
        architecture: &str,
        values: &[(f64, f64, f64, bool)],
    ) -> SpecimenPredictions {
        SpecimenPredictions {
            specimen_id: specimen_id.to_owned(),
            manufacturer_id: manufacturer.to_owned(),
            actuator_model_id: model.to_owned(),
            architecture_id: architecture.to_owned(),
            evidence_class: MeasurementEvidenceClass::IndependentPhysical,
            evidence_reference: format!("https://example.invalid/{specimen_id}"),
            measurement_method: "Synthetic known-answer measurement fixture".to_owned(),
            calibration_digest: format!("cal-{specimen_id}"),
            observations: values
                .iter()
                .enumerate()
                .map(|(index, (lower, upper, observed, in_domain))| PredictionObservation {
                    sample_id: format!("{specimen_id}-sample-{index}"),
                    sample_index: index,
                    predicted_lower_torque_nm: *lower,
                    predicted_upper_torque_nm: *upper,
                    observed_torque_nm: *observed,
                    in_domain: *in_domain,
                })
                .collect(),
        }
    }

    fn calibration_dataset(groups: Vec<SpecimenPredictions>) -> PredictionDataset {
        PredictionDataset {
            split_digest: CAL.to_owned(),
            model_digest: MODEL.to_owned(),
            mission_set_digest: MISSION.to_owned(),
            system_boundary_digest: "installed-joint-boundary-v1".to_owned(),
            operating_condition_digest: "48V-25C-cooling-A".to_owned(),
            domain_digest: DOMAIN.to_owned(),
            specimens: groups,
        }
    }

    fn test_dataset(groups: Vec<SpecimenPredictions>) -> PredictionDataset {
        PredictionDataset {
            split_digest: TEST.to_owned(),
            model_digest: MODEL.to_owned(),
            mission_set_digest: MISSION.to_owned(),
            system_boundary_digest: "installed-joint-boundary-v1".to_owned(),
            operating_condition_digest: "48V-25C-cooling-A".to_owned(),
            domain_digest: DOMAIN.to_owned(),
            specimens: groups,
        }
    }

    fn known_calibration() -> PredictionDataset {
        calibration_dataset(vec![
            group("cal-1", "maker-a", "actuator-a", "qdd", &[(9.0, 11.0, 10.0, true), (9.0, 11.0, 12.0, true)]),
            group("cal-2", "maker-a", "actuator-b", "qdd", &[(18.0, 22.0, 20.0, true), (18.0, 22.0, 19.0, true)]),
            group("cal-3", "maker-b", "actuator-c", "harmonic", &[(29.0, 31.0, 32.0, true), (29.0, 31.0, 30.0, true)]),
        ])
    }

    fn known_test() -> PredictionDataset {
        test_dataset(vec![
            group("test-1", "maker-a", "heldout-a", "qdd", &[(9.0, 11.0, 11.5, true), (19.0, 21.0, 20.0, true)]),
            group("test-2", "maker-c", "heldout-b", "harmonic", &[(4.0, 6.0, 8.0, true), (9.0, 11.0, 10.0, true)]),
            group("test-3", "maker-z", "ood-c", "novel", &[(0.0, 1.0, 0.5, false), (0.0, 1.0, 0.5, true)]),
        ])
    }

    #[test]
    fn calibration_001_group_max_score_and_finite_sample_rank_are_exact() {
        let report = calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert_eq!(report.status, CalibrationStatus::Calibrated);
        assert_eq!(report.calibration_specimen_count, 3);
        assert_eq!(report.order_statistic_rank, 3);
        assert_eq!(report.correction_q_nm, Some(1.0));
        assert_eq!(report.calibration_scores.iter().map(|score| score.maximum_interval_miss_nm).collect::<Vec<_>>(), vec![0.0, 1.0, 1.0]);
    }

    #[test]
    fn calibration_002_test_coverage_is_grouped_and_ood_abstains() {
        let report = calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert_eq!(report.test_specimen_count, 3);
        assert_eq!(report.in_domain_test_specimen_count, 2);
        assert_eq!(report.covered_test_specimen_count, 1);
        assert_eq!(report.uncovered_test_specimen_count, 1);
        assert_eq!(report.out_of_domain_test_specimen_count, 1);
        assert_eq!(report.empirical_group_coverage, Some(0.5));
        assert_eq!(report.test_assessments[0].disposition, GroupTestDisposition::Covered);
        assert_eq!(report.test_assessments[1].disposition, GroupTestDisposition::Uncovered);
        assert_eq!(report.test_assessments[2].disposition, GroupTestDisposition::OutOfDomain);
    }

    #[test]
    fn calibration_003_group_wide_coverage_fails_if_any_sample_misses() {
        let mut test = known_test();
        test.specimens[0].observations[1].observed_torque_nm = 25.0;
        let report = calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &test).unwrap();
        assert_eq!(report.test_assessments[0].disposition, GroupTestDisposition::Uncovered);
    }

    #[test]
    fn calibration_004_insufficient_finite_sample_rank_abstains() {
        let report = calibrate_grouped_intervals(&plan(0.05), &known_calibration(), &known_test()).unwrap();
        assert_eq!(report.status, CalibrationStatus::InsufficientCalibrationGroups);
        assert_eq!(report.order_statistic_rank, 4);
        assert_eq!(report.correction_q_nm, None);
        assert!(report.empirical_group_coverage.is_none());
        assert!(report.test_assessments.iter().all(|group| group.disposition == GroupTestDisposition::NotEvaluated));
    }

    #[test]
    fn calibration_005_training_calibration_test_splits_must_be_distinct() {
        let mut invalid = plan(0.25);
        invalid.test_split_digest = invalid.calibration_split_digest.clone();
        assert!(calibrate_grouped_intervals(&invalid, &known_calibration(), &known_test()).is_err());
    }

    #[test]
    fn calibration_006_calibration_and_test_specimens_must_be_disjoint() {
        let calibration = known_calibration();
        let mut test = known_test();
        test.specimens[0].specimen_id = calibration.specimens[0].specimen_id.clone();
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &test).is_err());
    }

    #[test]
    fn calibration_007_training_specimens_cannot_leak_into_calibration_or_test() {
        let mut invalid = plan(0.25);
        invalid.training_specimen_ids.push("cal-1".to_owned());
        assert!(calibrate_grouped_intervals(&invalid, &known_calibration(), &known_test()).is_err());
    }

    #[test]
    fn calibration_008_duplicate_sample_ids_reject() {
        let mut calibration = known_calibration();
        calibration.specimens[1].observations[0].sample_id =
            calibration.specimens[0].observations[0].sample_id.clone();
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &known_test()).is_err());
    }

    #[test]
    fn calibration_009_out_of_domain_calibration_sample_rejects() {
        let mut calibration = known_calibration();
        calibration.specimens[0].observations[0].in_domain = false;
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &known_test()).is_err());
    }

    #[test]
    fn calibration_010_any_out_of_domain_test_sample_abstains_whole_specimen() {
        let mut test = known_test();
        test.specimens[0].observations[1].in_domain = false;
        let report = calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &test).unwrap();
        assert_eq!(report.test_assessments[0].disposition, GroupTestDisposition::OutOfDomain);
        assert_eq!(report.in_domain_test_specimen_count, 1);
        assert_eq!(report.out_of_domain_test_specimen_count, 2);
    }

    #[test]
    fn calibration_011_unknown_evidence_is_rejected() {
        let mut calibration = known_calibration();
        calibration.specimens[0].evidence_class = MeasurementEvidenceClass::Unknown;
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &known_test()).is_err());
    }

    #[test]
    fn calibration_012_mixed_measurement_evidence_classes_are_rejected() {
        let mut test = known_test();
        test.specimens[0].evidence_class = MeasurementEvidenceClass::VendorPhysical;
        assert!(calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &test).is_err());
    }

    #[test]
    fn calibration_013_model_mission_or_domain_digest_mismatch_rejects() {
        let mut test = known_test();
        test.domain_digest = "different-domain".to_owned();
        assert!(calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &test).is_err());

        let mut calibration = known_calibration();
        calibration.model_digest = "different-model".to_owned();
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &known_test()).is_err());
    }

    #[test]
    fn calibration_014_system_boundary_or_operating_condition_mismatch_rejects() {
        let mut test = known_test();
        test.system_boundary_digest = "motor-only-boundary".to_owned();
        assert!(calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &test).is_err());

        let mut calibration = known_calibration();
        calibration.operating_condition_digest = "different-cooling".to_owned();
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &known_test()).is_err());
    }

    #[test]
    fn calibration_015_prediction_bounds_must_be_ordered_and_finite() {
        let mut calibration = known_calibration();
        calibration.specimens[0].observations[0].predicted_lower_torque_nm = 20.0;
        calibration.specimens[0].observations[0].predicted_upper_torque_nm = 10.0;
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &known_test()).is_err());

        let mut test = known_test();
        test.specimens[0].observations[0].observed_torque_nm = f64::NAN;
        assert!(calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &test).is_err());
    }

    #[test]
    fn calibration_016_observation_indices_and_group_lengths_must_match() {
        let mut calibration = known_calibration();
        calibration.specimens[0].observations[1].sample_index = 0;
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &known_test()).is_err());

        let mut test = known_test();
        test.specimens[0].observations.pop();
        assert!(calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &test).is_err());
    }

    #[test]
    fn calibration_017_empty_in_domain_test_cohort_has_no_fake_zero_coverage() {
        let mut test = known_test();
        for specimen in &mut test.specimens {
            for observation in &mut specimen.observations {
                observation.in_domain = false;
            }
        }
        let report = calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &test).unwrap();
        assert_eq!(report.empirical_group_coverage, None);
        assert_eq!(report.wilson_95_lower, None);
        assert_eq!(report.wilson_95_upper, None);
        assert_eq!(report.mean_adjusted_interval_width_nm, None);
    }

    #[test]
    fn calibration_018_wilson_interval_contains_observed_group_coverage() {
        let report = calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert!(report.wilson_95_lower.unwrap() <= 0.5);
        assert!(report.wilson_95_upper.unwrap() >= 0.5);
        assert!(report.wilson_95_lower.unwrap() >= 0.0);
        assert!(report.wilson_95_upper.unwrap() <= 1.0);
    }

    #[test]
    fn calibration_019_invalid_alpha_rejects() {
        assert!(calibrate_grouped_intervals(&plan(0.0), &known_calibration(), &known_test()).is_err());
        assert!(calibrate_grouped_intervals(&plan(1.0), &known_calibration(), &known_test()).is_err());
        assert!(calibrate_grouped_intervals(&plan(f64::NAN), &known_calibration(), &known_test()).is_err());
    }

    #[test]
    fn calibration_020_sample_id_overlap_between_calibration_and_test_rejects() {
        let calibration = known_calibration();
        let mut test = known_test();
        test.specimens[0].observations[0].sample_id =
            calibration.specimens[0].observations[0].sample_id.clone();
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &test).is_err());
    }

    #[test]
    fn calibration_021_calibration_score_receipts_preserve_provenance_and_split_identity() {
        let report = calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &known_test()).unwrap();
        assert_eq!(report.training_split_digest, TRAIN);
        assert_eq!(report.calibration_split_digest, CAL);
        assert_eq!(report.test_split_digest, TEST);
        assert_eq!(report.calibration_scores[0].evidence_class, MeasurementEvidenceClass::IndependentPhysical);
        assert!(report.calibration_scores[0].evidence_reference.starts_with("https://"));
        assert!(!report.calibration_scores[0].calibration_digest.is_empty());
    }

    #[test]
    fn calibration_022_empty_dataset_or_specimen_group_rejects() {
        let mut calibration = known_calibration();
        calibration.specimens.clear();
        assert!(calibrate_grouped_intervals(&plan(0.25), &calibration, &known_test()).is_err());

        let mut test = known_test();
        test.specimens[0].observations.clear();
        assert!(calibrate_grouped_intervals(&plan(0.25), &known_calibration(), &test).is_err());
    }
}
