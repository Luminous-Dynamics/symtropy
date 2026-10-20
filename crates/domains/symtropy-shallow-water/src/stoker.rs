// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact wet-bed dam-break Riemann solution for the 1D shallow-water equations.
//!
//! This case complements the dry-bed Ritter rarefaction with a right-moving shock.
//! It assumes a flat bed, zero initial velocities, constant gravity, and depths
//! h_left > h_right > 0. It is an analytical verification target, not field data.

use super::{BoundaryConditions, CellState, ShallowWater1D, SolverConfig, SolverError};

/// Exact shallow-water state for a dam break with water on both sides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StokerState {
    /// Water depth (m).
    pub depth_m: f64,
    /// Depth-averaged velocity (m/s).
    pub velocity_m_s: f64,
    /// Discharge per unit width, q = h*u (m²/s).
    pub discharge_m2_s: f64,
}

/// Exact Riemann solution for a flat-bed wet-bed dam break.
///
/// Initial state is stationary depth `left_depth_m` for x < 0 and
/// `right_depth_m` for x > 0, with `left_depth_m > right_depth_m > 0`.
/// The solution consists of a left rarefaction, a constant star region, and
/// a right-moving shock. This function solves the star depth by bisection.
///
/// # Errors
///
/// Returns `InvalidAnalyticalInput` for non-finite values, non-positive time or
/// gravity, invalid depths, overflow, or an unresolvable star state.
pub fn stoker_dam_break(
    x_m: f64,
    time_s: f64,
    left_depth_m: f64,
    right_depth_m: f64,
    gravity_m_s2: f64,
) -> Result<StokerState, SolverError> {
    if !x_m.is_finite()
        || !time_s.is_finite()
        || !left_depth_m.is_finite()
        || !right_depth_m.is_finite()
        || !gravity_m_s2.is_finite()
        || time_s <= 0.0
        || left_depth_m <= right_depth_m
        || right_depth_m <= 0.0
        || gravity_m_s2 <= 0.0
    {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    let initial_wave_speed = (gravity_m_s2 * left_depth_m).sqrt();
    let similarity = x_m / time_s;
    if !initial_wave_speed.is_finite() || !similarity.is_finite() {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    // For this h_left > h_right case, h_right < h_star < h_left:
    // left-going wave curve is a rarefaction and right-going wave is a shock.
    let mut lower = right_depth_m;
    let mut upper = left_depth_m;
    for _ in 0..100 {
        let middle = lower + 0.5 * (upper - lower);
        let middle_celerity = (gravity_m_s2 * middle).sqrt();
        let right_shock_curve = (middle - right_depth_m)
            * (0.5
                * gravity_m_s2
                * (1.0 / middle + 1.0 / right_depth_m))
                .sqrt();
        let residual = 2.0 * (middle_celerity - initial_wave_speed) + right_shock_curve;
        if !middle_celerity.is_finite() || !residual.is_finite() {
            return Err(SolverError::InvalidAnalyticalInput);
        }
        if residual > 0.0 {
            upper = middle;
        } else {
            lower = middle;
        }
    }

    let star_depth_m = lower + 0.5 * (upper - lower);
    let star_celerity = (gravity_m_s2 * star_depth_m).sqrt();
    let star_velocity_m_s = 2.0 * (initial_wave_speed - star_celerity);
    let shock_denominator = star_depth_m - right_depth_m;
    if !star_depth_m.is_finite()
        || !star_celerity.is_finite()
        || !star_velocity_m_s.is_finite()
        || shock_denominator <= 0.0
    {
        return Err(SolverError::InvalidAnalyticalInput);
    }
    let shock_speed = star_depth_m * star_velocity_m_s / shock_denominator;
    let rarefaction_head_speed = star_velocity_m_s - star_celerity;
    if !shock_speed.is_finite() || !rarefaction_head_speed.is_finite() {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    let (depth_m, velocity_m_s) = if similarity <= -initial_wave_speed {
        (left_depth_m, 0.0)
    } else if similarity < rarefaction_head_speed {
        let celerity = (2.0 * initial_wave_speed - similarity) / 3.0;
        (celerity * celerity / gravity_m_s2, (2.0 / 3.0) * (initial_wave_speed + similarity))
    } else if similarity < shock_speed {
        (star_depth_m, star_velocity_m_s)
    } else {
        (right_depth_m, 0.0)
    };

    let discharge_m2_s = depth_m * velocity_m_s;
    if !depth_m.is_finite() || depth_m <= 0.0 || !velocity_m_s.is_finite() || !discharge_m2_s.is_finite() {
        return Err(SolverError::InvalidAnalyticalInput);
    }

    Ok(StokerState {
        depth_m,
        velocity_m_s,
        discharge_m2_s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(dx: f64) -> SolverConfig {
        SolverConfig {
            cell_width_m: dx,
            cfl: 0.4,
            ..SolverConfig::default()
        }
    }

    #[test]
    fn stoker_solution_matches_left_plateau_star_region_and_right_plateau() {
        let h_left: f64 = 1.0;
        let h_right: f64 = 0.1;
        let gravity: f64 = 9.81;
        let time: f64 = 0.2;
        let c_left = (gravity * h_left).sqrt();

        let left = stoker_dam_break(-2.0 * c_left * time, time, h_left, h_right, gravity)
            .unwrap();
        assert!((left.depth_m - h_left).abs() < 1.0e-12);
        assert_eq!(left.velocity_m_s, 0.0);

        // x=0 falls within the constant star state for this depth ratio.
        let star = stoker_dam_break(0.0, time, h_left, h_right, gravity).unwrap();
        assert!(star.depth_m > h_right && star.depth_m < h_left);
        assert!(star.velocity_m_s > 0.0);
        assert!((star.discharge_m2_s - star.depth_m * star.velocity_m_s).abs() < 1.0e-12);

        let right = stoker_dam_break(10.0 * time, time, h_left, h_right, gravity).unwrap();
        assert!((right.depth_m - h_right).abs() < 1.0e-12);
        assert_eq!(right.velocity_m_s, 0.0);
    }

    fn dam_break_depth_rmse(dx: f64) -> f64 {
        let cfg = config(dx);
        let x_min = -5.0;
        let x_max = 5.0;
        let time_end = 0.2;
        let h_left = 1.0;
        let h_right = 0.1;
        let count = ((x_max - x_min) / dx).round() as usize;
        let cells = (0..count)
            .map(|index| {
                let x = x_min + (index as f64 + 0.5) * dx;
                CellState::still_water(if x < 0.0 { h_left } else { h_right }, 0.0)
            })
            .collect();
        let mut solver = ShallowWater1D::new(cells, cfg).unwrap();
        let mut time = 0.0;
        let mut steps = 0_usize;
        while time < time_end - 1.0e-14 {
            let dt = solver
                .advance(time_end - time, cfg, BoundaryConditions::OPEN)
                .unwrap();
            time += dt;
            steps += 1;
            assert!(steps < 100_000, "wet-bed dam-break integration did not terminate");
        }

        let squared_error = solver
            .cells()
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let x = x_min + (index as f64 + 0.5) * dx;
                let expected =
                    stoker_dam_break(x, time_end, h_left, h_right, cfg.gravity_m_s2).unwrap();
                (cell.depth_m - expected.depth_m).powi(2)
            })
            .sum::<f64>();
        let rmse = (squared_error / count as f64).sqrt();
        println!(
            "STOKER_WET_BED_DAM_BREAK dx_m={dx:.6} cells={count} steps={steps} time_s={time_end:.3} depth_rmse_m={rmse:.9}"
        );
        rmse
    }

    #[test]
    fn wet_bed_dam_break_error_decreases_across_three_grid_resolutions() {
        let coarse = dam_break_depth_rmse(0.1);
        let medium = dam_break_depth_rmse(0.05);
        let fine = dam_break_depth_rmse(0.025);

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
    fn stoker_rejects_dry_right_state_and_invalid_parameters() {
        assert_eq!(
            stoker_dam_break(0.0, 0.2, 1.0, 0.0, 9.81),
            Err(SolverError::InvalidAnalyticalInput)
        );
        assert_eq!(
            stoker_dam_break(0.0, 0.2, 0.1, 1.0, 9.81),
            Err(SolverError::InvalidAnalyticalInput)
        );
        assert_eq!(
            stoker_dam_break(f64::NAN, 0.2, 1.0, 0.1, 9.81),
            Err(SolverError::InvalidAnalyticalInput)
        );
    }
}
