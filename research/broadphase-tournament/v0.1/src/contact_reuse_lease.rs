//! Conservative geometric admission primitive for persistent manifold reuse.
//!
//! This module is a research reference for #6865. It does not cache or mutate
//! production manifolds. It accepts an independently constructed relative
//! transform envelope, feature-stability lower bound, normal-variation upper
//! bound, and updated separation interval. Any unsupported or inconclusive
//! input refreshes the manifold or retires the contact; it never silently
//! extends a lease.
//!
//! The transform adapter must compute the difference between the current and
//! cached relative rotation matrices in one frozen, canonical order. The
//! Frobenius norm upper-bounds the spectral norm, so radius times that value
//! conservatively bounds displacement of every cached anchor. Feature margins
//! and gap intervals must come from an independent geometry witness with
//! outward-rounded bounds. This module checks supplied witnesses; it does not
//! prove their provenance.

use incremental_aabb_tree_reference::{ProxyId, ProxyPair};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeMotionEnvelope<const D: usize> {
    /// Translation change of the canonical relative transform.
    pub relative_translation_delta: [f64; D],
    /// Outward upper bounds on abs(current_relative_rotation - cached_relative_rotation), entry-wise.
    pub relative_rotation_delta_abs_upper: [[f64; D]; D],
    /// Conservative maximum radius of the cached contact geometry from the
    /// relative transform's rotation reference point.
    pub maximum_lever_arm_radius: f64,
    /// Explicit non-negative allowance for transform extraction and arithmetic.
    pub rounding_allowance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionBound {
    pub translation_upper: f64,
    pub rotation_displacement_upper: f64,
    pub total_displacement_upper: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContactFeatureIdentity {
    pub feature_a: u64,
    pub feature_b: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeparationInterval {
    /// Lower bound on updated signed gap; negative values indicate penetration.
    pub lower: f64,
    /// Upper bound on updated signed gap.
    pub upper: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactGeometryLease {
    pub pair: ProxyPair,
    pub generations: [u64; 2],
    pub shape_revisions: [u64; 2],
    /// None means the narrowphase did not provide a stable feature witness.
    pub feature_identity: Option<ContactFeatureIdentity>,
    /// Lower bound on relative movement allowed before a support feature may
    /// change, from an independent geometry-specific feature analysis.
    pub feature_margin_lower_bound: f64,
    /// Upper bound on manifold-normal variation over the proposed step.
    pub normal_variation_upper_bound_rad: f64,
    /// Admitted upper limit for manifold-normal variation.
    pub normal_variation_limit_rad: f64,
    /// Updated separation/gap interval from transformed cached anchors.
    pub updated_gap: SeparationInterval,
    /// Maximum gap still considered an active contact by this profile.
    pub contact_slop: f64,
    /// Maximum penetration allowed for cached separation update.
    pub maximum_penetration: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactLeaseRefreshReason {
    ProxyGenerationChanged,
    ShapeRevisionChanged,
    FeatureIdentityChanged,
    RelativeMotionExceedsFeatureMargin,
    NormalVariationExceedsLimit,
    GapIntervalStraddlesContactBoundary,
    PenetrationBoundExceeded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactLeaseDisposition {
    ReuseCertified,
    ContactEnded,
    RefreshRequired(ContactLeaseRefreshReason),
    UnsupportedGeometry,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactLeaseDecision {
    pub disposition: ContactLeaseDisposition,
    pub motion_bound: Option<MotionBound>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactLeaseError {
    NonCanonicalPair,
    PairIdentityMismatch,
    InvalidRelativeMotion,
    UnsupportedDimension,
    InvalidFeatureMargin,
    InvalidNormalBound,
    InvalidGapInterval,
    InvalidContactSlop,
    InvalidPenetrationBound,
    ArithmeticOverflow,
}

/// Outward-rounded floating-point addition. An overflow is rejected rather
/// than treated as an unbounded but valid lease.
fn add_upper(left: f64, right: f64) -> Result<f64, ContactLeaseError> {
    let value = left + right;
    if !value.is_finite() || value < 0.0 {
        return Err(ContactLeaseError::ArithmeticOverflow);
    }
    let upper = value.next_up();
    if upper.is_finite() {
        Ok(upper)
    } else {
        Err(ContactLeaseError::ArithmeticOverflow)
    }
}

/// Outward-rounded floating-point multiplication of non-negative inputs.
fn multiply_upper(left: f64, right: f64) -> Result<f64, ContactLeaseError> {
    let value = left * right;
    if !value.is_finite() || value < 0.0 {
        return Err(ContactLeaseError::ArithmeticOverflow);
    }
    let upper = value.next_up();
    if upper.is_finite() {
        Ok(upper)
    } else {
        Err(ContactLeaseError::ArithmeticOverflow)
    }
}

/// Conservative Euclidean norm upper bound for a finite vector.
fn norm_upper<const D: usize>(value: &[f64; D]) -> Result<f64, ContactLeaseError> {
    let mut squared_sum = 0.0_f64;
    for component in value {
        if !component.is_finite() {
            return Err(ContactLeaseError::InvalidRelativeMotion);
        }
        let square = multiply_upper(component.abs(), component.abs())?;
        squared_sum = add_upper(squared_sum, square)?;
    }
    let norm = squared_sum.sqrt();
    let upper = norm.next_up();
    if upper.is_finite() {
        Ok(upper)
    } else {
        Err(ContactLeaseError::ArithmeticOverflow)
    }
}

/// Frobenius norm upper bound for the relative rotation-matrix delta.
///
/// For any point vector x, ||DeltaR * x||_2 <= ||DeltaR||_F * ||x||_2.
/// Each product and sum is rounded upward; a non-finite matrix entry fails closed.
fn frobenius_norm_upper<const D: usize>(
    matrix: &[[f64; D]; D],
) -> Result<f64, ContactLeaseError> {
    let mut squared_sum = 0.0_f64;
    for row in matrix {
        for entry in row {
            if !entry.is_finite() || *entry < 0.0 {
                return Err(ContactLeaseError::InvalidRelativeMotion);
            }
            let square = multiply_upper(entry.abs(), entry.abs())?;
            squared_sum = add_upper(squared_sum, square)?;
        }
    }
    let norm = squared_sum.sqrt();
    let upper = norm.next_up();
    if upper.is_finite() {
        Ok(upper)
    } else {
        Err(ContactLeaseError::ArithmeticOverflow)
    }
}

impl<const D: usize> RelativeMotionEnvelope<D> {
    /// Bound maximum point displacement of the cached contact geometry:
    ///
    /// norm(delta_translation) + radius * ||delta_rotation||_F + allowance.
    ///
    /// The Frobenius norm upper-bounds the spectral norm, so this holds for
    /// any supplied matrix delta and does not require extracting principal
    /// rotation angles. The transform adapter must provide an outward-rounded
    /// delta between the current and last-fresh relative rotation matrices.
    pub fn displacement_upper_bound(&self) -> Result<MotionBound, ContactLeaseError> {
        if !(2..=4).contains(&D) {
            return Err(ContactLeaseError::UnsupportedDimension);
        }
        if !self.maximum_lever_arm_radius.is_finite()
            || self.maximum_lever_arm_radius < 0.0
            || !self.rounding_allowance.is_finite()
            || self.rounding_allowance < 0.0
        {
            return Err(ContactLeaseError::InvalidRelativeMotion);
        }

        let translation_upper = norm_upper(&self.relative_translation_delta)?;
        let rotation_matrix_delta_upper = frobenius_norm_upper(&self.relative_rotation_delta_abs_upper)?;
        let rotation_displacement_upper =
            multiply_upper(self.maximum_lever_arm_radius, rotation_matrix_delta_upper)?;
        let combined = add_upper(translation_upper, rotation_displacement_upper)?;
        let total_displacement_upper = add_upper(combined, self.rounding_allowance)?;

        Ok(MotionBound {
            translation_upper,
            rotation_displacement_upper,
            total_displacement_upper,
        })
    }
}

/// Evaluate a prior manifold witness against the current identity/revisions,
/// an independently computed relative-motion envelope, feature margin,
/// normal-variation upper bound, and updated gap interval.
pub fn evaluate_contact_geometry_lease<const D: usize>(
    lease: &ContactGeometryLease,
    current_pair: ProxyPair,
    current_generations: [u64; 2],
    current_shape_revisions: [u64; 2],
    current_feature_identity: Option<ContactFeatureIdentity>,
    motion: &RelativeMotionEnvelope<D>,
) -> Result<ContactLeaseDecision, ContactLeaseError> {
    if lease.pair.0 >= lease.pair.1 || current_pair.0 >= current_pair.1 {
        return Err(ContactLeaseError::NonCanonicalPair);
    }
    if lease.pair != current_pair {
        return Err(ContactLeaseError::PairIdentityMismatch);
    }
    if current_generations != lease.generations {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::ProxyGenerationChanged,
            ),
            motion_bound: None,
        });
    }
    if current_shape_revisions != lease.shape_revisions {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::ShapeRevisionChanged,
            ),
            motion_bound: None,
        });
    }
    let Some(cached_feature_identity) = lease.feature_identity else {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::UnsupportedGeometry,
            motion_bound: None,
        });
    };
    let Some(current_feature_identity) = current_feature_identity else {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::UnsupportedGeometry,
            motion_bound: None,
        });
    };
    if current_feature_identity != cached_feature_identity {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::FeatureIdentityChanged,
            ),
            motion_bound: None,
        });
    }
    if !lease.feature_margin_lower_bound.is_finite()
        || lease.feature_margin_lower_bound < 0.0
    {
        return Err(ContactLeaseError::InvalidFeatureMargin);
    }
    if !lease.normal_variation_upper_bound_rad.is_finite()
        || lease.normal_variation_upper_bound_rad < 0.0
        || !lease.normal_variation_limit_rad.is_finite()
        || lease.normal_variation_limit_rad < 0.0
        || lease.normal_variation_limit_rad > std::f64::consts::PI
    {
        return Err(ContactLeaseError::InvalidNormalBound);
    }
    if !lease.updated_gap.lower.is_finite()
        || !lease.updated_gap.upper.is_finite()
        || lease.updated_gap.lower > lease.updated_gap.upper
    {
        return Err(ContactLeaseError::InvalidGapInterval);
    }
    if !lease.contact_slop.is_finite() || lease.contact_slop < 0.0 {
        return Err(ContactLeaseError::InvalidContactSlop);
    }
    if !lease.maximum_penetration.is_finite() || lease.maximum_penetration < 0.0 {
        return Err(ContactLeaseError::InvalidPenetrationBound);
    }

    let motion_bound = motion.displacement_upper_bound()?;
    if lease.updated_gap.lower > lease.contact_slop {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::ContactEnded,
            motion_bound: Some(motion_bound),
        });
    }
    if lease.updated_gap.upper >= lease.contact_slop {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::GapIntervalStraddlesContactBoundary,
            ),
            motion_bound: Some(motion_bound),
        });
    }
    if lease.updated_gap.lower < -lease.maximum_penetration {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::PenetrationBoundExceeded,
            ),
            motion_bound: Some(motion_bound),
        });
    }
    if lease.feature_margin_lower_bound <= motion_bound.total_displacement_upper {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::RelativeMotionExceedsFeatureMargin,
            ),
            motion_bound: Some(motion_bound),
        });
    }
    if lease.normal_variation_upper_bound_rad > lease.normal_variation_limit_rad {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::NormalVariationExceedsLimit,
            ),
            motion_bound: Some(motion_bound),
        });
    }

    Ok(ContactLeaseDecision {
        disposition: ContactLeaseDisposition::ReuseCertified,
        motion_bound: Some(motion_bound),
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> ProxyPair {
        ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct ids")
    }

    fn motion<const D: usize>(
        translation: [f64; D],
        angle: f64,
        radius: f64,
    ) -> RelativeMotionEnvelope<D> {
        let mut relative_rotation_delta_abs_upper = [[0.0; D]; D];
        if D >= 2 {
            let cosine_delta = angle.cos() - 1.0;
            let sine = angle.sin();
            relative_rotation_delta_abs_upper[0][0] = cosine_delta.abs();
            relative_rotation_delta_abs_upper[0][1] = sine.abs();
            relative_rotation_delta_abs_upper[1][0] = sine.abs();
            relative_rotation_delta_abs_upper[1][1] = cosine_delta.abs();
        }
        RelativeMotionEnvelope {
            relative_translation_delta: translation,
            relative_rotation_delta_abs_upper,
            maximum_lever_arm_radius: radius,
            rounding_allowance: 1.0e-12,
        }
    }

    fn lease() -> ContactGeometryLease {
        ContactGeometryLease {
            pair: pair(),
            generations: [3, 4],
            shape_revisions: [7, 8],
            feature_identity: Some(ContactFeatureIdentity {
                feature_a: 11,
                feature_b: 21,
            }),
            feature_margin_lower_bound: 0.5,
            normal_variation_upper_bound_rad: 0.01,
            normal_variation_limit_rad: 0.05,
            updated_gap: SeparationInterval {
                lower: -0.005,
                upper: 0.005,
            },
            contact_slop: 0.01,
            maximum_penetration: 0.02,
        }
    }

    fn evaluate(
        witness: &ContactGeometryLease,
        motion: &RelativeMotionEnvelope<3>,
    ) -> Result<ContactLeaseDecision, ContactLeaseError> {
        evaluate_contact_geometry_lease(
            witness,
            pair(),
            [3, 4],
            [7, 8],
            witness.feature_identity,
            motion,
        )
    }

    #[test]
    fn zero_relative_motion_reuses_even_if_bodies_move_together() {
        let decision = evaluate(&lease(), &motion([0.0; 3], 0.0, 5.0)).expect("motion");
        assert_eq!(decision.disposition, ContactLeaseDisposition::ReuseCertified);
    }

    #[test]
    fn translation_inside_feature_margin_reuses() {
        let decision = evaluate(&lease(), &motion([0.1, 0.0, 0.0], 0.0, 5.0))
            .expect("motion");
        assert_eq!(decision.disposition, ContactLeaseDisposition::ReuseCertified);
    }

    #[test]
    fn translation_beyond_feature_margin_refreshes() {
        let decision = evaluate(&lease(), &motion([0.6, 0.0, 0.0], 0.0, 5.0))
            .expect("motion");
        assert_eq!(
            decision.disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::RelativeMotionExceedsFeatureMargin
            )
        );
    }

    #[test]
    fn relative_rotation_consumes_feature_margin() {
        let decision = evaluate(&lease(), &motion([0.0; 3], 0.2, 3.0)).expect("motion");
        assert_eq!(
            decision.disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::RelativeMotionExceedsFeatureMargin
            )
        );
    }

    #[test]
    fn generation_change_refreshes() {
        let witness = lease();
        let decision = evaluate_contact_geometry_lease(
            &witness,
            pair(),
            [3, 5],
            [7, 8],
            witness.feature_identity,
            &motion([0.0; 3], 0.0, 1.0),
        )
        .expect("generation change");
        assert_eq!(
            decision.disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::ProxyGenerationChanged
            )
        );
    }

    #[test]
    fn shape_revision_change_refreshes() {
        let witness = lease();
        let decision = evaluate_contact_geometry_lease(
            &witness,
            pair(),
            [3, 4],
            [7, 9],
            witness.feature_identity,
            &motion([0.0; 3], 0.0, 1.0),
        )
        .expect("shape revision");
        assert_eq!(
            decision.disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::ShapeRevisionChanged
            )
        );
    }

    #[test]
    fn featureless_geometry_is_unsupported() {
        let mut witness = lease();
        witness.feature_identity = None;
        assert_eq!(
            evaluate(&witness, &motion([0.0; 3], 0.0, 1.0))
                .expect("unsupported geometry")
                .disposition,
            ContactLeaseDisposition::UnsupportedGeometry
        );
    }

    #[test]
    fn normal_variation_over_limit_refreshes() {
        let mut witness = lease();
        witness.normal_variation_upper_bound_rad = 0.06;
        assert_eq!(
            evaluate(&witness, &motion([0.0; 3], 0.0, 1.0))
                .expect("normal interval")
                .disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::NormalVariationExceedsLimit
            )
        );
    }

    #[test]
    fn ambiguous_gap_interval_refreshes() {
        let mut witness = lease();
        witness.updated_gap.upper = 0.02;
        assert_eq!(
            evaluate(&witness, &motion([0.0; 3], 0.0, 1.0))
                .expect("gap interval")
                .disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::GapIntervalStraddlesContactBoundary
            )
        );
    }

    #[test]
    fn definitely_separated_contact_ends() {
        let mut witness = lease();
        witness.updated_gap = SeparationInterval {
            lower: 0.02,
            upper: 0.03,
        };
        assert_eq!(
            evaluate(&witness, &motion([0.0; 3], 0.0, 1.0))
                .expect("separation interval")
                .disposition,
            ContactLeaseDisposition::ContactEnded
        );
    }

    #[test]
    fn excessive_penetration_refreshes() {
        let mut witness = lease();
        witness.updated_gap.lower = -0.03;
        assert_eq!(
            evaluate(&witness, &motion([0.0; 3], 0.0, 1.0))
                .expect("penetration interval")
                .disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::PenetrationBoundExceeded
            )
        );
    }

    #[test]
    fn feature_margin_boundary_fails_closed() {
        let mut witness = lease();
        witness.feature_margin_lower_bound = 0.1;
        assert_eq!(
            evaluate(&witness, &motion([0.1, 0.0, 0.0], 0.0, 1.0))
                .expect("feature boundary")
                .disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::RelativeMotionExceedsFeatureMargin
            )
        );
    }

    #[test]
    fn nonfinite_motion_fails_closed() {
        let witness = lease();
        assert_eq!(
            evaluate(
                &witness,
                &motion([f64::NAN, 0.0, 0.0], 0.0, 1.0)
            ),
            Err(ContactLeaseError::InvalidRelativeMotion)
        );
    }

    #[test]
    fn invalid_gap_interval_fails_closed() {
        let mut witness = lease();
        witness.updated_gap = SeparationInterval {
            lower: 0.2,
            upper: 0.1,
        };
        assert_eq!(
            evaluate(&witness, &motion([0.0; 3], 0.0, 1.0)),
            Err(ContactLeaseError::InvalidGapInterval)
        );
    }

    #[test]
    fn nonfinite_rotation_matrix_delta_fails_closed() {
        let witness = lease();
        let invalid_motion = RelativeMotionEnvelope {
            relative_translation_delta: [0.0; 3],
            relative_rotation_delta_abs_upper: [
                [f64::NAN, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
            maximum_lever_arm_radius: 1.0,
            rounding_allowance: 1.0e-12,
        };
        assert_eq!(
            evaluate(&witness, &invalid_motion),
            Err(ContactLeaseError::InvalidRelativeMotion)
        );
    }

    #[test]
    fn canonical_pair_mismatch_is_rejected() {
        let witness = lease();
        let reversed = ProxyPair(ProxyId(2), ProxyId(1));
        assert_eq!(
            evaluate_contact_geometry_lease(
                &witness,
                reversed,
                [3, 4],
                [7, 8],
                witness.feature_identity,
                &motion([0.0; 3], 0.0, 1.0),
            ),
            Err(ContactLeaseError::NonCanonicalPair)
        );
    }

    #[test]
    fn zero_lever_arm_has_no_rotation_displacement() {
        let motion = motion([0.0; 3], std::f64::consts::PI, 0.0);
        let bound = motion.displacement_upper_bound().expect("bound");
        assert!(bound.rotation_displacement_upper < 1.0e-300);
        assert!(bound.total_displacement_upper < 1.0e-9);
    }

    #[test]
    fn gap_interval_touching_contact_slop_refreshes() {
        let mut witness = lease();
        witness.updated_gap = SeparationInterval {
            lower: 0.01,
            upper: 0.01,
        };
        assert_eq!(
            evaluate(&witness, &motion([0.0; 3], 0.0, 1.0))
                .expect("boundary contact"),
            ContactLeaseDecision {
                disposition: ContactLeaseDisposition::RefreshRequired(
                    ContactLeaseRefreshReason::GapIntervalStraddlesContactBoundary
                ),
                motion_bound: Some(
                    motion([0.0; 3], 0.0, 1.0)
                        .displacement_upper_bound()
                        .expect("bound"),
                ),
            }
        );
    }


    #[test]
    fn unsupported_dimensions_fail_closed() {
        let bad_motion = motion([0.0; 1], 0.0, 1.0);
        assert_eq!(
            bad_motion.displacement_upper_bound(),
            Err(ContactLeaseError::UnsupportedDimension)
        );
    }



    #[test]
    fn changed_support_feature_forces_fresh_narrowphase() {
        let witness = lease();
        let mut changed_feature = witness.feature_identity.expect("cached feature");
        changed_feature.feature_b += 1;
        let decision = evaluate_contact_geometry_lease(
            &witness,
            pair(),
            [3, 4],
            [7, 8],
            Some(changed_feature),
            &motion([0.0; 3], 0.0, 1.0),
        )
        .expect("feature transition");
        assert_eq!(
            decision.disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::FeatureIdentityChanged
            )
        );
    }

    #[test]
    fn missing_current_support_feature_is_unsupported() {
        let witness = lease();
        let decision = evaluate_contact_geometry_lease(
            &witness,
            pair(),
            [3, 4],
            [7, 8],
            None,
            &motion([0.0; 3], 0.0, 1.0),
        )
        .expect("missing feature witness");
        assert_eq!(decision.disposition, ContactLeaseDisposition::UnsupportedGeometry);
    }



    #[test]
    fn lease_predicate_supports_two_and_four_dimensions() {
        let witness = lease();
        let feature = witness.feature_identity;
        let two_dimensional = evaluate_contact_geometry_lease(
            &witness,
            pair(),
            [3, 4],
            [7, 8],
            feature,
            &motion([0.0; 2], 0.0, 1.0),
        )
        .expect("2D relative transform");
        let four_dimensional = evaluate_contact_geometry_lease(
            &witness,
            pair(),
            [3, 4],
            [7, 8],
            feature,
            &motion([0.0; 4], 0.0, 1.0),
        )
        .expect("4D relative transform");
        assert_eq!(two_dimensional.disposition, ContactLeaseDisposition::ReuseCertified);
        assert_eq!(four_dimensional.disposition, ContactLeaseDisposition::ReuseCertified);
    }

    #[test]
    fn cumulative_relative_motion_is_measured_from_last_fresh_manifold() {
        let witness = lease();
        let feature = witness.feature_identity;
        for cumulative_translation in [0.1, 0.2, 0.3, 0.4] {
            let decision = evaluate_contact_geometry_lease(
                &witness,
                pair(),
                [3, 4],
                [7, 8],
                feature,
                &motion([cumulative_translation, 0.0, 0.0], 0.0, 1.0),
            )
            .expect("cumulative relative motion");
            assert_eq!(decision.disposition, ContactLeaseDisposition::ReuseCertified);
        }
        let at_margin = evaluate_contact_geometry_lease(
            &witness,
            pair(),
            [3, 4],
            [7, 8],
            feature,
            &motion([0.5, 0.0, 0.0], 0.0, 1.0),
        )
        .expect("cached transform reference is unchanged");
        assert_eq!(
            at_margin.disposition,
            ContactLeaseDisposition::RefreshRequired(
                ContactLeaseRefreshReason::RelativeMotionExceedsFeatureMargin
            )
        );
    }



    #[test]
    fn four_dimensional_two_plane_rotation_uses_all_matrix_delta_entries() {
        let mut delta = [[0.0; 4]; 4];
        let angle_a: f64 = 0.2;
        let angle_b: f64 = 0.3;
        let (sin_a, cos_a) = angle_a.sin_cos();
        let (sin_b, cos_b) = angle_b.sin_cos();
        delta[0][0] = (cos_a - 1.0).abs();
        delta[0][1] = sin_a.abs();
        delta[1][0] = sin_a.abs();
        delta[1][1] = (cos_a - 1.0).abs();
        delta[2][2] = (cos_b - 1.0).abs();
        delta[2][3] = sin_b.abs();
        delta[3][2] = sin_b.abs();
        delta[3][3] = (cos_b - 1.0).abs();

        let bound = RelativeMotionEnvelope {
            relative_translation_delta: [0.0; 4],
            relative_rotation_delta_abs_upper: delta,
            maximum_lever_arm_radius: 1.0,
            rounding_allowance: 1.0e-12,
        }
        .displacement_upper_bound()
        .expect("4D two-plane relative rotation");
        assert!(bound.rotation_displacement_upper > 0.0);
        assert!(bound.rotation_displacement_upper < 1.0);
    }

    #[test]
    fn negative_rotation_delta_upper_entry_fails_closed() {
        let invalid = RelativeMotionEnvelope {
            relative_translation_delta: [0.0; 3],
            relative_rotation_delta_abs_upper: [
                [-0.1, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
            maximum_lever_arm_radius: 1.0,
            rounding_allowance: 0.0,
        };
        assert_eq!(
            invalid.displacement_upper_bound(),
            Err(ContactLeaseError::InvalidRelativeMotion)
        );
    }

}
