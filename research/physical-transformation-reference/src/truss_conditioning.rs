// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditioning-aware verification cases for the two-bar planar truss.
//!
//! The oracle is an independently assembled 2x2 reduced stiffness matrix. This
//! module distinguishes backward residual from forward error and records when
//! the double-precision reference solver must abstain near a mechanism.

use crate::planar_truss::{PlanarTrussInput, TrussBar, TrussNode, solve};

pub type Check<T> = Result<T, String>;

pub const CONDITIONING_BENCHMARK_ID: &str = "PHYS-TRUSS-CONDITION-001";
pub const CONDITION_REJECT_THRESHOLD: f64 = 1.0e-3;
pub const HEIGHTS_M: [f64; 9] = [
    1.0, 0.1, 0.01, 0.001, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7, 1.0e-8,
];
const SPAN_M: f64 = 4.0;
const APEX_X_M: f64 = 1.0;
const AXIAL_STIFFNESS_A_N_PER_M: f64 = 1.0e9;
const AXIAL_STIFFNESS_B_N_PER_M: f64 = 2.0e9;
const YOUNGS_MODULUS_PA: f64 = 200.0e9;
const TARGET_UX_M: f64 = 1.0e-6;
const TARGET_UY_M: f64 = -2.0e-6;
const RELATIVE_VERTICAL_RHS_PERTURBATION: f64 = 1.0e-10;

#[derive(Clone, Debug, PartialEq)]
pub struct ConditioningCaseReport {
    pub case_index: usize,
    pub apex_height_m: f64,
    pub reduced_kxx_n_per_m: f64,
    pub reduced_kxy_n_per_m: f64,
    pub reduced_kyy_n_per_m: f64,
    pub determinant_n2_per_m2: f64,
    pub condition_infinity: f64,
    pub reciprocal_condition_infinity: f64,
    pub condition_times_machine_epsilon: f64,
    pub target_displacement_m: [f64; 2],
    pub applied_load_n: [f64; 2],
    /// A controlled relative perturbation of the vertical right-hand-side component.
    /// This is a sensitivity probe, not a measured uncertainty distribution.
    pub vertical_rhs_perturbation_n: f64,
    pub vertical_rhs_perturbation_relative_amplitude: f64,
    pub vertical_rhs_plus_relative_displacement_change: f64,
    pub vertical_rhs_minus_relative_displacement_change: f64,
    pub vertical_rhs_plus_amplification: f64,
    pub vertical_rhs_minus_amplification: f64,
    /// "accepted" means the numerical response met the preregistered error checks.
    /// "rejected_numerically_unresolved" means the solver abstained as expected.
    pub disposition: String,
    pub observed_displacement_m: Option<[f64; 2]>,
    pub forward_error_relative: Option<f64>,
    pub backward_error_relative: Option<f64>,
    pub condition_amplified_backward_error: Option<f64>,
    pub forward_error_bound: Option<f64>,
    pub forward_error_acceptance_limit: Option<f64>,
    pub normalized_force_residual: Option<f64>,
    pub normalized_moment_residual: Option<f64>,
    pub normalized_free_dof_residual: Option<f64>,
    pub normalized_energy_residual: Option<f64>,
    pub solver_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConditioningSweepReport {
    pub benchmark_id: String,
    pub machine_epsilon: f64,
    pub condition_reject_threshold: f64,
    pub accepted_case_count: usize,
    pub rejected_case_count: usize,
    pub cases: Vec<ConditioningCaseReport>,
    pub claim_ceiling: String,
}

/// Compute all nine conditioning cases and their actual per-case diagnostics.
///
/// The returned report exists only if every accepted case passes its forward-error
/// envelope and every case expected to be unresolved is rejected by the production
/// solver. A regression that accepts an out-of-envelope case or rejects an expected
/// eligible case returns Err, so CI cannot publish a misleading "pass" receipt.
pub fn conditioning_sweep_report() -> Check<ConditioningSweepReport> {
    let mut cases = Vec::with_capacity(HEIGHTS_M.len());
    let mut accepted_case_count = 0usize;
    let mut rejected_case_count = 0usize;
    let mut previous_condition = 0.0_f64;

    for (case_index, apex_height_m) in HEIGHTS_M.into_iter().enumerate() {
        let length_a_m = APEX_X_M.hypot(apex_height_m);
        let length_b_m = (APEX_X_M - SPAN_M).hypot(apex_height_m);
        let cosine_a = APEX_X_M / length_a_m;
        let sine_a = apex_height_m / length_a_m;
        let cosine_b = (APEX_X_M - SPAN_M) / length_b_m;
        let sine_b = apex_height_m / length_b_m;

        let kxx = AXIAL_STIFFNESS_A_N_PER_M * cosine_a * cosine_a
            + AXIAL_STIFFNESS_B_N_PER_M * cosine_b * cosine_b;
        let kxy = AXIAL_STIFFNESS_A_N_PER_M * cosine_a * sine_a
            + AXIAL_STIFFNESS_B_N_PER_M * cosine_b * sine_b;
        let kyy = AXIAL_STIFFNESS_A_N_PER_M * sine_a * sine_a
            + AXIAL_STIFFNESS_B_N_PER_M * sine_b * sine_b;

        // Use a geometrical identity for det(K), avoiding cancellation in a*d-b*b
        // near collinearity. This oracle is specific to the two-bar reduced system.
        let direction_cross = cosine_a * sine_b - sine_a * cosine_b;
        let determinant = AXIAL_STIFFNESS_A_N_PER_M
            * AXIAL_STIFFNESS_B_N_PER_M
            * direction_cross
            * direction_cross;
        if !determinant.is_finite() || determinant <= 0.0 {
            return Err(format!(
                "conditioning case {case_index} h={apex_height_m:.3e}: independent determinant is invalid"
            ));
        }

        let matrix_inf_norm = (kxx.abs() + kxy.abs()).max(kxy.abs() + kyy.abs());
        let inverse_inf_norm = ((kyy.abs() + kxy.abs()) / determinant)
            .max((kxx.abs() + kxy.abs()) / determinant);
        let condition_infinity = matrix_inf_norm * inverse_inf_norm;
        let condition_times_machine_epsilon = condition_infinity * f64::EPSILON;
        if !condition_infinity.is_finite()
            || condition_infinity < 1.0
            || !condition_times_machine_epsilon.is_finite()
            || condition_infinity <= previous_condition
        {
            return Err(format!(
                "conditioning case {case_index} h={apex_height_m:.3e}: invalid or non-monotone condition estimate {condition_infinity:.17e}"
            ));
        }
        previous_condition = condition_infinity;
        let reciprocal_condition_infinity = 1.0 / condition_infinity;

        // Establish truth first, then independently derive the applied load F=K*u*.
        let target_displacement_m = [TARGET_UX_M, TARGET_UY_M];
        let applied_load_n = [
            kxx * TARGET_UX_M + kxy * TARGET_UY_M,
            kxy * TARGET_UX_M + kyy * TARGET_UY_M,
        ];
        if !applied_load_n.iter().all(|component| component.is_finite()) {
            return Err(format!(
                "conditioning case {case_index} h={apex_height_m:.3e}: independent target load is non-finite"
            ));
        }
        let sensitivity = vertical_rhs_perturbation_sensitivity(
            kxx,
            kxy,
            kyy,
            determinant,
            applied_load_n,
        )?;

        let input = build_input(apex_height_m, applied_load_n, length_a_m, length_b_m);
        let solved = solve(&input);
        if condition_times_machine_epsilon >= CONDITION_REJECT_THRESHOLD {
            match solved {
                Ok(_) => {
                    return Err(format!(
                        "conditioning case {case_index} h={apex_height_m:.3e}: solver accepted an out-of-envelope system (condition*epsilon={condition_times_machine_epsilon:.3e})"
                    ));
                }
                Err(error)
                    if error.contains("singular or numerically unresolved")
                        || error.contains("singular truss") =>
                {
                    rejected_case_count += 1;
                    cases.push(ConditioningCaseReport {
                        case_index,
                        apex_height_m,
                        reduced_kxx_n_per_m: kxx,
                        reduced_kxy_n_per_m: kxy,
                        reduced_kyy_n_per_m: kyy,
                        determinant_n2_per_m2: determinant,
                        condition_infinity,
                        reciprocal_condition_infinity,
                        condition_times_machine_epsilon,
                        target_displacement_m,
                        applied_load_n,
                        vertical_rhs_perturbation_n: sensitivity.perturbation_n,
                        vertical_rhs_perturbation_relative_amplitude: sensitivity.relative_amplitude,
                        vertical_rhs_plus_relative_displacement_change: sensitivity.plus_relative_displacement_change,
                        vertical_rhs_minus_relative_displacement_change: sensitivity.minus_relative_displacement_change,
                        vertical_rhs_plus_amplification: sensitivity.plus_amplification,
                        vertical_rhs_minus_amplification: sensitivity.minus_amplification,
                        disposition: "rejected_numerically_unresolved".to_owned(),
                        observed_displacement_m: None,
                        forward_error_relative: None,
                        backward_error_relative: None,
                        condition_amplified_backward_error: None,
                        forward_error_bound: None,
                        forward_error_acceptance_limit: None,
                        normalized_force_residual: None,
                        normalized_moment_residual: None,
                        normalized_free_dof_residual: None,
                        normalized_energy_residual: None,
                        solver_error: Some(error),
                    });
                    continue;
                }
                Err(error) => {
                    return Err(format!(
                        "conditioning case {case_index} h={apex_height_m:.3e}: expected numerical-abstention error, got {error}"
                    ));
                }
            }
        }

        let result = solved.map_err(|error| {
            format!(
                "conditioning case {case_index} h={apex_height_m:.3e}, condition={condition_infinity:.3e}: an eligible case unexpectedly failed: {error}"
            )
        })?;
        let apex = result
            .node_results
            .iter()
            .find(|node| node.node_id == "C")
            .ok_or_else(|| format!("conditioning case {case_index}: output omitted apex C"))?;
        let observed_displacement_m = [apex.displacement_x_m, apex.displacement_y_m];
        if !observed_displacement_m.iter().all(|component| component.is_finite()) {
            return Err(format!(
                "conditioning case {case_index} h={apex_height_m:.3e}: output displacement is non-finite"
            ));
        }

        let target_scale = TARGET_UY_M.abs().max(TARGET_UX_M.abs());
        let forward_error_relative = (observed_displacement_m[0] - TARGET_UX_M)
            .abs()
            .max((observed_displacement_m[1] - TARGET_UY_M).abs())
            / target_scale;
        let residual_x = kxx * observed_displacement_m[0]
            + kxy * observed_displacement_m[1]
            - applied_load_n[0];
        let residual_y = kxy * observed_displacement_m[0]
            + kyy * observed_displacement_m[1]
            - applied_load_n[1];
        let backward_numerator = residual_x.abs().max(residual_y.abs());
        let displacement_inf_norm = observed_displacement_m[0]
            .abs()
            .max(observed_displacement_m[1].abs());
        let force_inf_norm = applied_load_n[0].abs().max(applied_load_n[1].abs());
        let backward_denominator = matrix_inf_norm * displacement_inf_norm + force_inf_norm;
        if !backward_denominator.is_finite() || backward_denominator <= 0.0 {
            return Err(format!(
                "conditioning case {case_index} h={apex_height_m:.3e}: invalid backward-error scale"
            ));
        }
        let backward_error_relative = backward_numerator / backward_denominator;
        let condition_amplified_backward_error =
            condition_infinity * backward_error_relative;
        if !forward_error_relative.is_finite()
            || !backward_error_relative.is_finite()
            || !condition_amplified_backward_error.is_finite()
            || condition_amplified_backward_error >= 0.25
        {
            return Err(format!(
                "conditioning case {case_index} h={apex_height_m:.3e}: backward residual cannot support the forward-error bound (condition={condition_infinity:.3e}, backward={backward_error_relative:.3e})"
            ));
        }

        let forward_error_bound = condition_amplified_backward_error
            / (1.0 - condition_amplified_backward_error)
            + 16.0 * condition_times_machine_epsilon
            + 1.0e-10;
        let forward_error_acceptance_limit = forward_error_bound.max(1.0e-8);
        if forward_error_relative > forward_error_acceptance_limit {
            return Err(format!(
                "conditioning case {case_index} h={apex_height_m:.3e}: forward error {forward_error_relative:.3e} exceeds limit {forward_error_acceptance_limit:.3e}"
            ));
        }

        for (name, residual) in [
            ("force", result.equilibrium.normalized_force_residual),
            ("moment", result.equilibrium.normalized_moment_residual),
            ("free-DOF", result.equilibrium.normalized_free_dof_residual),
            ("energy", result.energy.normalized_residual),
        ] {
            if !residual.is_finite() || residual >= 1.0e-12 {
                return Err(format!(
                    "conditioning case {case_index} h={apex_height_m:.3e}: normalized {name} residual {residual:.3e} exceeds 1e-12"
                ));
            }
        }

        accepted_case_count += 1;
        cases.push(ConditioningCaseReport {
            case_index,
            apex_height_m,
            reduced_kxx_n_per_m: kxx,
            reduced_kxy_n_per_m: kxy,
            reduced_kyy_n_per_m: kyy,
            determinant_n2_per_m2: determinant,
            condition_infinity,
            reciprocal_condition_infinity,
            condition_times_machine_epsilon,
            target_displacement_m,
            applied_load_n,
            vertical_rhs_perturbation_n: sensitivity.perturbation_n,
            vertical_rhs_perturbation_relative_amplitude: sensitivity.relative_amplitude,
            vertical_rhs_plus_relative_displacement_change: sensitivity.plus_relative_displacement_change,
            vertical_rhs_minus_relative_displacement_change: sensitivity.minus_relative_displacement_change,
            vertical_rhs_plus_amplification: sensitivity.plus_amplification,
            vertical_rhs_minus_amplification: sensitivity.minus_amplification,
            disposition: "accepted".to_owned(),
            observed_displacement_m: Some(observed_displacement_m),
            forward_error_relative: Some(forward_error_relative),
            backward_error_relative: Some(backward_error_relative),
            condition_amplified_backward_error: Some(condition_amplified_backward_error),
            forward_error_bound: Some(forward_error_bound),
            forward_error_acceptance_limit: Some(forward_error_acceptance_limit),
            normalized_force_residual: Some(result.equilibrium.normalized_force_residual),
            normalized_moment_residual: Some(result.equilibrium.normalized_moment_residual),
            normalized_free_dof_residual: Some(result.equilibrium.normalized_free_dof_residual),
            normalized_energy_residual: Some(result.energy.normalized_residual),
            solver_error: None,
        });
    }

    if accepted_case_count != 7 || rejected_case_count != 2 || cases.len() != HEIGHTS_M.len() {
        return Err(format!(
            "conditioning sweep count contract changed: accepted={accepted_case_count}, rejected={rejected_case_count}, total={}",
            cases.len()
        ));
    }

    Ok(ConditioningSweepReport {
        benchmark_id: CONDITIONING_BENCHMARK_ID.to_owned(),
        machine_epsilon: f64::EPSILON,
        condition_reject_threshold: CONDITION_REJECT_THRESHOLD,
        accepted_case_count,
        rejected_case_count,
        cases,
        claim_ceiling: "synthetic static small-displacement linear-elastic 2D pin-jointed truss conditioning test only; no physical validation, external FEM qualification, or structural safety claim".to_owned(),
    })
}

struct RhsPerturbationSensitivity {
    perturbation_n: f64,
    relative_amplitude: f64,
    plus_relative_displacement_change: f64,
    minus_relative_displacement_change: f64,
    plus_amplification: f64,
    minus_amplification: f64,
}

fn vertical_rhs_perturbation_sensitivity(
    kxx: f64,
    kxy: f64,
    kyy: f64,
    determinant: f64,
    force: [f64; 2],
) -> Check<RhsPerturbationSensitivity> {
    let force_scale = force[0].abs().max(force[1].abs());
    if !force_scale.is_finite() || force_scale <= 0.0 {
        return Err("vertical RHS sensitivity has an invalid force scale".to_owned());
    }
    let perturbation_n = RELATIVE_VERTICAL_RHS_PERTURBATION * force_scale;
    let relative_amplitude = perturbation_n / force_scale;
    let solve_two_by_two = |fx: f64, fy: f64| -> [f64; 2] {
        [
            (kyy * fx - kxy * fy) / determinant,
            (kxx * fy - kxy * fx) / determinant,
        ]
    };
    let baseline = solve_two_by_two(force[0], force[1]);
    let plus = solve_two_by_two(force[0], force[1] + perturbation_n);
    let minus = solve_two_by_two(force[0], force[1] - perturbation_n);
    let output_scale = baseline[0].abs().max(baseline[1].abs());
    if !baseline.iter().chain(plus.iter()).chain(minus.iter()).all(|value| value.is_finite())
        || !output_scale.is_finite()
        || output_scale <= 0.0
        || !relative_amplitude.is_finite()
        || relative_amplitude <= 0.0
    {
        return Err("vertical RHS sensitivity calculation is non-finite or undefined".to_owned());
    }
    let plus_change = (plus[0] - baseline[0])
        .abs()
        .max((plus[1] - baseline[1]).abs())
        / output_scale;
    let minus_change = (minus[0] - baseline[0])
        .abs()
        .max((minus[1] - baseline[1]).abs())
        / output_scale;
    let plus_amplification = plus_change / relative_amplitude;
    let minus_amplification = minus_change / relative_amplitude;
    if ![
        plus_change,
        minus_change,
        plus_amplification,
        minus_amplification,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err("vertical RHS sensitivity outputs are non-finite".to_owned());
    }
    Ok(RhsPerturbationSensitivity {
        perturbation_n,
        relative_amplitude,
        plus_relative_displacement_change: plus_change,
        minus_relative_displacement_change: minus_change,
        plus_amplification,
        minus_amplification,
    })
}

fn build_input(
    apex_height_m: f64,
    applied_load_n: [f64; 2],
    length_a_m: f64,
    length_b_m: f64,
) -> PlanarTrussInput {
    PlanarTrussInput {
        nodes: vec![
            TrussNode {
                id: "A".to_owned(),
                x_m: 0.0,
                y_m: 0.0,
                restrained_x: true,
                restrained_y: true,
                force_x_n: 0.0,
                force_y_n: 0.0,
            },
            TrussNode {
                id: "B".to_owned(),
                x_m: SPAN_M,
                y_m: 0.0,
                restrained_x: true,
                restrained_y: true,
                force_x_n: 0.0,
                force_y_n: 0.0,
            },
            TrussNode {
                id: "C".to_owned(),
                x_m: APEX_X_M,
                y_m: apex_height_m,
                restrained_x: false,
                restrained_y: false,
                force_x_n: applied_load_n[0],
                force_y_n: applied_load_n[1],
            },
        ],
        bars: vec![
            TrussBar {
                id: "AC".to_owned(),
                node_i: "A".to_owned(),
                node_j: "C".to_owned(),
                area_m2: AXIAL_STIFFNESS_A_N_PER_M * length_a_m / YOUNGS_MODULUS_PA,
                youngs_modulus_pa: YOUNGS_MODULUS_PA,
            },
            TrussBar {
                id: "BC".to_owned(),
                node_i: "B".to_owned(),
                node_j: "C".to_owned(),
                area_m2: AXIAL_STIFFNESS_B_N_PER_M * length_b_m / YOUNGS_MODULUS_PA,
                youngs_modulus_pa: YOUNGS_MODULUS_PA,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_is_deterministic_and_separates_accepted_from_rejected_cases() {
        let first = conditioning_sweep_report().expect("conditioning sweep should pass");
        let second = conditioning_sweep_report().expect("same frozen inputs should pass");
        assert_eq!(first, second);
        assert_eq!(first.benchmark_id, CONDITIONING_BENCHMARK_ID);
        assert_eq!(first.cases.len(), 9);
        assert_eq!(first.accepted_case_count, 7);
        assert_eq!(first.rejected_case_count, 2);
        assert!(
            first
                .cases
                .iter()
                .all(|case| case.vertical_rhs_perturbation_relative_amplitude > 0.0
                    && case.vertical_rhs_plus_amplification.is_finite()
                    && case.vertical_rhs_minus_amplification.is_finite())
        );
        assert!(first.cases.iter().all(|case| {
            (case.vertical_rhs_perturbation_relative_amplitude
                - RELATIVE_VERTICAL_RHS_PERTURBATION)
                .abs()
                < 1.0e-20
        }));
        assert!(
            first.cases.windows(2).all(|pair| {
                pair[0].vertical_rhs_plus_amplification
                    < pair[1].vertical_rhs_plus_amplification
                    && pair[0].vertical_rhs_minus_amplification
                        < pair[1].vertical_rhs_minus_amplification
            })
        );
        assert!(
            first
                .cases
                .windows(2)
                .all(|pair| pair[0].condition_infinity < pair[1].condition_infinity)
        );
        for case in &first.cases {
            if case.disposition == "accepted" {
                assert!(case.forward_error_relative.is_some_and(|value| value.is_finite()));
                assert!(case.backward_error_relative.is_some_and(f64::is_finite));
                assert!(case.forward_error_acceptance_limit.is_some_and(f64::is_finite));
                assert!(case.normalized_energy_residual.is_some_and(f64::is_finite));
                assert!(case.solver_error.is_none());
            } else {
                assert_eq!(case.disposition, "rejected_numerically_unresolved");
                assert!(case.observed_displacement_m.is_none());
                assert!(case.forward_error_relative.is_none());
                assert!(case.backward_error_relative.is_none());
                assert!(case.solver_error.is_some());
            }
        }
    }
}
