// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Arithmetic audit of numerical statements in Nikola Tesla US787,412A.
//!
//! All input constants are transcribed from the patent and must be treated as
//! patent assertions/estimates, not verified physical measurements. This
//! module checks dimensional consistency only; it does not solve Maxwell's
//! equations or identify phase, group, front, or energy velocity.

use std::f64::consts::PI;

pub const SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;
pub const EARTH_MEAN_RADIUS_M: f64 = 6_371_000.0;
pub const TESLA_STATED_MEAN_SPEED_KM_PER_S: f64 = 471_240.0;
pub const TESLA_ESTIMATED_RETURN_INTERVAL_S: f64 = 0.08484;
pub const TESLA_WAVELENGTH_MIN_M: f64 = 25_000.0;
pub const TESLA_WAVELENGTH_MAX_M: f64 = 70_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatentArithmeticAudit {
    pub stated_speed_m_per_s: f64,
    pub speed_ratio_to_c: f64,
    pub estimated_interval_s: f64,
    pub implied_round_trip_distance_m: f64,
    pub earth_circumference_m: f64,
    pub path_to_circumference_ratio: f64,
    pub frequency_at_min_wavelength_using_claimed_speed_hz: f64,
    pub frequency_at_max_wavelength_using_claimed_speed_hz: f64,
    pub frequency_at_min_wavelength_using_c_hz: f64,
    pub frequency_at_max_wavelength_using_c_hz: f64,
    pub quarter_wave_odd_multiple_min: f64,
    pub quarter_wave_odd_multiple_max: f64,
}

pub fn audit() -> PatentArithmeticAudit {
    let speed = TESLA_STATED_MEAN_SPEED_KM_PER_S * 1_000.0;
    let interval = TESLA_ESTIMATED_RETURN_INTERVAL_S;
    let round_trip = speed * interval;
    let circumference = 2.0 * PI * EARTH_MEAN_RADIUS_M;
    let lambda_min = TESLA_WAVELENGTH_MIN_M;
    let lambda_max = TESLA_WAVELENGTH_MAX_M;
    let diameter = 2.0 * EARTH_MEAN_RADIUS_M;
    PatentArithmeticAudit {
        stated_speed_m_per_s: speed,
        speed_ratio_to_c: speed / SPEED_OF_LIGHT_M_PER_S,
        estimated_interval_s: interval,
        implied_round_trip_distance_m: round_trip,
        earth_circumference_m: circumference,
        path_to_circumference_ratio: round_trip / circumference,
        frequency_at_min_wavelength_using_claimed_speed_hz: speed / lambda_min,
        frequency_at_max_wavelength_using_claimed_speed_hz: speed / lambda_max,
        frequency_at_min_wavelength_using_c_hz: SPEED_OF_LIGHT_M_PER_S / lambda_min,
        frequency_at_max_wavelength_using_c_hz: SPEED_OF_LIGHT_M_PER_S / lambda_max,
        quarter_wave_odd_multiple_min: 4.0 * diameter / lambda_max,
        quarter_wave_odd_multiple_max: 4.0 * diameter / lambda_min,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproduces_patent_speed_and_superluminal_ratio_arithmetically() {
        let result = audit();
        assert_eq!(result.stated_speed_m_per_s, 471_240_000.0);
        assert!((result.speed_ratio_to_c - 1.571_887_442_2 ).abs() < 1e-3);
    }

    #[test]
    fn patent_speed_and_interval_imply_about_one_earth_circumference() {
        let result = audit();
        assert!((result.implied_round_trip_distance_m - 39_980_001.6 ).abs() < 1.0);
        assert!((result.path_to_circumference_ratio - 1.0).abs() < 0.003);
    }

    #[test]
    fn claimed_speed_maps_claimed_wavelengths_below_twenty_khz() {
        let result = audit();
        assert!(result.frequency_at_min_wavelength_using_claimed_speed_hz < 20_000.0);
        assert!(result.frequency_at_max_wavelength_using_claimed_speed_hz < 20_000.0);
        assert!(result.frequency_at_max_wavelength_using_claimed_speed_hz
            < result.frequency_at_min_wavelength_using_claimed_speed_hz);
    }

    #[test]
    fn quarter_wave_relation_is_reported_as_very_large_odd_multiple_scale() {
        let result = audit();
        assert!(result.quarter_wave_odd_multiple_min > 700.0);
        assert!(result.quarter_wave_odd_multiple_max > result.quarter_wave_odd_multiple_min);
    }
}