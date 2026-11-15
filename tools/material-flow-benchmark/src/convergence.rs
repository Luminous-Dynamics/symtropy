// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Analytical four-timestep free-fall convergence campaign.
//!
//! This validates translational integration against the closed-form solution
//! before contact. It does not qualify collision response, asymmetric inertia,
//! articulation, real packaging or robot dynamics.

use nalgebra::SVector;
use serde::Serialize;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const DT_VALUES: [f64; 4] = [1.0 / 30.0, 1.0 / 60.0, 1.0 / 120.0, 1.0 / 240.0];
const DURATION_SECONDS: f64 = 1.0;
const INITIAL_POSITION_M: f64 = 100.0;
const GRAVITY_M_S2: f64 = -9.81;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FreeFallCase {
    pub dt_seconds: f64,
    pub steps: usize,
    pub duration_seconds: f64,
    pub observed_position_y_m: f64,
    pub analytical_position_y_m: f64,
    pub absolute_position_error_m: f64,
    pub observed_velocity_y_m_s: f64,
    pub analytical_velocity_y_m_s: f64,
    pub absolute_velocity_error_m_s: f64,
    pub relative_energy_drift: f64,
    pub max_rotation_orthogonality_error: f64,
    pub max_rotation_determinant_error: f64,
    pub non_finite_body_count: usize,
    pub numerically_healthy: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FreeFallConvergenceReport {
    pub schema_version: u16,
    pub engine: String,
    pub integration_method: String,
    pub feature_contract: String,
    pub gravity_m_s2: f64,
    pub initial_position_y_m: f64,
    pub requested_duration_seconds: f64,
    pub position_error_monotonic: bool,
    pub estimated_position_order: Option<f64>,
    pub all_cases_numerically_healthy: bool,
    pub cases: Vec<FreeFallCase>,
    pub limitations: Vec<String>,
}

fn run_case(dt_seconds: f64) -> Result<FreeFallCase, String> {
    let steps_float = DURATION_SECONDS / dt_seconds;
    let steps = steps_float.round() as usize;
    if steps == 0 || (steps as f64 * dt_seconds - DURATION_SECONDS).abs() > 1e-12 {
        return Err(format!(
            "timestep {dt_seconds} does not divide the requested physical duration"
        ));
    }

    let gravity = SVector::from([0.0, GRAVITY_M_S2, 0.0]);
    let mut world = PhysicsWorld::<3>::new(gravity);
    let handle = world.add_sphere(Point::new([0.0, INITIAL_POSITION_M, 0.0]), 0.5, 1.0);
    {
        let body = world
            .body_mut(handle)
            .ok_or_else(|| "free-fall body disappeared after insertion".to_string())?;
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
    }

    let before = world.invariant_snapshot();
    for _ in 0..steps {
        world.step(dt_seconds);
    }
    let after = world.invariant_snapshot();
    let drift = before.drift_to(&after);
    let body = world
        .body(handle)
        .ok_or_else(|| "free-fall body disappeared after integration".to_string())?;

    let duration = steps as f64 * dt_seconds;
    let analytical_position = INITIAL_POSITION_M + 0.5 * GRAVITY_M_S2 * duration * duration;
    let analytical_velocity = GRAVITY_M_S2 * duration;
    let observed_position = body.transform.translation.coord(1);
    let observed_velocity = body.linear_velocity[1];
    let healthy = after.is_numerically_healthy(1e-8);

    Ok(FreeFallCase {
        dt_seconds,
        steps,
        duration_seconds: duration,
        observed_position_y_m: observed_position,
        analytical_position_y_m: analytical_position,
        absolute_position_error_m: (observed_position - analytical_position).abs(),
        observed_velocity_y_m_s: observed_velocity,
        analytical_velocity_y_m_s: analytical_velocity,
        absolute_velocity_error_m_s: (observed_velocity - analytical_velocity).abs(),
        relative_energy_drift: drift.relative_energy_drift,
        max_rotation_orthogonality_error: after.max_rotation_orthogonality_error,
        max_rotation_determinant_error: after.max_rotation_determinant_error,
        non_finite_body_count: after.non_finite_body_count,
        numerically_healthy: healthy,
    })
}

/// Run the same one-second free-fall initial condition at four fixed timesteps.
pub fn run_free_fall_convergence() -> Result<FreeFallConvergenceReport, String> {
    let mut cases = Vec::with_capacity(DT_VALUES.len());
    for dt in DT_VALUES {
        cases.push(run_case(dt)?);
    }

    let position_error_monotonic = cases
        .windows(2)
        .all(|pair| pair[1].absolute_position_error_m < pair[0].absolute_position_error_m);

    let orders = cases
        .windows(2)
        .filter_map(|pair| {
            let coarse = pair[0].absolute_position_error_m;
            let fine = pair[1].absolute_position_error_m;
            if coarse > 0.0 && fine > 0.0 {
                Some((coarse / fine).log2())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let estimated_position_order = if orders.is_empty() {
        None
    } else {
        Some(orders.iter().sum::<f64>() / orders.len() as f64)
    };

    let all_cases_numerically_healthy = cases.iter().all(|case| {
        case.numerically_healthy
            && case.non_finite_body_count == 0
            && case.absolute_velocity_error_m_s <= 1e-3
    });

    Ok(FreeFallConvergenceReport {
        schema_version: 1,
        engine: "symtropy-physics 0.2.1".into(),
        integration_method: "semi_implicit_euler".into(),
        feature_contract: "deterministic-net Q16.16".into(),
        gravity_m_s2: GRAVITY_M_S2,
        initial_position_y_m: INITIAL_POSITION_M,
        requested_duration_seconds: DURATION_SECONDS,
        position_error_monotonic,
        estimated_position_order,
        all_cases_numerically_healthy,
        cases,
        limitations: vec![
            "Analytical translational free fall only; no contacts, packaging, rotation or articulated dynamics".into(),
            "Q16.16 deterministic-net quantization is part of the measured numerical method".into(),
            "Same-build same-host measurements; cross-architecture numerical equivalence is not claimed".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_timestep_free_fall_error_decreases_monotonically() {
        let report = run_free_fall_convergence().expect("convergence campaign should run");
        assert_eq!(report.cases.len(), 4);
        assert!(report.position_error_monotonic, "{report:?}");
        assert!(report.all_cases_numerically_healthy, "{report:?}");
        assert!(
            report
                .estimated_position_order
                .is_some_and(|order| order > 0.8 && order < 1.2)
        );
    }

    #[test]
    fn constant_acceleration_velocity_matches_closed_form() {
        let report = run_free_fall_convergence().expect("convergence campaign should run");
        assert!(
            report
                .cases
                .iter()
                .all(|case| case.absolute_velocity_error_m_s <= 1e-3),
            "{report:?}"
        );
    }

    #[test]
    fn convergence_campaign_uses_identical_physical_duration() {
        let report = run_free_fall_convergence().expect("convergence campaign should run");
        assert!(
            report.cases.iter().all(|case| {
                (case.duration_seconds - report.requested_duration_seconds).abs() <= 1e-12
            }),
            "{report:?}"
        );
    }
}
