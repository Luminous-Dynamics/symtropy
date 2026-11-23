// SPDX-License-Identifier: AGPL-3.0-or-later
//! Common-space comparison for full and reduced contact-solver responses.
//!
//! This crate does not solve contact dynamics. It compares solver snapshots
//! using body-level generalized responses and shared full-patch contact probes.
//! Raw per-contact multipliers are intentionally not the primary metric because
//! different candidate sets can have different multiplier dimensions.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "solver-response-oracle-v0.1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseError {
    pub reason: String,
}

impl ResponseError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Display for ResponseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for ResponseError {}

#[derive(Clone, Debug, PartialEq)]
pub struct BodyResponse {
    pub body_id: String,
    /// Solver-produced centre-of-mass linear velocity change, m/s.
    pub delta_linear_velocity: [f64; 3],
    /// Solver-produced angular velocity change, rad/s.
    pub delta_angular_velocity: [f64; 3],
    /// Net generalized linear impulse, N·s.
    pub linear_impulse: [f64; 3],
    /// Net generalized angular impulse about the declared COM, N·m·s.
    pub angular_impulse: [f64; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SolverTermination {
    Converged,
    IterationLimit,
    WorkBudgetExhausted,
    Diverged,
    Aborted,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolverResponseSnapshot {
    pub scenario_digest: String,
    pub initial_state_digest: String,
    pub physics_model_digest: String,
    pub solver_config_digest: String,
    pub work_unit_schema_digest: String,
    /// Maximum work allowed for both paired runs.
    pub work_budget_units: u64,
    /// Work actually consumed; the two runs need not consume the same amount.
    pub work_consumed_units: u64,
    pub solver_iterations: u64,
    pub solver_rows_submitted: u64,
    pub solver_termination: SolverTermination,
    /// Candidate-set and warm-start digests may differ between policies.
    pub candidate_set_digest: String,
    pub warm_start_digest: String,
    pub bodies: Vec<BodyResponse>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactProbe {
    pub contact_id: String,
    pub body_a_id: String,
    pub body_b_id: String,
    /// World-space lever arm from body A COM to the common probe point, metres.
    pub lever_arm_a: [f64; 3],
    /// World-space lever arm from body B COM to the same probe point, metres.
    pub lever_arm_b: [f64; 3],
    /// World-space contact normal; normalized by the oracle.
    pub normal: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactProbeSet {
    /// Digest of the canonical common full-patch probe geometry and IDs.
    pub digest: String,
    pub probes: Vec<ContactProbe>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResponseScales {
    pub characteristic_length: f64,
    pub velocity_scale: f64,
    pub impulse_scale: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResponseComparison {
    pub scenario_digest: String,
    pub initial_state_digest: String,
    pub physics_model_digest: String,
    pub solver_config_digest: String,
    pub work_unit_schema_digest: String,
    pub work_budget_units: u64,
    pub probe_set_digest: String,
    pub characteristic_length: f64,
    pub velocity_scale: f64,
    pub impulse_scale: f64,
    pub full_work_consumed_units: u64,
    pub reduced_work_consumed_units: u64,
    pub full_solver_iterations: u64,
    pub reduced_solver_iterations: u64,
    pub full_solver_rows_submitted: u64,
    pub reduced_solver_rows_submitted: u64,
    pub full_solver_termination: SolverTermination,
    pub reduced_solver_termination: SolverTermination,
    pub both_solvers_converged: bool,
    pub full_candidate_set_digest: String,
    pub reduced_candidate_set_digest: String,
    pub full_warm_start_digest: String,
    pub reduced_warm_start_digest: String,
    pub body_count: usize,
    pub contact_probe_count: usize,
    /// Max over bodies of sqrt(||delta-v||² + ||L delta-omega||²) / velocity_scale.
    pub maximum_generalized_velocity_error: f64,
    /// Max over bodies of sqrt(||delta-p||² + ||delta-angular-impulse/L||²) / impulse_scale.
    pub maximum_generalized_impulse_error: f64,
    /// Max absolute normal component of the common-probe relative velocity error / velocity_scale.
    pub maximum_contact_normal_velocity_error: f64,
    /// Max tangent-plane norm of the common-probe relative velocity error / velocity_scale.
    pub maximum_contact_tangent_velocity_error: f64,
    /// RMS common-probe relative velocity error / velocity_scale.
    pub rms_contact_velocity_error: f64,
    pub generalized_velocity_witness_body: String,
    pub generalized_impulse_witness_body: String,
    pub normal_velocity_witness_contact: String,
    pub tangent_velocity_witness_contact: String,
}

type BodyIndex<'a> = BTreeMap<&'a str, &'a BodyResponse>;
type ValidatedProbe = ([f64; 3], [f64; 3], [f64; 3]);

fn finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn norm(value: [f64; 3]) -> f64 {
    value[0].hypot(value[1]).hypot(value[2])
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn multiply(value: [f64; 3], scalar: f64) -> [f64; 3] {
    [value[0] * scalar, value[1] * scalar, value[2] * scalar]
}

fn body_index(snapshot: &SolverResponseSnapshot) -> Result<BodyIndex<'_>, ResponseError> {
    for (label, digest) in [
        ("scenario", &snapshot.scenario_digest),
        ("initial state", &snapshot.initial_state_digest),
        ("physics model", &snapshot.physics_model_digest),
        ("solver config", &snapshot.solver_config_digest),
        ("work unit schema", &snapshot.work_unit_schema_digest),
        ("candidate set", &snapshot.candidate_set_digest),
        ("warm start", &snapshot.warm_start_digest),
    ] {
        if digest.trim().is_empty() {
            return Err(ResponseError::new(format!(
                "{label} digest must be non-empty"
            )));
        }
    }
    if snapshot.bodies.is_empty() {
        return Err(ResponseError::new(
            "solver snapshot must contain at least one body",
        ));
    }

    let mut bodies = BTreeMap::new();
    for body in &snapshot.bodies {
        if body.body_id.trim().is_empty() || bodies.insert(body.body_id.as_str(), body).is_some() {
            return Err(ResponseError::new("body IDs must be non-empty and unique"));
        }
        if !finite(&body.delta_linear_velocity)
            || !finite(&body.delta_angular_velocity)
            || !finite(&body.linear_impulse)
            || !finite(&body.angular_impulse)
        {
            return Err(ResponseError::new("body response values must be finite"));
        }
    }
    Ok(bodies)
}

fn validate_pair<'a, 'b>(
    full: &'a SolverResponseSnapshot,
    reduced: &'b SolverResponseSnapshot,
) -> Result<(BodyIndex<'a>, BodyIndex<'b>), ResponseError> {
    if full.scenario_digest != reduced.scenario_digest {
        return Err(ResponseError::new("scenario digests differ"));
    }
    if full.initial_state_digest != reduced.initial_state_digest {
        return Err(ResponseError::new("initial-state digests differ"));
    }
    if full.physics_model_digest != reduced.physics_model_digest {
        return Err(ResponseError::new("physics-model digests differ"));
    }
    if full.solver_config_digest != reduced.solver_config_digest {
        return Err(ResponseError::new("solver configuration digests differ"));
    }
    if full.work_unit_schema_digest != reduced.work_unit_schema_digest {
        return Err(ResponseError::new("work-unit schema digests differ"));
    }
    if full.work_budget_units != reduced.work_budget_units {
        return Err(ResponseError::new(
            "equal-work comparison requires matching work budgets",
        ));
    }
    if full.work_consumed_units > full.work_budget_units
        || reduced.work_consumed_units > reduced.work_budget_units
    {
        return Err(ResponseError::new(
            "actual work consumption exceeds its declared budget",
        ));
    }

    let full_bodies = body_index(full)?;
    let reduced_bodies = body_index(reduced)?;
    if full_bodies.len() != reduced_bodies.len() || !full_bodies.keys().eq(reduced_bodies.keys()) {
        return Err(ResponseError::new(
            "full and reduced snapshots must contain the same body IDs",
        ));
    }
    Ok((full_bodies, reduced_bodies))
}

fn validate_probes(
    probe_set: &ContactProbeSet,
    bodies: &BodyIndex<'_>,
) -> Result<Vec<ValidatedProbe>, ResponseError> {
    if probe_set.digest.trim().is_empty() {
        return Err(ResponseError::new("probe-set digest must be non-empty"));
    }
    if probe_set.probes.is_empty() {
        return Err(ResponseError::new(
            "at least one common full-patch contact probe is required",
        ));
    }

    let mut ids = BTreeSet::new();
    let mut validated = Vec::with_capacity(probe_set.probes.len());
    for probe in &probe_set.probes {
        if probe.contact_id.trim().is_empty() || !ids.insert(probe.contact_id.as_str()) {
            return Err(ResponseError::new(
                "contact probe IDs must be non-empty and unique",
            ));
        }
        if probe.body_a_id == probe.body_b_id
            || !bodies.contains_key(probe.body_a_id.as_str())
            || !bodies.contains_key(probe.body_b_id.as_str())
        {
            return Err(ResponseError::new(
                "each probe must reference two distinct bodies present in both snapshots",
            ));
        }
        if !finite(&probe.lever_arm_a) || !finite(&probe.lever_arm_b) || !finite(&probe.normal) {
            return Err(ResponseError::new("contact probe geometry must be finite"));
        }
        let normal_norm = norm(probe.normal);
        if !normal_norm.is_finite() || normal_norm <= 0.0 {
            return Err(ResponseError::new("contact probe normal must be non-zero"));
        }
        validated.push((
            probe.lever_arm_a,
            probe.lever_arm_b,
            multiply(probe.normal, normal_norm.recip()),
        ));
    }
    Ok(validated)
}

fn point_delta_velocity(body: &BodyResponse, lever_arm: [f64; 3]) -> [f64; 3] {
    add(
        body.delta_linear_velocity,
        cross(body.delta_angular_velocity, lever_arm),
    )
}

fn update_maximum(current: f64, current_witness: &str, value: f64, witness: &str) -> (f64, String) {
    if value > current
        || (value == current && (current_witness.is_empty() || witness < current_witness))
    {
        (value, witness.to_owned())
    } else {
        (current, current_witness.to_owned())
    }
}

/// Compare two snapshots using identical common-patch probes.
///
/// The candidate-set and warm-start digests are recorded but may differ. Scenario,
/// state, model, solver configuration, work-unit schema and budget must match.
pub fn compare_responses(
    full: &SolverResponseSnapshot,
    reduced: &SolverResponseSnapshot,
    probe_set: &ContactProbeSet,
    scales: ResponseScales,
) -> Result<ResponseComparison, ResponseError> {
    if !scales.characteristic_length.is_finite()
        || scales.characteristic_length <= 0.0
        || !scales.velocity_scale.is_finite()
        || scales.velocity_scale <= 0.0
        || !scales.impulse_scale.is_finite()
        || scales.impulse_scale <= 0.0
    {
        return Err(ResponseError::new(
            "response scales must be finite and strictly positive",
        ));
    }

    let (full_bodies, reduced_bodies) = validate_pair(full, reduced)?;
    let validated_probes = validate_probes(probe_set, &full_bodies)?;

    let mut max_velocity_error = 0.0_f64;
    let mut velocity_witness = String::new();
    let mut max_impulse_error = 0.0_f64;
    let mut impulse_witness = String::new();

    for (body_id, full_body) in &full_bodies {
        let reduced_body = reduced_bodies
            .get(body_id)
            .copied()
            .ok_or_else(|| ResponseError::new("body ID disappeared during comparison"))?;

        let linear_velocity_error = norm(subtract(
            full_body.delta_linear_velocity,
            reduced_body.delta_linear_velocity,
        ));
        let angular_velocity_error = norm(subtract(
            full_body.delta_angular_velocity,
            reduced_body.delta_angular_velocity,
        )) * scales.characteristic_length;
        let body_velocity_error =
            linear_velocity_error.hypot(angular_velocity_error) / scales.velocity_scale;
        if !body_velocity_error.is_finite() {
            return Err(ResponseError::new(
                "generalized velocity response error is non-finite",
            ));
        }
        (max_velocity_error, velocity_witness) = update_maximum(
            max_velocity_error,
            &velocity_witness,
            body_velocity_error,
            body_id,
        );

        let linear_impulse_error = norm(subtract(
            full_body.linear_impulse,
            reduced_body.linear_impulse,
        ));
        let angular_impulse_error = norm(subtract(
            full_body.angular_impulse,
            reduced_body.angular_impulse,
        )) / scales.characteristic_length;
        let body_impulse_error =
            linear_impulse_error.hypot(angular_impulse_error) / scales.impulse_scale;
        if !body_impulse_error.is_finite() {
            return Err(ResponseError::new(
                "generalized impulse response error is non-finite",
            ));
        }
        (max_impulse_error, impulse_witness) = update_maximum(
            max_impulse_error,
            &impulse_witness,
            body_impulse_error,
            body_id,
        );
    }

    let mut max_normal_error = 0.0_f64;
    let mut normal_witness = String::new();
    let mut max_tangent_error = 0.0_f64;
    let mut tangent_witness = String::new();
    let mut rms_accumulator = 0.0_f64;

    for (probe, (lever_a, lever_b, normal)) in probe_set.probes.iter().zip(&validated_probes) {
        let full_a = full_bodies[probe.body_a_id.as_str()];
        let full_b = full_bodies[probe.body_b_id.as_str()];
        let reduced_a = reduced_bodies[probe.body_a_id.as_str()];
        let reduced_b = reduced_bodies[probe.body_b_id.as_str()];

        let full_relative = subtract(
            point_delta_velocity(full_a, *lever_a),
            point_delta_velocity(full_b, *lever_b),
        );
        let reduced_relative = subtract(
            point_delta_velocity(reduced_a, *lever_a),
            point_delta_velocity(reduced_b, *lever_b),
        );
        let difference = subtract(full_relative, reduced_relative);

        let normal_component = dot(difference, *normal);
        let tangent_component = subtract(difference, multiply(*normal, normal_component));
        let normal_error = normal_component.abs() / scales.velocity_scale;
        let tangent_error = norm(tangent_component) / scales.velocity_scale;
        let combined_error = norm(difference) / scales.velocity_scale;
        if !normal_error.is_finite() || !tangent_error.is_finite() || !combined_error.is_finite() {
            return Err(ResponseError::new(
                "contact-point velocity response error is non-finite",
            ));
        }

        (max_normal_error, normal_witness) = update_maximum(
            max_normal_error,
            &normal_witness,
            normal_error,
            &probe.contact_id,
        );
        (max_tangent_error, tangent_witness) = update_maximum(
            max_tangent_error,
            &tangent_witness,
            tangent_error,
            &probe.contact_id,
        );
        rms_accumulator = rms_accumulator.hypot(combined_error);
    }

    let rms_contact_velocity_error = rms_accumulator / (probe_set.probes.len() as f64).sqrt();
    if !rms_contact_velocity_error.is_finite() {
        return Err(ResponseError::new(
            "RMS contact response error is non-finite",
        ));
    }

    Ok(ResponseComparison {
        scenario_digest: full.scenario_digest.clone(),
        initial_state_digest: full.initial_state_digest.clone(),
        physics_model_digest: full.physics_model_digest.clone(),
        solver_config_digest: full.solver_config_digest.clone(),
        work_unit_schema_digest: full.work_unit_schema_digest.clone(),
        work_budget_units: full.work_budget_units,
        probe_set_digest: probe_set.digest.clone(),
        characteristic_length: scales.characteristic_length,
        velocity_scale: scales.velocity_scale,
        impulse_scale: scales.impulse_scale,
        full_work_consumed_units: full.work_consumed_units,
        reduced_work_consumed_units: reduced.work_consumed_units,
        full_solver_iterations: full.solver_iterations,
        reduced_solver_iterations: reduced.solver_iterations,
        full_solver_rows_submitted: full.solver_rows_submitted,
        reduced_solver_rows_submitted: reduced.solver_rows_submitted,
        full_solver_termination: full.solver_termination,
        reduced_solver_termination: reduced.solver_termination,
        both_solvers_converged: full.solver_termination == SolverTermination::Converged
            && reduced.solver_termination == SolverTermination::Converged,
        full_candidate_set_digest: full.candidate_set_digest.clone(),
        reduced_candidate_set_digest: reduced.candidate_set_digest.clone(),
        full_warm_start_digest: full.warm_start_digest.clone(),
        reduced_warm_start_digest: reduced.warm_start_digest.clone(),
        body_count: full_bodies.len(),
        contact_probe_count: probe_set.probes.len(),
        maximum_generalized_velocity_error: max_velocity_error,
        maximum_generalized_impulse_error: max_impulse_error,
        maximum_contact_normal_velocity_error: max_normal_error,
        maximum_contact_tangent_velocity_error: max_tangent_error,
        rms_contact_velocity_error,
        generalized_velocity_witness_body: velocity_witness,
        generalized_impulse_witness_body: impulse_witness,
        normal_velocity_witness_contact: normal_witness,
        tangent_velocity_witness_contact: tangent_witness,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(id: &str, linear: [f64; 3], angular: [f64; 3], impulse: [f64; 3]) -> BodyResponse {
        BodyResponse {
            body_id: id.to_owned(),
            delta_linear_velocity: linear,
            delta_angular_velocity: angular,
            linear_impulse: impulse,
            angular_impulse: [0.0; 3],
        }
    }

    fn snapshot(candidate: &str, bodies: Vec<BodyResponse>) -> SolverResponseSnapshot {
        SolverResponseSnapshot {
            scenario_digest: "scenario-001".to_owned(),
            initial_state_digest: "state-001".to_owned(),
            physics_model_digest: "model-001".to_owned(),
            solver_config_digest: "solver-001".to_owned(),
            work_unit_schema_digest: "constraint-row-visits-v1".to_owned(),
            work_budget_units: 128,
            work_consumed_units: 100,
            solver_iterations: 10,
            solver_rows_submitted: 4,
            solver_termination: SolverTermination::Converged,
            candidate_set_digest: candidate.to_owned(),
            warm_start_digest: format!("warm-{candidate}"),
            bodies,
        }
    }

    fn probe(normal: [f64; 3]) -> ContactProbe {
        ContactProbe {
            contact_id: "contact-001".to_owned(),
            body_a_id: "a".to_owned(),
            body_b_id: "b".to_owned(),
            lever_arm_a: [1.0, 0.0, 0.0],
            lever_arm_b: [-1.0, 0.0, 0.0],
            normal,
        }
    }

    fn probe_set(probes: Vec<ContactProbe>) -> ContactProbeSet {
        ContactProbeSet {
            digest: "test-common-full-patch-probe-set-001".to_owned(),
            probes,
        }
    }

    fn scales() -> ResponseScales {
        ResponseScales {
            characteristic_length: 1.0,
            velocity_scale: 1.0,
            impulse_scale: 1.0,
        }
    }

    fn zero_pair() -> (SolverResponseSnapshot, SolverResponseSnapshot) {
        let bodies = vec![
            body("a", [0.0; 3], [0.0; 3], [0.0; 3]),
            body("b", [0.0; 3], [0.0; 3], [0.0; 3]),
        ];
        (
            snapshot("full", bodies.clone()),
            snapshot("reduced", bodies),
        )
    }

    #[test]
    fn identical_snapshots_have_zero_error() {
        let bodies = vec![
            body("a", [0.1, 0.2, 0.0], [0.0, 0.0, 0.1], [1.0, 2.0, 0.0]),
            body("b", [-0.1, 0.0, 0.0], [0.0; 3], [-1.0, -2.0, 0.0]),
        ];
        let full = snapshot("full", bodies.clone());
        let reduced = snapshot("reduced", bodies);
        let result = compare_responses(
            &full,
            &reduced,
            &probe_set(vec![probe([0.0, 1.0, 0.0])]),
            scales(),
        )
        .expect("valid comparison");
        assert_eq!(result.maximum_generalized_velocity_error, 0.0);
        assert_eq!(result.maximum_generalized_impulse_error, 0.0);
        assert_eq!(
            result.probe_set_digest,
            "test-common-full-patch-probe-set-001"
        );
        assert_eq!(result.characteristic_length, 1.0);
        assert_eq!(result.velocity_scale, 1.0);
        assert_eq!(result.impulse_scale, 1.0);
        assert_eq!(result.work_unit_schema_digest, full.work_unit_schema_digest);
        assert_eq!(result.maximum_contact_normal_velocity_error, 0.0);
        assert_eq!(result.maximum_contact_tangent_velocity_error, 0.0);
        assert_eq!(result.rms_contact_velocity_error, 0.0);
    }

    #[test]
    fn linear_response_is_detected_at_shared_contact_probe() {
        let full = snapshot(
            "full",
            vec![
                body("a", [0.0, 1.0, 0.0], [0.0; 3], [0.0; 3]),
                body("b", [0.0; 3], [0.0; 3], [0.0; 3]),
            ],
        );
        let (mut reduced, _) = zero_pair();
        reduced.candidate_set_digest = "reduced".to_owned();
        let result = compare_responses(
            &full,
            &reduced,
            &probe_set(vec![probe([0.0, 2.0, 0.0])]),
            scales(),
        )
        .expect("valid comparison");
        assert!((result.maximum_contact_normal_velocity_error - 1.0).abs() < 1.0e-12);
        assert_eq!(result.maximum_contact_tangent_velocity_error, 0.0);
    }

    #[test]
    fn angular_response_is_measured_at_contact_lever_arm() {
        let full = snapshot(
            "full",
            vec![
                body("a", [0.0; 3], [0.0, 0.0, 1.0], [0.0; 3]),
                body("b", [0.0; 3], [0.0; 3], [0.0; 3]),
            ],
        );
        let (reduced, _) = zero_pair();
        let result = compare_responses(
            &full,
            &reduced,
            &probe_set(vec![probe([0.0, 1.0, 0.0])]),
            scales(),
        )
        .expect("valid comparison");
        assert!((result.maximum_contact_normal_velocity_error - 1.0).abs() < 1.0e-12);
        assert!((result.maximum_generalized_velocity_error - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn body_and_probe_order_do_not_change_metrics() {
        let full = snapshot(
            "full",
            vec![
                body("a", [0.0, 1.0, 0.0], [0.0, 0.0, 0.2], [1.0, 0.0, 0.0]),
                body("b", [0.0; 3], [0.0; 3], [0.0; 3]),
            ],
        );
        let reduced = snapshot(
            "reduced",
            vec![
                body("a", [0.0, 0.5, 0.0], [0.0; 3], [0.5, 0.0, 0.0]),
                body("b", [0.0; 3], [0.0; 3], [0.0; 3]),
            ],
        );
        let probes = probe_set(vec![
            probe([0.0, 1.0, 0.0]),
            ContactProbe {
                contact_id: "contact-002".to_owned(),
                body_a_id: "a".to_owned(),
                body_b_id: "b".to_owned(),
                lever_arm_a: [0.0, 0.0, 1.0],
                lever_arm_b: [0.0, 0.0, -1.0],
                normal: [1.0, 0.0, 0.0],
            },
        ]);
        let first = compare_responses(&full, &reduced, &probes, scales()).expect("first");
        let mut reversed_full = full.clone();
        reversed_full.bodies.reverse();
        let mut reversed_reduced = reduced.clone();
        reversed_reduced.bodies.reverse();
        let reversed_probes =
            probe_set(probes.probes.clone().into_iter().rev().collect::<Vec<_>>());
        let second = compare_responses(
            &reversed_full,
            &reversed_reduced,
            &reversed_probes,
            scales(),
        )
        .expect("second");
        assert_eq!(
            first.maximum_generalized_velocity_error,
            second.maximum_generalized_velocity_error
        );
        assert_eq!(
            first.maximum_generalized_impulse_error,
            second.maximum_generalized_impulse_error
        );
        assert_eq!(
            first.maximum_contact_normal_velocity_error,
            second.maximum_contact_normal_velocity_error
        );
        assert_eq!(
            first.maximum_contact_tangent_velocity_error,
            second.maximum_contact_tangent_velocity_error
        );
        assert!(
            (first.rms_contact_velocity_error - second.rms_contact_velocity_error).abs() < 1.0e-12
        );
        assert_eq!(
            first.normal_velocity_witness_contact,
            second.normal_velocity_witness_contact
        );
    }

    #[test]
    fn solver_work_and_termination_metadata_are_preserved() {
        let (full, mut reduced) = zero_pair();
        reduced.work_consumed_units = 72;
        reduced.solver_iterations = 12;
        reduced.solver_rows_submitted = 3;
        reduced.solver_termination = SolverTermination::IterationLimit;
        let result = compare_responses(
            &full,
            &reduced,
            &probe_set(vec![probe([1.0, 0.0, 0.0])]),
            scales(),
        )
        .expect("valid response comparison");
        assert_eq!(result.full_work_consumed_units, 100);
        assert_eq!(result.reduced_work_consumed_units, 72);
        assert_eq!(result.full_solver_iterations, 10);
        assert_eq!(result.reduced_solver_iterations, 12);
        assert_eq!(result.full_solver_rows_submitted, 4);
        assert_eq!(result.reduced_solver_rows_submitted, 3);
        assert_eq!(
            result.reduced_solver_termination,
            SolverTermination::IterationLimit
        );
        assert!(!result.both_solvers_converged);
    }

    #[test]
    fn work_unit_schema_must_match() {
        let (full, mut reduced) = zero_pair();
        reduced.work_unit_schema_digest = "different-unit-schema".to_owned();
        assert!(
            compare_responses(
                &full,
                &reduced,
                &probe_set(vec![probe([1.0, 0.0, 0.0])]),
                scales()
            )
            .is_err()
        );
    }

    #[test]
    fn consumed_work_cannot_exceed_budget() {
        let (full, mut reduced) = zero_pair();
        reduced.work_consumed_units = reduced.work_budget_units + 1;
        assert!(
            compare_responses(
                &full,
                &reduced,
                &probe_set(vec![probe([1.0, 0.0, 0.0])]),
                scales()
            )
            .is_err()
        );
    }

    #[test]
    fn unequal_work_budgets_are_rejected() {
        let (full, mut reduced) = zero_pair();
        reduced.work_budget_units += 1;
        assert!(
            compare_responses(
                &full,
                &reduced,
                &probe_set(vec![probe([1.0, 0.0, 0.0])]),
                scales()
            )
            .expect_err("unequal work must fail")
            .reason
            .contains("work budgets")
        );
    }

    #[test]
    fn mismatched_problem_identity_is_rejected() {
        let (full, mut reduced) = zero_pair();
        reduced.solver_config_digest = "different-solver".to_owned();
        assert!(
            compare_responses(
                &full,
                &reduced,
                &probe_set(vec![probe([1.0, 0.0, 0.0])]),
                scales()
            )
            .is_err()
        );
        reduced.solver_config_digest = full.solver_config_digest.clone();
        reduced.initial_state_digest = "other-state".to_owned();
        assert!(
            compare_responses(
                &full,
                &reduced,
                &probe_set(vec![probe([1.0, 0.0, 0.0])]),
                scales()
            )
            .is_err()
        );
    }

    #[test]
    fn body_sets_must_match() {
        let (full, _) = zero_pair();
        let reduced = snapshot("reduced", vec![body("a", [0.0; 3], [0.0; 3], [0.0; 3])]);
        assert!(
            compare_responses(
                &full,
                &reduced,
                &probe_set(vec![probe([1.0, 0.0, 0.0])]),
                scales()
            )
            .is_err()
        );
    }

    #[test]
    fn non_finite_body_values_are_rejected() {
        let full = snapshot(
            "full",
            vec![
                body("a", [f64::NAN, 0.0, 0.0], [0.0; 3], [0.0; 3]),
                body("b", [0.0; 3], [0.0; 3], [0.0; 3]),
            ],
        );
        let reduced = snapshot(
            "reduced",
            vec![
                body("a", [0.0; 3], [0.0; 3], [0.0; 3]),
                body("b", [0.0; 3], [0.0; 3], [0.0; 3]),
            ],
        );
        assert!(
            compare_responses(
                &full,
                &reduced,
                &probe_set(vec![probe([1.0, 0.0, 0.0])]),
                scales()
            )
            .is_err()
        );
    }

    #[test]
    fn empty_probe_set_digest_is_rejected() {
        let (full, reduced) = zero_pair();
        let mut probes = probe_set(vec![probe([1.0, 0.0, 0.0])]);
        probes.digest.clear();
        assert!(compare_responses(&full, &reduced, &probes, scales()).is_err());
    }

    #[test]
    fn invalid_probe_and_non_positive_scales_are_rejected() {
        let (full, reduced) = zero_pair();
        assert!(
            compare_responses(&full, &reduced, &probe_set(vec![probe([0.0; 3])]), scales())
                .is_err()
        );
        assert!(
            compare_responses(
                &full,
                &reduced,
                &probe_set(vec![probe([1.0, 0.0, 0.0])]),
                ResponseScales {
                    velocity_scale: 0.0,
                    ..scales()
                },
            )
            .is_err()
        );
    }

    #[test]
    fn contact_normal_input_is_scale_invariant() {
        let full = snapshot(
            "full",
            vec![
                body("a", [0.0, 1.0, 0.0], [0.0; 3], [0.0; 3]),
                body("b", [0.0; 3], [0.0; 3], [0.0; 3]),
            ],
        );
        let reduced = snapshot(
            "reduced",
            vec![
                body("a", [0.0; 3], [0.0; 3], [0.0; 3]),
                body("b", [0.0; 3], [0.0; 3], [0.0; 3]),
            ],
        );
        let unit = compare_responses(
            &full,
            &reduced,
            &probe_set(vec![probe([0.0, 1.0, 0.0])]),
            scales(),
        )
        .expect("unit");
        let scaled = compare_responses(
            &full,
            &reduced,
            &probe_set(vec![probe([0.0, 17.0, 0.0])]),
            scales(),
        )
        .expect("scaled");
        assert_eq!(
            unit.maximum_contact_normal_velocity_error,
            scaled.maximum_contact_normal_velocity_error
        );
    }
}
