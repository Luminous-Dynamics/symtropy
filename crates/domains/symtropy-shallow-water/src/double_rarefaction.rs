// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact symmetric double-rarefaction solution for source-free 1D shallow water.
//!
//! The two initially wet states move outward on a flat bed. This gives two
//! rarefaction fans with a constant, lower-depth intermediate state, provided
//! the initial outward speed is below the dry-state threshold 2*sqrt(g*h0).
//! This analytical case verifies the solver on two interacting wave families.

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
/// `+outward_velocity_m_s`. The bottom is flat, gravity is constant, both
/// initial sides are wet, and the speed must be below the dry-state threshold.
///
/// The result includes the undisturbed left/right states, each rarefaction fan,
/// and the constant intermediate state. This is an analytical verification
/// reference—not an observed event or a complete physical flood model.
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
    if !initial_celerity_m_s.is_finite()
        || !similarity_m_s.is_finite()
        || outward_velocity_m_s >= 2.0 * initial_celerity_m_s
    {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    // The two rarefaction tails meet the intermediate state at +/- c_star.
    let star_celerity_m_s = initial_celerity_m_s - 0.5 * outward_velocity_m_s;
    let star_depth_m = star_celerity_m_s * star_celerity_m_s / gravity_m_s2;
    let left_head_m_s = -outward_velocity_m_s - initial_celerity_m_s;
    let right_head_m_s = outward_velocity_m_s + initial_celerity_m_s;
    let left_tail_m_s = -star_celerity_m_s;
    let right_tail_m_s = star_celerity_m_s;

    let (depth_m, velocity_m_s) = if similarity_m_s <= left_head_m_s {
        (initial_depth_m, -outward_velocity_m_s)
    } else if similarity_m_s < left_tail_m_s {
        let celerity = (2.0 * initial_celerity_m_s
            - outward_velocity_m_s
            - similarity_m_s)
            / 3.0;
        (celerity * celerity / gravity_m_s2, similarity_m_s + celerity)
    } else if similarity_m_s <= right_tail_m_s {
        (star_depth_m, 0.0)
    } else if similarity_m_s < right_head_m_s {
        let celerity = (similarity_m_s
            - outward_velocity_m_s
            + 2.0 * initial_celerity_m_s)
            / 3.0;
        (celerity * celerity / gravity_m_s2, similarity_m_s - celerity)
    } else {
        (initial_depth_m, outward_velocity_m_s)
    };

    let discharge_m2_s = depth_m * velocity_m_s;
    if !depth_m.is_finite()
        || depth_m <= 0.0
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
    use super::super::{BoundaryConditions, CellState, ShallowWater1D, SolverConfig};
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

    fn depth_rmse(dx: f64) -> f64 {
        let cfg = config(dx);
        let x_min = -5.0;
        let x_max = 5.0;
        let t_end = 0.25;
        let h0 = 1.0;
        let outward_velocity = 1.0;
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

        let squared_error = solver
            .cells()
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let x = x_min + (index as f64 + 0.5) * dx;
                let expected =
                    double_rarefaction(x, t_end, h0, outward_velocity, cfg.gravity_m_s2).unwrap();
                (cell.depth_m - expected.depth_m).powi(2)
            })
            .sum::<f64>();
        let rmse = (squared_error / count as f64).sqrt();
        println!(
            "DOUBLE_RAREFACTION dx_m={dx:.6} cells={count} steps={steps} time_s={t_end:.3} depth_rmse_m={rmse:.9}"
        );
        rmse
    }

    #[test]
    fn double_rarefaction_error_decreases_across_three_grid_resolutions() {
        let coarse = depth_rmse(0.1);
        let medium = depth_rmse(0.05);
        let fine = depth_rmse(0.025);
        assert!(
            medium < coarse,
            "refinement did not improve RMSE: coarse={coarse}, medium={medium}"
        );
        assert!(
            fine < medium,
            "refinement did not improve RMSE: medium={medium}, fine={fine}"
        );
    }

    #[test]
    fn double_rarefaction_rejects_invalid_and_dry_threshold_inputs() {
        for args in [
            (0.0, 0.0, 1.0, 1.0, 9.81),
            (0.0, 1.0, 0.0, 1.0, 9.81),
            (0.0, 1.0, 1.0, -0.1, 9.81),
            (0.0, 1.0, 1.0, 10.0, 9.81),
            (f64::NAN, 1.0, 1.0, 1.0, 9.81),
        ] {
            assert_eq!(
                double_rarefaction(args.0, args.1, args.2, args.3, args.4),
                Err(SolverError::InvalidAnalyticalInput)
            );
        }
    }
}
