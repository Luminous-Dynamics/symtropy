// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic synthetic smoke tournament for the response comparator.

use solver_response_oracle::{
    BodyResponse, ContactProbe, ContactProbeSet, ResponseScales, SolverResponseSnapshot, SolverTermination,
    compare_responses,
};

fn body(
    id: &str,
    linear_delta: [f64; 3],
    angular_delta: [f64; 3],
    linear_impulse: [f64; 3],
    angular_impulse: [f64; 3],
) -> BodyResponse {
    BodyResponse {
        body_id: id.to_owned(),
        delta_linear_velocity: linear_delta,
        delta_angular_velocity: angular_delta,
        linear_impulse,
        angular_impulse,
    }
}

fn snapshot(candidate: &str, reduced: bool) -> SolverResponseSnapshot {
    let (linear_delta, angular_delta, impulse) = if reduced {
        ([0.0, 0.94, 0.0], [0.0, 0.0, 0.08], [0.0, 9.4, 0.0])
    } else {
        ([0.0, 1.0, 0.0], [0.0, 0.0, 0.1], [0.0, 10.0, 0.0])
    };

    SolverResponseSnapshot {
        scenario_digest: "synthetic-box-stack-state-001".to_owned(),
        initial_state_digest: "initial-state-001".to_owned(),
        physics_model_digest: "frictionless-contact-smoke-model-001".to_owned(),
        solver_config_digest: "solver-config-equal-budget-001".to_owned(),
        work_unit_schema_digest: "constraint-row-visits-v1".to_owned(),
        work_budget_units: 256,
        work_consumed_units: if reduced { 240 } else { 220 },
        solver_iterations: if reduced { 9 } else { 7 },
        solver_rows_submitted: if reduced { 3 } else { 4 },
        solver_termination: SolverTermination::Converged,
        candidate_set_digest: candidate.to_owned(),
        warm_start_digest: if reduced {
            "warm-start-reduced-001"
        } else {
            "warm-start-full-001"
        }
        .to_owned(),
        bodies: vec![
            body("box", linear_delta, angular_delta, impulse, [0.0, 0.0, 0.5]),
            body(
                "ground",
                [0.0; 3],
                [0.0; 3],
                [0.0, -10.0, 0.0],
                [0.0, 0.0, -0.5],
            ),
        ],
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let full = snapshot("full-candidate-set-001", false);
    let reduced = snapshot("reduced-candidate-set-001", true);
    let probes = ContactProbeSet {
        digest: "full-patch-contact-probes-smoke-001".to_owned(),
        probes: vec![
        ContactProbe {
            contact_id: "corner-left".to_owned(),
            body_a_id: "box".to_owned(),
            body_b_id: "ground".to_owned(),
            lever_arm_a: [-0.5, 0.0, 0.0],
            lever_arm_b: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
        },
        ContactProbe {
            contact_id: "corner-right".to_owned(),
            body_a_id: "box".to_owned(),
            body_b_id: "ground".to_owned(),
            lever_arm_a: [0.5, 0.0, 0.0],
            lever_arm_b: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
        },
        ],
    };
    let report = compare_responses(
        &full,
        &reduced,
        &probes,
        ResponseScales {
            characteristic_length: 0.5,
            velocity_scale: 1.0,
            impulse_scale: 10.0,
        },
    )?;

    println!(
        "schema,scenario,model,solver,work_unit_schema,budget,probe_set_digest,probe_count,characteristic_length,velocity_scale,impulse_scale,full_work,reduced_work,full_iterations,reduced_iterations,full_rows,reduced_rows,both_converged,max_generalized_velocity_error,max_generalized_impulse_error,max_contact_normal_velocity_error,max_contact_tangent_velocity_error,rms_contact_velocity_error,velocity_witness_body,normal_witness_contact"
    );
    println!(
        "{},{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{},{}",
        solver_response_oracle::SCHEMA_VERSION,
        report.scenario_digest,
        report.physics_model_digest,
        report.solver_config_digest,
        report.work_unit_schema_digest,
        report.work_budget_units,
        report.probe_set_digest,
        report.contact_probe_count,
        report.characteristic_length,
        report.velocity_scale,
        report.impulse_scale,
        report.full_work_consumed_units,
        report.reduced_work_consumed_units,
        report.full_solver_iterations,
        report.reduced_solver_iterations,
        report.full_solver_rows_submitted,
        report.reduced_solver_rows_submitted,
        report.both_solvers_converged,
        report.maximum_generalized_velocity_error,
        report.maximum_generalized_impulse_error,
        report.maximum_contact_normal_velocity_error,
        report.maximum_contact_tangent_velocity_error,
        report.rms_contact_velocity_error,
        report.generalized_velocity_witness_body,
        report.normal_velocity_witness_contact,
    );
    Ok(())
}
