// SPDX-License-Identifier: Apache-2.0 OR MIT
#![forbid(unsafe_code)]

//! First-order propagation of probe-position covariance into magnetic
//! flux-density covariance through an explicitly supplied local field Jacobian.
//!
//! This crate does not calibrate a sensor, estimate a Jacobian, or infer
//! uncertainty. Unknown covariance must remain unknown at the orchestration
//! layer rather than being replaced by a zero matrix.

use std::fmt;

/// Round-off tolerance after covariance entries are normalized by their largest
/// absolute component.
const PSD_TOLERANCE: f64 = 64.0 * f64::EPSILON;

/// Symmetric 3x3 covariance stored as (xx, yy, zz, xy, xz, yz).
///
/// Construction validates finite values and positive semidefiniteness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SymmetricCovariance3 {
    components: [f64; 6],
}

/// Position covariance, in square metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionCovarianceM2(pub SymmetricCovariance3);

/// Direct field-sensor covariance, in tesla squared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldCovarianceTesla2(pub SymmetricCovariance3);

/// Local field Jacobian dB/dx, in tesla per metre.
///
/// Rows correspond to Bx, By, Bz; columns correspond to x, y, z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldJacobianTeslaPerMeter(pub [[f64; 3]; 3]);

/// Covariance validation or propagation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CovarianceError {
    NonFiniteCovariance,
    NegativeVariance,
    NegativePrincipalMinor,
    NegativeDeterminant,
    NonFiniteJacobian,
    NonFiniteArithmetic,
    NonPositiveSemidefiniteOutput,
}

impl fmt::Display for CovarianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCovariance => f.write_str("covariance contains a non-finite value"),
            Self::NegativeVariance => f.write_str("covariance has a negative diagonal variance"),
            Self::NegativePrincipalMinor => {
                f.write_str("covariance has a negative 2x2 principal minor")
            }
            Self::NegativeDeterminant => f.write_str("covariance has a negative determinant"),
            Self::NonFiniteJacobian => f.write_str("field Jacobian contains a non-finite value"),
            Self::NonFiniteArithmetic => {
                f.write_str("covariance propagation produced non-finite arithmetic")
            }
            Self::NonPositiveSemidefiniteOutput => {
                f.write_str("propagated covariance is not positive semidefinite")
            }
        }
    }
}

impl std::error::Error for CovarianceError {}

impl SymmetricCovariance3 {
    /// Construct and validate a symmetric covariance.
    ///
    /// Inputs are the three diagonal entries followed by xy, xz and yz.
    /// A real symmetric 3x3 matrix is positive semidefinite iff all its
    /// principal minors are non-negative. Entries are scaled before products
    /// are evaluated to avoid overflow.
    ///
    /// # Errors
    ///
    /// Rejects non-finite entries, negative variances, negative 2x2 principal
    /// minors, or a negative determinant beyond floating-point tolerance.
    pub fn new(
        xx: f64,
        yy: f64,
        zz: f64,
        xy: f64,
        xz: f64,
        yz: f64,
    ) -> Result<Self, CovarianceError> {
        let components = [xx, yy, zz, xy, xz, yz];
        if !components.iter().all(|value| value.is_finite()) {
            return Err(CovarianceError::NonFiniteCovariance);
        }

        let scale = components
            .iter()
            .fold(0.0_f64, |largest, value| largest.max(value.abs()));
        if scale == 0.0 {
            return Ok(Self { components });
        }

        let [a, b, c, d, e, f] = components.map(|value| value / scale);
        if a < -PSD_TOLERANCE || b < -PSD_TOLERANCE || c < -PSD_TOLERANCE {
            return Err(CovarianceError::NegativeVariance);
        }

        let minor_xy = a * b - d * d;
        let minor_xz = a * c - e * e;
        let minor_yz = b * c - f * f;
        if minor_xy < -PSD_TOLERANCE
            || minor_xz < -PSD_TOLERANCE
            || minor_yz < -PSD_TOLERANCE
        {
            return Err(CovarianceError::NegativePrincipalMinor);
        }

        let determinant =
            a * b * c + 2.0 * d * e * f - a * f * f - b * e * e - c * d * d;
        if determinant < -PSD_TOLERANCE {
            return Err(CovarianceError::NegativeDeterminant);
        }

        Ok(Self { components })
    }

    /// Explicitly known zero covariance. This is not an unknown value.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            components: [0.0; 6],
        }
    }

    /// Return components in (xx, yy, zz, xy, xz, yz) order.
    #[must_use]
    pub const fn components(self) -> [f64; 6] {
        self.components
    }

    /// Return the full symmetric matrix.
    #[must_use]
    pub const fn matrix(self) -> [[f64; 3]; 3] {
        let [xx, yy, zz, xy, xz, yz] = self.components;
        [[xx, xy, xz], [xy, yy, yz], [xz, yz, zz]]
    }
}

impl PositionCovarianceM2 {
    /// Construct a validated position covariance in m².
    #[must_use]
    pub const fn new(covariance: SymmetricCovariance3) -> Self {
        Self(covariance)
    }

    #[must_use]
    pub const fn covariance(self) -> SymmetricCovariance3 {
        self.0
    }
}

impl FieldCovarianceTesla2 {
    /// Construct a validated direct sensor covariance in T².
    #[must_use]
    pub const fn new(covariance: SymmetricCovariance3) -> Self {
        Self(covariance)
    }

    #[must_use]
    pub const fn covariance(self) -> SymmetricCovariance3 {
        self.0
    }
}

impl FieldJacobianTeslaPerMeter {
    /// Construct a finite local 3x3 field Jacobian in T/m.
    ///
    /// # Errors
    ///
    /// Rejects any non-finite matrix entry.
    pub fn new(matrix: [[f64; 3]; 3]) -> Result<Self, CovarianceError> {
        if !matrix.iter().flatten().all(|value| value.is_finite()) {
            return Err(CovarianceError::NonFiniteJacobian);
        }
        Ok(Self(matrix))
    }

    #[must_use]
    pub const fn matrix(self) -> [[f64; 3]; 3] {
        self.0
    }
}

/// Propagate position covariance into field covariance and add direct
/// per-sample sensor covariance.
///
/// The operation is:
///
/// Sigma_B = G * Sigma_x * G^T + Sigma_sensor
///
/// G is supplied in T/m, Sigma_x in m², Sigma_sensor in T², and the result
/// is in T².
///
/// Shared calibration, coordinate-registration or temperature errors must
/// remain separately represented at map level. They must not be copied into
/// Sigma_sensor as though every sample's shared bias were independent.
///
/// This is a local first-order approximation. If the local Jacobian is
/// unavailable, the caller must represent propagation as unavailable rather
/// than passing an all-zero Jacobian.
///
/// # Errors
///
/// Rejects invalid Jacobians, non-finite arithmetic, overflow, or an output
/// covariance that fails the positive-semidefinite check.
pub fn propagate_position_to_field_covariance(
    jacobian: FieldJacobianTeslaPerMeter,
    position_covariance: PositionCovarianceM2,
    sensor_covariance: FieldCovarianceTesla2,
) -> Result<FieldCovarianceTesla2, CovarianceError> {
    let gradient = jacobian.matrix();
    if !gradient.iter().flatten().all(|value| value.is_finite()) {
        return Err(CovarianceError::NonFiniteJacobian);
    }

    let sigma_x = position_covariance.covariance().matrix();
    let sigma_sensor = sensor_covariance.covariance().matrix();

    // First compute G * Sigma_x.
    let mut left_product = [[0.0_f64; 3]; 3];
    for (row_index, output_row) in left_product.iter_mut().enumerate() {
        for (column_index, output) in output_row.iter_mut().enumerate() {
            for (inner_index, gradient_value) in gradient[row_index].iter().enumerate() {
                *output += *gradient_value * sigma_x[inner_index][column_index];
            }
            if !output.is_finite() {
                return Err(CovarianceError::NonFiniteArithmetic);
            }
        }
    }

    // Then compute G * Sigma_x * G^T.
    let mut propagated = [[0.0_f64; 3]; 3];
    for (row_index, output_row) in propagated.iter_mut().enumerate() {
        for (column_index, output) in output_row.iter_mut().enumerate() {
            for (inner_index, left_value) in left_product[row_index].iter().enumerate() {
                *output += *left_value * gradient[column_index][inner_index];
            }
            if !output.is_finite() {
                return Err(CovarianceError::NonFiniteArithmetic);
            }
        }
    }

    // Average symmetric pairs to suppress floating-point asymmetry.
    let xx = propagated[0][0] + sigma_sensor[0][0];
    let yy = propagated[1][1] + sigma_sensor[1][1];
    let zz = propagated[2][2] + sigma_sensor[2][2];
    let xy = 0.5 * (propagated[0][1] + propagated[1][0]) + sigma_sensor[0][1];
    let xz = 0.5 * (propagated[0][2] + propagated[2][0]) + sigma_sensor[0][2];
    let yz = 0.5 * (propagated[1][2] + propagated[2][1]) + sigma_sensor[1][2];

    if ![xx, yy, zz, xy, xz, yz]
        .iter()
        .all(|value| value.is_finite())
    {
        return Err(CovarianceError::NonFiniteArithmetic);
    }

    let output = SymmetricCovariance3::new(xx, yy, zz, xy, xz, yz)
        .map_err(|_| CovarianceError::NonPositiveSemidefiniteOutput)?;

    Ok(FieldCovarianceTesla2::new(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covariance(
        xx: f64,
        yy: f64,
        zz: f64,
        xy: f64,
        xz: f64,
        yz: f64,
    ) -> SymmetricCovariance3 {
        SymmetricCovariance3::new(xx, yy, zz, xy, xz, yz).unwrap()
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn diagonal_covariance_is_preserved_under_identity_jacobian() {
        let position = PositionCovarianceM2::new(covariance(1.0, 4.0, 9.0, 0.0, 0.0, 0.0));
        let sensor = FieldCovarianceTesla2::new(covariance(0.25, 0.25, 0.25, 0.0, 0.0, 0.0));
        let identity = FieldJacobianTeslaPerMeter::new([
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ])
        .unwrap();

        let output = propagate_position_to_field_covariance(identity, position, sensor).unwrap();
        let [xx, yy, zz, xy, xz, yz] = output.covariance().components();

        close(xx, 1.25, 1.0e-14);
        close(yy, 4.25, 1.0e-14);
        close(zz, 9.25, 1.0e-14);
        close(xy, 0.0, 1.0e-14);
        close(xz, 0.0, 1.0e-14);
        close(yz, 0.0, 1.0e-14);
    }

    #[test]
    fn anisotropic_gradient_scales_position_uncertainty_by_axis() {
        let position = PositionCovarianceM2::new(covariance(1.0, 4.0, 9.0, 0.0, 0.0, 0.0));
        let sensor = FieldCovarianceTesla2::new(covariance(0.25, 0.25, 0.25, 0.0, 0.0, 0.0));
        let gradient = FieldJacobianTeslaPerMeter::new([
            [2.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 4.0],
        ])
        .unwrap();

        let output = propagate_position_to_field_covariance(gradient, position, sensor).unwrap();
        let [xx, yy, zz, xy, xz, yz] = output.covariance().components();

        close(xx, 4.25, 1.0e-14);
        close(yy, 36.25, 1.0e-14);
        close(zz, 144.25, 1.0e-14);
        close(xy, 0.0, 1.0e-14);
        close(xz, 0.0, 1.0e-14);
        close(yz, 0.0, 1.0e-14);
    }

    #[test]
    fn correlated_position_error_propagates_cross_axis_terms() {
        let position =
            PositionCovarianceM2::new(covariance(1.0, 4.0, 9.0, 0.5, 0.0, 0.0));
        let sensor =
            FieldCovarianceTesla2::new(covariance(0.1, 0.2, 0.3, 0.0, 0.0, 0.0));
        let gradient = FieldJacobianTeslaPerMeter::new([
            [1.0, 2.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ])
        .unwrap();

        let output = propagate_position_to_field_covariance(gradient, position, sensor).unwrap();
        let [xx, yy, zz, xy, xz, yz] = output.covariance().components();

        close(xx, 19.1, 1.0e-12);
        close(yy, 4.2, 1.0e-12);
        close(zz, 9.3, 1.0e-12);
        close(xy, 8.5, 1.0e-12);
        close(xz, 0.0, 1.0e-12);
        close(yz, 0.0, 1.0e-12);
    }

    #[test]
    fn zero_jacobian_preserves_only_known_sensor_covariance() {
        let position = PositionCovarianceM2::new(covariance(1.0, 2.0, 3.0, 0.1, 0.2, 0.3));
        let sensor = FieldCovarianceTesla2::new(covariance(0.4, 0.5, 0.6, 0.01, 0.02, 0.03));
        let zero = FieldJacobianTeslaPerMeter::new([[0.0; 3]; 3]).unwrap();

        let output = propagate_position_to_field_covariance(zero, position, sensor).unwrap();
        assert_eq!(output.covariance(), sensor.covariance());
    }

    #[test]
    fn rank_one_positive_semidefinite_covariance_is_admitted() {
        let rank_one = covariance(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        assert_eq!(rank_one.components(), [1.0; 6]);
    }

    #[test]
    fn invalid_negative_variance_is_rejected() {
        assert_eq!(
            SymmetricCovariance3::new(-1.0, 1.0, 1.0, 0.0, 0.0, 0.0),
            Err(CovarianceError::NegativeVariance)
        );
    }

    #[test]
    fn invalid_pairwise_correlation_is_rejected() {
        assert_eq!(
            SymmetricCovariance3::new(1.0, 1.0, 1.0, 1.1, 0.0, 0.0),
            Err(CovarianceError::NegativePrincipalMinor)
        );
    }

    #[test]
    fn pairwise_valid_but_negative_determinant_is_rejected() {
        assert_eq!(
            SymmetricCovariance3::new(1.0, 1.0, 1.0, 0.9, 0.9, -0.9),
            Err(CovarianceError::NegativeDeterminant)
        );
    }

    #[test]
    fn non_finite_covariance_and_jacobian_are_rejected() {
        assert_eq!(
            SymmetricCovariance3::new(f64::NAN, 0.0, 0.0, 0.0, 0.0, 0.0),
            Err(CovarianceError::NonFiniteCovariance)
        );
        assert_eq!(
            FieldJacobianTeslaPerMeter::new([
                [1.0, 0.0, f64::INFINITY],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ]),
            Err(CovarianceError::NonFiniteJacobian)
        );
    }

    #[test]
    fn overflow_during_propagation_is_rejected() {
        let position = PositionCovarianceM2::new(covariance(
            1.0e300, 1.0e300, 1.0e300, 0.0, 0.0, 0.0
        ));
        let sensor = FieldCovarianceTesla2::new(SymmetricCovariance3::zero());
        let gradient = FieldJacobianTeslaPerMeter::new([
            [1.0e100, 0.0, 0.0],
            [0.0, 1.0e100, 0.0],
            [0.0, 0.0, 1.0e100],
        ])
        .unwrap();

        assert_eq!(
            propagate_position_to_field_covariance(gradient, position, sensor),
            Err(CovarianceError::NonFiniteArithmetic)
        );
    }

    #[test]
    fn covariance_scaling_preserves_correlation_structure() {
        let source = covariance(1.0, 4.0, 9.0, 0.5, -0.25, 1.0);
        let scaled = covariance(25.0, 100.0, 225.0, 12.5, -6.25, 25.0);
        let a = source.components();
        let b = scaled.components();
        for (scaled_component, original_component) in b.iter().zip(a.iter()) {
            close(*scaled_component, 25.0 * original_component, 1.0e-12);
        }
    }
}
