// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local static Euler–Bernoulli and Timoshenko comparison primitives.
//!
//! This module adds a 2-node Timoshenko *stiffness* matrix only. It does not
//! silently borrow Euler–Bernoulli inertia: a later dynamic Timoshenko model
//! must declare translational/rotary inertia, interpolation and locking
//! treatment independently.

use crate::beam_element::{Matrix6, euler_bernoulli_frame_element};
use crate::Check;

/// Local Timoshenko frame-element stiffness in DOF order
/// [u1, v1, theta1, u2, v2, theta2].
#[derive(Clone, Debug, PartialEq)]
pub struct TimoshenkoElementStiffness {
    pub length_m: f64,
    pub shear_correction_factor: f64,
    /// Dimensionless shear-flexibility parameter phi = 12 E I / (kappa G A L^2).
    pub shear_flexibility_phi: f64,
    pub stiffness_local: Matrix6,
}

/// Build the local Timoshenko frame stiffness matrix.
///
/// All values are SI units. The shear correction factor is an explicit input:
/// the implementation does not infer it from the shape or silently substitute
/// a rectangular-section value. No mass matrix is returned by this static
/// element primitive.
pub fn timoshenko_frame_element_stiffness(
    length_m: f64,
    area_m2: f64,
    youngs_modulus_pa: f64,
    shear_modulus_pa: f64,
    second_moment_m4: f64,
    shear_correction_factor: f64,
) -> Check<TimoshenkoElementStiffness> {
    validate_positive("length_m", length_m)?;
    validate_positive("area_m2", area_m2)?;
    validate_positive("youngs_modulus_pa", youngs_modulus_pa)?;
    validate_positive("shear_modulus_pa", shear_modulus_pa)?;
    validate_positive("second_moment_m4", second_moment_m4)?;
    validate_positive("shear_correction_factor", shear_correction_factor)?;

    let l2 = length_m * length_m;
    let l3 = l2 * length_m;
    let shear_flexibility_phi = 12.0 * youngs_modulus_pa * second_moment_m4
        / (shear_correction_factor * shear_modulus_pa * area_m2 * l2);
    if !shear_flexibility_phi.is_finite() || shear_flexibility_phi < 0.0 {
        return Err("Timoshenko shear flexibility must be finite and non-negative".to_owned());
    }

    let mut stiffness = [[0.0; 6]; 6];
    let axial_stiffness = youngs_modulus_pa * area_m2 / length_m;
    add_two_dof_block(
        &mut stiffness,
        [0, 3],
        [
            [axial_stiffness, -axial_stiffness],
            [-axial_stiffness, axial_stiffness],
        ],
    );

    // Two-node Timoshenko bending/shear stiffness, in [v1,theta1,v2,theta2].
    // This form retains shear flexibility through phi and converges to the
    // standard Euler-Bernoulli matrix as phi approaches zero.
    let factor = youngs_modulus_pa * second_moment_m4 / (l3 * (1.0 + shear_flexibility_phi));
    let bending_shear = [
        [12.0, 6.0 * length_m, -12.0, 6.0 * length_m],
        [
            6.0 * length_m,
            (4.0 + shear_flexibility_phi) * l2,
            -6.0 * length_m,
            (2.0 - shear_flexibility_phi) * l2,
        ],
        [-12.0, -6.0 * length_m, 12.0, -6.0 * length_m],
        [
            6.0 * length_m,
            (2.0 - shear_flexibility_phi) * l2,
            -6.0 * length_m,
            (4.0 + shear_flexibility_phi) * l2,
        ],
    ];
    add_four_dof_block(
        &mut stiffness,
        [1, 2, 4, 5],
        scale_four(bending_shear, factor),
    );

    if stiffness.iter().flatten().any(|value| !value.is_finite()) {
        return Err("Timoshenko stiffness matrix contains a non-finite entry".to_owned());
    }
    Ok(TimoshenkoElementStiffness {
        length_m,
        shear_correction_factor,
        shear_flexibility_phi,
        stiffness_local: stiffness,
    })
}

/// Construct the same local Euler-Bernoulli stiffness model for a controlled
/// matrix-by-matrix comparison. The placeholder density is required by the
/// shared constructor but does not affect the returned stiffness.
pub fn euler_bernoulli_stiffness_for_comparison(
    length_m: f64,
    area_m2: f64,
    youngs_modulus_pa: f64,
    second_moment_m4: f64,
) -> Check<Matrix6> {
    Ok(euler_bernoulli_frame_element(
        length_m,
        area_m2,
        youngs_modulus_pa,
        1.0,
        second_moment_m4,
    )?
    .stiffness_local)
}

fn validate_positive(name: &str, value: f64) -> Check<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be finite and positive"))
    }
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

#[cfg(test)]
mod tests {
    use super::{
        euler_bernoulli_stiffness_for_comparison, timoshenko_frame_element_stiffness,
    };

    const LENGTH_M: f64 = 1.0;
    const AREA_M2: f64 = 1.0e-4;
    const YOUNGS_MODULUS_PA: f64 = 200.0e9;
    const SHEAR_MODULUS_PA: f64 = 80.0e9;
    const SECOND_MOMENT_M4: f64 = 1.0e-8;
    const SHEAR_CORRECTION: f64 = 1.0;

    fn close(actual: f64, expected: f64, tolerance: f64, label: &str) {
        let scale = expected.abs().max(1.0);
        assert!(
            actual.is_finite() && (actual - expected).abs() <= tolerance * scale,
            "{label}: actual={actual:.17e}, expected={expected:.17e}"
        );
    }

    fn cantilever_tip_response(
        stiffness: &[[f64; 6]; 6],
        tip_load_n: f64,
    ) -> (f64, f64) {
        // Fixed root leaves only [v2, theta2] in the bending subproblem.
        let k11 = stiffness[4][4];
        let k12 = stiffness[4][5];
        let k22 = stiffness[5][5];
        let determinant = k11 * k22 - k12 * k12;
        assert!(determinant.is_finite() && determinant > 0.0);
        (
            k22 * tip_load_n / determinant,
            -k12 * tip_load_n / determinant,
        )
    }

    #[test]
    fn local_timoshenko_stiffness_is_symmetric_and_finite() {
        let result = timoshenko_frame_element_stiffness(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            SHEAR_MODULUS_PA,
            SECOND_MOMENT_M4,
            SHEAR_CORRECTION,
        )
        .expect("valid Timoshenko element");
        for row in 0..6 {
            for col in 0..6 {
                close(
                    result.stiffness_local[row][col],
                    result.stiffness_local[col][row],
                    1.0e-13,
                    "Timoshenko stiffness symmetry",
                );
            }
        }
    }

    #[test]
    fn cantilever_tip_compliance_matches_timoshenko_analytical_solution() {
        let tip_load_n = 50.0;
        let result = timoshenko_frame_element_stiffness(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            SHEAR_MODULUS_PA,
            SECOND_MOMENT_M4,
            SHEAR_CORRECTION,
        )
        .expect("valid Timoshenko element");
        let (actual_displacement, actual_rotation) =
            cantilever_tip_response(&result.stiffness_local, tip_load_n);
        let expected_displacement = tip_load_n * LENGTH_M.powi(3)
            / (3.0 * YOUNGS_MODULUS_PA * SECOND_MOMENT_M4)
            + tip_load_n * LENGTH_M
                / (SHEAR_CORRECTION * SHEAR_MODULUS_PA * AREA_M2);
        let expected_rotation = tip_load_n * LENGTH_M.powi(2)
            / (2.0 * YOUNGS_MODULUS_PA * SECOND_MOMENT_M4);
        close(
            actual_displacement,
            expected_displacement,
            1.0e-12,
            "Timoshenko cantilever displacement",
        );
        close(
            actual_rotation,
            expected_rotation,
            1.0e-12,
            "Timoshenko cantilever rotation",
        );
    }

    #[test]
    fn euler_bernoulli_limit_is_recovered_as_shear_flexibility_vanishes() {
        let eb = euler_bernoulli_stiffness_for_comparison(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            SECOND_MOMENT_M4,
        )
        .expect("Euler-Bernoulli stiffness");
        let timo = timoshenko_frame_element_stiffness(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            SHEAR_MODULUS_PA,
            SECOND_MOMENT_M4,
            1.0e16,
        )
        .expect("very-high-shear-stiffness Timoshenko limit");
        for row in 0..6 {
            for col in 0..6 {
                let scale = eb[row][col].abs().max(1.0);
                assert!(
                    (eb[row][col] - timo.stiffness_local[row][col]).abs() / scale < 1.0e-12,
                    "Euler-Bernoulli limit mismatch at ({row},{col})"
                );
            }
        }
    }

    #[test]
    fn shear_flexibility_reproduces_dimensionless_cantilever_sensitivity() {
        let length = 1.0;
        let youngs = 200.0e9;
        let shear = 80.0e9;
        let area = 1.0e-4;
        let load = 100.0;
        for (shear_slenderness, expected_ratio) in [(25.0, 1.12), (50.0, 1.06), (100.0, 1.03)] {
            let inertia = shear * area * length.powi(2)
                / (youngs * shear_slenderness);
            let eb = euler_bernoulli_stiffness_for_comparison(length, area, youngs, inertia)
                .expect("Euler-Bernoulli stiffness");
            let timo = timoshenko_frame_element_stiffness(
                length,
                area,
                youngs,
                shear,
                inertia,
                1.0,
            )
            .expect("Timoshenko stiffness");
            let (eb_displacement, _) = cantilever_tip_response(&eb, load);
            let (timo_displacement, _) = cantilever_tip_response(&timo.stiffness_local, load);
            let ratio = timo_displacement / eb_displacement;
            close(
                ratio,
                expected_ratio,
                1.0e-12,
                "Timoshenko/Euler-Bernoulli cantilever displacement ratio",
            );
        }
    }

    #[test]
    fn invalid_timoshenko_domains_fail_closed() {
        assert!(timoshenko_frame_element_stiffness(
            0.0,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            SHEAR_MODULUS_PA,
            SECOND_MOMENT_M4,
            SHEAR_CORRECTION,
        )
        .is_err());
        assert!(timoshenko_frame_element_stiffness(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            SHEAR_MODULUS_PA,
            SECOND_MOMENT_M4,
            0.0,
        )
        .is_err());
        assert!(timoshenko_frame_element_stiffness(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            SHEAR_MODULUS_PA,
            SECOND_MOMENT_M4,
            f64::NAN,
        )
        .is_err());
    }
}
