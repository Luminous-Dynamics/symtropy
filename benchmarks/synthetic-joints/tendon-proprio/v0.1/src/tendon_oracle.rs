//! Independent known-answer tests for a linear resistive tendon-length sensor.
//!
//! All values in this module are synthetic. The paper's measured calibration
//! RMSE and performance data are recorded separately in the JSON/README.
//! This module estimates tendon arc length only; it does not estimate tension
//! or joint torque.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SensorError {
    InvalidParameter,
    OutOfCalibrationDomain,
}

#[derive(Debug, Clone, Copy)]
struct LinearTendonCalibration {
    reference_length_mm: f64,
    reference_resistance_ohm: f64,
    slope_ohm_per_mm: f64,
    calibrated_min_length_mm: f64,
    calibrated_max_length_mm: f64,
}

impl LinearTendonCalibration {
    fn validate(self) -> Result<(), SensorError> {
        let values = [
            self.reference_length_mm,
            self.reference_resistance_ohm,
            self.slope_ohm_per_mm,
            self.calibrated_min_length_mm,
            self.calibrated_max_length_mm,
        ];
        if values.iter().any(|value| !value.is_finite())
            || self.reference_resistance_ohm <= 0.0
            || self.slope_ohm_per_mm <= 0.0
            || self.calibrated_min_length_mm < 0.0
            || self.calibrated_max_length_mm <= self.calibrated_min_length_mm
            || self.reference_length_mm < self.calibrated_min_length_mm
            || self.reference_length_mm > self.calibrated_max_length_mm
        {
            return Err(SensorError::InvalidParameter);
        }
        Ok(())
    }

    fn resistance_at_length(self, length_mm: f64) -> Result<f64, SensorError> {
        self.validate()?;
        if !length_mm.is_finite() {
            return Err(SensorError::InvalidParameter);
        }
        if length_mm < self.calibrated_min_length_mm
            || length_mm > self.calibrated_max_length_mm
        {
            return Err(SensorError::OutOfCalibrationDomain);
        }
        let resistance = self.reference_resistance_ohm
            + self.slope_ohm_per_mm * (length_mm - self.reference_length_mm);
        if resistance <= 0.0 || !resistance.is_finite() {
            return Err(SensorError::InvalidParameter);
        }
        Ok(resistance)
    }

    fn length_at_resistance(self, resistance_ohm: f64) -> Result<f64, SensorError> {
        self.validate()?;
        if !resistance_ohm.is_finite() || resistance_ohm <= 0.0 {
            return Err(SensorError::InvalidParameter);
        }
        let min_resistance = self.reference_resistance_ohm
            + self.slope_ohm_per_mm
                * (self.calibrated_min_length_mm - self.reference_length_mm);
        let max_resistance = self.reference_resistance_ohm
            + self.slope_ohm_per_mm
                * (self.calibrated_max_length_mm - self.reference_length_mm);
        if resistance_ohm < min_resistance || resistance_ohm > max_resistance {
            return Err(SensorError::OutOfCalibrationDomain);
        }
        let length = self.reference_length_mm
            + (resistance_ohm - self.reference_resistance_ohm) / self.slope_ohm_per_mm;
        if length < self.calibrated_min_length_mm || length > self.calibrated_max_length_mm {
            return Err(SensorError::OutOfCalibrationDomain);
        }
        Ok(length)
    }
}

fn average_excitation_power_w(
    voltage_v: f64,
    duty_factor: f64,
    resistance_ohm: f64,
) -> Option<f64> {
    if !voltage_v.is_finite()
        || !duty_factor.is_finite()
        || !resistance_ohm.is_finite()
        || voltage_v < 0.0
        || !(0.0..=1.0).contains(&duty_factor)
        || resistance_ohm <= 0.0
    {
        return None;
    }
    Some(duty_factor * voltage_v.powi(2) / resistance_ohm)
}

fn equivalent_temperature_length_drift_mm(
    reference_resistance_ohm: f64,
    coefficient_per_c: f64,
    temperature_change_c: f64,
    slope_ohm_per_mm: f64,
) -> Option<f64> {
    if !reference_resistance_ohm.is_finite()
        || !coefficient_per_c.is_finite()
        || !temperature_change_c.is_finite()
        || !slope_ohm_per_mm.is_finite()
        || reference_resistance_ohm <= 0.0
        || coefficient_per_c < 0.0
        || slope_ohm_per_mm <= 0.0
    {
        return None;
    }
    Some(
        reference_resistance_ohm * coefficient_per_c * temperature_change_c
            / slope_ohm_per_mm,
    )
}

fn inverse_length_standard_uncertainty_mm(
    resistance_delta_ohm: f64,
    resistance_sigma_ohm: f64,
    slope_ohm_per_mm: f64,
    slope_sigma_ohm_per_mm: f64,
) -> Option<f64> {
    let values = [
        resistance_delta_ohm,
        resistance_sigma_ohm,
        slope_ohm_per_mm,
        slope_sigma_ohm_per_mm,
    ];
    if values.iter().any(|value| !value.is_finite())
        || resistance_sigma_ohm < 0.0
        || slope_ohm_per_mm <= 0.0
        || slope_sigma_ohm_per_mm < 0.0
    {
        return None;
    }
    let variance = (resistance_sigma_ohm / slope_ohm_per_mm).powi(2)
        + (resistance_delta_ohm * slope_sigma_ohm_per_mm
            / slope_ohm_per_mm.powi(2))
        .powi(2);
    Some(variance.sqrt())
}

fn calibration() -> LinearTendonCalibration {
    LinearTendonCalibration {
        reference_length_mm: 40.0,
        reference_resistance_ohm: 4.0,
        slope_ohm_per_mm: 0.2,
        calibrated_min_length_mm: 40.0,
        calibrated_max_length_mm: 80.0,
    }
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = 1.0e-12_f64.max(expected.abs() * 1.0e-12);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected:.15e}, got {actual:.15e}, tolerance {tolerance:.3e}"
    );
}

#[test]
fn tendon_001_linear_resistance_from_known_length() {
    let resistance = calibration()
        .resistance_at_length(60.0)
        .expect("60 mm is inside the calibrated interval");
    assert_close(resistance, 8.0, "sensor resistance [ohm]");
}

#[test]
fn tendon_002_inverse_length_and_strict_domain() {
    let sensor = calibration();
    let length = sensor
        .length_at_resistance(8.0)
        .expect("8 ohm is inside the calibration-resistance interval");
    assert_close(length, 60.0, "estimated tendon length [mm]");
    assert_eq!(
        sensor.length_at_resistance(3.9),
        Err(SensorError::OutOfCalibrationDomain)
    );
    assert_eq!(
        sensor.length_at_resistance(12.1),
        Err(SensorError::OutOfCalibrationDomain)
    );
}

#[test]
fn tendon_003_non_overlapping_pulse_average_joule_power() {
    let power = average_excitation_power_w(5.0, 0.01, 8.0)
        .expect("valid pulse and resistance must be admitted");
    assert_close(power, 0.03125, "average excitation power [W]");
}

#[test]
fn tendon_004_temperature_equivalent_length_drift() {
    let drift = equivalent_temperature_length_drift_mm(4.0, 0.0004, 10.0, 0.2)
        .expect("valid synthetic thermal parameters must be admitted");
    assert_close(drift, 0.08, "temperature-equivalent length drift [mm]");
}

#[test]
fn tendon_005_inverse_calibration_first_order_uncertainty() {
    let sigma = inverse_length_standard_uncertainty_mm(4.0, 0.1, 0.2, 0.002)
        .expect("valid synthetic uncertainty terms must be admitted");
    assert_close(
        sigma,
        0.5385164807134505,
        "inverse length standard uncertainty [mm]",
    );
}

#[test]
fn tendon_006_invalid_and_non_finite_inputs_fail_closed() {
    let mut invalid = calibration();
    invalid.slope_ohm_per_mm = 0.0;
    assert_eq!(
        invalid.resistance_at_length(60.0),
        Err(SensorError::InvalidParameter)
    );

    assert_eq!(
        calibration().resistance_at_length(f64::NAN),
        Err(SensorError::InvalidParameter)
    );
    assert!(average_excitation_power_w(5.0, 1.1, 8.0).is_none());
    assert!(average_excitation_power_w(5.0, 0.1, 0.0).is_none());
    assert!(equivalent_temperature_length_drift_mm(4.0, 0.0004, 10.0, 0.0).is_none());
    assert!(inverse_length_standard_uncertainty_mm(4.0, -0.1, 0.2, 0.002).is_none());
}
