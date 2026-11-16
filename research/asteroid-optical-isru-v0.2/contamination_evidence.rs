// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Typed paired-observation comparison for optical witness and QCM evidence.
// Run tests with:
//   rustc --edition=2021 --test research/asteroid-optical-isru-v0.2/contamination_evidence.rs -o /tmp/isru-fouling-tests
//   /tmp/isru-fouling-tests

#![forbid(unsafe_code)]

//! Keep deposition mass and optical degradation as separately measured states.
//!
//! A QCM reading is local to its crystal. It does not identify the chemical
//! species, prove deposition on another optic, or quantify optical throughput.
//! Optical transmission degradation is assessed only from direct, comparable
//! optical-witness readings. Uncertainty is bounded conservatively by the sum
//! of absolute uncertainty bounds; no independence assumption is introduced.
//!
//! A passing calibration eligibility field is an upstream declaration from
//! the evidence/authority path, not a signature check performed here.

use std::error::Error;
use std::fmt;

const VALUE_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationEligibility {
    EligibleAtMeasurementTime,
    Unknown,
    Ineligible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeDisposition {
    DecreaseEstablished,
    IncreaseEstablished,
    NoChangeResolved,
    IndeterminateWithinUncertainty,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpticalWitnessReading {
    pub witness_id: String,
    pub location_id: String,
    pub wavelength_nm: u32,
    pub orientation_deg: f64,
    pub temperature_k: f64,
    pub measured_at_utc_s: i64,
    /// Directly measured spectral transmittance, normalized to [0, 1].
    pub transmittance: f64,
    /// Absolute uncertainty bound for transmittance, in the same units.
    pub absolute_uncertainty: f64,
    pub calibration_eligibility: CalibrationEligibility,
    pub calibration_receipt_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QcmReading {
    pub qcm_id: String,
    pub location_id: String,
    pub orientation_deg: f64,
    pub temperature_k: f64,
    pub measured_at_utc_s: i64,
    /// Deposited areal mass at this QCM crystal, expressed in g/cm^2.
    pub areal_mass_g_cm2: f64,
    /// Absolute uncertainty bound in g/cm^2.
    pub absolute_uncertainty_g_cm2: f64,
    pub calibration_eligibility: CalibrationEligibility,
    pub calibration_receipt_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChangeAssessment {
    pub raw_change: f64,
    pub conservative_change_lower_bound: f64,
    pub conservative_change_upper_bound: f64,
    pub disposition: ChangeDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvidenceError {
    EmptyIdentity(&'static str),
    CalibrationNotEligible,
    InvalidWavelength,
    InvalidOrientation,
    InvalidTemperature,
    InvalidTimestampOrder,
    InvalidTransmittance,
    InvalidUncertainty,
    InvalidQcmMass,
    DifferentWitnessIdentity,
    DifferentQcmIdentity,
    DifferentLocation,
    DifferentSpectralBand,
    DifferentOpticalGeometry,
    TemperatureMismatch,
    InvalidComparisonTolerance,
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for EvidenceError {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_calibration(
    eligibility: CalibrationEligibility,
    receipt_id: &str,
) -> Result<(), EvidenceError> {
    if eligibility != CalibrationEligibility::EligibleAtMeasurementTime || !nonempty(receipt_id) {
        return Err(EvidenceError::CalibrationNotEligible);
    }
    Ok(())
}

fn validate_witness(reading: &OpticalWitnessReading) -> Result<(), EvidenceError> {
    if !nonempty(&reading.witness_id) {
        return Err(EvidenceError::EmptyIdentity("witness_id"));
    }
    if !nonempty(&reading.location_id) {
        return Err(EvidenceError::EmptyIdentity("location_id"));
    }
    if reading.wavelength_nm == 0 {
        return Err(EvidenceError::InvalidWavelength);
    }
    if !reading.orientation_deg.is_finite() {
        return Err(EvidenceError::InvalidOrientation);
    }
    if !reading.temperature_k.is_finite() || reading.temperature_k <= 0.0 {
        return Err(EvidenceError::InvalidTemperature);
    }
    if !reading.transmittance.is_finite()
        || reading.transmittance < 0.0
        || reading.transmittance > 1.0
    {
        return Err(EvidenceError::InvalidTransmittance);
    }
    if !reading.absolute_uncertainty.is_finite()
        || reading.absolute_uncertainty < 0.0
        || reading.absolute_uncertainty > 1.0
    {
        return Err(EvidenceError::InvalidUncertainty);
    }
    validate_calibration(reading.calibration_eligibility, &reading.calibration_receipt_id)
}

fn validate_qcm(reading: &QcmReading) -> Result<(), EvidenceError> {
    if !nonempty(&reading.qcm_id) {
        return Err(EvidenceError::EmptyIdentity("qcm_id"));
    }
    if !nonempty(&reading.location_id) {
        return Err(EvidenceError::EmptyIdentity("location_id"));
    }
    if !reading.orientation_deg.is_finite() {
        return Err(EvidenceError::InvalidOrientation);
    }
    if !reading.temperature_k.is_finite() || reading.temperature_k <= 0.0 {
        return Err(EvidenceError::InvalidTemperature);
    }
    if !reading.areal_mass_g_cm2.is_finite() || reading.areal_mass_g_cm2 < 0.0 {
        return Err(EvidenceError::InvalidQcmMass);
    }
    if !reading.absolute_uncertainty_g_cm2.is_finite()
        || reading.absolute_uncertainty_g_cm2 < 0.0
    {
        return Err(EvidenceError::InvalidUncertainty);
    }
    validate_calibration(reading.calibration_eligibility, &reading.calibration_receipt_id)
}

fn assess_change(
    baseline_value: f64,
    current_value: f64,
    baseline_uncertainty: f64,
    current_uncertainty: f64,
    multiplier: f64,
) -> ChangeAssessment {
    let raw_change = current_value - baseline_value;
    // Triangle-bound propagation: conservative even when covariance is
    // unknown. Do not assume the two channels/readings are independent.
    let bound = multiplier * (baseline_uncertainty + current_uncertainty);
    let lower = raw_change - bound;
    let upper = raw_change + bound;
    let disposition = if lower > VALUE_TOLERANCE {
        ChangeDisposition::IncreaseEstablished
    } else if upper < -VALUE_TOLERANCE {
        ChangeDisposition::DecreaseEstablished
    } else if lower >= -VALUE_TOLERANCE && upper <= VALUE_TOLERANCE {
        ChangeDisposition::NoChangeResolved
    } else {
        ChangeDisposition::IndeterminateWithinUncertainty
    };

    ChangeAssessment {
        raw_change,
        conservative_change_lower_bound: lower,
        conservative_change_upper_bound: upper,
        disposition,
    }
}

/// Compare before/after transmission on the same witness and optical band.
///
/// This establishes a change in the measured witness transmission, not the
/// chemical cause of that change and not automatically the transmission of a
/// different production optic.
pub fn compare_optical_witness(
    baseline: &OpticalWitnessReading,
    current: &OpticalWitnessReading,
    uncertainty_multiplier: f64,
    max_temperature_delta_k: f64,
    max_orientation_delta_deg: f64,
) -> Result<ChangeAssessment, EvidenceError> {
    validate_witness(baseline)?;
    validate_witness(current)?;

    if baseline.measured_at_utc_s >= current.measured_at_utc_s {
        return Err(EvidenceError::InvalidTimestampOrder);
    }
    if baseline.witness_id != current.witness_id {
        return Err(EvidenceError::DifferentWitnessIdentity);
    }
    if baseline.location_id != current.location_id {
        return Err(EvidenceError::DifferentLocation);
    }
    if baseline.wavelength_nm != current.wavelength_nm {
        return Err(EvidenceError::DifferentSpectralBand);
    }
    if (baseline.temperature_k - current.temperature_k).abs() > max_temperature_delta_k {
        return Err(EvidenceError::TemperatureMismatch);
    }
    if (baseline.orientation_deg - current.orientation_deg).abs() > max_orientation_delta_deg {
        return Err(EvidenceError::DifferentOpticalGeometry);
    }
    if !uncertainty_multiplier.is_finite()
        || uncertainty_multiplier <= 0.0
        || !max_temperature_delta_k.is_finite()
        || max_temperature_delta_k < 0.0
        || !max_orientation_delta_deg.is_finite()
        || max_orientation_delta_deg < 0.0
    {
        return Err(EvidenceError::InvalidComparisonTolerance);
    }

    Ok(assess_change(
        baseline.transmittance,
        current.transmittance,
        baseline.absolute_uncertainty,
        current.absolute_uncertainty,
        uncertainty_multiplier,
    ))
}

/// Compare before/after deposition on the same QCM crystal.
///
/// This establishes a local QCM mass change only. It does not infer optical
/// degradation, identify deposit species, or prove mass flux on another surface.
pub fn compare_qcm_deposition(
    baseline: &QcmReading,
    current: &QcmReading,
    uncertainty_multiplier: f64,
    max_temperature_delta_k: f64,
    max_orientation_delta_deg: f64,
) -> Result<ChangeAssessment, EvidenceError> {
    validate_qcm(baseline)?;
    validate_qcm(current)?;

    if baseline.measured_at_utc_s >= current.measured_at_utc_s {
        return Err(EvidenceError::InvalidTimestampOrder);
    }
    if baseline.qcm_id != current.qcm_id {
        return Err(EvidenceError::DifferentQcmIdentity);
    }
    if baseline.location_id != current.location_id {
        return Err(EvidenceError::DifferentLocation);
    }
    if (baseline.temperature_k - current.temperature_k).abs() > max_temperature_delta_k {
        return Err(EvidenceError::TemperatureMismatch);
    }
    if (baseline.orientation_deg - current.orientation_deg).abs() > max_orientation_delta_deg {
        return Err(EvidenceError::DifferentOpticalGeometry);
    }
    if !uncertainty_multiplier.is_finite()
        || uncertainty_multiplier <= 0.0
        || !max_temperature_delta_k.is_finite()
        || max_temperature_delta_k < 0.0
        || !max_orientation_delta_deg.is_finite()
        || max_orientation_delta_deg < 0.0
    {
        return Err(EvidenceError::InvalidComparisonTolerance);
    }

    Ok(assess_change(
        baseline.areal_mass_g_cm2,
        current.areal_mass_g_cm2,
        baseline.absolute_uncertainty_g_cm2,
        current.absolute_uncertainty_g_cm2,
        uncertainty_multiplier,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn witness(time: i64, transmittance: f64, uncertainty: f64) -> OpticalWitnessReading {
        OpticalWitnessReading {
            witness_id: "witness-optic-1".to_string(),
            location_id: "receiver-window-1".to_string(),
            wavelength_nm: 1080,
            orientation_deg: 0.0,
            temperature_k: 300.0,
            measured_at_utc_s: time,
            transmittance,
            absolute_uncertainty: uncertainty,
            calibration_eligibility: CalibrationEligibility::EligibleAtMeasurementTime,
            calibration_receipt_id: "verified-cal-receipt-1".to_string(),
        }
    }

    fn qcm(time: i64, mass: f64, uncertainty: f64) -> QcmReading {
        QcmReading {
            qcm_id: "qcm-1".to_string(),
            location_id: "near-receiver-window".to_string(),
            orientation_deg: 0.0,
            temperature_k: 300.0,
            measured_at_utc_s: time,
            areal_mass_g_cm2: mass,
            absolute_uncertainty_g_cm2: uncertainty,
            calibration_eligibility: CalibrationEligibility::EligibleAtMeasurementTime,
            calibration_receipt_id: "verified-qcm-cal-1".to_string(),
        }
    }

    #[test]
    fn direct_witness_measurement_establishes_transmission_decrease() {
        let result = compare_optical_witness(&witness(10, 0.98, 0.002), &witness(20, 0.90, 0.002), 2.0, 1.0, 1.0).unwrap();
        assert_eq!(result.disposition, ChangeDisposition::DecreaseEstablished);
        assert!((result.raw_change + 0.08).abs() < 1.0e-12);
    }

    #[test]
    fn uncertainty_bound_prevents_weak_signal_from_becoming_a_degradation_claim() {
        let result = compare_optical_witness(&witness(10, 0.980, 0.010), &witness(20, 0.975, 0.010), 2.0, 1.0, 1.0).unwrap();
        assert_eq!(result.disposition, ChangeDisposition::IndeterminateWithinUncertainty);
    }

    #[test]
    fn qcm_deposition_does_not_imply_optical_transmission_loss() {
        let deposition = compare_qcm_deposition(&qcm(10, 1.0e-8, 1.0e-10), &qcm(20, 5.0e-8, 1.0e-10), 2.0, 1.0, 1.0).unwrap();
        let witness_result = compare_optical_witness(&witness(10, 0.98, 0.002), &witness(20, 0.979, 0.002), 2.0, 1.0, 1.0).unwrap();

        assert_eq!(deposition.disposition, ChangeDisposition::IncreaseEstablished);
        assert_eq!(witness_result.disposition, ChangeDisposition::IndeterminateWithinUncertainty);
    }

    #[test]
    fn optical_loss_can_be_observed_without_a_qcm_increase() {
        let qcm_result = compare_qcm_deposition(&qcm(10, 1.0e-8, 1.0e-10), &qcm(20, 1.0e-8, 1.0e-10), 2.0, 1.0, 1.0).unwrap();
        let witness_result = compare_optical_witness(&witness(10, 0.98, 0.001), &witness(20, 0.90, 0.001), 2.0, 1.0, 1.0).unwrap();

        assert_eq!(qcm_result.disposition, ChangeDisposition::NoChangeResolved);
        assert_eq!(witness_result.disposition, ChangeDisposition::DecreaseEstablished);
    }

    #[test]
    fn calibration_unknown_or_ineligible_fails_closed() {
        let mut unqualified = witness(20, 0.90, 0.001);
        unqualified.calibration_eligibility = CalibrationEligibility::Unknown;
        assert_eq!(
            compare_optical_witness(&witness(10, 0.98, 0.001), &unqualified, 2.0, 1.0, 1.0),
            Err(EvidenceError::CalibrationNotEligible)
        );
    }

    #[test]
    fn different_band_or_witness_cannot_be_compared_as_same_surface_trace() {
        let mut different_band = witness(20, 0.90, 0.001);
        different_band.wavelength_nm = 940;
        assert_eq!(
            compare_optical_witness(&witness(10, 0.98, 0.001), &different_band, 2.0, 1.0, 1.0),
            Err(EvidenceError::DifferentSpectralBand)
        );

        let mut different_witness = witness(20, 0.90, 0.001);
        different_witness.witness_id = "witness-optic-2".to_string();
        assert_eq!(
            compare_optical_witness(&witness(10, 0.98, 0.001), &different_witness, 2.0, 1.0, 1.0),
            Err(EvidenceError::DifferentWitnessIdentity)
        );
    }

    #[test]
    fn temperature_and_orientation_mismatch_are_rejected() {
        let mut hot = witness(20, 0.90, 0.001);
        hot.temperature_k = 320.0;
        assert_eq!(
            compare_optical_witness(&witness(10, 0.98, 0.001), &hot, 2.0, 1.0, 1.0),
            Err(EvidenceError::TemperatureMismatch)
        );

        let mut rotated = witness(20, 0.90, 0.001);
        rotated.orientation_deg = 5.0;
        assert_eq!(
            compare_optical_witness(&witness(10, 0.98, 0.001), &rotated, 2.0, 1.0, 1.0),
            Err(EvidenceError::DifferentOpticalGeometry)
        );
    }

    #[test]
    fn qcm_identity_and_location_are_required() {
        let mut other_qcm = qcm(20, 5.0e-8, 1.0e-10);
        other_qcm.qcm_id = "qcm-2".to_string();
        assert_eq!(
            compare_qcm_deposition(&qcm(10, 1.0e-8, 1.0e-10), &other_qcm, 2.0, 1.0, 1.0),
            Err(EvidenceError::DifferentQcmIdentity)
        );
    }

    #[test]
    fn reversed_time_and_invalid_uncertainty_are_rejected() {
        assert_eq!(
            compare_optical_witness(&witness(20, 0.98, 0.001), &witness(10, 0.90, 0.001), 2.0, 1.0, 1.0),
            Err(EvidenceError::InvalidTimestampOrder)
        );

        let mut invalid = witness(20, 0.90, f64::NAN);
        invalid.measured_at_utc_s = 30;
        assert_eq!(
            compare_optical_witness(&witness(10, 0.98, 0.001), &invalid, 2.0, 1.0, 1.0),
            Err(EvidenceError::InvalidUncertainty)
        );
    }

    #[test]
    fn invalid_comparison_threshold_is_rejected() {
        assert_eq!(
            compare_optical_witness(&witness(10, 0.98, 0.001), &witness(20, 0.90, 0.001), 0.0, 1.0, 1.0),
            Err(EvidenceError::InvalidComparisonTolerance)
        );
    }
}
