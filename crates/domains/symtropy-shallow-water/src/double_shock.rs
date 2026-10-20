// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact symmetric double-shock Riemann solution for source-free 1D shallow water.
//!
//! Initially equal-depth streams move toward each other. The solution consists
//! of two outward-moving shocks with a compressed, stationary intermediate state.

use super::SolverError;

/// Exact state for the symmetric double-shock Riemann problem.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DoubleShockState {
    /// Water depth (m).
    pub depth_m: f64,
    /// Depth-averaged velocity (m/s; negative is leftward).
    pub velocity_m_s: f64,
    /// Discharge per unit width q = h*u (m²/s).
    pub discharge_m2_s: f64,
}

/// Exact self-similar solution for two symmetric, converging shallow-water states.
///
/// The initial depth is `initial_depth_m` on both sides of x=0. The left-side
/// velocity is `+inward_velocity_m_s` and the right-side velocity is its
/// negative. Flat bed and constant gravity are assumed. The shock-connected
/// intermediate depth is solved with bounded bisection from the Rankine–Hugoniot
/// wave curve.
///
/// # Errors
///
/// Returns `InvalidAnalyticalInput` for invalid/non-finite inputs or outputs,
/// or when a finite positive shock-connected star state cannot be represented.
pub fn double_shock(
    x_m: f64,
    time_s: f64,
    initial_depth_m: f64,
    inward_velocity_m_s: f64,
    gravity_m_s2: f64,
) -> Result<DoubleShockState, SolverError> {
    if !x_m.is_finite()
        || !time_s.is_finite()
        || !initial_depth_m.is_finite()
        || !inward_velocity_m_s.is_finite()
        || !gravity_m_s2.is_finite()
        || time_s <= 0.0
        || initial_depth_m <= 0.0
        || inward_velocity_m_s <= 0.0
        || gravity_m_s2 <= 0.0
    {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    let initial_celerity_m_s = (gravity_m_s2 * initial_depth_m).sqrt();
    let similarity_m_s = x_m / time_s;
    let upper_increment_m =
        2.0 * inward_velocity_m_s * (initial_depth_m / gravity_m_s2).sqrt();
    let mut lower_depth_m = initial_depth_m;
    let mut upper_depth_m = initial_depth_m + upper_increment_m;
    if !initial_celerity_m_s.is_finite()
        || !similarity_m_s.is_finite()
        || !upper_increment_m.is_finite()
        || !upper_depth_m.is_finite()
        || upper_depth_m <= initial_depth_m
    {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    // Symmetry gives u_star = 0. Solve v = (h_star-h0)
    // * sqrt(g/2 * (1/h_star + 1/h0)) for h_star > h0.
    for _ in 0..100 {
        let middle_depth_m = lower_depth_m + 0.5 * (upper_depth_m - lower_depth_m);
        let shock_curve_m_s = (middle_depth_m - initial_depth_m)
            * (0.5
                * gravity_m_s2
                * (1.0 / middle_depth_m + 1.0 / initial_depth_m))
                .sqrt();
        if !shock_curve_m_s.is_finite() {
            return Err(SolverError::InvalidAnalyticalInput);
        }
        if shock_curve_m_s > inward_velocity_m_s {
            upper_depth_m = middle_depth_m;
        } else {
            lower_depth_m = middle_depth_m;
        }
    }

    let star_depth_m = lower_depth_m + 0.5 * (upper_depth_m - lower_depth_m);
    let depth_jump_m = star_depth_m - initial_depth_m;
    if !star_depth_m.is_finite() || !depth_jump_m.is_finite() || depth_jump_m <= 0.0 {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    let left_shock_speed_m_s = -initial_depth_m * inward_velocity_m_s / depth_jump_m;
    let right_shock_speed_m_s = -left_shock_speed_m_s;
    if !left_shock_speed_m_s.is_finite() || !right_shock_speed_m_s.is_finite() {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    let (depth_m, velocity_m_s) = if similarity_m_s <= left_shock_speed_m_s {
        (initial_depth_m, inward_velocity_m_s)
    } else if similarity_m_s < right_shock_speed_m_s {
        (star_depth_m, 0.0)
    } else {
        (initial_depth_m, -inward_velocity_m_s)
    };
    let discharge_m2_s = depth_m * velocity_m_s;
    if !depth_m.is_finite() || depth_m <= 0.0 || !velocity_m_s.is_finite() || !discharge_m2_s.is_finite() {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    Ok(DoubleShockState {
        depth_m,
        velocity_m_s,
        discharge_m2_s,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{
        BoundaryConditions, CellState, FieldErrorMetrics, ShallowWater1D, SolverConfig,
        field_error_metrics,
    };
    use super::*;

    fn config(dx: f64) -> SolverConfig {
        SolverConfig {
            cell_width_m: dx,
            cfl: 0.4,
            ..SolverConfig::default()
        }
    }

    fn exact_star_and_shock_speed(h0: f64, inward_velocity: f64, gravity: f64) -> (f64, f64) {
        let mut lower = h0;
        let mut upper = h0 + 2.0 * inward_velocity * (h0 / gravity).sqrt();
        for _ in 0..100 {
            let middle = lower + 0.5 * (upper - lower);
            let curve = (middle - h0) * (0.5 * gravity * (1.0 / middle + 1.0 / h0)).sqrt();
            if curve > inward_velocity {
                upper = middle;
            } else {
                lower = middle;
            }
        }
        let star_depth = lower + 0.5 * (upper - lower);
        let shock_speed = h0 * inward_velocity / (star_depth - h0);
        (star_depth, shock_speed)
    }

    #[test]
    fn double_shock_matches_outer_states_star_state_and_jump_conditions() {
        let h0: f64 = 1.0;
        let inward_velocity: f64 = 1.0;
        let gravity: f64 = 9.81;
        let time: f64 = 0.2;
        let (star_depth, shock_speed) = exact_star_and_shock_speed(h0, inward_velocity, gravity);

        let left = double_shock(-10.0 * time, time, h0, inward_velocity, gravity).unwrap();
        assert!((left.depth_m - h0).abs() < 1.0e-12);
        assert!((left.velocity_m_s - inward_velocity).abs() < 1.0e-12);

        let star = double_shock(0.0, time, h0, inward_velocity, gravity).unwrap();
        assert!((star.depth_m - star_depth).abs() < 1.0e-12);
        assert!(star.depth_m > h0);
        assert_eq!(star.velocity_m_s, 0.0);
        assert_eq!(star.discharge_m2_s, 0.0);

        let left_shock = double_shock(
            (-shock_speed - 1.0e-4) * time,
            time,
            h0,
            inward_velocity,
            gravity,
        )
        .unwrap();
        let left_star = double_shock(
            (-shock_speed + 1.0e-4) * time,
            time,
            h0,
            inward_velocity,
            gravity,
        )
        .unwrap();
        assert!((left_shock.depth_m - h0).abs() < 1.0e-12);
        assert!((left_star.depth_m - star_depth).abs() < 1.0e-12);

        let right_star = double_shock(
            (shock_speed - 1.0e-4) * time,
            time,
            h0,
            inward_velocity,
            gravity,
        )
        .unwrap();
        let right_shock = double_shock(
            (shock_speed + 1.0e-4) * time,
            time,
            h0,
            inward_velocity,
            gravity,
        )
        .unwrap();
        assert!((right_star.depth_m - star_depth).abs() < 1.0e-12);
        assert!((right_shock.depth_m - h0).abs() < 1.0e-12);

        // Both mass and momentum satisfy the Rankine–Hugoniot jump at the left shock.
        let s_left = -shock_speed;
        let q_left = h0 * inward_velocity;
        let mass_residual = s_left * (star_depth - h0) - (star.discharge_m2_s - q_left);
        let flux_left = q_left * q_left / h0 + 0.5 * gravity * h0.powi(2);
        let flux_star = 0.5 * gravity * star_depth.powi(2);
        let momentum_residual = s_left * (star.discharge_m2_s - q_left) - (flux_star - flux_left);
        assert!(mass_residual.abs() < 1.0e-12);
        assert!(momentum_residual.abs() < 1.0e-10);
    }

    fn field_errors(
        dx: f64,
    ) -> (
        FieldErrorMetrics,
        FieldErrorMetrics,
        FieldErrorMetrics,
        usize,
    ) {
        let cfg = config(dx);
        let x_min = -5.0;
        let x_max = 5.0;
        let t_end = 0.2;
        let h0 = 1.0;
        let inward_velocity = 1.0;
        let count = ((x_max - x_min) / dx).round() as usize;
        let cells = (0..count)
            .map(|index| {
                let x = x_min + (index as f64 + 0.5) * dx;
                let velocity = if x < 0.0 { inward_velocity } else { -inward_velocity };
                CellState {
                    depth_m: h0,
                    discharge_m2_s: h0 * velocity,
                    bed_elevation_m: 0.0,
                }
            })
            .collect();
        let mut solver = ShallowWater1D::new(cells, cfg).unwrap();
        let mut time = 0.0;
        let mut steps = 0_usize;
        while time < t_end - 1.0e-14 {
            let dt = solver
                .advance(t_end - time, cfg, BoundaryConditions::OPEN)
                .unwrap();
            time += dt;
            steps += 1;
            assert!(steps < 100_000, "double-shock integration did not terminate");
        }

        let mut actual_depth = Vec::with_capacity(count);
        let mut expected_depth = Vec::with_capacity(count);
        let mut actual_discharge = Vec::with_capacity(count);
        let mut expected_discharge = Vec::with_capacity(count);
        let mut actual_velocity = Vec::with_capacity(count);
        let mut expected_velocity = Vec::with_capacity(count);
        for (index, cell) in solver.cells().iter().enumerate() {
            let x = x_min + (index as f64 + 0.5) * dx;
            let expected =
                double_shock(x, t_end, h0, inward_velocity, cfg.gravity_m_s2).unwrap();
            actual_depth.push(cell.depth_m);
            expected_depth.push(expected.depth_m);
            actual_discharge.push(cell.discharge_m2_s);
            expected_discharge.push(expected.discharge_m2_s);
            if expected.depth_m > cfg.dry_depth_m {
                actual_velocity.push(cell.velocity_m_s(cfg.dry_depth_m));
                expected_velocity.push(expected.velocity_m_s);
            }
        }

        let depth = field_error_metrics(&actual_depth, &expected_depth).unwrap();
        let discharge = field_error_metrics(&actual_discharge, &expected_discharge).unwrap();
        let velocity = field_error_metrics(&actual_velocity, &expected_velocity).unwrap();
        println!(
            "DOUBLE_SHOCK dx_m={dx:.6} cells={count} steps={steps} time_s={t_end:.3} depth_rmse_m={:.9} discharge_rmse_m2_s={:.9} wet_velocity_rmse_m_s={:.9} wet_velocity_samples={}",
            depth.root_mean_squared_error,
            discharge.root_mean_squared_error,
            velocity.root_mean_squared_error,
            velocity.sample_count,
        );
        (depth, discharge, velocity, steps)
    }

    #[test]
    fn double_shock_errors_decrease_across_three_grid_resolutions() {
        let coarse = field_errors(0.1);
        let medium = field_errors(0.05);
        let fine = field_errors(0.025);

        for (label, coarse_metric, medium_metric, fine_metric) in [
            ("depth", coarse.0, medium.0, fine.0),
            ("discharge", coarse.1, medium.1, fine.1),
            ("wet velocity", coarse.2, medium.2, fine.2),
        ] {
            assert!(
                medium_metric.root_mean_squared_error < coarse_metric.root_mean_squared_error,
                "{label} refinement did not improve RMSE: coarse={}, medium={}",
                coarse_metric.root_mean_squared_error,
                medium_metric.root_mean_squared_error
            );
            assert!(
                fine_metric.root_mean_squared_error < medium_metric.root_mean_squared_error,
                "{label} refinement did not improve RMSE: medium={}, fine={}",
                medium_metric.root_mean_squared_error,
                fine_metric.root_mean_squared_error
            );
        }
    }

    #[test]
    fn double_shock_rejects_invalid_inputs() {
        for args in [
            (0.0, 0.0, 1.0, 1.0, 9.81),
            (0.0, 1.0, 0.0, 1.0, 9.81),
            (0.0, 1.0, 1.0, 0.0, 9.81),
            (0.0, 1.0, 1.0, 1.0, -9.81),
            (f64::NAN, 1.0, 1.0, 1.0, 9.81),
        ] {
            assert_eq!(
                double_shock(args.0, args.1, args.2, args.3, args.4),
                Err(SolverError::InvalidAnalyticalInput)
            );
        }
    }
}
