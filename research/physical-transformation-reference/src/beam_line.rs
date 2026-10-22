// SPDX-License-Identifier: AGPL-3.0-or-later
//! One-dimensional continuous Euler-Bernoulli beam-line modal reference.
//!
//! Nodal transverse displacement and rotation are shared along a continuous
//! beam (unlike an ideal-pin truss assembly, where member rotations are private).
//! This module evaluates candidate SOFiSTiK BE25 inputs under explicit mass
//! interpretations. It is not itself a benchmark pass.

use crate::Check;
use crate::modal_bar::{generalized_symmetric_eigenpairs, normalized_eigen_residual};

const MAX_ELEMENTS: usize = 64;
const MAX_FREE_DOFS: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeamLineBoundaryCase {
    SimpleCantilever,
    CantileverWithSimplySupportedEnd,
    SimplySupported,
    BothEndsFixed,
}

impl BeamLineBoundaryCase {
    pub const ALL: [Self; 4] = [
        Self::SimpleCantilever,
        Self::CantileverWithSimplySupportedEnd,
        Self::SimplySupported,
        Self::BothEndsFixed,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::SimpleCantilever => "simple-cantilever",
            Self::CantileverWithSimplySupportedEnd => "cantilever-with-simply-supported-end",
            Self::SimplySupported => "simply-supported",
            Self::BothEndsFixed => "both-ends-fixed",
        }
    }

    const fn lambda(self) -> f64 {
        match self {
            Self::SimpleCantilever => 1.875104068711961,
            Self::CantileverWithSimplySupportedEnd => 3.926602312047919,
            Self::SimplySupported => std::f64::consts::PI,
            Self::BothEndsFixed => 4.730040744862704,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeamLineMassModel {
    /// Hermite-interpolated translational consistent mass; rotary inertia omitted.
    ConsistentTranslational,
}

/// Frozen canonical request for the source-literal BE25 candidate runs.
pub const fn canonical_request() -> &'static str {
    concat!(
        "schema=sofistik-be25-line-candidate-v0.1",
        "|benchmark=SOFiSTiK-2026-BE25",
        "|model=uniform_1d_continuous_euler_bernoulli_beam",
        "|elements=10|mass_model=consistent_translational_only",
        "|length_m=1|youngs_modulus_pa=200000000|second_moment_m4=1e-9",
        "|mass_candidate_printed_mu_kg_per_m=25",
        "|mass_candidate_gamma_area_over_g_kg_per_m=0.25492905324448206",
        "|boundary_cases=simple-cantilever,cantilever-with-simply-supported-end,simply-supported,both-ends-fixed",
        "|claim_ceiling=numerical_candidate_analysis_not_BE25_reproduction"
    )
}

fn published_reference_frequency_hz(case: BeamLineBoundaryCase) -> f64 {
    match case {
        BeamLineBoundaryCase::SimpleCantilever => 0.457,
        BeamLineBoundaryCase::CantileverWithSimplySupportedEnd => 2.003,
        BeamLineBoundaryCase::SimplySupported => 1.283,
        BeamLineBoundaryCase::BothEndsFixed => 2.907,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeamLineMode {
    pub eigenvalue_rad_s_squared: f64,
    pub frequency_hz: f64,
    pub normalized_eigen_residual: f64,
    /// Full vector ordered [v0, theta0, v1, theta1, ...].
    pub shape: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeamLineModalSolution {
    pub boundary_case: BeamLineBoundaryCase,
    pub element_count: usize,
    pub length_m: f64,
    pub mass_per_length_kg_per_m: f64,
    pub mass_model: BeamLineMassModel,
    pub fixed_dof_indices: Vec<usize>,
    pub free_dof_indices: Vec<usize>,
    pub modes: Vec<BeamLineMode>,
    pub analytical_first_frequency_hz: f64,
    pub first_frequency_relative_error: f64,
    pub published_reference_frequency_hz: f64,
    pub first_frequency_relative_error_to_published_reference: f64,
    pub maximum_normalized_eigen_residual: f64,
}

/// Solve a uniform Euler-Bernoulli beam line using consistent translational
/// mass. The mass-per-length input is independent of gamma*A/g so contradictory
/// source inputs stay explicit rather than being silently reconciled.
pub fn solve_beam_line_modes(
    length_m: f64,
    youngs_modulus_pa: f64,
    second_moment_m4: f64,
    mass_per_length_kg_per_m: f64,
    element_count: usize,
    boundary_case: BeamLineBoundaryCase,
    mass_model: BeamLineMassModel,
) -> Check<BeamLineModalSolution> {
    for (name, value) in [
        ("length_m", length_m),
        ("youngs_modulus_pa", youngs_modulus_pa),
        ("second_moment_m4", second_moment_m4),
        ("mass_per_length_kg_per_m", mass_per_length_kg_per_m),
    ] {
        validate_positive(name, value)?;
    }
    if element_count == 0 || element_count > MAX_ELEMENTS {
        return Err(format!("element count must be between 1 and {MAX_ELEMENTS}"));
    }
    if mass_model != BeamLineMassModel::ConsistentTranslational {
        return Err("unsupported BE25 mass model".to_owned());
    }

    let node_count = element_count + 1;
    let total_dofs = node_count * 2;
    let h = length_m / element_count as f64;
    let l2 = h * h;
    let l3 = l2 * h;
    let stiffness_factor = youngs_modulus_pa * second_moment_m4 / l3;
    let stiffness_shape = [
        [12.0, 6.0 * h, -12.0, 6.0 * h],
        [6.0 * h, 4.0 * l2, -6.0 * h, 2.0 * l2],
        [-12.0, -6.0 * h, 12.0, -6.0 * h],
        [6.0 * h, 2.0 * l2, -6.0 * h, 4.0 * l2],
    ];
    let mass_factor = mass_per_length_kg_per_m * h / 420.0;
    let mass_shape = [
        [156.0, 22.0 * h, 54.0, -13.0 * h],
        [22.0 * h, 4.0 * l2, 13.0 * h, -3.0 * l2],
        [54.0, 13.0 * h, 156.0, -22.0 * h],
        [-13.0 * h, -3.0 * l2, -22.0 * h, 4.0 * l2],
    ];

    let mut stiffness = vec![vec![0.0; total_dofs]; total_dofs];
    let mut mass = vec![vec![0.0; total_dofs]; total_dofs];
    for element in 0..element_count {
        let base = 2 * element;
        let dof_map = [base, base + 1, base + 2, base + 3];
        for row in 0..4 {
            for col in 0..4 {
                stiffness[dof_map[row]][dof_map[col]] +=
                    stiffness_factor * stiffness_shape[row][col];
                mass[dof_map[row]][dof_map[col]] += mass_factor * mass_shape[row][col];
            }
        }
    }
    ensure_finite_symmetric("beam-line stiffness", &stiffness)?;
    ensure_finite_symmetric("beam-line mass", &mass)?;

    let fixed_dof_indices = boundary_fixed_dofs(element_count, boundary_case);
    let fixed: std::collections::BTreeSet<usize> =
        fixed_dof_indices.iter().copied().collect();
    let free_dof_indices = (0..total_dofs)
        .filter(|index| !fixed.contains(index))
        .collect::<Vec<_>>();
    if free_dof_indices.is_empty() || free_dof_indices.len() > MAX_FREE_DOFS {
        return Err(format!("free-DOF count must be between 1 and {MAX_FREE_DOFS}"));
    }

    let stiffness_reduced = reduce_matrix(&stiffness, &free_dof_indices);
    let mass_reduced = reduce_matrix(&mass, &free_dof_indices);
    let eigenpairs = generalized_symmetric_eigenpairs(&stiffness_reduced, &mass_reduced)?;
    if eigenpairs.len() != free_dof_indices.len() {
        return Err("beam-line eigenpair count differs from free-DOF count".to_owned());
    }

    let mut modes = Vec::with_capacity(eigenpairs.len());
    for (eigenvalue, reduced_shape) in eigenpairs {
        if !eigenvalue.is_finite() || eigenvalue <= 0.0 {
            return Err("beam-line case contains a non-positive eigenvalue".to_owned());
        }
        let normalized_eigen_residual = normalized_eigen_residual(
            &stiffness_reduced,
            &mass_reduced,
            eigenvalue,
            &reduced_shape,
        )?;
        let mut shape = vec![0.0; total_dofs];
        for (reduced_index, global_index) in free_dof_indices.iter().enumerate() {
            shape[*global_index] = reduced_shape[reduced_index];
        }
        modes.push(BeamLineMode {
            eigenvalue_rad_s_squared: eigenvalue,
            frequency_hz: eigenvalue.sqrt() / (2.0 * std::f64::consts::PI),
            normalized_eigen_residual,
            shape,
        });
    }

    let analytical_first_frequency_hz = analytical_frequency_hz(
        boundary_case.lambda(),
        youngs_modulus_pa,
        second_moment_m4,
        mass_per_length_kg_per_m,
        length_m,
    )?;
    let first_frequency = modes
        .first()
        .ok_or_else(|| "beam-line eigenproblem returned no modes".to_owned())?
        .frequency_hz;
    let first_frequency_relative_error =
        relative_error(first_frequency, analytical_first_frequency_hz);
    let published_reference_frequency_hz = published_reference_frequency_hz(boundary_case);
    let first_frequency_relative_error_to_published_reference =
        relative_error(first_frequency, published_reference_frequency_hz);
    let maximum_normalized_eigen_residual = modes
        .iter()
        .map(|mode| mode.normalized_eigen_residual)
        .fold(0.0_f64, f64::max);

    Ok(BeamLineModalSolution {
        boundary_case,
        element_count,
        length_m,
        mass_per_length_kg_per_m,
        mass_model,
        fixed_dof_indices,
        free_dof_indices,
        modes,
        analytical_first_frequency_hz,
        first_frequency_relative_error,
        published_reference_frequency_hz,
        first_frequency_relative_error_to_published_reference,
        maximum_normalized_eigen_residual,
    })
}

fn boundary_fixed_dofs(
    element_count: usize,
    boundary_case: BeamLineBoundaryCase,
) -> Vec<usize> {
    let last_node = element_count;
    match boundary_case {
        BeamLineBoundaryCase::SimpleCantilever => vec![0, 1],
        BeamLineBoundaryCase::CantileverWithSimplySupportedEnd => vec![0, 1, 2 * last_node],
        BeamLineBoundaryCase::SimplySupported => vec![0, 2 * last_node],
        BeamLineBoundaryCase::BothEndsFixed => vec![0, 1, 2 * last_node, 2 * last_node + 1],
    }
}

fn analytical_frequency_hz(
    lambda: f64,
    youngs_modulus_pa: f64,
    second_moment_m4: f64,
    mass_per_length_kg_per_m: f64,
    length_m: f64,
) -> Check<f64> {
    let frequency = lambda.powi(2) / (2.0 * std::f64::consts::PI)
        * (youngs_modulus_pa * second_moment_m4
            / (mass_per_length_kg_per_m * length_m.powi(4)))
            .sqrt();
    if frequency.is_finite() && frequency > 0.0 {
        Ok(frequency)
    } else {
        Err("analytical beam frequency is not finite and positive".to_owned())
    }
}

fn validate_positive(name: &str, value: f64) -> Check<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be finite and positive"))
    }
}

fn ensure_finite_symmetric(name: &str, matrix: &[Vec<f64>]) -> Check<()> {
    let size = matrix.len();
    if matrix.iter().any(|row| row.len() != size) {
        return Err(format!("{name} must be square"));
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(format!("{name} contains a non-finite entry"));
    }
    for row in 0..size {
        for col in 0..size {
            let scale = matrix[row][col].abs().max(matrix[col][row].abs()).max(1.0);
            if (matrix[row][col] - matrix[col][row]).abs() > 1.0e-10 * scale {
                return Err(format!("{name} is not symmetric at ({row}, {col})"));
            }
        }
    }
    Ok(())
}

fn reduce_matrix(matrix: &[Vec<f64>], free_dof_indices: &[usize]) -> Vec<Vec<f64>> {
    free_dof_indices
        .iter()
        .map(|row| {
            free_dof_indices
                .iter()
                .map(|col| matrix[*row][*col])
                .collect()
        })
        .collect()
}

fn relative_error(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE)
}

#[cfg(test)]
mod tests {
    use super::{BeamLineBoundaryCase, BeamLineMassModel, solve_beam_line_modes};

    const E_PA: f64 = 200.0e6;
    const I_M4: f64 = 0.1e-8;
    const L_M: f64 = 1.0;
    const MU_FROM_GAMMA_KG_PER_M: f64 = 25_000.0 * 1.0e-4 / 9.80665;
    const MU_PRINTED_KG_PER_M: f64 = 25.0;

    fn solve(
        case: BeamLineBoundaryCase,
        elements: usize,
        mu_kg_per_m: f64,
    ) -> super::BeamLineModalSolution {
        solve_beam_line_modes(
            L_M,
            E_PA,
            I_M4,
            mu_kg_per_m,
            elements,
            case,
            BeamLineMassModel::ConsistentTranslational,
        )
        .expect("valid BE25 candidate beam-line solve")
    }

    #[test]
    fn canonical_request_matches_the_frozen_candidate_fixture() {
        assert_eq!(
            super::canonical_request(),
            include_str!("../fixtures/sofistik_be25_line_candidate_request_v0_1.txt")
                .trim_end_matches('\n')
        );
    }

    #[test]
    fn ten_element_candidate_runs_cover_all_four_published_boundary_cases() {
        for case in BeamLineBoundaryCase::ALL {
            let derived = solve(case, 10, MU_FROM_GAMMA_KG_PER_M);
            let printed = solve(case, 10, MU_PRINTED_KG_PER_M);
            assert_eq!(derived.modes.len(), derived.free_dof_indices.len());
            assert_eq!(printed.modes.len(), printed.free_dof_indices.len());
            assert!(derived.maximum_normalized_eigen_residual < 1.0e-8);
            assert!(printed.maximum_normalized_eigen_residual < 1.0e-8);
            assert!(derived.first_frequency_relative_error < 1.0e-2);
            assert!(printed.first_frequency_relative_error < 1.0e-2);
        }
    }

    #[test]
    fn beam_line_numerical_candidates_preserve_the_source_mass_conflict() {
        let derived = solve(BeamLineBoundaryCase::SimpleCantilever, 10, MU_FROM_GAMMA_KG_PER_M);
        let printed = solve(BeamLineBoundaryCase::SimpleCantilever, 10, MU_PRINTED_KG_PER_M);
        assert!((derived.modes[0].frequency_hz - 0.4955962448521477).abs() < 0.01);
        assert!((printed.modes[0].frequency_hz - 0.05004580383402026).abs() < 0.005);
        assert!((derived.modes[0].frequency_hz - 0.457).abs() > 0.03);
        assert!((printed.modes[0].frequency_hz - 0.457).abs() > 0.3);
    }

    #[test]
    fn thirty_two_element_mesh_converges_toward_the_continuum_frequency_for_each_case() {
        for case in BeamLineBoundaryCase::ALL {
            let ten = solve(case, 10, MU_FROM_GAMMA_KG_PER_M);
            let refined = solve(case, 32, MU_FROM_GAMMA_KG_PER_M);
            assert!(
                refined.first_frequency_relative_error < ten.first_frequency_relative_error,
                "{}: refined mesh did not decrease frequency error",
                case.name()
            );
            assert!(refined.first_frequency_relative_error < 2.0e-3);
        }
    }

    #[test]
    fn frequencies_obey_modulus_mass_and_length_scalings() {
        let base = solve(BeamLineBoundaryCase::SimpleCantilever, 10, MU_FROM_GAMMA_KG_PER_M);
        let stiff = solve_beam_line_modes(
            L_M, 4.0 * E_PA, I_M4, MU_FROM_GAMMA_KG_PER_M, 10,
            BeamLineBoundaryCase::SimpleCantilever, BeamLineMassModel::ConsistentTranslational
        ).expect("four-times modulus");
        let dense = solve(
            BeamLineBoundaryCase::SimpleCantilever, 10, 4.0 * MU_FROM_GAMMA_KG_PER_M
        );
        let long = solve_beam_line_modes(
            1.7 * L_M, E_PA, 1.7_f64.powi(4) * I_M4,
            1.7_f64.powi(2) * MU_FROM_GAMMA_KG_PER_M, 10,
            BeamLineBoundaryCase::SimpleCantilever, BeamLineMassModel::ConsistentTranslational
        ).expect("geometrically similar beam");
        assert!(((stiff.modes[0].frequency_hz / base.modes[0].frequency_hz) - 2.0).abs() < 1.0e-8);
        assert!(((dense.modes[0].frequency_hz / base.modes[0].frequency_hz) - 0.5).abs() < 1.0e-8);
        assert!(((long.modes[0].frequency_hz / base.modes[0].frequency_hz) - 1.0 / 1.7).abs() < 1.0e-8);
    }

    #[test]
    fn malformed_inputs_and_element_counts_fail_closed() {
        assert!(solve_beam_line_modes(
            0.0, E_PA, I_M4, MU_FROM_GAMMA_KG_PER_M, 10,
            BeamLineBoundaryCase::SimpleCantilever, BeamLineMassModel::ConsistentTranslational
        ).is_err());
        assert!(solve_beam_line_modes(
            L_M, E_PA, I_M4, MU_FROM_GAMMA_KG_PER_M, 0,
            BeamLineBoundaryCase::SimpleCantilever, BeamLineMassModel::ConsistentTranslational
        ).is_err());
        assert!(solve_beam_line_modes(
            L_M, E_PA, I_M4, MU_FROM_GAMMA_KG_PER_M, 65,
            BeamLineBoundaryCase::SimpleCantilever, BeamLineMassModel::ConsistentTranslational
        ).is_err());
    }
}
