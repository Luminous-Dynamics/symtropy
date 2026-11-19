#![forbid(unsafe_code)]

//! Dependency-free analytic primitives for four-quadrant energy accounting.
//!
//! All sign conventions are explicit at call sites. These routines integrate
//! supplied port measurements; they do not infer electrical recovery from
//! negative mechanical work or from DC-bus voltage rise alone.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingQuadrant {
    IForwardMotoring,
    IIForwardRegenerating,
    IIIReverseMotoring,
    IVReverseRegenerating,
    ZeroPowerBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerSample {
    pub time_s: f64,
    /// Signed power in watts under the caller's declared port convention.
    pub power_w: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BusVoltageSample {
    pub time_s: f64,
    pub voltage_v: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoltageExcursion {
    pub maximum_voltage_v: f64,
    pub threshold_voltage_v: f64,
    pub samples_above_threshold: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyBalance {
    pub input_energy_j: f64,
    pub output_energy_j: f64,
    /// Signed change in stored energy: positive when the control volume stores energy.
    pub stored_energy_change_j: f64,
    pub dissipated_energy_j: f64,
    /// Positive residual means the declared inputs exceed accounted outputs/storage/losses.
    pub residual_energy_j: f64,
}

fn finite(value: f64) -> bool {
    value.is_finite()
}

pub fn classify_quadrant(motor_torque_nm: f64, motor_speed_rad_s: f64) -> Option<OperatingQuadrant> {
    if !finite(motor_torque_nm) || !finite(motor_speed_rad_s) {
        return None;
    }
    if motor_torque_nm == 0.0 || motor_speed_rad_s == 0.0 {
        return Some(OperatingQuadrant::ZeroPowerBoundary);
    }

    match (motor_torque_nm.is_sign_positive(), motor_speed_rad_s.is_sign_positive()) {
        (true, true) => Some(OperatingQuadrant::IForwardMotoring),
        (false, true) => Some(OperatingQuadrant::IIForwardRegenerating),
        (false, false) => Some(OperatingQuadrant::IIIReverseMotoring),
        (true, false) => Some(OperatingQuadrant::IVReverseRegenerating),
    }
}

/// Integrates signed power samples with the composite trapezoidal rule.
///
/// Positive result means energy follows the caller's positive power convention;
/// negative result means energy flows in the opposite direction. At least two
/// samples and strictly increasing finite timestamps are required.
pub fn integrate_power_trapezoid(samples: &[PowerSample]) -> Result<f64, String> {
    if samples.len() < 2 {
        return Err("at least two power samples are required".to_owned());
    }

    let mut energy_j = 0.0_f64;
    for (index, sample) in samples.iter().enumerate() {
        if !finite(sample.time_s) || !finite(sample.power_w) {
            return Err(format!("non-finite time or power at sample {index}"));
        }
        if index + 1 < samples.len() {
            let next = samples[index + 1];
            if !finite(next.time_s) || !finite(next.power_w) {
                return Err(format!("non-finite time or power at sample {}", index + 1));
            }
            let delta_t = next.time_s - sample.time_s;
            if !finite(delta_t) || delta_t <= 0.0 {
                return Err(format!("timestamps are not strictly increasing at sample {index}"));
            }
            energy_j += 0.5 * (sample.power_w + next.power_w) * delta_t;
            if !finite(energy_j) {
                return Err("integrated energy is non-finite".to_owned());
            }
        }
    }
    Ok(energy_j)
}

/// Computes the ideal energy stored in a capacitor: 0.5 * C * V^2.
pub fn capacitor_energy_j(capacitance_f: f64, voltage_v: f64) -> Option<f64> {
    if !finite(capacitance_f) || !finite(voltage_v) || capacitance_f <= 0.0 || voltage_v < 0.0 {
        return None;
    }
    let energy = 0.5 * capacitance_f * voltage_v.powi(2);
    finite(energy).then_some(energy)
}

/// Computes signed capacitor energy change from initial to final bus voltage.
pub fn capacitor_energy_change_j(
    capacitance_f: f64,
    initial_voltage_v: f64,
    final_voltage_v: f64,
) -> Option<f64> {
    let initial = capacitor_energy_j(capacitance_f, initial_voltage_v)?;
    let final_energy = capacitor_energy_j(capacitance_f, final_voltage_v)?;
    let delta = final_energy - initial;
    finite(delta).then_some(delta)
}

/// Audits bus-voltage samples against a declared threshold.
///
/// This reports voltage observations only; it does not estimate current,
/// power, regenerated energy, or time above threshold.
pub fn voltage_excursion(
    samples: &[BusVoltageSample],
    threshold_voltage_v: f64,
) -> Option<VoltageExcursion> {
    if samples.is_empty() || !finite(threshold_voltage_v) || threshold_voltage_v < 0.0 {
        return None;
    }

    let mut maximum_voltage_v = f64::NEG_INFINITY;
    let mut samples_above_threshold = 0usize;
    let mut previous_time: Option<f64> = None;

    for sample in samples {
        if !finite(sample.time_s) || !finite(sample.voltage_v) || sample.voltage_v < 0.0 {
            return None;
        }
        if let Some(previous) = previous_time {
            if sample.time_s <= previous {
                return None;
            }
        }
        previous_time = Some(sample.time_s);
        maximum_voltage_v = maximum_voltage_v.max(sample.voltage_v);
        if sample.voltage_v > threshold_voltage_v {
            samples_above_threshold += 1;
        }
    }

    Some(VoltageExcursion {
        maximum_voltage_v,
        threshold_voltage_v,
        samples_above_threshold,
    })
}

/// Computes a balance residual for positive input/output/dissipation magnitudes
/// and a signed stored-energy change.
///
/// residual = input - output - stored_change - dissipated
///
/// All energy quantities use joules and the control volume must be fixed by the
/// caller. This routine does not invent a balancing loss or repair missing ports.
pub fn energy_balance(
    input_energy_j: f64,
    output_energy_j: f64,
    stored_energy_change_j: f64,
    dissipated_energy_j: f64,
) -> Option<EnergyBalance> {
    if !finite(input_energy_j)
        || !finite(output_energy_j)
        || !finite(stored_energy_change_j)
        || !finite(dissipated_energy_j)
        || input_energy_j < 0.0
        || output_energy_j < 0.0
        || dissipated_energy_j < 0.0
    {
        return None;
    }

    let residual = input_energy_j
        - output_energy_j
        - stored_energy_change_j
        - dissipated_energy_j;
    if !finite(residual) {
        return None;
    }

    Some(EnergyBalance {
        input_energy_j,
        output_energy_j,
        stored_energy_change_j,
        dissipated_energy_j,
        residual_energy_j: residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected:.12e}, got {actual:.12e}, tolerance {tolerance:.3e}"
        );
    }

    #[test]
    fn regen_001_all_four_quadrants_have_standard_signs() {
        assert_eq!(
            classify_quadrant(2.0, 3.0),
            Some(OperatingQuadrant::IForwardMotoring)
        );
        assert_eq!(
            classify_quadrant(-2.0, 3.0),
            Some(OperatingQuadrant::IIForwardRegenerating)
        );
        assert_eq!(
            classify_quadrant(-2.0, -3.0),
            Some(OperatingQuadrant::IIIReverseMotoring)
        );
        assert_eq!(
            classify_quadrant(2.0, -3.0),
            Some(OperatingQuadrant::IVReverseRegenerating)
        );
        assert_eq!(
            classify_quadrant(0.0, 3.0),
            Some(OperatingQuadrant::ZeroPowerBoundary)
        );
    }

    #[test]
    fn regen_002_invalid_quadrant_inputs_fail_closed() {
        assert_eq!(classify_quadrant(f64::NAN, 1.0), None);
        assert_eq!(classify_quadrant(1.0, f64::INFINITY), None);
    }

    #[test]
    fn regen_003_constant_positive_power_integrates_to_energy() {
        let samples = [
            PowerSample { time_s: 0.0, power_w: 10.0 },
            PowerSample { time_s: 1.0, power_w: 10.0 },
            PowerSample { time_s: 2.0, power_w: 10.0 },
        ];
        close(integrate_power_trapezoid(&samples).unwrap(), 20.0, 1.0e-12);
    }

    #[test]
    fn regen_004_negative_power_preserves_direction() {
        let samples = [
            PowerSample { time_s: 0.0, power_w: -10.0 },
            PowerSample { time_s: 2.0, power_w: -10.0 },
        ];
        close(integrate_power_trapezoid(&samples).unwrap(), -20.0, 1.0e-12);
    }

    #[test]
    fn regen_005_ramp_power_matches_analytic_trapezoid() {
        let samples = [
            PowerSample { time_s: 0.0, power_w: 0.0 },
            PowerSample { time_s: 2.0, power_w: 10.0 },
        ];
        close(integrate_power_trapezoid(&samples).unwrap(), 10.0, 1.0e-12);
    }

    #[test]
    fn regen_006_nonmonotonic_and_nonfinite_samples_reject() {
        let duplicate_time = [
            PowerSample { time_s: 0.0, power_w: 1.0 },
            PowerSample { time_s: 0.0, power_w: 1.0 },
        ];
        assert!(integrate_power_trapezoid(&duplicate_time).is_err());

        let nonfinite_power = [
            PowerSample { time_s: 0.0, power_w: 1.0 },
            PowerSample { time_s: 1.0, power_w: f64::NAN },
        ];
        assert!(integrate_power_trapezoid(&nonfinite_power).is_err());

        assert!(integrate_power_trapezoid(&[]).is_err());
    }

    #[test]
    fn regen_007_capacitor_energy_and_change_have_known_answers() {
        close(capacitor_energy_j(0.01, 24.0).unwrap(), 2.88, 1.0e-12);
        close(
            capacitor_energy_change_j(0.01, 12.0, 24.0).unwrap(),
            2.16,
            1.0e-12,
        );
        close(
            capacitor_energy_change_j(0.01, 24.0, 12.0).unwrap(),
            -2.16,
            1.0e-12,
        );
    }

    #[test]
    fn regen_008_invalid_capacitance_and_voltage_reject() {
        assert!(capacitor_energy_j(0.0, 24.0).is_none());
        assert!(capacitor_energy_j(-0.01, 24.0).is_none());
        assert!(capacitor_energy_j(0.01, -1.0).is_none());
        assert!(capacitor_energy_j(f64::INFINITY, 24.0).is_none());
    }

    #[test]
    fn regen_009_voltage_excursion_reports_count_not_energy() {
        let samples = [
            BusVoltageSample { time_s: 0.0, voltage_v: 14.8 },
            BusVoltageSample { time_s: 0.1, voltage_v: 16.0 },
            BusVoltageSample { time_s: 0.2, voltage_v: 21.8 },
            BusVoltageSample { time_s: 0.3, voltage_v: 17.0 },
        ];
        let result = voltage_excursion(&samples, 16.0).unwrap();
        close(result.maximum_voltage_v, 21.8, 1.0e-12);
        assert_eq!(result.samples_above_threshold, 2);
    }

    #[test]
    fn regen_010_voltage_excursion_rejects_invalid_timestamps_and_values() {
        let duplicate = [
            BusVoltageSample { time_s: 0.0, voltage_v: 12.0 },
            BusVoltageSample { time_s: 0.0, voltage_v: 13.0 },
        ];
        assert!(voltage_excursion(&duplicate, 16.0).is_none());

        let invalid = [BusVoltageSample { time_s: 0.0, voltage_v: f64::NAN }];
        assert!(voltage_excursion(&invalid, 16.0).is_none());
    }

    #[test]
    fn regen_011_energy_balance_residual_has_declared_sign() {
        let balance = energy_balance(100.0, 20.0, 30.0, 40.0).unwrap();
        close(balance.residual_energy_j, 10.0, 1.0e-12);

        let discharged_store = energy_balance(0.0, 20.0, -20.0, 0.0).unwrap();
        close(discharged_store.residual_energy_j, 0.0, 1.0e-12);
    }

    #[test]
    fn regen_012_energy_balance_rejects_invalid_magnitudes() {
        assert!(energy_balance(-1.0, 0.0, 0.0, 0.0).is_none());
        assert!(energy_balance(1.0, f64::NAN, 0.0, 0.0).is_none());
        assert!(energy_balance(1.0, 0.0, 0.0, -1.0).is_none());
    }
}
