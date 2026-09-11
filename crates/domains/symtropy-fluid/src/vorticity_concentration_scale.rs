// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Measurement-only vorticity-gradient concentration scale for periodic MAC states.
//!
//! The measured length
//!
//! `ell_omega = rms(omega) / rms(grad omega)`
//!
//! is the square root of the ratio of mean-squared vorticity to mean-squared
//! vorticity gradient. In 2-D turbulence this enstrophy/palinstrophy ratio is a
//! characteristic small-scale length. Vorticity is evaluated on the natural MAC
//! dual grid with staggered first differences, and its gradient uses periodic
//! forward differences. This avoids the Nyquist checkerboard null of a centered
//! first derivative. The module reports measurements only; it does not decide
//! whether a simulation is resolved and grants no refinement/promotion authority.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::reference::{PeriodicMac2d, ReferenceConfigError};

pub const VORTICITY_CONCENTRATION_SCALE_SCHEMA_ID: &str =
    "periodic-mac-vorticity-gradient-concentration-scale-v0.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VorticityConcentrationScaleUnavailableReason {
    /// The sampled state has exactly zero RMS vorticity, so there is no
    /// vorticity structure from which to infer a vorticity length.
    ZeroVorticity,
    /// Vorticity is present but the chosen periodic difference operator reports
    /// an exactly zero vorticity gradient. Preserve this as typed unavailable
    /// evidence rather than serializing infinity or inventing a finite sentinel.
    ZeroVorticityGradient,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum VorticityConcentrationScaleValue {
    Measured {
        length_m: f64,
        /// Conservative cell count based on the coarsest axis spacing.
        cells_per_length: f64,
        effective_wavenumber_per_m: f64,
        /// Effective wavenumber divided by the smallest axis Nyquist wavenumber.
        effective_to_minimum_axis_nyquist_ratio: f64,
    },
    Unavailable(VorticityConcentrationScaleUnavailableReason),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VorticityConcentrationScaleReport {
    pub schema_id: String,
    pub solver_profile: String,
    pub nx: usize,
    pub ny: usize,
    pub dx_m: f64,
    pub dy_m: f64,
    pub minimum_grid_spacing_m: f64,
    pub maximum_grid_spacing_m: f64,
    pub rms_vorticity_per_s: f64,
    pub rms_vorticity_gradient_per_m_s: f64,
    pub mean_vorticity_squared_per_s2: f64,
    pub mean_vorticity_gradient_squared_per_m2_s2: f64,
    pub maximum_absolute_vorticity_per_s: f64,
    pub minimum_axis_nyquist_wavenumber_per_m: f64,
    pub scale: VorticityConcentrationScaleValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VorticityConcentrationScaleError {
    NonFiniteDerivedMetric(&'static str),
    Config(ReferenceConfigError),
}

impl fmt::Display for VorticityConcentrationScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteDerivedMetric(name) => write!(
                f,
                "derived vorticity concentration-scale metric {name} is non-finite"
            ),
            Self::Config(source) => write!(f, "invalid reference configuration: {source}"),
        }
    }
}

impl std::error::Error for VorticityConcentrationScaleError {}

impl From<ReferenceConfigError> for VorticityConcentrationScaleError {
    fn from(value: ReferenceConfigError) -> Self {
        Self::Config(value)
    }
}

pub fn measure_vorticity_concentration_scale(
    state: &PeriodicMac2d,
) -> Result<VorticityConcentrationScaleReport, VorticityConcentrationScaleError> {
    let config = state.config();
    config.validate()?;

    let nx = config.nx;
    let ny = config.ny;
    let dx_m = config.dx();
    let dy_m = config.dy();
    let minimum_grid_spacing_m = dx_m.min(dy_m);
    let maximum_grid_spacing_m = dx_m.max(dy_m);
    let minimum_axis_nyquist_wavenumber_per_m =
        (std::f64::consts::PI / dx_m).min(std::f64::consts::PI / dy_m);

    // Natural dual-grid MAC curl at corner (i*dx, j*dy):
    // omega = d(v)/dx - d(u)/dy. These staggered first differences remain
    // sensitive to an alternating/Nyquist face mode, unlike a centered first
    // derivative of an already interpolated cell-centered velocity field.
    let cells = nx * ny;
    let mut vorticity = vec![0.0_f64; cells];
    let mut maximum_absolute_vorticity_per_s = 0.0_f64;
    let mut vorticity_squared_sum = 0.0_f64;
    for j in 0..ny {
        let south = previous(j, ny);
        for i in 0..nx {
            let west = previous(i, nx);
            let cell_index = index(i, j, nx);
            let dv_dx =
                (state.v_faces()[cell_index] - state.v_faces()[index(west, j, nx)]) / dx_m;
            let du_dy =
                (state.u_faces()[cell_index] - state.u_faces()[index(i, south, nx)]) / dy_m;
            let omega = dv_dx - du_dy;
            ensure_finite("vorticity_per_s", omega)?;
            vorticity[cell_index] = omega;
            vorticity_squared_sum += omega * omega;
            maximum_absolute_vorticity_per_s = maximum_absolute_vorticity_per_s.max(omega.abs());
        }
    }

    // Forward differences of dual-grid vorticity also remain sensitive at the
    // Nyquist mode: |D+ exp(ikx)| = 2|sin(k dx / 2)| / dx.
    let mut vorticity_gradient_squared_sum = 0.0_f64;
    for j in 0..ny {
        let north = next(j, ny);
        for i in 0..nx {
            let east = next(i, nx);
            let cell_index = index(i, j, nx);
            let d_omega_dx =
                (vorticity[index(east, j, nx)] - vorticity[cell_index]) / dx_m;
            let d_omega_dy =
                (vorticity[index(i, north, nx)] - vorticity[cell_index]) / dy_m;
            ensure_finite("vorticity_gradient_x_per_m_s", d_omega_dx)?;
            ensure_finite("vorticity_gradient_y_per_m_s", d_omega_dy)?;
            vorticity_gradient_squared_sum +=
                d_omega_dx * d_omega_dx + d_omega_dy * d_omega_dy;
        }
    }

    let mean_vorticity_squared_per_s2 = vorticity_squared_sum / cells as f64;
    let mean_vorticity_gradient_squared_per_m2_s2 =
        vorticity_gradient_squared_sum / cells as f64;
    let rms_vorticity_per_s = mean_vorticity_squared_per_s2.sqrt();
    let rms_vorticity_gradient_per_m_s =
        mean_vorticity_gradient_squared_per_m2_s2.sqrt();

    for (name, value) in [
        ("dx_m", dx_m),
        ("dy_m", dy_m),
        ("minimum_grid_spacing_m", minimum_grid_spacing_m),
        ("maximum_grid_spacing_m", maximum_grid_spacing_m),
        ("rms_vorticity_per_s", rms_vorticity_per_s),
        (
            "rms_vorticity_gradient_per_m_s",
            rms_vorticity_gradient_per_m_s,
        ),
        (
            "mean_vorticity_squared_per_s2",
            mean_vorticity_squared_per_s2,
        ),
        (
            "mean_vorticity_gradient_squared_per_m2_s2",
            mean_vorticity_gradient_squared_per_m2_s2,
        ),
        (
            "maximum_absolute_vorticity_per_s",
            maximum_absolute_vorticity_per_s,
        ),
        (
            "minimum_axis_nyquist_wavenumber_per_m",
            minimum_axis_nyquist_wavenumber_per_m,
        ),
    ] {
        ensure_finite(name, value)?;
    }

    let scale = if rms_vorticity_per_s == 0.0 {
        VorticityConcentrationScaleValue::Unavailable(
            VorticityConcentrationScaleUnavailableReason::ZeroVorticity,
        )
    } else if rms_vorticity_gradient_per_m_s == 0.0 {
        VorticityConcentrationScaleValue::Unavailable(
            VorticityConcentrationScaleUnavailableReason::ZeroVorticityGradient,
        )
    } else {
        let length_m = rms_vorticity_per_s / rms_vorticity_gradient_per_m_s;
        let cells_per_length = length_m / maximum_grid_spacing_m;
        let effective_wavenumber_per_m = 1.0 / length_m;
        let effective_to_minimum_axis_nyquist_ratio =
            effective_wavenumber_per_m / minimum_axis_nyquist_wavenumber_per_m;
        for (name, value) in [
            ("length_m", length_m),
            ("cells_per_length", cells_per_length),
            ("effective_wavenumber_per_m", effective_wavenumber_per_m),
            (
                "effective_to_minimum_axis_nyquist_ratio",
                effective_to_minimum_axis_nyquist_ratio,
            ),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(VorticityConcentrationScaleError::NonFiniteDerivedMetric(name));
            }
        }
        VorticityConcentrationScaleValue::Measured {
            length_m,
            cells_per_length,
            effective_wavenumber_per_m,
            effective_to_minimum_axis_nyquist_ratio,
        }
    };

    Ok(VorticityConcentrationScaleReport {
        schema_id: VORTICITY_CONCENTRATION_SCALE_SCHEMA_ID.to_owned(),
        solver_profile: config.profile_identity(),
        nx,
        ny,
        dx_m,
        dy_m,
        minimum_grid_spacing_m,
        maximum_grid_spacing_m,
        rms_vorticity_per_s,
        rms_vorticity_gradient_per_m_s,
        mean_vorticity_squared_per_s2,
        mean_vorticity_gradient_squared_per_m2_s2,
        maximum_absolute_vorticity_per_s,
        minimum_axis_nyquist_wavenumber_per_m,
        scale,
    })
}

fn ensure_finite(
    name: &'static str,
    value: f64,
) -> Result<(), VorticityConcentrationScaleError> {
    if !value.is_finite() {
        return Err(VorticityConcentrationScaleError::NonFiniteDerivedMetric(name));
    }
    Ok(())
}

fn index(i: usize, j: usize, nx: usize) -> usize {
    j * nx + i
}

fn previous(index: usize, size: usize) -> usize {
    if index == 0 { size - 1 } else { index - 1 }
}

fn next(index: usize, size: usize) -> usize {
    if index + 1 == size { 0 } else { index + 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::PeriodicMacConfig;

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

    fn measured_length(report: &VorticityConcentrationScaleReport) -> f64 {
        match report.scale {
            VorticityConcentrationScaleValue::Measured { length_m, .. } => length_m,
            VorticityConcentrationScaleValue::Unavailable(reason) => {
                panic!("expected measured vorticity scale, got {reason:?}")
            }
        }
    }

    #[test]
    fn taylor_green_matches_exact_discrete_gradient_scale() {
        for n in [12, 16, 24, 32, 48] {
            let cfg = config(n);
            let dx = cfg.dx();
            let k = std::f64::consts::TAU / cfg.length_x_m;
            let discrete_k = 2.0 * (0.5 * k * dx).sin() / dx;
            let expected = 1.0 / (2.0_f64.sqrt() * discrete_k.abs());
            let state = PeriodicMac2d::taylor_green(cfg, 0.08).unwrap();
            let report = measure_vorticity_concentration_scale(&state).unwrap();
            assert!((measured_length(&report) - expected).abs() < 1.0e-12);
        }
    }

    #[test]
    fn taylor_green_scale_is_amplitude_invariant() {
        let cfg = config(24);
        let a = PeriodicMac2d::taylor_green(cfg.clone(), 0.04).unwrap();
        let b = PeriodicMac2d::taylor_green(cfg, 0.16).unwrap();
        let a = measure_vorticity_concentration_scale(&a).unwrap();
        let b = measure_vorticity_concentration_scale(&b).unwrap();
        assert!((measured_length(&a) - measured_length(&b)).abs() < 1.0e-13);
    }

    #[test]
    fn refinement_moves_discrete_scale_toward_continuum_value() {
        let continuum = 1.0 / 2.0_f64.sqrt();
        let coarse = PeriodicMac2d::taylor_green(config(12), 0.08).unwrap();
        let fine = PeriodicMac2d::taylor_green(config(48), 0.08).unwrap();
        let coarse = measure_vorticity_concentration_scale(&coarse).unwrap();
        let fine = measure_vorticity_concentration_scale(&fine).unwrap();
        assert!(
            (measured_length(&fine) - continuum).abs()
                < (measured_length(&coarse) - continuum).abs()
        );
        let coarse_cells = match coarse.scale {
            VorticityConcentrationScaleValue::Measured {
                cells_per_length, ..
            } => cells_per_length,
            _ => unreachable!(),
        };
        let fine_cells = match fine.scale {
            VorticityConcentrationScaleValue::Measured {
                cells_per_length, ..
            } => cells_per_length,
            _ => unreachable!(),
        };
        assert!(fine_cells > coarse_cells);
    }

    #[test]
    fn uniform_flow_is_typed_unavailable_zero_vorticity() {
        let state = PeriodicMac2d::uniform(config(16), 0.08, -0.03).unwrap();
        let report = measure_vorticity_concentration_scale(&state).unwrap();
        assert_eq!(
            report.scale,
            VorticityConcentrationScaleValue::Unavailable(
                VorticityConcentrationScaleUnavailableReason::ZeroVorticity
            )
        );
    }

    #[test]
    fn nyquist_checkerboard_is_not_a_zero_gradient_blind_spot() {
        let cfg = config(16);
        let dx = cfg.dx();
        let cells = cfg.nx * cfg.ny;
        let u_faces = vec![0.0; cells];
        let mut v_faces = vec![0.0; cells];
        for j in 0..cfg.ny {
            for i in 0..cfg.nx {
                v_faces[j * cfg.nx + i] = if i % 2 == 0 { 0.08 } else { -0.08 };
            }
        }
        let state = PeriodicMac2d::from_faces(cfg, u_faces, v_faces).unwrap();
        let report = measure_vorticity_concentration_scale(&state).unwrap();
        match report.scale {
            VorticityConcentrationScaleValue::Measured {
                length_m,
                cells_per_length,
                ..
            } => {
                assert!((length_m - 0.5 * dx).abs() < 1.0e-13);
                assert!((cells_per_length - 0.5).abs() < 1.0e-13);
            }
            VorticityConcentrationScaleValue::Unavailable(reason) => {
                panic!("checkerboard mode was hidden as unavailable: {reason:?}")
            }
        }
    }

    #[test]
    fn report_is_replay_deterministic() {
        let state = PeriodicMac2d::taylor_green(config(24), 0.08).unwrap();
        let a = measure_vorticity_concentration_scale(&state).unwrap();
        let b = measure_vorticity_concentration_scale(&state).unwrap();
        assert_eq!(a, b);
    }
}
