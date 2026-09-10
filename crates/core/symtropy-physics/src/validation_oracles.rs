// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Independent analytical oracles for the physics validation corpus.
//!
//! These helpers are intentionally narrower than the simulation API. An
//! analytical solution is only useful evidence when its assumptions are made
//! explicit and enforced. The initial oracle therefore accepts exactly one
//! dynamic body under uniform acceleration with no contacts, joints, callbacks,
//! or other forces.

use std::fmt;

use crate::validation::{
    BodyObservation3d, PHYSICS_TRACE_SCHEMA_VERSION, PhysicsBackendDescriptor, PhysicsSample3d,
    PhysicsScenario3d, PhysicsTrace3d, ScenarioBodyType, ScenarioValidationError,
};

/// Failure to construct an analytical trace without violating its assumptions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalyticalOracleError {
    Scenario(ScenarioValidationError),
    RequiresExactlyOneBody,
    RequiresDynamicBody,
}

impl fmt::Display for AnalyticalOracleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scenario(error) => write!(f, "invalid scenario: {error}"),
            Self::RequiresExactlyOneBody => {
                write!(f, "uniform-acceleration oracle requires exactly one body")
            }
            Self::RequiresDynamicBody => {
                write!(f, "uniform-acceleration oracle requires a dynamic body")
            }
        }
    }
}

impl std::error::Error for AnalyticalOracleError {}

impl From<ScenarioValidationError> for AnalyticalOracleError {
    fn from(error: ScenarioValidationError) -> Self {
        Self::Scenario(error)
    }
}

/// Construct the exact continuous-time trace for one non-interacting body under
/// constant uniform acceleration.
///
/// For every sampled time `t`:
///
/// - `x(t) = x0 + v0*t + 0.5*a*t^2`
/// - `v(t) = v0 + a*t`
///
/// The returned trace uses the same deterministic sampling schedule as the
/// native scenario runner. It does not model collision, sleeping, rotation, or
/// damping and therefore rejects multi-body/static fixtures rather than
/// pretending the closed-form solution still applies.
pub fn uniform_acceleration_trace_3d(
    scenario: &PhysicsScenario3d,
) -> Result<PhysicsTrace3d, AnalyticalOracleError> {
    let scenario = scenario.normalized()?;
    if scenario.bodies.len() != 1 {
        return Err(AnalyticalOracleError::RequiresExactlyOneBody);
    }
    let body = &scenario.bodies[0];
    if body.body_type != ScenarioBodyType::Dynamic {
        return Err(AnalyticalOracleError::RequiresDynamicBody);
    }

    let mut samples = Vec::new();
    for step in 0..=scenario.steps {
        if step != 0
            && step != scenario.steps
            && !step.is_multiple_of(scenario.sample_every_steps)
        {
            continue;
        }

        let time_s = scenario.dt_s * step as f64;
        let mut position_m = [0.0; 3];
        let mut linear_velocity_mps = [0.0; 3];
        for axis in 0..3 {
            position_m[axis] = body.position_m[axis]
                + body.linear_velocity_mps[axis] * time_s
                + 0.5 * scenario.gravity_mps2[axis] * time_s * time_s;
            linear_velocity_mps[axis] =
                body.linear_velocity_mps[axis] + scenario.gravity_mps2[axis] * time_s;
        }

        samples.push(PhysicsSample3d {
            step,
            time_s,
            bodies: vec![BodyObservation3d {
                body_id: body.id,
                position_m,
                linear_velocity_mps,
                sleeping: false,
            }],
        });
    }

    Ok(PhysicsTrace3d {
        schema_version: PHYSICS_TRACE_SCHEMA_VERSION,
        scenario_id: scenario.scenario_id,
        backend: PhysicsBackendDescriptor {
            engine: "analytical-uniform-acceleration".to_owned(),
            engine_version: "closed-form-v1".to_owned(),
            profile: "single-body-no-contact-v0.1".to_owned(),
        },
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{
        PHYSICS_SCENARIO_SCHEMA_VERSION, ScenarioBody3d, ScenarioShape3d, ValidationBodyId,
        compare_traces_3d, run_native_scenario_3d,
    };

    fn free_fall_scenario(hz: u64) -> PhysicsScenario3d {
        PhysicsScenario3d {
            schema_version: PHYSICS_SCENARIO_SCHEMA_VERSION,
            scenario_id: format!("analytical-free-fall-{hz}hz"),
            gravity_mps2: [0.0, -9.81, 0.0],
            dt_s: 1.0 / hz as f64,
            steps: hz,
            sample_every_steps: hz,
            bodies: vec![ScenarioBody3d {
                id: ValidationBodyId(1),
                body_type: ScenarioBodyType::Dynamic,
                shape: ScenarioShape3d::Sphere { radius_m: 0.5 },
                position_m: [0.0, 10.0, 0.0],
                linear_velocity_mps: [1.0, 0.0, -0.5],
                mass_kg: 1.0,
                friction: 0.5,
                restitution: 0.0,
            }],
        }
    }

    #[test]
    fn closed_form_oracle_matches_known_one_second_solution() {
        let scenario = free_fall_scenario(60);
        let trace = uniform_acceleration_trace_3d(&scenario).unwrap();
        let final_body = &trace.final_sample().unwrap().bodies[0];

        assert!((final_body.position_m[0] - 1.0).abs() < 1e-12);
        assert!((final_body.position_m[1] - 5.095).abs() < 1e-12);
        assert!((final_body.position_m[2] + 0.5).abs() < 1e-12);
        assert!((final_body.linear_velocity_mps[0] - 1.0).abs() < 1e-12);
        assert!((final_body.linear_velocity_mps[1] + 9.81).abs() < 1e-12);
        assert!((final_body.linear_velocity_mps[2] + 0.5).abs() < 1e-12);
    }

    #[test]
    fn oracle_rejects_multi_body_fixture() {
        let mut scenario = free_fall_scenario(60);
        scenario.bodies.push(scenario.bodies[0].clone());
        scenario.bodies[1].id = ValidationBodyId(2);
        assert_eq!(
            uniform_acceleration_trace_3d(&scenario),
            Err(AnalyticalOracleError::RequiresExactlyOneBody)
        );
    }

    #[test]
    fn native_free_fall_position_error_converges_first_order() {
        let mut errors = Vec::new();
        for hz in [30_u64, 60, 120, 240] {
            let scenario = free_fall_scenario(hz);
            let native = run_native_scenario_3d(&scenario).unwrap();
            let exact = uniform_acceleration_trace_3d(&scenario).unwrap();
            let metrics = compare_traces_3d(&native, &exact).unwrap();

            assert_eq!(metrics.left_non_finite_observations, 0);
            assert_eq!(metrics.right_non_finite_observations, 0);
            assert!(metrics.max_position_error_m.is_finite());
            assert!(metrics.max_linear_velocity_error_mps < 1e-10);
            errors.push(metrics.max_position_error_m);
        }

        for window in errors.windows(2) {
            assert!(
                window[1] < window[0],
                "position error must decrease as dt decreases: {errors:?}"
            );
            let ratio = window[0] / window[1];
            assert!(
                (ratio - 2.0).abs() < 0.05,
                "semi-implicit Euler free-fall position error should converge first-order; ratio={ratio}, errors={errors:?}"
            );
        }
    }
}
