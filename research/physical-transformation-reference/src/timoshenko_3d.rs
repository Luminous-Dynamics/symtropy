// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local 3D Timoshenko beam element with explicit shear flexibility and
//! lumped translational + rotary-inertia mass.
//!
//! Local DOF order per node is [ux, uy, uz, rx, ry, rz], with node 2 offset
//! by six indices. Shear areas are effective values (including any correction
//! factor). The torsional stiffness constant J is distinct from the polar
//! section second moment Ip = Iy + Iz used for rotary mass about the beam axis.
//!
//! This module builds local element matrices only. It does not transform axes,
//! assemble a structure, model offsets/point masses, or claim FV4/FV5 coverage.

#![allow(clippy::needless_range_loop)] // Dense 12-DOF block insertion deliberately couples indexed coordinates.

use crate::Check;

pub type Matrix12 = [[f64; 12]; 12];

/// Explicit properties for one straight, uniform 3D Timoshenko beam element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timoshenko3dParameters {
    pub length_m: f64,
    pub area_m2: f64,
    pub second_moment_y_m4: f64,
    pub second_moment_z_m4: f64,
    /// Saint-Venant torsional constant, not the polar mass second moment.
    pub torsional_constant_m4: f64,
    /// Effective shear area for local-y shear / bending about local z.
    pub shear_area_y_m2: f64,
    /// Effective shear area for local-z shear / bending about local y.
    pub shear_area_z_m2: f64,
    pub youngs_modulus_pa: f64,
    pub poisson_ratio: f64,
    pub density_kg_m3: f64,
}

/// Local stiffness and a declared lumped mass model for a 12-DOF element.
#[derive(Clone, Debug, PartialEq)]
pub struct Timoshenko3dElementMatrices {
    pub length_m: f64,
    pub shear_modulus_pa: f64,
    /// Timoshenko shear parameter for local-y transverse displacement.
    pub shear_parameter_z: f64,
    /// Timoshenko shear parameter for local-z transverse displacement.
    pub shear_parameter_y: f64,
    pub stiffness_local: Matrix12,
    /// Diagonal translational lumped mass: rho*A*L/2 per translational DOF.
    pub translational_mass_lumped: Matrix12,
    /// Diagonal cross-sectional rotary inertia; I_p=I_y+I_z about local x.
    pub rotary_inertia_mass_lumped: Matrix12,
    /// Sum of the two declared lumped mass contributions.
    pub mass_lumped: Matrix12,
}

/// Construct the local 3D Timoshenko stiffness matrix and lumped mass terms.
///
/// Bending shear parameters are phi = 12*E*I/(G*A_s*L^2). The mass choice is
/// explicit: translational mass is lumped equally to both nodes, and rotary
/// inertia is lumped from I_y, I_z and I_p=I_y+I_z. No cross-section is inferred
/// from an external benchmark annotation. Consistent mass and axis transforms
/// remain separate implementation/model choices.
pub fn timoshenko_3d_element(
    parameters: Timoshenko3dParameters,
) -> Check<Timoshenko3dElementMatrices> {
    validate_parameters(parameters)?;

    let length = parameters.length_m;
    let area = parameters.area_m2;
    let youngs_modulus = parameters.youngs_modulus_pa;
    let density = parameters.density_kg_m3;
    let shear_modulus = youngs_modulus / (2.0 * (1.0 + parameters.poisson_ratio));
    validate_positive("shear_modulus_pa", shear_modulus)?;

    let length_squared = length * length;
    let length_cubed = length_squared * length;
    let phi_z = 12.0 * youngs_modulus * parameters.second_moment_z_m4
        / (shear_modulus * parameters.shear_area_y_m2 * length_squared);
    let phi_y = 12.0 * youngs_modulus * parameters.second_moment_y_m4
        / (shear_modulus * parameters.shear_area_z_m2 * length_squared);
    validate_positive("shear_parameter_z", phi_z)?;
    validate_positive("shear_parameter_y", phi_y)?;

    let mut stiffness = [[0.0; 12]; 12];
    let mut translational_mass = [[0.0; 12]; 12];
    let mut rotary_mass = [[0.0; 12]; 12];

    let axial_stiffness = youngs_modulus * area / length;
    add_two_dof_block(
        &mut stiffness,
        [0, 6],
        [
            [axial_stiffness, -axial_stiffness],
            [-axial_stiffness, axial_stiffness],
        ],
    );

    let torsional_stiffness =
        shear_modulus * parameters.torsional_constant_m4 / length;
    add_two_dof_block(
        &mut stiffness,
        [3, 9],
        [
            [torsional_stiffness, -torsional_stiffness],
            [-torsional_stiffness, torsional_stiffness],
        ],
    );

    // Local-y bending with nodal [uy, rz] pairs; phi_z uses effective shear area Ay.
    let factor_z = youngs_modulus * parameters.second_moment_z_m4
        / (length_cubed * (1.0 + phi_z));
    let bending_about_z = [
        [12.0, 6.0 * length, -12.0, 6.0 * length],
        [
            6.0 * length,
            (4.0 + phi_z) * length_squared,
            -6.0 * length,
            (2.0 - phi_z) * length_squared,
        ],
        [-12.0, -6.0 * length, 12.0, -6.0 * length],
        [
            6.0 * length,
            (2.0 - phi_z) * length_squared,
            -6.0 * length,
            (4.0 + phi_z) * length_squared,
        ],
    ];
    add_four_dof_block(
        &mut stiffness,
        [1, 5, 7, 11],
        scale_four(bending_about_z, factor_z),
    );

    // Local-z bending with nodal [uz, ry] pairs; the sign convention differs.
    let factor_y = youngs_modulus * parameters.second_moment_y_m4
        / (length_cubed * (1.0 + phi_y));
    let bending_about_y = [
        [12.0, -6.0 * length, -12.0, -6.0 * length],
        [
            -6.0 * length,
            (4.0 + phi_y) * length_squared,
            6.0 * length,
            (2.0 - phi_y) * length_squared,
        ],
        [-12.0, 6.0 * length, 12.0, 6.0 * length],
        [
            -6.0 * length,
            (2.0 - phi_y) * length_squared,
            6.0 * length,
            (4.0 + phi_y) * length_squared,
        ],
    ];
    add_four_dof_block(
        &mut stiffness,
        [2, 4, 8, 10],
        scale_four(bending_about_y, factor_y),
    );

    // Lumped translational inertia is mass/2 at each endpoint and in each
    // translational DOF; each node receives the same total translational mass.
    let half_mass = density * area * length / 2.0;
    for index in [0, 1, 2, 6, 7, 8] {
        translational_mass[index][index] = half_mass;
    }

    // Rotary inertia is a distinct diagonal contribution. About local x,
    // polar second moment Ip equals Iy+Iz; it is not generally the torsional
    // stiffness constant J (e.g. for a rectangular/square section).
    let polar_second_moment = parameters.second_moment_y_m4
        + parameters.second_moment_z_m4;
    validate_positive("polar_second_moment_m4", polar_second_moment)?;
    rotary_mass[3][3] = density * polar_second_moment * length / 2.0;
    rotary_mass[9][9] = rotary_mass[3][3];
    rotary_mass[4][4] = density * parameters.second_moment_y_m4 * length / 2.0;
    rotary_mass[10][10] = rotary_mass[4][4];
    rotary_mass[5][5] = density * parameters.second_moment_z_m4 * length / 2.0;
    rotary_mass[11][11] = rotary_mass[5][5];

    let mass = add_matrices(&translational_mass, &rotary_mass);
    ensure_finite_matrix("stiffness_local", &stiffness)?;
    ensure_finite_matrix("translational_mass_lumped", &translational_mass)?;
    ensure_finite_matrix("rotary_inertia_mass_lumped", &rotary_mass)?;
    ensure_finite_matrix("mass_lumped", &mass)?;

    for (name, matrix) in [
        ("stiffness_local", &stiffness),
        ("translational_mass_lumped", &translational_mass),
        ("rotary_inertia_mass_lumped", &rotary_mass),
        ("mass_lumped", &mass),
    ] {
        ensure_symmetric(name, matrix, 1.0e-12)?;
    }

    Ok(Timoshenko3dElementMatrices {
        length_m: length,
        shear_modulus_pa: shear_modulus,
        shear_parameter_z: phi_z,
        shear_parameter_y: phi_y,
        stiffness_local: stiffness,
        translational_mass_lumped: translational_mass,
        rotary_inertia_mass_lumped: rotary_mass,
        mass_lumped: mass,
    })
}

fn validate_parameters(parameters: Timoshenko3dParameters) -> Check<()> {
    for (name, value) in [
        ("length_m", parameters.length_m),
        ("area_m2", parameters.area_m2),
        ("second_moment_y_m4", parameters.second_moment_y_m4),
        ("second_moment_z_m4", parameters.second_moment_z_m4),
        ("torsional_constant_m4", parameters.torsional_constant_m4),
        ("shear_area_y_m2", parameters.shear_area_y_m2),
        ("shear_area_z_m2", parameters.shear_area_z_m2),
        ("youngs_modulus_pa", parameters.youngs_modulus_pa),
        ("density_kg_m3", parameters.density_kg_m3),
    ] {
        validate_positive(name, value)?;
    }
    if !parameters.poisson_ratio.is_finite()
        || parameters.poisson_ratio <= -1.0
        || parameters.poisson_ratio >= 0.5
    {
        return Err("poisson_ratio must be finite and inside (-1, 0.5)".to_owned());
    }
    Ok(())
}

fn validate_positive(name: &str, value: f64) -> Check<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be finite and positive"))
    }
}

fn add_two_dof_block(
    matrix: &mut Matrix12,
    indices: [usize; 2],
    block: [[f64; 2]; 2],
) {
    for row in 0..2 {
        for col in 0..2 {
            matrix[indices[row]][indices[col]] += block[row][col];
        }
    }
}

fn add_four_dof_block(
    matrix: &mut Matrix12,
    indices: [usize; 4],
    block: [[f64; 4]; 4],
) {
    for row in 0..4 {
        for col in 0..4 {
            matrix[indices[row]][indices[col]] += block[row][col];
        }
    }
}

fn scale_four(matrix: [[f64; 4]; 4], factor: f64) -> [[f64; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for row in 0..4 {
        for col in 0..4 {
            result[row][col] = matrix[row][col] * factor;
        }
    }
    result
}

fn add_matrices(left: &Matrix12, right: &Matrix12) -> Matrix12 {
    let mut result = [[0.0; 12]; 12];
    for row in 0..12 {
        for col in 0..12 {
            result[row][col] = left[row][col] + right[row][col];
        }
    }
    result
}

fn ensure_finite_matrix(name: &str, matrix: &Matrix12) -> Check<()> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(format!("{name} contains a non-finite entry"))
    }
}

fn ensure_symmetric(name: &str, matrix: &Matrix12, tolerance: f64) -> Check<()> {
    for row in 0..12 {
        for col in 0..12 {
            let left = matrix[row][col];
            let right = matrix[col][row];
            let scale = left.abs().max(right.abs()).max(1.0);
            if (left - right).abs() > tolerance * scale {
                return Err(format!("{name} is not symmetric at ({row}, {col})"));
            }
        }
    }
    Ok(())
}

fn matrix_vector_product(matrix: &Matrix12, vector: &[f64; 12]) -> [f64; 12] {
    let mut result = [0.0; 12];
    for row in 0..12 {
        for col in 0..12 {
            result[row] += matrix[row][col] * vector[col];
        }
    }
    result
}

fn matrix_infinity_norm(matrix: &Matrix12) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0_f64, f64::max)
}

#[cfg(test)]
mod tests {
    use super::{Timoshenko3dParameters, timoshenko_3d_element};

    fn parameters() -> Timoshenko3dParameters {
        Timoshenko3dParameters {
            length_m: 3.5,
            area_m2: 0.016,
            second_moment_y_m4: 1.2e-4,
            second_moment_z_m4: 8.0e-5,
            torsional_constant_m4: 1.0e-4,
            shear_area_y_m2: 0.012,
            shear_area_z_m2: 0.010,
            youngs_modulus_pa: 200.0e9,
            poisson_ratio: 0.3,
            density_kg_m3: 8000.0,
        }
    }

    fn close(actual: f64, expected: f64, relative_tolerance: f64, label: &str) {
        let scale = expected.abs().max(1.0);
        assert!(
            actual.is_finite() && (actual - expected).abs() <= relative_tolerance * scale,
            "{label}: actual={actual:.17e}, expected={expected:.17e}"
        );
    }

    #[test]
    fn axial_torsional_and_both_bending_blocks_match_closed_forms() {
        let p = parameters();
        let element = timoshenko_3d_element(p).expect("valid Timoshenko element");
        let g = p.youngs_modulus_pa / (2.0 * (1.0 + p.poisson_ratio));
        let axial = p.youngs_modulus_pa * p.area_m2 / p.length_m;
        let torsion = g * p.torsional_constant_m4 / p.length_m;
        let phi_z = 12.0 * p.youngs_modulus_pa * p.second_moment_z_m4
            / (g * p.shear_area_y_m2 * p.length_m.powi(2));
        let phi_y = 12.0 * p.youngs_modulus_pa * p.second_moment_y_m4
            / (g * p.shear_area_z_m2 * p.length_m.powi(2));
        let factor_z = p.youngs_modulus_pa * p.second_moment_z_m4
            / (p.length_m.powi(3) * (1.0 + phi_z));
        let factor_y = p.youngs_modulus_pa * p.second_moment_y_m4
            / (p.length_m.powi(3) * (1.0 + phi_y));

        close(element.stiffness_local[0][0], axial, 1.0e-13, "axial EA/L");
        close(element.stiffness_local[0][6], -axial, 1.0e-13, "axial coupling");
        close(element.stiffness_local[3][3], torsion, 1.0e-13, "torsion GJ/L");
        close(element.stiffness_local[3][9], -torsion, 1.0e-13, "torsion coupling");
        close(element.stiffness_local[1][1], 12.0 * factor_z, 1.0e-13, "z-bending diagonal");
        close(element.stiffness_local[1][5], 6.0 * p.length_m * factor_z, 1.0e-13, "z-bending coupling");
        close(element.stiffness_local[2][2], 12.0 * factor_y, 1.0e-13, "y-bending diagonal");
        close(element.stiffness_local[2][4], -6.0 * p.length_m * factor_y, 1.0e-13, "y-bending coupling");
        close(element.shear_parameter_z, phi_z, 1.0e-13, "z shear parameter");
        close(element.shear_parameter_y, phi_y, 1.0e-13, "y shear parameter");
    }

    #[test]
    fn all_six_rigid_body_states_have_zero_internal_force() {
        let p = parameters();
        let element = timoshenko_3d_element(p).expect("valid Timoshenko element");
        let mut states = [[0.0; 12]; 6];
        states[0][0] = 1.0;
        states[0][6] = 1.0;
        states[1][1] = 1.0;
        states[1][7] = 1.0;
        states[2][2] = 1.0;
        states[2][8] = 1.0;
        states[3][3] = 1.0;
        states[3][9] = 1.0;
        let rotation_y = 0.02;
        states[4][4] = rotation_y;
        states[4][10] = rotation_y;
        states[4][8] = -rotation_y * p.length_m;
        let rotation_z = -0.017;
        states[5][5] = rotation_z;
        states[5][11] = rotation_z;
        states[5][7] = rotation_z * p.length_m;

        let matrix_norm = super::matrix_infinity_norm(&element.stiffness_local);
        for (index, state) in states.iter().enumerate() {
            let force = super::matrix_vector_product(&element.stiffness_local, state);
            let residual = force.iter().map(|value| value.abs()).fold(0.0_f64, f64::max)
                / (matrix_norm * state.iter().map(|value| value.abs()).fold(0.0_f64, f64::max));
            assert!(
                residual < 1.0e-12,
                "rigid-body state {index} has normalized force residual {residual:.17e}"
            );
        }
    }

    #[test]
    fn greater_effective_shear_area_approaches_euler_bernoulli_bending_limit() {
        let base = parameters();
        let normal = timoshenko_3d_element(base).expect("base element");
        let mut soft = base;
        soft.shear_area_y_m2 *= 0.1;
        let soft = timoshenko_3d_element(soft).expect("soft shear element");
        let mut stiff = base;
        stiff.shear_area_y_m2 *= 1.0e10;
        let stiff = timoshenko_3d_element(stiff).expect("high shear-area element");
        assert!(soft.stiffness_local[1][1] < normal.stiffness_local[1][1]);
        assert!(normal.stiffness_local[1][1] < stiff.stiffness_local[1][1]);
        let eb_limit = 12.0 * base.youngs_modulus_pa * base.second_moment_z_m4
            / base.length_m.powi(3);
        close(stiff.stiffness_local[1][1], eb_limit, 1.0e-8, "Euler-Bernoulli limit");
    }

    #[test]
    fn lumped_translational_and_rotary_mass_are_explicit_and_scale_correctly() {
        let p = parameters();
        let element = timoshenko_3d_element(p).expect("base element");
        let half_mass = p.density_kg_m3 * p.area_m2 * p.length_m / 2.0;
        let polar = p.second_moment_y_m4 + p.second_moment_z_m4;
        close(element.translational_mass_lumped[0][0], half_mass, 1.0e-13, "half translational mass");
        close(element.rotary_inertia_mass_lumped[3][3], p.density_kg_m3 * polar * p.length_m / 2.0, 1.0e-13, "polar rotary inertia");
        close(element.rotary_inertia_mass_lumped[4][4], p.density_kg_m3 * p.second_moment_y_m4 * p.length_m / 2.0, 1.0e-13, "y rotary inertia");
        close(element.rotary_inertia_mass_lumped[5][5], p.density_kg_m3 * p.second_moment_z_m4 * p.length_m / 2.0, 1.0e-13, "z rotary inertia");
        for row in 0..12 {
            for col in 0..12 {
                close(
                    element.mass_lumped[row][col],
                    element.translational_mass_lumped[row][col]
                        + element.rotary_inertia_mass_lumped[row][col],
                    1.0e-13,
                    "mass contribution sum",
                );
            }
        }
        let mut doubled_density = p;
        doubled_density.density_kg_m3 *= 2.0;
        let doubled = timoshenko_3d_element(doubled_density).expect("twice density");
        close(
            doubled.mass_lumped[3][3],
            2.0 * element.mass_lumped[3][3],
            1.0e-13,
            "density rotary mass scaling",
        );
    }

    #[test]
    fn local_stiffness_and_mass_matrices_are_symmetric() {
        let element = timoshenko_3d_element(parameters()).expect("valid element");
        for row in 0..12 {
            for col in 0..12 {
                close(element.stiffness_local[row][col], element.stiffness_local[col][row], 1.0e-13, "stiffness symmetry");
                close(element.translational_mass_lumped[row][col], element.translational_mass_lumped[col][row], 1.0e-13, "translational mass symmetry");
                close(element.rotary_inertia_mass_lumped[row][col], element.rotary_inertia_mass_lumped[col][row], 1.0e-13, "rotary mass symmetry");
                close(element.mass_lumped[row][col], element.mass_lumped[col][row], 1.0e-13, "total mass symmetry");
            }
        }
    }

    #[test]
    fn invalid_domains_fail_closed() {
        let p = parameters();
        let mut invalid = p;
        invalid.poisson_ratio = 0.5;
        assert!(timoshenko_3d_element(invalid).is_err());
        invalid = p;
        invalid.shear_area_y_m2 = 0.0;
        assert!(timoshenko_3d_element(invalid).is_err());
        invalid = p;
        invalid.torsional_constant_m4 = f64::NAN;
        assert!(timoshenko_3d_element(invalid).is_err());
        invalid = p;
        invalid.second_moment_z_m4 = -1.0;
        assert!(timoshenko_3d_element(invalid).is_err());
    }
}
