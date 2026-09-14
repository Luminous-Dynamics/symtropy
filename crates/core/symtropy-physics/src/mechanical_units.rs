// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Explicit conversion between solver-native mechanical units and SI energy.
//!
//! The rigid-body solver intentionally accepts unitless `f64` coordinates, mass,
//! and time increments. A numerical kinetic-energy value therefore becomes Joules
//! only when the embedding application establishes how one solver mass, length,
//! and time unit map to kilograms, metres, and seconds.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MechanicalUnitCalibrationError {
    InvalidMassScale,
    InvalidLengthScale,
    InvalidTimeScale,
    UnrepresentableEnergyScale,
    InvalidEnergyMagnitude,
    InvalidSignedEnergy,
    UnrepresentableJoules,
}

/// Checked M-L-T calibration for interpreting solver mechanical energy in SI.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MechanicalUnitCalibration {
    kilograms_per_mass_unit: f64,
    meters_per_length_unit: f64,
    seconds_per_time_unit: f64,
    joules_per_solver_energy_unit: f64,
}

impl MechanicalUnitCalibration {
    pub fn new(
        kilograms_per_mass_unit: f64,
        meters_per_length_unit: f64,
        seconds_per_time_unit: f64,
    ) -> Result<Self, MechanicalUnitCalibrationError> {
        if !kilograms_per_mass_unit.is_finite() || kilograms_per_mass_unit <= 0.0 {
            return Err(MechanicalUnitCalibrationError::InvalidMassScale);
        }
        if !meters_per_length_unit.is_finite() || meters_per_length_unit <= 0.0 {
            return Err(MechanicalUnitCalibrationError::InvalidLengthScale);
        }
        if !seconds_per_time_unit.is_finite() || seconds_per_time_unit <= 0.0 {
            return Err(MechanicalUnitCalibrationError::InvalidTimeScale);
        }

        let length_squared = meters_per_length_unit * meters_per_length_unit;
        let time_squared = seconds_per_time_unit * seconds_per_time_unit;
        let joules_per_solver_energy_unit =
            kilograms_per_mass_unit * length_squared / time_squared;
        if !length_squared.is_finite()
            || !time_squared.is_finite()
            || !joules_per_solver_energy_unit.is_finite()
            || joules_per_solver_energy_unit <= 0.0
        {
            return Err(MechanicalUnitCalibrationError::UnrepresentableEnergyScale);
        }

        Ok(Self {
            kilograms_per_mass_unit,
            meters_per_length_unit,
            seconds_per_time_unit,
            joules_per_solver_energy_unit,
        })
    }

    pub const fn kilograms_per_mass_unit(self) -> f64 {
        self.kilograms_per_mass_unit
    }

    pub const fn meters_per_length_unit(self) -> f64 {
        self.meters_per_length_unit
    }

    pub const fn seconds_per_time_unit(self) -> f64 {
        self.seconds_per_time_unit
    }

    pub const fn joules_per_solver_energy_unit(self) -> f64 {
        self.joules_per_solver_energy_unit
    }

    /// Convert a non-negative solver mechanical-energy magnitude to Joules.
    ///
    /// A positive input is not permitted to underflow to exact zero: that would
    /// silently erase a real mechanical-energy quantity at the unit boundary.
    pub fn energy_to_joules(
        self,
        solver_energy: f64,
    ) -> Result<f64, MechanicalUnitCalibrationError> {
        if !solver_energy.is_finite() || solver_energy < 0.0 {
            return Err(MechanicalUnitCalibrationError::InvalidEnergyMagnitude);
        }
        let joules = solver_energy * self.joules_per_solver_energy_unit;
        if !joules.is_finite()
            || joules < 0.0
            || (solver_energy > 0.0 && joules == 0.0)
        {
            return Err(MechanicalUnitCalibrationError::UnrepresentableJoules);
        }
        Ok(joules)
    }

    /// Convert a signed solver energy delta to Joules while preserving sign.
    ///
    /// A nonzero input is not permitted to underflow to exact zero, because that
    /// would erase both magnitude and the dissipation/injection distinction.
    pub fn signed_energy_to_joules(
        self,
        solver_energy_delta: f64,
    ) -> Result<f64, MechanicalUnitCalibrationError> {
        if !solver_energy_delta.is_finite() {
            return Err(MechanicalUnitCalibrationError::InvalidSignedEnergy);
        }
        let joules = solver_energy_delta * self.joules_per_solver_energy_unit;
        if !joules.is_finite() || (solver_energy_delta != 0.0 && joules == 0.0) {
            return Err(MechanicalUnitCalibrationError::UnrepresentableJoules);
        }
        Ok(joules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn si_identity_maps_one_solver_energy_unit_to_one_joule() {
        let calibration = MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap();
        assert_eq!(calibration.joules_per_solver_energy_unit(), 1.0);
        assert_eq!(calibration.energy_to_joules(2.5).unwrap(), 2.5);
        assert_eq!(calibration.signed_energy_to_joules(-2.5).unwrap(), -2.5);
    }

    #[test]
    fn pixel_like_length_scale_changes_energy_quadratically() {
        // 32 solver length units per metre => 1 world unit = 1/32 m.
        let calibration = MechanicalUnitCalibration::new(1.0, 1.0 / 32.0, 1.0).unwrap();
        assert_eq!(calibration.joules_per_solver_energy_unit(), 1.0 / 1024.0);
        assert_eq!(calibration.energy_to_joules(1024.0).unwrap(), 1.0);
    }

    #[test]
    fn complete_mlt_conversion_is_dimensionally_composed() {
        let calibration = MechanicalUnitCalibration::new(2.0, 3.0, 0.5).unwrap();
        // 2 kg * (3 m)^2 / (0.5 s)^2 = 72 J per solver-energy unit.
        assert_eq!(calibration.joules_per_solver_energy_unit(), 72.0);
        assert_eq!(calibration.energy_to_joules(0.5).unwrap(), 36.0);
        assert_eq!(calibration.signed_energy_to_joules(-0.5).unwrap(), -36.0);
    }

    #[test]
    fn each_base_scale_fails_closed_when_non_positive_or_non_finite() {
        for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                MechanicalUnitCalibration::new(invalid, 1.0, 1.0),
                Err(MechanicalUnitCalibrationError::InvalidMassScale)
            );
            assert_eq!(
                MechanicalUnitCalibration::new(1.0, invalid, 1.0),
                Err(MechanicalUnitCalibrationError::InvalidLengthScale)
            );
            assert_eq!(
                MechanicalUnitCalibration::new(1.0, 1.0, invalid),
                Err(MechanicalUnitCalibrationError::InvalidTimeScale)
            );
        }
    }

    #[test]
    fn derived_energy_scale_overflow_fails_closed() {
        assert_eq!(
            MechanicalUnitCalibration::new(f64::MAX, f64::MAX, 1.0),
            Err(MechanicalUnitCalibrationError::UnrepresentableEnergyScale)
        );
        assert_eq!(
            MechanicalUnitCalibration::new(1.0, 1.0, f64::MIN_POSITIVE),
            Err(MechanicalUnitCalibrationError::UnrepresentableEnergyScale)
        );
    }

    #[test]
    fn magnitude_and_signed_conversion_have_distinct_domains() {
        let calibration = MechanicalUnitCalibration::new(1.0, 2.0, 1.0).unwrap();
        assert_eq!(
            calibration.energy_to_joules(-1.0),
            Err(MechanicalUnitCalibrationError::InvalidEnergyMagnitude)
        );
        assert_eq!(calibration.signed_energy_to_joules(-1.0).unwrap(), -4.0);
        assert_eq!(
            calibration.signed_energy_to_joules(f64::NAN),
            Err(MechanicalUnitCalibrationError::InvalidSignedEnergy)
        );
    }

    #[test]
    fn finite_input_that_overflows_joules_is_rejected() {
        let calibration = MechanicalUnitCalibration::new(1.0e100, 1.0e100, 1.0).unwrap();
        assert_eq!(
            calibration.energy_to_joules(1.0e100),
            Err(MechanicalUnitCalibrationError::UnrepresentableJoules)
        );
    }

    #[test]
    fn positive_magnitude_underflow_to_zero_is_rejected() {
        let calibration = MechanicalUnitCalibration::new(f64::MIN_POSITIVE, 1.0, 1.0).unwrap();
        assert_eq!(
            calibration.energy_to_joules(f64::MIN_POSITIVE),
            Err(MechanicalUnitCalibrationError::UnrepresentableJoules)
        );
    }

    #[test]
    fn signed_underflow_to_zero_is_rejected_without_erasing_sign() {
        let calibration = MechanicalUnitCalibration::new(f64::MIN_POSITIVE, 1.0, 1.0).unwrap();
        assert_eq!(
            calibration.signed_energy_to_joules(-f64::MIN_POSITIVE),
            Err(MechanicalUnitCalibrationError::UnrepresentableJoules)
        );
    }
}
