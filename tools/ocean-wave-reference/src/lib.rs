//! Dependency-free analytic reference for linear gravity-wave kinematics.
//!
//! This is a numerical oracle for tests and future renderer/physics adapters.
//! It is not a nonlinear ocean solver, directional-spectrum synthesizer, CFD
//! solver, buoyancy authority, or claim of empirical sea-state accuracy.

use std::f64::consts::TAU;

/// Linear gravity-wave kinematics for one wavenumber and water depth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveKinematics {
    /// Angular frequency, radians per second.
    pub omega_rad_s: f64,
    /// Wave period, seconds.
    pub period_s: f64,
    /// Phase velocity, metres per second.
    pub phase_velocity_m_s: f64,
    /// Group velocity, metres per second, for the linear gravity-wave model.
    pub group_velocity_m_s: f64,
    /// Wavelength, metres.
    pub wavelength_m: f64,
}

/// Invalid input or a non-finite result in the analytic reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveError {
    InvalidGravity,
    InvalidWavenumber,
    InvalidDepth,
    InvalidAmplitude,
    NonFiniteDerivedQuantity,
}

/// Evaluate the linear gravity-wave dispersion relation.
///
/// Uses omega squared = g * k * tanh(k * h), phase velocity = omega / k,
/// and group velocity = phase velocity / 2 * (1 + 2*k*h / sinh(2*k*h)).
///
/// This linear, inviscid gravity-wave reference excludes surface tension,
/// currents, breaking, nonlinear interactions, and wave-generation physics.
pub fn dispersion(
    gravity_m_s2: f64,
    wavenumber_rad_m: f64,
    depth_m: f64,
) -> Result<WaveKinematics, WaveError> {
    if !gravity_m_s2.is_finite() || gravity_m_s2 <= 0.0 {
        return Err(WaveError::InvalidGravity);
    }
    if !wavenumber_rad_m.is_finite() || wavenumber_rad_m <= 0.0 {
        return Err(WaveError::InvalidWavenumber);
    }
    if !depth_m.is_finite() || depth_m <= 0.0 {
        return Err(WaveError::InvalidDepth);
    }

    let kh = wavenumber_rad_m * depth_m;
    if !kh.is_finite() || kh <= 0.0 {
        return Err(WaveError::NonFiniteDerivedQuantity);
    }

    let omega_squared = gravity_m_s2 * wavenumber_rad_m * kh.tanh();
    if !omega_squared.is_finite() || omega_squared <= 0.0 {
        return Err(WaveError::NonFiniteDerivedQuantity);
    }

    let omega_rad_s = omega_squared.sqrt();
    let period_s = TAU / omega_rad_s;
    let phase_velocity_m_s = omega_rad_s / wavenumber_rad_m;
    let wavelength_m = TAU / wavenumber_rad_m;

    // Avoid sinh overflow in deep water, where this ratio tends to zero.
    let two_kh = 2.0 * kh;
    let ratio = if two_kh > 700.0 {
        0.0
    } else {
        two_kh / two_kh.sinh()
    };
    let group_velocity_m_s = 0.5 * phase_velocity_m_s * (1.0 + ratio);

    let values = [
        omega_rad_s,
        period_s,
        phase_velocity_m_s,
        group_velocity_m_s,
        wavelength_m,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(WaveError::NonFiniteDerivedQuantity);
    }

    Ok(WaveKinematics {
        omega_rad_s,
        period_s,
        phase_velocity_m_s,
        group_velocity_m_s,
        wavelength_m,
    })
}

/// Evaluate one-dimensional linear surface elevation:
/// eta(x,t) = amplitude * cos(k*x - omega*t + phase).
///
/// Coordinates are metres, time is seconds, wavenumber is radians per metre,
/// angular frequency is radians per second, and phase is radians.
pub fn surface_elevation(
    amplitude_m: f64,
    wavenumber_rad_m: f64,
    x_m: f64,
    omega_rad_s: f64,
    time_s: f64,
    phase_rad: f64,
) -> Result<f64, WaveError> {
    if !amplitude_m.is_finite() || amplitude_m < 0.0 {
        return Err(WaveError::InvalidAmplitude);
    }
    if !wavenumber_rad_m.is_finite() || wavenumber_rad_m <= 0.0 {
        return Err(WaveError::InvalidWavenumber);
    }
    if !x_m.is_finite()
        || !omega_rad_s.is_finite()
        || omega_rad_s <= 0.0
        || !time_s.is_finite()
        || !phase_rad.is_finite()
    {
        return Err(WaveError::NonFiniteDerivedQuantity);
    }

    let angle = wavenumber_rad_m * x_m - omega_rad_s * time_s + phase_rad;
    let elevation = amplitude_m * angle.cos();
    if !angle.is_finite() || !elevation.is_finite() {
        return Err(WaveError::NonFiniteDerivedQuantity);
    }
    Ok(elevation)
}

#[cfg(test)]
mod tests {
    use super::{dispersion, surface_elevation, WaveError};

    const FIXTURE: &str = include_str!("../fixtures/dispersion_v0.csv");

    fn near(actual: f64, expected: f64, relative_tolerance: f64) {
        let tolerance = relative_tolerance * expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.15e}, expected={expected:.15e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn frozen_dispersion_fixtures_match_reference() {
        let mut rows = FIXTURE.lines();
        let header = rows.next().expect("fixture header");
        assert_eq!(
            header,
            "case_id,gravity_m_s2,wavenumber_rad_m,depth_m,expected_omega_rad_s,expected_period_s,expected_phase_velocity_m_s,expected_group_velocity_m_s,expected_wavelength_m"
        );

        let mut count = 0;
        for row in rows {
            if row.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = row.split(',').collect();
            assert_eq!(fields.len(), 9, "malformed fixture row: {row}");
            let parse = |index: usize| -> f64 {
                fields[index]
                    .parse::<f64>()
                    .unwrap_or_else(|error| panic!("bad value in {row}: {error}"))
            };
            let actual = dispersion(parse(1), parse(2), parse(3)).expect("valid fixture");
            near(actual.omega_rad_s, parse(4), 1.0e-10);
            near(actual.period_s, parse(5), 1.0e-10);
            near(actual.phase_velocity_m_s, parse(6), 1.0e-10);
            near(actual.group_velocity_m_s, parse(7), 1.0e-10);
            near(actual.wavelength_m, parse(8), 1.0e-10);
            count += 1;
        }
        assert_eq!(count, 4, "unexpected fixture count");
    }

    #[test]
    fn dispersion_respects_deep_water_limit() {
        let g = 9.81;
        let k = 0.1;
        let result = dispersion(g, k, 1000.0).expect("valid input");
        let deep_water_omega = (g * k).sqrt();
        near(result.omega_rad_s, deep_water_omega, 1.0e-12);
        near(
            result.group_velocity_m_s,
            result.phase_velocity_m_s * 0.5,
            1.0e-12,
        );
    }

    #[test]
    fn surface_elevation_is_deterministic_and_amplitude_bounded() {
        let amplitude = 0.75;
        let omega = dispersion(9.81, 0.2, 4.0).unwrap().omega_rad_s;
        let first = surface_elevation(amplitude, 0.2, 3.0, omega, 1.25, 0.4).unwrap();
        let replay = surface_elevation(amplitude, 0.2, 3.0, omega, 1.25, 0.4).unwrap();
        assert_eq!(first, replay);
        assert!(first.abs() <= amplitude);
        near(
            surface_elevation(amplitude, 0.2, 0.0, omega, 0.0, 0.0).unwrap(),
            amplitude,
            1.0e-12,
        );
    }

    #[test]
    fn invalid_configuration_fails_closed() {
        assert_eq!(dispersion(0.0, 0.1, 10.0), Err(WaveError::InvalidGravity));
        assert_eq!(dispersion(9.81, 0.0, 10.0), Err(WaveError::InvalidWavenumber));
        assert_eq!(dispersion(9.81, 0.1, 0.0), Err(WaveError::InvalidDepth));
        assert_eq!(
            dispersion(f64::NAN, 0.1, 10.0),
            Err(WaveError::InvalidGravity)
        );
        assert_eq!(
            surface_elevation(-0.1, 0.1, 0.0, 1.0, 0.0, 0.0),
            Err(WaveError::InvalidAmplitude)
        );
    }
}
