// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use nalgebra::{SMatrix, SVector};

/// Maximum supported dimension for bivectors.
/// D=4 → 6, D=5 → 10, D=6 → 15, D=7 → 21, D=8 → 28, D=9 → 36.
/// Supports up to 9D physics for dimensional sweep experiments.
const MAX_BIVECTOR_COMPONENTS: usize = 36;

/// A bivector in D-dimensional space (grade-2 element of the geometric algebra).
///
/// Bivectors represent oriented planes. In 3D they are dual to pseudovectors.
/// In ND they properly represent rotations: a rotation happens IN a plane.
///
/// **Stack-allocated**: uses `[f64; 6]` fixed array (max for D=4).
/// Zero heap allocation — safe for physics hot loops at 60+ Hz.
///
/// Component ordering: lexicographic (i,j) with i < j.
/// For D=4: [e01, e02, e03, e12, e13, e23].
#[derive(Clone, Copy, Debug)]
pub struct Bivector<const D: usize> {
    components: [f64; MAX_BIVECTOR_COMPONENTS],
}

// Compile-time assertion: D must be ≤ 9 (36 bivector components max)
const fn _assert_d_le_9<const D: usize>() {
    assert!(
        D * (D - 1) / 2 <= MAX_BIVECTOR_COMPONENTS,
        "Bivector supports D <= 9 (max 36 components)"
    );
}

impl<const D: usize> Bivector<D> {
    /// Number of independent components: D*(D-1)/2.
    pub const fn num_components() -> usize {
        D * (D - 1) / 2
    }

    /// Zero bivector.
    #[inline]
    pub fn zero() -> Self {
        _assert_d_le_9::<D>();
        Self {
            components: [0.0; MAX_BIVECTOR_COMPONENTS],
        }
    }

    /// Unit bivector in the plane spanned by axes i and j (i < j).
    pub fn unit_plane(i: usize, j: usize) -> Self {
        assert!(i < j && j < D, "requires i < j < D");
        let mut bv = Self::zero();
        bv.set(i, j, 1.0);
        bv
    }

    /// Flat index for the (i,j) component with i < j.
    #[inline]
    fn index(i: usize, j: usize) -> usize {
        debug_assert!(i < j && j < D);
        i * D - i * (i + 1) / 2 + j - i - 1
    }

    /// Get the component for the (i, j) plane (i < j).
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.components[Self::index(i, j)]
    }

    /// Set the component for the (i, j) plane (i < j).
    #[inline]
    pub fn set(&mut self, i: usize, j: usize, value: f64) {
        self.components[Self::index(i, j)] = value;
    }

    /// Convert to antisymmetric matrix representation.
    pub fn to_matrix(&self) -> SMatrix<f64, D, D> {
        let mut mat = SMatrix::zeros();
        for i in 0..D {
            for j in (i + 1)..D {
                let val = self.get(i, j);
                mat[(i, j)] = val;
                mat[(j, i)] = -val;
            }
        }
        mat
    }

    /// Construct from an antisymmetric matrix.
    pub fn from_matrix(mat: &SMatrix<f64, D, D>) -> Self {
        let mut bv = Self::zero();
        for i in 0..D {
            for j in (i + 1)..D {
                bv.set(i, j, (mat[(i, j)] - mat[(j, i)]) / 2.0);
            }
        }
        bv
    }

    /// Returns true if all components are finite (not NaN or infinite).
    #[inline]
    pub fn is_finite(&self) -> bool {
        let n = Self::num_components();
        for i in 0..n {
            if !self.components[i].is_finite() {
                return false;
            }
        }
        true
    }

    /// Squared norm.
    #[inline]
    pub fn norm_squared(&self) -> f64 {
        let n = Self::num_components();
        let mut sum = 0.0;
        for i in 0..n {
            sum += self.components[i] * self.components[i];
        }
        sum
    }

    /// Norm.
    #[inline]
    pub fn norm(&self) -> f64 {
        self.norm_squared().sqrt()
    }

    /// Normalize to unit bivector. Returns None if zero.
    pub fn normalized(&self) -> Option<Self> {
        let n = self.norm();
        if n < 1e-15 {
            return None;
        }
        Some(self.scale(1.0 / n))
    }

    /// Scale all components by a scalar.
    #[inline]
    pub fn scale(&self, s: f64) -> Self {
        let mut result = *self;
        let n = Self::num_components();
        for i in 0..n {
            result.components[i] *= s;
        }
        result
    }

    /// Add two bivectors.
    #[inline]
    pub fn add(&self, other: &Self) -> Self {
        let mut result = *self;
        let n = Self::num_components();
        for i in 0..n {
            result.components[i] += other.components[i];
        }
        result
    }

    /// Apply this bivector (as angular velocity) to a vector.
    /// Returns B · v (antisymmetric matrix × vector).
    #[inline]
    pub fn apply_to_vector(&self, v: &SVector<f64, D>) -> SVector<f64, D> {
        self.to_matrix() * v
    }

    /// Wedge product `a ∧ b`: the bivector representing the oriented plane
    /// spanned by `a` and `b`. Component `(i, j) = a_i*b_j - a_j*b_i`.
    ///
    /// This is the dimension-agnostic replacement for the 3D cross product
    /// used to compute torque/angular-impulse from a force/impulse applied
    /// at an offset from the center of mass.
    ///
    /// **Ordering matters** and is chosen to be consistent with
    /// [`Self::apply_to_vector`]: given an impulse `j` applied at an offset
    /// `r` from a body's center of mass, the correct torque bivector is
    /// `Bivector::from_wedge(&j, &r)` (impulse first, offset second) — NOT
    /// `from_wedge(&r, &j)`. With this ordering,
    /// `(inv_inertia * Bivector::from_wedge(&j, &r)).apply_to_vector(&r)`
    /// reproduces the standard `(inv_inertia * (r × j)) × r` lever-arm
    /// identity from 3D rigid body dynamics (verified in
    /// `wedge_matches_3d_lever_arm_identity` below); the reversed argument
    /// order produces the negated (physically wrong) sign.
    #[inline]
    pub fn from_wedge(a: &SVector<f64, D>, b: &SVector<f64, D>) -> Self {
        let mut bv = Self::zero();
        for i in 0..D {
            for j in (i + 1)..D {
                bv.set(i, j, a[i] * b[j] - a[j] * b[i]);
            }
        }
        bv
    }
}

impl<const D: usize> Default for Bivector<D> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<const D: usize> PartialEq for Bivector<D> {
    fn eq(&self, other: &Self) -> bool {
        let n = Self::num_components();
        for i in 0..n {
            if (self.components[i] - other.components[i]).abs() > 1e-14 {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_components() {
        assert_eq!(Bivector::<2>::num_components(), 1);
        assert_eq!(Bivector::<3>::num_components(), 3);
        assert_eq!(Bivector::<4>::num_components(), 6);
    }

    #[test]
    fn unit_plane_3d() {
        let bv = Bivector::<3>::unit_plane(0, 1);
        assert!((bv.get(0, 1) - 1.0).abs() < 1e-15);
        assert!(bv.get(0, 2).abs() < 1e-15);
        assert!(bv.get(1, 2).abs() < 1e-15);
    }

    #[test]
    fn matrix_roundtrip() {
        let mut bv = Bivector::<4>::zero();
        bv.set(0, 1, 1.0);
        bv.set(0, 3, -0.5);
        bv.set(2, 3, 0.7);
        let mat = bv.to_matrix();
        let recovered = Bivector::<4>::from_matrix(&mat);
        assert_eq!(bv, recovered);
    }

    #[test]
    fn antisymmetric_matrix() {
        let bv = Bivector::<3>::unit_plane(0, 2);
        let mat = bv.to_matrix();
        assert!((mat[(0, 2)] - 1.0).abs() < 1e-15);
        assert!((mat[(2, 0)] + 1.0).abs() < 1e-15);
    }

    #[test]
    fn angular_velocity_3d() {
        let bv = Bivector::<3>::unit_plane(0, 1);
        let v = SVector::from([1.0, 0.0, 0.0]);
        let result = bv.apply_to_vector(&v);
        assert!(result[0].abs() < 1e-15);
        assert!((result[1] + 1.0).abs() < 1e-15);
        assert!(result[2].abs() < 1e-15);
    }

    #[test]
    fn norm_and_normalize() {
        let mut bv = Bivector::<3>::zero();
        bv.set(0, 1, 3.0);
        bv.set(0, 2, 4.0);
        assert!((bv.norm() - 5.0).abs() < 1e-14);
        let unit = bv.normalized().unwrap();
        assert!((unit.norm() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn four_d_double_rotation() {
        let mut bv = Bivector::<4>::zero();
        bv.set(0, 1, 1.0);
        bv.set(2, 3, 1.0);
        assert!((bv.norm_squared() - 2.0).abs() < 1e-14);
    }

    #[test]
    fn is_copy() {
        let a = Bivector::<3>::unit_plane(0, 1);
        let b = a; // Copy, not move
        assert_eq!(a, b);
    }

    #[test]
    fn scale_no_alloc() {
        let bv = Bivector::<4>::unit_plane(0, 1);
        let scaled = bv.scale(3.0);
        assert!((scaled.get(0, 1) - 3.0).abs() < 1e-14);
        // Original unchanged (Copy semantics)
        assert!((bv.get(0, 1) - 1.0).abs() < 1e-14);
    }

    #[test]
    fn add_no_alloc() {
        let a = Bivector::<3>::unit_plane(0, 1);
        let b = Bivector::<3>::unit_plane(0, 2);
        let c = a.add(&b);
        assert!((c.get(0, 1) - 1.0).abs() < 1e-14);
        assert!((c.get(0, 2) - 1.0).abs() < 1e-14);
    }

    #[test]
    fn wedge_antisymmetric() {
        let a = SVector::from([1.0, 2.0, 3.0]);
        let b = SVector::from([4.0, -1.0, 0.5]);
        let ab = Bivector::<3>::from_wedge(&a, &b);
        let ba = Bivector::<3>::from_wedge(&b, &a);
        // a∧b = -(b∧a)
        for i in 0..3 {
            for j in (i + 1)..3 {
                assert!((ab.get(i, j) + ba.get(i, j)).abs() < 1e-14);
            }
        }
    }

    #[test]
    fn wedge_parallel_vectors_is_zero() {
        let a = SVector::from([2.0, 0.0, 0.0]);
        let b = SVector::from([5.0, 0.0, 0.0]);
        let bv = Bivector::<3>::from_wedge(&a, &b);
        assert!(bv.norm() < 1e-14, "parallel vectors should wedge to zero");
    }

    /// Verifies that `Bivector::from_wedge(&j, &r)` combined with
    /// `apply_to_vector` reproduces the standard 3D lever-arm identity
    /// `(inv_i * (r × j)) × r`, which for a unit-inertia body reduces to
    /// `j * |r|^2 - r * (r·j)` (BAC-CAB expansion). This is the physical
    /// contract the contact solver's angular-impulse code depends on: torque
    /// bivector must be built as `from_wedge(&impulse, &r)`, NOT
    /// `from_wedge(&r, &impulse)`.
    #[test]
    fn wedge_matches_3d_lever_arm_identity() {
        let r = SVector::from([1.0, 0.0, 0.0]);
        let j = SVector::from([0.0, 1.0, 0.0]); // impulse along +Y, offset along +X

        let torque = Bivector::<3>::from_wedge(&j, &r);
        let delta_v_rot = torque.apply_to_vector(&r); // unit inverse-inertia

        // Expected: j * |r|^2 - r * (r·j) = (0,1,0)*1 - (1,0,0)*0 = (0,1,0)
        let expected = j * r.norm_squared() - r * r.dot(&j);
        assert!(
            (delta_v_rot - expected).norm() < 1e-14,
            "delta_v_rot = {delta_v_rot:?}, expected = {expected:?}"
        );

        // The normal-direction (n = j here) component must be POSITIVE —
        // this is the "effective mass" contribution used by the contact
        // solver and must never be negative (would imply energy injection
        // if flipped when combined with the solver's sign convention).
        let n = j.normalize();
        assert!(
            delta_v_rot.dot(&n) > 0.0,
            "lever-arm contribution should be positive for this convention"
        );

        // Reversed argument order must give the negated (wrong) result.
        let reversed = Bivector::<3>::from_wedge(&r, &j).apply_to_vector(&r);
        assert!(
            (reversed + expected).norm() < 1e-14,
            "reversed wedge order should negate the result"
        );
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// `to_matrix()` must always be antisymmetric (zero diagonal, M[j,i] =
    /// -M[i,j]) for every possible bivector, not just the hand-picked
    /// examples in the unit tests above -- this is the algebraic property
    /// the whole rotor construction (`Rotor::from_plane_angle`) relies on.
    proptest! {
        #[test]
        fn to_matrix_is_antisymmetric(
            e01 in -10.0f64..10.0, e02 in -10.0f64..10.0, e03 in -10.0f64..10.0,
            e12 in -10.0f64..10.0, e13 in -10.0f64..10.0, e23 in -10.0f64..10.0,
        ) {
            let mut bv = Bivector::<4>::zero();
            bv.set(0, 1, e01);
            bv.set(0, 2, e02);
            bv.set(0, 3, e03);
            bv.set(1, 2, e12);
            bv.set(1, 3, e13);
            bv.set(2, 3, e23);

            let mat = bv.to_matrix();
            for i in 0..4 {
                prop_assert!((mat[(i, i)]).abs() < 1e-12, "diagonal must be zero");
                for j in (i + 1)..4 {
                    prop_assert!(
                        (mat[(i, j)] + mat[(j, i)]).abs() < 1e-10,
                        "M[{i},{j}]={} should be -M[{j},{i}]={}", mat[(i, j)], mat[(j, i)]
                    );
                }
            }
        }
    }

    proptest! {
        /// `from_matrix` must exactly invert `to_matrix` for any bivector --
        /// generalizes the single fixed-example `matrix_roundtrip` unit test.
        #[test]
        fn matrix_roundtrip_random(
            e01 in -10.0f64..10.0, e02 in -10.0f64..10.0, e03 in -10.0f64..10.0,
            e12 in -10.0f64..10.0, e13 in -10.0f64..10.0, e23 in -10.0f64..10.0,
        ) {
            let mut bv = Bivector::<4>::zero();
            bv.set(0, 1, e01);
            bv.set(0, 2, e02);
            bv.set(0, 3, e03);
            bv.set(1, 2, e12);
            bv.set(1, 3, e13);
            bv.set(2, 3, e23);

            let recovered = Bivector::<4>::from_matrix(&bv.to_matrix());
            prop_assert_eq!(bv, recovered);
        }
    }

    proptest! {
        /// `a ∧ b = -(b ∧ a)` for arbitrary (not just hand-picked) vectors --
        /// generalizes `wedge_antisymmetric`.
        #[test]
        fn wedge_antisymmetric_random(
            ax in -10.0f64..10.0, ay in -10.0f64..10.0, az in -10.0f64..10.0,
            bx in -10.0f64..10.0, by in -10.0f64..10.0, bz in -10.0f64..10.0,
        ) {
            let a = SVector::from([ax, ay, az]);
            let b = SVector::from([bx, by, bz]);
            let ab = Bivector::<3>::from_wedge(&a, &b);
            let ba = Bivector::<3>::from_wedge(&b, &a);
            prop_assert_eq!(ab, ba.scale(-1.0));
        }
    }

    proptest! {
        /// Bilinearity in the first argument: `(a1+a2) ∧ b = a1∧b + a2∧b`.
        /// This is a fundamental exterior-algebra identity that
        /// `from_wedge`'s per-component `a_i*b_j - a_j*b_i` formula must
        /// satisfy for the torque-accumulation code (which sums wedge
        /// products across multiple contact points) to be physically valid.
        #[test]
        fn wedge_bilinear_in_first_argument(
            a1x in -10.0f64..10.0, a1y in -10.0f64..10.0, a1z in -10.0f64..10.0,
            a2x in -10.0f64..10.0, a2y in -10.0f64..10.0, a2z in -10.0f64..10.0,
            bx in -10.0f64..10.0, by in -10.0f64..10.0, bz in -10.0f64..10.0,
        ) {
            let a1 = SVector::from([a1x, a1y, a1z]);
            let a2 = SVector::from([a2x, a2y, a2z]);
            let b = SVector::from([bx, by, bz]);

            let sum_then_wedge = Bivector::<3>::from_wedge(&(a1 + a2), &b);
            let wedge_then_sum = Bivector::<3>::from_wedge(&a1, &b).add(&Bivector::<3>::from_wedge(&a2, &b));
            // `Bivector`'s `PartialEq` uses a fixed 1e-14 *absolute* epsilon,
            // too tight for the ~100-magnitude values these ranges produce
            // (floating-point addition/multiplication in a different order
            // legitimately differs in the last 1-2 ULPs at this scale) --
            // compare via the difference bivector's norm instead, which
            // scales sensibly with the inputs' own magnitude.
            let diff = sum_then_wedge.add(&wedge_then_sum.scale(-1.0));
            prop_assert!(diff.norm() < 1e-8, "diff={:?}", diff);
        }
    }

    proptest! {
        /// Scaling distributes over addition: `(a+b)*s = a*s + b*s`.
        #[test]
        fn scale_distributes_over_add(
            e01 in -10.0f64..10.0, e02 in -10.0f64..10.0, e12 in -10.0f64..10.0,
            f01 in -10.0f64..10.0, f02 in -10.0f64..10.0, f12 in -10.0f64..10.0,
            s in -5.0f64..5.0,
        ) {
            let mut a = Bivector::<3>::zero();
            a.set(0, 1, e01);
            a.set(0, 2, e02);
            a.set(1, 2, e12);
            let mut b = Bivector::<3>::zero();
            b.set(0, 1, f01);
            b.set(0, 2, f02);
            b.set(1, 2, f12);

            let lhs = a.add(&b).scale(s);
            let rhs = a.scale(s).add(&b.scale(s));
            // Same rationale as `wedge_bilinear_in_first_argument`: compare
            // via the difference's norm, not the type's tight-epsilon
            // `PartialEq`, since `s` up to 5x amplifies rounding to well
            // past 1e-14 in absolute terms at these magnitudes.
            let diff = lhs.add(&rhs.scale(-1.0));
            prop_assert!(diff.norm() < 1e-8, "diff={:?}", diff);
        }
    }

    proptest! {
        /// Triangle inequality: `|a+b| <= |a| + |b|` for any two bivectors --
        /// a basic norm-consistency property that would be violated by any
        /// accidental non-Euclidean component weighting in `norm_squared`.
        #[test]
        fn norm_triangle_inequality(
            e01 in -10.0f64..10.0, e02 in -10.0f64..10.0, e12 in -10.0f64..10.0,
            f01 in -10.0f64..10.0, f02 in -10.0f64..10.0, f12 in -10.0f64..10.0,
        ) {
            let mut a = Bivector::<3>::zero();
            a.set(0, 1, e01);
            a.set(0, 2, e02);
            a.set(1, 2, e12);
            let mut b = Bivector::<3>::zero();
            b.set(0, 1, f01);
            b.set(0, 2, f02);
            b.set(1, 2, f12);

            prop_assert!(a.add(&b).norm() <= a.norm() + b.norm() + 1e-9);
        }
    }

    proptest! {
        /// A normalized non-zero bivector always has norm exactly 1.
        #[test]
        fn normalized_has_unit_norm(
            e01 in -10.0f64..10.0, e02 in -10.0f64..10.0, e12 in -10.0f64..10.0,
        ) {
            let mut bv = Bivector::<3>::zero();
            bv.set(0, 1, e01);
            bv.set(0, 2, e02);
            bv.set(1, 2, e12);
            prop_assume!(bv.norm() > 1e-6); // skip the near-zero degenerate case

            let unit = bv.normalized().expect("non-zero bivector should normalize");
            prop_assert!((unit.norm() - 1.0).abs() < 1e-9);
        }
    }
}
