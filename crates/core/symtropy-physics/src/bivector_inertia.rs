// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Rotational inertia on the N-dimensional bivector rotation space.
//!
//! The centered second-moment matrix is S = integral r r^T dm. Symtropy stores
//! the rotor generator B, whose increment is exp(-B); its physical point
//! velocity is therefore -B*r. In the library's stored-bivector convention,
//! angular momentum is I(B) = B*S + S*B. This is the dimension-generic form of
//! the familiar 3D spatial inertia tensor and naturally couples rotation planes
//! for asymmetric mass distributions.
//!
//! This module provides an independent reference for forward/inverse inertia
//! application, coordinate-frame rotation, energy, gyroscopic acceleration,
//! and rotational contact effective mass. It is not wired into the production
//! RigidBody or PhysicsWorld path. Collider-to-mass-property extraction from
//! general CAD/meshes and independent physical validation remain separate.

use nalgebra::{SMatrix, SVector};
use symtropy_math::{Bivector, Rotor};

const ROTATION_VALIDITY_TOLERANCE: f64 = 1.0e-8;

/// Average symmetric entries without overflowing or erasing a nonzero
/// average that lies below the smallest representable `f64`.
#[inline]
fn symmetric_average(left: f64, right: f64) -> Result<f64, RotationalInertiaError> {
    if !left.is_finite() || !right.is_finite() {
        return Err(RotationalInertiaError::NonFinite);
    }
    if left == right {
        return Ok(left);
    }
    let scale = left.abs().max(right.abs());
    if scale == 0.0 {
        return Ok(0.0);
    }
    let normalized = 0.5 * (left / scale) + 0.5 * (right / scale);
    if normalized == 0.0 {
        if left == -right {
            return Ok(0.0);
        }
        return Err(RotationalInertiaError::NonFinite);
    }
    let average = normalized * scale;
    if !average.is_finite() || average == 0.0 {
        return Err(RotationalInertiaError::NonFinite);
    }
    Ok(average)
}

/// Compute `m * d * d^T` without overflowing the unweighted outer product.
/// Nonzero components whose products underflow are rejected rather than
/// silently deleting a physical parallel-axis contribution.
fn checked_mass_weighted_outer<const D: usize>(
    mass: f64,
    offset: &SVector<f64, D>,
) -> Result<SMatrix<f64, D, D>, RotationalInertiaError> {
    if mass == 0.0 {
        return Ok(SMatrix::<f64, D, D>::zeros());
    }
    let weighted_offset = offset * mass.sqrt();
    if !weighted_offset.iter().all(|value| value.is_finite()) {
        return Err(RotationalInertiaError::NonFinite);
    }
    for axis in 0..D {
        if offset[axis] != 0.0 && weighted_offset[axis] == 0.0 {
            return Err(RotationalInertiaError::NonFinite);
        }
    }
    let outer = weighted_offset * weighted_offset.transpose();
    for row in 0..D {
        for column in 0..D {
            if weighted_offset[row] != 0.0
                && weighted_offset[column] != 0.0
                && outer[(row, column)] == 0.0
            {
                return Err(RotationalInertiaError::NonFinite);
            }
        }
    }
    if outer.iter().all(|value| value.is_finite()) {
        Ok(outer)
    } else {
        Err(RotationalInertiaError::NonFinite)
    }
}

/// Construction errors for a rotational inertia operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationalInertiaError {
    /// Bivectors require a spatial dimension of at least two.
    InvalidDimension,
    /// The supplied second-moment matrix contains NaN or infinity.
    NonFinite,
    /// The supplied second-moment matrix is not symmetric within tolerance.
    NotSymmetric,
    /// The supplied second-moment matrix has a materially negative eigenvalue.
    NotPositiveSemidefinite,
    /// A mass or geometric extent is non-positive or otherwise invalid.
    InvalidMassProperties,
    /// A requested inverse is undefined because at least one rotation plane has zero inertia.
    SingularInertia,
    /// An inverse-operation tolerance is non-finite or outside `(0, 1)`.
    InvalidTolerance,
    /// A contact/impulse direction is zero or non-finite.
    InvalidDirection,
    /// Component offsets do not place the combined center of mass at the origin.
    OffsetsNotCentered,
    /// A supplied Rotor is non-finite or not a proper rotation.
    InvalidRotation,
}

/// Rotational inertia acting on Lambda^2(R^D), the bivector space of rotations.
///
/// The second-moment matrix must be centered at the body's center of mass and
/// use physical mass units. For a uniform solid D-ball of mass m and radius r,
/// S = (m r^2 / (D+2)) I, giving I_plane = 2 m r^2 / (D+2) in every rotation
/// plane.
#[derive(Clone, Debug)]
pub struct BivectorInertia<const D: usize> {
    second_moment: SMatrix<f64, D, D>,
    principal_second_moments: SVector<f64, D>,
    principal_axes: SMatrix<f64, D, D>,
}

impl BivectorInertia<3> {
    /// Construct the second-moment operator from a physical 3D inertia tensor
    /// about the center of mass. For `I = tr(S) * 1 - S`, the inverse relation
    /// is `S = 0.5 * tr(I) * 1 - I`. Passing the result through
    /// `from_second_moment` enforces the physical principal-moment triangle
    /// inequalities instead of accepting an arbitrary positive matrix.
    pub fn from_inertia_tensor_3d(
        inertia_tensor: SMatrix<f64, 3, 3>,
    ) -> Result<Self, RotationalInertiaError> {
        if !inertia_tensor.iter().all(|value| value.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }
        let scale = inertia_tensor
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        let symmetry_tolerance = 1.0e-12 * scale.max(f64::MIN_POSITIVE);
        for row in 0..3 {
            for column in 0..3 {
                if (inertia_tensor[(row, column)] - inertia_tensor[(column, row)]).abs()
                    > symmetry_tolerance
                {
                    return Err(RotationalInertiaError::NotSymmetric);
                }
            }
        }
        // Symmetrize only within the already-checked roundoff tolerance.
        let mut symmetric_inertia = SMatrix::<f64, 3, 3>::zeros();
        for row in 0..3 {
            for column in 0..3 {
                symmetric_inertia[(row, column)] = symmetric_average(
                    inertia_tensor[(row, column)],
                    inertia_tensor[(column, row)],
                )?;
            }
        }
        let mut second_moment = SMatrix::<f64, 3, 3>::zeros();
        for row in 0..3 {
            let other_axes = match row {
                0 => [1, 2],
                1 => [0, 2],
                _ => [0, 1],
            };
            let diagonal_scale = symmetric_inertia[(row, row)]
                .abs()
                .max(symmetric_inertia[(other_axes[0], other_axes[0])].abs())
                .max(symmetric_inertia[(other_axes[1], other_axes[1])].abs());
            if diagonal_scale > 0.0 {
                let normalized_value = 0.5
                    * (symmetric_inertia[(other_axes[0], other_axes[0])] / diagonal_scale)
                    + 0.5 * (symmetric_inertia[(other_axes[1], other_axes[1])] / diagonal_scale)
                    - 0.5 * (symmetric_inertia[(row, row)] / diagonal_scale);
                let value = normalized_value * diagonal_scale;
                if !value.is_finite() || (normalized_value != 0.0 && value == 0.0) {
                    return Err(RotationalInertiaError::NonFinite);
                }
                second_moment[(row, row)] = value;
            }
        }
        for row in 0..3 {
            for column in 0..3 {
                if row != column {
                    second_moment[(row, column)] = -symmetric_inertia[(row, column)];
                }
            }
        }
        if !second_moment.iter().all(|value| value.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }
        Self::from_second_moment(second_moment)
    }

    /// Reconstruct the conventional physical 3D inertia tensor about the
    /// center of mass: `I = tr(S) * 1 - S`.
    pub fn inertia_tensor_3d(&self) -> Result<SMatrix<f64, 3, 3>, RotationalInertiaError> {
        let second_moment = &self.second_moment;
        let tensor = SMatrix::<f64, 3, 3>::from_fn(|row, column| {
            if row == column {
                match row {
                    0 => second_moment[(1, 1)] + second_moment[(2, 2)],
                    1 => second_moment[(0, 0)] + second_moment[(2, 2)],
                    _ => second_moment[(0, 0)] + second_moment[(1, 1)],
                }
            } else {
                -second_moment[(row, column)]
            }
        });
        if tensor.iter().all(|value| value.is_finite()) {
            Ok(tensor)
        } else {
            Err(RotationalInertiaError::NonFinite)
        }
    }
}

impl<const D: usize> BivectorInertia<D> {
    /// Construct from the centered second-moment matrix S = integral r r^T dm.
    ///
    /// Small floating-point asymmetry is removed by symmetrizing the input.
    /// The symmetric matrix is checked for positive semidefiniteness using a
    /// scale-relative eigenvalue tolerance.
    pub fn from_second_moment(
        second_moment: SMatrix<f64, D, D>,
    ) -> Result<Self, RotationalInertiaError> {
        if D < 2 || D > Bivector::<D>::MAX_SUPPORTED_DIMENSION {
            return Err(RotationalInertiaError::InvalidDimension);
        }
        if !second_moment.iter().all(|value| value.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }

        let scale = second_moment
            .iter()
            .fold(0.0_f64, |m, value| m.max(value.abs()));
        let tolerance = 1e-12 * scale.max(f64::MIN_POSITIVE);
        for i in 0..D {
            for j in 0..D {
                if (second_moment[(i, j)] - second_moment[(j, i)]).abs() > tolerance {
                    return Err(RotationalInertiaError::NotSymmetric);
                }
            }
        }

        // Average through a scale-normalized helper. It preserves equal
        // subnormal entries and rejects a nonzero average below f64 range.
        let mut symmetric = SMatrix::<f64, D, D>::zeros();
        for row in 0..D {
            for column in 0..D {
                symmetric[(row, column)] =
                    symmetric_average(second_moment[(row, column)], second_moment[(column, row)])?;
            }
        }
        if !symmetric.iter().all(|value| value.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }

        // nalgebra's fixed-size SymmetricEigen requires a ToTypenum bound that
        // an unconstrained const-generic D cannot provide. Decompose through a
        // dynamic matrix, then copy the eigenpairs back into fixed-size storage.
        let symmetric_dynamic = nalgebra::DMatrix::<f64>::from_fn(D, D, |i, j| symmetric[(i, j)]);
        let eigensystem = nalgebra::linalg::SymmetricEigen::new(symmetric_dynamic);
        if !eigensystem
            .eigenvalues
            .iter()
            .all(|value| value.is_finite())
            || !eigensystem
                .eigenvectors
                .iter()
                .all(|value| value.is_finite())
        {
            return Err(RotationalInertiaError::NonFinite);
        }
        let min_eigenvalue = eigensystem
            .eigenvalues
            .iter()
            .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        if min_eigenvalue < -tolerance {
            return Err(RotationalInertiaError::NotPositiveSemidefinite);
        }

        // Project roundoff-sized negative eigenvalues to zero. Keep the
        // reconstructed matrix and cached principal moments identical so the
        // forward and inverse operators share exactly the same PSD model.
        let principal_second_moments = SVector::<f64, D>::from_iterator(
            eigensystem.eigenvalues.iter().map(|value| value.max(0.0)),
        );
        let principal_axes = SMatrix::<f64, D, D>::from_fn(|i, j| eigensystem.eigenvectors[(i, j)]);
        let projected_second_moment = principal_axes
            * SMatrix::<f64, D, D>::from_diagonal(&principal_second_moments)
            * principal_axes.transpose();

        if !projected_second_moment
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(RotationalInertiaError::NonFinite);
        }
        Ok(Self {
            second_moment: projected_second_moment,
            principal_second_moments,
            principal_axes,
        })
    }

    /// Combine two inertia operators whose component centers are expressed
    /// relative to the same combined center of mass.
    ///
    /// The second-moment parallel-axis rule is:
    /// S = S_a + m_a*d_a*d_a^T + S_b + m_b*d_b*d_b^T.
    /// The offsets must satisfy m_a*d_a + m_b*d_b = 0 within a
    /// scale-relative tolerance. This prevents accidentally combining
    /// centered operators about an arbitrary point.
    pub fn combine_at_common_center(
        mass_a: f64,
        inertia_a: &Self,
        offset_a: &SVector<f64, D>,
        mass_b: f64,
        inertia_b: &Self,
        offset_b: &SVector<f64, D>,
    ) -> Result<Self, RotationalInertiaError> {
        if !mass_a.is_finite()
            || !mass_b.is_finite()
            || !offset_a.iter().all(|value| value.is_finite())
            || !offset_b.iter().all(|value| value.is_finite())
        {
            return Err(RotationalInertiaError::NonFinite);
        }
        if mass_a < 0.0 || mass_b < 0.0 || mass_a + mass_b <= 0.0 {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }
        // A massless component cannot carry a nonzero second-moment tensor.
        // This preserves mass/inertia consistency when combining components.
        if (mass_a == 0.0 && inertia_a.second_moment.iter().any(|value| *value != 0.0))
            || (mass_b == 0.0 && inertia_b.second_moment.iter().any(|value| *value != 0.0))
        {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }

        let total_mass = mass_a + mass_b;
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }
        // Check the weighted COM residual without multiplying huge
        // coordinates by tiny/large masses or taking overflow-prone norms.
        let mass_scale = mass_a.max(mass_b);
        let normalized_mass_a = mass_a / mass_scale;
        let normalized_mass_b = mass_b / mass_scale;
        let normalized_total_mass = normalized_mass_a + normalized_mass_b;
        let offset_scale = offset_a
            .iter()
            .chain(offset_b.iter())
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        if offset_scale > 0.0 {
            let normalized_center_residual = (offset_a / offset_scale) * normalized_mass_a
                + (offset_b / offset_scale) * normalized_mass_b;
            if !normalized_center_residual
                .iter()
                .all(|value| value.is_finite())
                || normalized_center_residual.norm() > 1e-10 * normalized_total_mass
            {
                return Err(RotationalInertiaError::OffsetsNotCentered);
            }
        }

        // Form m*d*d^T as (sqrt(m)*d)(sqrt(m)*d)^T so an intermediate
        // d*d^T cannot overflow when the mass-weighted tensor is finite.
        let parallel_axis_a = checked_mass_weighted_outer(mass_a, offset_a)?;
        let parallel_axis_b = checked_mass_weighted_outer(mass_b, offset_b)?;
        let second_moment =
            inertia_a.second_moment + parallel_axis_a + inertia_b.second_moment + parallel_axis_b;
        Self::from_second_moment(second_moment)
    }

    /// Construct from a discrete set of point masses.
    ///
    /// Positions may be in any translated world frame; this constructor first
    /// computes the center of mass and then forms the centered second moment.
    /// It is suitable for CAD/mesh mass-lumping and point-cloud reference tests.
    pub fn from_point_masses<I>(point_masses: I) -> Result<Self, RotationalInertiaError>
    where
        I: IntoIterator<Item = (SVector<f64, D>, f64)>,
    {
        if D < 2 || D > Bivector::<D>::MAX_SUPPORTED_DIMENSION {
            return Err(RotationalInertiaError::InvalidDimension);
        }

        let mut samples: Vec<_> = point_masses.into_iter().collect();
        if samples.is_empty() {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }
        for (position, mass) in &samples {
            if !mass.is_finite() || !position.iter().all(|value| value.is_finite()) {
                return Err(RotationalInertiaError::NonFinite);
            }
            if *mass < 0.0 {
                return Err(RotationalInertiaError::InvalidMassProperties);
            }
        }
        // Zero-mass samples have no physical effect and must not define the
        // numerical origin; a remote zero-mass point could otherwise cause
        // overflow when subtracting the first sample from positive-mass data.
        samples.retain(|(_, mass)| *mass > 0.0);
        if samples.is_empty() {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }
        // Floating-point accumulation must not depend on mesh/point-cloud
        // iterator order. Sort by position then mass using total ordering.
        samples.sort_by(|(position_a, mass_a), (position_b, mass_b)| {
            for axis in 0..D {
                let order = position_a[axis].total_cmp(&position_b[axis]);
                if !order.is_eq() {
                    return order;
                }
            }
            mass_a.total_cmp(mass_b)
        });

        // Work relative to the canonical first sample before accumulating
        // the center of mass. Summing absolute world coordinates can lose
        // small geometric offsets for CAD far from the world origin.
        let reference = samples[0].0;
        let mut total_mass = 0.0;
        let mut weighted_relative_position = SVector::<f64, D>::zeros();
        for (position, mass) in &samples {
            total_mass += *mass;
            weighted_relative_position += (*position - reference) * *mass;
        }
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }
        if !weighted_relative_position
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(RotationalInertiaError::NonFinite);
        }

        let center_relative = weighted_relative_position / total_mass;
        if !center_relative.iter().all(|value| value.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }
        let mut second_moment = SMatrix::<f64, D, D>::zeros();
        for (position, mass) in samples {
            let offset = (position - reference) - center_relative;
            if !offset.iter().all(|value| value.is_finite()) {
                return Err(RotationalInertiaError::NonFinite);
            }
            second_moment += checked_mass_weighted_outer(mass, &offset)?;
        }

        Self::from_second_moment(second_moment)
    }

    /// Construct the centered inertia operator for a uniform solid D-ball.
    pub fn uniform_solid_ball(mass: f64, radius: f64) -> Result<Self, RotationalInertiaError> {
        if !radius.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }
        Self::uniform_solid_ellipsoid(mass, SVector::<f64, D>::from_element(radius))
    }

    /// Construct from a uniform solid D-ellipsoid with the supplied semi-axes.
    ///
    /// For a uniform ellipsoid, the centered second moment along axis i is
    /// mass * a_i^2 / (D + 2). This gives a direct analytical mass-property
    /// path for ellipsoidal CAD primitives without assuming isotropy.
    pub fn uniform_solid_ellipsoid(
        mass: f64,
        semi_axes: SVector<f64, D>,
    ) -> Result<Self, RotationalInertiaError> {
        if !mass.is_finite() || !semi_axes.iter().all(|axis| axis.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }
        if mass <= 0.0 || semi_axes.iter().any(|axis| *axis <= 0.0) {
            return Err(RotationalInertiaError::InvalidMassProperties);
        }

        let second_moments = semi_axes.map(|axis| mass * axis * axis / (D as f64 + 2.0));
        // Positive finite mass and axes define a strictly positive tensor.
        // Reject overflow and underflow rather than silently turning a
        // representable input geometry into zero/negative rotational inertia.
        if second_moments
            .iter()
            .any(|moment| !moment.is_finite() || *moment <= 0.0)
        {
            return Err(RotationalInertiaError::NonFinite);
        }
        Self::from_second_moment(SMatrix::<f64, D, D>::from_diagonal(&second_moments))
    }

    /// Rotate the mass distribution into another coordinate frame.
    ///
    /// If the supplied proper rotation R maps body-frame vectors to world-frame
    /// vectors, the transformed second moment is S_world = R*S_body*R^T.
    /// This lets callers keep immutable body-space mass properties and derive
    /// the corresponding world-space operator for contact and diagnostics.
    pub fn rotated_by(&self, rotation: &Rotor<D>) -> Result<Self, RotationalInertiaError> {
        let r = rotation.to_matrix();
        if !r.iter().all(|value| value.is_finite())
            || !rotation.is_proper_rotation(ROTATION_VALIDITY_TOLERANCE)
        {
            return Err(RotationalInertiaError::InvalidRotation);
        }
        Self::from_second_moment(r * self.second_moment * r.transpose())
    }

    /// Return the centered second-moment matrix used by this operator.
    pub fn second_moment(&self) -> &SMatrix<f64, D, D> {
        &self.second_moment
    }

    /// Map Symtropy's stored angular state B to physical angular momentum:
    /// `L = B*S + S*B`, where `S` is the centered second-moment matrix.
    /// The antisymmetric matrix representation of `L` is the negative of the
    /// conventional cross-product matrix, while its lexicographic bivector
    /// coordinates `(i,j), i<j` map to the physical axial angular-momentum
    /// vector. This mirrors the Rotor convention `exp(-B)` and point velocity
    /// `-B*r`. Pass the stored `RigidBody.angular_velocity` directly; do not
    /// negate it here.
    pub fn angular_momentum(
        &self,
        angular_velocity: &Bivector<D>,
    ) -> Result<Bivector<D>, RotationalInertiaError> {
        if !angular_velocity.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }
        let omega = angular_velocity.to_matrix();
        let momentum = omega * self.second_moment + self.second_moment * omega;
        if !momentum.iter().all(|value| value.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }
        let result = Bivector::from_matrix(&momentum);
        if result.is_finite() {
            Ok(result)
        } else {
            Err(RotationalInertiaError::NonFinite)
        }
    }

    /// Invert stored-sign angular momentum L=I(B) to recover stored state B.
    ///
    /// The symmetric eigendecomposition S = Q*Lambda*Q^T diagonalizes the
    /// inertia operator on bivectors: in principal coordinates each plane
    /// (i,j) has eigenvalue lambda_i + lambda_j. This avoids constructing a
    /// dense D(D-1)/2 square matrix and preserves coupling in the world frame.
    ///
    /// The returned value is the stored B consumed by Rotor::from_bivector,
    /// which advances orientation with exp(-B). Physical point velocity is
    /// obtained separately with -B*r.
    ///
    /// The relative tolerance is applied to the largest eigenvalue of S. The
    /// inverse acts as a pseudoinverse on singular planes only when the supplied
    /// momentum has no material component in that nullspace; otherwise the
    /// operation fails rather than amplifying noise.
    pub fn angular_velocity_from_momentum(
        &self,
        angular_momentum: &Bivector<D>,
        relative_tolerance: f64,
    ) -> Result<Bivector<D>, RotationalInertiaError> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 || relative_tolerance >= 1.0
        {
            return Err(RotationalInertiaError::InvalidTolerance);
        }
        if !angular_momentum.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }

        let largest_eigenvalue = self
            .principal_second_moments
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(*value));
        let q = &self.principal_axes;
        let q_t = q.transpose();
        let momentum_local = q_t * angular_momentum.to_matrix() * q;
        if !momentum_local.iter().all(|value| value.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }
        let momentum_scale = momentum_local
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        let mut omega_local = nalgebra::SMatrix::<f64, D, D>::zeros();

        for i in 0..D {
            for j in (i + 1)..D {
                let plane_inertia =
                    self.principal_second_moments[i] + self.principal_second_moments[j];
                let momentum_component = momentum_local[(i, j)];
                if !plane_inertia.is_finite() {
                    return Err(RotationalInertiaError::NonFinite);
                }
                // Compare dimensionless ratios rather than multiplying the
                // largest eigenvalue or momentum magnitude by the tolerance.
                // The product can underflow for subnormal but representable
                // mass distributions, making a scale-relative test behave
                // like an absolute zero threshold.
                let relative_plane_inertia = if largest_eigenvalue > 0.0 {
                    plane_inertia / largest_eigenvalue
                } else {
                    0.0
                };
                let relative_momentum_component = if momentum_scale > 0.0 {
                    momentum_component.abs() / momentum_scale
                } else {
                    0.0
                };
                if !relative_plane_inertia.is_finite() || !relative_momentum_component.is_finite() {
                    return Err(RotationalInertiaError::NonFinite);
                }
                if relative_plane_inertia <= relative_tolerance {
                    if relative_momentum_component <= relative_tolerance {
                        continue;
                    }
                    return Err(RotationalInertiaError::SingularInertia);
                }

                let value = momentum_component / plane_inertia;
                if !value.is_finite() {
                    return Err(RotationalInertiaError::NonFinite);
                }
                omega_local[(i, j)] = value;
                omega_local[(j, i)] = -value;
            }
        }

        let omega_world = q * omega_local * q_t;
        let result = Bivector::from_matrix(&omega_world);
        if result.is_finite() {
            Ok(result)
        } else {
            Err(RotationalInertiaError::NonFinite)
        }
    }

    /// Rotational contribution to the inverse effective mass at a contact point.
    ///
    /// For unit impulse direction n and center-of-mass offset r, the stored
    /// angular impulse is `r wedge n`, matching `angular_impulse_from_point`.
    /// The induced physical point-velocity change is `-B*r`, because the
    /// Rotor update is `exp(-B)`.
    /// The returned scalar is n dot delta_v and is non-negative for a
    /// nonsingular, physically valid mass distribution.
    pub fn rotational_inverse_effective_mass(
        &self,
        offset: &SVector<f64, D>,
        impulse_direction: &SVector<f64, D>,
        relative_tolerance: f64,
    ) -> Result<f64, RotationalInertiaError> {
        if !offset.iter().all(|value| value.is_finite())
            || !impulse_direction.iter().all(|value| value.is_finite())
        {
            return Err(RotationalInertiaError::NonFinite);
        }
        // Normalize after scaling by the largest component so very large or
        // subnormal-but-nonzero finite directions do not overflow/underflow
        // during the norm calculation.
        let direction_scale = impulse_direction
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        if direction_scale == 0.0 {
            return Err(RotationalInertiaError::InvalidDirection);
        }
        let scaled_direction = impulse_direction / direction_scale;
        let scaled_norm = scaled_direction.norm();
        if !scaled_norm.is_finite() || scaled_norm == 0.0 {
            return Err(RotationalInertiaError::InvalidDirection);
        }
        let direction = scaled_direction / scaled_norm;
        let torque_accumulator = Bivector::from_wedge(offset, &direction);
        let angular_response =
            self.angular_velocity_from_momentum(&torque_accumulator, relative_tolerance)?;
        let point_velocity_change = -angular_response.apply_to_vector(offset);
        if !point_velocity_change.iter().all(|value| value.is_finite()) {
            return Err(RotationalInertiaError::NonFinite);
        }
        let effective_mass = direction.dot(&point_velocity_change);
        if !effective_mass.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }
        let response_scale = point_velocity_change
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        let normalized_response_norm = if response_scale == 0.0 {
            0.0
        } else {
            (point_velocity_change / response_scale).norm()
        };
        let normalized_effective_mass = if response_scale == 0.0 {
            effective_mass
        } else {
            effective_mass / response_scale
        };
        if !normalized_response_norm.is_finite() || !normalized_effective_mass.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }
        if normalized_effective_mass < -1e-12 * normalized_response_norm {
            return Err(RotationalInertiaError::NotPositiveSemidefinite);
        }
        Ok(effective_mass.max(0.0))
    }

    /// Evaluate contact effective mass from world-frame input while retaining
    /// this inertia in its body/principal reference frame.
    ///
    /// The method rotates the contact offset and impulse direction into the
    /// inertia frame, then reuses the same checked body-frame solve. This
    /// avoids constructing and re-diagonalizing a world-frame inertia object
    /// for every contact query.
    pub fn rotational_inverse_effective_mass_world(
        &self,
        body_to_world: &Rotor<D>,
        offset_world: &SVector<f64, D>,
        impulse_direction_world: &SVector<f64, D>,
        relative_tolerance: f64,
    ) -> Result<f64, RotationalInertiaError> {
        let rotation_matrix = body_to_world.to_matrix();
        if !rotation_matrix.iter().all(|value| value.is_finite())
            || !body_to_world.is_proper_rotation(ROTATION_VALIDITY_TOLERANCE)
        {
            return Err(RotationalInertiaError::InvalidRotation);
        }
        if !offset_world.iter().all(|value| value.is_finite())
            || !impulse_direction_world
                .iter()
                .all(|value| value.is_finite())
        {
            return Err(RotationalInertiaError::NonFinite);
        }
        let world_to_body = body_to_world.reverse();
        let offset_body = world_to_body.rotate_vector(offset_world);
        let impulse_direction_body = world_to_body.rotate_vector(impulse_direction_world);
        self.rotational_inverse_effective_mass(
            &offset_body,
            &impulse_direction_body,
            relative_tolerance,
        )
    }

    /// Compute the derivative of Symtropy's stored spatial angular state B.
    ///
    /// The world-space second moment S rotates with the body. With Rotor's
    /// exp(-B) update, its derivative is dot(S) = -B*S + S*B. The physical
    /// spatial angular-momentum bivector is represented in this stored basis
    /// by L = I(B) = B*S + S*B. Differentiating this expression gives:
    ///
    ///   I(dot(B)) = [B, L] + torque_accumulator
    ///   dot(L) = torque_accumulator
    ///
    /// `torque_accumulator` is the physical torque in the stored-bivector
    /// convention, built by the offset-first `r wedge j` impulse rule from
    /// #4619. Its matrix representation is the negative of the conventional
    /// cross-product matrix, just like the matrix representation of the
    /// physical angular-momentum bivector.
    /// The supplied inertia, B, and torque must all be expressed in the same
    /// coordinate frame. For spatial/world-frame state, rotate body-space
    /// mass moments with `rotated_by` before evaluating this method.
    pub fn angular_acceleration(
        &self,
        stored_angular_state: &Bivector<D>,
        torque_accumulator: &Bivector<D>,
        relative_tolerance: f64,
    ) -> Result<Bivector<D>, RotationalInertiaError> {
        if !stored_angular_state.is_finite() || !torque_accumulator.is_finite() {
            return Err(RotationalInertiaError::NonFinite);
        }

        let b = stored_angular_state.to_matrix();
        let momentum = self.angular_momentum(stored_angular_state)?.to_matrix();
        let coadjoint_rate = b * momentum - momentum * b;
        let momentum_rate = coadjoint_rate + torque_accumulator.to_matrix();
        let momentum_rate_bivector = Bivector::from_matrix(&momentum_rate);

        self.angular_velocity_from_momentum(&momentum_rate_bivector, relative_tolerance)
    }

    /// Rotational kinetic energy, `1/2 <B, I(B)>`, with checked representability.
    pub fn kinetic_energy(
        &self,
        angular_velocity: &Bivector<D>,
    ) -> Result<f64, RotationalInertiaError> {
        let momentum = self.angular_momentum(angular_velocity)?;
        let mut pairing = 0.0;
        for i in 0..D {
            for j in (i + 1)..D {
                pairing += angular_velocity.get(i, j) * momentum.get(i, j);
            }
        }
        let energy = 0.5 * pairing;
        if energy.is_finite() && energy >= 0.0 {
            Ok(energy)
        } else {
            Err(RotationalInertiaError::NonFinite)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_uniform_ball_plane_moment<const D: usize>(expected: f64) {
        let inertia = BivectorInertia::<D>::uniform_solid_ball(3.0, 2.0).unwrap();
        let omega = Bivector::<D>::unit_plane(0, 1);
        let momentum = inertia.angular_momentum(&omega).unwrap();
        assert!(
            (momentum.get(0, 1) - expected).abs() < 1e-12,
            "D={D}: plane inertia={}, expected={expected}",
            momentum.get(0, 1)
        );
    }

    #[test]
    fn parallel_axis_combination_matches_two_component_mass_moments() {
        let body_a = BivectorInertia::<3>::uniform_solid_ball(2.0, 1.0).unwrap();
        let body_b = BivectorInertia::<3>::uniform_solid_ball(1.0, 0.5).unwrap();
        let offset_a = SVector::new(-1.0, 0.0, 0.0);
        let offset_b = SVector::new(2.0, 0.0, 0.0);
        let combined = BivectorInertia::<3>::combine_at_common_center(
            2.0, &body_a, &offset_a, 1.0, &body_b, &offset_b,
        )
        .unwrap();

        // S_xx = 0.4 + 2*1^2 + 0.0125 + 1*2^2 = 5.4125.
        // S_yy = S_zz = 0.4 + 0.0125 = 0.4125.
        assert!((combined.second_moment()[(0, 0)] - 5.4125).abs() < 1e-12);
        assert!((combined.second_moment()[(1, 1)] - 0.4125).abs() < 1e-12);
        assert!((combined.second_moment()[(2, 2)] - 0.4125).abs() < 1e-12);
        let plane_xy = Bivector::<3>::unit_plane(0, 1);
        assert!((combined.angular_momentum(&plane_xy).unwrap().get(0, 1) - 5.825).abs() < 1e-12);
    }

    #[test]
    fn parallel_axis_combination_is_scale_safe_for_large_offsets_and_tiny_masses() {
        let mass = 1.0e-310;
        let inertia = BivectorInertia::<3>::uniform_solid_ball(mass, 1.0).unwrap();
        let offset_a = SVector::new(1.0e155, 0.0, 0.0);
        let offset_b = SVector::new(-1.0e155, 0.0, 0.0);
        let combined = BivectorInertia::<3>::combine_at_common_center(
            mass, &inertia, &offset_a, mass, &inertia, &offset_b,
        )
        .unwrap();
        assert!((combined.second_moment()[(0, 0)] - 2.0).abs() < 1.0e-10);
        assert!(
            combined
                .second_moment()
                .iter()
                .all(|value| value.is_finite())
        );

        let bad = BivectorInertia::<3>::combine_at_common_center(
            mass, &inertia, &offset_a, mass, &inertia, &offset_a,
        );
        assert_eq!(bad.unwrap_err(), RotationalInertiaError::OffsetsNotCentered);
    }

    #[test]
    fn parallel_axis_combination_rejects_underflowed_nonzero_mass_terms() {
        let mass = 1.0e-300;
        let inertia = BivectorInertia::<3>::uniform_solid_ball(mass, 1.0).unwrap();
        let offset_a = SVector::new(1.0e-100, 0.0, 0.0);
        let offset_b = SVector::new(-1.0e-100, 0.0, 0.0);
        let result = BivectorInertia::<3>::combine_at_common_center(
            mass, &inertia, &offset_a, mass, &inertia, &offset_b,
        );
        assert_eq!(result.unwrap_err(), RotationalInertiaError::NonFinite);
    }

    #[test]
    fn point_mass_constructor_rejects_weighted_offset_underflow() {
        let mass = f64::from_bits(1);
        let samples = [
            (SVector::<f64, 3>::new(-1.0e-200, 0.0, 0.0), mass),
            (SVector::<f64, 3>::new(1.0e-200, 0.0, 0.0), mass),
        ];
        assert_eq!(
            BivectorInertia::<3>::from_point_masses(samples).unwrap_err(),
            RotationalInertiaError::NonFinite
        );
    }

    #[test]
    fn combine_rejects_nonzero_inertia_with_zero_component_mass() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        let zero_inertia = BivectorInertia::<3>::from_second_moment(SMatrix::zeros()).unwrap();
        assert_eq!(
            BivectorInertia::<3>::combine_at_common_center(
                0.0,
                &inertia,
                &SVector::zeros(),
                1.0,
                &zero_inertia,
                &SVector::zeros(),
            )
            .unwrap_err(),
            RotationalInertiaError::InvalidMassProperties
        );
    }

    #[test]
    fn point_mass_constructor_ignores_zero_mass_samples_for_reference_origin() {
        let samples = [
            (SVector::<f64, 3>::new(-f64::MAX, 0.0, 0.0), 0.0),
            (SVector::<f64, 3>::new(f64::MAX, 0.0, 0.0), 1.0),
        ];
        let inertia = BivectorInertia::<3>::from_point_masses(samples).unwrap();
        assert_eq!(*inertia.second_moment(), SMatrix::<f64, 3, 3>::zeros());
    }

    #[test]
    fn point_mass_constructor_rejects_all_zero_mass_samples() {
        let samples = [
            (SVector::<f64, 3>::new(-1.0, 0.0, 0.0), 0.0),
            (SVector::<f64, 3>::new(1.0, 0.0, 0.0), 0.0),
        ];
        assert_eq!(
            BivectorInertia::<3>::from_point_masses(samples).unwrap_err(),
            RotationalInertiaError::InvalidMassProperties
        );
    }

    #[test]
    fn parallel_axis_combination_rejects_non_centered_offsets() {
        let body_a = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        let body_b = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        assert_eq!(
            BivectorInertia::<3>::combine_at_common_center(
                1.0,
                &body_a,
                &SVector::new(0.0, 0.0, 0.0),
                1.0,
                &body_b,
                &SVector::new(1.0, 0.0, 0.0),
            )
            .unwrap_err(),
            RotationalInertiaError::OffsetsNotCentered
        );
    }

    #[test]
    fn point_mass_constructor_handles_large_offsets_with_tiny_masses() {
        let mass = 1.0e-310;
        let samples = [
            (SVector::<f64, 3>::new(-1.0e155, 0.0, 0.0), mass),
            (SVector::<f64, 3>::new(1.0e155, 0.0, 0.0), mass),
        ];
        let inertia = BivectorInertia::<3>::from_point_masses(samples).unwrap();
        assert!((inertia.second_moment()[(0, 0)] - 2.0).abs() < 1.0e-10);
        assert!(
            inertia
                .second_moment()
                .iter()
                .all(|value| value.is_finite())
        );
    }

    #[test]
    fn point_mass_constructor_rejects_underflowed_nonzero_mass_terms() {
        let mass = 1.0e-300;
        let samples = [
            (SVector::<f64, 3>::new(-1.0e-100, 0.0, 0.0), mass),
            (SVector::<f64, 3>::new(1.0e-100, 0.0, 0.0), mass),
        ];
        assert_eq!(
            BivectorInertia::<3>::from_point_masses(samples).unwrap_err(),
            RotationalInertiaError::NonFinite
        );
    }

    #[test]
    fn point_mass_constructor_is_independent_of_input_order() {
        let samples = [
            (SVector::<f64, 3>::new(2.0, -1.0, 0.5), 3.0),
            (SVector::<f64, 3>::new(-0.5, 1.5, 0.2), 1.25),
            (SVector::<f64, 3>::new(0.3, 0.1, -2.0), 0.75),
            (SVector::<f64, 3>::new(1.1, 0.2, 0.8), 2.0),
        ];
        let forward = BivectorInertia::<3>::from_point_masses(samples).unwrap();
        let reversed = BivectorInertia::<3>::from_point_masses(samples.into_iter().rev()).unwrap();
        assert_eq!(forward.second_moment(), reversed.second_moment());
    }

    #[test]
    fn point_mass_constructor_is_translation_invariant() {
        let origin_cloud = [
            (SVector::<f64, 3>::new(-1.0, 0.0, 0.0), 1.0),
            (SVector::<f64, 3>::new(1.0, 0.0, 0.0), 1.0),
        ];
        let translated_cloud = [
            (SVector::<f64, 3>::new(9.0, -4.0, 12.0), 1.0),
            (SVector::<f64, 3>::new(11.0, -4.0, 12.0), 1.0),
        ];

        let origin = BivectorInertia::<3>::from_point_masses(origin_cloud).unwrap();
        let translated = BivectorInertia::<3>::from_point_masses(translated_cloud).unwrap();

        assert!((origin.second_moment() - translated.second_moment()).norm() < 1e-12);
        let omega = Bivector::<3>::unit_plane(0, 1);
        let expected = 2.0;
        assert!((translated.angular_momentum(&omega).unwrap().get(0, 1) - expected).abs() < 1e-12);
    }

    #[test]
    fn point_mass_constructor_retains_small_geometry_at_large_world_offsets() {
        let cloud = [
            (SVector::<f64, 3>::new(1.0e12, -1.0e12, 5.0e11), 1.0),
            (SVector::<f64, 3>::new(1.0e12 + 0.125, -1.0e12, 5.0e11), 1.0),
        ];
        let inertia = BivectorInertia::<3>::from_point_masses(cloud).unwrap();
        assert!(
            (inertia.second_moment()[(0, 0)] - 0.0078125).abs() < 1e-12,
            "small CAD-scale inertia lost at large world offset: S={:?}",
            inertia.second_moment()
        );
    }

    #[test]
    fn uniform_ball_has_dimension_correct_plane_inertia() {
        // I_plane = 2*m*r^2/(D+2), with m=3 and r=2.
        assert_uniform_ball_plane_moment::<2>(6.0);
        assert_uniform_ball_plane_moment::<3>(4.8);
        assert_uniform_ball_plane_moment::<4>(4.0);
    }

    #[test]
    fn uniform_ellipsoid_recovers_analytical_three_dimensional_moments() {
        // Unit mass, semi-axes (2, 1, 0.5): S_ii = a_i^2 / 5.
        // Rotation-plane moments are sums of the corresponding S diagonal.
        let inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(2.0, 1.0, 0.5))
                .unwrap();

        let xy = Bivector::<3>::unit_plane(0, 1); // rotation about z
        let xz = Bivector::<3>::unit_plane(0, 2); // rotation about y
        let yz = Bivector::<3>::unit_plane(1, 2); // rotation about x
        assert!((inertia.angular_momentum(&xy).unwrap().get(0, 1) - 1.0).abs() < 1e-12);
        assert!((inertia.angular_momentum(&xz).unwrap().get(0, 2) - 0.85).abs() < 1e-12);
        assert!((inertia.angular_momentum(&yz).unwrap().get(1, 2) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn asymmetric_three_dimensional_second_moments_recover_principal_inertia() {
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([1.0, 2.0, 4.0]));
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();

        let mut omega = Bivector::<3>::zero();
        omega.set(0, 1, 1.0); // rotation about axis 2: moment 1 + 2
        omega.set(0, 2, 2.0); // rotation about axis 1: moment 1 + 4
        omega.set(1, 2, -1.0); // rotation about axis 0: moment 2 + 4
        let momentum = inertia.angular_momentum(&omega).unwrap();

        assert!((momentum.get(0, 1) - 3.0).abs() < 1e-12);
        assert!((momentum.get(0, 2) - 10.0).abs() < 1e-12);
        assert!((momentum.get(1, 2) + 6.0).abs() < 1e-12);
        assert!((inertia.kinetic_energy(&omega).unwrap() - 14.5).abs() < 1e-12);
    }

    #[test]
    fn off_diagonal_mass_moments_couple_rotation_planes() {
        let second_moment =
            SMatrix::<f64, 3, 3>::new(2.0, 0.25, 0.0, 0.25, 3.0, 0.0, 0.0, 0.0, 4.0);
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let omega = Bivector::<3>::unit_plane(0, 2);
        let momentum = inertia.angular_momentum(&omega).unwrap();

        assert!((momentum.get(0, 2) - 6.0).abs() < 1e-12);
        assert!((momentum.get(1, 2) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn rotated_inertia_is_covariant_and_preserves_kinetic_energy() {
        let body_inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(2.0, 1.0, 0.5))
                .unwrap();
        let rotation = Rotor::<3>::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), 0.63);
        let world_inertia = body_inertia.rotated_by(&rotation).unwrap();

        let mut omega_body = Bivector::<3>::zero();
        omega_body.set(0, 1, 0.7);
        omega_body.set(0, 2, -0.4);
        omega_body.set(1, 2, 1.1);
        let omega_world_matrix =
            rotation.to_matrix() * omega_body.to_matrix() * rotation.to_matrix().transpose();
        let omega_world = Bivector::<3>::from_matrix(&omega_world_matrix);

        let momentum_body = body_inertia.angular_momentum(&omega_body).unwrap();
        let momentum_world = world_inertia.angular_momentum(&omega_world).unwrap();
        let expected_world_momentum =
            rotation.to_matrix() * momentum_body.to_matrix() * rotation.to_matrix().transpose();

        assert!((momentum_world.to_matrix() - expected_world_momentum).norm() < 1e-11);
        assert!(
            (body_inertia.kinetic_energy(&omega_body).unwrap()
                - world_inertia.kinetic_energy(&omega_world).unwrap())
            .abs()
                < 1e-11
        );
    }

    fn bivector_pairing(a: &Bivector<3>, b: &Bivector<3>) -> f64 {
        (0..3)
            .flat_map(|i| ((i + 1)..3).map(move |j| (i, j)))
            .map(|(i, j)| a.get(i, j) * b.get(i, j))
            .sum()
    }

    #[test]
    fn gyroscopic_acceleration_is_covariant_between_body_and_world_frames() {
        let body_inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(2.0, SVector::new(1.5, 0.9, 0.4))
                .unwrap();
        let rotation = Rotor::<3>::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), 0.71);
        let world_inertia = body_inertia.rotated_by(&rotation).unwrap();

        let mut b_body = Bivector::<3>::zero();
        b_body.set(0, 1, 0.8);
        b_body.set(0, 2, -0.3);
        b_body.set(1, 2, 0.5);
        let mut torque_body = Bivector::<3>::zero();
        torque_body.set(0, 1, -0.2);
        torque_body.set(0, 2, 0.1);
        torque_body.set(1, 2, 0.4);

        let to_world = |value: &Bivector<3>| {
            Bivector::from_matrix(
                &(rotation.to_matrix() * value.to_matrix() * rotation.to_matrix().transpose()),
            )
        };
        let b_world = to_world(&b_body);
        let torque_world = to_world(&torque_body);

        let b_dot_body = body_inertia
            .angular_acceleration(&b_body, &torque_body, 1e-12)
            .unwrap();
        let b_dot_world = world_inertia
            .angular_acceleration(&b_world, &torque_world, 1e-12)
            .unwrap();

        assert!(
            b_dot_world.add(&to_world(&b_dot_body).scale(-1.0)).norm() < 1e-11,
            "body-frame and world-frame angular accelerations differ"
        );
    }

    #[test]
    fn rotational_inverse_effective_mass_matches_isotropic_lever_arm() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(3.0, 2.0).unwrap();
        let offset = SVector::new(1.0, 0.0, 0.0);
        let normal = SVector::new(0.0, 1.0, 0.0);
        let value = inertia
            .rotational_inverse_effective_mass(&offset, &normal, 1e-12)
            .unwrap();
        assert!((value - 1.0 / 4.8).abs() < 1e-12);
    }

    #[test]
    fn rotational_inverse_effective_mass_uses_asymmetric_plane_inertia() {
        let inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(2.0, 1.0, 0.5))
                .unwrap();
        let offset = SVector::new(1.0, 0.0, 0.0);
        let normal = SVector::new(0.0, 1.0, 0.0);
        let value = inertia
            .rotational_inverse_effective_mass(&offset, &normal, 1e-12)
            .unwrap();
        // XY plane moment is S_xx + S_yy = 0.8 + 0.2 = 1.0.
        assert!((value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn off_center_angular_impulse_matches_contact_velocity_and_energy_identity() {
        let inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(2.0, SVector::new(1.4, 0.9, 0.5))
                .unwrap();
        let offset = SVector::new(0.7, -0.25, 0.4);
        let impulse = SVector::new(-0.2, 0.9, 0.35).normalize();
        let angular_impulse = Bivector::from_wedge(&offset, &impulse);
        let angular_response = inertia
            .angular_velocity_from_momentum(&angular_impulse, 1e-12)
            .unwrap();
        let point_velocity_change = -angular_response.apply_to_vector(&offset);
        let effective_mass = inertia
            .rotational_inverse_effective_mass(&offset, &impulse, 1e-12)
            .unwrap();
        let rotational_energy = inertia.kinetic_energy(&angular_response).unwrap();
        let impulse_work = 0.5 * impulse.dot(&point_velocity_change);

        assert!(point_velocity_change.iter().all(|value| value.is_finite()));
        assert!(impulse.dot(&point_velocity_change) > 0.0);
        assert!(
            (impulse.dot(&point_velocity_change) - effective_mass).abs() < 2.0e-11,
            "contact-point response disagrees with effective mass"
        );
        assert!(
            (rotational_energy - impulse_work).abs() <= 2.0e-11 * rotational_energy.abs().max(1.0),
            "energy/half-work mismatch: energy={rotational_energy:e}, work={impulse_work:e}"
        );
    }

    #[test]
    fn contact_impulse_matches_independent_three_dimensional_tensor_solve() {
        // Independent route: form the conventional spatial tensor
        // I = tr(S) 1 - S, solve I*omega = r x n, then evaluate omega x r.
        let second_moment =
            SMatrix::<f64, 3, 3>::new(2.0, 0.25, -0.1, 0.25, 1.5, 0.15, -0.1, 0.15, 0.9);
        let operator = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let tensor = SMatrix::<f64, 3, 3>::identity() * second_moment.trace() - second_moment;
        let spatial_inertia = tensor.lu();

        for sample in 1..=32 {
            let k = sample as f64;
            let offset = SVector::new(0.2 + 0.07 * k, (0.31 * k).sin(), (0.17 * k).cos());
            let direction =
                SVector::new((0.71 * k).sin(), (0.37 * k).cos(), 0.5 + (0.13 * k).sin())
                    .normalize();
            let angular_impulse = Bivector::from_wedge(&offset, &direction);
            let angular_response = operator
                .angular_velocity_from_momentum(&angular_impulse, 1.0e-12)
                .unwrap();
            let omega_from_bivector = SVector::from([
                angular_response.get(1, 2),
                -angular_response.get(0, 2),
                angular_response.get(0, 1),
            ]);
            let torque = offset.cross(&direction);
            let omega_from_tensor = spatial_inertia
                .solve(&torque)
                .expect("positive-definite spatial inertia");
            let point_velocity_from_bivector = -angular_response.apply_to_vector(&offset);
            let point_velocity_from_tensor = omega_from_tensor.cross(&offset);
            let observed_effective_mass = operator
                .rotational_inverse_effective_mass(&offset, &direction, 1.0e-12)
                .unwrap();
            let expected_effective_mass = direction.dot(&point_velocity_from_tensor);

            assert!(
                (omega_from_bivector - omega_from_tensor).norm() < 2.0e-11,
                "angular-velocity mismatch for sample {sample}"
            );
            assert!(
                (point_velocity_from_bivector - point_velocity_from_tensor).norm() < 2.0e-11,
                "point-velocity mismatch for sample {sample}"
            );
            assert!(
                (observed_effective_mass - expected_effective_mass).abs() < 2.0e-11,
                "effective-mass mismatch for sample {sample}: \
                 observed={observed_effective_mass:e}, expected={expected_effective_mass:e}"
            );
        }
    }

    #[test]
    fn rotational_inverse_effective_mass_is_covariant_under_global_rotation() {
        let body_inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(2.0, SVector::new(1.4, 0.9, 0.5))
                .unwrap();
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.3);
        generator.set(0, 2, -0.2);
        generator.set(1, 2, 0.4);
        let global = Rotor::from_bivector(&generator);
        let world_inertia = body_inertia.rotated_by(&global).unwrap();
        let offset = SVector::new(0.7, -0.25, 0.4);
        let direction = SVector::new(-0.2, 0.9, 0.35).normalize();
        let expected = body_inertia
            .rotational_inverse_effective_mass(&offset, &direction, 1e-12)
            .unwrap();
        let observed = world_inertia
            .rotational_inverse_effective_mass(
                &global.rotate_vector(&offset),
                &global.rotate_vector(&direction),
                1e-12,
            )
            .unwrap();
        assert!(
            (observed - expected).abs() < 2.0e-11,
            "rotational effective mass changed under global frame rotation: \
             expected={expected:e}, observed={observed:e}"
        );
    }

    #[test]
    fn body_frame_contact_effective_mass_matches_world_rotated_operator() {
        let body_inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(2.0, SVector::new(1.4, 0.9, 0.5))
                .unwrap();
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.31);
        generator.set(0, 2, -0.22);
        generator.set(1, 2, 0.43);
        let body_to_world = Rotor::from_bivector(&generator);
        let offset_body = SVector::new(0.7, -0.25, 0.4);
        let direction_body = SVector::new(-0.2, 0.9, 0.35).normalize();
        let expected = body_inertia
            .rotational_inverse_effective_mass(&offset_body, &direction_body, 1e-12)
            .unwrap();
        let world_operator = body_inertia.rotated_by(&body_to_world).unwrap();
        let expected_world = world_operator
            .rotational_inverse_effective_mass(
                &body_to_world.rotate_vector(&offset_body),
                &body_to_world.rotate_vector(&direction_body),
                1e-12,
            )
            .unwrap();
        let observed = body_inertia
            .rotational_inverse_effective_mass_world(
                &body_to_world,
                &body_to_world.rotate_vector(&offset_body),
                &body_to_world.rotate_vector(&direction_body),
                1e-12,
            )
            .unwrap();
        assert!((observed - expected).abs() < 2.0e-11);
        assert!((observed - expected_world).abs() < 2.0e-11);
    }

    #[test]
    fn rotational_inverse_effective_mass_normalizes_extreme_finite_direction() {
        let inertia =
            BivectorInertia::<3>::uniform_solid_ellipsoid(2.0, SVector::new(1.4, 0.9, 0.5))
                .unwrap();
        let offset = SVector::new(0.7, -0.25, 0.4);
        let direction = SVector::new(-0.2, 0.9, 0.35).normalize();
        let expected = inertia
            .rotational_inverse_effective_mass(&offset, &direction, 1e-12)
            .unwrap();
        let observed = inertia
            .rotational_inverse_effective_mass(&offset, &(direction * 1.0e300), 1e-12)
            .unwrap();
        assert!((observed - expected).abs() < 2.0e-11);
    }

    #[test]
    fn rotational_inverse_effective_mass_rejects_zero_direction() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        assert_eq!(
            inertia
                .rotational_inverse_effective_mass(
                    &SVector::new(1.0, 0.0, 0.0),
                    &SVector::zeros(),
                    1e-12,
                )
                .unwrap_err(),
            RotationalInertiaError::InvalidDirection
        );
    }

    #[test]
    fn torque_free_angular_acceleration_preserves_energy_and_momentum_casimir() {
        let second_moment =
            SMatrix::<f64, 3, 3>::new(1.0, 0.15, 0.05, 0.15, 2.0, 0.1, 0.05, 0.1, 3.0);
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let mut b = Bivector::<3>::zero();
        b.set(0, 1, 0.7);
        b.set(0, 2, -0.4);
        b.set(1, 2, 1.1);

        let zero_torque = Bivector::<3>::zero();
        let b_dot = inertia
            .angular_acceleration(&b, &zero_torque, 1e-12)
            .unwrap();
        let l = inertia.angular_momentum(&b).unwrap();
        // I(dot(B)) equals the commutator correction. The actual spatial
        // momentum derivative also includes the rotation of S in world space.
        let inertia_applied_to_b_dot = inertia.angular_momentum(&b_dot).unwrap();
        let commutator = Bivector::<3>::from_matrix(
            &(b.to_matrix() * l.to_matrix() - l.to_matrix() * b.to_matrix()),
        );
        assert!(
            inertia_applied_to_b_dot.add(&commutator.scale(-1.0)).norm() < 1e-11,
            "I(b_dot)={inertia_applied_to_b_dot:?}, commutator={commutator:?}"
        );

        let b_matrix = b.to_matrix();
        let b_dot_matrix = b_dot.to_matrix();
        let s = inertia.second_moment();
        let s_dot = -b_matrix * s + s * b_matrix;
        let actual_l_dot_matrix =
            b_dot_matrix * s + s * b_dot_matrix + b_matrix * s_dot + s_dot * b_matrix;
        let actual_l_dot = Bivector::<3>::from_matrix(&actual_l_dot_matrix);
        assert!(actual_l_dot.norm() < 1e-11, "dot(L)={actual_l_dot:?}");

        // The torque-free spatial angular momentum is constant, and the
        // kinetic-energy derivative is zero under this same moving-S update.
        assert!(bivector_pairing(&l, &actual_l_dot).abs() < 1e-11);
        let energy_rate =
            0.5 * (bivector_pairing(&b_dot, &l) + bivector_pairing(&b, &actual_l_dot));
        assert!(energy_rate.abs() < 1e-11, "energy_rate={energy_rate}");
    }

    #[test]
    fn angular_acceleration_reproduces_instantaneous_applied_world_torque() {
        let inertia = BivectorInertia::<3>::from_second_moment(SMatrix::<f64, 3, 3>::new(
            1.4, 0.12, -0.08, 0.12, 2.2, 0.17, -0.08, 0.17, 3.1,
        ))
        .unwrap();
        let mut b = Bivector::<3>::zero();
        b.set(0, 1, 0.4);
        b.set(0, 2, -0.25);
        b.set(1, 2, 0.7);
        let mut torque = Bivector::<3>::zero();
        torque.set(0, 1, -0.3);
        torque.set(0, 2, 0.8);
        torque.set(1, 2, 0.2);

        let b_dot = inertia.angular_acceleration(&b, &torque, 1e-12).unwrap();
        let s = inertia.second_moment();
        let b_matrix = b.to_matrix();
        let b_dot_matrix = b_dot.to_matrix();
        let s_dot = -b_matrix * s + s * b_matrix;
        let l_dot_matrix =
            b_dot_matrix * s + s * b_dot_matrix + b_matrix * s_dot + s_dot * b_matrix;
        let l_dot = Bivector::<3>::from_matrix(&l_dot_matrix);
        assert!(
            l_dot.add(&torque.scale(-1.0)).norm() < 2.0e-11,
            "instantaneous angular-momentum derivative must equal applied torque: \
             got={l_dot:?}, expected={torque:?}"
        );
    }

    #[test]
    fn angular_momentum_derivative_matches_rotor_finite_difference() {
        let inertia = BivectorInertia::<3>::from_second_moment(SMatrix::<f64, 3, 3>::new(
            1.4, 0.12, -0.08, 0.12, 2.2, 0.17, -0.08, 0.17, 3.1,
        ))
        .unwrap();
        let mut angular_state = Bivector::<3>::zero();
        angular_state.set(0, 1, 0.4);
        angular_state.set(0, 2, -0.25);
        angular_state.set(1, 2, 0.7);
        let mut torque = Bivector::<3>::zero();
        torque.set(0, 1, -0.3);
        torque.set(0, 2, 0.8);
        torque.set(1, 2, 0.2);

        let angular_acceleration = inertia
            .angular_acceleration(&angular_state, &torque, 1e-12)
            .unwrap();
        let initial_momentum = inertia.angular_momentum(&angular_state).unwrap();
        let dt = 1.0e-7;
        let delta = Rotor::from_bivector(&angular_state.scale(dt));
        let next_inertia = inertia.rotated_by(&delta).unwrap();
        let next_angular_state = angular_state.add(&angular_acceleration.scale(dt));
        let next_momentum = next_inertia.angular_momentum(&next_angular_state).unwrap();
        let observed_rate = next_momentum
            .add(&initial_momentum.scale(-1.0))
            .scale(1.0 / dt);
        let error = observed_rate.add(&torque.scale(-1.0)).norm();
        assert!(
            error < 2.0e-6,
            "Rotor finite-difference torque error={error:e}"
        );
    }

    #[test]
    fn isotropic_angular_acceleration_has_offset_first_torque_sign() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(3.0, 2.0).unwrap();
        let zero_state = Bivector::<3>::zero();
        let mut torque_accumulator = Bivector::<3>::zero();
        torque_accumulator.set(0, 1, 2.0);
        let acceleration = inertia
            .angular_acceleration(&zero_state, &torque_accumulator, 1e-12)
            .unwrap();
        assert!((acceleration.get(0, 1) - 2.0 / 4.8).abs() < 1e-12);
        assert!(acceleration.get(0, 2).abs() < 1e-12);
        assert!(acceleration.get(1, 2).abs() < 1e-12);
    }

    #[test]
    fn four_dimensional_coupled_inertia_roundtrips_all_six_rotation_planes() {
        let second_moment = SMatrix::<f64, 4, 4>::new(
            2.0, 0.2, 0.0, 0.1, 0.2, 3.0, 0.15, 0.0, 0.0, 0.15, 4.0, 0.25, 0.1, 0.0, 0.25, 5.0,
        );
        let inertia = BivectorInertia::<4>::from_second_moment(second_moment).unwrap();
        let mut angular_state = Bivector::<4>::zero();
        angular_state.set(0, 1, 0.7);
        angular_state.set(0, 2, -0.4);
        angular_state.set(0, 3, 0.2);
        angular_state.set(1, 2, 1.1);
        angular_state.set(1, 3, -0.8);
        angular_state.set(2, 3, 0.35);

        let momentum = inertia.angular_momentum(&angular_state).unwrap();
        let recovered = inertia
            .angular_velocity_from_momentum(&momentum, 1e-12)
            .unwrap();
        assert!(
            recovered.add(&angular_state.scale(-1.0)).norm() < 1e-11,
            "four-dimensional inverse failed: recovered={recovered:?}, expected={angular_state:?}"
        );
        assert!(inertia.kinetic_energy(&angular_state).unwrap() > 0.0);
    }

    #[cfg(feature = "nd-bivector-9")]
    #[test]
    fn nine_dimensional_operator_roundtrips_all_36_rotation_planes() {
        let semi_axes = SVector::<f64, 9>::from_fn(|axis, _| 0.5 + axis as f64 * 0.2);
        let inertia = BivectorInertia::<9>::uniform_solid_ellipsoid(2.5, semi_axes).unwrap();
        let mut angular_state = Bivector::<9>::zero();
        for i in 0..9 {
            for j in (i + 1)..9 {
                angular_state.set(i, j, ((i + 1) as f64 * (j + 2) as f64).sin() * 0.3);
            }
        }
        let momentum = inertia.angular_momentum(&angular_state).unwrap();
        let recovered = inertia
            .angular_velocity_from_momentum(&momentum, 1e-12)
            .unwrap();
        assert_eq!(Bivector::<9>::num_components(), 36);
        assert!(
            recovered.add(&angular_state.scale(-1.0)).norm() < 2.0e-11,
            "D=9 inverse failed: recovered={recovered:?}, expected={angular_state:?}"
        );
    }

    #[test]
    fn four_dimensional_inertia_is_covariant_under_global_so4_rotation() {
        let second_moment = SMatrix::<f64, 4, 4>::new(
            2.0, 0.2, 0.0, 0.1, 0.2, 3.0, 0.15, 0.0, 0.0, 0.15, 4.0, 0.25, 0.1, 0.0, 0.25, 5.0,
        );
        let body_inertia = BivectorInertia::<4>::from_second_moment(second_moment).unwrap();
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, 0.23);
        generator.set(0, 2, -0.17);
        generator.set(0, 3, 0.11);
        generator.set(1, 2, 0.19);
        generator.set(1, 3, -0.21);
        generator.set(2, 3, 0.14);
        let global = Rotor::<4>::from_bivector(&generator);
        let world_inertia = body_inertia.rotated_by(&global).unwrap();

        let mut body_angular_state = Bivector::<4>::zero();
        body_angular_state.set(0, 1, 0.7);
        body_angular_state.set(0, 2, -0.4);
        body_angular_state.set(0, 3, 0.2);
        body_angular_state.set(1, 2, 1.1);
        body_angular_state.set(1, 3, -0.8);
        body_angular_state.set(2, 3, 0.35);
        let rotation_matrix = global.to_matrix();
        let world_angular_state = Bivector::from_matrix(
            &(rotation_matrix * body_angular_state.to_matrix() * rotation_matrix.transpose()),
        );

        let body_momentum = body_inertia.angular_momentum(&body_angular_state).unwrap();
        let world_momentum = world_inertia
            .angular_momentum(&world_angular_state)
            .unwrap();
        let expected_world_momentum = Bivector::from_matrix(
            &(rotation_matrix * body_momentum.to_matrix() * rotation_matrix.transpose()),
        );
        assert!(
            world_momentum
                .add(&expected_world_momentum.scale(-1.0))
                .norm()
                < 2.0e-10,
            "SO(4) angular-momentum covariance failed"
        );
        assert!(
            (world_inertia.kinetic_energy(&world_angular_state).unwrap()
                - body_inertia.kinetic_energy(&body_angular_state).unwrap())
            .abs()
                < 2.0e-10,
            "SO(4) kinetic-energy covariance failed"
        );

        let recovered = world_inertia
            .angular_velocity_from_momentum(&world_momentum, 1.0e-12)
            .unwrap();
        assert!(
            recovered.add(&world_angular_state.scale(-1.0)).norm() < 2.0e-10,
            "SO(4) inverse failed to recover all six rotation-plane components"
        );
    }

    #[cfg(feature = "nd-bivector-9")]
    #[test]
    fn nine_dimensional_inertia_is_covariant_under_global_so9_rotation() {
        let factor = SMatrix::<f64, 9, 9>::from_fn(|row, column| {
            if row == column {
                1.0 + row as f64 * 0.13
            } else {
                0.015 * ((row + 1) as f64 * (column + 3) as f64).sin()
            }
        });
        let second_moment = factor * factor.transpose();
        let body_inertia = BivectorInertia::<9>::from_second_moment(second_moment).unwrap();

        let mut generator = Bivector::<9>::zero();
        let mut plane = 1.0_f64;
        for row in 0..9 {
            for column in (row + 1)..9 {
                generator.set(row, column, 0.002 * (plane * 0.37).sin());
                plane += 1.0;
            }
        }
        let global = Rotor::<9>::from_bivector(&generator);
        let world_inertia = body_inertia.rotated_by(&global).unwrap();
        let rotation_matrix = global.to_matrix();

        let mut body_angular_state = Bivector::<9>::zero();
        let mut component = 0.0_f64;
        for row in 0..9 {
            for column in (row + 1)..9 {
                body_angular_state.set(row, column, 0.12 * (component * 0.29 + 0.4).cos());
                component += 1.0;
            }
        }
        let world_angular_state = Bivector::from_matrix(
            &(rotation_matrix * body_angular_state.to_matrix() * rotation_matrix.transpose()),
        );

        let body_momentum = body_inertia.angular_momentum(&body_angular_state).unwrap();
        let world_momentum = world_inertia
            .angular_momentum(&world_angular_state)
            .unwrap();
        let expected_world_momentum = Bivector::from_matrix(
            &(rotation_matrix * body_momentum.to_matrix() * rotation_matrix.transpose()),
        );
        assert!(
            world_momentum
                .add(&expected_world_momentum.scale(-1.0))
                .norm()
                < 2.0e-9,
            "SO(9) angular-momentum covariance failed"
        );

        let body_energy = body_inertia.kinetic_energy(&body_angular_state).unwrap();
        let world_energy = world_inertia.kinetic_energy(&world_angular_state).unwrap();
        assert!(
            (world_energy - body_energy).abs() / body_energy.abs().max(f64::MIN_POSITIVE) < 2.0e-10,
            "SO(9) kinetic-energy covariance failed: body={body_energy:e}, world={world_energy:e}"
        );

        let recovered = world_inertia
            .angular_velocity_from_momentum(&world_momentum, 1.0e-12)
            .unwrap();
        assert!(
            recovered.add(&world_angular_state.scale(-1.0)).norm() < 2.0e-9,
            "SO(9) inverse failed to recover all 36 rotation-plane components"
        );
    }

    #[test]
    fn inverse_operator_recovers_coupled_world_frame_angular_velocity() {
        let second_moment =
            SMatrix::<f64, 3, 3>::new(2.0, 0.25, 0.1, 0.25, 3.0, 0.2, 0.1, 0.2, 4.0);
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();

        let mut omega = Bivector::<3>::zero();
        omega.set(0, 1, 0.7);
        omega.set(0, 2, -1.2);
        omega.set(1, 2, 0.35);

        let momentum = inertia.angular_momentum(&omega).unwrap();
        let recovered = inertia
            .angular_velocity_from_momentum(&momentum, 1e-12)
            .unwrap();
        assert!(
            recovered.add(&omega.scale(-1.0)).norm() < 1e-11,
            "recovered={recovered:?}, expected={omega:?}"
        );
    }

    #[test]
    fn inverse_operator_is_scale_relative_for_subnormal_mass_moments() {
        let tiny = 1.0e-320;
        let second_moment = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([tiny, tiny, tiny]));
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let mut angular_state = Bivector::<3>::zero();
        angular_state.set(0, 1, 0.5);
        angular_state.set(0, 2, -0.25);
        angular_state.set(1, 2, 0.75);
        let momentum = inertia.angular_momentum(&angular_state).unwrap();
        let recovered = inertia
            .angular_velocity_from_momentum(&momentum, 1.0e-12)
            .unwrap();
        assert!(
            recovered.add(&angular_state.scale(-1.0)).norm() < 1.0e-12,
            "subnormal inertia changed the recovered angular state: {recovered:?}"
        );
    }

    #[test]
    fn inverse_operator_uses_pseudoinverse_on_unexcited_singular_planes() {
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([2.0, 0.0, 0.0]));
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let momentum = Bivector::<3>::unit_plane(0, 1).scale(4.0);
        let recovered = inertia
            .angular_velocity_from_momentum(&momentum, 1e-12)
            .unwrap();
        assert!((recovered.get(0, 1) - 2.0).abs() < 1e-12);
        assert!(recovered.get(0, 2).abs() < 1e-12);
        assert!(recovered.get(1, 2).abs() < 1e-12);
    }

    #[test]
    fn inverse_operator_rejects_singular_rotation_plane() {
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([2.0, 0.0, 0.0]));
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let momentum = Bivector::<3>::unit_plane(1, 2);
        assert_eq!(
            inertia
                .angular_velocity_from_momentum(&momentum, 1e-12)
                .unwrap_err(),
            RotationalInertiaError::SingularInertia
        );
    }

    #[test]
    fn inverse_operator_rejects_invalid_tolerance() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        assert_eq!(
            inertia
                .angular_velocity_from_momentum(&Bivector::zero(), 0.0)
                .unwrap_err(),
            RotationalInertiaError::InvalidTolerance
        );
        assert_eq!(
            inertia
                .angular_velocity_from_momentum(&Bivector::zero(), 1.0)
                .unwrap_err(),
            RotationalInertiaError::InvalidTolerance
        );
    }

    #[test]
    fn ellipsoid_constructor_rejects_underflowed_positive_moments() {
        assert_eq!(
            BivectorInertia::<3>::uniform_solid_ellipsoid(
                1.0e-300,
                SVector::new(1.0e-100, 1.0e-100, 1.0e-100),
            )
            .unwrap_err(),
            RotationalInertiaError::NonFinite
        );
    }

    #[test]
    fn zero_mass_and_degenerate_ellipsoid_fail_closed() {
        assert_eq!(
            BivectorInertia::<3>::uniform_solid_ellipsoid(0.0, SVector::new(1.0, 2.0, 3.0),)
                .unwrap_err(),
            RotationalInertiaError::InvalidMassProperties
        );
        assert_eq!(
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(1.0, 0.0, 3.0),)
                .unwrap_err(),
            RotationalInertiaError::InvalidMassProperties
        );
    }

    #[test]
    fn inertia_tensor_3d_adapter_roundtrips_products_of_inertia_and_energy() {
        let inertia_tensor =
            SMatrix::<f64, 3, 3>::new(46.0, -6.0, 0.0, -6.0, 46.0, 0.0, 0.0, 0.0, 20.0);
        let operator = BivectorInertia::<3>::from_inertia_tensor_3d(inertia_tensor).unwrap();
        let recovered = operator.inertia_tensor_3d().unwrap();
        assert!((recovered - inertia_tensor).norm() < 1.0e-12);

        let omega = SVector::from([0.7, -1.1, 0.4]);
        let mut angular_state = Bivector::<3>::zero();
        angular_state.set(1, 2, omega[0]);
        angular_state.set(0, 2, -omega[1]);
        angular_state.set(0, 1, omega[2]);
        let tensor_energy = 0.5 * omega.dot(&(inertia_tensor * omega));
        let operator_energy = operator.kinetic_energy(&angular_state).unwrap();
        assert!((operator_energy - tensor_energy).abs() < 1.0e-12);
    }

    #[test]
    fn second_moment_symmetrization_rejects_unrepresentable_average() {
        let minimum = f64::from_bits(1);
        let mut matrix = SMatrix::<f64, 2, 2>::identity();
        matrix[(0, 1)] = minimum;
        matrix[(1, 0)] = 0.0;
        assert_eq!(
            BivectorInertia::<2>::from_second_moment(matrix).unwrap_err(),
            RotationalInertiaError::NonFinite
        );
    }

    #[test]
    fn inertia_tensor_adapter_rejects_unrepresentable_half_subnormal_moments() {
        let minimum = f64::from_bits(1);
        let tensor = SMatrix::<f64, 3, 3>::identity() * minimum;
        assert_eq!(
            BivectorInertia::<3>::from_inertia_tensor_3d(tensor).unwrap_err(),
            RotationalInertiaError::NonFinite
        );
    }

    #[test]
    fn inertia_tensor_adapter_preserves_minimum_representable_moments() {
        let minimum = f64::from_bits(1);
        let tensor = SMatrix::<f64, 3, 3>::identity() * (2.0 * minimum);
        let operator = BivectorInertia::<3>::from_inertia_tensor_3d(tensor).unwrap();
        assert_eq!(operator.inertia_tensor_3d().unwrap(), tensor);
    }

    #[test]
    fn inertia_tensor_adapter_rejects_unphysical_triangle_inequality() {
        let impossible = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([1.0, 1.0, 3.0]));
        assert_eq!(
            BivectorInertia::<3>::from_inertia_tensor_3d(impossible).unwrap_err(),
            RotationalInertiaError::NotPositiveSemidefinite,
        );
    }

    #[test]
    fn inertia_tensor_adapter_rejects_asymmetry() {
        let mut asymmetric = SMatrix::<f64, 3, 3>::identity();
        asymmetric[(0, 1)] = 0.1;
        assert_eq!(
            BivectorInertia::<3>::from_inertia_tensor_3d(asymmetric).unwrap_err(),
            RotationalInertiaError::NotSymmetric,
        );
    }

    #[test]
    fn uniform_mass_scaling_preserves_angular_state_and_scales_energy() {
        let base_second_moment =
            SMatrix::<f64, 3, 3>::new(1.4, 0.12, -0.08, 0.12, 2.2, 0.17, -0.08, 0.17, 3.1);
        let base = BivectorInertia::<3>::from_second_moment(base_second_moment).unwrap();
        let mut angular_state = Bivector::<3>::zero();
        angular_state.set(0, 1, 0.3);
        angular_state.set(0, 2, -0.7);
        angular_state.set(1, 2, 0.2);
        let base_momentum = base.angular_momentum(&angular_state).unwrap();
        let base_energy = base.kinetic_energy(&angular_state).unwrap();

        for scale in [1.0e-12, 1.0, 1.0e12] {
            let scaled =
                BivectorInertia::<3>::from_second_moment(base_second_moment * scale).unwrap();
            let scaled_momentum = scaled.angular_momentum(&angular_state).unwrap();
            assert!(
                (scaled_momentum.add(&base_momentum.scale(-scale))).norm()
                    <= 2.0e-11 * base_momentum.norm().max(1.0) * scale,
                "momentum mass-scaling mismatch at {scale:e}"
            );
            let recovered = scaled
                .angular_velocity_from_momentum(&scaled_momentum, 1.0e-12)
                .unwrap();
            assert!(
                recovered.add(&angular_state.scale(-1.0)).norm() < 2.0e-11,
                "angular state changed under mass scale {scale:e}"
            );
            let scaled_energy = scaled.kinetic_energy(&angular_state).unwrap();
            let relative_energy_error = (scaled_energy / (base_energy * scale) - 1.0).abs();
            assert!(
                relative_energy_error < 2.0e-11,
                "energy mass-scaling mismatch at {scale:e}: relative={relative_energy_error:e}"
            );
        }
    }

    #[test]
    fn rotational_inverse_effective_mass_scales_inversely_with_mass() {
        let base = BivectorInertia::<3>::uniform_solid_ellipsoid(2.0, SVector::new(1.4, 0.9, 0.5))
            .unwrap();
        let offset = SVector::new(0.7, -0.25, 0.4);
        let direction = SVector::new(-0.2, 0.9, 0.35).normalize();
        let base_value = base
            .rotational_inverse_effective_mass(&offset, &direction, 1e-12)
            .unwrap();

        for scale in [1.0e-12, 1.0, 1.0e12] {
            let scaled = BivectorInertia::<3>::uniform_solid_ellipsoid(
                2.0 * scale,
                SVector::new(1.4, 0.9, 0.5),
            )
            .unwrap();
            let observed = scaled
                .rotational_inverse_effective_mass(&offset, &direction, 1e-12)
                .unwrap();
            let expected = base_value / scale;
            assert!(
                ((observed - expected) / expected).abs() < 2.0e-11,
                "inverse effective mass has wrong mass scaling at {scale:e}: \
                 observed={observed:e}, expected={expected:e}"
            );
        }
    }

    #[cfg(not(feature = "nd-bivector-9"))]
    #[test]
    fn dimensions_five_to_nine_require_research_feature() {
        assert_eq!(
            BivectorInertia::<5>::from_second_moment(SMatrix::<f64, 5, 5>::identity()).unwrap_err(),
            RotationalInertiaError::InvalidDimension,
        );
        assert_eq!(
            BivectorInertia::<5>::from_point_masses([(SVector::<f64, 5>::zeros(), 1.0)])
                .unwrap_err(),
            RotationalInertiaError::InvalidDimension,
        );
    }

    #[test]
    fn operator_rejects_dimensions_outside_bivector_capacity() {
        assert_eq!(
            BivectorInertia::<1>::from_second_moment(SMatrix::<f64, 1, 1>::identity()).unwrap_err(),
            RotationalInertiaError::InvalidDimension
        );
        assert_eq!(
            BivectorInertia::<10>::from_second_moment(SMatrix::<f64, 10, 10>::identity())
                .unwrap_err(),
            RotationalInertiaError::InvalidDimension
        );
    }

    #[test]
    fn rotated_operator_rejects_improper_rotor() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(1.0, 1.0).unwrap();
        let mut reflection = SMatrix::<f64, 3, 3>::identity();
        reflection[(0, 0)] = -1.0;
        let invalid = Rotor::from_matrix(reflection);
        assert_eq!(
            inertia.rotated_by(&invalid).unwrap_err(),
            RotationalInertiaError::InvalidRotation
        );
    }

    #[test]
    fn finite_state_that_overflows_momentum_or_energy_is_rejected() {
        let inertia = BivectorInertia::<3>::uniform_solid_ball(1.0e200, 1.0e-50).unwrap();
        let huge = Bivector::<3>::unit_plane(0, 1).scale(1.0e300);
        assert_eq!(
            inertia.angular_momentum(&huge).unwrap_err(),
            RotationalInertiaError::NonFinite
        );
        assert_eq!(
            inertia.kinetic_energy(&huge).unwrap_err(),
            RotationalInertiaError::NonFinite
        );
    }

    #[test]
    fn rejects_indefinite_second_moment() {
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([1.0, 1.0, -0.01]));
        assert_eq!(
            BivectorInertia::<3>::from_second_moment(second_moment).unwrap_err(),
            RotationalInertiaError::NotPositiveSemidefinite
        );
    }

    #[test]
    fn constructor_projects_roundoff_sized_negative_eigenvalue_to_psd_boundary() {
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([1.0, 1.0, -5e-13]));
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let minimum_eigenvalue =
            nalgebra::linalg::SymmetricEigen::new(inertia.second_moment().clone())
                .eigenvalues
                .iter()
                .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        assert!(minimum_eigenvalue >= -1e-14);
        assert!(inertia.second_moment()[(2, 2)].abs() < 1e-14);
    }

    #[test]
    fn rejects_small_scale_indefinite_second_moment() {
        // PSD tolerance must scale with the physical mass-moment magnitude,
        // not with an arbitrary unit floor.
        let second_moment =
            SMatrix::<f64, 3, 3>::from_diagonal(&nalgebra::SVector::from([1e-20, 1e-20, -1e-15]));
        assert_eq!(
            BivectorInertia::<3>::from_second_moment(second_moment).unwrap_err(),
            RotationalInertiaError::NotPositiveSemidefinite
        );
    }

    #[test]
    fn rejects_non_symmetric_second_moment() {
        let second_moment = SMatrix::<f64, 3, 3>::new(1.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
        assert_eq!(
            BivectorInertia::<3>::from_second_moment(second_moment).unwrap_err(),
            RotationalInertiaError::NotSymmetric
        );
    }


    #[test]
    fn anisotropic_two_point_delassus_matches_tensor_oracle_and_is_frame_covariant() {
        let second_moment = SMatrix::<f64, 3, 3>::new(
            1.7, 0.21, -0.13,
            0.21, 1.1, 0.18,
            -0.13, 0.18, 0.8,
        );
        let inertia = BivectorInertia::<3>::from_second_moment(second_moment).unwrap();
        let mass = 2.3;
        let offsets = [
            SVector::new(0.6, -0.4, 0.2),
            SVector::new(-0.3, -0.4, 0.5),
        ];
        let normal = SVector::new(0.0, 1.0, 0.0);

        // Construct the complete two-contact Delassus operator by applying
        // each unit contact impulse through the same checked bivector inertia
        // used by the single-contact reference.
        let operator_metric = |operator: &BivectorInertia<3>,
                               offsets: &[SVector<f64, 3>; 2],
                               normal: &SVector<f64, 3>| {
            let mut matrix = SMatrix::<f64, 2, 2>::zeros();
            for column in 0..2 {
                let angular_impulse = Bivector::from_wedge(&offsets[column], normal);
                let angular_response = operator
                    .angular_velocity_from_momentum(&angular_impulse, 1.0e-12)
                    .unwrap();
                for row in 0..2 {
                    let point_velocity_change = *normal / mass
                        - angular_response.apply_to_vector(&offsets[row]);
                    matrix[(row, column)] = normal.dot(&point_velocity_change);
                }
            }
            matrix
        };

        // Independent conventional-vector oracle:
        // I = tr(S) 1 - S, tau_j = r_j x n, delta-v_i = n/m + omega_j x r_i.
        let tensor = SMatrix::<f64, 3, 3>::identity() * second_moment.trace() - second_moment;
        let tensor_factor = tensor.cholesky().expect("physical inertia tensor is SPD");
        let mut tensor_metric = SMatrix::<f64, 2, 2>::zeros();
        for column in 0..2 {
            let torque = offsets[column].cross(&normal);
            let omega = tensor_factor.solve(&torque);
            for row in 0..2 {
                let point_velocity_change = normal / mass + omega.cross(&offsets[row]);
                tensor_metric[(row, column)] = normal.dot(&point_velocity_change);
            }
        }

        let observed = operator_metric(&inertia, &offsets, &normal);
        let scale = observed.norm().max(f64::MIN_POSITIVE);
        assert!(
            (observed - tensor_metric).norm() / scale < 2.0e-11,
            "bivector and independent 3D tensor Delassus matrices disagree:              operator={observed:?}, tensor={tensor_metric:?}"
        );
        assert!(
            (observed - observed.transpose()).norm() / scale < 2.0e-12,
            "two-contact Delassus operator is not symmetric: {observed:?}"
        );
        assert!(
            observed.cholesky().is_some(),
            "validated anisotropic inertia must induce an SPD two-contact metric"
        );
        assert!(
            (observed[(0, 1)] - 1.0 / mass).abs() > 1.0e-3,
            "fixture must exercise a nonzero coupled rotational off-diagonal"
        );

        // A positive active-set solution is known exactly by construction.
        // This checks the small coupled KKT solve rather than validating two
        // unrelated scalar effective masses.
        let expected_impulses = SVector::new(0.7, 1.3);
        let free_normal_speeds = -(observed * expected_impulses);
        let recovered_impulses = observed
            .lu()
            .solve(&(-free_normal_speeds))
            .expect("SPD Delassus matrix has a unique active-set solution");
        let attained_speeds = free_normal_speeds + observed * recovered_impulses;
        assert!(
            (recovered_impulses - expected_impulses).norm() < 2.0e-11,
            "coupled active-set impulse did not recover the independent solution"
        );
        assert!(recovered_impulses.iter().all(|value| *value > 0.0));
        assert!(attained_speeds.norm() < 2.0e-11);

        // Rotating the body, both contact offsets, and the normal together
        // must not change the contact-space matrix.
        let mut frame_generator = Bivector::<3>::zero();
        frame_generator.set(0, 1, 0.37);
        frame_generator.set(0, 2, -0.24);
        frame_generator.set(1, 2, 0.19);
        let body_to_world = Rotor::from_bivector(&frame_generator);
        let world_inertia = inertia.rotated_by(&body_to_world).unwrap();
        let world_offsets = offsets.map(|offset| body_to_world.rotate_vector(&offset));
        let world_normal = body_to_world.rotate_vector(&normal);
        let world_observed = operator_metric(&world_inertia, &world_offsets, &world_normal);
        let world_scale = world_observed.norm().max(f64::MIN_POSITIVE);
        assert!(
            (world_observed - observed).norm() / world_scale < 2.0e-11,
            "two-contact Delassus matrix changed under global frame rotation"
        );
    }

}
