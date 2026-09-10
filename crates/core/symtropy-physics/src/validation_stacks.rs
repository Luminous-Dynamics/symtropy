// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Canonical vertical-stack fixtures and outcome metrics.
//!
//! The existing three-box regression and ignored ten-box stretch test establish
//! an important experimental boundary: short stacks are a control for the
//! repaired contact-starvation path, while taller stacks expose a separate
//! coupled-solver scaling problem. This module turns that boundary into an
//! engine-neutral fixture/measurement vocabulary without declaring any solver
//! result a pass merely because it is finite.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::validation::{
    BodyObservation3d, PHYSICS_SCENARIO_SCHEMA_VERSION, PHYSICS_TRACE_SCHEMA_VERSION,
    PhysicsScenario3d, PhysicsTrace3d, ScenarioBody3d, ScenarioBodyType, ScenarioShape3d,
    ValidationBodyId,
};

/// Stable identity reserved for the canonical stack ground body.
pub const STACK_GROUND_ID: ValidationBodyId = ValidationBodyId(0);

/// Fixed v0.1 stack geometry shared by independent backends.
pub const STACK_BOX_HALF_EXTENT_M: f64 = 0.5;
/// Slight initial overlap used by the historical native regression.
pub const STACK_INITIAL_CENTER_SPACING_M: f64 = 0.99;
/// Settled target spacing for unit boxes.
pub const STACK_SETTLED_CENTER_SPACING_M: f64 = 1.0;

/// Final-state metrics that characterize a vertical stack without embedding a
/// pass/fail policy into the measurement layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StackOutcome3d {
    pub scenario_id: String,
    pub box_count: u32,
    pub vertical_order_violations: u32,
    pub max_horizontal_drift_m: f64,
    pub rms_expected_center_error_m: f64,
    pub top_height_error_m: f64,
    pub max_final_speed_mps: f64,
    pub sleeping_boxes: u32,
    pub non_finite_boxes: u32,
}

/// Structural trace failure that prevents a stack outcome from being measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StackAnalysisError {
    UnsupportedTraceSchemaVersion(u32),
    NoFinalSample,
    DuplicateBodyObservation(ValidationBodyId),
    MissingGround,
    MissingBox(ValidationBodyId),
    UnexpectedBody(ValidationBodyId),
}

impl fmt::Display for StackAnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTraceSchemaVersion(version) => {
                write!(f, "unsupported trace schema version {version}")
            }
            Self::NoFinalSample => write!(f, "stack trace has no final sample"),
            Self::DuplicateBodyObservation(id) => {
                write!(f, "duplicate stack body observation {}", id.0)
            }
            Self::MissingGround => write!(f, "stack trace is missing ground body 0"),
            Self::MissingBox(id) => write!(f, "stack trace is missing box body {}", id.0),
            Self::UnexpectedBody(id) => {
                write!(f, "stack trace contains unexpected body {}", id.0)
            }
        }
    }
}

impl std::error::Error for StackAnalysisError {}

/// Build the canonical v0.1 axis-aligned box-stack fixture.
///
/// Body `0` is a fixed ground cuboid whose top surface is y=0. Dynamic boxes
/// use IDs `1..=box_count`, unit side length, 1 kg mass, zero restitution, and
/// a 0.99 m initial center spacing to preserve the historical 0.01 m overlap.
pub fn vertical_box_stack_scenario_3d(
    box_count: u32,
    dt_s: f64,
    steps: u64,
    sample_every_steps: u64,
) -> PhysicsScenario3d {
    let mut bodies = Vec::with_capacity(box_count as usize + 1);
    bodies.push(ScenarioBody3d {
        id: STACK_GROUND_ID,
        body_type: ScenarioBodyType::Static,
        shape: ScenarioShape3d::Cuboid {
            half_extents_m: [5.0, 0.5, 5.0],
        },
        position_m: [0.0, -0.5, 0.0],
        linear_velocity_mps: [0.0; 3],
        mass_kg: 0.0,
        friction: 0.5,
        restitution: 0.0,
    });

    for index in 0..box_count {
        bodies.push(ScenarioBody3d {
            id: ValidationBodyId(index as u64 + 1),
            body_type: ScenarioBodyType::Dynamic,
            shape: ScenarioShape3d::Cuboid {
                half_extents_m: [STACK_BOX_HALF_EXTENT_M; 3],
            },
            position_m: [
                0.0,
                STACK_BOX_HALF_EXTENT_M + index as f64 * STACK_INITIAL_CENTER_SPACING_M,
                0.0,
            ],
            linear_velocity_mps: [0.0; 3],
            mass_kg: 1.0,
            friction: 0.5,
            restitution: 0.0,
        });
    }

    PhysicsScenario3d {
        schema_version: PHYSICS_SCENARIO_SCHEMA_VERSION,
        scenario_id: format!("vertical-box-stack-{box_count}-v0"),
        gravity_mps2: [0.0, -9.81, 0.0],
        dt_s,
        steps,
        sample_every_steps,
        bodies,
    }
}

/// Characterize the final state of a canonical vertical-box stack.
///
/// This function intentionally returns measurements only. Thresholds belong to
/// a declared solver profile/campaign so a known ten-box failure cannot be
/// silently converted into a passing result by choosing convenient limits.
pub fn analyze_vertical_box_stack_3d(
    trace: &PhysicsTrace3d,
    box_count: u32,
) -> Result<StackOutcome3d, StackAnalysisError> {
    if trace.schema_version != PHYSICS_TRACE_SCHEMA_VERSION {
        return Err(StackAnalysisError::UnsupportedTraceSchemaVersion(
            trace.schema_version,
        ));
    }
    let final_sample = trace.final_sample().ok_or(StackAnalysisError::NoFinalSample)?;

    let mut bodies = BTreeMap::new();
    for body in &final_sample.bodies {
        if bodies.insert(body.body_id, body).is_some() {
            return Err(StackAnalysisError::DuplicateBodyObservation(body.body_id));
        }
    }

    if !bodies.contains_key(&STACK_GROUND_ID) {
        return Err(StackAnalysisError::MissingGround);
    }
    for id in bodies.keys() {
        if *id != STACK_GROUND_ID && (id.0 == 0 || id.0 > box_count as u64) {
            return Err(StackAnalysisError::UnexpectedBody(*id));
        }
    }

    let mut ordered = Vec::with_capacity(box_count as usize);
    for index in 0..box_count {
        let id = ValidationBodyId(index as u64 + 1);
        ordered.push(
            *bodies
                .get(&id)
                .ok_or(StackAnalysisError::MissingBox(id))?,
        );
    }

    let mut vertical_order_violations = 0_u32;
    let mut max_horizontal_drift_m = 0.0_f64;
    let mut center_error_sq_sum = 0.0_f64;
    let mut max_final_speed_mps = 0.0_f64;
    let mut sleeping_boxes = 0_u32;
    let mut non_finite_boxes = 0_u32;

    for (index, body) in ordered.iter().enumerate() {
        if !body.is_finite() {
            non_finite_boxes += 1;
            continue;
        }

        let expected_y =
            STACK_BOX_HALF_EXTENT_M + index as f64 * STACK_SETTLED_CENTER_SPACING_M;
        let dx = body.position_m[0];
        let dy = body.position_m[1] - expected_y;
        let dz = body.position_m[2];
        let horizontal_drift = (dx * dx + dz * dz).sqrt();
        let center_error_sq = dx * dx + dy * dy + dz * dz;
        let speed = body
            .linear_velocity_mps
            .iter()
            .map(|component| component * component)
            .sum::<f64>()
            .sqrt();

        max_horizontal_drift_m = max_horizontal_drift_m.max(horizontal_drift);
        center_error_sq_sum += center_error_sq;
        max_final_speed_mps = max_final_speed_mps.max(speed);
        if body.sleeping {
            sleeping_boxes += 1;
        }
    }

    for pair in ordered.windows(2) {
        if pair[0].is_finite()
            && pair[1].is_finite()
            && pair[1].position_m[1] <= pair[0].position_m[1]
        {
            vertical_order_violations += 1;
        }
    }

    let rms_expected_center_error_m = if box_count > 0 {
        (center_error_sq_sum / box_count as f64).sqrt()
    } else {
        0.0
    };
    let top_height_error_m = ordered
        .last()
        .filter(|body| body.is_finite())
        .map(|body| {
            let expected = STACK_BOX_HALF_EXTENT_M
                + (box_count.saturating_sub(1)) as f64 * STACK_SETTLED_CENTER_SPACING_M;
            (body.position_m[1] - expected).abs()
        })
        .unwrap_or(0.0);

    Ok(StackOutcome3d {
        scenario_id: trace.scenario_id.clone(),
        box_count,
        vertical_order_violations,
        max_horizontal_drift_m,
        rms_expected_center_error_m,
        top_height_error_m,
        max_final_speed_mps,
        sleeping_boxes,
        non_finite_boxes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{PhysicsBackendDescriptor, PhysicsSample3d};

    fn perfect_stack_trace(box_count: u32) -> PhysicsTrace3d {
        let scenario = vertical_box_stack_scenario_3d(box_count, 1.0 / 60.0, 60, 60);
        let mut observations = Vec::new();
        observations.push(BodyObservation3d {
            body_id: STACK_GROUND_ID,
            position_m: [0.0, -0.5, 0.0],
            linear_velocity_mps: [0.0; 3],
            sleeping: true,
        });
        for index in 0..box_count {
            observations.push(BodyObservation3d {
                body_id: ValidationBodyId(index as u64 + 1),
                position_m: [
                    0.0,
                    STACK_BOX_HALF_EXTENT_M
                        + index as f64 * STACK_SETTLED_CENTER_SPACING_M,
                    0.0,
                ],
                linear_velocity_mps: [0.0; 3],
                sleeping: true,
            });
        }

        PhysicsTrace3d {
            schema_version: PHYSICS_TRACE_SCHEMA_VERSION,
            scenario_id: scenario.scenario_id,
            backend: PhysicsBackendDescriptor {
                engine: "synthetic".to_owned(),
                engine_version: "1".to_owned(),
                profile: "perfect-stack".to_owned(),
            },
            samples: vec![PhysicsSample3d {
                step: 60,
                time_s: 1.0,
                bodies: observations,
            }],
        }
    }

    #[test]
    fn canonical_stack_fixture_preserves_historical_geometry() {
        let scenario = vertical_box_stack_scenario_3d(3, 1.0 / 60.0, 1200, 1200);
        scenario.validate().unwrap();
        assert_eq!(scenario.bodies.len(), 4);
        assert_eq!(scenario.bodies[0].id, STACK_GROUND_ID);
        assert!((scenario.bodies[1].position_m[1] - 0.5).abs() < 1e-12);
        assert!((scenario.bodies[2].position_m[1] - 1.49).abs() < 1e-12);
        assert!((scenario.bodies[3].position_m[1] - 2.48).abs() < 1e-12);
    }

    #[test]
    fn perfect_stack_has_zero_geometric_error() {
        let outcome = analyze_vertical_box_stack_3d(&perfect_stack_trace(10), 10).unwrap();
        assert_eq!(outcome.box_count, 10);
        assert_eq!(outcome.vertical_order_violations, 0);
        assert_eq!(outcome.max_horizontal_drift_m, 0.0);
        assert_eq!(outcome.rms_expected_center_error_m, 0.0);
        assert_eq!(outcome.top_height_error_m, 0.0);
        assert_eq!(outcome.max_final_speed_mps, 0.0);
        assert_eq!(outcome.sleeping_boxes, 10);
        assert_eq!(outcome.non_finite_boxes, 0);
    }

    #[test]
    fn collapsed_stack_is_measured_not_relabelled_as_pass() {
        let mut trace = perfect_stack_trace(3);
        trace.samples[0].bodies[3].position_m = [1.2, 0.7, 0.0];
        trace.samples[0].bodies[3].linear_velocity_mps = [2.0, 0.0, 0.0];
        trace.samples[0].bodies[3].sleeping = false;

        let outcome = analyze_vertical_box_stack_3d(&trace, 3).unwrap();
        assert_eq!(outcome.vertical_order_violations, 1);
        assert!(outcome.max_horizontal_drift_m >= 1.2);
        assert!(outcome.top_height_error_m >= 1.8);
        assert!(outcome.max_final_speed_mps >= 2.0);
        assert_eq!(outcome.sleeping_boxes, 2);
    }

    #[test]
    fn duplicate_body_observation_is_rejected() {
        let mut trace = perfect_stack_trace(3);
        trace.samples[0].bodies.push(trace.samples[0].bodies[1].clone());
        assert_eq!(
            analyze_vertical_box_stack_3d(&trace, 3),
            Err(StackAnalysisError::DuplicateBodyObservation(
                ValidationBodyId(1)
            ))
        );
    }
}
