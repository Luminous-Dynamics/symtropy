// SPDX-License-Identifier: MIT OR Apache-2.0
#![forbid(unsafe_code)]

//! Exact on-axis flux density of a uniformly polarized rectangular prism.
//!
//! This is a finite-body analytic reference, not a general 3D cuboid solver.
//! Observation points are restricted to the prism's symmetry axis and
//! polarization is parallel to that axis.

use std::fmt;

const PI: f64 = std::f64::consts::PI;
const MU0: f64 = 4.0 * PI * 1.0e-7;

/// Positive half-dimensions (a, b, c) in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HalfDimensionsMeters(pub [f64; 3]);

/// Uniform magnetic polarization J = mu0*M, in tesla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolarizationTesla(pub f64);

/// Observation coordinate on the cuboid Z axis, in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisPositionMeters(pub f64);

/// Axial magnetic flux density Bz, in tesla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxialFluxDensityTesla(pub f64);

/// Uniformly polarized cuboid centred at the origin, with J parallel to Z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxiallyPolarizedCuboid {
    half_dimensions: HalfDimensionsMeters,
    polarization: PolarizationTesla,
}

/// Validation or arithmetic error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CuboidError {
    NonFiniteDimension,
    NonPositiveDimension,
    NonFinitePolarization,
    NonFiniteObserver,
    NonFiniteDisplacement,
    NonFiniteField,
}

impl fmt::Display for CuboidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteDimension => f.write_str("cuboid half-dimension is non-finite"),
            Self::NonPositiveDimension => f.write_str("cuboid half-dimensions must be positive"),
            Self::NonFinitePolarization => f.write_str("polarization is non-finite"),
            Self::NonFiniteObserver => f.write_str("axial observation coordinate is non-finite"),
            Self::NonFiniteDisplacement => {
                f.write_str("axial displacement is outside the f64 range")
            }
            Self::NonFiniteField => f.write_str("axial field is outside the f64 range"),
        }
    }
}

impl std::error::Error for CuboidError {}

impl AxiallyPolarizedCuboid {
    /// Construct a centred cuboid with positive half-dimensions and uniform Z polarization.
    ///
    /// Polarization is J = mu0*M, expressed in tesla. It is not a remanence
    /// inferred from a material datasheet, and the model assumes prescribed
    /// uniform polarization rather than solving a nonlinear constitutive law.
    ///
    /// # Errors
    ///
    /// Rejects non-finite polarization/dimensions and non-positive dimensions.
    pub fn new(
        half_dimensions: HalfDimensionsMeters,
        polarization: PolarizationTesla,
    ) -> Result<Self, CuboidError> {
        if !polarization.0.is_finite() {
            return Err(CuboidError::NonFinitePolarization);
        }
        for dimension in half_dimensions.0 {
            if !dimension.is_finite() {
                return Err(CuboidError::NonFiniteDimension);
            }
            if dimension <= 0.0 {
                return Err(CuboidError::NonPositiveDimension);
            }
        }
        Ok(Self {
            half_dimensions,
            polarization,
        })
    }

    #[must_use]
    pub const fn half_dimensions(self) -> HalfDimensionsMeters {
        self.half_dimensions
    }

    #[must_use]
    pub const fn polarization(self) -> PolarizationTesla {
        self.polarization
    }

    /// Evaluate Bz at an observer on the cuboid's Z symmetry axis.
    ///
    /// The cuboid is centred at the origin, its half-dimensions are (a,b,c),
    /// and its uniform polarization is (0,0,Jz). The result is magnetic flux
    /// density Bz in tesla, including the material term Jz strictly inside.
    ///
    /// The closed form follows from the scalar potential of the uniformly
    /// charged top and bottom faces:
    ///
    /// Bz = Jz/pi * (theta(z-c) - theta(z+c)) + Jz * I(|z| < c)
    ///
    /// theta(d) = atan(a*b / (d*sqrt(d*d + a*a + b*b))).
    ///
    /// Signed principal arctangents and one-sided limits handle the end-face
    /// centres. The Bz normal component is continuous at both face centres.
    /// This axial line does not intersect cuboid edges because a,b > 0.
    ///
    /// # Errors
    ///
    /// Rejects a non-finite observer, unrepresentable displacements or output.
    pub fn field_at_z(
        self,
        observer: AxisPositionMeters,
    ) -> Result<AxialFluxDensityTesla, CuboidError> {
        let z = observer.0;
        if !z.is_finite() {
            return Err(CuboidError::NonFiniteObserver);
        }

        let [a, b, c] = self.half_dimensions.0;
        let top_distance = z - c;
        let bottom_distance = z + c;
        if !top_distance.is_finite() || !bottom_distance.is_finite() {
            return Err(CuboidError::NonFiniteDisplacement);
        }

        // At z=+c the upper face uses its exterior (+) limit; at z=-c
        // the lower face uses its exterior (-) limit.
        let top_angle = rectangle_solid_angle_half_angle(a, b, top_distance, 1.0);
        let bottom_angle = rectangle_solid_angle_half_angle(a, b, bottom_distance, -1.0);
        let material_term = if z.abs() < c { 1.0 } else { 0.0 };
        let geometric_factor = (top_angle - bottom_angle) / PI + material_term;
        let field = self.polarization.0 * geometric_factor;

        if !field.is_finite() {
            return Err(CuboidError::NonFiniteField);
        }
        Ok(AxialFluxDensityTesla(field))
    }
}

/// Signed half solid angle of an axis-aligned rectangle at axial distance d.
///
/// Dimensionless scaling avoids overflowing a*b or d*sqrt(d*d+a*a+b*b).
/// zero_limit_sign chooses the correct one-sided limit for d exactly zero.
fn rectangle_solid_angle_half_angle(a: f64, b: f64, d: f64, zero_limit_sign: f64) -> f64 {
    if d == 0.0 {
        return zero_limit_sign * (PI / 2.0);
    }
    let scale = a.max(b).max(d.abs());
    let an = a / scale;
    let bn = b / scale;
    let dn = d / scale;
    let denominator = dn.hypot(an).hypot(bn) * dn;
    let ratio = (an * bn) / denominator;
    ratio.atan()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(j: f64) -> AxiallyPolarizedCuboid {
        AxiallyPolarizedCuboid::new(
            HalfDimensionsMeters([0.05, 0.05, 0.05]),
            PolarizationTesla(j),
        )
        .unwrap()
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn cube_centre_interior_face_and_exterior_known_answers() {
        let magnet = cube(1.2);
        close(
            magnet.field_at_z(AxisPositionMeters(0.0)).unwrap().0,
            0.8,
            1.0e-14,
        );
        close(
            magnet.field_at_z(AxisPositionMeters(0.025)).unwrap().0,
            0.726_331_230_043_011_6,
            1.0e-14,
        );
        close(
            magnet.field_at_z(AxisPositionMeters(0.05)).unwrap().0,
            0.523_086_939_781_23,
            1.0e-14,
        );
        close(
            magnet.field_at_z(AxisPositionMeters(0.1)).unwrap().0,
            0.161_738_863_484_888_08,
            1.0e-14,
        );
    }

    #[test]
    fn non_cubic_prism_known_answers() {
        let magnet = AxiallyPolarizedCuboid::new(
            HalfDimensionsMeters([0.02, 0.03, 0.04]),
            PolarizationTesla(0.9),
        )
        .unwrap();

        close(
            magnet.field_at_z(AxisPositionMeters(0.0)).unwrap().0,
            0.744_351_933_386_115_8,
            1.0e-14,
        );
        close(
            magnet.field_at_z(AxisPositionMeters(0.04)).unwrap().0,
            0.425_573_888_507_814_36,
            1.0e-14,
        );
        close(
            magnet.field_at_z(AxisPositionMeters(0.08)).unwrap().0,
            0.066_398_342_784_277_32,
            1.0e-14,
        );
    }

    #[test]
    fn axial_field_is_even_in_z() {
        let magnet = cube(1.2);
        for z in [0.0, 0.01, 0.025, 0.05, 0.075, 0.1, 0.2] {
            let positive = magnet.field_at_z(AxisPositionMeters(z)).unwrap().0;
            let negative = magnet.field_at_z(AxisPositionMeters(-z)).unwrap().0;
            close(positive, negative, 1.0e-14);
        }
    }

    #[test]
    fn end_face_centre_limits_are_continuous_and_symmetric() {
        let magnet = cube(1.2);
        let upper = magnet.field_at_z(AxisPositionMeters(0.05)).unwrap().0;
        let lower = magnet.field_at_z(AxisPositionMeters(-0.05)).unwrap().0;
        close(upper, 0.523_086_939_781_23, 1.0e-14);
        close(lower, upper, 1.0e-14);
    }

    #[test]
    fn polarization_is_linear_and_reversal_changes_field_sign() {
        let positive = cube(1.2);
        let negative = cube(-1.2);
        let doubled = cube(2.4);
        for z in [0.0, 0.025, 0.05, 0.1, 0.2] {
            let p = positive.field_at_z(AxisPositionMeters(z)).unwrap().0;
            let n = negative.field_at_z(AxisPositionMeters(z)).unwrap().0;
            let d = doubled.field_at_z(AxisPositionMeters(z)).unwrap().0;
            close(n, -p, 1.0e-14);
            close(d, 2.0 * p, 1.0e-14);
        }
    }

    #[test]
    fn geometric_scaling_preserves_field_when_polarization_is_fixed() {
        let base = AxiallyPolarizedCuboid::new(
            HalfDimensionsMeters([0.02, 0.03, 0.04]),
            PolarizationTesla(0.9),
        )
        .unwrap();
        let scaled = AxiallyPolarizedCuboid::new(
            HalfDimensionsMeters([0.2, 0.3, 0.4]),
            PolarizationTesla(0.9),
        )
        .unwrap();

        for z in [0.0, 0.02, 0.04, 0.08, 0.16] {
            let first = base.field_at_z(AxisPositionMeters(z)).unwrap().0;
            let second = scaled.field_at_z(AxisPositionMeters(z * 10.0)).unwrap().0;
            close(first, second, 1.0e-14);
        }
    }

    #[test]
    fn far_field_converges_to_the_point_dipole_moment() {
        let magnet = cube(1.2);
        let [a, b, c] = magnet.half_dimensions().0;
        let volume = 8.0 * a * b * c;
        let moment_am2 = volume * magnet.polarization().0 / MU0;
        let z: f64 = 5.0;
        let exact = magnet.field_at_z(AxisPositionMeters(z)).unwrap().0;
        let dipole = 2.0e-7 * moment_am2 / z.powi(3);
        close(exact, dipole, dipole * 5.0e-8);
    }

    #[test]
    fn invalid_geometry_polarization_and_observers_are_rejected() {
        assert_eq!(
            AxiallyPolarizedCuboid::new(
                HalfDimensionsMeters([0.0, 0.05, 0.05]),
                PolarizationTesla(1.0)
            ),
            Err(CuboidError::NonPositiveDimension)
        );
        assert_eq!(
            AxiallyPolarizedCuboid::new(
                HalfDimensionsMeters([f64::NAN, 0.05, 0.05]),
                PolarizationTesla(1.0)
            ),
            Err(CuboidError::NonFiniteDimension)
        );
        assert_eq!(
            AxiallyPolarizedCuboid::new(
                HalfDimensionsMeters([0.05; 3]),
                PolarizationTesla(f64::INFINITY)
            ),
            Err(CuboidError::NonFinitePolarization)
        );
        let magnet = cube(1.0);
        assert_eq!(
            magnet.field_at_z(AxisPositionMeters(f64::NAN)),
            Err(CuboidError::NonFiniteObserver)
        );
    }

    #[test]
    fn very_large_dimensions_do_not_overflow_intermediate_products() {
        let magnet = AxiallyPolarizedCuboid::new(
            HalfDimensionsMeters([1.0e150; 3]),
            PolarizationTesla(0.75),
        )
        .unwrap();
        let result = magnet.field_at_z(AxisPositionMeters(0.0)).unwrap();
        assert!(result.0.is_finite());
        close(result.0, 0.5, 1.0e-14);
    }
}
