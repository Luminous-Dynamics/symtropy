//! Verified 3D Euler-Bernoulli beam element stiffness kernel.
//!
//! This module provides one 12-DOF linear beam element and its local/global
//! transformation. It does not yet assemble a frame, solve a global system,
//! compute geometric stiffness, eigenvalue buckling, nonlinear response,
//! shear deformation, warping, or connector coupling.
//!
//! DOF order per node: [u_x, u_y, u_z, theta_x, theta_y, theta_z].
//! Local x follows node i -> node j. Local y is the projected orientation
//! hint; local z = x cross y. The resulting frame is right-handed.

use std::error::Error;
use std::fmt;

use symtropy_freeflow_structure_lab::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamMaterial3D {
    pub youngs_modulus_pa: f64,
    pub shear_modulus_pa: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamSection3D {
    pub area_m2: f64,
    pub torsional_constant_m4: f64,
    pub second_moment_y_m4: f64,
    pub second_moment_z_m4: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamLocalFrame {
    pub x_axis_global: Vec3,
    pub y_axis_global: Vec3,
    pub z_axis_global: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamElement3D {
    pub node_i_global: Vec3,
    pub node_j_global: Vec3,
    pub length_m: f64,
    pub material: BeamMaterial3D,
    pub section: BeamSection3D,
    pub local_frame: BeamLocalFrame,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BeamElementError {
    NonFiniteCoordinate,
    ZeroLength,
    InvalidMaterial(&'static str),
    InvalidSection(&'static str),
    InvalidOrientationHint,
    NonFiniteStiffness,
}

impl fmt::Display for BeamElementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCoordinate => write!(f, "beam node coordinate is non-finite"),
            Self::ZeroLength => write!(f, "beam element has zero or invalid length"),
            Self::InvalidMaterial(name) => write!(f, "invalid beam material property: {name}"),
            Self::InvalidSection(name) => write!(f, "invalid beam section property: {name}"),
            Self::InvalidOrientationHint => {
                write!(f, "orientation hint is parallel or nearly parallel to the beam axis")
            }
            Self::NonFiniteStiffness => write!(f, "beam stiffness contains a non-finite value"),
        }
    }
}
impl Error for BeamElementError {}

impl BeamElement3D {
    pub fn new(
        node_i_global: Vec3,
        node_j_global: Vec3,
        orientation_hint_global: Vec3,
        material: BeamMaterial3D,
        section: BeamSection3D,
    ) -> Result<Self, BeamElementError> {
        if !node_i_global.is_finite() || !node_j_global.is_finite() {
            return Err(BeamElementError::NonFiniteCoordinate);
        }
        if !orientation_hint_global.is_finite() {
            return Err(BeamElementError::InvalidOrientationHint);
        }
        if !material.youngs_modulus_pa.is_finite()
            || material.youngs_modulus_pa <= 0.0
        {
            return Err(BeamElementError::InvalidMaterial("youngs_modulus_pa"));
        }
        if !material.shear_modulus_pa.is_finite()
            || material.shear_modulus_pa <= 0.0
        {
            return Err(BeamElementError::InvalidMaterial("shear_modulus_pa"));
        }
        for (name, value) in [
            ("area_m2", section.area_m2),
            ("torsional_constant_m4", section.torsional_constant_m4),
            ("second_moment_y_m4", section.second_moment_y_m4),
            ("second_moment_z_m4", section.second_moment_z_m4),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(BeamElementError::InvalidSection(name));
            }
        }

        let delta = node_j_global - node_i_global;
        let length_m = delta.norm();
        if !length_m.is_finite() || length_m <= f64::MIN_POSITIVE {
            return Err(BeamElementError::ZeroLength);
        }
        let x_axis = delta.scale(1.0 / length_m);
        let hint_length = orientation_hint_global.norm();
        let projected_hint =
            orientation_hint_global - x_axis.scale(orientation_hint_global.dot(x_axis));
        let projected_length = projected_hint.norm();
        if !hint_length.is_finite()
            || hint_length <= f64::MIN_POSITIVE
            || !projected_length.is_finite()
            || projected_length <= hint_length * 1.0e-10
        {
            return Err(BeamElementError::InvalidOrientationHint);
        }
        let y_axis = projected_hint
            .normalized()
            .ok_or(BeamElementError::InvalidOrientationHint)?;
        let z_axis = x_axis
            .cross(y_axis)
            .normalized()
            .ok_or(BeamElementError::InvalidOrientationHint)?;

        let element = Self {
            node_i_global,
            node_j_global,
            length_m,
            material,
            section,
            local_frame: BeamLocalFrame {
                x_axis_global: x_axis,
                y_axis_global: y_axis,
                z_axis_global: z_axis,
            },
        };
        let local = element.local_stiffness_matrix();
        if !local.iter().flatten().all(|value| value.is_finite()) {
            return Err(BeamElementError::NonFiniteStiffness);
        }
        Ok(element)
    }

    /// 12x12 local-coordinate Euler-Bernoulli stiffness matrix.
    ///
    /// Axial EA/L, torsional GJ/L, and two-plane bending EI terms are included.
    /// Shear deformation is omitted; use a Timoshenko formulation when the
    /// member is too deep/short for Euler-Bernoulli assumptions.
    pub fn local_stiffness_matrix(&self) -> [[f64; 12]; 12] {
        let mut matrix = [[0.0_f64; 12]; 12];
        let length = self.length_m;
        let e = self.material.youngs_modulus_pa;
        let g = self.material.shear_modulus_pa;
        let area = self.section.area_m2;
        let torsion = self.section.torsional_constant_m4;
        let iy = self.section.second_moment_y_m4;
        let iz = self.section.second_moment_z_m4;

        add_two_node_spring(&mut matrix, 0, 6, e * area / length);
        add_two_node_spring(&mut matrix, 3, 9, g * torsion / length);

        let l2 = length * length;
        let l3 = l2 * length;
        let scale_z = e * iz / l3;
        let bend_about_z = [
            [12.0, 6.0 * length, -12.0, 6.0 * length],
            [6.0 * length, 4.0 * l2, -6.0 * length, 2.0 * l2],
            [-12.0, -6.0 * length, 12.0, -6.0 * length],
            [6.0 * length, 2.0 * l2, -6.0 * length, 4.0 * l2],
        ];
        add_four_dof_block(&mut matrix, [1, 5, 7, 11], bend_about_z, scale_z);

        let scale_y = e * iy / l3;
        let bend_about_y = [
            [12.0, -6.0 * length, -12.0, -6.0 * length],
            [-6.0 * length, 4.0 * l2, 6.0 * length, 2.0 * l2],
            [-12.0, 6.0 * length, 12.0, 6.0 * length],
            [-6.0 * length, 2.0 * l2, 6.0 * length, 4.0 * l2],
        ];
        add_four_dof_block(&mut matrix, [2, 4, 8, 10], bend_about_y, scale_y);
        matrix
    }

    /// Transformation T with q_local = T * q_global.
    pub fn global_to_local_transform(&self) -> [[f64; 12]; 12] {
        let mut transform = [[0.0_f64; 12]; 12];
        let axes = [
            self.local_frame.x_axis_global,
            self.local_frame.y_axis_global,
            self.local_frame.z_axis_global,
        ];
        for node_offset in [0usize, 6usize] {
            for (local_axis, axis) in axes.iter().enumerate() {
                let components = [axis.x, axis.y, axis.z];
                for (global_axis, component) in components.iter().copied().enumerate() {
                    transform[node_offset + local_axis][node_offset + global_axis] = component;
                    transform[node_offset + 3 + local_axis][node_offset + 3 + global_axis] =
                        component;
                }
            }
        }
        transform
    }

    /// 12x12 global-coordinate stiffness matrix: T^T * K_local * T.
    pub fn global_stiffness_matrix(&self) -> [[f64; 12]; 12] {
        let local = self.local_stiffness_matrix();
        let transform = self.global_to_local_transform();
        let mut temp = [[0.0_f64; 12]; 12];
        let mut global = [[0.0_f64; 12]; 12];

        for (row, output_row) in temp.iter_mut().enumerate() {
            for (column, output) in output_row.iter_mut().enumerate() {
                *output = local[row]
                    .iter()
                    .zip(transform.iter())
                    .map(|(local_value, transform_row)| local_value * transform_row[column])
                    .sum();
            }
        }
        for (row, output_row) in global.iter_mut().enumerate() {
            for (column, output) in output_row.iter_mut().enumerate() {
                *output = transform
                    .iter()
                    .zip(temp.iter())
                    .map(|(transform_row, temp_row)| transform_row[row] * temp_row[column])
                    .sum();
            }
        }
        global
    }
}

fn add_two_node_spring(
    matrix: &mut [[f64; 12]; 12],
    dof_i: usize,
    dof_j: usize,
    stiffness: f64,
) {
    matrix[dof_i][dof_i] += stiffness;
    matrix[dof_i][dof_j] -= stiffness;
    matrix[dof_j][dof_i] -= stiffness;
    matrix[dof_j][dof_j] += stiffness;
}

fn add_four_dof_block(
    matrix: &mut [[f64; 12]; 12],
    indices: [usize; 4],
    block: [[f64; 4]; 4],
    scale: f64,
) {
    for (local_row, global_row) in indices.iter().copied().enumerate() {
        for (local_column, global_column) in indices.iter().copied().enumerate() {
            matrix[global_row][global_column] += scale * block[local_row][local_column];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material() -> BeamMaterial3D {
        BeamMaterial3D {
            youngs_modulus_pa: 200.0e9,
            shear_modulus_pa: 80.0e9,
        }
    }

    fn section() -> BeamSection3D {
        BeamSection3D {
            area_m2: 0.01,
            torsional_constant_m4: 1.0e-5,
            second_moment_y_m4: 1.0e-5,
            second_moment_z_m4: 2.0e-5,
        }
    }

    fn solve_dense_6(mut a: [[f64; 6]; 6], mut b: [f64; 6]) -> [f64; 6] {
        for pivot in 0..6 {
            let mut best = pivot;
            for row in (pivot + 1)..6 {
                if a[row][pivot].abs() > a[best][pivot].abs() {
                    best = row;
                }
            }
            assert!(a[best][pivot].abs() > 1.0e-18);
            a.swap(best, pivot);
            b.swap(best, pivot);
            for row in (pivot + 1)..6 {
                let factor = a[row][pivot] / a[pivot][pivot];
                for column in pivot..6 {
                    a[row][column] -= factor * a[pivot][column];
                }
                b[row] -= factor * b[pivot];
            }
        }
        let mut x = [0.0; 6];
        for row in (0..6).rev() {
            let tail: f64 = ((row + 1)..6).map(|column| a[row][column] * x[column]).sum();
            x[row] = (b[row] - tail) / a[row][row];
        }
        x
    }

    #[test]
    fn local_stiffness_is_symmetric_and_finite() {
        let beam = BeamElement3D::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            material(),
            section(),
        )
        .unwrap();
        let stiffness = beam.local_stiffness_matrix();
        for (row, row_values) in stiffness.iter().enumerate() {
            for (column, value) in row_values.iter().enumerate() {
                assert!(value.is_finite());
                assert!((*value - stiffness[column][row]).abs() < 1.0e-8);
            }
        }
    }

    #[test]
    fn cantilever_tip_deflection_matches_closed_form_euler_bernoulli_beam() {
        let length = 2.0;
        let force = 1_000.0;
        let beam = BeamElement3D::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(length, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            material(),
            section(),
        )
        .unwrap();
        let k = beam.local_stiffness_matrix();
        let mut reduced = [[0.0; 6]; 6];
        let mut rhs = [0.0; 6];
        for (row, reduced_row) in reduced.iter_mut().enumerate() {
            for (column, value) in reduced_row.iter_mut().enumerate() {
                *value = k[row + 6][column + 6];
            }
        }
        // Tip local-y force; node i is fully fixed.
        rhs[1] = force;
        let tip = solve_dense_6(reduced, rhs);
        let expected_deflection = force * length.powi(3)
            / (3.0 * material().youngs_modulus_pa * section().second_moment_z_m4);
        let expected_rotation = force * length.powi(2)
            / (2.0 * material().youngs_modulus_pa * section().second_moment_z_m4);
        assert!((tip[1] - expected_deflection).abs() / expected_deflection < 1.0e-10);
        assert!((tip[5] - expected_rotation).abs() / expected_rotation < 1.0e-10);
    }

    #[test]
    fn cantilever_tip_deflection_matches_second_principal_bending_plane() {
        let length = 2.0;
        let force = 1_000.0;
        let beam = BeamElement3D::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(length, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            material(),
            section(),
        )
        .unwrap();
        let k = beam.local_stiffness_matrix();
        let mut reduced = [[0.0; 6]; 6];
        let mut rhs = [0.0; 6];
        for (row, reduced_row) in reduced.iter_mut().enumerate() {
            for (column, value) in reduced_row.iter_mut().enumerate() {
                *value = k[row + 6][column + 6];
            }
        }
        // Tip local-z force bends about local y.
        rhs[2] = force;
        let tip = solve_dense_6(reduced, rhs);
        let expected_deflection = force * length.powi(3)
            / (3.0 * material().youngs_modulus_pa * section().second_moment_y_m4);
        let expected_rotation = -force * length.powi(2)
            / (2.0 * material().youngs_modulus_pa * section().second_moment_y_m4);
        assert!((tip[2] - expected_deflection).abs() / expected_deflection < 1.0e-10);
        assert!((tip[4] - expected_rotation).abs() / expected_rotation.abs() < 1.0e-10);
    }

    #[test]
    fn all_six_rigid_body_modes_have_zero_element_strain_energy() {
        let length = 2.5;
        let beam = BeamElement3D::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(length, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            material(),
            section(),
        )
        .unwrap();
        let stiffness = beam.local_stiffness_matrix();

        let rigid_modes = [
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -length, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, length, 0.0, 0.0, 0.0, 1.0],
        ];

        for mode in rigid_modes {
            let response = stiffness.map(|row| {
                row.iter()
                    .zip(mode.iter())
                    .map(|(stiffness, displacement)| stiffness * displacement)
                    .sum::<f64>()
            });
            let residual = response
                .iter()
                .map(|value| value.abs())
                .fold(0.0_f64, f64::max);
            assert!(residual < 1.0e-7);
        }
    }

    #[test]
    fn cantilever_axial_extension_matches_closed_form() {
        let length = 2.0;
        let force = 1_000.0;
        let beam = BeamElement3D::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(length, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            material(),
            section(),
        )
        .unwrap();
        let stiffness = beam.local_stiffness_matrix();
        let mut reduced = [[0.0; 6]; 6];
        for (row, reduced_row) in reduced.iter_mut().enumerate() {
            for (column, value) in reduced_row.iter_mut().enumerate() {
                *value = stiffness[row + 6][column + 6];
            }
        }
        let mut rhs = [0.0; 6];
        rhs[0] = force;
        let tip = solve_dense_6(reduced, rhs);
        let expected = force * length / (material().youngs_modulus_pa * section().area_m2);
        assert!((tip[0] - expected).abs() / expected < 1.0e-10);
    }

    #[test]
    fn cantilever_torsional_rotation_matches_closed_form() {
        let length = 2.0;
        let torque = 100.0;
        let beam = BeamElement3D::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(length, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            material(),
            section(),
        )
        .unwrap();
        let stiffness = beam.local_stiffness_matrix();
        let mut reduced = [[0.0; 6]; 6];
        for (row, reduced_row) in reduced.iter_mut().enumerate() {
            for (column, value) in reduced_row.iter_mut().enumerate() {
                *value = stiffness[row + 6][column + 6];
            }
        }
        let mut rhs = [0.0; 6];
        rhs[3] = torque;
        let tip = solve_dense_6(reduced, rhs);
        let expected = torque * length
            / (material().shear_modulus_pa * section().torsional_constant_m4);
        assert!((tip[3] - expected).abs() / expected < 1.0e-10);
    }

    #[test]
    fn local_frame_is_right_handed_and_orientation_hint_is_projected() {
        let beam = BeamElement3D::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(2.0, 3.0, 4.0),
            Vec3::new(0.0, 0.0, 1.0),
            material(),
            section(),
        )
        .unwrap();
        let x = beam.local_frame.x_axis_global;
        let y = beam.local_frame.y_axis_global;
        let z = beam.local_frame.z_axis_global;
        assert!((x.norm() - 1.0).abs() < 1.0e-12);
        assert!((y.norm() - 1.0).abs() < 1.0e-12);
        assert!((z.norm() - 1.0).abs() < 1.0e-12);
        assert!(x.dot(y).abs() < 1.0e-12);
        assert!(y.dot(z).abs() < 1.0e-12);
        assert!(z.dot(x).abs() < 1.0e-12);
        assert!((x.cross(y).dot(z) - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn global_stiffness_is_symmetric_after_coordinate_transformation() {
        let beam = BeamElement3D::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
            material(),
            section(),
        )
        .unwrap();
        let stiffness = beam.global_stiffness_matrix();
        for (row, row_values) in stiffness.iter().enumerate() {
            for (column, value) in row_values.iter().enumerate() {
                assert!(value.is_finite());
                assert!((*value - stiffness[column][row]).abs() < 1.0e-7);
            }
        }
    }

    #[test]
    fn local_global_transform_is_orthogonal_and_recovers_local_stiffness() {
        let beam = BeamElement3D::new(
            Vec3::new(-2.0, 1.0, 0.5),
            Vec3::new(1.0, 3.0, 4.5),
            Vec3::new(0.4, -0.2, 1.0),
            material(),
            section(),
        )
        .unwrap();
        let transform = beam.global_to_local_transform();
        let global = beam.global_stiffness_matrix();
        let mut recovered = [[0.0_f64; 12]; 12];
        let mut temp = [[0.0_f64; 12]; 12];

        for (row, transform_row) in transform.iter().enumerate() {
            for (column, other_row) in transform.iter().enumerate() {
                let dot: f64 = transform_row
                    .iter()
                    .zip(other_row.iter())
                    .map(|(left, right)| left * right)
                    .sum();
                let expected = if row == column { 1.0 } else { 0.0 };
                assert!((dot - expected).abs() < 1.0e-12);
            }
        }

        for (row, temp_row) in temp.iter_mut().enumerate() {
            for (column, value) in temp_row.iter_mut().enumerate() {
                *value = transform[row]
                    .iter()
                    .zip(global.iter())
                    .map(|(transform_value, global_row)| transform_value * global_row[column])
                    .sum();
            }
        }
        for (row, recovered_row) in recovered.iter_mut().enumerate() {
            for (column, value) in recovered_row.iter_mut().enumerate() {
                *value = temp[row]
                    .iter()
                    .zip(transform[column].iter())
                    .map(|(temp_value, transform_value)| temp_value * transform_value)
                    .sum();
            }
        }
        let local = beam.local_stiffness_matrix();
        let matrix_scale = local
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        let tolerance = matrix_scale * 1.0e-12;
        for (row, row_values) in recovered.iter().enumerate() {
            for (column, value) in row_values.iter().enumerate() {
                assert!(
                    (*value - local[row][column]).abs() < tolerance,
                    "transform mismatch at ({row}, {column})"
                );
            }
        }
    }

    #[test]
    fn nearly_parallel_orientation_hint_fails_closed() {
        let error = BeamElement3D::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0e-14, 0.0),
            material(),
            section(),
        )
        .unwrap_err();
        assert_eq!(error, BeamElementError::InvalidOrientationHint);
    }

    #[test]
    fn parallel_orientation_hint_fails_closed() {
        let error = BeamElement3D::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            material(),
            section(),
        )
        .unwrap_err();
        assert_eq!(error, BeamElementError::InvalidOrientationHint);
    }
}
