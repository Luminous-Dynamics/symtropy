//! Independent uniform-density 3D capsule-link mass-property reference.
//!
//! This crate is an analytical oracle, not a production rigid-body solver.
//! Local capsule axis is Z; the origin is the capsule center of mass.
//! The supplied length is the overall end-to-end capsule length, including caps.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt::{Display, Formatter};

pub type Vec3 = [f64; 3];
pub type Mat3 = [[f64; 3]; 3];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapsuleMassError {
    NonFiniteInput,
    NonPositiveMass,
    NonPositiveRadius,
    LengthBelowDiameter,
    NonPositiveVolume,
    InvalidRotation,
    NonFiniteResult,
}

impl Display for CapsuleMassError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(formatter, "mass, length and radius must be finite"),
            Self::NonPositiveMass => write!(formatter, "mass must be positive"),
            Self::NonPositiveRadius => write!(formatter, "radius must be positive"),
            Self::LengthBelowDiameter => write!(
                formatter,
                "overall length must be at least the cap diameter"
            ),
            Self::NonPositiveVolume => {
                write!(formatter, "capsule volume must be positive and finite")
            }
            Self::InvalidRotation => write!(formatter, "rotation must be proper and orthogonal"),
            Self::NonFiniteResult => write!(
                formatter,
                "capsule mass-property calculation produced a non-finite result"
            ),
        }
    }
}
impl Error for CapsuleMassError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapsuleMassProperties3 {
    mass_kg: f64,
    overall_length_m: f64,
    radius_m: f64,
    cylinder_length_m: f64,
    density_kg_m3: f64,
    cylinder_mass_kg: f64,
    cap_mass_kg: f64,
    /// Diagonal of C = integral(r r^T dm), in local COM frame, kg m^2.
    second_moment_diagonal_kg_m2: Vec3,
    /// Principal moments of inertia about local X, Y and Z, in kg m^2.
    principal_inertia_kg_m2: Vec3,
}

impl CapsuleMassProperties3 {
    /// Construct a uniform-density capsule from total mass, overall length and radius.
    ///
    /// Geometry is a cylinder of length Lc = L - 2r plus two hemispherical caps.
    /// The longitudinal/capsule axis is local Z and COM is the local origin.
    pub fn from_uniform_mass(
        mass_kg: f64,
        overall_length_m: f64,
        radius_m: f64,
    ) -> Result<Self, CapsuleMassError> {
        if !mass_kg.is_finite() || !overall_length_m.is_finite() || !radius_m.is_finite() {
            return Err(CapsuleMassError::NonFiniteInput);
        }
        if mass_kg <= 0.0 {
            return Err(CapsuleMassError::NonPositiveMass);
        }
        if radius_m <= 0.0 {
            return Err(CapsuleMassError::NonPositiveRadius);
        }
        let diameter = 2.0 * radius_m;
        if !diameter.is_finite() || overall_length_m < diameter {
            return Err(CapsuleMassError::LengthBelowDiameter);
        }

        let cylinder_length_m = overall_length_m - diameter;
        let cylinder_half_length = 0.5 * cylinder_length_m;
        let cylinder_volume = PI * radius_m * radius_m * cylinder_length_m;
        let cap_volume = (4.0 / 3.0) * PI * radius_m.powi(3);
        let total_volume = cylinder_volume + cap_volume;
        if !total_volume.is_finite() || total_volume <= 0.0 {
            return Err(CapsuleMassError::NonPositiveVolume);
        }

        let density_kg_m3 = mass_kg / total_volume;
        let cylinder_mass_kg = density_kg_m3 * cylinder_volume;
        let cap_mass_kg = density_kg_m3 * cap_volume;

        // C = integral(r r^T dm) about the capsule COM.
        //
        // Cylinder: Cxx = Cyy = mc*r^2/4; Czz = mc*Lc^2/12.
        // Caps: each hemisphere's COM is 3r/8 from its flat face. The
        // second moment about its own COM in each direction is 19*m_h*r^2/320.
        // Its axial COM lies at Lc/2 + 3r/8 from the capsule COM.
        let radial_second_moment =
            cylinder_mass_kg * radius_m * radius_m / 4.0 + cap_mass_kg * radius_m * radius_m / 5.0;
        let axial_cap_offset = cylinder_half_length + 3.0 * radius_m / 8.0;
        let axial_second_moment = cylinder_mass_kg * cylinder_length_m * cylinder_length_m / 12.0
            + cap_mass_kg
                * (19.0 * radius_m * radius_m / 320.0 + axial_cap_offset * axial_cap_offset);

        let principal_inertia_kg_m2 = [
            radial_second_moment + axial_second_moment,
            radial_second_moment + axial_second_moment,
            2.0 * radial_second_moment,
        ];
        let values = [
            density_kg_m3,
            cylinder_mass_kg,
            cap_mass_kg,
            radial_second_moment,
            axial_second_moment,
        ];
        if values.iter().any(|x| !x.is_finite())
            || principal_inertia_kg_m2.iter().any(|x| !x.is_finite())
        {
            return Err(CapsuleMassError::NonFiniteResult);
        }

        Ok(Self {
            mass_kg,
            overall_length_m,
            radius_m,
            cylinder_length_m,
            density_kg_m3,
            cylinder_mass_kg,
            cap_mass_kg,
            second_moment_diagonal_kg_m2: [
                radial_second_moment,
                radial_second_moment,
                axial_second_moment,
            ],
            principal_inertia_kg_m2,
        })
    }

    pub fn mass_kg(&self) -> f64 {
        self.mass_kg
    }
    pub fn overall_length_m(&self) -> f64 {
        self.overall_length_m
    }
    pub fn radius_m(&self) -> f64 {
        self.radius_m
    }
    pub fn cylinder_length_m(&self) -> f64 {
        self.cylinder_length_m
    }
    pub fn density_kg_m3(&self) -> f64 {
        self.density_kg_m3
    }
    pub fn cylinder_mass_kg(&self) -> f64 {
        self.cylinder_mass_kg
    }
    pub fn cap_mass_kg(&self) -> f64 {
        self.cap_mass_kg
    }
    pub fn second_moment_diagonal_kg_m2(&self) -> Vec3 {
        self.second_moment_diagonal_kg_m2
    }
    pub fn principal_inertia_kg_m2(&self) -> Vec3 {
        self.principal_inertia_kg_m2
    }

    pub fn local_second_moment_matrix_kg_m2(&self) -> Mat3 {
        [
            [self.second_moment_diagonal_kg_m2[0], 0.0, 0.0],
            [0.0, self.second_moment_diagonal_kg_m2[1], 0.0],
            [0.0, 0.0, self.second_moment_diagonal_kg_m2[2]],
        ]
    }

    /// Rotate the local second-moment tensor into another proper orthonormal frame.
    pub fn second_moment_in_frame(&self, local_to_world: Mat3) -> Result<Mat3, CapsuleMassError> {
        validate_rotation(local_to_world)?;
        let local = self.local_second_moment_matrix_kg_m2();
        let rotated = multiply(
            &multiply(&local_to_world, &local),
            &transpose(local_to_world),
        );
        if rotated.iter().flatten().all(|x| x.is_finite()) {
            Ok(rotated)
        } else {
            Err(CapsuleMassError::NonFiniteResult)
        }
    }
}

fn transpose(matrix: Mat3) -> Mat3 {
    [
        [matrix[0][0], matrix[1][0], matrix[2][0]],
        [matrix[0][1], matrix[1][1], matrix[2][1]],
        [matrix[0][2], matrix[1][2], matrix[2][2]],
    ]
}

fn multiply(left: &Mat3, right: &Mat3) -> Mat3 {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            for k in 0..3 {
                result[row][column] += left[row][k] * right[k][column];
            }
        }
    }
    result
}

fn validate_rotation(rotation: Mat3) -> Result<(), CapsuleMassError> {
    if rotation.iter().flatten().any(|x| !x.is_finite()) {
        return Err(CapsuleMassError::NonFiniteInput);
    }
    let product = multiply(&transpose(rotation), &rotation);
    for row in 0..3 {
        for column in 0..3 {
            let expected = if row == column { 1.0 } else { 0.0 };
            if (product[row][column] - expected).abs() > 1.0e-10 {
                return Err(CapsuleMassError::InvalidRotation);
            }
        }
    }
    let determinant = rotation[0][0]
        * (rotation[1][1] * rotation[2][2] - rotation[1][2] * rotation[2][1])
        - rotation[0][1] * (rotation[1][0] * rotation[2][2] - rotation[1][2] * rotation[2][0])
        + rotation[0][2] * (rotation[1][0] * rotation[2][1] - rotation[1][1] * rotation[2][0]);
    if (determinant - 1.0).abs() > 1.0e-10 {
        return Err(CapsuleMassError::InvalidRotation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.15e}, expected {expected:.15e}, tolerance {tolerance:.3e}"
        );
    }

    fn identity() -> Mat3 {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    #[test]
    fn sphere_boundary_reduces_to_uniform_solid_sphere() {
        let properties =
            CapsuleMassProperties3::from_uniform_mass(1.0, 0.06, 0.03).expect("sphere limit");
        let expected_c = 1.0 * 0.03 * 0.03 / 5.0;
        let expected_i = 2.0 * 1.0 * 0.03 * 0.03 / 5.0;
        for value in properties.second_moment_diagonal_kg_m2() {
            close(value, expected_c, 1.0e-15);
        }
        for value in properties.principal_inertia_kg_m2() {
            close(value, expected_i, 1.0e-15);
        }
        close(properties.cylinder_mass_kg(), 0.0, 1.0e-15);
        close(properties.cap_mass_kg(), 1.0, 1.0e-12);
    }

    #[test]
    fn reference_link_matches_closed_form_mass_split_and_tensor() {
        let properties =
            CapsuleMassProperties3::from_uniform_mass(1.0, 0.3, 0.03).expect("capsule");
        close(properties.cylinder_length_m(), 0.24, 1.0e-14);
        close(properties.cylinder_mass_kg(), 6.0 / 7.0, 1.0e-12);
        close(properties.cap_mass_kg(), 1.0 / 7.0, 1.0e-12);
        close(
            properties.second_moment_diagonal_kg_m2()[0],
            0.00021857142857142857,
            1.0e-15,
        );
        close(
            properties.second_moment_diagonal_kg_m2()[2],
            0.006582857142857142,
            1.0e-14,
        );
        close(
            properties.principal_inertia_kg_m2()[0],
            0.006801428571428571,
            1.0e-14,
        );
        close(
            properties.principal_inertia_kg_m2()[2],
            0.00043714285714285714,
            1.0e-15,
        );
    }

    #[test]
    fn slender_link_has_larger_transverse_than_axial_inertia() {
        let properties =
            CapsuleMassProperties3::from_uniform_mass(1.0, 0.5, 0.01).expect("slender capsule");
        let inertia = properties.principal_inertia_kg_m2();
        assert!(inertia[0] > 100.0 * inertia[2], "transverse={:?}", inertia);
        close(inertia[0], 0.02031189189189189, 1.0e-13);
        close(inertia[2], 0.00004972972972972974, 1.0e-15);
    }

    #[test]
    fn mass_properties_scale_linearly_with_mass() {
        let one = CapsuleMassProperties3::from_uniform_mass(1.0, 0.3, 0.03).expect("one");
        let two = CapsuleMassProperties3::from_uniform_mass(2.0, 0.3, 0.03).expect("two");
        for axis in 0..3 {
            close(
                two.second_moment_diagonal_kg_m2()[axis],
                2.0 * one.second_moment_diagonal_kg_m2()[axis],
                1.0e-14,
            );
            close(
                two.principal_inertia_kg_m2()[axis],
                2.0 * one.principal_inertia_kg_m2()[axis],
                1.0e-14,
            );
        }
        close(two.density_kg_m3(), 2.0 * one.density_kg_m3(), 1.0e-10);
    }

    #[test]
    fn geometric_scaling_squares_mass_moments_at_fixed_mass() {
        let original = CapsuleMassProperties3::from_uniform_mass(1.0, 0.3, 0.03).expect("original");
        let doubled = CapsuleMassProperties3::from_uniform_mass(1.0, 0.6, 0.06).expect("doubled");
        for axis in 0..3 {
            close(
                doubled.second_moment_diagonal_kg_m2()[axis],
                4.0 * original.second_moment_diagonal_kg_m2()[axis],
                1.0e-13,
            );
            close(
                doubled.principal_inertia_kg_m2()[axis],
                4.0 * original.principal_inertia_kg_m2()[axis],
                1.0e-13,
            );
        }
    }

    #[test]
    fn rotated_frame_preserves_tensor_trace_and_principal_values() {
        let properties =
            CapsuleMassProperties3::from_uniform_mass(1.0, 0.3, 0.03).expect("capsule");
        let angle = std::f64::consts::FRAC_PI_4;
        let rotation = [
            [angle.cos(), 0.0, angle.sin()],
            [0.0, 1.0, 0.0],
            [-angle.sin(), 0.0, angle.cos()],
        ];
        let rotated = properties
            .second_moment_in_frame(rotation)
            .expect("rotated tensor");
        let original = properties.second_moment_diagonal_kg_m2();
        close(
            rotated[0][0] + rotated[1][1] + rotated[2][2],
            original[0] + original[1] + original[2],
            1.0e-14,
        );
        close(rotated[0][0], 0.5 * (original[0] + original[2]), 1.0e-14);
        close(rotated[2][2], 0.5 * (original[0] + original[2]), 1.0e-14);
        assert!(
            rotated[0][2].abs() > 1.0e-5,
            "rotation must retain off-diagonal coupling"
        );
        assert!((rotated[0][2] - rotated[2][0]).abs() < 1.0e-14);
    }

    #[test]
    fn formula_agrees_with_independent_cap_slice_quadrature() {
        let properties =
            CapsuleMassProperties3::from_uniform_mass(1.0, 0.3, 0.03).expect("capsule");
        let radius = properties.radius_m();
        let cylinder_length = properties.cylinder_length_m();
        let half = 0.5 * cylinder_length;
        let dz = radius / 20_000.0;
        let mut cxx_caps = 0.0;
        let mut czz_caps = 0.0;
        for index in 0..20_000 {
            let z = half + (index as f64 + 0.5) * dz;
            let slice_radius_sq = radius * radius - (z - half) * (z - half);
            let disk_area = PI * slice_radius_sq;
            let disk_x2_integral = PI * slice_radius_sq * slice_radius_sq / 4.0;
            cxx_caps += properties.density_kg_m3() * 2.0 * disk_x2_integral * dz;
            czz_caps += properties.density_kg_m3() * 2.0 * z * z * disk_area * dz;
        }
        let cylinder_mass = properties.cylinder_mass_kg();
        let cxx_cylinder = cylinder_mass * radius * radius / 4.0;
        let czz_cylinder = cylinder_mass * cylinder_length * cylinder_length / 12.0;
        close(
            properties.second_moment_diagonal_kg_m2()[0],
            cxx_cylinder + cxx_caps,
            1.0e-12,
        );
        close(
            properties.second_moment_diagonal_kg_m2()[2],
            czz_cylinder + czz_caps,
            1.0e-10,
        );
    }

    #[test]
    fn rejects_invalid_mass_geometry_and_rotation() {
        assert_eq!(
            CapsuleMassProperties3::from_uniform_mass(0.0, 0.3, 0.03),
            Err(CapsuleMassError::NonPositiveMass)
        );
        assert_eq!(
            CapsuleMassProperties3::from_uniform_mass(1.0, 0.3, 0.0),
            Err(CapsuleMassError::NonPositiveRadius)
        );
        assert_eq!(
            CapsuleMassProperties3::from_uniform_mass(1.0, 0.05, 0.03),
            Err(CapsuleMassError::LengthBelowDiameter)
        );
        assert_eq!(
            CapsuleMassProperties3::from_uniform_mass(f64::NAN, 0.3, 0.03),
            Err(CapsuleMassError::NonFiniteInput)
        );
        let reflection = [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let capsule = CapsuleMassProperties3::from_uniform_mass(1.0, 0.3, 0.03).expect("capsule");
        assert_eq!(
            capsule.second_moment_in_frame(reflection),
            Err(CapsuleMassError::InvalidRotation)
        );
    }
}
