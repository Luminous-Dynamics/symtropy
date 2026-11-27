//! Conservative geometric admission primitive for persistent manifold reuse.
//!
//! This module is a research reference for #6865. It does not cache or mutate
//! production manifolds. It accepts an independently constructed relative
//! transform envelope, feature-stability lower bound, normal-variation upper
//! bound, and updated separation interval. Any unsupported or inconclusive
//! input refreshes the manifold or retires the contact; it never silently
//! extends a lease.
//!
//! The transform adapter must compute the relative transform in one frozen,
//! canonical order. The largest point-rotation angle is the maximum principal
//! rotation angle (not an arbitrary matrix norm). Feature margins and gap
//! intervals must come from an independent geometry witness with outward-
//! rounded bounds. This module checks the supplied witness; it does not prove
//! the geometry provider's construction.

use incremental_aabb_tree_reference::{ProxyId, ProxyPair};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeMotionEnvelope<const D: usize> {
    /// Translation change of the canonical relative transform.
    pub relative_translation_delta: [f64; D],
    /// Maximum point-rotation angle of the relative rotation, radians in [0, pi].
    pub max_point_rotation_angle_rad: f64,
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

/// Conservative norm upper bound for a finite translation vector.
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

impl<const D: usize> RelativeMotionEnvelope<D> {
    /// Bound maximum movement of any point in the cached contact geometry:
    ///
    /// 
    /// Bound maximum point displacement of the cached contact geometry:
    /// |relative translation| + radius * maximum rotation angle + allowance.
    ///
    /// The rotation term is conservative because sin(x) <= x for x >= 0.
    /// The adapter must supply the largest point-rotation angle of the
    /// canonical relative rotation, in [0, pi].
    pub fn displacement_upper_bound(&self) -> Result<MotionBound, ContactLeaseError> {
        if !self.max_point_rotation_angle_rad.is_finite()
            || !(0.0..=std::f64::consts::PI).contains(&self.max_point_rotation_angle_rad)
            || !self.maximum_lever_arm_radius.is_finite()
            || self.maximum_lever_arm_radius < 0.0
            || !self.rounding_allowance.is_finite()
            || self.rounding_allowance < 0.0
        {
            return Err(ContactLeaseError::InvalidRelativeMotion);
        }

        let translation_upper = norm_upper(&self.relative_translation_delta)?;
        let rotation_displacement_upper = multiply_upper(
            self.maximum_lever_arm_radius,
            self.max_point_rotation_angle_rad,
        )?;
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
    if lease.feature_identity.is_none() {
        return Ok(ContactLeaseDecision {
            disposition: ContactLeaseDisposition::UnsupportedGeometry,
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
    if lease.updated_gap.upper > lease.contact_slop {
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
