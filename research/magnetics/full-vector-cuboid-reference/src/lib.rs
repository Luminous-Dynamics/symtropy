// SPDX-License-Identifier: Apache-2.0 OR MIT
#![forbid(unsafe_code)]

//! Full-vector analytical field of a uniformly polarized rectangular cuboid.
//!
//! This independent reference sums the six rectangular surface-charge faces.
//! It is exact for prescribed uniform polarization in free space, away from
//! cuboid coordinate planes. It is not a nonlinear or multi-material solver.

use std::fmt;

const FOUR_PI: f64 = 4.0 * std::f64::consts::PI;

/// Positive cuboid half-dimensions (a,b,c), in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HalfDimensionsMeters(pub [f64; 3]);

/// Uniform magnetic polarization J = mu0*M, in tesla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolarizationTesla(pub [f64; 3]);

/// Observer position in a right-handed Cartesian frame, in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionMeters(pub [f64; 3]);

/// Magnetic flux density B, in tesla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldVectorTesla(pub [f64; 3]);

impl FieldVectorTesla {
    #[must_use]
    pub fn magnitude(self) -> f64 {
        self.0[0].hypot(self.0[1]).hypot(self.0[2])
    }
}

/// Homogeneously polarized cuboid centred at the origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UniformlyPolarizedCuboid {
    half_dimensions: HalfDimensionsMeters,
    polarization: PolarizationTesla,
}

/// Invalid geometry, observer, boundary, or numerical range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CuboidFieldError {
    NonFiniteDimension,
    NonPositiveDimension,
    NonFinitePolarization,
    NonFiniteObserver,
    CoordinatePlaneBoundary,
    NumericalRange,
    NonFiniteField,
}

impl fmt::Display for CuboidFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteDimension => f.write_str("cuboid half-dimension is non-finite"),
            Self::NonPositiveDimension => f.write_str("cuboid half-dimensions must be positive"),
            Self::NonFinitePolarization => f.write_str("polarization contains a non-finite value"),
            Self::NonFiniteObserver => f.write_str("observer position contains a non-finite value"),
            Self::CoordinatePlaneBoundary => {
                f.write_str("observer lies exactly on a cuboid coordinate plane; use a one-sided API")
            }
            Self::NumericalRange => {
                f.write_str("geometry or observer separation is outside the supported f64 range")
            }
            Self::NonFiniteField => f.write_str("field calculation produced a non-finite value"),
        }
    }
}

impl std::error::Error for CuboidFieldError {}

impl UniformlyPolarizedCuboid {
    /// Construct a cuboid centred at the origin with uniform prescribed
    /// polarization J = mu0*M. Polarization components are in tesla.
    ///
    /// This is a constitutive input, not a prediction of remanence, coercivity,
    /// or a nonlinear B-H response.
    ///
    /// # Errors
    ///
    /// Rejects non-finite polarization/dimensions and non-positive dimensions.
    pub fn new(
        half_dimensions: HalfDimensionsMeters,
        polarization: PolarizationTesla,
    ) -> Result<Self, CuboidFieldError> {
        if !polarization.0.iter().all(|value| value.is_finite()) {
            return Err(CuboidFieldError::NonFinitePolarization);
        }
        for dimension in half_dimensions.0 {
            if !dimension.is_finite() {
                return Err(CuboidFieldError::NonFiniteDimension);
            }
            if dimension <= 0.0 {
                return Err(CuboidFieldError::NonPositiveDimension);
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

    /// Evaluate the complete vector B field at an off-boundary observer.
    ///
    /// Each face has magnetic surface charge sigma = J dot n. Its field is
    /// integrated analytically in a local right-handed face frame. The six
    /// face contributions are summed and the material term J is added only
    /// for strictly interior observers.
    ///
    /// v0.1 conservatively rejects any observer whose coordinate exactly
    /// equals one of the cuboid half-dimensions, including coordinate-plane
    /// extensions outside the face. This avoids hidden face-side semantics;
    /// a later API must define explicit one-sided face limits and edge rules.
    ///
    /// # Errors
    ///
    /// Rejects non-finite observers, exact coordinate-plane boundaries,
    /// non-representable normalized geometry, and non-finite field output.
    pub fn field_at(
        self,
        observer: PositionMeters,
    ) -> Result<FieldVectorTesla, CuboidFieldError> {
        let [x, y, z] = observer.0;
        if !observer.0.iter().all(|value| value.is_finite()) {
            return Err(CuboidFieldError::NonFiniteObserver);
        }

        let [a, b, c] = self.half_dimensions.0;
        if x.abs() == a || y.abs() == b || z.abs() == c {
            return Err(CuboidFieldError::CoordinatePlaneBoundary);
        }

        // The face integrals are scale invariant in geometry. Normalize all
        // lengths by one common scale to avoid overflowing corner products.
        let scale = a.max(b).max(c).max(x.abs()).max(y.abs()).max(z.abs());
        if !scale.is_finite() || scale == 0.0 {
            return Err(CuboidFieldError::NumericalRange);
        }

        let half = [a / scale, b / scale, c / scale];
        let point = [x / scale, y / scale, z / scale];
        if half.iter().any(|value| *value == 0.0)
            || point.iter().any(|value| !value.is_finite())
        {
            return Err(CuboidFieldError::NumericalRange);
        }

        let polarization = self.polarization.0;
        let mut total = [0.0_f64; 3];

        // Each basis satisfies u x v = n. The indices select each face's
        // in-plane half-widths and its distance from the cuboid origin.
        const FACES: [FaceSpec; 6] = [
            FaceSpec {
                u: [0.0, 1.0, 0.0],
                v: [0.0, 0.0, 1.0],
                n: [1.0, 0.0, 0.0],
                u_axis: 1,
                v_axis: 2,
                n_axis: 0,
                charge_sign: 1.0,
            },
            FaceSpec {
                u: [0.0, 1.0, 0.0],
                v: [0.0, 0.0, -1.0],
                n: [-1.0, 0.0, 0.0],
                u_axis: 1,
                v_axis: 2,
                n_axis: 0,
                charge_sign: -1.0,
            },
            FaceSpec {
                u: [0.0, 0.0, 1.0],
                v: [1.0, 0.0, 0.0],
                n: [0.0, 1.0, 0.0],
                u_axis: 2,
                v_axis: 0,
                n_axis: 1,
                charge_sign: 1.0,
            },
            FaceSpec {
                u: [0.0, 0.0, 1.0],
                v: [-1.0, 0.0, 0.0],
                n: [0.0, -1.0, 0.0],
                u_axis: 2,
                v_axis: 0,
                n_axis: 1,
                charge_sign: -1.0,
            },
            FaceSpec {
                u: [1.0, 0.0, 0.0],
                v: [0.0, 1.0, 0.0],
                n: [0.0, 0.0, 1.0],
                u_axis: 0,
                v_axis: 1,
                n_axis: 2,
                charge_sign: 1.0,
            },
            FaceSpec {
                u: [1.0, 0.0, 0.0],
                v: [0.0, -1.0, 0.0],
                n: [0.0, 0.0, -1.0],
                u_axis: 0,
                v_axis: 1,
                n_axis: 2,
                charge_sign: -1.0,
            },
        ];

        for face in FACES {
            let half_u = half[face.u_axis];
            let half_v = half[face.v_axis];
            let half_n = half[face.n_axis];
            let center = scale_vector(face.n, half_n);
            let relative = subtract(point, center);

            let local_u = dot(relative, face.u);
            let local_v = dot(relative, face.v);
            let local_n = dot(relative, face.n);
            if local_n == 0.0 {
                return Err(CuboidFieldError::NumericalRange);
            }

            let [integral_u, integral_v, solid_angle] =
                rectangular_face_integral(local_u, local_v, local_n, half_u, half_v)?;

            let face_charge = face.charge_sign * polarization[face.n_axis];
            let face_integral = add(
                add(
                    scale_vector(face.u, integral_u),
                    scale_vector(face.v, integral_v),
                ),
                scale_vector(face.n, solid_angle),
            );
            total = add(total, scale_vector(face_integral, face_charge / FOUR_PI));
            if !total.iter().all(|value| value.is_finite()) {
                return Err(CuboidFieldError::NonFiniteField);
            }
        }

        // B = mu0*H + J inside magnetic material; outside B = mu0*H.
        if x.abs() < a && y.abs() < b && z.abs() < c {
            total = add(total, polarization);
        }

        if !total.iter().all(|value| value.is_finite()) {
            return Err(CuboidFieldError::NonFiniteField);
        }
        Ok(FieldVectorTesla(total))
    }
}

#[derive(Clone, Copy)]
struct FaceSpec {
    u: [f64; 3],
    v: [f64; 3],
    n: [f64; 3],
    u_axis: usize,
    v_axis: usize,
    n_axis: usize,
    charge_sign: f64,
}

/// Return dimensionless face integrals in the local (u,v,n) frame.
fn rectangular_face_integral(
    u: f64,
    v: f64,
    w: f64,
    half_u: f64,
    half_v: f64,
) -> Result<[f64; 3], CuboidFieldError> {
    let u_minus = u - half_u;
    let u_plus = u + half_u;
    let v_minus = v - half_v;
    let v_plus = v + half_v;

    let g = |q: f64| -> Result<f64, CuboidFieldError> {
        let distance = q.hypot(w);
        Ok(asinh_ratio(v + half_v, distance)? - asinh_ratio(v - half_v, distance)?)
    };
    let h = |q: f64| -> Result<f64, CuboidFieldError> {
        let distance = q.hypot(w);
        Ok(asinh_ratio(u + half_u, distance)? - asinh_ratio(u - half_u, distance)?)
    };

    let integral_u = g(u_minus)? - g(u_plus)?;
    let integral_v = h(v_minus)? - h(v_plus)?;

    let r_mm = norm3([u_minus, v_minus, w]);
    let r_pm = norm3([u_plus, v_minus, w]);
    let r_mp = norm3([u_minus, v_plus, w]);
    let r_pp = norm3([u_plus, v_plus, w]);

    let angle_mm = atan_ratio(u_minus * v_minus, w, r_mm)?;
    let angle_pm = atan_ratio(u_plus * v_minus, w, r_pm)?;
    let angle_mp = atan_ratio(u_minus * v_plus, w, r_mp)?;
    let angle_pp = atan_ratio(u_plus * v_plus, w, r_pp)?;
    let solid_angle = angle_mm - angle_pm - angle_mp + angle_pp;

    let result = [integral_u, integral_v, solid_angle];
    if !result.iter().all(|value| value.is_finite()) {
        return Err(CuboidFieldError::NonFiniteField);
    }
    Ok(result)
}

/// Stable asinh(x/d), where d is a positive distance.
fn asinh_ratio(x: f64, distance: f64) -> Result<f64, CuboidFieldError> {
    if !distance.is_finite() || distance <= 0.0 || !x.is_finite() {
        return Err(CuboidFieldError::NumericalRange);
    }
    if x == 0.0 {
        return Ok(0.0);
    }

    let ratio = x / distance;
    let value = if ratio.is_finite() {
        ratio.asinh()
    } else {
        x.signum() * (x.abs().ln() + std::f64::consts::LN_2 - distance.ln())
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CuboidFieldError::NumericalRange)
    }
}

/// Principal arctangent of numerator/(w*radius), preserving the correct
/// signed solid-angle branch for observers both inside and outside the face.
fn atan_ratio(numerator: f64, w: f64, radius: f64) -> Result<f64, CuboidFieldError> {
    let denominator = w * radius;
    if !numerator.is_finite() || !denominator.is_finite() || denominator == 0.0 {
        return Err(CuboidFieldError::NumericalRange);
    }
    let value = (numerator / denominator).atan();
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CuboidFieldError::NumericalRange)
    }
}

fn norm3(vector: [f64; 3]) -> f64 {
    vector[0].hypot(vector[1]).hypot(vector[2])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale_vector(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

#[cfg(test)]
mod tests {
    use super::*;

    const MU0: f64 = 4.0 * std::f64::consts::PI * 1.0e-7;

    fn cuboid(half: [f64; 3], polarization: [f64; 3]) -> UniformlyPolarizedCuboid {
        UniformlyPolarizedCuboid::new(
            HalfDimensionsMeters(half),
            PolarizationTesla(polarization),
        )
        .unwrap()
    }

    fn close_vector(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
        for (axis, (actual_value, expected_value)) in actual.iter().zip(expected.iter()).enumerate()
        {
            assert!(
                (actual_value - expected_value).abs() <= tolerance,
                "axis {axis}: actual={actual_value:.17e}, expected={expected_value:.17e}, tolerance={tolerance:.3e}"
            );
        }
    }

    #[test]
    fn independent_surface_quadrature_centre_known_answer() {
        let magnet = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2]);
        let field = magnet.field_at(PositionMeters([0.0, 0.0, 0.0])).unwrap();
        close_vector(
            field.0,
            [
                0.237_507_364_626_284_69,
                -0.068_957_385_855_997_394,
                0.622_481_911_222_892_65,
            ],
            5.0e-13,
        );
    }

    #[test]
    fn independent_surface_quadrature_off_axis_interior_known_answer() {
        let magnet = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2]);
        let field = magnet
            .field_at(PositionMeters([0.03, 0.02, 0.01]))
            .unwrap();
        close_vector(
            field.0,
            [
                0.262_060_187_744_897_53,
                -0.001_301_987_412_693_786_4,
                0.730_057_939_934_821_49,
            ],
            5.0e-13,
        );
    }

    #[test]
    fn independent_surface_quadrature_off_axis_exterior_known_answer() {
        let magnet = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2]);
        let field = magnet
            .field_at(PositionMeters([0.1, 0.08, 0.06]))
            .unwrap();
        close_vector(
            field.0,
            [
                0.015_926_295_793_236_546,
                0.018_502_084_308_102_039,
                -0.003_908_380_850_669_943_7,
            ],
            5.0e-13,
        );
    }

    #[test]
    fn independent_surface_quadrature_mixed_octant_known_answer() {
        let magnet = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2]);
        let field = magnet
            .field_at(PositionMeters([-0.01, 0.03, 0.075]))
            .unwrap();
        close_vector(
            field.0,
            [
                -0.027_407_633_010_405_701,
                0.059_147_108_617_825_26,
                0.096_075_180_845_258_373,
            ],
            5.0e-13,
        );
    }

    #[test]
    fn axial_specialization_matches_previous_reference() {
        let magnet = cuboid([0.02, 0.03, 0.04], [0.0, 0.0, 0.9]);
        let field = magnet.field_at(PositionMeters([0.0, 0.0, 0.0])).unwrap();
        close_vector(field.0, [0.0, 0.0, 0.744_351_933_386_115_8], 5.0e-13);
    }

    #[test]
    fn central_inversion_preserves_the_axial_vector_field() {
        let magnet = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2]);
        let point = [0.03, -0.015, 0.02];
        let opposite = [-point[0], -point[1], -point[2]];
        let first = magnet.field_at(PositionMeters(point)).unwrap();
        let second = magnet.field_at(PositionMeters(opposite)).unwrap();
        close_vector(first.0, second.0, 2.0e-14);
    }

    #[test]
    fn field_is_linear_in_polarization_and_reverses_sign() {
        let point = PositionMeters([0.03, 0.02, 0.01]);
        let base = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2])
            .field_at(point)
            .unwrap();
        let doubled = cuboid([0.05, 0.04, 0.03], [0.6, -0.2, 2.4])
            .field_at(point)
            .unwrap();
        let reversed = cuboid([0.05, 0.04, 0.03], [-0.3, 0.1, -1.2])
            .field_at(point)
            .unwrap();

        close_vector(doubled.0, scale_vector(base.0, 2.0), 1.0e-14);
        close_vector(reversed.0, scale_vector(base.0, -1.0), 1.0e-14);
    }

    #[test]
    fn common_geometric_scaling_preserves_field_at_fixed_polarization() {
        let first = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2])
            .field_at(PositionMeters([0.03, 0.02, 0.01]))
            .unwrap();
        let scaled = cuboid([0.5, 0.4, 0.3], [0.3, -0.1, 1.2])
            .field_at(PositionMeters([0.3, 0.2, 0.1]))
            .unwrap();
        close_vector(first.0, scaled.0, 1.0e-14);
    }

    #[test]
    fn far_field_converges_to_equivalent_point_dipole() {
        let half = [0.05, 0.04, 0.03];
        let polarization = [0.3, -0.1, 1.2];
        let magnet = cuboid(half, polarization);
        let point = [10.0, 8.0, 6.0];
        let field = magnet.field_at(PositionMeters(point)).unwrap().0;

        let volume = 8.0 * half[0] * half[1] * half[2];
        let moment = scale_vector(polarization, volume / MU0);
        let radius = norm3(point);
        let r_hat = scale_vector(point, 1.0 / radius);
        let moment_dot_r = dot(moment, r_hat);
        let angular = subtract(scale_vector(r_hat, 3.0 * moment_dot_r), moment);
        let dipole = scale_vector(angular, 1.0e-7 / radius.powi(3));

        close_vector(field, dipole, 3.0e-13);
    }

    #[test]
    fn exact_coordinate_plane_and_face_points_are_rejected() {
        let magnet = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2]);
        assert_eq!(
            magnet.field_at(PositionMeters([0.05, 0.0, 0.0])),
            Err(CuboidFieldError::CoordinatePlaneBoundary)
        );
        assert_eq!(
            magnet.field_at(PositionMeters([0.1, 0.04, 0.2])),
            Err(CuboidFieldError::CoordinatePlaneBoundary)
        );
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        assert_eq!(
            UniformlyPolarizedCuboid::new(
                HalfDimensionsMeters([0.0, 0.04, 0.03]),
                PolarizationTesla([0.0, 0.0, 1.0])
            ),
            Err(CuboidFieldError::NonPositiveDimension)
        );
        assert_eq!(
            UniformlyPolarizedCuboid::new(
                HalfDimensionsMeters([0.05, f64::NAN, 0.03]),
                PolarizationTesla([0.0, 0.0, 1.0])
            ),
            Err(CuboidFieldError::NonFiniteDimension)
        );
        assert_eq!(
            UniformlyPolarizedCuboid::new(
                HalfDimensionsMeters([0.05, 0.04, 0.03]),
                PolarizationTesla([0.0, f64::INFINITY, 1.0])
            ),
            Err(CuboidFieldError::NonFinitePolarization)
        );
        let magnet = cuboid([0.05, 0.04, 0.03], [0.3, -0.1, 1.2]);
        assert_eq!(
            magnet.field_at(PositionMeters([0.0, f64::NAN, 0.0])),
            Err(CuboidFieldError::NonFiniteObserver)
        );
    }
}
