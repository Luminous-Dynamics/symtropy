// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Analytical scale-separation evidence for the smooth Taylor-Green control.
//!
//! This module does **not** estimate a dynamic turbulence or singularity
//! concentration scale. It reports how the known fundamental Taylor-Green mode
//! relates to the exact grid/Nyquist scales of a reference configuration. The
//! result is a calibration/control observable for future refinement policy, not
//! a resolved/unresolved verdict.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::reference::{PeriodicMacConfig, ReferenceConfigError};

pub const SMOOTH_SCALE_SEPARATION_SCHEMA_ID: &str =
    "taylor-green-smooth-control-scale-separation-v0.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicalBoundaryTruncationApplicability {
    /// The reference problem is periodic; there is no physical inflow/outflow/
    /// wall truncation boundary to estimate. Discretization error still exists.
    NotApplicablePeriodicDomain,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SmoothControlScaleSeparationReport {
    pub schema_id: String,
    pub solver_profile: String,
    pub nx: usize,
    pub ny: usize,
    pub dx_m: f64,
    pub dy_m: f64,
    pub minimum_grid_scale_m: f64,
    pub fundamental_wavelength_x_m: f64,
    pub fundamental_wavelength_y_m: f64,
    /// Reciprocal-wavenumber scale `1/k`, distinct from full wavelength `2π/k`.
    pub reciprocal_mode_scale_x_m: f64,
    pub reciprocal_mode_scale_y_m: f64,
    pub cells_per_wavelength_x: f64,
    pub cells_per_wavelength_y: f64,
    pub cells_per_reciprocal_mode_scale_x: f64,
    pub cells_per_reciprocal_mode_scale_y: f64,
    pub fundamental_wavenumber_x_per_m: f64,
    pub fundamental_wavenumber_y_per_m: f64,
    pub nyquist_wavenumber_x_per_m: f64,
    pub nyquist_wavenumber_y_per_m: f64,
    /// `k_mode / k_nyquist`; smaller means greater spectral separation.
    pub mode_to_nyquist_ratio_x: f64,
    pub mode_to_nyquist_ratio_y: f64,
    pub maximum_mode_to_nyquist_ratio: f64,
    pub minimum_cells_per_wavelength: f64,
    pub minimum_cells_per_reciprocal_mode_scale: f64,
    pub physical_boundary_truncation: PhysicalBoundaryTruncationApplicability,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SmoothScaleSeparationError {
    NonCanonicalDomain,
    NonFiniteDerivedMetric(&'static str),
    Config(ReferenceConfigError),
}

impl fmt::Display for SmoothScaleSeparationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonCanonicalDomain => write!(
                f,
                "smooth-control scale separation V0.1 requires a square periodic Taylor-Green domain"
            ),
            Self::NonFiniteDerivedMetric(name) => {
                write!(
                    f,
                    "derived smooth-control scale metric {name} is non-finite"
                )
            }
            Self::Config(source) => write!(f, "invalid reference configuration: {source}"),
        }
    }
}

impl std::error::Error for SmoothScaleSeparationError {}

impl From<ReferenceConfigError> for SmoothScaleSeparationError {
    fn from(value: ReferenceConfigError) -> Self {
        Self::Config(value)
    }
}

pub fn measure_taylor_green_smooth_scale_separation(
    config: &PeriodicMacConfig,
) -> Result<SmoothControlScaleSeparationReport, SmoothScaleSeparationError> {
    config.validate()?;
    if config.nx != config.ny || config.length_x_m.to_bits() != config.length_y_m.to_bits() {
        return Err(SmoothScaleSeparationError::NonCanonicalDomain);
    }

    let dx_m = config.dx();
    let dy_m = config.dy();
    let fundamental_wavelength_x_m = config.length_x_m;
    let fundamental_wavelength_y_m = config.length_y_m;
    let fundamental_wavenumber_x_per_m = std::f64::consts::TAU / fundamental_wavelength_x_m;
    let fundamental_wavenumber_y_per_m = std::f64::consts::TAU / fundamental_wavelength_y_m;
    let reciprocal_mode_scale_x_m = 1.0 / fundamental_wavenumber_x_per_m;
    let reciprocal_mode_scale_y_m = 1.0 / fundamental_wavenumber_y_per_m;
    let nyquist_wavenumber_x_per_m = std::f64::consts::PI / dx_m;
    let nyquist_wavenumber_y_per_m = std::f64::consts::PI / dy_m;
    let cells_per_wavelength_x = fundamental_wavelength_x_m / dx_m;
    let cells_per_wavelength_y = fundamental_wavelength_y_m / dy_m;
    let cells_per_reciprocal_mode_scale_x = reciprocal_mode_scale_x_m / dx_m;
    let cells_per_reciprocal_mode_scale_y = reciprocal_mode_scale_y_m / dy_m;
    let mode_to_nyquist_ratio_x = fundamental_wavenumber_x_per_m / nyquist_wavenumber_x_per_m;
    let mode_to_nyquist_ratio_y = fundamental_wavenumber_y_per_m / nyquist_wavenumber_y_per_m;
    let maximum_mode_to_nyquist_ratio = mode_to_nyquist_ratio_x.max(mode_to_nyquist_ratio_y);
    let minimum_cells_per_wavelength = cells_per_wavelength_x.min(cells_per_wavelength_y);
    let minimum_cells_per_reciprocal_mode_scale =
        cells_per_reciprocal_mode_scale_x.min(cells_per_reciprocal_mode_scale_y);

    for (name, value) in [
        ("dx_m", dx_m),
        ("dy_m", dy_m),
        ("fundamental_wavelength_x_m", fundamental_wavelength_x_m),
        ("fundamental_wavelength_y_m", fundamental_wavelength_y_m),
        ("reciprocal_mode_scale_x_m", reciprocal_mode_scale_x_m),
        ("reciprocal_mode_scale_y_m", reciprocal_mode_scale_y_m),
        ("cells_per_wavelength_x", cells_per_wavelength_x),
        ("cells_per_wavelength_y", cells_per_wavelength_y),
        (
            "cells_per_reciprocal_mode_scale_x",
            cells_per_reciprocal_mode_scale_x,
        ),
        (
            "cells_per_reciprocal_mode_scale_y",
            cells_per_reciprocal_mode_scale_y,
        ),
        (
            "fundamental_wavenumber_x_per_m",
            fundamental_wavenumber_x_per_m,
        ),
        (
            "fundamental_wavenumber_y_per_m",
            fundamental_wavenumber_y_per_m,
        ),
        ("nyquist_wavenumber_x_per_m", nyquist_wavenumber_x_per_m),
        ("nyquist_wavenumber_y_per_m", nyquist_wavenumber_y_per_m),
        ("mode_to_nyquist_ratio_x", mode_to_nyquist_ratio_x),
        ("mode_to_nyquist_ratio_y", mode_to_nyquist_ratio_y),
        (
            "maximum_mode_to_nyquist_ratio",
            maximum_mode_to_nyquist_ratio,
        ),
        ("minimum_cells_per_wavelength", minimum_cells_per_wavelength),
        (
            "minimum_cells_per_reciprocal_mode_scale",
            minimum_cells_per_reciprocal_mode_scale,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(SmoothScaleSeparationError::NonFiniteDerivedMetric(name));
        }
    }

    Ok(SmoothControlScaleSeparationReport {
        schema_id: SMOOTH_SCALE_SEPARATION_SCHEMA_ID.to_owned(),
        solver_profile: config.profile_identity(),
        nx: config.nx,
        ny: config.ny,
        dx_m,
        dy_m,
        minimum_grid_scale_m: dx_m.min(dy_m),
        fundamental_wavelength_x_m,
        fundamental_wavelength_y_m,
        reciprocal_mode_scale_x_m,
        reciprocal_mode_scale_y_m,
        cells_per_wavelength_x,
        cells_per_wavelength_y,
        cells_per_reciprocal_mode_scale_x,
        cells_per_reciprocal_mode_scale_y,
        fundamental_wavenumber_x_per_m,
        fundamental_wavenumber_y_per_m,
        nyquist_wavenumber_x_per_m,
        nyquist_wavenumber_y_per_m,
        mode_to_nyquist_ratio_x,
        mode_to_nyquist_ratio_y,
        maximum_mode_to_nyquist_ratio,
        minimum_cells_per_wavelength,
        minimum_cells_per_reciprocal_mode_scale,
        physical_boundary_truncation:
            PhysicalBoundaryTruncationApplicability::NotApplicablePeriodicDomain,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(n: usize) -> PeriodicMacConfig {
        PeriodicMacConfig {
            nx: n,
            ny: n,
            length_x_m: std::f64::consts::TAU,
            length_y_m: std::f64::consts::TAU,
            slab_depth_m: 1.0,
            density_kg_m3: 1.0,
            kinematic_viscosity_m2_s: 0.01,
            pressure_iterations: 400,
            max_advective_cfl: 0.5,
            max_diffusion_number: 0.24,
        }
    }

    #[test]
    fn canonical_mode_has_n_cells_per_full_wavelength_and_two_over_n_nyquist_ratio() {
        for n in [8, 12, 16, 24, 32, 48] {
            let report = measure_taylor_green_smooth_scale_separation(&config(n)).unwrap();
            assert!((report.cells_per_wavelength_x - n as f64).abs() < 1.0e-12);
            assert!((report.cells_per_wavelength_y - n as f64).abs() < 1.0e-12);
            assert!((report.mode_to_nyquist_ratio_x - 2.0 / n as f64).abs() < 1.0e-14);
            assert!((report.mode_to_nyquist_ratio_y - 2.0 / n as f64).abs() < 1.0e-14);
        }
    }

    #[test]
    fn reciprocal_mode_scale_is_wavelength_over_tau() {
        let cfg = config(24);
        let report = measure_taylor_green_smooth_scale_separation(&cfg).unwrap();
        let expected = cfg.length_x_m / std::f64::consts::TAU;
        assert!((report.reciprocal_mode_scale_x_m - expected).abs() < 1.0e-14);
        assert!(
            (report.cells_per_reciprocal_mode_scale_x - 24.0 / std::f64::consts::TAU).abs()
                < 1.0e-14
        );
    }

    #[test]
    fn refinement_improves_scale_separation_without_assigning_a_threshold() {
        let coarse = measure_taylor_green_smooth_scale_separation(&config(12)).unwrap();
        let fine = measure_taylor_green_smooth_scale_separation(&config(24)).unwrap();
        assert!(fine.minimum_cells_per_wavelength > coarse.minimum_cells_per_wavelength);
        assert!(fine.maximum_mode_to_nyquist_ratio < coarse.maximum_mode_to_nyquist_ratio);
    }

    #[test]
    fn periodic_control_marks_physical_boundary_truncation_not_applicable() {
        let report = measure_taylor_green_smooth_scale_separation(&config(16)).unwrap();
        assert_eq!(
            report.physical_boundary_truncation,
            PhysicalBoundaryTruncationApplicability::NotApplicablePeriodicDomain
        );
    }

    #[test]
    fn rectangular_control_is_rejected_fail_closed() {
        let mut cfg = config(16);
        cfg.ny = 12;
        assert_eq!(
            measure_taylor_green_smooth_scale_separation(&cfg),
            Err(SmoothScaleSeparationError::NonCanonicalDomain)
        );
    }
}
