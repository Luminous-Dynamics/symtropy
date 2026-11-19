#![forbid(unsafe_code)]

//! Bounded synchronized mechanical-port metrology.
//!
//! This M1 reference does not calibrate instruments or establish traceability.
//! It accepts only externally supported bounded intervals and provenance,
//! expands each channel for bounded clock/sample/filter skew using a declared
//! absolute slew-rate bound, then computes the exact rectangular interval
//! product tau * omega. It deliberately makes no probability claim.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    OutputTorque,
    AngularVelocity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundRepresentation {
    ValidatedBound,
    StandardUncertainty,
    ExpandedUncertainty,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationStatus {
    InDate,
    Expired,
    OutOfTolerance,
    Unknown,
    NotCalibrated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceabilityStatus {
    Established,
    Partial,
    NotEstablished,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    IndependentPhysical,
    SourceAuthorPhysical,
    VendorPhysical,
    ModelDerived,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementDisposition {
    QualifiedBoundedEstimate,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSign {
    PositiveMechanicalPower,
    NegativeMechanicalPower,
    CrossesZeroOrAmbiguous,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelMeasurement {
    pub kind: ChannelKind,
    /// Conservative interval before time-alignment expansion.
    pub value: NumericInterval,
    /// Physical range of the installed sensor.
    pub sensor_range: NumericInterval,
    /// Range covered by the referenced calibration.
    pub calibrated_range: NumericInterval,
    pub bound_representation: BoundRepresentation,
    pub calibration_status: CalibrationStatus,
    pub traceability_status: TraceabilityStatus,
    pub evidence_class: EvidenceClass,

    pub sensor_serial: String,
    pub calibration_sensor_serial: String,
    pub calibration_certificate_reference: String,
    pub reference_standard_reference: String,
    pub calibration_chain_digest: String,
    pub calibration_digest: String,

    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
    pub source_reference: String,
    pub measurement_method: String,
    pub raw_trace_digest: String,

    /// All local timestamps have been mapped to this common reference clock.
    pub reference_clock_id: String,
    pub clock_mapping_digest: String,
    /// Desired common physical instant to which both channels are aligned.
    pub reference_instant_s: f64,
    /// Estimated channel sample instant relative to reference_instant_s.
    pub sample_time_offset_s: f64,
    /// Absolute bound on residual clock mapping error.
    pub clock_offset_uncertainty_s: f64,
    /// Absolute bound on acquisition timestamp jitter.
    pub timestamp_jitter_bound_s: f64,
    /// Measured filter group delay before compensation.
    pub filter_group_delay_s: f64,
    /// Delay compensated in the time alignment.
    pub filter_delay_compensation_s: f64,
    /// Absolute bound on residual filter-delay error.
    pub filter_delay_uncertainty_s: f64,

    /// Validated absolute signal slew bound near the aligned instant.
    /// Torque channel: N m/s. Velocity channel: (rad/s)/s.
    pub absolute_slew_bound_per_s: f64,
    pub slew_bound_provenance_digest: String,
    pub slew_bound_source_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelAlignment {
    pub original_interval: NumericInterval,
    pub aligned_interval: NumericInterval,
    pub maximum_time_skew_s: f64,
    pub timing_expansion: f64,
    pub evidence_class: EvidenceClass,
    pub sensor_serial: String,
    pub calibration_digest: String,
    pub calibration_chain_digest: String,
    pub clock_mapping_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortPowerAssessment {
    pub disposition: MeasurementDisposition,
    pub power_sign: Option<PowerSign>,
    /// Exact rectangular interval product before time-skew expansion.
    pub pre_timing_power_w: Option<NumericInterval>,
    /// Exact rectangular interval product after time-skew expansion.
    pub aligned_power_w: Option<NumericInterval>,
    /// Maximum outward expansion contributed by time alignment, in watts.
    pub added_timing_power_bound_w: Option<f64>,
    pub torque_alignment: Option<ChannelAlignment>,
    pub angular_velocity_alignment: Option<ChannelAlignment>,
    pub evidence_classes: Vec<EvidenceClass>,
    pub reason: Option<String>,
    pub interpretation: String,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite_interval(interval: NumericInterval) -> bool {
    interval.lower.is_finite()
        && interval.upper.is_finite()
        && interval.lower <= interval.upper
}

fn unknown(reason: impl Into<String>) -> PortPowerAssessment {
    PortPowerAssessment {
        disposition: MeasurementDisposition::Unknown,
        power_sign: None,
        pre_timing_power_w: None,
        aligned_power_w: None,
        added_timing_power_bound_w: None,
        torque_alignment: None,
        angular_velocity_alignment: None,
        evidence_classes: Vec::new(),
        reason: Some(reason.into()),
        interpretation: "No qualified mechanical-port power estimate is available. Unknown is not zero and is not a pass.".to_owned(),
    }
}

fn validate_numeric_channel(channel: &ChannelMeasurement) -> Result<(), String> {
    if !finite_interval(channel.value)
        || !finite_interval(channel.sensor_range)
        || !finite_interval(channel.calibrated_range)
    {
        return Err("channel intervals must be finite and ordered".to_owned());
    }
    if !channel.reference_instant_s.is_finite()
        || !channel.sample_time_offset_s.is_finite()
        || !channel.clock_offset_uncertainty_s.is_finite()
        || !channel.timestamp_jitter_bound_s.is_finite()
        || !channel.filter_group_delay_s.is_finite()
        || !channel.filter_delay_compensation_s.is_finite()
        || !channel.filter_delay_uncertainty_s.is_finite()
        || !channel.absolute_slew_bound_per_s.is_finite()
    {
        return Err("channel timing, delay and slew fields must be finite".to_owned());
    }
    if channel.clock_offset_uncertainty_s < 0.0
        || channel.timestamp_jitter_bound_s < 0.0
        || channel.filter_group_delay_s < 0.0
        || channel.filter_delay_uncertainty_s < 0.0
        || channel.absolute_slew_bound_per_s < 0.0
    {
        return Err("clock, jitter, filter-delay uncertainty and slew bounds must be nonnegative".to_owned());
    }
    if channel.value.lower < channel.sensor_range.lower
        || channel.value.upper > channel.sensor_range.upper
    {
        return Err("measured interval exceeds the declared sensor range".to_owned());
    }
    if !finite_interval(channel.calibrated_range) || !finite_interval(channel.sensor_range) {
        return Err("sensor and calibration ranges must be finite and ordered".to_owned());
    }
    Ok(())
}

fn validate_channel_provenance(channel: &ChannelMeasurement) -> Option<String> {
    if channel.bound_representation != BoundRepresentation::ValidatedBound {
        return Some("input is not an externally validated deterministic bound; standard/expanded uncertainty is not silently treated as a hard bound".to_owned());
    }
    if channel.calibration_status != CalibrationStatus::InDate {
        return Some(format!("calibration status is {:?}", channel.calibration_status));
    }
    if channel.traceability_status != TraceabilityStatus::Established {
        return Some(format!("traceability status is {:?}", channel.traceability_status));
    }
    if channel.evidence_class == EvidenceClass::Unknown {
        return Some("measurement evidence class is Unknown".to_owned());
    }
    if !nonempty(&channel.sensor_serial)
        || !nonempty(&channel.calibration_sensor_serial)
        || channel.sensor_serial != channel.calibration_sensor_serial
    {
        return Some("calibration certificate sensor serial does not match the measured channel".to_owned());
    }
    if channel.calibrated_range.lower > channel.value.lower
        || channel.calibrated_range.upper < channel.value.upper
    {
        return Some("measured interval lies outside the calibration-covered operating range".to_owned());
    }
    if !nonempty(&channel.calibration_certificate_reference)
        || !nonempty(&channel.reference_standard_reference)
        || !nonempty(&channel.calibration_chain_digest)
        || !nonempty(&channel.calibration_digest)
    {
        return Some("calibration certificate, reference standard or traceability-chain digest is missing".to_owned());
    }
    if !nonempty(&channel.source_reference)
        || !nonempty(&channel.measurement_method)
        || !nonempty(&channel.raw_trace_digest)
    {
        return Some("measurement source or method is missing".to_owned());
    }
    if !nonempty(&channel.system_boundary_digest)
        || !nonempty(&channel.operating_condition_digest)
    {
        return Some("system-boundary or operating-condition digest is missing".to_owned());
    }
    if !nonempty(&channel.reference_clock_id) || !nonempty(&channel.clock_mapping_digest) {
        return Some("common reference clock or clock-mapping digest is missing".to_owned());
    }
    if !nonempty(&channel.slew_bound_provenance_digest)
        || !nonempty(&channel.slew_bound_source_reference)
    {
        return Some("signal-slew bound lacks source provenance".to_owned());
    }
    None
}

fn product_interval(
    left: NumericInterval,
    right: NumericInterval,
) -> Result<NumericInterval, String> {
    let products = [
        left.lower * right.lower,
        left.lower * right.upper,
        left.upper * right.lower,
        left.upper * right.upper,
    ];
    if products.iter().any(|value| !value.is_finite()) {
        return Err("interval product overflowed or became non-finite".to_owned());
    }
    let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
    let upper = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Ok(NumericInterval { lower, upper })
}

fn power_sign(interval: NumericInterval) -> PowerSign {
    if interval.lower > 0.0 {
        PowerSign::PositiveMechanicalPower
    } else if interval.upper < 0.0 {
        PowerSign::NegativeMechanicalPower
    } else {
        PowerSign::CrossesZeroOrAmbiguous
    }
}

fn align_channel(channel: &ChannelMeasurement) -> Result<Option<ChannelAlignment>, String> {
    let residual_filter_delay =
        channel.filter_group_delay_s - channel.filter_delay_compensation_s;
    let maximum_time_skew_s = channel.sample_time_offset_s.abs()
        + channel.clock_offset_uncertainty_s
        + channel.timestamp_jitter_bound_s
        + residual_filter_delay.abs()
        + channel.filter_delay_uncertainty_s;
    if !maximum_time_skew_s.is_finite() {
        return Err("combined channel time-skew bound is non-finite".to_owned());
    }

    let timing_expansion =
        channel.absolute_slew_bound_per_s * maximum_time_skew_s;
    if !timing_expansion.is_finite() {
        return Err("timing uncertainty expansion is non-finite".to_owned());
    }

    let aligned_interval = NumericInterval {
        lower: channel.value.lower - timing_expansion,
        upper: channel.value.upper + timing_expansion,
    };
    if !finite_interval(aligned_interval) {
        return Err("time-aligned channel interval is non-finite or inverted".to_owned());
    }
    if aligned_interval.lower < channel.calibrated_range.lower
        || aligned_interval.upper > channel.calibrated_range.upper
    {
        return Ok(None);
    }

    Ok(Some(ChannelAlignment {
        original_interval: channel.value,
        aligned_interval,
        maximum_time_skew_s,
        timing_expansion,
        evidence_class: channel.evidence_class,
        sensor_serial: channel.sensor_serial.clone(),
        calibration_digest: channel.calibration_digest.clone(),
        calibration_certificate_reference: channel.calibration_certificate_reference.clone(),
        reference_standard_reference: channel.reference_standard_reference.clone(),
        calibration_chain_digest: channel.calibration_chain_digest.clone(),
        clock_mapping_digest: channel.clock_mapping_digest.clone(),
        reference_clock_id: channel.reference_clock_id.clone(),
        reference_instant_s: channel.reference_instant_s,
        raw_trace_digest: channel.raw_trace_digest.clone(),
        source_reference: channel.source_reference.clone(),
        measurement_method: channel.measurement_method.clone(),
        slew_bound_provenance_digest: channel.slew_bound_provenance_digest.clone(),
        slew_bound_source_reference: channel.slew_bound_source_reference.clone(),
        system_boundary_digest: channel.system_boundary_digest.clone(),
        operating_condition_digest: channel.operating_condition_digest.clone(),
    }))
}

/// Computes a bounded mechanical-port power interval tau*omega.
///
/// The torque and angular-velocity samples must be expressed relative to the
/// same reference instant, common clock, system boundary and operating
/// conditions. Clock, acquisition jitter, residual filter group delay and
/// filter-delay uncertainty contribute to each channel's time-skew bound.
/// The channel's validated absolute slew bound expands its measured interval.
pub fn evaluate_mechanical_port_power(
    torque: &ChannelMeasurement,
    angular_velocity: &ChannelMeasurement,
) -> Result<PortPowerAssessment, String> {
    validate_numeric_channel(torque)?;
    validate_numeric_channel(angular_velocity)?;

    if torque.kind != ChannelKind::OutputTorque {
        return Err("first channel must be output torque in N m".to_owned());
    }
    if angular_velocity.kind != ChannelKind::AngularVelocity {
        return Err("second channel must be angular velocity in rad/s".to_owned());
    }

    let mut unknown_reasons = Vec::new();
    if let Some(reason) = validate_channel_provenance(torque) {
        unknown_reasons.push(format!("torque: {reason}"));
    }
    if let Some(reason) = validate_channel_provenance(angular_velocity) {
        unknown_reasons.push(format!("angular velocity: {reason}"));
    }
    if !unknown_reasons.is_empty() {
        return Ok(unknown(unknown_reasons.join("; ")));
    }

    if torque.reference_clock_id != angular_velocity.reference_clock_id {
        return Ok(unknown("torque and angular velocity do not share a mapped reference clock"));
    }
    if (torque.reference_instant_s - angular_velocity.reference_instant_s).abs() > 1.0e-9 {
        return Ok(unknown("torque and angular velocity are not referenced to the same physical instant"));
    }
    if torque.system_boundary_digest != angular_velocity.system_boundary_digest {
        return Ok(unknown("torque and angular velocity belong to different system boundaries"));
    }
    if torque.operating_condition_digest != angular_velocity.operating_condition_digest {
        return Ok(unknown("torque and angular velocity belong to different operating conditions"));
    }

    let Some(torque_alignment) = align_channel(torque)? else {
        return Ok(unknown("time-expanded torque interval extends outside the calibration-covered operating range"));
    };
    let Some(velocity_alignment) = align_channel(angular_velocity)? else {
        return Ok(unknown("time-expanded angular-velocity interval extends outside the calibration-covered operating range"));
    };

    let pre_timing_power =
        product_interval(torque.value, angular_velocity.value)?;
    let aligned_power = product_interval(
        torque_alignment.aligned_interval,
        velocity_alignment.aligned_interval,
    )?;

    // Since both channel intervals only expand, the aligned product interval
    // contains the pre-timing product interval. Report the greatest outward
    // endpoint movement as a symmetric timing contribution, without
    // converting it into a statistical standard uncertainty.
    let added_timing_power_bound_w = (pre_timing_power.lower - aligned_power.lower)
        .max(aligned_power.upper - pre_timing_power.upper)
        .max(0.0);
    if !added_timing_power_bound_w.is_finite() {
        return Err("timing-induced power expansion is non-finite".to_owned());
    }

    Ok(PortPowerAssessment {
        disposition: MeasurementDisposition::QualifiedBoundedEstimate,
        power_sign: Some(power_sign(aligned_power)),
        pre_timing_power_w: Some(pre_timing_power),
        aligned_power_w: Some(aligned_power),
        added_timing_power_bound_w: Some(added_timing_power_bound_w),
        torque_alignment: Some(torque_alignment),
        angular_velocity_alignment: Some(velocity_alignment),
        evidence_classes: vec![torque.evidence_class, angular_velocity.evidence_class],
        reason: None,
        interpretation: "Deterministic bounded mechanical-port power only. Negative mechanical power is not proof of DC-bus return or stored electrical regeneration. No energy integral, statistical confidence interval or whole-robot safety claim is produced.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(kind: ChannelKind, value: NumericInterval) -> ChannelMeasurement {
        ChannelMeasurement {
            kind,
            value,
            sensor_range: NumericInterval { lower: -200.0, upper: 200.0 },
            calibrated_range: NumericInterval { lower: -200.0, upper: 200.0 },
            bound_representation: BoundRepresentation::ValidatedBound,
            calibration_status: CalibrationStatus::InDate,
            traceability_status: TraceabilityStatus::Established,
            evidence_class: EvidenceClass::IndependentPhysical,
            sensor_serial: match kind {
                ChannelKind::OutputTorque => "torque-sensor-serial-01",
                ChannelKind::AngularVelocity => "encoder-serial-01",
            }.to_owned(),
            calibration_sensor_serial: match kind {
                ChannelKind::OutputTorque => "torque-sensor-serial-01",
                ChannelKind::AngularVelocity => "encoder-serial-01",
            }.to_owned(),
            calibration_certificate_reference: "https://example.invalid/calibration/cert-01".to_owned(),
            reference_standard_reference: "https://example.invalid/reference/standard-01".to_owned(),
            calibration_chain_digest: "traceability-chain-digest-01".to_owned(),
            calibration_digest: match kind {
                ChannelKind::OutputTorque => "torque-calibration-digest-01",
                ChannelKind::AngularVelocity => "encoder-calibration-digest-01",
            }.to_owned(),
            system_boundary_digest: "complete-installed-joint-boundary-01".to_owned(),
            operating_condition_digest: "48V-25C-cooling-A-01".to_owned(),
            source_reference: "https://example.invalid/measurement/run-01".to_owned(),
            measurement_method: "Synthetic known-answer bounded calibration fixture".to_owned(),
            raw_trace_digest: "raw-trace-digest-01".to_owned(),
            reference_clock_id: "ptp-clock-domain-01".to_owned(),
            clock_mapping_digest: "clock-map-digest-01".to_owned(),
            reference_instant_s: 1.0,
            sample_time_offset_s: 0.0,
            clock_offset_uncertainty_s: 0.0,
            timestamp_jitter_bound_s: 0.0,
            filter_group_delay_s: 0.0,
            filter_delay_compensation_s: 0.0,
            filter_delay_uncertainty_s: 0.0,
            absolute_slew_bound_per_s: 0.0,
            slew_bound_provenance_digest: "slew-bound-digest-01".to_owned(),
            slew_bound_source_reference: "https://example.invalid/slew-bound".to_owned(),
        }
    }

    fn pair() -> (ChannelMeasurement, ChannelMeasurement) {
        (
            channel(
                ChannelKind::OutputTorque,
                NumericInterval { lower: 9.0, upper: 11.0 },
            ),
            channel(
                ChannelKind::AngularVelocity,
                NumericInterval { lower: 4.0, upper: 6.0 },
            ),
        )
    }

    #[test]
    fn metrology_001_rectangular_power_interval_uses_all_four_corners() {
        let (torque, speed) = pair();
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::QualifiedBoundedEstimate);
        assert_eq!(report.pre_timing_power_w, Some(NumericInterval { lower: 36.0, upper: 66.0 }));
        assert_eq!(report.aligned_power_w, Some(NumericInterval { lower: 36.0, upper: 66.0 }));
        assert_eq!(report.power_sign, Some(PowerSign::PositiveMechanicalPower));
        assert_eq!(report.added_timing_power_bound_w, Some(0.0));
    }

    #[test]
    fn metrology_002_negative_mechanical_power_is_not_called_electrical_regeneration() {
        let (torque, mut speed) = pair();
        let mut torque = torque;
        torque.value = NumericInterval { lower: -11.0, upper: -9.0 };
        speed.value = NumericInterval { lower: 4.0, upper: 6.0 };
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.aligned_power_w, Some(NumericInterval { lower: -66.0, upper: -36.0 }));
        assert_eq!(report.power_sign, Some(PowerSign::NegativeMechanicalPower));
        assert!(report.interpretation.contains("not proof of DC-bus return"));
    }

    #[test]
    fn metrology_003_sign_crossing_is_reported_as_ambiguous() {
        let (mut torque, speed) = pair();
        torque.value = NumericInterval { lower: -1.0, upper: 1.0 };
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.power_sign, Some(PowerSign::CrossesZeroOrAmbiguous));
        assert_eq!(report.aligned_power_w, Some(NumericInterval { lower: -6.0, upper: 6.0 }));
    }

    #[test]
    fn metrology_004_clock_and_signal_slew_expand_the_power_bounds() {
        let (mut torque, mut speed) = pair();
        torque.absolute_slew_bound_per_s = 2.0;
        torque.sample_time_offset_s = 0.05;
        speed.absolute_slew_bound_per_s = 3.0;
        speed.sample_time_offset_s = -0.1;
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(
            report.torque_alignment.as_ref().unwrap().aligned_interval,
            NumericInterval { lower: 8.9, upper: 11.1 }
        );
        assert_eq!(
            report.angular_velocity_alignment.as_ref().unwrap().aligned_interval,
            NumericInterval { lower: 3.7, upper: 6.3 }
        );
        assert_eq!(
            report.aligned_power_w,
            Some(NumericInterval { lower: 32.93, upper: 69.93 })
        );
        assert!((report.added_timing_power_bound_w.unwrap() - 3.93).abs() < 1.0e-12);
    }

    #[test]
    fn metrology_005_uncancelled_filter_delay_expands_time_skew() {
        let (mut torque, speed) = pair();
        torque.filter_group_delay_s = 0.01;
        torque.absolute_slew_bound_per_s = 10.0;
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.torque_alignment.as_ref().unwrap().maximum_time_skew_s, 0.01);
        assert_eq!(
            report.torque_alignment.as_ref().unwrap().aligned_interval,
            NumericInterval { lower: 8.9, upper: 11.1 }
        );
    }

    #[test]
    fn metrology_006_compensated_filter_delay_is_not_double_counted() {
        let (mut torque, speed) = pair();
        torque.filter_group_delay_s = 0.02;
        torque.filter_delay_compensation_s = 0.02;
        torque.absolute_slew_bound_per_s = 10.0;
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.torque_alignment.as_ref().unwrap().maximum_time_skew_s, 0.0);
        assert_eq!(report.torque_alignment.as_ref().unwrap().aligned_interval, torque.value);
    }

    #[test]
    fn metrology_007_expired_calibration_is_unknown_not_zero() {
        let (mut torque, speed) = pair();
        torque.calibration_status = CalibrationStatus::Expired;
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
        assert_eq!(report.aligned_power_w, None);
        assert!(report.reason.as_ref().unwrap().contains("Expired"));
    }

    #[test]
    fn metrology_008_calibration_serial_mismatch_is_unknown() {
        let (mut torque, speed) = pair();
        torque.calibration_sensor_serial = "different-sensor".to_owned();
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
    }

    #[test]
    fn metrology_009_incomplete_traceability_chain_is_unknown() {
        let (mut torque, speed) = pair();
        torque.calibration_chain_digest.clear();
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
        assert!(report.reason.as_ref().unwrap().contains("traceability-chain"));
    }

    #[test]
    fn metrology_010_different_reference_clocks_are_unknown() {
        let (torque, mut speed) = pair();
        speed.reference_clock_id = "free-running-encoder-clock".to_owned();
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
        assert!(report.reason.as_ref().unwrap().contains("reference clock"));
    }

    #[test]
    fn metrology_011_different_reference_instants_are_unknown() {
        let (torque, mut speed) = pair();
        speed.reference_instant_s += 0.01;
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
        assert!(report.reason.as_ref().unwrap().contains("same physical instant"));
    }

    #[test]
    fn metrology_012_standard_uncertainty_is_not_silently_a_hard_bound() {
        let (mut torque, speed) = pair();
        torque.bound_representation = BoundRepresentation::StandardUncertainty;
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
        assert!(report.reason.as_ref().unwrap().contains("not an externally validated deterministic bound"));
    }

    #[test]
    fn metrology_013_boundary_and_condition_mismatches_are_unknown() {
        let (torque, mut speed) = pair();
        speed.system_boundary_digest = "motor-only-boundary".to_owned();
        let boundary = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(boundary.disposition, MeasurementDisposition::Unknown);

        speed.system_boundary_digest = torque.system_boundary_digest.clone();
        speed.operating_condition_digest = "different-cooling".to_owned();
        let condition = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(condition.disposition, MeasurementDisposition::Unknown);
    }

    #[test]
    fn metrology_014_missing_slew_bound_provenance_is_unknown() {
        let (mut torque, speed) = pair();
        torque.slew_bound_provenance_digest.clear();
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
    }

    #[test]
    fn metrology_015_time_expansion_outside_calibration_range_is_unknown_by_invalid_domain() {
        let (mut torque, speed) = pair();
        torque.calibrated_range = NumericInterval { lower: 9.0, upper: 11.0 };
        torque.absolute_slew_bound_per_s = 10.0;
        torque.sample_time_offset_s = 0.1;
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
    }

    #[test]
    fn metrology_016_wrong_channel_kind_rejects() {
        let (torque, speed) = pair();
        assert!(evaluate_mechanical_port_power(&speed, &torque).is_err());
    }

    #[test]
    fn metrology_017_inverted_or_nonfinite_intervals_reject() {
        let (mut torque, speed) = pair();
        torque.value = NumericInterval { lower: 12.0, upper: 10.0 };
        assert!(evaluate_mechanical_port_power(&torque, &speed).is_err());

        let (mut torque, speed) = pair();
        torque.value.lower = f64::NAN;
        assert!(evaluate_mechanical_port_power(&torque, &speed).is_err());
    }

    #[test]
    fn metrology_018_measurement_outside_calibration_covered_range_is_unknown() {
        let (mut torque, speed) = pair();
        torque.calibrated_range = NumericInterval { lower: 10.0, upper: 10.5 };
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
        assert!(report.reason.as_ref().unwrap().contains("outside the calibration-covered operating range"));
    }

    #[test]
    fn metrology_019_vendor_evidence_class_is_preserved() {
        let (mut torque, speed) = pair();
        torque.evidence_class = EvidenceClass::VendorPhysical;
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::QualifiedBoundedEstimate);
        assert_eq!(report.evidence_classes, vec![EvidenceClass::VendorPhysical, EvidenceClass::IndependentPhysical]);
    }

    #[test]
    fn metrology_020_traceability_not_established_is_unknown() {
        let (mut torque, speed) = pair();
        torque.traceability_status = TraceabilityStatus::Partial;
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
        assert!(report.reason.as_ref().unwrap().contains("traceability status"));
    }

    #[test]
    fn metrology_021_missing_clock_mapping_digest_is_unknown() {
        let (mut torque, speed) = pair();
        torque.clock_mapping_digest.clear();
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        assert_eq!(report.disposition, MeasurementDisposition::Unknown);
        assert!(report.reason.as_ref().unwrap().contains("clock-mapping digest"));
    }

    #[test]
    fn metrology_022_result_receipts_preserve_raw_trace_and_calibration_chain() {
        let (torque, speed) = pair();
        let report = evaluate_mechanical_port_power(&torque, &speed).unwrap();
        let torque_result = report.torque_alignment.unwrap();
        assert_eq!(torque_result.raw_trace_digest, "raw-trace-digest-01");
        assert_eq!(torque_result.reference_clock_id, "ptp-clock-domain-01");
        assert_eq!(
            torque_result.calibration_certificate_reference,
            "https://example.invalid/calibration/cert-01"
        );
        assert_eq!(
            torque_result.slew_bound_provenance_digest,
            "slew-bound-digest-01"
        );
        assert_eq!(torque_result.source_reference, "https://example.invalid/measurement/run-01");
        assert_eq!(torque_result.measurement_method, "Synthetic known-answer bounded calibration fixture");
        assert_eq!(report.angular_velocity_alignment.unwrap().raw_trace_digest, "raw-trace-digest-01");
        let _ = speed;
    }

    #[test]
    fn metrology_023_nonfinite_combined_skew_rejects() {
        let (mut torque, speed) = pair();
        torque.sample_time_offset_s = f64::MAX;
        torque.clock_offset_uncertainty_s = f64::MAX;
        assert!(evaluate_mechanical_port_power(&torque, &speed).is_err());
    }
}
