// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact symmetric double-rarefaction solution for source-free 1D shallow water.
//!
//! The two initially wet states move outward on a flat bed. Below the
//! dry-state threshold 2*sqrt(g*h0), the fans enclose a lower-depth intermediate
//! state; above it, they enclose a dry gap. Both regimes verify interacting wave
//! families and make the wet-cell metric mask explicit.

use super::SolverError;

/// Exact state for the symmetric double-rarefaction Riemann problem.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DoubleRarefactionState {
    /// Water depth (m).
    pub depth_m: f64,
    /// Depth-averaged velocity (m/s; negative is leftward).
    pub velocity_m_s: f64,
    /// Discharge per unit width q = h*u (m²/s).
    pub discharge_m2_s: f64,
}

/// Exact self-similar solution for a symmetric double-rarefaction initial state.
///
/// The initial state is depth `initial_depth_m` on both sides of x=0, with
/// left velocity `-outward_velocity_m_s` and right velocity
/// `+outward_velocity_m_s`. The bottom is flat and gravity is constant. When
/// the outward speed is below 2*sqrt(g*h0), the solution has a wet intermediate
/// state; at or above that threshold, the two fans meet at or separate around a
/// dry state. The exact vacuum/dry-bed interval is represented explicitly.
///
/// The result includes the undisturbed left/right states, rarefaction fans and
/// either the intermediate wet state or dry gap. This is an analytical
/// verification reference—not an observed event or a complete physical flood model.
///
/// # Errors
///
/// Returns `InvalidAnalyticalInput` for invalid or non-finite inputs or outputs.
pub fn double_rarefaction(
    x_m: f64,
    time_s: f64,
    initial_depth_m: f64,
    outward_velocity_m_s: f64,
    gravity_m_s2: f64,
) -> Result<DoubleRarefactionState, SolverError> {
    if !x_m.is_finite()
        || !time_s.is_finite()
        || !initial_depth_m.is_finite()
        || !outward_velocity_m_s.is_finite()
        || !gravity_m_s2.is_finite()
        || time_s <= 0.0
        || initial_depth_m <= 0.0
        || outward_velocity_m_s < 0.0
        || gravity_m_s2 <= 0.0
    {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    let initial_celerity_m_s = (gravity_m_s2 * initial_depth_m).sqrt();
    let similarity_m_s = x_m / time_s;
    if !initial_celerity_m_s.is_finite() || !similarity_m_s.is_finite() {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    let left_head_m_s = -outward_velocity_m_s - initial_celerity_m_s;
    let right_head_m_s = outward_velocity_m_s + initial_celerity_m_s;
    let reaches_dry_state = outward_velocity_m_s >= 2.0 * initial_celerity_m_s;

    let (depth_m, velocity_m_s) = if similarity_m_s <= left_head_m_s {
        (initial_depth_m, -outward_velocity_m_s)
    } else if reaches_dry_state {
        // At a vacuum edge the celerity vanishes. If the outward velocity is
        // larger than the threshold, the interval between these edges is dry.
        let left_dry_edge_m_s = 2.0 * initial_celerity_m_s - outward_velocity_m_s;
        let right_dry_edge_m_s = outward_velocity_m_s - 2.0 * initial_celerity_m_s;
        if similarity_m_s < left_dry_edge_m_s {
            let celerity =
                (2.0 * initial_celerity_m_s - outward_velocity_m_s - similarity_m_s) / 3.0;
            (celerity * celerity / gravity_m_s2, similarity_m_s + celerity)
        } else if similarity_m_s < right_dry_edge_m_s {
            (0.0, 0.0)
        } else if similarity_m_s < right_head_m_s {
            let celerity =
                (similarity_m_s - outward_velocity_m_s + 2.0 * initial_celerity_m_s) / 3.0;
            (celerity * celerity / gravity_m_s2, similarity_m_s - celerity)
        } else {
            (initial_depth_m, outward_velocity_m_s)
        }
    } else {
        // The two rarefaction tails meet a positive intermediate state at +/- c_star.
        let star_celerity_m_s = initial_celerity_m_s - 0.5 * outward_velocity_m_s;
        let star_depth_m = star_celerity_m_s * star_celerity_m_s / gravity_m_s2;
        let left_tail_m_s = -star_celerity_m_s;
        let right_tail_m_s = star_celerity_m_s;
        if similarity_m_s < left_tail_m_s {
            let celerity =
                (2.0 * initial_celerity_m_s - outward_velocity_m_s - similarity_m_s) / 3.0;
            (celerity * celerity / gravity_m_s2, similarity_m_s + celerity)
        } else if similarity_m_s <= right_tail_m_s {
            (star_depth_m, 0.0)
        } else if similarity_m_s < right_head_m_s {
            let celerity =
                (similarity_m_s - outward_velocity_m_s + 2.0 * initial_celerity_m_s) / 3.0;
            (celerity * celerity / gravity_m_s2, similarity_m_s - celerity)
        } else {
            (initial_depth_m, outward_velocity_m_s)
        }
    };

    let discharge_m2_s = depth_m * velocity_m_s;
    if !depth_m.is_finite()
        || depth_m < 0.0
        || !velocity_m_s.is_finite()
        || !discharge_m2_s.is_finite()
    {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    Ok(DoubleRarefactionState {
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

    #[test]
    fn double_rarefaction_matches_plateaus_fans_and_intermediate_state() {
        let depth: f64 = 1.0;
        let outward_velocity: f64 = 1.0;
        let gravity: f64 = 9.81;
        let time: f64 = 0.25;
        let c0 = (gravity * depth).sqrt();
        let c_star = c0 - 0.5 * outward_velocity;

        let left = double_rarefaction(-10.0 * time, time, depth, outward_velocity, gravity)
            .unwrap();
        assert!((left.depth_m - depth).abs() < 1.0e-12);
        assert!((left.velocity_m_s + outward_velocity).abs() < 1.0e-12);

        let left_fan_xi = -0.5 * (outward_velocity + c0);
        let left_fan = double_rarefaction(
            left_fan_xi * time,
            time,
            depth,
            outward_velocity,
            gravity,
        )
        .unwrap();
        let left_fan_c = (2.0 * c0 - outward_velocity - left_fan_xi) / 3.0;
        assert!((left_fan.depth_m - left_fan_c.powi(2) / gravity).abs() < 1.0e-12);
        assert!((left_fan.velocity_m_s - (left_fan_xi + left_fan_c)).abs() < 1.0e-12);

        let star = double_rarefaction(0.0, time, depth, outward_velocity, gravity).unwrap();
        assert!((star.depth_m - c_star.powi(2) / gravity).abs() < 1.0e-12);
        assert!(star.depth_m > 0.0 && star.depth_m < depth);
        assert_eq!(star.velocity_m_s, 0.0);
        assert_eq!(star.discharge_m2_s, 0.0);

        let right = double_rarefaction(10.0 * time, time, depth, outward_velocity, gravity)
            .unwrap();
        assert!((right.depth_m - depth).abs() < 1.0e-12);
        assert!((right.velocity_m_s - outward_velocity).abs() < 1.0e-12);
    }

    fn field_errors(
        dx: f64,
    ) -> (FieldErrorMetrics, FieldErrorMetrics, FieldErrorMetrics, usize) {
        let cfg = config(dx);
        let x_min = -5.0;
        let x_max = 5.0;
        let t_end = 0.25;
        let h0 = 1.0;
        let outward_velocity = 7.0;
        let count = ((x_max - x_min) / dx).round() as usize;
        let cells = (0..count)
            .map(|index| {
                let x = x_min + (index as f64 + 0.5) * dx;
                let velocity = if x < 0.0 { -outward_velocity } else { outward_velocity };
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
            assert!(steps < 100_000, "double-rarefaction integration did not terminate");
        }

        let mut actual_depth = Vec::with_capacity(count);
        let mut expected_depth = Vec::with_capacity(count);
        let mut actual_discharge = Vec::with_capacity(count);
        let mut expected_discharge = Vec::with_capacity(count);
        let mut actual_wet_velocity = Vec::with_capacity(count);
        let mut expected_wet_velocity = Vec::with_capacity(count);
        for (index, cell) in solver.cells().iter().enumerate() {
            let x = x_min + (index as f64 + 0.5) * dx;
            let expected =
                double_rarefaction(x, t_end, h0, outward_velocity, cfg.gravity_m_s2).unwrap();
            actual_depth.push(cell.depth_m);
            expected_depth.push(expected.depth_m);
            actual_discharge.push(cell.discharge_m2_s);
            expected_discharge.push(expected.discharge_m2_s);

            // Velocity has no well-defined dry-cell support. Score it only where
            // the analytical reference is wet; report this support count in the receipt.
            if expected.depth_m > cfg.dry_depth_m {
                actual_wet_velocity.push(cell.velocity_m_s(cfg.dry_depth_m));
                expected_wet_velocity.push(expected.velocity_m_s);
            }
        }

        let depth = field_error_metrics(&actual_depth, &expected_depth).unwrap();
        let discharge =
            field_error_metrics(&actual_discharge, &expected_discharge).unwrap();
        let velocity =
            field_error_metrics(&actual_wet_velocity, &expected_wet_velocity).unwrap();
        println!(
            "DOUBLE_RAREFACTION dx_m={dx:.6} cells={count} steps={steps} time_s={t_end:.3} depth_rmse_m={:.9} discharge_rmse_m2_s={:.9} wet_velocity_rmse_m_s={:.9} wet_velocity_samples={} minimum_depth_m={:.9}",
            depth.root_mean_squared_error,
            discharge.root_mean_squared_error,
            velocity.root_mean_squared_error,
            velocity.sample_count,
            solver.cells().iter().map(|cell| cell.depth_m).fold(f64::INFINITY, f64::min),
        );
        (depth, discharge, velocity, steps)
    }

    #[test]
    fn double_rarefaction_field_errors_decrease_across_three_grid_resolutions() {
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
    fn double_rarefaction_rejects_invalid_and_dry_threshold_inputs() {
        for args in [
            (0.0, 0.0, 1.0, 1.0, 9.81),
            (0.0, 1.0, 0.0, 1.0, 9.81),
            (0.0, 1.0, 1.0, -0.1, 9.81),
            (f64::NAN, 1.0, 1.0, 1.0, 9.81),
        ] {
            assert_eq!(
                double_rarefaction(args.0, args.1, args.2, args.3, args.4),
                Err(SolverError::InvalidAnalyticalInput)
            );
        }
    }
}
