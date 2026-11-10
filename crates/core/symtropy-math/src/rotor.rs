// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use crate::bivector::Bivector;
use crate::point::Point;
use nalgebra::{SMatrix, SVector};

/// A rotor — performs rotations in D dimensions.
///
/// Internally stores the rotation matrix. This is optimal for the hot path
/// (applying rotations to many points) and avoids lossy rotor↔matrix round-trips.
/// Construction uses geometric algebra (bivectors, plane-angle), but once built
/// the matrix is the canonical representation.
///
/// Stack-allocated via `SMatrix<f64, D, D>`.
#[derive(Clone, Debug)]
pub struct Rotor<const D: usize> {
    mat: SMatrix<f64, D, D>,
}
/// Failure while admitting an externally supplied matrix as a proper rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RotorMatrixError {
    /// The requested validation tolerance was negative, NaN, or infinite.
    InvalidTolerance,
    /// At least one matrix entry was NaN or infinite.
    NonFiniteEntry,
    /// The matrix does not satisfy RᵀR = I within the requested tolerance.
    NotOrthogonal { max_error: f64 },
    /// The matrix is orthogonal but does not preserve orientation (det(R) != +1).
    ImproperOrientation { determinant: f64 },
}

impl std::fmt::Display for RotorMatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTolerance => write!(
                f,
                "rotation-matrix tolerance must be finite and non-negative"
            ),
            Self::NonFiniteEntry => write!(f, "rotation matrix contains a non-finite entry"),
            Self::NotOrthogonal { max_error } => {
                write!(
                    f,
                    "rotation matrix is not orthogonal: max residual {max_error:e}"
                )
            }
            Self::ImproperOrientation { determinant } => {
                write!(
                    f,
                    "orthogonal matrix does not preserve orientation: determinant {determinant:e}"
                )
            }
        }
    }
}

impl std::error::Error for RotorMatrixError {}

impl<const D: usize> Rotor<D> {
    /// Identity rotor (no rotation).
    pub fn identity() -> Self {
        Self {
            mat: SMatrix::identity(),
        }
    }

    /// Rotation by `angle` radians in the direction encoded by `plane`.
    ///
    /// For a simple unit plane such as `unit_plane(0, 1)`, positive angles
    /// rotate axis 0 toward axis 1. For a general bivector containing several
    /// independent planes (for example a 4D double rotation), the normalized
    /// bivector is exponentiated as one element of `so(D)` rather than being
    /// forced through the single-plane Rodrigues formula.
    pub fn from_plane_angle(plane: &Bivector<D>, angle: f64) -> Self {
        let norm = plane.norm();
        if norm < 1e-15 || angle.abs() < 1e-15 {
            return Self::identity();
        }

        Self::from_bivector(&plane.scale(angle / norm))
    }

    /// Exponentiate a bivector whose components are angular displacements in
    /// radians.
    ///
    /// This computes `exp(-B)` where `B` is the antisymmetric matrix associated
    /// with the bivector. The minus sign preserves Symtropy's convention that
    /// a positive `e01` rotation maps +x toward +y.
    ///
    /// The implementation uses scaling-and-squaring with a convergent Taylor
    /// series. Unlike Rodrigues' single-plane formula, this remains valid for
    /// simultaneous rotations in multiple independent planes.
    pub fn from_bivector(generator: &Bivector<D>) -> Self {
        if generator.norm() < 1e-15 {
            return Self::identity();
        }
        if !generator.is_finite() {
            debug_assert!(false, "non-finite bivector passed to Rotor::from_bivector");
            return Self::identity();
        }

        let mut a = -generator.to_matrix();

        // Scale until the matrix 1-norm is small enough for rapid Taylor
        // convergence, then square the result back to the original magnitude.
        let mut one_norm = 0.0_f64;
        for j in 0..D {
            let mut column_sum = 0.0;
            for i in 0..D {
                column_sum += a[(i, j)].abs();
            }
            one_norm = one_norm.max(column_sum);
        }

        let mut squarings = 0_u32;
        while one_norm > 0.5 && squarings < 60 {
            a *= 0.5;
            one_norm *= 0.5;
            squarings += 1;
        }

        let mut result = SMatrix::<f64, D, D>::identity();
        let mut term = SMatrix::<f64, D, D>::identity();
        for k in 1..=32 {
            term = (term * a) * (1.0 / k as f64);
            result += term;

            let mut max_term = 0.0_f64;
            for value in term.iter() {
                max_term = max_term.max(value.abs());
            }
            if max_term < 1e-16 {
                break;
            }
        }

        for _ in 0..squarings {
            result = result * result;
        }

        Self { mat: result }
    }

    /// Rotation from vector `from` to vector `to`.
    ///
    /// Inputs are normalized internally. Zero-length inputs return identity.
    /// Antiparallel vectors are handled by selecting a deterministic orthogonal
    /// direction and constructing a valid π rotation plane.
    pub fn from_vectors(from: &SVector<f64, D>, to: &SVector<f64, D>) -> Self {
        let from_norm = from.norm();
        let to_norm = to.norm();
        if from_norm < 1e-15 || to_norm < 1e-15 {
            return Self::identity();
        }

        let from_unit = from / from_norm;
        let to_unit = to / to_norm;
        let dot = from_unit.dot(&to_unit).clamp(-1.0, 1.0);

        if dot > 1.0 - 1e-14 {
            return Self::identity();
        }

        if dot < -1.0 + 1e-12 {
            // Choose the coordinate axis least aligned with `from`, then remove
            // its projection onto `from`. This is deterministic and works in
            // every D >= 2 without relying on a 3D cross product.
            if D < 2 {
                return Self::identity();
            }
            let axis = (0..D)
                .min_by(|&i, &j| from_unit[i].abs().total_cmp(&from_unit[j].abs()))
                .expect("D >= 2");
            let mut orthogonal = SVector::<f64, D>::zeros();
            orthogonal[axis] = 1.0;
            orthogonal -= from_unit * from_unit.dot(&orthogonal);
            let orthogonal_norm = orthogonal.norm();
            if orthogonal_norm < 1e-15 {
                return Self::identity();
            }
            orthogonal /= orthogonal_norm;
            let plane = Bivector::from_wedge(&from_unit, &orthogonal);
            return Self::from_plane_angle(&plane, std::f64::consts::PI);
        }

        let plane = Bivector::from_wedge(&from_unit, &to_unit);
        Self::from_plane_angle(&plane, dot.acos())
    }

    /// Validate and construct from a matrix in SO(D).
    ///
    /// Admission requires finite entries, orthogonality (RᵀR ≈ I), and
    /// orientation preservation (det(R) ≈ +1), each under the same
    /// absolute tolerance. Use this at external-input boundaries.
    /// The unchecked from_matrix constructor remains available for callers
    /// that have already established the invariant by construction.
    pub fn try_from_matrix(
        mat: SMatrix<f64, D, D>,
        tolerance: f64,
    ) -> Result<Self, RotorMatrixError> {
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(RotorMatrixError::InvalidTolerance);
        }
        if !mat.iter().all(|value| value.is_finite()) {
            return Err(RotorMatrixError::NonFiniteEntry);
        }

        let candidate = Self { mat };
        let max_error = candidate.orthogonality_error();
        if !max_error.is_finite() || max_error > tolerance {
            return Err(RotorMatrixError::NotOrthogonal { max_error });
        }

        let determinant = candidate.determinant();
        if !determinant.is_finite() || (determinant - 1.0).abs() > tolerance {
            return Err(RotorMatrixError::ImproperOrientation { determinant });
        }

        Ok(candidate)
    }

    /// Construct directly from a rotation matrix without checking it.
    ///
    /// The caller must guarantee orthogonality and det(R)=+1. Prefer
    /// try_from_matrix for externally supplied matrices.
    pub fn from_matrix(mat: SMatrix<f64, D, D>) -> Self {
        Self { mat }
    }

    /// Get the rotation matrix.
    #[inline]
    pub fn to_matrix(&self) -> &SMatrix<f64, D, D> {
        &self.mat
    }

    /// Maximum absolute entry of `RᵀR - I`.
    ///
    /// A proper rotation should keep this close to zero. This diagnostic is
    /// intentionally allocation-free so research harnesses can sample it on
    /// every tick when validating long integrations.
    pub fn orthogonality_error(&self) -> f64 {
        let residual = self.mat.transpose() * self.mat - SMatrix::<f64, D, D>::identity();
        residual
            .iter()
            .fold(0.0_f64, |max_error, value| max_error.max(value.abs()))
    }

    /// Determinant of the stored matrix. Proper rotations have determinant +1.
    ///
    /// Uses small dense Gaussian elimination instead of nalgebra's generic LU
    /// bounds, keeping this method available for every supported const-generic
    /// dimension.
    pub fn determinant(&self) -> f64 {
        let mut matrix = self.mat;
        let mut determinant = 1.0;

        for column in 0..D {
            let mut pivot = column;
            for row in (column + 1)..D {
                if matrix[(row, column)].abs() > matrix[(pivot, column)].abs() {
                    pivot = row;
                }
            }

            let pivot_value = matrix[(pivot, column)];
            if pivot_value.abs() < 1e-15 {
                return 0.0;
            }

            if pivot != column {
                matrix.swap_rows(pivot, column);
                determinant = -determinant;
            }

            let diagonal = matrix[(column, column)];
            determinant *= diagonal;
            for row in (column + 1)..D {
                let factor = matrix[(row, column)] / diagonal;
                for k in (column + 1)..D {
                    matrix[(row, k)] -= factor * matrix[(column, k)];
                }
            }
        }

        determinant
    }

    /// Validate both orthogonality and orientation preservation.
    pub fn is_proper_rotation(&self, tolerance: f64) -> bool {
        self.orthogonality_error() <= tolerance && (self.determinant() - 1.0).abs() <= tolerance
    }

    /// Reverse (inverse): R† = R^T for rotation matrices.
    #[inline]
    pub fn reverse(&self) -> Self {
        Self {
            mat: self.mat.transpose(),
        }
    }

    /// Compose: apply `other` first, then `self`.
    #[inline]
    pub fn compose(&self, other: &Self) -> Self {
        Self {
            mat: self.mat * other.mat,
        }
    }

    /// Rotate a point.
    #[inline]
    pub fn rotate_point(&self, point: &Point<D>) -> Point<D> {
        Point(self.mat * point.0)
    }

    /// Rotate a vector.
    #[inline]
    pub fn rotate_vector(&self, v: &SVector<f64, D>) -> SVector<f64, D> {
        self.mat * v
    }

    /// Slerp: identity at t=0, this rotation at t=1.
    ///
    /// Extracts the rotation angle and plane from the matrix,
    /// then builds a partial rotation.
    pub fn slerp(&self, t: f64) -> Self {
        if t <= 1e-14 {
            return Self::identity();
        }
        if (t - 1.0).abs() <= 1e-14 {
            return self.clone();
        }

        // The antisymmetric part of M is -sin(θ)*B̂ (due to our sign convention)
        let antisym = (self.mat - self.mat.transpose()) / 2.0;
        // Negate to get sin(θ)*B̂, then extract plane
        let plane = Bivector::from_matrix(&(-antisym));
        let sin_theta = plane.norm();

        if sin_theta < 1e-14 {
            return Self::identity();
        }

        let trace = self.mat.trace();
        let cos_theta = ((trace - (D as f64 - 2.0)) / 2.0).clamp(-1.0, 1.0);
        let theta = f64::atan2(sin_theta, cos_theta);

        Self::from_plane_angle(&plane, theta * t)
    }
}

impl<const D: usize> Default for Rotor<D> {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-10
    }

    fn approx_vec<const N: usize>(a: &SVector<f64, N>, b: &SVector<f64, N>) -> bool {
        a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1e-10)
    }

    #[test]
    fn identity_preserves_point() {
        let r = Rotor::<4>::identity();
        let p = Point::new([1.0, 2.0, 3.0, 4.0]);
        assert_eq!(r.rotate_point(&p), p);
    }

    #[test]
    fn rotation_2d_90() {
        let plane = Bivector::<2>::unit_plane(0, 1);
        let r = Rotor::from_plane_angle(&plane, FRAC_PI_2);
        let p = Point::new([1.0, 0.0]);
        let q = r.rotate_point(&p);
        assert!(approx_eq(q.coord(0), 0.0));
        assert!(approx_eq(q.coord(1), 1.0));
    }

    #[test]
    fn rotation_3d_xy_plane() {
        let plane = Bivector::<3>::unit_plane(0, 1);
        let r = Rotor::from_plane_angle(&plane, FRAC_PI_2);
        let p = Point::new([1.0, 0.0, 0.0]);
        let q = r.rotate_point(&p);
        assert!(approx_eq(q.coord(0), 0.0));
        assert!(approx_eq(q.coord(1), 1.0));
        assert!(approx_eq(q.coord(2), 0.0));
    }

    #[test]
    fn preserves_distance() {
        let plane = Bivector::<4>::unit_plane(1, 3);
        let r = Rotor::from_plane_angle(&plane, 1.0);
        let p = Point::new([1.0, 2.0, 3.0, 4.0]);
        let q = r.rotate_point(&p);
        assert!(approx_eq(p.0.norm(), q.0.norm()));
    }

    #[test]
    fn full_rotation() {
        let plane = Bivector::<3>::unit_plane(0, 2);
        let r = Rotor::from_plane_angle(&plane, 2.0 * PI);
        let p = Point::new([1.0, 2.0, 3.0]);
        let q = r.rotate_point(&p);
        assert!(approx_vec(&p.0, &q.0));
    }

    #[test]
    fn compose_additive() {
        let plane = Bivector::<3>::unit_plane(0, 1);
        let r1 = Rotor::from_plane_angle(&plane, FRAC_PI_4);
        let r2 = Rotor::from_plane_angle(&plane, FRAC_PI_4);
        let composed = r1.compose(&r2);
        let direct = Rotor::from_plane_angle(&plane, FRAC_PI_2);

        let p = Point::new([1.0, 0.0, 0.0]);
        assert!(approx_vec(
            &composed.rotate_point(&p).0,
            &direct.rotate_point(&p).0
        ));
    }

    #[test]
    fn reverse_undoes() {
        let plane = Bivector::<4>::unit_plane(0, 3);
        let r = Rotor::from_plane_angle(&plane, 1.7);
        let p = Point::new([1.0, 2.0, 3.0, 4.0]);
        let q = r.rotate_point(&p);
        let back = r.reverse().rotate_point(&q);
        assert!(approx_vec(&p.0, &back.0));
    }

    #[test]
    fn from_vectors_3d() {
        let from = SVector::from([1.0, 0.0, 0.0]);
        let to = SVector::from([0.0, 1.0, 0.0]);
        let r = Rotor::<3>::from_vectors(&from, &to);
        assert!(approx_vec(&r.rotate_vector(&from), &to));
    }

    #[test]
    fn from_vectors_4d() {
        let from = SVector::from([1.0, 0.0, 0.0, 0.0]);
        let to = SVector::from([0.0, 0.0, 0.0, 1.0]);
        let r = Rotor::<4>::from_vectors(&from, &to);
        assert!(approx_vec(&r.rotate_vector(&from), &to));
    }

    #[test]
    fn orthogonal_matrix() {
        let plane = Bivector::<4>::unit_plane(0, 2);
        let r = Rotor::from_plane_angle(&plane, 1.23);
        let mat = r.to_matrix();
        let product = mat * mat.transpose();
        let identity = SMatrix::<f64, 4, 4>::identity();
        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    (product[(i, j)] - identity[(i, j)]).abs() < 1e-10,
                    "not orthogonal at ({i},{j})"
                );
            }
        }
    }

    #[test]
    fn det_is_one() {
        let plane = Bivector::<3>::unit_plane(0, 1);
        let r = Rotor::from_plane_angle(&plane, 0.5);
        assert!(approx_eq(r.to_matrix().determinant(), 1.0));
    }

    #[test]
    fn try_from_matrix_accepts_proper_rotations_in_3d_and_4d() {
        let rotation_3d = Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), 0.73);
        let admitted_3d = Rotor::try_from_matrix(*rotation_3d.to_matrix(), 1e-10).unwrap();
        assert!(admitted_3d.is_proper_rotation(1e-10));

        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, 0.4);
        generator.set(2, 3, -0.9);
        let rotation_4d = Rotor::from_bivector(&generator);
        let admitted_4d = Rotor::try_from_matrix(*rotation_4d.to_matrix(), 1e-10).unwrap();
        assert!(admitted_4d.is_proper_rotation(1e-10));
    }

    #[test]
    fn try_from_matrix_rejects_non_orthogonal_matrix() {
        let mut matrix = SMatrix::<f64, 3, 3>::identity();
        matrix[(0, 1)] = 0.25;
        assert!(matches!(
            Rotor::try_from_matrix(matrix, 1e-10),
            Err(RotorMatrixError::NotOrthogonal { .. })
        ));
    }

    #[test]
    fn try_from_matrix_rejects_reflection() {
        let mut reflection = SMatrix::<f64, 3, 3>::identity();
        reflection[(0, 0)] = -1.0;
        assert!(matches!(
            Rotor::try_from_matrix(reflection, 1e-10),
            Err(RotorMatrixError::ImproperOrientation { determinant })
                if (determinant + 1.0).abs() < 1e-12
        ));
    }

    #[test]
    fn try_from_matrix_rejects_non_finite_entries() {
        let mut matrix = SMatrix::<f64, 3, 3>::identity();
        matrix[(1, 2)] = f64::NAN;
        assert!(matches!(
            Rotor::try_from_matrix(matrix, 1e-10),
            Err(RotorMatrixError::NonFiniteEntry)
        ));
    }

    #[test]
    fn try_from_matrix_rejects_invalid_tolerance() {
        assert!(matches!(
            Rotor::try_from_matrix(SMatrix::<f64, 3, 3>::identity(), f64::NAN),
            Err(RotorMatrixError::InvalidTolerance)
        ));
        assert!(matches!(
            Rotor::try_from_matrix(SMatrix::<f64, 3, 3>::identity(), -1.0),
            Err(RotorMatrixError::InvalidTolerance)
        ));
    }

    fn assert_simple_plane_rotations_are_proper<const D: usize>() {
        let point = SVector::<f64, D>::from_fn(|i, _| (i as f64 + 1.0) / D as f64);
        for i in 0..D {
            for j in (i + 1)..D {
                let plane = Bivector::<D>::unit_plane(i, j);
                let angle = 0.11 + (i + j) as f64 * 0.071;
                let rotation = Rotor::from_plane_angle(&plane, angle);
                assert!(
                    rotation.is_proper_rotation(1e-9),
                    "SO({D}) plane ({i},{j}) failed: orthogonality={}, determinant={}",
                    rotation.orthogonality_error(),
                    rotation.determinant()
                );

                let rotated = rotation.rotate_vector(&point);
                assert!((rotated.norm() - point.norm()).abs() < 1e-9);
                let recovered = rotation.reverse().rotate_vector(&rotated);
                assert!((recovered - point).norm() < 1e-9);

                let admitted = Rotor::try_from_matrix(*rotation.to_matrix(), 1e-9)
                    .expect("a constructed proper rotation must pass matrix admission");
                assert!(admitted.is_proper_rotation(1e-9));
            }
        }
    }

    #[test]
    fn every_supported_dimension_preserves_rotor_group_invariants() {
        // Bivector currently supports D=2..=9. Exercise every coordinate plane,
        // not only the common 2D/3D/4D specializations.
        assert_simple_plane_rotations_are_proper::<2>();
        assert_simple_plane_rotations_are_proper::<3>();
        assert_simple_plane_rotations_are_proper::<4>();
        assert_simple_plane_rotations_are_proper::<5>();
        assert_simple_plane_rotations_are_proper::<6>();
        assert_simple_plane_rotations_are_proper::<7>();
        assert_simple_plane_rotations_are_proper::<8>();
        assert_simple_plane_rotations_are_proper::<9>();
    }

    fn assert_adjoint_equivariance<const D: usize>() {
        let mut frame_generator = Bivector::<D>::zero();
        let mut body_generator = Bivector::<D>::zero();
        for i in 0..D {
            for j in (i + 1)..D {
                let parity = if (i + j) % 2 == 0 { 1.0 } else { -1.0 };
                frame_generator.set(i, j, parity * 0.009 * (i + 1) as f64 * (j + 2) as f64);
                body_generator.set(i, j, -parity * 0.007 * (i + 2) as f64 * (j + 1) as f64);
            }
        }

        let frame = Rotor::from_bivector(&frame_generator);
        let original_exponential = Rotor::from_bivector(&body_generator);
        let conjugated = frame
            .compose(&original_exponential)
            .compose(&frame.reverse());

        let transformed_matrix =
            frame.to_matrix() * body_generator.to_matrix() * frame.to_matrix().transpose();
        let transformed_generator = Bivector::<D>::from_matrix(&transformed_matrix);
        let exponential_after_adjoint = Rotor::from_bivector(&transformed_generator);

        let residual = *conjugated.to_matrix() - *exponential_after_adjoint.to_matrix();
        let max_error = residual.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
        assert!(
            max_error < 1e-8,
            "SO({D}) adjoint-equivariance residual={max_error:e}"
        );
        assert!(conjugated.is_proper_rotation(1e-8));
        assert!(exponential_after_adjoint.is_proper_rotation(1e-8));
    }

    #[test]
    fn bivector_exponential_is_adjoint_equivariant_in_dimensions_three_through_nine() {
        assert_adjoint_equivariance::<3>();
        assert_adjoint_equivariance::<4>();
        assert_adjoint_equivariance::<5>();
        assert_adjoint_equivariance::<6>();
        assert_adjoint_equivariance::<7>();
        assert_adjoint_equivariance::<8>();
        assert_adjoint_equivariance::<9>();
    }
    #[test]
    fn bivector_exponential_respects_adjoint_equivariance() {
        // The exponential map commutes with the adjoint action:
        // Q exp(B) Q⁻¹ = exp(Q B Q⁻¹). This is a core SO(D) frame-covariance
        // property and catches basis/order/sign errors in bivector conversion.
        let mut frame_generator = Bivector::<4>::zero();
        frame_generator.set(0, 2, 0.31);
        frame_generator.set(1, 3, -0.47);
        frame_generator.set(0, 1, 0.19);
        let frame = Rotor::from_bivector(&frame_generator);

        let mut body_generator = Bivector::<4>::zero();
        body_generator.set(0, 3, -0.62);
        body_generator.set(1, 2, 0.28);
        body_generator.set(0, 2, 0.11);

        let original_exponential = Rotor::from_bivector(&body_generator);
        let conjugated = frame
            .compose(&original_exponential)
            .compose(&frame.reverse());

        let transformed_matrix =
            frame.to_matrix() * body_generator.to_matrix() * frame.to_matrix().transpose();
        let transformed_generator = Bivector::<4>::from_matrix(&transformed_matrix);
        let exponential_after_adjoint = Rotor::from_bivector(&transformed_generator);

        let residual = conjugated.to_matrix() - exponential_after_adjoint.to_matrix();
        let max_error = residual.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
        assert!(
            max_error < 1e-9,
            "SO(4) adjoint-equivariance residual={max_error:e}"
        );
        assert!(conjugated.is_proper_rotation(1e-9));
        assert!(exponential_after_adjoint.is_proper_rotation(1e-9));
    }

    #[test]
    fn slerp_endpoints() {
        let plane = Bivector::<3>::unit_plane(0, 1);
        let r = Rotor::from_plane_angle(&plane, 1.0);
        let p = Point::new([1.0, 0.0, 0.0]);

        let at_0 = r.slerp(0.0).rotate_point(&p);
        assert!(approx_vec(&at_0.0, &p.0));

        let at_1 = r.slerp(1.0).rotate_point(&p);
        assert!(approx_vec(&at_1.0, &r.rotate_point(&p).0));
    }

    #[test]
    fn compose_different_planes() {
        // Compose rotations in different planes
        let r1 = Rotor::from_plane_angle(&Bivector::<4>::unit_plane(0, 1), 0.3);
        let r2 = Rotor::from_plane_angle(&Bivector::<4>::unit_plane(2, 3), 0.5);
        let composed = r2.compose(&r1);

        let p = Point::new([1.0, 1.0, 1.0, 1.0]);
        let sequential = r2.rotate_point(&r1.rotate_point(&p));
        let via_compose = composed.rotate_point(&p);
        assert!(approx_vec(&sequential.0, &via_compose.0));
    }

    #[test]
    fn slerp_midpoint() {
        let plane = Bivector::<3>::unit_plane(0, 1);
        let r = Rotor::from_plane_angle(&plane, FRAC_PI_2);
        let half = r.slerp(0.5);
        let expected = Rotor::from_plane_angle(&plane, FRAC_PI_4);

        let p = Point::new([1.0, 0.0, 0.0]);
        assert!(approx_vec(
            &half.rotate_point(&p).0,
            &expected.rotate_point(&p).0
        ));
    }
    #[test]
    fn antiparallel_vectors_rotate_correctly_3d() {
        let from = SVector::from([1.0, 0.0, 0.0]);
        let to = SVector::from([-1.0, 0.0, 0.0]);
        let r = Rotor::<3>::from_vectors(&from, &to);
        assert!(approx_vec(&r.rotate_vector(&from), &to));
        assert!(r.is_proper_rotation(1e-10));
    }

    #[test]
    fn antiparallel_vectors_rotate_correctly_4d() {
        let from = SVector::from([0.0, 0.0, 1.0, 0.0]);
        let to = SVector::from([0.0, 0.0, -1.0, 0.0]);
        let r = Rotor::<4>::from_vectors(&from, &to);
        assert!(approx_vec(&r.rotate_vector(&from), &to));
        assert!(r.is_proper_rotation(1e-10));
    }

    #[test]
    fn general_4d_double_rotation_is_proper() {
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, 0.7);
        generator.set(2, 3, -1.1);
        let r = Rotor::from_bivector(&generator);

        assert!(
            r.is_proper_rotation(1e-10),
            "orthogonality={}, determinant={}",
            r.orthogonality_error(),
            r.determinant()
        );

        let p = Point::new([1.0, 2.0, 3.0, 4.0]);
        let q = r.rotate_point(&p);
        assert!(approx_eq(p.0.norm(), q.0.norm()));
    }

    #[test]
    fn commuting_4d_planes_match_composition() {
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, 0.35);
        generator.set(2, 3, -0.8);
        let combined = Rotor::from_bivector(&generator);

        let xy = Rotor::from_plane_angle(&Bivector::<4>::unit_plane(0, 1), 0.35);
        let zw = Rotor::from_plane_angle(&Bivector::<4>::unit_plane(2, 3), -0.8);
        let composed = zw.compose(&xy);

        let p = Point::new([0.3, -1.2, 2.5, 0.75]);
        assert!(approx_vec(
            &combined.rotate_point(&p).0,
            &composed.rotate_point(&p).0
        ));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // A random rotation in a random 3D plane must always be orthogonal
    // (R * R^T = I) -- generalizes the fixed-example `orthogonal_matrix`
    // unit test. This is the property `Rotor` exists to guarantee: any
    // break here means rigid bodies would gain or lose length under
    // rotation, an energy-conservation violation.
    proptest! {
        #[test]
        fn random_rotation_3d_is_orthogonal(
            i in 0usize..3, j in 0usize..3,
            angle in -6.3f64..6.3,
        ) {
            prop_assume!(i < j);
            let plane = Bivector::<3>::unit_plane(i, j);
            let r = Rotor::from_plane_angle(&plane, angle);
            let mat = r.to_matrix();
            let product = mat * mat.transpose();
            let identity = SMatrix::<f64, 3, 3>::identity();
            for a in 0..3 {
                for b in 0..3 {
                    prop_assert!(
                        (product[(a, b)] - identity[(a, b)]).abs() < 1e-9,
                        "not orthogonal at ({a},{b})"
                    );
                }
            }
        }
    }

    proptest! {
        /// Same orthogonality property in 4D, where rotation planes don't
        /// have a unique orthogonal-complement axis (unlike 3D) -- this is
        /// exactly the regime where an off-by-one in the D-dimensional
        /// Rodrigues formula would most likely surface.
        #[test]
        fn random_rotation_4d_is_orthogonal(
            i in 0usize..4, j in 0usize..4,
            angle in -6.3f64..6.3,
        ) {
            prop_assume!(i < j);
            let plane = Bivector::<4>::unit_plane(i, j);
            let r = Rotor::from_plane_angle(&plane, angle);
            let mat = r.to_matrix();
            let product = mat * mat.transpose();
            let identity = SMatrix::<f64, 4, 4>::identity();
            for a in 0..4 {
                for b in 0..4 {
                    prop_assert!(
                        (product[(a, b)] - identity[(a, b)]).abs() < 1e-9,
                        "not orthogonal at ({a},{b})"
                    );
                }
            }
        }
    }

    proptest! {
        /// Determinant must be exactly +1 for any random rotation -- rules
        /// out reflections (det=-1), which would silently mirror geometry
        /// (e.g. a body's handedness) instead of rotating it.
        #[test]
        fn random_rotation_det_is_one(
            i in 0usize..3, j in 0usize..3,
            angle in -6.3f64..6.3,
        ) {
            prop_assume!(i < j);
            let plane = Bivector::<3>::unit_plane(i, j);
            let r = Rotor::from_plane_angle(&plane, angle);
            prop_assert!((r.to_matrix().determinant() - 1.0).abs() < 1e-9);
        }
    }

    proptest! {
        /// Rotation preserves distance from the origin for any point/plane/
        /// angle combination -- generalizes `preserves_distance`.
        #[test]
        fn random_rotation_preserves_distance(
            i in 0usize..4, j in 0usize..4,
            angle in -6.3f64..6.3,
            px in -10.0f64..10.0, py in -10.0f64..10.0, pz in -10.0f64..10.0, pw in -10.0f64..10.0,
        ) {
            prop_assume!(i < j);
            let plane = Bivector::<4>::unit_plane(i, j);
            let r = Rotor::from_plane_angle(&plane, angle);
            let p = Point::new([px, py, pz, pw]);
            let q = r.rotate_point(&p);
            prop_assert!((p.0.norm() - q.0.norm()).abs() < 1e-9);
        }
    }

    proptest! {
        /// `reverse()` must always undo the rotation, for any plane/angle/
        /// point -- generalizes `reverse_undoes`. This is the identity the
        /// physics solver's warm-starting/contact-basis code relies on.
        #[test]
        fn random_reverse_undoes_rotation(
            i in 0usize..3, j in 0usize..3,
            angle in -6.3f64..6.3,
            px in -10.0f64..10.0, py in -10.0f64..10.0, pz in -10.0f64..10.0,
        ) {
            prop_assume!(i < j);
            let plane = Bivector::<3>::unit_plane(i, j);
            let r = Rotor::from_plane_angle(&plane, angle);
            let p = Point::new([px, py, pz]);
            let q = r.rotate_point(&p);
            let back = r.reverse().rotate_point(&q);
            prop_assert!((back.0 - p.0).norm() < 1e-8);
        }
    }

    proptest! {
        /// Composing two random rotations must match applying them
        /// sequentially, for any pair of planes/angles and any point --
        /// generalizes `compose_different_planes`.
        #[test]
        fn random_compose_matches_sequential(
            pair1 in 0usize..6, angle1 in -6.3f64..6.3,
            pair2 in 0usize..6, angle2 in -6.3f64..6.3,
            px in -10.0f64..10.0, py in -10.0f64..10.0, pz in -10.0f64..10.0, pw in -10.0f64..10.0,
        ) {
            // Index directly into the 6 valid (i,j) pairs for D=4 instead of
            // generating i/j independently and `prop_assume!`-rejecting
            // invalid (i>=j) combinations: with 2 independent (i,j) pairs
            // that filter, only ~14% of cases pass, which blew through
            // proptest's global-reject budget ("Too many global rejects")
            // rather than ever reaching the actual property under test.
            const PLANES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
            let (i1, j1) = PLANES[pair1];
            let (i2, j2) = PLANES[pair2];
            let r1 = Rotor::from_plane_angle(&Bivector::<4>::unit_plane(i1, j1), angle1);
            let r2 = Rotor::from_plane_angle(&Bivector::<4>::unit_plane(i2, j2), angle2);
            let composed = r2.compose(&r1);

            let p = Point::new([px, py, pz, pw]);
            let sequential = r2.rotate_point(&r1.rotate_point(&p));
            let via_compose = composed.rotate_point(&p);
            prop_assert!((sequential.0 - via_compose.0).norm() < 1e-8);
        }
    }
}
