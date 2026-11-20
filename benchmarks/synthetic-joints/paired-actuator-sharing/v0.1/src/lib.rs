#![forbid(unsafe_code)]

//! Evidence-bound paired/multi-actuator load-sharing reference.
//!
//! The evaluator never assumes equal sharing and never converts current into
//! torque. It consumes per-unit output-equivalent torque intervals at one
//! declared reference plane, checks each unit against its own speed/torque/
//! duration/thermal envelope, and optionally reconciles the sum with an
//! independent joint-output torque interval.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    /// Synthetic known-answer values; arithmetic fixtures only, never physical observations.
    Synthetic,
    IndependentPhysical,
    SourceAuthorPhysical,
    VendorPhysical,
    ModelDerived,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitCapabilityDisposition {
    Continuous,
    PeakLimited,
    Infeasible,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointDisposition {
    Feasible,
    Infeasible,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TorqueSumConsistency {
    /// Overlapping intervals from a joint cross-check marked IndependentPhysical.
    Consistent,
    /// Non-overlapping intervals from an IndependentPhysical cross-check.
    Contradictory,
    /// Overlapping intervals from an explicitly Synthetic reference.
    SyntheticConsistent,
    /// Non-overlapping intervals from an explicitly Synthetic reference.
    SyntheticContradictory,
    Unknown,
    NotAvailable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActuatorUnit {
    pub actuator_id: String,
    pub configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub output_reference_plane_digest: String,
    pub reference_clock_id: String,
    /// Common physical instant used for the synchronized per-unit record.
    pub reference_instant_s: f64,

    /// Optional electrical-current magnitude observation. It is never used
    /// to infer output torque or torque share.
    pub current_magnitude_a: Option<Interval>,
    pub current_evidence_class: EvidenceClass,
    pub current_source_reference: Option<String>,
    pub current_method: Option<String>,
    pub current_calibration_digest: Option<String>,

    /// Output-equivalent delivered torque at the common reference plane.
    pub output_torque_nm: Option<Interval>,
    pub torque_evidence_class: EvidenceClass,
    pub torque_source_reference: Option<String>,
    pub torque_method: Option<String>,
    pub torque_calibration_digest: Option<String>,

    pub capability_evidence_class: EvidenceClass,
    pub capability_source_reference: Option<String>,
    pub capability_method: Option<String>,
    pub capability_digest: Option<String>,

    /// Conservative lower bounds on per-unit capability.
    pub continuous_torque_capability_lower_nm: Option<f64>,
    pub peak_torque_capability_lower_nm: Option<f64>,
    pub maximum_speed_capability_lower_rad_s: Option<f64>,
    /// Measured speed interval at the same output reference plane.
    pub output_speed_rad_s: Option<Interval>,
    /// Required time at peak load for the evaluated task segment.
    pub required_peak_duration_s: Option<f64>,
    /// Conservative remaining peak-duration budget for this unit.
    pub available_peak_duration_lower_s: Option<f64>,

    /// Per-unit measured temperature interval and conservative thermal-limit
    /// lower bound. This is not an inferred winding temperature unless the
    /// evidence method explicitly says so.
    pub temperature_c: Option<Interval>,
    pub thermal_limit_lower_c: Option<f64>,
    pub thermal_evidence_class: EvidenceClass,
    pub thermal_source_reference: Option<String>,
    pub thermal_method: Option<String>,
    pub thermal_calibration_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointTorqueCrossCheck {
    pub output_torque_nm: Interval,
    pub evidence_class: EvidenceClass,
    pub source_reference: String,
    pub method: String,
    pub calibration_digest: String,
    pub output_reference_plane_digest: String,
    pub reference_clock_id: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub reference_instant_s: f64,
    pub joint_configuration_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointEvaluationInput {
    pub joint_id: String,
    pub joint_configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub output_reference_plane_digest: String,
    pub reference_clock_id: String,
    pub reference_instant_s: f64,
    pub actuator_units: Vec<ActuatorUnit>,
    /// Optional joint-output torque reference. EvidenceClass distinguishes
    /// independent physical measurement from synthetic known-answer input.
    pub joint_torque_cross_check: Option<JointTorqueCrossCheck>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActuatorAssessment {
    pub actuator_id: String,
    pub delivered_torque_nm: Option<Interval>,
    /// Signed torque contribution divided by the sum of signed unit torques.
    /// Not present when the aggregate interval includes zero or torque evidence
    /// is missing. It is never derived from current share.
    pub load_share_fraction: Option<Interval>,
    /// Electrical current-magnitude share; never a torque-share substitute.
    pub current_share_fraction: Option<Interval>,
    pub continuous_torque_margin_nm: Option<f64>,
    pub peak_torque_margin_nm: Option<f64>,
    pub speed_margin_rad_s: Option<f64>,
    pub thermal_margin_c: Option<f64>,
    pub capability_disposition: UnitCapabilityDisposition,
    pub torque_evidence_class: EvidenceClass,
    pub torque_source_reference: Option<String>,
    pub torque_method: Option<String>,
    pub torque_calibration_digest: Option<String>,
    pub capability_evidence_class: EvidenceClass,
    pub capability_source_reference: Option<String>,
    pub capability_method: Option<String>,
    pub capability_digest: Option<String>,
    pub current_evidence_class: EvidenceClass,
    pub current_source_reference: Option<String>,
    pub current_method: Option<String>,
    pub current_calibration_digest: Option<String>,
    pub thermal_evidence_class: EvidenceClass,
    pub thermal_source_reference: Option<String>,
    pub thermal_method: Option<String>,
    pub thermal_calibration_digest: Option<String>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointEvaluation {
    pub joint_id: String,
    pub joint_configuration_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub output_reference_plane_digest: String,
    pub disposition: JointDisposition,
    pub aggregate_unit_torque_nm: Option<Interval>,
    pub aggregate_current_magnitude_a: Option<Interval>,
    /// Joint-output reference interval; inspect its evidence class before
    /// treating it as a physical measurement.
    pub joint_torque_cross_check_nm: Option<Interval>,
    pub joint_torque_cross_check_evidence: Option<JointTorqueCrossCheck>,
    /// Aggregate unit torque interval minus the declared joint-output reference.
    pub torque_sum_residual_nm: Option<Interval>,
    pub torque_sum_consistency: TorqueSumConsistency,
    pub hottest_unit_id: Option<String>,
    pub minimum_thermal_margin_c: Option<f64>,
    pub minimum_continuous_torque_margin_nm: Option<f64>,
    pub minimum_speed_margin_rad_s: Option<f64>,
    pub unit_assessments: Vec<ActuatorAssessment>,
    pub interpretation: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_interval(value: Interval) -> bool {
    value.lower.is_finite() && value.upper.is_finite() && value.lower <= value.upper
}

fn interval_add(a: Interval, b: Interval) -> Option<Interval> {
    let result = Interval {
        lower: a.lower + b.lower,
        upper: a.upper + b.upper,
    };
    valid_interval(result).then_some(result)
}

fn interval_subtract(a: Interval, b: Interval) -> Option<Interval> {
    let result = Interval {
        lower: a.lower - b.upper,
        upper: a.upper - b.lower,
    };
    valid_interval(result).then_some(result)
}

fn interval_divide(numerator: Interval, denominator: Interval) -> Option<Interval> {
    if denominator.lower <= 0.0 && denominator.upper >= 0.0 {
        return None;
    }
    let values = [
        numerator.lower / denominator.lower,
        numerator.lower / denominator.upper,
        numerator.upper / denominator.lower,
        numerator.upper / denominator.upper,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    Some(Interval {
        lower: values.iter().copied().fold(f64::INFINITY, f64::min),
        upper: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    })
}

fn maximum_absolute(interval: Interval) -> f64 {
    interval.lower.abs().max(interval.upper.abs())
}

fn validate_input(input: &JointEvaluationInput) -> Result<(), String> {
    if !nonempty(&input.joint_id)
        || !nonempty(&input.joint_configuration_digest)
        || !nonempty(&input.system_boundary_digest)
        || !nonempty(&input.operating_condition_digest)
        || !nonempty(&input.output_reference_plane_digest)
        || !nonempty(&input.reference_clock_id)
        || !input.reference_instant_s.is_finite()
    {
        return Err("joint identity, configuration, boundary, condition, reference plane and clock are required".to_owned());
    }
    if input.actuator_units.len() < 2 {
        return Err("at least two actuator units are required".to_owned());
    }

    let mut ids = std::collections::BTreeSet::new();
    for unit in &input.actuator_units {
        if !nonempty(&unit.actuator_id) || !ids.insert(unit.actuator_id.as_str()) {
            return Err("actuator IDs must be nonempty and unique".to_owned());
        }
        if !nonempty(&unit.configuration_digest)
            || !nonempty(&unit.system_boundary_digest)
            || !nonempty(&unit.operating_condition_digest)
            || !nonempty(&unit.output_reference_plane_digest)
            || !nonempty(&unit.reference_clock_id)
            || !unit.reference_instant_s.is_finite()
        {
            return Err(format!("actuator {} has incomplete configuration/boundary identity", unit.actuator_id));
        }
        if unit.system_boundary_digest != input.system_boundary_digest
            || unit.operating_condition_digest != input.operating_condition_digest
            || unit.output_reference_plane_digest != input.output_reference_plane_digest
            || unit.reference_clock_id != input.reference_clock_id
            || (unit.reference_instant_s - input.reference_instant_s).abs() > 1.0e-9
        {
            return Err(format!("actuator {} is not at the common joint boundary/condition/reference plane/clock", unit.actuator_id));
        }
        if unit.output_torque_nm.is_some_and(|v| !valid_interval(v))
            || unit.output_speed_rad_s.is_some_and(|v| !valid_interval(v))
            || unit.temperature_c.is_some_and(|v| !valid_interval(v))
            || unit.current_magnitude_a.is_some_and(|v| !valid_interval(v) || v.lower < 0.0)
        {
            return Err(format!("actuator {} has an invalid torque, speed or temperature interval", unit.actuator_id));
        }
        for (name, value) in [
            ("continuous torque", unit.continuous_torque_capability_lower_nm),
            ("peak torque", unit.peak_torque_capability_lower_nm),
            ("maximum speed", unit.maximum_speed_capability_lower_rad_s),
            ("thermal limit", unit.thermal_limit_lower_c),
            ("required peak duration", unit.required_peak_duration_s),
            ("available peak duration", unit.available_peak_duration_lower_s),
        ] {
            if value.is_some_and(|v| !v.is_finite()) {
                return Err(format!("actuator {} has non-finite {name}", unit.actuator_id));
            }
        }
        if unit.continuous_torque_capability_lower_nm.is_some_and(|v| v < 0.0)
            || unit.peak_torque_capability_lower_nm.is_some_and(|v| v < 0.0)
            || unit.maximum_speed_capability_lower_rad_s.is_some_and(|v| v < 0.0)
            || unit.required_peak_duration_s.is_some_and(|v| v < 0.0)
            || unit.available_peak_duration_lower_s.is_some_and(|v| v < 0.0)
        {
            return Err(format!("actuator {} has negative capability/duration values", unit.actuator_id));
        }
        if let (Some(continuous), Some(peak)) = (
            unit.continuous_torque_capability_lower_nm,
            unit.peak_torque_capability_lower_nm,
        ) {
            if peak < continuous {
                return Err(format!("actuator {} peak lower bound is below continuous lower bound", unit.actuator_id));
            }
        }
        if unit.torque_evidence_class != EvidenceClass::Unknown
            && (unit.torque_source_reference.as_deref().is_none_or(|v| !nonempty(v))
                || unit.torque_method.as_deref().is_none_or(|v| !nonempty(v))
                || unit.torque_calibration_digest.as_deref().is_none_or(|v| !nonempty(v)))
        {
            return Err(format!("actuator {} torque evidence is incomplete", unit.actuator_id));
        }
        if unit.thermal_evidence_class != EvidenceClass::Unknown
            && (unit.thermal_source_reference.as_deref().is_none_or(|v| !nonempty(v))
                || unit.thermal_method.as_deref().is_none_or(|v| !nonempty(v))
                || unit.thermal_calibration_digest.as_deref().is_none_or(|v| !nonempty(v)))
        {
            return Err(format!("actuator {} thermal evidence is incomplete", unit.actuator_id));
        }
        if unit.capability_evidence_class != EvidenceClass::Unknown
            && (unit.capability_source_reference.as_deref().is_none_or(|v| !nonempty(v))
                || unit.capability_method.as_deref().is_none_or(|v| !nonempty(v))
                || unit.capability_digest.as_deref().is_none_or(|v| !nonempty(v)))
        {
            return Err(format!("actuator {} capability evidence is incomplete", unit.actuator_id));
        }
        if unit.current_magnitude_a.is_some()
            && unit.current_evidence_class != EvidenceClass::Unknown
            && (unit.current_source_reference.as_deref().is_none_or(|v| !nonempty(v))
                || unit.current_method.as_deref().is_none_or(|v| !nonempty(v))
                || unit.current_calibration_digest.as_deref().is_none_or(|v| !nonempty(v)))
        {
            return Err(format!("actuator {} current evidence is incomplete", unit.actuator_id));
        }
    }

    if let Some(check) = &input.joint_torque_cross_check {
        if !valid_interval(check.output_torque_nm)
            || !nonempty(&check.source_reference)
            || !nonempty(&check.method)
            || !nonempty(&check.calibration_digest)
            || !matches!(
                check.evidence_class,
                EvidenceClass::IndependentPhysical | EvidenceClass::Synthetic
            )
            || !check.reference_instant_s.is_finite()
            || !nonempty(&check.joint_configuration_digest)
        {
            return Err("joint torque cross-check interval/evidence is invalid or unknown".to_owned());
        }
        if check.output_reference_plane_digest != input.output_reference_plane_digest
            || check.reference_clock_id != input.reference_clock_id
            || check.system_boundary_digest != input.system_boundary_digest
            || check.operating_condition_digest != input.operating_condition_digest
        {
            return Err("joint torque cross-check is not at the common reference plane/clock/boundary/condition".to_owned());
        }
    }
    Ok(())
}

fn assess_unit(
    unit: &ActuatorUnit,
    total_torque: Option<Interval>,
    total_current: Option<Interval>,
) -> ActuatorAssessment {
    let mut reasons = Vec::new();
    let mut infeasible = false;
    let mut unknown = false;

    let torque = unit.output_torque_nm;
    let demand_upper = torque.map(maximum_absolute);

    if torque.is_none() || unit.torque_evidence_class == EvidenceClass::Unknown {
        unknown = true;
        reasons.push("individual output-equivalent torque evidence is unknown".to_owned());
    }
    if unit.continuous_torque_capability_lower_nm.is_none()
        || unit.peak_torque_capability_lower_nm.is_none()
        || unit.capability_evidence_class == EvidenceClass::Unknown
    {
        unknown = true;
        reasons.push("continuous/peak torque capability lower bound or its evidence is missing".to_owned());
    }
    if unit.output_speed_rad_s.is_none() || unit.maximum_speed_capability_lower_rad_s.is_none() {
        unknown = true;
        reasons.push("loaded output speed or qualified speed limit is missing".to_owned());
    }
    if unit.temperature_c.is_none() || unit.thermal_limit_lower_c.is_none()
        || unit.thermal_evidence_class == EvidenceClass::Unknown
    {
        unknown = true;
        reasons.push("per-unit temperature or thermal-limit evidence is missing".to_owned());
    }

    let continuous_margin = demand_upper.zip(unit.continuous_torque_capability_lower_nm)
        .map(|(demand, limit)| limit - demand);
    let peak_margin = demand_upper.zip(unit.peak_torque_capability_lower_nm)
        .map(|(demand, limit)| limit - demand);
    let speed_margin = unit.output_speed_rad_s.zip(unit.maximum_speed_capability_lower_rad_s)
        .map(|(speed, limit)| limit - maximum_absolute(speed));
    let thermal_margin = unit.temperature_c.zip(unit.thermal_limit_lower_c)
        .map(|(temperature, limit)| limit - temperature.upper);

    if peak_margin.is_some_and(|margin| margin < 0.0) {
        infeasible = true;
        reasons.push("delivered torque exceeds the per-unit peak capability lower bound".to_owned());
    }
    if speed_margin.is_some_and(|margin| margin < 0.0) {
        infeasible = true;
        reasons.push("loaded speed exceeds the per-unit qualified speed limit".to_owned());
    }
    if thermal_margin.is_some_and(|margin| margin <= 0.0) {
        infeasible = true;
        reasons.push("temperature upper bound reaches/exceeds thermal-limit lower bound".to_owned());
    }

    let mut unit_disposition = if infeasible {
        UnitCapabilityDisposition::Infeasible
    } else if unknown {
        UnitCapabilityDisposition::Unknown
    } else if continuous_margin.is_some_and(|margin| margin >= 0.0) {
        UnitCapabilityDisposition::Continuous
    } else {
        match (
            unit.required_peak_duration_s,
            unit.available_peak_duration_lower_s,
            peak_margin,
        ) {
            (Some(required), Some(available), Some(margin)) if margin >= 0.0 && required <= available => {
                UnitCapabilityDisposition::PeakLimited
            }
            (Some(required), Some(available), Some(_)) if required > available => {
                infeasible = true;
                reasons.push("required peak duration exceeds the unit's remaining qualified duration".to_owned());
                UnitCapabilityDisposition::Infeasible
            }
            _ => {
                unknown = true;
                reasons.push("peak operation has no qualified remaining-duration budget".to_owned());
                UnitCapabilityDisposition::Unknown
            }
        }
    };

    // The load-share ratio is deliberately independent of capability status:
    // it is reported only from per-unit torque intervals and a non-zero total.
    let load_share_fraction = torque.zip(total_torque)
        .and_then(|(individual, total)| interval_divide(individual, total));
    let current_share_fraction = if unit.current_evidence_class == EvidenceClass::Unknown {
        None
    } else {
        unit.current_magnitude_a.zip(total_current)
            .and_then(|(individual, total)| interval_divide(individual, total))
    };

    if infeasible {
        unit_disposition = UnitCapabilityDisposition::Infeasible;
    } else if unknown && unit_disposition != UnitCapabilityDisposition::Infeasible {
        unit_disposition = UnitCapabilityDisposition::Unknown;
    }

    ActuatorAssessment {
        actuator_id: unit.actuator_id.clone(),
        delivered_torque_nm: torque,
        load_share_fraction,
        current_share_fraction,
        continuous_torque_margin_nm: continuous_margin,
        peak_torque_margin_nm: peak_margin,
        speed_margin_rad_s: speed_margin,
        thermal_margin_c: thermal_margin,
        capability_disposition: unit_disposition,
        torque_evidence_class: unit.torque_evidence_class,
        torque_source_reference: unit.torque_source_reference.clone(),
        torque_method: unit.torque_method.clone(),
        torque_calibration_digest: unit.torque_calibration_digest.clone(),
        capability_evidence_class: unit.capability_evidence_class,
        capability_source_reference: unit.capability_source_reference.clone(),
        capability_method: unit.capability_method.clone(),
        capability_digest: unit.capability_digest.clone(),
        current_evidence_class: unit.current_evidence_class,
        current_source_reference: unit.current_source_reference.clone(),
        current_method: unit.current_method.clone(),
        current_calibration_digest: unit.current_calibration_digest.clone(),
        thermal_evidence_class: unit.thermal_evidence_class,
        thermal_source_reference: unit.thermal_source_reference.clone(),
        thermal_method: unit.thermal_method.clone(),
        thermal_calibration_digest: unit.thermal_calibration_digest.clone(),
        reasons,
    }
}

/// Evaluates per-unit output torque, loaded speed, peak duration and thermal
/// margins. No equal-share assumption or current-to-torque conversion exists.
pub fn evaluate_joint(input: &JointEvaluationInput) -> Result<JointEvaluation, String> {
    validate_input(input)?;

    let mut aggregate: Option<Interval> = Some(Interval { lower: 0.0, upper: 0.0 });
    for unit in &input.actuator_units {
        aggregate = match (aggregate, unit.output_torque_nm) {
            (Some(sum), Some(contribution)) => interval_add(sum, contribution),
            _ => None,
        };
    }

    let mut aggregate_current: Option<Interval> = Some(Interval { lower: 0.0, upper: 0.0 });
    for unit in &input.actuator_units {
        aggregate_current = match (aggregate_current, unit.current_magnitude_a) {
            (Some(sum), Some(contribution)) if unit.current_evidence_class != EvidenceClass::Unknown => {
                interval_add(sum, contribution)
            }
            _ => None,
        };
    }

    let unit_assessments: Vec<ActuatorAssessment> = input.actuator_units
        .iter()
        .map(|unit| assess_unit(unit, aggregate, aggregate_current))
        .collect();

    let (joint_measured, residual, consistency) = match (
        &input.joint_torque_cross_check,
        aggregate,
    ) {
        (None, _) => (None, None, TorqueSumConsistency::NotAvailable),
        (Some(check), Some(sum)) => {
            if (check.reference_instant_s - input.reference_instant_s).abs() > 1.0e-9
                || check.joint_configuration_digest != input.joint_configuration_digest
            {
                return Err("joint torque cross-check instant/configuration does not match joint input".to_owned());
            }
            let residual = interval_subtract(sum, check.output_torque_nm)
                .ok_or_else(|| "joint torque residual interval overflowed".to_owned())?;
            let overlaps = residual.lower <= 0.0 && residual.upper >= 0.0;
            let consistency = match (check.evidence_class, overlaps) {
                (EvidenceClass::Synthetic, true) => TorqueSumConsistency::SyntheticConsistent,
                (EvidenceClass::Synthetic, false) => TorqueSumConsistency::SyntheticContradictory,
                (_, true) => TorqueSumConsistency::Consistent,
                (_, false) => TorqueSumConsistency::Contradictory,
            };
            (Some(check.output_torque_nm), Some(residual), consistency)
        }
        (Some(_), None) => (None, None, TorqueSumConsistency::Unknown),
    };

    let hottest_unit = input.actuator_units.iter()
        .filter_map(|unit| unit.temperature_c.map(|t| (unit, t.upper)))
        .max_by(|a, b| a.1.total_cmp(&b.1));
    let hottest_unit_id = hottest_unit.map(|(unit, _)| unit.actuator_id.clone());

    let minimum_thermal_margin = unit_assessments.iter()
        .filter_map(|assessment| assessment.thermal_margin_c)
        .reduce(f64::min);
    let minimum_continuous_margin = unit_assessments.iter()
        .filter_map(|assessment| assessment.continuous_torque_margin_nm)
        .reduce(f64::min);
    let minimum_speed_margin = unit_assessments.iter()
        .filter_map(|assessment| assessment.speed_margin_rad_s)
        .reduce(f64::min);

    let any_infeasible = unit_assessments.iter().any(|assessment| {
        assessment.capability_disposition == UnitCapabilityDisposition::Infeasible
    }) || matches!(
        consistency,
        TorqueSumConsistency::Contradictory | TorqueSumConsistency::SyntheticContradictory
    );

    let any_unknown = aggregate.is_none()
        || unit_assessments.iter().any(|assessment| {
            assessment.capability_disposition == UnitCapabilityDisposition::Unknown
        })
        || consistency == TorqueSumConsistency::Unknown;

    let disposition = if any_infeasible {
        JointDisposition::Infeasible
    } else if any_unknown {
        JointDisposition::Unknown
    } else {
        JointDisposition::Feasible
    };

    Ok(JointEvaluation {
        joint_id: input.joint_id.clone(),
        joint_configuration_digest: input.joint_configuration_digest.clone(),
        system_boundary_digest: input.system_boundary_digest.clone(),
        operating_condition_digest: input.operating_condition_digest.clone(),
        output_reference_plane_digest: input.output_reference_plane_digest.clone(),
        disposition,
        aggregate_unit_torque_nm: aggregate,
        aggregate_current_magnitude_a: aggregate_current,
        joint_torque_cross_check_nm: joint_measured,
        joint_torque_cross_check_evidence: input.joint_torque_cross_check.clone(),
        torque_sum_residual_nm: residual,
        torque_sum_consistency: consistency,
        hottest_unit_id,
        minimum_thermal_margin_c: minimum_thermal_margin,
        minimum_continuous_torque_margin_nm: minimum_continuous_margin,
        minimum_speed_margin_rad_s: minimum_speed_margin,
        unit_assessments,
        interpretation: "This is a per-unit torque/speed/duration/temperature envelope check only. Cross-check evidence class is preserved: Synthetic is arithmetic-only; only IndependentPhysical is an independent measurement. A Feasible result does not prove task success, full thermal life, current-to-torque accuracy, passivity or hardware/HRI safety.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDARY: &str = "complete-paired-joint-boundary-v1";
    const CONDITION: &str = "48V-25C-plate-A-v1";
    const PLANE: &str = "joint-output-shaft-v1";
    const CLOCK: &str = "ptp-clock-A-v1";

    fn interval(lower: f64, upper: f64) -> Interval {
        Interval { lower, upper }
    }

    fn unit(id: &str, torque: Interval, temperature: Interval) -> ActuatorUnit {
        ActuatorUnit {
            actuator_id: id.to_owned(),
            configuration_digest: format!("config-{id}"),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            output_reference_plane_digest: PLANE.to_owned(),
            reference_clock_id: CLOCK.to_owned(),
            reference_instant_s: 1.0,
            current_magnitude_a: None,
            current_evidence_class: EvidenceClass::Unknown,
            current_source_reference: None,
            current_method: None,
            current_calibration_digest: None,
            output_torque_nm: Some(torque),
            torque_evidence_class: EvidenceClass::Synthetic,
            torque_source_reference: Some(format!("urn:synthetic:paired-actuator-sharing:v0.1:torque:{id}")),
            torque_method: Some("Synthetic output-equivalent torque interval".to_owned()),
            torque_calibration_digest: Some(format!("cal-torque-{id}")),
            capability_evidence_class: EvidenceClass::Synthetic,
            capability_source_reference: Some(format!("urn:synthetic:paired-actuator-sharing:v0.1:capability:{id}")),
            capability_method: Some("Synthetic conservative capability envelope".to_owned()),
            capability_digest: Some(format!("capability-{id}-v1")),
            continuous_torque_capability_lower_nm: Some(10.0),
            peak_torque_capability_lower_nm: Some(15.0),
            maximum_speed_capability_lower_rad_s: Some(10.0),
            output_speed_rad_s: Some(interval(4.0, 5.0)),
            required_peak_duration_s: Some(0.2),
            available_peak_duration_lower_s: Some(0.5),
            temperature_c: Some(temperature),
            thermal_limit_lower_c: Some(70.0),
            thermal_evidence_class: EvidenceClass::Synthetic,
            thermal_source_reference: Some(format!("urn:synthetic:paired-actuator-sharing:v0.1:temperature:{id}")),
            thermal_method: Some("Synthetic temperature interval".to_owned()),
            thermal_calibration_digest: Some(format!("cal-temperature-{id}")),
        }
    }

    fn input(units: Vec<ActuatorUnit>) -> JointEvaluationInput {
        JointEvaluationInput {
            joint_id: "paired-knee".to_owned(),
            joint_configuration_digest: "paired-knee-config-v1".to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
            output_reference_plane_digest: PLANE.to_owned(),
            reference_clock_id: CLOCK.to_owned(),
            reference_instant_s: 1.0,
            actuator_units: units,
            joint_torque_cross_check: None,
        }
    }

    fn cross_check(torque: Interval) -> JointTorqueCrossCheck {
        JointTorqueCrossCheck {
            output_torque_nm: torque,
            evidence_class: EvidenceClass::Synthetic,
            source_reference: "urn:synthetic:paired-actuator-sharing:v0.1:joint-torque:known-answer-01".to_owned(),
            method: "Synthetic common-output torque interval for arithmetic consistency testing".to_owned(),
            calibration_digest: "joint-output-calibration-v1".to_owned(),
            output_reference_plane_digest: PLANE.to_owned(),
            reference_clock_id: CLOCK.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            reference_instant_s: 1.0,
            joint_configuration_digest: "paired-knee-config-v1".to_owned(),
            operating_condition_digest: CONDITION.to_owned(),
        }
    }

    #[test]
    fn sharing_001_sum_of_individual_intervals_is_conservative() {
        let report = evaluate_joint(&input(vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ])).unwrap();
        assert_eq!(report.aggregate_unit_torque_nm, Some(interval(9.0, 11.0)));
        assert_eq!(report.disposition, JointDisposition::Feasible);
    }

    #[test]
    fn sharing_002_load_share_is_reported_from_torque_not_assumed_equal() {
        let report = evaluate_joint(&input(vec![
            unit("A", interval(2.0, 3.0), interval(40.0, 42.0)),
            unit("B", interval(7.0, 8.0), interval(41.0, 43.0)),
        ])).unwrap();
        let share_a = report.unit_assessments[0].load_share_fraction.unwrap();
        assert!(share_a.lower < 0.3 && share_a.upper < 0.4);
        assert_ne!(share_a, interval(0.5, 0.5));
    }

    #[test]
    fn sharing_003_missing_per_unit_torque_does_not_fabricate_fifty_fifty() {
        let mut a = unit("A", interval(2.0, 3.0), interval(40.0, 42.0));
        a.output_torque_nm = None;
        a.torque_evidence_class = EvidenceClass::Unknown;
        let report = evaluate_joint(&input(vec![
            a,
            unit("B", interval(7.0, 8.0), interval(41.0, 43.0)),
        ])).unwrap();
        assert_eq!(report.disposition, JointDisposition::Unknown);
        assert_eq!(report.unit_assessments[0].load_share_fraction, None);
    }

    #[test]
    fn sharing_004_equal_commands_are_not_part_of_the_torque_share_model() {
        let report = evaluate_joint(&input(vec![
            unit("A", interval(1.0, 2.0), interval(40.0, 42.0)),
            unit("B", interval(8.0, 9.0), interval(41.0, 43.0)),
        ])).unwrap();
        assert!(report.unit_assessments[0].load_share_fraction.unwrap().upper < 0.25);
        assert!(report.unit_assessments[1].load_share_fraction.unwrap().lower > 0.75);
    }

    #[test]
    fn sharing_005_hot_unit_not_hidden_by_average_temperature() {
        let report = evaluate_joint(&input(vec![
            unit("cool", interval(4.0, 5.0), interval(45.0, 48.0)),
            unit("hot", interval(4.0, 5.0), interval(68.0, 71.0)),
        ])).unwrap();
        assert_eq!(report.disposition, JointDisposition::Infeasible);
        assert_eq!(report.hottest_unit_id.as_deref(), Some("hot"));
        assert_eq!(report.minimum_thermal_margin_c, Some(-1.0));
    }

    #[test]
    fn sharing_006_joint_sum_can_pass_while_one_unit_exceeds_continuous_limit() {
        let mut units = vec![
            unit("A", interval(11.0, 12.0), interval(40.0, 42.0)),
            unit("B", interval(1.0, 2.0), interval(41.0, 43.0)),
        ];
        units[0].peak_torque_capability_lower_nm = Some(15.0);
        units[0].continuous_torque_capability_lower_nm = Some(10.0);
        units[0].required_peak_duration_s = Some(0.7);
        units[0].available_peak_duration_lower_s = Some(0.5);
        let report = evaluate_joint(&input(units)).unwrap();
        assert_eq!(report.disposition, JointDisposition::Infeasible);
        assert_eq!(report.unit_assessments[0].capability_disposition, UnitCapabilityDisposition::Infeasible);
    }

    #[test]
    fn sharing_007_loaded_speed_not_no_load_speed_governs() {
        let mut units = vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(4.0, 5.0), interval(41.0, 43.0)),
        ];
        units[1].output_speed_rad_s = Some(interval(10.1, 10.5));
        let report = evaluate_joint(&input(units)).unwrap();
        assert_eq!(report.disposition, JointDisposition::Infeasible);
        assert!(report.unit_assessments[1].speed_margin_rad_s.unwrap() < 0.0);
    }

    #[test]
    fn sharing_008_peak_duration_is_per_unit_and_not_reset_by_other_unit() {
        let mut units = vec![
            unit("A", interval(11.0, 12.0), interval(40.0, 42.0)),
            unit("B", interval(4.0, 5.0), interval(41.0, 43.0)),
        ];
        units[0].required_peak_duration_s = Some(0.8);
        units[0].available_peak_duration_lower_s = Some(0.5);
        let report = evaluate_joint(&input(units)).unwrap();
        assert_eq!(report.disposition, JointDisposition::Infeasible);
        assert!(report.unit_assessments[0].reasons.iter().any(|reason| reason.contains("duration")));
    }

    #[test]
    fn sharing_009_cross_check_confirms_sum_when_intervals_overlap() {
        let mut input = input(vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ]);
        input.joint_torque_cross_check = Some(cross_check(interval(10.0, 12.0)));
        let report = evaluate_joint(&input).unwrap();
        assert_eq!(report.torque_sum_consistency, TorqueSumConsistency::SyntheticConsistent);
        assert_eq!(report.torque_sum_residual_nm, Some(interval(-3.0, 1.0)));
    }

    #[test]
    fn sharing_010_cross_check_contradiction_is_not_ignored() {
        let mut input = input(vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ]);
        input.joint_torque_cross_check = Some(cross_check(interval(20.0, 22.0)));
        let report = evaluate_joint(&input).unwrap();
        assert_eq!(report.disposition, JointDisposition::Infeasible);
        assert_eq!(report.torque_sum_consistency, TorqueSumConsistency::SyntheticContradictory);
    }

    #[test]
    fn sharing_011_mismatched_reference_planes_reject() {
        let mut units = vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ];
        units[1].output_reference_plane_digest = "motor-shaft-plane".to_owned();
        assert!(evaluate_joint(&input(units)).is_err());
    }

    #[test]
    fn sharing_012_mismatched_clock_condition_or_boundary_reject() {
        let mut units = vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ];
        units[1].reference_clock_id = "free-running-clock".to_owned();
        assert!(evaluate_joint(&input(units)).is_err());

        let mut units = vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ];
        units[1].operating_condition_digest = "different-cooling".to_owned();
        assert!(evaluate_joint(&input(units)).is_err());
    }

    #[test]
    fn sharing_013_unknown_current_or_torque_never_becomes_zero_or_equal() {
        let mut a = unit("A", interval(4.0, 5.0), interval(40.0, 42.0));
        a.torque_evidence_class = EvidenceClass::Unknown;
        let report = evaluate_joint(&input(vec![
            a,
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ])).unwrap();
        assert_eq!(report.disposition, JointDisposition::Unknown);
        assert_eq!(report.unit_assessments[0].load_share_fraction, None);
    }

    #[test]
    fn sharing_014_vendor_and_model_evidence_are_preserved() {
        let mut a = unit("A", interval(4.0, 5.0), interval(40.0, 42.0));
        a.torque_evidence_class = EvidenceClass::ModelDerived;
        a.thermal_evidence_class = EvidenceClass::VendorPhysical;
        let report = evaluate_joint(&input(vec![
            a,
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ])).unwrap();
        assert_eq!(report.unit_assessments[0].torque_evidence_class, EvidenceClass::ModelDerived);
        assert_eq!(report.unit_assessments[0].thermal_evidence_class, EvidenceClass::VendorPhysical);
    }

    #[test]
    fn sharing_015_current_share_is_reported_separately_from_torque_share() {
        let mut a = unit("A", interval(2.0, 3.0), interval(40.0, 42.0));
        let mut b = unit("B", interval(7.0, 8.0), interval(41.0, 43.0));
        a.current_magnitude_a = Some(interval(4.0, 5.0));
        b.current_magnitude_a = Some(interval(8.0, 9.0));
        for u in [&mut a, &mut b] {
            u.current_evidence_class = EvidenceClass::Synthetic;
            u.current_source_reference = Some("urn:synthetic:paired-actuator-sharing:v0.1:current:run".to_owned());
            u.current_method = Some("Synthetic current magnitude interval".to_owned());
            u.current_calibration_digest = Some("current-calibration-v1".to_owned());
        }
        let report = evaluate_joint(&input(vec![a, b])).unwrap();
        assert!(report.unit_assessments[0].current_share_fraction.is_some());
        assert_ne!(
            report.unit_assessments[0].current_share_fraction,
            report.unit_assessments[0].load_share_fraction
        );
        // Current share is separately recorded and never used to replace
        // output torque share or alter the torque-capability decision.
    }

    #[test]
    fn sharing_016_joint_cross_check_must_match_instant_and_configuration() {
        let mut input = input(vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ]);
        let mut check = cross_check(interval(9.0, 11.0));
        check.reference_instant_s = 2.0;
        input.joint_torque_cross_check = Some(check);
        assert!(evaluate_joint(&input).is_err());
    }

    #[test]
    fn sharing_017_unit_evidence_receipts_survive_evaluation() {
        let mut a = unit("A", interval(4.0, 5.0), interval(40.0, 42.0));
        a.current_magnitude_a = Some(interval(4.0, 5.0));
        a.current_evidence_class = EvidenceClass::Synthetic;
        a.current_source_reference = Some("urn:synthetic:paired-actuator-sharing:v0.1:current:A".to_owned());
        a.current_method = Some("fixture current method".to_owned());
        a.current_calibration_digest = Some("current-cal-A".to_owned());
        let report = evaluate_joint(&input(vec![
            a,
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ])).unwrap();
        let assessment = &report.unit_assessments[0];
        assert_eq!(assessment.torque_source_reference.as_deref(), Some("https://example.invalid/torque/A"));
        assert_eq!(assessment.capability_digest.as_deref(), Some("capability-A-v1"));
        assert_eq!(assessment.current_calibration_digest.as_deref(), Some("current-cal-A"));
        assert_eq!(assessment.thermal_calibration_digest.as_deref(), Some("cal-temperature-A"));
    }

    #[test]
    fn sharing_018_unknown_capability_provenance_cannot_be_feasible() {
        let mut a = unit("A", interval(4.0, 5.0), interval(40.0, 42.0));
        a.capability_evidence_class = EvidenceClass::Unknown;
        a.capability_source_reference = None;
        a.capability_method = None;
        a.capability_digest = None;
        let report = evaluate_joint(&input(vec![
            a,
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ])).unwrap();
        assert_eq!(report.disposition, JointDisposition::Unknown);
    }

    #[test]
    fn sharing_019_duplicate_ids_and_invalid_capability_reject() {
        let a = unit("same", interval(4.0, 5.0), interval(40.0, 42.0));
        let b = unit("same", interval(5.0, 6.0), interval(41.0, 43.0));
        assert!(evaluate_joint(&input(vec![a, b])).is_err());

        let mut a = unit("A", interval(4.0, 5.0), interval(40.0, 42.0));
        a.peak_torque_capability_lower_nm = Some(5.0);
        assert!(evaluate_joint(&input(vec![
            a,
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ])).is_err());
    }

    #[test]
    fn sharing_020_joint_torque_cross_check_rejects_vendor_physical_class() {
        let mut input = input(vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ]);
        let mut check = cross_check(interval(9.0, 11.0));
        check.evidence_class = EvidenceClass::VendorPhysical;
        input.joint_torque_cross_check = Some(check);
        assert!(evaluate_joint(&input).is_err());
    }

    #[test]
    fn sharing_023_synthetic_cross_check_is_not_promoted_to_physical_measurement() {
        let mut input = input(vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ]);
        input.joint_torque_cross_check = Some(cross_check(interval(9.0, 11.0)));
        let report = evaluate_joint(&input).unwrap();
        assert_eq!(report.torque_sum_consistency, TorqueSumConsistency::SyntheticConsistent);
        assert_eq!(report.joint_torque_cross_check_nm, Some(interval(9.0, 11.0)));
        assert_eq!(
            report.joint_torque_cross_check_evidence.as_ref().unwrap().evidence_class,
            EvidenceClass::Synthetic
        );
        assert!(report.interpretation.contains("Synthetic is arithmetic-only"));
    }

    #[test]
    fn sharing_024_independent_physical_cross_check_remains_distinguishable() {
        let mut input = input(vec![
            unit("A", interval(4.0, 5.0), interval(40.0, 42.0)),
            unit("B", interval(5.0, 6.0), interval(41.0, 43.0)),
        ]);
        let mut check = cross_check(interval(9.0, 11.0));
        check.evidence_class = EvidenceClass::IndependentPhysical;
        check.source_reference = "urn:measurement:joint-output-transducer:run-01".to_owned();
        input.joint_torque_cross_check = Some(check);
        let report = evaluate_joint(&input).unwrap();
        assert_eq!(
            report.joint_torque_cross_check_evidence.as_ref().unwrap().evidence_class,
            EvidenceClass::IndependentPhysical
        );
        assert_eq!(report.torque_sum_consistency, TorqueSumConsistency::Consistent);
    }

    #[test]
    fn sharing_021_known_peak_overload_precedes_missing_continuous_rating() {
        let mut a = unit("A", interval(16.0, 17.0), interval(40.0, 42.0));
        a.continuous_torque_capability_lower_nm = None;
        let report = evaluate_joint(&input(vec![
            a,
            unit("B", interval(4.0, 5.0), interval(41.0, 43.0)),
        ])).unwrap();
        assert_eq!(report.unit_assessments[0].capability_disposition, UnitCapabilityDisposition::Infeasible);
        assert_eq!(report.disposition, JointDisposition::Infeasible);
    }

    #[test]
    fn sharing_022_aggregate_temperature_is_not_used_as_unit_temperature() {
        let report = evaluate_joint(&input(vec![
            unit("A", interval(4.0, 5.0), interval(30.0, 32.0)),
            unit("B", interval(5.0, 6.0), interval(60.0, 65.0)),
        ])).unwrap();
        assert_eq!(report.hottest_unit_id.as_deref(), Some("B"));
        assert_eq!(report.minimum_thermal_margin_c, Some(5.0));
    }
}
