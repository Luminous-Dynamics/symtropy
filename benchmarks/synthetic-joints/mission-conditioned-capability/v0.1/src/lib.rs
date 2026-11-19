#![forbid(unsafe_code)]

//! Mission-conditioned, evidence-bounded joint capability evaluation.
//!
//! The M1 reference evaluates scalar output torque against declared
//! direction-specific torque-speed envelopes. It never extrapolates outside
//! supplied envelope points. Thermal, voltage, current, passivity, geometry,
//! lifecycle and electrical-storage gates remain separate qualification axes.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperatingRegime {
    Hold,
    ForwardMotoring,
    ForwardRegenerating,
    ReverseMotoring,
    ReverseRegenerating,
    Coast,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    IndependentRawTrace,
    IndependentTabulated,
    SourceAuthorPhysicalTest,
    VendorPhysicalTest,
    DigitizedSourcePlot,
    ReportedWithoutMethod,
    ModelDerived,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleDecision {
    Continuous,
    PeakLimited,
    Infeasible,
    Unknown,
    Coast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MechanicalEnvelopeDecision {
    Feasible,
    Infeasible,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceRecord {
    pub class: EvidenceClass,
    pub source_reference: String,
    pub method: String,
    pub calibration_reference: Option<String>,
    pub configuration_digest: String,
    pub operating_condition_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorqueSpeedPoint {
    pub speed_abs_rad_s: f64,
    pub continuous_torque_nm: f64,
    pub continuous_uncertainty_nm: f64,
    pub peak_torque_nm: f64,
    pub peak_uncertainty_nm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegimeEnvelope {
    pub regime: OperatingRegime,
    pub points: Vec<TorqueSpeedPoint>,
    /// Interpolation is rejected when adjacent knots are farther apart.
    pub maximum_interpolation_gap_rad_s: f64,
    /// Independently bounded maximum error of the piecewise-linear
    /// interpolation between knots, in N m. None means off-knot values are
    /// unknown; exact knot values remain usable.
    pub interpolation_error_bound_nm: Option<f64>,
    /// None means that peak-duration capability has not been qualified.
    pub maximum_contiguous_peak_duration_s: Option<f64>,
    pub evidence: EvidenceRecord,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateEnvelope {
    pub candidate_id: String,
    pub configuration_digest: String,
    /// Digest covering supply voltage, ambient, cooling fixture, controller,
    /// firmware, sample rate and other conditions affecting this envelope.
    pub operating_condition_digest: String,
    pub regimes: Vec<RegimeEnvelope>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MissionSample {
    pub time_s: f64,
    pub required_torque_nm: f64,
    pub required_torque_uncertainty_nm: f64,
    pub angular_speed_rad_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionProfile {
    pub mission_id: String,
    pub candidate_configuration_digest: String,
    pub operating_condition_digest: String,
    /// Samples are ordered in time; every sample defines a left-held interval
    /// ending at the next sample for peak-duration accounting.
    pub samples: Vec<MissionSample>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampleAssessment {
    pub sample_index: usize,
    pub time_s: f64,
    pub regime: OperatingRegime,
    pub decision: SampleDecision,
    pub required_torque_upper_nm: f64,
    pub continuous_torque_lower_nm: Option<f64>,
    pub peak_torque_lower_nm: Option<f64>,
    pub continuous_margin_nm: Option<f64>,
    pub peak_margin_nm: Option<f64>,
    pub evidence_class: Option<EvidenceClass>,
    pub evidence_reference: Option<String>,
    pub evidence_method: Option<String>,
    pub calibration_reference: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionMetrics {
    pub peak_required_torque_nm: f64,
    pub peak_required_torque_upper_nm: f64,
    pub peak_speed_rad_s: f64,
    pub rms_required_torque_nm: f64,
    pub positive_mechanical_work_j: f64,
    /// Absolute magnitude of negative mechanical work (absorbed at the
    /// mechanical port). This is not electrical regeneration.
    pub absorbed_mechanical_work_j: f64,
    pub minimum_continuous_margin_nm: Option<f64>,
    pub minimum_peak_margin_nm: Option<f64>,
    /// Fraction of non-coast samples with a known, configuration-matched torque envelope.
    /// Coast samples are excluded from the denominator.
    pub torque_envelope_sample_coverage: Option<f64>,
    pub torque_demand_sample_count: usize,
    pub evidence_covered_torque_sample_count: usize,
    pub coast_sample_count: usize,
    pub unknown_sample_count: usize,
    pub infeasible_sample_count: usize,
    pub peak_duration_violation_count: usize,
    pub maximum_observed_contiguous_peak_duration_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissionEvaluation {
    pub candidate_id: String,
    pub mission_id: String,
    pub candidate_configuration_digest: String,
    pub operating_condition_digest: String,
    /// This decision covers the mechanical torque-speed envelope only.
    pub decision: MechanicalEnvelopeDecision,
    pub samples: Vec<SampleAssessment>,
    pub metrics: MissionMetrics,
    pub interpretation: String,
}

fn finite(value: f64) -> bool {
    value.is_finite()
}

fn classify_regime(
    torque_nm: f64,
    torque_uncertainty_nm: f64,
    speed_rad_s: f64,
) -> Option<OperatingRegime> {
    if speed_rad_s == 0.0 {
        if torque_nm == 0.0 && torque_uncertainty_nm == 0.0 {
            Some(OperatingRegime::Coast)
        } else {
            Some(OperatingRegime::Hold)
        }
    } else if torque_nm.abs() <= torque_uncertainty_nm && torque_uncertainty_nm > 0.0 {
        // The uncertainty interval crosses zero, so motoring versus
        // regeneration is not identifiable from this sample.
        None
    } else if torque_nm == 0.0 {
        Some(OperatingRegime::Coast)
    } else {
        Some(match (torque_nm.is_sign_positive(), speed_rad_s.is_sign_positive()) {
            (true, true) => OperatingRegime::ForwardMotoring,
            (false, true) => OperatingRegime::ForwardRegenerating,
            (false, false) => OperatingRegime::ReverseMotoring,
            (true, false) => OperatingRegime::ReverseRegenerating,
        })
    }
}

fn validate_candidate(candidate: &CandidateEnvelope) -> Result<(), String> {
    if candidate.candidate_id.trim().is_empty()
        || candidate.configuration_digest.trim().is_empty()
        || candidate.operating_condition_digest.trim().is_empty()
    {
        return Err("candidate identity, configuration digest and condition digest are required".to_owned());
    }

    let mut regimes = std::collections::BTreeSet::new();
    for envelope in &candidate.regimes {
        if !regimes.insert(envelope.regime) {
            return Err(format!("duplicate operating regime {:?}", envelope.regime));
        }
        if !finite(envelope.maximum_interpolation_gap_rad_s)
            || envelope.maximum_interpolation_gap_rad_s <= 0.0
            || envelope.points.len() < 2
            || envelope.interpolation_error_bound_nm.is_some_and(|bound| !finite(bound) || bound < 0.0)
        {
            return Err(format!("regime {:?} has invalid interpolation settings or fewer than two knots", envelope.regime));
        }
        if let Some(limit) = envelope.maximum_contiguous_peak_duration_s {
            if !finite(limit) || limit <= 0.0 {
                return Err(format!("regime {:?} has invalid peak duration limit", envelope.regime));
            }
        }
        if envelope.evidence.source_reference.trim().is_empty()
            || envelope.evidence.method.trim().is_empty()
            || envelope.evidence.configuration_digest != candidate.configuration_digest
            || envelope.evidence.operating_condition_digest != candidate.operating_condition_digest
        {
            return Err(format!("regime {:?} evidence is incomplete or belongs to another configuration", envelope.regime));
        }

        for (index, point) in envelope.points.iter().enumerate() {
            if !finite(point.speed_abs_rad_s)
                || !finite(point.continuous_torque_nm)
                || !finite(point.continuous_uncertainty_nm)
                || !finite(point.peak_torque_nm)
                || !finite(point.peak_uncertainty_nm)
                || point.speed_abs_rad_s < 0.0
                || point.continuous_torque_nm < 0.0
                || point.continuous_uncertainty_nm < 0.0
                || point.peak_torque_nm < 0.0
                || point.peak_uncertainty_nm < 0.0
            {
                return Err(format!("regime {:?} has an invalid envelope point at index {index}", envelope.regime));
            }
            if point.continuous_uncertainty_nm > point.continuous_torque_nm
                || point.peak_uncertainty_nm > point.peak_torque_nm
            {
                return Err(format!("regime {:?} has uncertainty larger than its reported torque", envelope.regime));
            }
            if point.peak_torque_nm - point.peak_uncertainty_nm
                < point.continuous_torque_nm - point.continuous_uncertainty_nm
            {
                return Err(format!("regime {:?} peak lower bound is below continuous lower bound", envelope.regime));
            }
            if index > 0 {
                let previous = envelope.points[index - 1];
                if point.speed_abs_rad_s <= previous.speed_abs_rad_s {
                    return Err(format!("regime {:?} speed knots must be strictly increasing", envelope.regime));
                }
            }
        }
    }

    Ok(())
}

fn interpolate_lower_bound(
    envelope: &RegimeEnvelope,
    speed_abs_rad_s: f64,
    peak: bool,
) -> Option<f64> {
    let points = &envelope.points;
    if !finite(speed_abs_rad_s)
        || speed_abs_rad_s < points[0].speed_abs_rad_s
        || speed_abs_rad_s > points[points.len() - 1].speed_abs_rad_s
    {
        return None;
    }

    for point in points {
        if speed_abs_rad_s == point.speed_abs_rad_s {
            return Some(if peak {
                point.peak_torque_nm - point.peak_uncertainty_nm
            } else {
                point.continuous_torque_nm - point.continuous_uncertainty_nm
            });
        }
    }

    let upper_index = points
        .iter()
        .position(|point| point.speed_abs_rad_s > speed_abs_rad_s)?;
    let lower = points[upper_index - 1];
    let upper = points[upper_index];
    let gap = upper.speed_abs_rad_s - lower.speed_abs_rad_s;
    if gap > envelope.maximum_interpolation_gap_rad_s {
        return None;
    }

    let lower_value = if peak {
        lower.peak_torque_nm - lower.peak_uncertainty_nm
    } else {
        lower.continuous_torque_nm - lower.continuous_uncertainty_nm
    };
    let upper_value = if peak {
        upper.peak_torque_nm - upper.peak_uncertainty_nm
    } else {
        upper.continuous_torque_nm - upper.continuous_uncertainty_nm
    };
    let fraction = (speed_abs_rad_s - lower.speed_abs_rad_s) / gap;
    // Knot uncertainty alone does not bound curvature between knots.
    // Require a separate validation/certification error bound and subtract it
    // from the interpolated lower capability before comparing demand.
    let interpolation_error = envelope.interpolation_error_bound_nm?;
    let value = lower_value + fraction * (upper_value - lower_value) - interpolation_error;
    finite(value).then_some(value)
}

fn validate_mission(candidate: &CandidateEnvelope, mission: &MissionProfile) -> Result<(), String> {
    if mission.mission_id.trim().is_empty() || mission.samples.len() < 2 {
        return Err("mission ID and at least two trajectory samples are required".to_owned());
    }
    if mission.candidate_configuration_digest != candidate.configuration_digest {
        return Err("mission candidate configuration digest does not match candidate envelope".to_owned());
    }
    if mission.operating_condition_digest != candidate.operating_condition_digest {
        return Err("mission operating-condition digest does not match candidate envelope".to_owned());
    }

    for (index, sample) in mission.samples.iter().enumerate() {
        if !finite(sample.time_s)
            || !finite(sample.required_torque_nm)
            || !finite(sample.required_torque_uncertainty_nm)
            || !finite(sample.angular_speed_rad_s)
            || sample.time_s < 0.0
            || sample.required_torque_uncertainty_nm < 0.0
        {
            return Err(format!("mission sample {index} has invalid time, torque, uncertainty or speed"));
        }
        if index > 0 && sample.time_s <= mission.samples[index - 1].time_s {
            return Err(format!("mission timestamps are not strictly increasing at sample {index}"));
        }
    }

    Ok(())
}

fn work_components(p0: f64, p1: f64, dt: f64) -> (f64, f64) {
    let signed_area = 0.5 * (p0 + p1) * dt;
    if p0 >= 0.0 && p1 >= 0.0 {
        return (signed_area, 0.0);
    }
    if p0 <= 0.0 && p1 <= 0.0 {
        return (0.0, -signed_area);
    }

    let fraction = p0.abs() / (p0.abs() + p1.abs());
    let left_duration = dt * fraction;
    let right_duration = dt - left_duration;
    if p0 > 0.0 {
        (0.5 * p0 * left_duration, 0.5 * (-p1) * right_duration)
    } else {
        (0.5 * p1 * right_duration, 0.5 * (-p0) * left_duration)
    }
}

/// Evaluates a complete scalar joint trajectory against the candidate's
/// evidence-bounded, direction-specific torque-speed envelope.
pub fn evaluate_mission(
    candidate: &CandidateEnvelope,
    mission: &MissionProfile,
) -> Result<MissionEvaluation, String> {
    validate_candidate(candidate)?;
    validate_mission(candidate, mission)?;

    let mut assessments = Vec::with_capacity(mission.samples.len());
    let mut unknown_count = 0usize;
    let mut infeasible_count = 0usize;
    let mut peak_duration_violations = 0usize;
    let mut maximum_peak_duration = 0.0_f64;
    let mut current_peak_regime: Option<OperatingRegime> = None;
    let mut current_peak_duration = 0.0_f64;
    let mut minimum_continuous_margin: Option<f64> = None;
    let mut minimum_peak_margin: Option<f64> = None;

    for (index, sample) in mission.samples.iter().enumerate() {
        let regime = classify_regime(
            sample.required_torque_nm,
            sample.required_torque_uncertainty_nm,
            sample.angular_speed_rad_s,
        );
        let required_upper = sample.required_torque_nm.abs() + sample.required_torque_uncertainty_nm;

        let assessment = if regime.is_none() {
            SampleAssessment {
                sample_index: index,
                time_s: sample.time_s,
                regime: OperatingRegime::Ambiguous,
                decision: SampleDecision::Unknown,
                required_torque_upper_nm: required_upper,
                continuous_torque_lower_nm: None,
                peak_torque_lower_nm: None,
                continuous_margin_nm: None,
                peak_margin_nm: None,
                evidence_class: None,
                evidence_reference: None,
                evidence_method: None,
                calibration_reference: None,
                reason: Some("torque uncertainty crosses zero, so operating direction is ambiguous".to_owned()),
            }
        } else if regime == Some(OperatingRegime::Coast) {
            SampleAssessment {
                sample_index: index,
                time_s: sample.time_s,
                regime: OperatingRegime::Coast,
                decision: SampleDecision::Coast,
                required_torque_upper_nm: required_upper,
                continuous_torque_lower_nm: None,
                peak_torque_lower_nm: None,
                continuous_margin_nm: None,
                peak_margin_nm: None,
                evidence_class: None,
                evidence_reference: None,
                evidence_method: None,
                calibration_reference: None,
                reason: None,
            }
        } else if let Some(envelope) = candidate.regimes.iter().find(|branch| Some(branch.regime) == regime) {
            if envelope.evidence.class == EvidenceClass::Unknown {
                SampleAssessment {
                    sample_index: index,
                    time_s: sample.time_s,
                    regime: regime.unwrap(),
                    decision: SampleDecision::Unknown,
                    required_torque_upper_nm: required_upper,
                    continuous_torque_lower_nm: None,
                    peak_torque_lower_nm: None,
                    continuous_margin_nm: None,
                    peak_margin_nm: None,
                    evidence_class: Some(EvidenceClass::Unknown),
                    evidence_reference: Some(envelope.evidence.source_reference.clone()),
                    evidence_method: Some(envelope.evidence.method.clone()),
                    calibration_reference: envelope.evidence.calibration_reference.clone(),
                    reason: Some("envelope evidence provenance is unknown".to_owned()),
                }
            } else {
            let speed = sample.angular_speed_rad_s.abs();
            let continuous = interpolate_lower_bound(envelope, speed, false);
            let peak = interpolate_lower_bound(envelope, speed, true);
            match (continuous, peak) {
                (Some(continuous_lower), Some(peak_lower)) => {
                    let continuous_margin = continuous_lower - required_upper;
                    let peak_margin = peak_lower - required_upper;
                    minimum_continuous_margin = Some(
                        minimum_continuous_margin.map_or(continuous_margin, |v| v.min(continuous_margin)),
                    );
                    minimum_peak_margin = Some(
                        minimum_peak_margin.map_or(peak_margin, |v| v.min(peak_margin)),
                    );

                    let (decision, reason) = if continuous_margin >= 0.0 {
                        (SampleDecision::Continuous, None)
                    } else if peak_margin >= 0.0 {
                        if envelope.maximum_contiguous_peak_duration_s.is_some() {
                            (SampleDecision::PeakLimited, None)
                        } else {
                            (SampleDecision::Unknown, Some("peak duration capability is not qualified".to_owned()))
                        }
                    } else {
                        (SampleDecision::Infeasible, Some("required torque upper bound exceeds the peak-torque lower bound".to_owned()))
                    };

                    SampleAssessment {
                        sample_index: index,
                        time_s: sample.time_s,
                        regime: regime.unwrap(),
                        decision,
                        required_torque_upper_nm: required_upper,
                        continuous_torque_lower_nm: Some(continuous_lower),
                        peak_torque_lower_nm: Some(peak_lower),
                        continuous_margin_nm: Some(continuous_margin),
                        peak_margin_nm: Some(peak_margin),
                        evidence_class: Some(envelope.evidence.class),
                        evidence_reference: Some(envelope.evidence.source_reference.clone()),
                        evidence_method: Some(envelope.evidence.method.clone()),
                        calibration_reference: envelope.evidence.calibration_reference.clone(),
                        reason,
                    }
                }
                _ => SampleAssessment {
                    sample_index: index,
                    time_s: sample.time_s,
                    regime: regime.unwrap(),
                    decision: SampleDecision::Unknown,
                    required_torque_upper_nm: required_upper,
                    continuous_torque_lower_nm: None,
                    peak_torque_lower_nm: None,
                    continuous_margin_nm: None,
                    peak_margin_nm: None,
                    evidence_class: Some(envelope.evidence.class),
                    evidence_reference: Some(envelope.evidence.source_reference.clone()),
                    evidence_method: Some(envelope.evidence.method.clone()),
                    calibration_reference: envelope.evidence.calibration_reference.clone(),
                    reason: Some("speed is outside the envelope domain or falls in an interpolation gap".to_owned()),
                },
            }
            }
        } else {
            SampleAssessment {
                sample_index: index,
                time_s: sample.time_s,
                regime: regime.unwrap(),
                decision: SampleDecision::Unknown,
                required_torque_upper_nm: required_upper,
                continuous_torque_lower_nm: None,
                peak_torque_lower_nm: None,
                continuous_margin_nm: None,
                peak_margin_nm: None,
                evidence_class: None,
                evidence_reference: None,
                evidence_method: None,
                calibration_reference: None,
                reason: Some("no evidence-backed envelope exists for this operating regime".to_owned()),
            }
        };

        if assessment.decision == SampleDecision::Unknown {
            unknown_count += 1;
        }
        if assessment.decision == SampleDecision::Infeasible {
            infeasible_count += 1;
        }
        assessments.push(assessment);
    }

    // Each sample describes the interval ending at the next sample.
    for index in 0..(assessments.len() - 1) {
        let assessment = &assessments[index];
        let dt = mission.samples[index + 1].time_s - mission.samples[index].time_s;
        if assessment.decision == SampleDecision::PeakLimited {
            if current_peak_regime == Some(assessment.regime) {
                current_peak_duration += dt;
            } else {
                current_peak_regime = Some(assessment.regime);
                current_peak_duration = dt;
            }
            maximum_peak_duration = maximum_peak_duration.max(current_peak_duration);
            let limit = candidate.regimes
                .iter()
                .find(|branch| branch.regime == assessment.regime)
                .and_then(|branch| branch.maximum_contiguous_peak_duration_s);
            if limit.is_some_and(|max_duration| current_peak_duration > max_duration) {
                peak_duration_violations += 1;
            }
        } else {
            current_peak_regime = None;
            current_peak_duration = 0.0;
        }
    }

    let sample_count = mission.samples.len();
    let coast_count = assessments
        .iter()
        .filter(|assessment| assessment.decision == SampleDecision::Coast)
        .count();
    let torque_demand_count = sample_count - coast_count;
    let evidence_covered_count = assessments
        .iter()
        .filter(|assessment| {
            assessment.evidence_class.is_some()
                && assessment.decision != SampleDecision::Unknown
                && assessment.decision != SampleDecision::Coast
        })
        .count();
    let torque_coverage = if torque_demand_count == 0 {
        None
    } else {
        Some(evidence_covered_count as f64 / torque_demand_count as f64)
    };

    let mut peak_required_torque = 0.0_f64;
    let mut peak_required_torque_upper = 0.0_f64;
    let mut peak_speed = 0.0_f64;
    for sample in &mission.samples {
        peak_required_torque = peak_required_torque.max(sample.required_torque_nm.abs());
        peak_required_torque_upper = peak_required_torque_upper.max(
            sample.required_torque_nm.abs() + sample.required_torque_uncertainty_nm,
        );
        peak_speed = peak_speed.max(sample.angular_speed_rad_s.abs());
    }

    let total_duration = mission.samples.last().unwrap().time_s - mission.samples[0].time_s;
    let mut torque_square_integral = 0.0_f64;
    let mut positive_work = 0.0_f64;
    let mut absorbed_work = 0.0_f64;
    for index in 0..(sample_count - 1) {
        let a = mission.samples[index];
        let b = mission.samples[index + 1];
        let dt = b.time_s - a.time_s;
        let tau_a = a.required_torque_nm;
        let tau_b = b.required_torque_nm;
        torque_square_integral += dt * (tau_a * tau_a + tau_a * tau_b + tau_b * tau_b) / 3.0;
        let power_a = tau_a * a.angular_speed_rad_s;
        let power_b = tau_b * b.angular_speed_rad_s;
        if !finite(power_a) || !finite(power_b) {
            return Err(format!("mission sample pair {index} produces non-finite power"));
        }
        let (positive, absorbed) = work_components(power_a, power_b, dt);
        positive_work += positive;
        absorbed_work += absorbed;
    }

    let rms_torque = (torque_square_integral / total_duration).sqrt();
    if !finite(rms_torque)
        || !finite(positive_work)
        || !finite(absorbed_work)
        || torque_coverage.is_some_and(|coverage| !finite(coverage))
    {
        return Err("mission metric calculation produced a non-finite value".to_owned());
    }

    let decision = if infeasible_count > 0 || peak_duration_violations > 0 {
        MechanicalEnvelopeDecision::Infeasible
    } else if unknown_count > 0 {
        MechanicalEnvelopeDecision::Unknown
    } else {
        MechanicalEnvelopeDecision::Feasible
    };

    Ok(MissionEvaluation {
        candidate_id: candidate.candidate_id.clone(),
        mission_id: mission.mission_id.clone(),
        candidate_configuration_digest: candidate.configuration_digest.clone(),
        operating_condition_digest: candidate.operating_condition_digest.clone(),
        decision,
        samples: assessments,
        metrics: MissionMetrics {
            peak_required_torque_nm: peak_required_torque,
            peak_required_torque_upper_nm: peak_required_torque_upper,
            peak_speed_rad_s: peak_speed,
            rms_required_torque_nm: rms_torque,
            positive_mechanical_work_j: positive_work,
            absorbed_mechanical_work_j: absorbed_work,
            minimum_continuous_margin_nm: minimum_continuous_margin,
            minimum_peak_margin_nm: minimum_peak_margin,
            torque_envelope_sample_coverage: torque_coverage,
            torque_demand_sample_count: torque_demand_count,
            evidence_covered_torque_sample_count: evidence_covered_count,
            coast_sample_count: coast_count,
            unknown_sample_count: unknown_count,
            infeasible_sample_count: infeasible_count,
            peak_duration_violation_count: peak_duration_violations,
            maximum_observed_contiguous_peak_duration_s: maximum_peak_duration,
        },
        interpretation: "This is a mechanical torque-speed envelope decision only, not a whole-robot task-success or safety decision. Thermal, voltage, current, electrical recovery, passivity, geometry, reliability and task acceptance require separate evidence-backed gates.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "candidate-config-a";
    const CONDITIONS: &str = "48V-25C-plate-A-controller-r2";

    fn evidence(class: EvidenceClass) -> EvidenceRecord {
        EvidenceRecord {
            class,
            source_reference: "https://example.invalid/test-fixture".to_owned(),
            method: "synthetic known-answer fixture".to_owned(),
            calibration_reference: Some("synthetic-calibration".to_owned()),
            configuration_digest: CONFIG.to_owned(),
            operating_condition_digest: CONDITIONS.to_owned(),
        }
    }

    fn branch(regime: OperatingRegime, peak_duration: Option<f64>) -> RegimeEnvelope {
        RegimeEnvelope {
            regime,
            points: vec![
                TorqueSpeedPoint {
                    speed_abs_rad_s: 0.0,
                    continuous_torque_nm: 10.0,
                    continuous_uncertainty_nm: 1.0,
                    peak_torque_nm: 15.0,
                    peak_uncertainty_nm: 1.0,
                },
                TorqueSpeedPoint {
                    speed_abs_rad_s: 10.0,
                    continuous_torque_nm: 8.0,
                    continuous_uncertainty_nm: 1.0,
                    peak_torque_nm: 12.0,
                    peak_uncertainty_nm: 1.0,
                },
                TorqueSpeedPoint {
                    speed_abs_rad_s: 20.0,
                    continuous_torque_nm: 5.0,
                    continuous_uncertainty_nm: 0.5,
                    peak_torque_nm: 8.0,
                    peak_uncertainty_nm: 0.5,
                },
            ],
            maximum_interpolation_gap_rad_s: 10.0,
            interpolation_error_bound_nm: Some(0.0),
            maximum_contiguous_peak_duration_s: peak_duration,
            evidence: evidence(EvidenceClass::IndependentRawTrace),
        }
    }

    fn candidate() -> CandidateEnvelope {
        CandidateEnvelope {
            candidate_id: "joint-candidate-a".to_owned(),
            configuration_digest: CONFIG.to_owned(),
            operating_condition_digest: CONDITIONS.to_owned(),
            regimes: vec![
                branch(OperatingRegime::Hold, Some(0.5)),
                branch(OperatingRegime::ForwardMotoring, Some(0.5)),
                branch(OperatingRegime::ForwardRegenerating, Some(0.5)),
                branch(OperatingRegime::ReverseMotoring, Some(0.5)),
                branch(OperatingRegime::ReverseRegenerating, Some(0.5)),
            ],
        }
    }

    fn mission(samples: Vec<MissionSample>) -> MissionProfile {
        MissionProfile {
            mission_id: "mission-a".to_owned(),
            candidate_configuration_digest: CONFIG.to_owned(),
            operating_condition_digest: CONDITIONS.to_owned(),
            samples,
        }
    }

    fn sample(time: f64, torque: f64, uncertainty: f64, speed: f64) -> MissionSample {
        MissionSample {
            time_s: time,
            required_torque_nm: torque,
            required_torque_uncertainty_nm: uncertainty,
            angular_speed_rad_s: speed,
        }
    }

    #[test]
    fn mcjce_001_interpolates_inside_domain_and_reports_robust_continuous_margin() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![sample(0.0, 5.0, 0.5, 5.0), sample(1.0, 5.0, 0.5, 5.0)]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Feasible);
        assert_eq!(result.samples[0].decision, SampleDecision::Continuous);
        // At 5 rad/s the interpolated nominal value is 9 N m and uncertainty 1 N m.
        assert!((result.samples[0].continuous_torque_lower_nm.unwrap() - 8.0).abs() < 1.0e-12);
        assert!((result.samples[0].continuous_margin_nm.unwrap() - 2.5).abs() < 1.0e-12);
    }

    #[test]
    fn mcjce_002_nominally_feasible_but_uncertainty_adjusted_demand_is_peak_limited() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![sample(0.0, 8.0, 0.5, 5.0), sample(0.1, 8.0, 0.5, 5.0)]),
        ).unwrap();
        assert_eq!(result.samples[0].decision, SampleDecision::PeakLimited);
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Feasible);
    }

    #[test]
    fn mcjce_003_peak_duration_overrun_makes_the_mission_infeasible() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![
                sample(0.0, 8.0, 0.5, 5.0),
                sample(0.3, 8.0, 0.5, 5.0),
                sample(0.6, 8.0, 0.5, 5.0),
            ]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Infeasible);
        assert!(result.metrics.peak_duration_violation_count > 0);
        assert!((result.metrics.maximum_observed_contiguous_peak_duration_s - 0.6).abs() < 1.0e-12);
    }

    #[test]
    fn mcjce_004_demand_above_peak_lower_bound_is_infeasible() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![sample(0.0, 14.0, 0.5, 5.0), sample(0.1, 14.0, 0.5, 5.0)]),
        ).unwrap();
        assert_eq!(result.samples[0].decision, SampleDecision::Infeasible);
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Infeasible);
    }

    #[test]
    fn mcjce_005_speed_outside_measured_domain_is_unknown_not_extrapolated() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![sample(0.0, 2.0, 0.0, 25.0), sample(0.1, 2.0, 0.0, 25.0)]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Unknown);
        assert_eq!(result.metrics.unknown_sample_count, 2);
        assert_eq!(result.metrics.torque_envelope_sample_coverage, Some(0.0));
    }

    #[test]
    fn mcjce_006_missing_quadrant_data_is_unknown() {
        let mut candidate = candidate();
        candidate.regimes.retain(|branch| branch.regime != OperatingRegime::ForwardRegenerating);
        let result = evaluate_mission(
            &candidate,
            &mission(vec![sample(0.0, -2.0, 0.0, 3.0), sample(0.1, -2.0, 0.0, 3.0)]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Unknown);
        assert_eq!(result.samples[0].regime, OperatingRegime::ForwardRegenerating);
    }

    #[test]
    fn mcjce_007_uncertainty_can_reverse_a_nominal_feasibility_claim() {
        let mut envelope = candidate();
        let branch = envelope.regimes.iter_mut()
            .find(|b| b.regime == OperatingRegime::ForwardMotoring).unwrap();
        for point in &mut branch.points {
            point.continuous_torque_nm = 8.0;
            point.continuous_uncertainty_nm = 1.0;
            point.peak_torque_nm = 10.0;
            point.peak_uncertainty_nm = 1.0;
        }
        // The nominal request (7.5 N m) is below the nominal 8 N m limit,
        // but the conservative lower capability is 7 N m.
        let result = evaluate_mission(
            &envelope,
            &mission(vec![sample(0.0, 7.5, 0.0, 5.0), sample(0.1, 7.5, 0.0, 5.0)]),
        ).unwrap();
        assert_eq!(result.samples[0].decision, SampleDecision::PeakLimited);
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Feasible);
        assert_eq!(result.samples[0].continuous_margin_nm, Some(-0.5));
    }

    #[test]
    fn mcjce_008_interpolation_gap_is_unknown() {
        let mut candidate = candidate();
        let b = candidate.regimes.iter_mut().find(|b| b.regime == OperatingRegime::ForwardMotoring).unwrap();
        b.maximum_interpolation_gap_rad_s = 4.0;
        let result = evaluate_mission(
            &candidate,
            &mission(vec![sample(0.0, 2.0, 0.0, 5.0), sample(0.1, 2.0, 0.0, 5.0)]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Unknown);
    }

    #[test]
    fn mcjce_009_off_knot_interpolation_requires_an_error_bound() {
        let mut envelope = candidate();
        let b = envelope.regimes.iter_mut()
            .find(|b| b.regime == OperatingRegime::ForwardMotoring).unwrap();
        b.interpolation_error_bound_nm = None;
        let result = evaluate_mission(
            &envelope,
            &mission(vec![sample(0.0, 2.0, 0.0, 5.0), sample(0.1, 2.0, 0.0, 5.0)]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Unknown);
        assert_eq!(result.metrics.unknown_sample_count, 2);
    }

    #[test]
    fn mcjce_010_interpolation_error_bound_is_subtracted_from_available_torque() {
        let mut envelope = candidate();
        let b = envelope.regimes.iter_mut()
            .find(|b| b.regime == OperatingRegime::ForwardMotoring).unwrap();
        b.interpolation_error_bound_nm = Some(0.75);
        let result = evaluate_mission(
            &envelope,
            &mission(vec![sample(0.0, 5.0, 0.0, 5.0), sample(0.1, 5.0, 0.0, 5.0)]),
        ).unwrap();
        // Raw linear lower bound is 8 N m; the validated interpolation
        // error bound reduces the usable lower bound to 7.25 N m.
        assert!((result.samples[0].continuous_torque_lower_nm.unwrap() - 7.25).abs() < 1.0e-12);
        assert!((result.samples[0].continuous_margin_nm.unwrap() - 2.25).abs() < 1.0e-12);
    }

    #[test]
    fn mcjce_011_configuration_or_condition_mismatch_fails_closed() {
        let mut wrong_config = mission(vec![sample(0.0, 1.0, 0.0, 1.0), sample(0.1, 1.0, 0.0, 1.0)]);
        wrong_config.candidate_configuration_digest = "different".to_owned();
        assert!(evaluate_mission(&candidate(), &wrong_config).is_err());

        let mut wrong_conditions = mission(vec![sample(0.0, 1.0, 0.0, 1.0), sample(0.1, 1.0, 0.0, 1.0)]);
        wrong_conditions.operating_condition_digest = "different".to_owned();
        assert!(evaluate_mission(&candidate(), &wrong_conditions).is_err());
    }

    #[test]
    fn mcjce_012_positive_and_absorbed_mechanical_work_are_separate() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![
                sample(0.0, 2.0, 0.0, 1.0),
                sample(1.0, -2.0, 0.0, 1.0),
            ]),
        ).unwrap();
        assert!((result.metrics.positive_mechanical_work_j - 0.5).abs() < 1.0e-12);
        assert!((result.metrics.absorbed_mechanical_work_j - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn mcjce_013_rms_torque_uses_time_weighted_linear_interpolation() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![sample(0.0, 2.0, 0.0, 1.0), sample(1.0, 2.0, 0.0, 1.0)]),
        ).unwrap();
        assert!((result.metrics.rms_required_torque_nm - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn mcjce_014_coast_and_zero_speed_hold_are_not_confused() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![sample(0.0, 2.0, 0.0, 0.0), sample(1.0, 0.0, 0.0, 3.0)]),
        ).unwrap();
        assert_eq!(result.samples[0].regime, OperatingRegime::Hold);
        assert_eq!(result.samples[0].decision, SampleDecision::Continuous);
        assert_eq!(result.samples[1].regime, OperatingRegime::Coast);
        assert_eq!(result.samples[1].decision, SampleDecision::Coast);
    }

    #[test]
    fn mcjce_015_uncertainty_crossing_zero_makes_power_direction_unknown() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![sample(0.0, 0.1, 0.2, 3.0), sample(0.1, 0.1, 0.2, 3.0)]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Unknown);
        assert_eq!(result.samples[0].decision, SampleDecision::Unknown);
        assert!(result.samples[0].reason.as_ref().unwrap().contains("direction is ambiguous"));
    }

    #[test]
    fn mcjce_016_invalid_knots_timestamp_and_uncertainty_reject() {
        let mut bad_candidate = candidate();
        bad_candidate.regimes[0].points[1].speed_abs_rad_s = 0.0;
        assert!(evaluate_mission(
            &bad_candidate,
            &mission(vec![sample(0.0, 1.0, 0.0, 0.0), sample(1.0, 1.0, 0.0, 0.0)]),
        ).is_err());

        let bad_time = mission(vec![sample(1.0, 1.0, 0.0, 1.0), sample(1.0, 1.0, 0.0, 1.0)]);
        assert!(evaluate_mission(&candidate(), &bad_time).is_err());

        let bad_uncertainty = mission(vec![sample(0.0, 1.0, -0.1, 1.0), sample(1.0, 1.0, 0.0, 1.0)]);
        assert!(evaluate_mission(&candidate(), &bad_uncertainty).is_err());
    }

    #[test]
    fn mcjce_017_unknown_evidence_cannot_produce_a_feasible_decision() {
        let mut envelope = candidate();
        envelope.regimes[0].evidence.class = EvidenceClass::Unknown;
        let result = evaluate_mission(
            &envelope,
            &mission(vec![sample(0.0, 2.0, 0.0, 0.0), sample(0.1, 2.0, 0.0, 0.0)]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Unknown);
        assert_eq!(result.samples[0].evidence_class, Some(EvidenceClass::Unknown));
    }

    #[test]
    fn mcjce_018_vendor_evidence_class_is_preserved_not_promoted() {
        let mut envelope = candidate();
        envelope.regimes[0].evidence.class = EvidenceClass::VendorPhysicalTest;
        let result = evaluate_mission(
            &envelope,
            &mission(vec![sample(0.0, 2.0, 0.0, 0.0), sample(0.1, 2.0, 0.0, 0.0)]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Feasible);
        assert_eq!(result.samples[0].evidence_class, Some(EvidenceClass::VendorPhysicalTest));
    }

    #[test]
    fn mcjce_019_coast_only_mission_does_not_claim_torque_envelope_coverage() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![sample(0.0, 0.0, 0.0, 2.0), sample(1.0, 0.0, 0.0, 3.0)]),
        ).unwrap();
        assert_eq!(result.decision, MechanicalEnvelopeDecision::Feasible);
        assert_eq!(result.metrics.torque_demand_sample_count, 0);
        assert_eq!(result.metrics.coast_sample_count, 2);
        assert_eq!(result.metrics.torque_envelope_sample_coverage, None);
    }

    #[test]
    fn mcjce_020_evidence_references_and_scope_digests_survive_evaluation() {
        let result = evaluate_mission(
            &candidate(),
            &mission(vec![sample(0.0, 2.0, 0.0, 0.0), sample(0.1, 2.0, 0.0, 0.0)]),
        ).unwrap();
        assert_eq!(result.candidate_configuration_digest, CONFIG);
        assert_eq!(result.operating_condition_digest, CONDITIONS);
        assert_eq!(
            result.samples[0].evidence_reference.as_deref(),
            Some("https://example.invalid/test-fixture")
        );
        assert_eq!(
            result.samples[0].evidence_method.as_deref(),
            Some("synthetic known-answer fixture")
        );
    }

    #[test]
    fn mcjce_021_duplicate_regime_and_inconsistent_evidence_reject() {
        let mut duplicate = candidate();
        duplicate.regimes.push(duplicate.regimes[0].clone());
        assert!(evaluate_mission(
            &duplicate,
            &mission(vec![sample(0.0, 1.0, 0.0, 0.0), sample(1.0, 1.0, 0.0, 0.0)]),
        ).is_err());

        let mut mismatch = candidate();
        mismatch.regimes[0].evidence.configuration_digest = "other-config".to_owned();
        assert!(evaluate_mission(
            &mismatch,
            &mission(vec![sample(0.0, 1.0, 0.0, 0.0), sample(1.0, 1.0, 0.0, 0.0)]),
        ).is_err());
    }
}
