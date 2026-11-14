//! Signed energy accounting and cost-of-transport reference oracle.
//!
//! Each meter boundary is integrated independently. Never add battery, DC bus,
//! actuator and auxiliary streams unless a separately qualified energy-flow
//! topology proves they are disjoint rather than nested.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBoundary {
    BatteryTerminal,
    DcBus,
    ActuatorElectrical,
    AuxiliaryElectrical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleQuality {
    Measured,
    Missing,
    Stale,
    Saturated,
    Clipped,
    Interpolated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunReceipt {
    pub run_id: String,
    pub mass_configuration_digest: String,
    pub route_digest: String,
    pub distance_receipt_id: String,
    pub total_mass_kg: f64,
    pub gravity_m_s2: f64,
    pub path_distance_m: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyChannelReceipt {
    pub channel_id: String,
    pub run_id: String,
    pub boundary: EnergyBoundary,
    pub calibration_id: String,
    pub clock_id: String,
    pub timebase_receipt_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerSample {
    pub sample_id: String,
    pub channel_id: String,
    pub timestamp_ns: u64,
    pub voltage_volts: f64,
    /// Positive means power flows from the declared source to its load.
    /// At the battery terminal, positive means battery discharge.
    pub current_amps: f64,
    pub quality: SampleQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyLedger {
    pub run_id: String,
    pub channel_id: String,
    pub boundary: EnergyBoundary,
    pub calibration_id: String,
    pub clock_id: String,
    pub timebase_receipt_id: String,
    pub sample_count: usize,
    pub duration_seconds: f64,
    pub discharge_joules: f64,
    pub returned_joules: f64,
    pub net_joules: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostOfTransport {
    pub gross_draw: f64,
    pub net_draw: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointWorkReceipt {
    pub joint_id: String,
    pub run_id: String,
    pub torque_calibration_id: String,
    pub velocity_calibration_id: String,
    pub clock_id: String,
    pub timebase_receipt_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointWorkSample {
    pub sample_id: String,
    pub joint_id: String,
    pub timestamp_ns: u64,
    pub torque_newton_metres: f64,
    pub angular_velocity_rad_s: f64,
    pub quality: SampleQuality,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointWorkLedger {
    pub run_id: String,
    pub joint_id: String,
    pub torque_calibration_id: String,
    pub velocity_calibration_id: String,
    pub clock_id: String,
    pub timebase_receipt_id: String,
    pub sample_count: usize,
    pub duration_seconds: f64,
    pub positive_work_joules: f64,
    pub negative_work_joules: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyError {
    EmptyIdentifier,
    NonFiniteValue,
    InvalidRunReceipt,
    InvalidChannelReceipt,
    InvalidJointReceipt,
    InvalidSampleQuality,
    InvalidVoltage,
    DuplicateSampleId,
    DuplicateTimestamp,
    WrongChannel,
    WrongJoint,
    MismatchedRun,
    TooFewSamples,
    NonMonotonicTime,
    SampleGapExceeded,
    NumericalFailure,
    WrongEnergyBoundary,
    NonPositiveNetEnergy,
}

impl RunReceipt {
    pub fn validate(&self) -> Result<(), EnergyError> {
        if self.run_id.trim().is_empty()
            || self.mass_configuration_digest.trim().is_empty()
            || self.route_digest.trim().is_empty()
            || self.distance_receipt_id.trim().is_empty()
        {
            return Err(EnergyError::EmptyIdentifier);
        }
        if !self.total_mass_kg.is_finite()
            || self.total_mass_kg <= 0.0
            || !self.gravity_m_s2.is_finite()
            || self.gravity_m_s2 <= 0.0
            || !self.path_distance_m.is_finite()
            || self.path_distance_m <= 0.0
        {
            return Err(EnergyError::InvalidRunReceipt);
        }
        Ok(())
    }
}

impl EnergyChannelReceipt {
    pub fn validate(&self) -> Result<(), EnergyError> {
        if self.channel_id.trim().is_empty()
            || self.run_id.trim().is_empty()
            || self.calibration_id.trim().is_empty()
            || self.clock_id.trim().is_empty()
            || self.timebase_receipt_id.trim().is_empty()
        {
            return Err(EnergyError::EmptyIdentifier);
        }
        Ok(())
    }
}

impl JointWorkReceipt {
    pub fn validate(&self) -> Result<(), EnergyError> {
        if self.joint_id.trim().is_empty()
            || self.run_id.trim().is_empty()
            || self.torque_calibration_id.trim().is_empty()
            || self.velocity_calibration_id.trim().is_empty()
            || self.clock_id.trim().is_empty()
            || self.timebase_receipt_id.trim().is_empty()
        {
            return Err(EnergyError::EmptyIdentifier);
        }
        Ok(())
    }
}

pub fn integrate_power_channel(
    run: &RunReceipt,
    channel: &EnergyChannelReceipt,
    samples: &[PowerSample],
    maximum_sample_gap_ns: u64,
) -> Result<EnergyLedger, EnergyError> {
    run.validate()?;
    channel.validate()?;
    if channel.run_id != run.run_id {
        return Err(EnergyError::MismatchedRun);
    }
    if maximum_sample_gap_ns == 0 {
        return Err(EnergyError::SampleGapExceeded);
    }
    let ordered = validate_and_sort_power_samples(channel, samples)?;
    let (draw, returned, duration) = integrate_signed_samples(
        ordered.iter().map(|sample| {
            (
                sample.timestamp_ns,
                sample.voltage_volts * sample.current_amps,
            )
        }),
        maximum_sample_gap_ns,
    )?;
    let net = draw - returned;
    if !draw.is_finite() || !returned.is_finite() || !net.is_finite() {
        return Err(EnergyError::NumericalFailure);
    }
    Ok(EnergyLedger {
        run_id: run.run_id.clone(),
        channel_id: channel.channel_id.clone(),
        boundary: channel.boundary,
        calibration_id: channel.calibration_id.clone(),
        clock_id: channel.clock_id.clone(),
        timebase_receipt_id: channel.timebase_receipt_id.clone(),
        sample_count: ordered.len(),
        duration_seconds: duration,
        discharge_joules: draw,
        returned_joules: returned,
        net_joules: net,
    })
}

pub fn battery_cost_of_transport(
    run: &RunReceipt,
    ledger: &EnergyLedger,
) -> Result<CostOfTransport, EnergyError> {
    run.validate()?;
    if ledger.run_id != run.run_id {
        return Err(EnergyError::MismatchedRun);
    }
    if ledger.boundary != EnergyBoundary::BatteryTerminal {
        return Err(EnergyError::WrongEnergyBoundary);
    }
    if !ledger.discharge_joules.is_finite()
        || ledger.discharge_joules < 0.0
        || !ledger.returned_joules.is_finite()
        || ledger.returned_joules < 0.0
    {
        return Err(EnergyError::NonFiniteValue);
    }
    let net = ledger.discharge_joules - ledger.returned_joules;
    if !net.is_finite() || net <= 0.0 {
        return Err(EnergyError::NonPositiveNetEnergy);
    }
    let denominator = run.total_mass_kg * run.gravity_m_s2 * run.path_distance_m;
    let gross_draw = ledger.discharge_joules / denominator;
    let net_draw = net / denominator;
    if !gross_draw.is_finite() || !net_draw.is_finite() || denominator <= 0.0 {
        return Err(EnergyError::NumericalFailure);
    }
    Ok(CostOfTransport {
        gross_draw,
        net_draw,
    })
}

pub fn integrate_joint_work(
    run: &RunReceipt,
    receipt: &JointWorkReceipt,
    samples: &[JointWorkSample],
    maximum_sample_gap_ns: u64,
) -> Result<JointWorkLedger, EnergyError> {
    run.validate()?;
    receipt.validate()?;
    if receipt.run_id != run.run_id {
        return Err(EnergyError::MismatchedRun);
    }
    if maximum_sample_gap_ns == 0 {
        return Err(EnergyError::SampleGapExceeded);
    }
    let ordered = validate_and_sort_joint_samples(receipt, samples)?;
    let (positive, negative, duration) = integrate_signed_samples(
        ordered.iter().map(|sample| {
            (
                sample.timestamp_ns,
                sample.torque_newton_metres * sample.angular_velocity_rad_s,
            )
        }),
        maximum_sample_gap_ns,
    )?;
    Ok(JointWorkLedger {
        run_id: run.run_id.clone(),
        joint_id: receipt.joint_id.clone(),
        torque_calibration_id: receipt.torque_calibration_id.clone(),
        velocity_calibration_id: receipt.velocity_calibration_id.clone(),
        clock_id: receipt.clock_id.clone(),
        timebase_receipt_id: receipt.timebase_receipt_id.clone(),
        sample_count: ordered.len(),
        duration_seconds: duration,
        positive_work_joules: positive,
        negative_work_joules: negative,
    })
}

fn validate_and_sort_power_samples<'a>(
    receipt: &EnergyChannelReceipt,
    samples: &'a [PowerSample],
) -> Result<Vec<&'a PowerSample>, EnergyError> {
    if samples.len() < 2 {
        return Err(EnergyError::TooFewSamples);
    }
    let mut ids = BTreeSet::new();
    let mut ordered: Vec<&PowerSample> = samples.iter().collect();
    for sample in &ordered {
        if sample.sample_id.trim().is_empty() {
            return Err(EnergyError::EmptyIdentifier);
        }
        if !ids.insert(sample.sample_id.as_str()) {
            return Err(EnergyError::DuplicateSampleId);
        }
        if sample.channel_id != receipt.channel_id {
            return Err(EnergyError::WrongChannel);
        }
        if sample.quality != SampleQuality::Measured {
            return Err(EnergyError::InvalidSampleQuality);
        }
        if !sample.voltage_volts.is_finite() || !sample.current_amps.is_finite() {
            return Err(EnergyError::NonFiniteValue);
        }
        if sample.voltage_volts <= 0.0 {
            return Err(EnergyError::InvalidVoltage);
        }
        if !(sample.voltage_volts * sample.current_amps).is_finite() {
            return Err(EnergyError::NumericalFailure);
        }
    }
    ordered.sort_by_key(|sample| sample.timestamp_ns);
    for pair in ordered.windows(2) {
        if pair[0].timestamp_ns == pair[1].timestamp_ns {
            return Err(EnergyError::DuplicateTimestamp);
        }
    }
    Ok(ordered)
}

fn validate_and_sort_joint_samples<'a>(
    receipt: &JointWorkReceipt,
    samples: &'a [JointWorkSample],
) -> Result<Vec<&'a JointWorkSample>, EnergyError> {
    if samples.len() < 2 {
        return Err(EnergyError::TooFewSamples);
    }
    let mut ids = BTreeSet::new();
    let mut ordered: Vec<&JointWorkSample> = samples.iter().collect();
    for sample in &ordered {
        if sample.sample_id.trim().is_empty() {
            return Err(EnergyError::EmptyIdentifier);
        }
        if !ids.insert(sample.sample_id.as_str()) {
            return Err(EnergyError::DuplicateSampleId);
        }
        if sample.joint_id != receipt.joint_id {
            return Err(EnergyError::WrongJoint);
        }
        if sample.quality != SampleQuality::Measured {
            return Err(EnergyError::InvalidSampleQuality);
        }
        if !sample.torque_newton_metres.is_finite() || !sample.angular_velocity_rad_s.is_finite() {
            return Err(EnergyError::NonFiniteValue);
        }
        if !(sample.torque_newton_metres * sample.angular_velocity_rad_s).is_finite() {
            return Err(EnergyError::NumericalFailure);
        }
    }
    ordered.sort_by_key(|sample| sample.timestamp_ns);
    for pair in ordered.windows(2) {
        if pair[0].timestamp_ns == pair[1].timestamp_ns {
            return Err(EnergyError::DuplicateTimestamp);
        }
    }
    Ok(ordered)
}

/// Integrate signed samples with piecewise-linear power/work between sample
/// times. Positive and negative triangular areas are split exactly at zero.
fn integrate_signed_samples(
    samples: impl Iterator<Item = (u64, f64)>,
    maximum_sample_gap_ns: u64,
) -> Result<(f64, f64, f64), EnergyError> {
    let values: Vec<_> = samples.collect();
    if values.len() < 2 {
        return Err(EnergyError::TooFewSamples);
    }
    let mut positive = 0.0;
    let mut negative = 0.0;
    let mut duration_seconds = 0.0;

    for pair in values.windows(2) {
        let (t0, p0) = pair[0];
        let (t1, p1) = pair[1];
        if t1 <= t0 {
            return Err(EnergyError::NonMonotonicTime);
        }
        let dt_ns = t1 - t0;
        if dt_ns > maximum_sample_gap_ns {
            return Err(EnergyError::SampleGapExceeded);
        }
        let dt = dt_ns as f64 * 1.0e-9;
        let (positive_area, negative_area) = signed_linear_segment(p0, p1, dt)?;
        positive += positive_area;
        negative += negative_area;
        duration_seconds += dt;
    }

    if !positive.is_finite()
        || !negative.is_finite()
        || !duration_seconds.is_finite()
        || duration_seconds <= 0.0
    {
        return Err(EnergyError::NumericalFailure);
    }
    Ok((positive, negative, duration_seconds))
}

fn signed_linear_segment(
    start: f64,
    end: f64,
    duration_seconds: f64,
) -> Result<(f64, f64), EnergyError> {
    if !start.is_finite()
        || !end.is_finite()
        || !duration_seconds.is_finite()
        || duration_seconds <= 0.0
    {
        return Err(EnergyError::NumericalFailure);
    }
    if start >= 0.0 && end >= 0.0 {
        return Ok((0.5 * (start + end) * duration_seconds, 0.0));
    }
    if start <= 0.0 && end <= 0.0 {
        return Ok((0.0, -0.5 * (start + end) * duration_seconds));
    }

    let fraction_to_zero = start.abs() / (start.abs() + end.abs());
    if !fraction_to_zero.is_finite() || !(0.0..=1.0).contains(&fraction_to_zero) {
        return Err(EnergyError::NumericalFailure);
    }
    if start > 0.0 {
        let positive_duration = duration_seconds * fraction_to_zero;
        let negative_duration = duration_seconds - positive_duration;
        Ok((
            0.5 * start * positive_duration,
            0.5 * (-end) * negative_duration,
        ))
    } else {
        let negative_duration = duration_seconds * fraction_to_zero;
        let positive_duration = duration_seconds - negative_duration;
        Ok((
            0.5 * end * positive_duration,
            0.5 * (-start) * negative_duration,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_receipt() -> RunReceipt {
        RunReceipt {
            run_id: "run-1".to_owned(),
            mass_configuration_digest: "sha256:mass-config".to_owned(),
            route_digest: "sha256:route".to_owned(),
            distance_receipt_id: "distance-receipt-1".to_owned(),
            total_mass_kg: 10.0,
            gravity_m_s2: 10.0,
            path_distance_m: 10.0,
        }
    }

    fn channel(boundary: EnergyBoundary) -> EnergyChannelReceipt {
        EnergyChannelReceipt {
            channel_id: "battery-meter-1".to_owned(),
            run_id: "run-1".to_owned(),
            boundary,
            calibration_id: "calibration-1".to_owned(),
            clock_id: "clock-1".to_owned(),
            timebase_receipt_id: "clock-map-1".to_owned(),
        }
    }

    fn power_sample(id: &str, timestamp_ns: u64, voltage: f64, current: f64) -> PowerSample {
        PowerSample {
            sample_id: id.to_owned(),
            channel_id: "battery-meter-1".to_owned(),
            timestamp_ns,
            voltage_volts: voltage,
            current_amps: current,
            quality: SampleQuality::Measured,
        }
    }

    #[test]
    fn constant_positive_power_integrates_to_exact_energy() {
        let ledger = integrate_power_channel(
            &run_receipt(),
            &channel(EnergyBoundary::BatteryTerminal),
            &[
                power_sample("s0", 0, 10.0, 2.0),
                power_sample("s1", 1_000_000_000, 10.0, 2.0),
                power_sample("s2", 2_000_000_000, 10.0, 2.0),
            ],
            1_000_000_000,
        )
        .unwrap();
        assert!((ledger.discharge_joules - 40.0).abs() < 1.0e-12);
        assert_eq!(ledger.returned_joules, 0.0);
        assert!((ledger.net_joules - 40.0).abs() < 1.0e-12);
    }

    #[test]
    fn positive_and_regenerative_power_are_split_at_zero_crossing() {
        let ledger = integrate_power_channel(
            &run_receipt(),
            &channel(EnergyBoundary::BatteryTerminal),
            &[
                power_sample("s0", 0, 10.0, 1.0),
                power_sample("s1", 2_000_000_000, 10.0, -1.0),
            ],
            2_000_000_000,
        )
        .unwrap();
        assert!((ledger.discharge_joules - 5.0).abs() < 1.0e-12);
        assert!((ledger.returned_joules - 5.0).abs() < 1.0e-12);
        assert!(ledger.net_joules.abs() < 1.0e-12);
    }

    #[test]
    fn non_uniform_timestamps_are_integrated_in_physical_time() {
        let ledger = integrate_power_channel(
            &run_receipt(),
            &channel(EnergyBoundary::BatteryTerminal),
            &[
                power_sample("s0", 0, 12.0, 1.0),
                power_sample("s1", 500_000_000, 12.0, 1.0),
                power_sample("s2", 2_000_000_000, 12.0, 1.0),
            ],
            1_500_000_000,
        )
        .unwrap();
        assert!((ledger.discharge_joules - 24.0).abs() < 1.0e-12);
        assert!((ledger.duration_seconds - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn input_order_does_not_change_energy_ledger() {
        let mut samples = vec![
            power_sample("s0", 0, 10.0, 1.0),
            power_sample("s1", 1_000_000_000, 10.0, -1.0),
            power_sample("s2", 2_000_000_000, 10.0, 1.0),
        ];
        let first = integrate_power_channel(
            &run_receipt(),
            &channel(EnergyBoundary::BatteryTerminal),
            &samples,
            1_000_000_000,
        )
        .unwrap();
        samples.reverse();
        let second = integrate_power_channel(
            &run_receipt(),
            &channel(EnergyBoundary::BatteryTerminal),
            &samples,
            1_000_000_000,
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn battery_cot_uses_gross_and_net_energy_without_double_counting() {
        let ledger = integrate_power_channel(
            &run_receipt(),
            &channel(EnergyBoundary::BatteryTerminal),
            &[
                power_sample("s0", 0, 10.0, 2.0),
                power_sample("s1", 2_000_000_000, 10.0, 1.0),
            ],
            2_000_000_000,
        )
        .unwrap();
        let cot = battery_cost_of_transport(&run_receipt(), &ledger).unwrap();
        assert!((cot.gross_draw - 0.03).abs() < 1.0e-12);
        assert!((cot.net_draw - 0.03).abs() < 1.0e-12);
    }

    #[test]
    fn energy_from_another_run_cannot_be_reused_for_cot() {
        let ledger = integrate_power_channel(
            &run_receipt(),
            &channel(EnergyBoundary::BatteryTerminal),
            &[
                power_sample("s0", 0, 10.0, 1.0),
                power_sample("s1", 1_000_000_000, 10.0, 1.0),
            ],
            1_000_000_000,
        )
        .unwrap();
        let mut other_run = run_receipt();
        other_run.run_id = "other-run".to_owned();
        assert_eq!(
            battery_cost_of_transport(&other_run, &ledger),
            Err(EnergyError::MismatchedRun)
        );
    }

    #[test]
    fn non_positive_net_energy_has_no_fabricated_cot() {
        let ledger = integrate_power_channel(
            &run_receipt(),
            &channel(EnergyBoundary::BatteryTerminal),
            &[
                power_sample("s0", 0, 10.0, 1.0),
                power_sample("s1", 2_000_000_000, 10.0, -1.0),
            ],
            2_000_000_000,
        )
        .unwrap();
        assert_eq!(
            battery_cost_of_transport(&run_receipt(), &ledger),
            Err(EnergyError::NonPositiveNetEnergy)
        );
    }

    #[test]
    fn internal_bus_energy_cannot_be_misreported_as_battery_cot() {
        let ledger = integrate_power_channel(
            &run_receipt(),
            &channel(EnergyBoundary::DcBus),
            &[
                power_sample("s0", 0, 10.0, 1.0),
                power_sample("s1", 1_000_000_000, 10.0, 1.0),
            ],
            1_000_000_000,
        )
        .unwrap();
        assert_eq!(
            battery_cost_of_transport(&run_receipt(), &ledger),
            Err(EnergyError::WrongEnergyBoundary)
        );
    }

    #[test]
    fn zero_distance_run_is_rejected_before_cot() {
        let mut run = run_receipt();
        run.path_distance_m = 0.0;
        assert_eq!(run.validate(), Err(EnergyError::InvalidRunReceipt));
    }

    #[test]
    fn wrong_run_identity_is_rejected() {
        let mut channel = channel(EnergyBoundary::BatteryTerminal);
        channel.run_id = "other-run".to_owned();
        assert_eq!(
            integrate_power_channel(
                &run_receipt(),
                &channel,
                &[
                    power_sample("s0", 0, 10.0, 1.0),
                    power_sample("s1", 1_000_000_000, 10.0, 1.0),
                ],
                1_000_000_000
            ),
            Err(EnergyError::MismatchedRun)
        );
    }

    #[test]
    fn duplicate_sample_id_and_timestamp_are_rejected() {
        let duplicate_id = vec![
            power_sample("same", 0, 10.0, 1.0),
            power_sample("same", 1_000_000_000, 10.0, 1.0),
        ];
        assert_eq!(
            integrate_power_channel(
                &run_receipt(),
                &channel(EnergyBoundary::BatteryTerminal),
                &duplicate_id,
                1_000_000_000
            ),
            Err(EnergyError::DuplicateSampleId)
        );

        let duplicate_time = vec![
            power_sample("s0", 0, 10.0, 1.0),
            power_sample("s1", 0, 10.0, 1.0),
        ];
        assert_eq!(
            integrate_power_channel(
                &run_receipt(),
                &channel(EnergyBoundary::BatteryTerminal),
                &duplicate_time,
                1_000_000_000
            ),
            Err(EnergyError::DuplicateTimestamp)
        );
    }

    #[test]
    fn missing_or_saturated_samples_are_not_zero_imputed() {
        let mut missing = power_sample("s1", 1_000_000_000, 10.0, 1.0);
        missing.quality = SampleQuality::Missing;
        assert_eq!(
            integrate_power_channel(
                &run_receipt(),
                &channel(EnergyBoundary::BatteryTerminal),
                &[power_sample("s0", 0, 10.0, 1.0), missing],
                1_000_000_000
            ),
            Err(EnergyError::InvalidSampleQuality)
        );

        let mut saturated = power_sample("s1", 1_000_000_000, 10.0, 1.0);
        saturated.quality = SampleQuality::Saturated;
        assert_eq!(
            integrate_power_channel(
                &run_receipt(),
                &channel(EnergyBoundary::BatteryTerminal),
                &[power_sample("s0", 0, 10.0, 1.0), saturated],
                1_000_000_000
            ),
            Err(EnergyError::InvalidSampleQuality)
        );
    }

    #[test]
    fn non_finite_and_non_positive_voltage_are_rejected() {
        let bad_voltage = vec![
            power_sample("s0", 0, 0.0, 1.0),
            power_sample("s1", 1_000_000_000, 10.0, 1.0),
        ];
        assert_eq!(
            integrate_power_channel(
                &run_receipt(),
                &channel(EnergyBoundary::BatteryTerminal),
                &bad_voltage,
                1_000_000_000
            ),
            Err(EnergyError::InvalidVoltage)
        );

        let bad_current = vec![
            power_sample("s0", 0, 10.0, f64::NAN),
            power_sample("s1", 1_000_000_000, 10.0, 1.0),
        ];
        assert_eq!(
            integrate_power_channel(
                &run_receipt(),
                &channel(EnergyBoundary::BatteryTerminal),
                &bad_current,
                1_000_000_000
            ),
            Err(EnergyError::NonFiniteValue)
        );
    }

    #[test]
    fn excessive_sampling_gap_is_rejected() {
        assert_eq!(
            integrate_power_channel(
                &run_receipt(),
                &channel(EnergyBoundary::BatteryTerminal),
                &[
                    power_sample("s0", 0, 10.0, 1.0),
                    power_sample("s1", 10_000_000_000, 10.0, 1.0),
                ],
                1_000_000_000
            ),
            Err(EnergyError::SampleGapExceeded)
        );
    }

    #[test]
    fn mechanical_negative_work_is_separate_from_electrical_regeneration() {
        let receipt = JointWorkReceipt {
            joint_id: "left-knee".to_owned(),
            run_id: "run-1".to_owned(),
            torque_calibration_id: "torque-cal".to_owned(),
            velocity_calibration_id: "velocity-cal".to_owned(),
            clock_id: "joint-clock".to_owned(),
            timebase_receipt_id: "joint-time-map".to_owned(),
        };
        let samples = vec![
            JointWorkSample {
                sample_id: "j0".to_owned(),
                joint_id: "left-knee".to_owned(),
                timestamp_ns: 0,
                torque_newton_metres: 2.0,
                angular_velocity_rad_s: 1.0,
                quality: SampleQuality::Measured,
            },
            JointWorkSample {
                sample_id: "j1".to_owned(),
                joint_id: "left-knee".to_owned(),
                timestamp_ns: 1_000_000_000,
                torque_newton_metres: -2.0,
                angular_velocity_rad_s: 1.0,
                quality: SampleQuality::Measured,
            },
        ];
        let work = integrate_joint_work(&run_receipt(), &receipt, &samples, 1_000_000_000).unwrap();
        assert!((work.positive_work_joules - 0.5).abs() < 1.0e-12);
        assert!((work.negative_work_joules - 0.5).abs() < 1.0e-12);
    }
}
