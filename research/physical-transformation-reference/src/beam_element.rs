// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local planar Euler-Bernoulli frame element with consistent translational
//! mass and an explicit rotary-inertia contribution.
//!
//! This is an element-matrix reference, not a global beam/modal solver.
//! Local DOF order is [u1, v1, theta1, u2, v2, theta2]. Shear deformation,
//! damping, geometric stiffness, nonlinear kinematics and joint-release
//! assembly are outside this module.

#![allow(clippy::needless_range_loop)] // Matrix assembly deliberately indexes coupled DOFs.

use crate::Check;

pub type Matrix6 = [[f64; 6]; 6];

/// Local stiffness and mass matrices for a uniform planar frame element.
/// The DOF order is [u1, v1, theta1, u2, v2, theta2]. Entries have mixed
/// dimensions because the generalized coordinates include length and angle.
#[derive(Clone, Debug, PartialEq)]
pub struct EulerBernoulliElementMatrices {
    /// Reference element length used in these local matrices.
    pub length_m: f64,
    /// Local stiffness matrix.
    pub stiffness_local: Matrix6,
    /// Total consistent mass matrix (translational plus rotary inertia).
    pub mass_local: Matrix6,
    /// Translational contribution only (axial and transverse).
    pub translational_mass_local: Matrix6,
    /// Rotary-inertia contribution in transverse-bending DOFs only.
    pub rotary_inertia_mass_local: Matrix6,
}

/// Global-axis matrices for a straight element whose end-coordinate delta
/// is explicitly provided. DOF order is [ux1, uy1, theta1, ux2, uy2, theta2].
#[derive(Clone, Debug, PartialEq)]
pub struct GlobalEulerBernoulliElementMatrices {
    pub length_m: f64,
    pub direction_cosine: f64,
    pub direction_sine: f64,
    pub stiffness_global: Matrix6,
    pub mass_global: Matrix6,
    pub translational_mass_global: Matrix6,
    pub rotary_inertia_mass_global: Matrix6,
}

/// Transform the local element matrices to global coordinates using the
/// supplied end-coordinate delta. The measured length must match the length
/// used to form the local matrices, otherwise the request is rejected.
pub fn transform_element_to_global(
    local: &EulerBernoulliElementMatrices,
    delta_x_m: f64,
    delta_y_m: f64,
) -> Check<GlobalEulerBernoulliElementMatrices> {
    if !delta_x_m.is_finite() || !delta_y_m.is_finite() {
        return Err("element coordinate deltas must be finite".to_owned());
    }
    let geometric_length = delta_x_m.hypot(delta_y_m);
    validate_positive("geometric_length_m", geometric_length)?;
    let length_scale = geometric_length.max(local.length_m);
    if (geometric_length - local.length_m).abs() > 1.0e-10 * length_scale {
        return Err("element geometry length does not match local matrix length".to_owned());
    }

    let cosine = delta_x_m / geometric_length;
    let sine = delta_y_m / geometric_length;
    let transform = coordinate_transform(cosine, sine);
    let stiffness = congruence_transform(&local.stiffness_local, &transform);
    let mass = congruence_transform(&local.mass_local, &transform);
    let translational_mass = congruence_transform(&local.translational_mass_local, &transform);
    let rotary_mass = congruence_transform(&local.rotary_inertia_mass_local, &transform);

    ensure_finite_matrix("stiffness_global", &stiffness)?;
    ensure_finite_matrix("mass_global", &mass)?;
    ensure_finite_matrix("translational_mass_global", &translational_mass)?;
    ensure_finite_matrix("rotary_inertia_mass_global", &rotary_mass)?;

    Ok(GlobalEulerBernoulliElementMatrices {
        length_m: geometric_length,
        direction_cosine: cosine,
        direction_sine: sine,
        stiffness_global: stiffness,
        mass_global: mass,
        translational_mass_global: translational_mass,
        rotary_inertia_mass_global: rotary_mass,
    })
}

fn coordinate_transform(cosine: f64, sine: f64) -> Matrix6 {
    [
        [cosine, sine, 0.0, 0.0, 0.0, 0.0],
        [-sine, cosine, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, cosine, sine, 0.0],
        [0.0, 0.0, 0.0, -sine, cosine, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    ]
}

fn transpose6(matrix: &Matrix6) -> Matrix6 {
    let mut transposed = [[0.0; 6]; 6];
    for row in 0..6 {
        for col in 0..6 {
            transposed[col][row] = matrix[row][col];
        }
    }
    transposed
}

fn multiply6(left: &Matrix6, right: &Matrix6) -> Matrix6 {
    let mut product = [[0.0; 6]; 6];
    for row in 0..6 {
        for inner in 0..6 {
            for col in 0..6 {
                product[row][col] += left[row][inner] * right[inner][col];
            }
        }
    }
    product
}

fn congruence_transform(local: &Matrix6, transform: &Matrix6) -> Matrix6 {
    multiply6(&multiply6(&transpose6(transform), local), transform)
}

fn matrix_vector_product(matrix: &Matrix6, vector: &[f64; 6]) -> [f64; 6] {
    let mut product = [0.0; 6];
    for row in 0..6 {
        for col in 0..6 {
            product[row] += matrix[row][col] * vector[col];
        }
    }
    product
}

fn matrix_infinity_norm(matrix: &Matrix6) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0_f64, f64::max)
}

fn vector_infinity_norm(vector: &[f64; 6]) -> f64 {
    vector
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
}

/// Build local matrices for a straight, uniform Euler-Bernoulli frame element.
///
/// Area and second moment are independent inputs and must describe the same
/// section. A circular section is not silently inferred. The consistent mass
/// includes cross-sectional rotary inertia, but omits shear deformation,
/// torsion and damping.
pub fn euler_bernoulli_frame_element(
    length_m: f64,
    area_m2: f64,
    youngs_modulus_pa: f64,
    density_kg_m3: f64,
    second_moment_m4: f64,
) -> Check<EulerBernoulliElementMatrices> {
    validate_positive("length_m", length_m)?;
    validate_positive("area_m2", area_m2)?;
    validate_positive("youngs_modulus_pa", youngs_modulus_pa)?;
    validate_positive("density_kg_m3", density_kg_m3)?;
    validate_positive("second_moment_m4", second_moment_m4)?;

    let mut stiffness = [[0.0; 6]; 6];
    let mut translational_mass = [[0.0; 6]; 6];
    let mut rotary_mass = [[0.0; 6]; 6];

    // Axial extension: EA/L * [[1,-1],[-1,1]], DOFs u1 and u2.
    let axial_stiffness = youngs_modulus_pa * area_m2 / length_m;
    add_two_dof_block(
        &mut stiffness,
        [0, 3],
        [
            [axial_stiffness, -axial_stiffness],
            [-axial_stiffness, axial_stiffness],
        ],
    );

    // Euler-Bernoulli bending stiffness in [v1, theta1, v2, theta2] order.
    let l2 = length_m * length_m;
    let l3 = l2 * length_m;
    let bending_factor = youngs_modulus_pa * second_moment_m4 / l3;
    let bending_stiffness = [
        [12.0, 6.0 * length_m, -12.0, 6.0 * length_m],
        [6.0 * length_m, 4.0 * l2, -6.0 * length_m, 2.0 * l2],
        [-12.0, -6.0 * length_m, 12.0, -6.0 * length_m],
        [6.0 * length_m, 2.0 * l2, -6.0 * length_m, 4.0 * l2],
    ];
    add_four_dof_block(
        &mut stiffness,
        [1, 2, 4, 5],
        scale_four(bending_stiffness, bending_factor),
    );

    // Consistent axial mass: rho*A*L/6 * [[2,1],[1,2]].
    let axial_mass_factor = density_kg_m3 * area_m2 * length_m / 6.0;
    add_two_dof_block(
        &mut translational_mass,
        [0, 3],
        [
            [2.0 * axial_mass_factor, axial_mass_factor],
            [axial_mass_factor, 2.0 * axial_mass_factor],
        ],
    );

    // Consistent transverse translational mass for Hermite interpolation.
    let transverse_mass_factor = density_kg_m3 * area_m2 * length_m / 420.0;
    let transverse_mass = [
        [156.0, 22.0 * length_m, 54.0, -13.0 * length_m],
        [22.0 * length_m, 4.0 * l2, 13.0 * length_m, -3.0 * l2],
        [54.0, 13.0 * length_m, 156.0, -22.0 * length_m],
        [-13.0 * length_m, -3.0 * l2, -22.0 * length_m, 4.0 * l2],
    ];
    add_four_dof_block(
        &mut translational_mass,
        [1, 2, 4, 5],
        scale_four(transverse_mass, transverse_mass_factor),
    );

    // Rotary inertia: integral rho*I*(dN/dx)^T*(dN/dx) dx for Hermite
    // bending functions. Keep it visible so a benchmark can audit its use.
    let rotary_mass_factor = density_kg_m3 * second_moment_m4 / (30.0 * length_m);
    let rotary_bending_mass = [
        [36.0, 3.0 * length_m, -36.0, 3.0 * length_m],
        [3.0 * length_m, 4.0 * l2, -3.0 * length_m, -l2],
        [-36.0, -3.0 * length_m, 36.0, -3.0 * length_m],
        [3.0 * length_m, -l2, -3.0 * length_m, 4.0 * l2],
    ];
    add_four_dof_block(
        &mut rotary_mass,
        [1, 2, 4, 5],
        scale_four(rotary_bending_mass, rotary_mass_factor),
    );

    let mass = add_matrices(&translational_mass, &rotary_mass);
    ensure_finite_matrix("stiffness_local", &stiffness)?;
    ensure_finite_matrix("translational_mass_local", &translational_mass)?;
    ensure_finite_matrix("rotary_inertia_mass_local", &rotary_mass)?;
    ensure_finite_matrix("mass_local", &mass)?;

    Ok(EulerBernoulliElementMatrices {
        length_m,
        stiffness_local: stiffness,
        mass_local: mass,
        translational_mass_local: translational_mass,
        rotary_inertia_mass_local: rotary_mass,
    })
}

fn validate_positive(name: &str, value: f64) -> Check<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{name} must be finite and positive"));
    }
    Ok(())
}

fn add_two_dof_block(matrix: &mut Matrix6, indices: [usize; 2], block: [[f64; 2]; 2]) {
    for row in 0..2 {
        for col in 0..2 {
            matrix[indices[row]][indices[col]] += block[row][col];
        }
    }
}

fn add_four_dof_block(matrix: &mut Matrix6, indices: [usize; 4], block: [[f64; 4]; 4]) {
    for row in 0..4 {
        for col in 0..4 {
            matrix[indices[row]][indices[col]] += block[row][col];
        }
    }
}

fn scale_four(matrix: [[f64; 4]; 4], factor: f64) -> [[f64; 4]; 4] {
    let mut scaled = [[0.0; 4]; 4];
    for row in 0..4 {
        for col in 0..4 {
            scaled[row][col] = matrix[row][col] * factor;
        }
    }
    scaled
}

fn add_matrices(left: &Matrix6, right: &Matrix6) -> Matrix6 {
    let mut result = [[0.0; 6]; 6];
    for row in 0..6 {
        for col in 0..6 {
            result[row][col] = left[row][col] + right[row][col];
        }
    }
    result
}

fn ensure_finite_matrix(name: &str, matrix: &Matrix6) -> Check<()> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(format!("{name} contains a non-finite entry"))
    }
}

#[cfg(test)]
mod tests {
    use super::euler_bernoulli_frame_element;

    const LENGTH_M: f64 = 2.0;
    const AREA_M2: f64 = 0.004;
    const YOUNGS_MODULUS_PA: f64 = 70.0e9;
    const DENSITY_KG_M3: f64 = 2700.0;
    const SECOND_MOMENT_M4: f64 = 2.0e-6;

    fn matrices() -> super::EulerBernoulliElementMatrices {
        euler_bernoulli_frame_element(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            SECOND_MOMENT_M4,
        )
        .expect("valid Euler-Bernoulli element")
    }

    fn close(actual: f64, expected: f64, relative_tolerance: f64, label: &str) {
        let scale = expected.abs().max(1.0);
        assert!(
            actual.is_finite() && (actual - expected).abs() <= relative_tolerance * scale,
            "{label}: actual={actual:.17e}, expected={expected:.17e}"
        );
    }

    #[test]
    fn stiffness_and_all_mass_contributions_are_symmetric() {
        let matrices = matrices();
        for row in 0..6 {
            for col in 0..6 {
                close(
                    matrices.stiffness_local[row][col],
                    matrices.stiffness_local[col][row],
                    1.0e-13,
                    "stiffness symmetry",
                );
                close(
                    matrices.translational_mass_local[row][col],
                    matrices.translational_mass_local[col][row],
                    1.0e-13,
                    "translational mass symmetry",
                );
                close(
                    matrices.rotary_inertia_mass_local[row][col],
                    matrices.rotary_inertia_mass_local[col][row],
                    1.0e-13,
                    "rotary mass symmetry",
                );
                close(
                    matrices.mass_local[row][col],
                    matrices.mass_local[col][row],
                    1.0e-13,
                    "total mass symmetry",
                );
            }
        }
    }

    #[test]
    fn axial_stiffness_and_consistent_mass_match_closed_forms() {
        let matrices = matrices();
        let axial_stiffness = YOUNGS_MODULUS_PA * AREA_M2 / LENGTH_M;
        let axial_mass = DENSITY_KG_M3 * AREA_M2 * LENGTH_M / 6.0;
        close(
            matrices.stiffness_local[0][0],
            axial_stiffness,
            1.0e-13,
            "axial stiffness diagonal",
        );
        close(
            matrices.stiffness_local[0][3],
            -axial_stiffness,
            1.0e-13,
            "axial stiffness coupling",
        );
        close(
            matrices.translational_mass_local[0][0],
            2.0 * axial_mass,
            1.0e-13,
            "axial mass diagonal",
        );
        close(
            matrices.translational_mass_local[0][3],
            axial_mass,
            1.0e-13,
            "axial mass coupling",
        );
    }

    #[test]
    fn bending_stiffness_matches_closed_form_entrywise() {
        let matrices = matrices();
        let factor = YOUNGS_MODULUS_PA * SECOND_MOMENT_M4 / LENGTH_M.powi(3);
        let expected = [
            [12.0, 6.0 * LENGTH_M, -12.0, 6.0 * LENGTH_M],
            [
                6.0 * LENGTH_M,
                4.0 * LENGTH_M.powi(2),
                -6.0 * LENGTH_M,
                2.0 * LENGTH_M.powi(2),
            ],
            [-12.0, -6.0 * LENGTH_M, 12.0, -6.0 * LENGTH_M],
            [
                6.0 * LENGTH_M,
                2.0 * LENGTH_M.powi(2),
                -6.0 * LENGTH_M,
                4.0 * LENGTH_M.powi(2),
            ],
        ];
        let indices = [1, 2, 4, 5];
        for row in 0..4 {
            for col in 0..4 {
                close(
                    matrices.stiffness_local[indices[row]][indices[col]],
                    expected[row][col] * factor,
                    1.0e-13,
                    "bending stiffness entry",
                );
            }
        }
    }

    #[test]
    fn released_root_cantilever_tip_compliance_matches_analytic_solution() {
        let matrices = matrices();
        // Root DOFs are fixed. The free tip bending block is:
        // [12EI/L^3, -6EI/L^2; -6EI/L^2, 4EI/L].
        let k11 = matrices.stiffness_local[4][4];
        let k12 = matrices.stiffness_local[4][5];
        let k22 = matrices.stiffness_local[5][5];
        let determinant = k11 * k22 - k12 * k12;
        let tip_load_n = 100.0;
        let tip_displacement_m = k22 * tip_load_n / determinant;
        let tip_rotation_rad = -k12 * tip_load_n / determinant;
        let expected_displacement =
            tip_load_n * LENGTH_M.powi(3) / (3.0 * YOUNGS_MODULUS_PA * SECOND_MOMENT_M4);
        let expected_rotation =
            tip_load_n * LENGTH_M.powi(2) / (2.0 * YOUNGS_MODULUS_PA * SECOND_MOMENT_M4);
        close(
            tip_displacement_m,
            expected_displacement,
            1.0e-12,
            "cantilever tip displacement",
        );
        close(
            tip_rotation_rad,
            expected_rotation,
            1.0e-12,
            "cantilever tip rotation",
        );
    }

    #[test]
    fn transverse_translational_mass_matches_closed_form_entrywise() {
        // Independent Hermite-shape integral: integral rho*A*N^T*N dx.
        let matrices = matrices();
        let factor = DENSITY_KG_M3 * AREA_M2 * LENGTH_M / 420.0;
        let l2 = LENGTH_M.powi(2);
        let expected = [
            [156.0, 22.0 * LENGTH_M, 54.0, -13.0 * LENGTH_M],
            [22.0 * LENGTH_M, 4.0 * l2, 13.0 * LENGTH_M, -3.0 * l2],
            [54.0, 13.0 * LENGTH_M, 156.0, -22.0 * LENGTH_M],
            [-13.0 * LENGTH_M, -3.0 * l2, -22.0 * LENGTH_M, 4.0 * l2],
        ];
        let indices = [1, 2, 4, 5];
        for row in 0..4 {
            for col in 0..4 {
                close(
                    matrices.translational_mass_local[indices[row]][indices[col]],
                    expected[row][col] * factor,
                    1.0e-13,
                    "transverse translational mass entry",
                );
            }
        }
    }

    #[test]
    fn rotary_inertia_mass_matches_closed_form_entrywise() {
        // Independent derivative-shape integral:
        // integral rho*I*(dN/dx)^T*(dN/dx) dx.
        let matrices = matrices();
        let factor = DENSITY_KG_M3 * SECOND_MOMENT_M4 / (30.0 * LENGTH_M);
        let l2 = LENGTH_M.powi(2);
        let expected = [
            [36.0, 3.0 * LENGTH_M, -36.0, 3.0 * LENGTH_M],
            [3.0 * LENGTH_M, 4.0 * l2, -3.0 * LENGTH_M, -l2],
            [-36.0, -3.0 * LENGTH_M, 36.0, -3.0 * LENGTH_M],
            [3.0 * LENGTH_M, -l2, -3.0 * LENGTH_M, 4.0 * l2],
        ];
        let indices = [1, 2, 4, 5];
        for row in 0..4 {
            for col in 0..4 {
                close(
                    matrices.rotary_inertia_mass_local[indices[row]][indices[col]],
                    expected[row][col] * factor,
                    1.0e-13,
                    "rotary-inertia mass entry",
                );
            }
        }
    }

    #[test]
    fn rotary_inertia_is_separate_and_scales_with_density_and_second_moment() {
        let base = matrices();
        let doubled_density = euler_bernoulli_frame_element(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            2.0 * DENSITY_KG_M3,
            SECOND_MOMENT_M4,
        )
        .expect("doubled density element");
        let doubled_inertia = euler_bernoulli_frame_element(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            2.0 * SECOND_MOMENT_M4,
        )
        .expect("doubled second-moment element");
        let base_rotary = base.rotary_inertia_mass_local[2][2];
        close(
            doubled_density.rotary_inertia_mass_local[2][2],
            2.0 * base_rotary,
            1.0e-13,
            "rotary mass density scaling",
        );
        close(
            doubled_inertia.rotary_inertia_mass_local[2][2],
            2.0 * base_rotary,
            1.0e-13,
            "rotary mass second-moment scaling",
        );
        close(
            base_rotary,
            DENSITY_KG_M3 * SECOND_MOMENT_M4 * 4.0 * LENGTH_M / 30.0,
            1.0e-13,
            "rotary mass closed form",
        );
        close(
            base.mass_local[2][2],
            base.translational_mass_local[2][2] + base.rotary_inertia_mass_local[2][2],
            1.0e-13,
            "total rotational DOF mass",
        );
    }

    #[test]
    fn horizontal_transform_is_the_identity() {
        let local = matrices();
        let global = super::transform_element_to_global(&local, LENGTH_M, 0.0)
            .expect("horizontal transform");
        assert_eq!(global.stiffness_global, local.stiffness_local);
        assert_eq!(global.mass_global, local.mass_local);
        assert_eq!(
            global.translational_mass_global,
            local.translational_mass_local
        );
        assert_eq!(
            global.rotary_inertia_mass_global,
            local.rotary_inertia_mass_local
        );
        assert_eq!(global.direction_cosine, 1.0);
        assert_eq!(global.direction_sine, 0.0);
    }

    #[test]
    fn inclined_transform_preserves_energy_and_matrix_symmetry() {
        let local = matrices();
        let angle_rad = 0.713;
        let cosine = angle_rad.cos();
        let sine = angle_rad.sin();
        let dx = LENGTH_M * cosine;
        let dy = LENGTH_M * sine;
        let global =
            super::transform_element_to_global(&local, dx, dy).expect("inclined transform");
        let transform = super::coordinate_transform(cosine, sine);
        let q_global = [0.003, -0.008, 0.02, 0.011, 0.004, -0.013];
        let q_local = super::matrix_vector_product(&transform, &q_global);
        let local_force = super::matrix_vector_product(&local.stiffness_local, &q_local);
        let global_force = super::matrix_vector_product(&global.stiffness_global, &q_global);
        let local_energy = q_local
            .iter()
            .zip(local_force)
            .map(|(q, force)| q * force)
            .sum::<f64>();
        let global_energy = q_global
            .iter()
            .zip(global_force)
            .map(|(q, force)| q * force)
            .sum::<f64>();
        close(
            local_energy,
            global_energy,
            1.0e-12,
            "coordinate-invariant strain energy",
        );

        for row in 0..6 {
            for col in 0..6 {
                close(
                    global.stiffness_global[row][col],
                    global.stiffness_global[col][row],
                    1.0e-12,
                    "global stiffness symmetry",
                );
                close(
                    global.mass_global[row][col],
                    global.mass_global[col][row],
                    1.0e-12,
                    "global mass symmetry",
                );
            }
        }
    }

    #[test]
    fn rigid_body_translation_and_rotation_have_zero_internal_force() {
        let local = matrices();
        let angle_rad = 0.713;
        let dx = LENGTH_M * angle_rad.cos();
        let dy = LENGTH_M * angle_rad.sin();
        let global =
            super::transform_element_to_global(&local, dx, dy).expect("inclined transform");
        let translations = [[0.4, -0.7, 0.0, 0.4, -0.7, 0.0]];
        let rotation = 0.015;
        let rotations = [[0.0, 0.0, rotation, -rotation * dy, rotation * dx, rotation]];

        for q in translations.iter().chain(rotations.iter()) {
            let force = super::matrix_vector_product(&global.stiffness_global, q);
            let normalized_residual = force
                .iter()
                .map(|value| value.abs())
                .fold(0.0_f64, f64::max)
                / (super::matrix_infinity_norm(&global.stiffness_global)
                    * super::vector_infinity_norm(q));
            assert!(
                normalized_residual < 1.0e-12,
                "rigid-body mode has normalized stiffness residual {normalized_residual:.17e}"
            );
        }
    }

    #[test]
    fn transform_rejects_geometry_length_mismatch() {
        let local = matrices();
        assert!(super::transform_element_to_global(&local, LENGTH_M * 1.01, 0.0).is_err());
        assert!(super::transform_element_to_global(&local, f64::NAN, 0.0).is_err());
    }

    #[test]
    fn invalid_domains_fail_closed() {
        assert!(
            euler_bernoulli_frame_element(
                0.0,
                AREA_M2,
                YOUNGS_MODULUS_PA,
                DENSITY_KG_M3,
                SECOND_MOMENT_M4,
            )
            .is_err()
        );
        assert!(
            euler_bernoulli_frame_element(
                LENGTH_M,
                AREA_M2,
                YOUNGS_MODULUS_PA,
                DENSITY_KG_M3,
                0.0,
            )
            .is_err()
        );
        assert!(
            euler_bernoulli_frame_element(
                LENGTH_M,
                AREA_M2,
                f64::INFINITY,
                DENSITY_KG_M3,
                SECOND_MOMENT_M4,
            )
            .is_err()
        );
    }
}
