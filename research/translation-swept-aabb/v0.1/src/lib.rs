// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Conservative swept AABBs for the represented fixed-orientation translation
//! path used by Symtropy's current semi-implicit Euler / CCD step.
//!
//! This crate does not integrate a body, infer forces, or prove rotation bounds.
//! It consumes an observed pre-state and post-state snapshot. Unsupported
//! rotational motion and unknown numeric profiles must use a safe fallback.

use std::error::Error;
use std::fmt;

const ROTATION_UPDATE_NORM_SQUARED_THRESHOLD: f64 = 1.0e-24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SweepError {
    ZeroDimension,
    NonFiniteTimeStep,
    NegativeTimeStep,
    NonFiniteState,
    InvalidAngularSpeed,
    NativeVelocityMismatch,
    EmptyProvenance,
}

impl fmt::Display for SweepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for SweepError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuantizationProfile {
    NativeBinary64,
    Q16_16,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyIdentity {
    body_handle: u64,
    body_generation: u32,
    collider_id: u32,
    subshape_path: Vec<u32>,
}

impl BodyIdentity {
    #[must_use]
    pub fn new(
        body_handle: u64,
        body_generation: u32,
        collider_id: u32,
        subshape_path: Vec<u32>,
    ) -> Self {
        Self {
            body_handle,
            body_generation,
            collider_id,
            subshape_path,
        }
    }

    #[must_use]
    pub fn body_handle(&self) -> u64 {
        self.body_handle
    }

    #[must_use]
    pub fn body_generation(&self) -> u32 {
        self.body_generation
    }

    #[must_use]
    pub fn collider_id(&self) -> u32 {
        self.collider_id
    }

    #[must_use]
    pub fn subshape_path(&self) -> &[u32] {
        &self.subshape_path
    }
}

/// All labels are provenance metadata supplied by the caller, not authenticated
/// identities. The state snapshot remains the numerical source of the sweep.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepContext {
    world_revision: u64,
    step_index: u64,
    dt_bits: u64,
    integrator_id: String,
    integrator_version: String,
    quantization: QuantizationProfile,
    shape_bound_provider: String,
    shape_bound_version: String,
}

impl SweepContext {
    pub fn new(
        world_revision: u64,
        step_index: u64,
        dt: f64,
        integrator_id: impl Into<String>,
        integrator_version: impl Into<String>,
        quantization: QuantizationProfile,
        shape_bound_provider: impl Into<String>,
        shape_bound_version: impl Into<String>,
    ) -> Result<Self, SweepError> {
        if !dt.is_finite() {
            return Err(SweepError::NonFiniteTimeStep);
        }
        if dt < 0.0 {
            return Err(SweepError::NegativeTimeStep);
        }
        let labels = [
            integrator_id.into(),
            integrator_version.into(),
            shape_bound_provider.into(),
            shape_bound_version.into(),
        ];
        if labels.iter().any(|label| label.trim().is_empty()) {
            return Err(SweepError::EmptyProvenance);
        }
        Ok(Self {
            world_revision,
            step_index,
            dt_bits: dt.to_bits(),
            integrator_id: labels[0].clone(),
            integrator_version: labels[1].clone(),
            quantization,
            shape_bound_provider: labels[2].clone(),
            shape_bound_version: labels[3].clone(),
        })
    }

    #[must_use]
    pub fn world_revision(&self) -> u64 {
        self.world_revision
    }

    #[must_use]
    pub fn step_index(&self) -> u64 {
        self.step_index
    }

    #[must_use]
    pub fn dt(&self) -> f64 {
        f64::from_bits(self.dt_bits)
    }

    #[must_use]
    pub fn quantization(&self) -> QuantizationProfile {
        self.quantization
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb<const D: usize> {
    min: [f64; D],
    max: [f64; D],
}

impl<const D: usize> Aabb<D> {
    pub fn new(min: [f64; D], max: [f64; D]) -> Result<Self, SweepError> {
        if D == 0 {
            return Err(SweepError::ZeroDimension);
        }
        for (lower, upper) in min.iter().zip(max.iter()) {
            if !lower.is_finite() || !upper.is_finite() {
                return Err(SweepError::NonFiniteState);
            }
            if lower > upper {
                return Err(SweepError::NonFiniteState);
            }
        }
        Ok(Self { min, max })
    }

    #[must_use]
    pub fn min(&self) -> &[f64; D] {
        &self.min
    }

    #[must_use]
    pub fn max(&self) -> &[f64; D] {
        &self.max
    }

    #[must_use]
    pub fn contains(&self, other: &Self) -> bool {
        (0..D).all(|axis| self.min[axis] <= other.min[axis] && self.max[axis] >= other.max[axis])
    }
}

/// A state snapshot taken immediately after Symtropy's linear integration and
/// deterministic-net quantization, but before later position corrections.
#[derive(Clone, Debug, PartialEq)]
pub struct TranslationSweepInput<const D: usize> {
    identity: BodyIdentity,
    context: SweepContext,
    start_position: [f64; D],
    post_position: [f64; D],
    integrator_post_linear_velocity: [f64; D],
    post_linear_velocity: [f64; D],
    post_angular_velocity_norm_squared: f64,
    start_world_aabb: Aabb<D>,
}

impl<const D: usize> TranslationSweepInput<D> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: BodyIdentity,
        context: SweepContext,
        start_position: [f64; D],
        post_position: [f64; D],
        integrator_post_linear_velocity: [f64; D],
        post_linear_velocity: [f64; D],
        post_angular_velocity_norm_squared: f64,
        start_world_aabb: Aabb<D>,
    ) -> Result<Self, SweepError> {
        if !start_position.iter().all(|value| value.is_finite())
            || !post_position.iter().all(|value| value.is_finite())
            || !integrator_post_linear_velocity
                .iter()
                .all(|value| value.is_finite())
            || !post_linear_velocity.iter().all(|value| value.is_finite())
        {
            return Err(SweepError::NonFiniteState);
        }
        if !post_angular_velocity_norm_squared.is_finite()
            || post_angular_velocity_norm_squared < 0.0
        {
            return Err(SweepError::InvalidAngularSpeed);
        }
        if context.quantization == QuantizationProfile::NativeBinary64
            && !same_array(
                &integrator_post_linear_velocity,
                &post_linear_velocity,
            )
        {
            return Err(SweepError::NativeVelocityMismatch);
        }
        Ok(Self {
            identity,
            context,
            start_position,
            post_position,
            integrator_post_linear_velocity,
            post_linear_velocity,
            post_angular_velocity_norm_squared,
            start_world_aabb,
        })
    }

    #[must_use]
    pub fn identity(&self) -> &BodyIdentity {
        &self.identity
    }

    #[must_use]
    pub fn context(&self) -> &SweepContext {
        &self.context
    }

    #[must_use]
    pub fn start_position(&self) -> &[f64; D] {
        &self.start_position
    }

    #[must_use]
    pub fn post_position(&self) -> &[f64; D] {
        &self.post_position
    }

    #[must_use]
    pub fn integrator_post_linear_velocity(&self) -> &[f64; D] {
        &self.integrator_post_linear_velocity
    }

    #[must_use]
    pub fn post_linear_velocity(&self) -> &[f64; D] {
        &self.post_linear_velocity
    }

    #[must_use]
    pub fn post_angular_velocity_norm_squared(&self) -> f64 {
        self.post_angular_velocity_norm_squared
    }

    #[must_use]
    pub fn start_world_aabb(&self) -> &Aabb<D> {
        &self.start_world_aabb
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedReason {
    RotationUpdated,
    UnknownQuantizationProfile,
    VelocityDisplacementOverflow,
    ObservedEndpointDeltaOverflow,
    OutwardBoundOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidReason {
    MalformedInput(SweepError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SweptAabbCertificate<const D: usize> {
    identity: BodyIdentity,
    context: SweepContext,
    source: TranslationSweepInput<D>,
    bounds: Aabb<D>,
}

impl<const D: usize> SweptAabbCertificate<D> {
    #[must_use]
    pub fn identity(&self) -> &BodyIdentity {
        &self.identity
    }

    #[must_use]
    pub fn context(&self) -> &SweepContext {
        &self.context
    }

    #[must_use]
    pub fn bounds(&self) -> &Aabb<D> {
        &self.bounds
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SweepDecision<const D: usize> {
    Certified(SweptAabbCertificate<D>),
    Unsupported(UnsupportedReason),
    Invalid(InvalidReason),
}

/// Construct a conservative union of:
/// 1. the start AABB translated along post_velocity * dt, which is the
///    straight-line CCD path documented by the current physics step; and
/// 2. the observed post-integration position endpoint, which can differ due to
///    deterministic position quantization.
///
/// Rotation is supported only when the integrator's actual rotor-update
/// threshold proves no rotation update occurred. Other cases must fall back.
#[must_use]
pub fn produce_swept_aabb<const D: usize>(input: &TranslationSweepInput<D>) -> SweepDecision<D> {
    if input.post_angular_velocity_norm_squared > ROTATION_UPDATE_NORM_SQUARED_THRESHOLD {
        return SweepDecision::Unsupported(UnsupportedReason::RotationUpdated);
    }
    if input.context.quantization == QuantizationProfile::Unknown {
        return SweepDecision::Unsupported(UnsupportedReason::UnknownQuantizationProfile);
    }

    let dt = input.context.dt();
    let mut min = input.start_world_aabb.min;
    let mut max = input.start_world_aabb.max;

    for axis in 0..D {
        // This multiplication is the same represented operation used by the
        // integrator when it computes post_velocity * dt for displacement.
        let integrated_delta = input.integrator_post_linear_velocity[axis] * dt;
        let ccd_delta = input.post_linear_velocity[axis] * dt;
        if !integrated_delta.is_finite() || !ccd_delta.is_finite() {
            return SweepDecision::Unsupported(UnsupportedReason::VelocityDisplacementOverflow);
        }

        // Include the actual stored post-position as a second endpoint. This
        // covers Q16.16 position rounding without assuming it equals p0 + v*dt.
        let observed_delta = input.post_position[axis] - input.start_position[axis];
        if !observed_delta.is_finite() {
            return SweepDecision::Unsupported(UnsupportedReason::ObservedEndpointDeltaOverflow);
        }
        // The subtraction itself rounds. Enclose its exact real difference
        // before composing it with the starting world-space shape bound.
        let observed_delta_lower = if observed_delta == 0.0 {
            observed_delta
        } else {
            next_down(observed_delta)
        };
        let observed_delta_upper = if observed_delta == 0.0 {
            observed_delta
        } else {
            next_up(observed_delta)
        };
        if !observed_delta_lower.is_finite() || !observed_delta_upper.is_finite() {
            return SweepDecision::Unsupported(UnsupportedReason::ObservedEndpointDeltaOverflow);
        }

        let lower_delta = 0.0_f64
            .min(integrated_delta)
            .min(ccd_delta)
            .min(observed_delta_lower);
        let upper_delta = 0.0_f64
            .max(integrated_delta)
            .max(ccd_delta)
            .max(observed_delta_upper);

        let lower = outward_lower_sum(input.start_world_aabb.min[axis], lower_delta);
        let upper = outward_upper_sum(input.start_world_aabb.max[axis], upper_delta);
        let (Some(lower), Some(upper)) = (lower, upper) else {
            return SweepDecision::Unsupported(UnsupportedReason::OutwardBoundOverflow);
        };
        if lower > upper {
            return SweepDecision::Unsupported(UnsupportedReason::OutwardBoundOverflow);
        }
        min[axis] = lower;
        max[axis] = upper;
    }

    let bounds = match Aabb::new(min, max) {
        Ok(bounds) => bounds,
        Err(error) => return SweepDecision::Invalid(InvalidReason::MalformedInput(error)),
    };

    SweepDecision::Certified(SweptAabbCertificate {
        identity: input.identity.clone(),
        context: input.context.clone(),
        source: input.clone(),
        bounds,
    })
}

/// Recompute the envelope from the supplied current state and compare every
/// identity, context, endpoint and resulting bound bit-for-bit.
pub fn verify_swept_aabb<const D: usize>(
    certificate: &SweptAabbCertificate<D>,
    current: &TranslationSweepInput<D>,
) -> Result<(), SweepVerificationError> {
    if certificate.identity != current.identity {
        return Err(SweepVerificationError::IdentityMismatch);
    }
    if certificate.context != current.context {
        return Err(SweepVerificationError::ContextMismatch);
    }
    if !same_input(&certificate.source, current) {
        return Err(SweepVerificationError::StateSnapshotMismatch);
    }

    match produce_swept_aabb(current) {
        SweepDecision::Certified(replayed) if same_aabb(&certificate.bounds, &replayed.bounds) => {
            Ok(())
        }
        SweepDecision::Certified(_) => Err(SweepVerificationError::EnvelopeMismatch),
        SweepDecision::Unsupported(_) => Err(SweepVerificationError::NoLongerSupported),
        SweepDecision::Invalid(_) => Err(SweepVerificationError::InvalidCurrentInput),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SweepVerificationError {
    IdentityMismatch,
    ContextMismatch,
    StateSnapshotMismatch,
    EnvelopeMismatch,
    NoLongerSupported,
    InvalidCurrentInput,
}

impl fmt::Display for SweepVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for SweepVerificationError {}

fn same_input<const D: usize>(
    left: &TranslationSweepInput<D>,
    right: &TranslationSweepInput<D>,
) -> bool {
    left.identity == right.identity
        && left.context == right.context
        && same_array(&left.start_position, &right.start_position)
        && same_array(&left.post_position, &right.post_position)
        && same_array(
            &left.integrator_post_linear_velocity,
            &right.integrator_post_linear_velocity,
        )
        && same_array(&left.post_linear_velocity, &right.post_linear_velocity)
        && left.post_angular_velocity_norm_squared.to_bits()
            == right.post_angular_velocity_norm_squared.to_bits()
        && same_aabb(&left.start_world_aabb, &right.start_world_aabb)
}

fn same_array<const D: usize>(left: &[f64; D], right: &[f64; D]) -> bool {
    left.iter()
        .zip(right.iter())
        .all(|(a, b)| a.to_bits() == b.to_bits())
}

fn same_aabb<const D: usize>(left: &Aabb<D>, right: &Aabb<D>) -> bool {
    same_array(&left.min, &right.min) && same_array(&left.max, &right.max)
}

fn outward_lower_sum(base: f64, delta: f64) -> Option<f64> {
    if delta == 0.0 {
        return Some(base);
    }
    let sum = base + delta;
    if !sum.is_finite() {
        return None;
    }
    let lower = next_down(sum);
    lower.is_finite().then_some(lower)
}

fn outward_upper_sum(base: f64, delta: f64) -> Option<f64> {
    if delta == 0.0 {
        return Some(base);
    }
    let sum = base + delta;
    if !sum.is_finite() {
        return None;
    }
    let upper = next_up(sum);
    upper.is_finite().then_some(upper)
}

fn next_down(value: f64) -> f64 {
    if value == 0.0 {
        return f64::from_bits((1_u64 << 63) | 1);
    }
    if value.is_sign_positive() {
        f64::from_bits(value.to_bits() - 1)
    } else {
        f64::from_bits(value.to_bits() + 1)
    }
}

fn next_up(value: f64) -> f64 {
    if value == 0.0 {
        return f64::from_bits(1);
    }
    if value.is_sign_positive() {
        f64::from_bits(value.to_bits() + 1)
    } else {
        f64::from_bits(value.to_bits() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(dt: f64, quantization: QuantizationProfile) -> SweepContext {
        SweepContext::new(
            9,
            27,
            dt,
            "semi-implicit-euler",
            "symtropy-integrator-v1",
            quantization,
            "world-aabb",
            "shape-bounds-v1",
        )
        .unwrap()
    }

    fn identity() -> BodyIdentity {
        BodyIdentity::new(12, 3, 2, vec![1, 4])
    }

    fn input(
        start_position: [f64; 2],
        post_position: [f64; 2],
        post_velocity: [f64; 2],
        angular_norm_sq: f64,
    ) -> TranslationSweepInput<2> {
        TranslationSweepInput::new(
            identity(),
            context(1.0, QuantizationProfile::NativeBinary64),
            start_position,
            post_position,
            post_velocity,
            post_velocity,
            angular_norm_sq,
            Aabb::new(
                [start_position[0] - 1.0, start_position[1] - 2.0],
                [start_position[0] + 1.0, start_position[1] + 2.0],
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn certified(input: &TranslationSweepInput<2>) -> SweptAabbCertificate<2> {
        match produce_swept_aabb(input) {
            SweepDecision::Certified(certificate) => certificate,
            other => panic!("expected certified sweep, got {other:?}"),
        }
    }

    #[test]
    fn positive_translation_expands_upper_faces() {
        let source = input([0.0, 0.0], [4.0, 0.0], [4.0, 0.0], 0.0);
        let proof = certified(&source);
        assert!(proof.bounds.min()[0] <= -1.0);
        assert!(proof.bounds.max()[0] >= 5.0);
        assert!(proof.bounds.min()[1] <= -2.0);
        assert!(proof.bounds.max()[1] >= 2.0);
        verify_swept_aabb(&proof, &source).unwrap();
    }

    #[test]
    fn negative_translation_expands_lower_faces() {
        let source = input([0.0, 0.0], [-3.0, 0.0], [-3.0, 0.0], 0.0);
        let proof = certified(&source);
        assert!(proof.bounds.min()[0] <= -4.0);
        assert!(proof.bounds.max()[0] >= 1.0);
    }

    #[test]
    fn multiaxis_translation_is_bounded_on_each_axis() {
        let source = input([2.0, -2.0], [5.0, -6.0], [3.0, -4.0], 0.0);
        let proof = certified(&source);
        assert!(proof.bounds.min()[0] <= 1.0);
        assert!(proof.bounds.max()[0] >= 6.0);
        assert!(proof.bounds.min()[1] <= -8.0);
        assert!(proof.bounds.max()[1] >= 0.0);
    }

    #[test]
    fn observed_post_position_is_included_even_when_quantized_off_path() {
        let source = input([0.0, 0.0], [1.00001, 0.0], [1.0, 0.0], 0.0);
        let proof = certified(&source);
        assert!(proof.bounds.max()[0] >= 2.00001);
    }

    #[test]
    fn stationary_motion_preserves_exact_start_bounds() {
        let source = input([0.0, 0.0], [0.0, 0.0], [0.0, 0.0], 0.0);
        let proof = certified(&source);
        assert_eq!(proof.bounds.min(), &[-1.0, -2.0]);
        assert_eq!(proof.bounds.max(), &[1.0, 2.0]);
    }

    #[test]
    fn rotation_above_integrator_threshold_forces_fallback() {
        let source = input(
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
            ROTATION_UPDATE_NORM_SQUARED_THRESHOLD * 2.0,
        );
        assert_eq!(
            produce_swept_aabb(&source),
            SweepDecision::Unsupported(UnsupportedReason::RotationUpdated)
        );
    }

    #[test]
    fn rotation_at_or_below_integrator_threshold_is_fixed_orientation() {
        let source = input(
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
            ROTATION_UPDATE_NORM_SQUARED_THRESHOLD,
        );
        assert!(matches!(
            produce_swept_aabb(&source),
            SweepDecision::Certified(_)
        ));
    }

    #[test]
    fn unknown_quantization_profile_forces_fallback() {
        let source = TranslationSweepInput::new(
            identity(),
            context(1.0, QuantizationProfile::Unknown),
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
            0.0,
            Aabb::new([-1.0, -1.0], [1.0, 1.0]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            produce_swept_aabb(&source),
            SweepDecision::Unsupported(UnsupportedReason::UnknownQuantizationProfile)
        );
    }

    #[test]
    fn q16_16_profile_is_bound_into_the_receipt() {
        let source = TranslationSweepInput::new(
            identity(),
            context(1.0 / 60.0, QuantizationProfile::Q16_16),
            [0.0, 0.0],
            [0.125, 0.0],
            [7.5001, 0.0],
            [7.5, 0.0],
            0.0,
            Aabb::new([-1.0, -1.0], [1.0, 1.0]).unwrap(),
        )
        .unwrap();
        let proof = match produce_swept_aabb(&source) {
            SweepDecision::Certified(proof) => proof,
            other => panic!("expected certified sweep, got {other:?}"),
        };
        assert_eq!(proof.context.quantization(), QuantizationProfile::Q16_16);
        assert!(proof.bounds.max()[0] > 1.125001);
        verify_swept_aabb(&proof, &source).unwrap();
    }

    #[test]
    fn malformed_time_and_state_are_rejected() {
        assert_eq!(
            SweepContext::new(
                1,
                1,
                f64::NAN,
                "i",
                "1",
                QuantizationProfile::NativeBinary64,
                "b",
                "1"
            ),
            Err(SweepError::NonFiniteTimeStep)
        );
        assert_eq!(
            SweepContext::new(
                1,
                1,
                -1.0,
                "i",
                "1",
                QuantizationProfile::NativeBinary64,
                "b",
                "1"
            ),
            Err(SweepError::NegativeTimeStep)
        );
        assert_eq!(Aabb::<0>::new([], []), Err(SweepError::ZeroDimension));
        assert_eq!(
            Aabb::new([f64::NAN, 0.0], [1.0, 1.0]),
            Err(SweepError::NonFiniteState)
        );
        assert_eq!(
            TranslationSweepInput::new(
                identity(),
                context(1.0, QuantizationProfile::NativeBinary64),
                [f64::INFINITY, 0.0],
                [1.0, 0.0],
                [1.0, 0.0],
                [1.0, 0.0],
                0.0,
                Aabb::new([-1.0, -1.0], [1.0, 1.0]).unwrap(),
            ),
            Err(SweepError::NonFiniteState)
        );
    }

    #[test]
    fn arithmetic_overflow_forces_fallback() {
        let source = TranslationSweepInput::new(
            identity(),
            context(2.0, QuantizationProfile::NativeBinary64),
            [0.0, 0.0],
            [0.0, 0.0],
            [f64::MAX, 0.0],
            [f64::MAX, 0.0],
            0.0,
            Aabb::new([-1.0, -1.0], [1.0, 1.0]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            produce_swept_aabb(&source),
            SweepDecision::Unsupported(UnsupportedReason::VelocityDisplacementOverflow)
        );
    }

    #[test]
    fn outward_rounding_overflow_forces_fallback() {
        let source = TranslationSweepInput::new(
            identity(),
            context(1.0, QuantizationProfile::NativeBinary64),
            [f64::MAX, 0.0],
            [f64::MAX, 0.0],
            [1.0, 0.0],
            [1.0, 0.0],
            0.0,
            Aabb::new([f64::MAX, 0.0], [f64::MAX, 1.0]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            produce_swept_aabb(&source),
            SweepDecision::Unsupported(UnsupportedReason::OutwardBoundOverflow)
        );
    }

    #[test]
    fn changed_state_or_identity_invalidates_replay() {
        let source = input([0.0, 0.0], [2.0, 0.0], [2.0, 0.0], 0.0);
        let proof = certified(&source);
        let changed = input([0.0, 0.0], [2.5, 0.0], [2.0, 0.0], 0.0);
        assert_eq!(
            verify_swept_aabb(&proof, &changed),
            Err(SweepVerificationError::StateSnapshotMismatch)
        );
    }

    #[test]
    fn deterministic_random_segments_are_contained() {
        let mut state = 0x8f3a_2c19_d7e4_b501_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state as i64 as f64) / (i64::MAX as f64) * 100.0
        };

        for _case in 0..256 {
            let start_position = [next(), next()];
            let velocity = [next() * 0.01, next() * 0.01];
            let dt = 0.25;
            let expected_displacement = [velocity[0] * dt, velocity[1] * dt];
            let observed_delta = [
                expected_displacement[0] + 1.0e-7,
                expected_displacement[1] - 1.0e-7,
            ];
            let post_position = [
                start_position[0] + observed_delta[0],
                start_position[1] + observed_delta[1],
            ];
            let source = TranslationSweepInput::new(
                identity(),
                context(dt, QuantizationProfile::NativeBinary64),
                start_position,
                post_position,
                velocity,
                velocity,
                0.0,
                Aabb::new(
                    [start_position[0] - 0.5, start_position[1] - 0.25],
                    [start_position[0] + 0.75, start_position[1] + 0.5],
                )
                .unwrap(),
            )
            .unwrap();
            let proof = certified(&source);

            // Independent segment sampling over both the linear CCD path and
            // the observed endpoint path. Sampling is a regression oracle,
            // not a mathematical proof of the path enclosure.
            for step in 0..=100 {
                let alpha = step as f64 / 100.0;
                for delta in [expected_displacement, observed_delta] {
                    let offset = [delta[0] * alpha, delta[1] * alpha];
                    let sample = Aabb::new(
                        [
                            start_position[0] - 0.5 + offset[0],
                            start_position[1] - 0.25 + offset[1],
                        ],
                        [
                            start_position[0] + 0.75 + offset[0],
                            start_position[1] + 0.5 + offset[1],
                        ],
                    )
                    .unwrap();
                    assert!(
                        proof.bounds.contains(&sample),
                        "sample escaped envelope: alpha={alpha}, sample={sample:?}, envelope={:?}",
                        proof.bounds
                    );
                }
            }
        }
    }

    #[test]
    fn three_dimensional_fixed_orientation_sweep_is_supported() {
        let source = TranslationSweepInput::new(
            BodyIdentity::new(1, 0, 0, vec![]),
            context(0.5, QuantizationProfile::NativeBinary64),
            [0.0, 0.0, 0.0],
            [1.0, -2.0, 3.0],
            [2.0, -4.0, 6.0],
            [2.0, -4.0, 6.0],
            0.0,
            Aabb::new([-1.0; 3], [1.0; 3]).unwrap(),
        )
        .unwrap();
        let proof = match produce_swept_aabb(&source) {
            SweepDecision::Certified(proof) => proof,
            other => panic!("expected certified sweep, got {other:?}"),
        };
        assert!(proof.bounds.min()[0] <= -1.0);
        assert!(proof.bounds.max()[0] >= 2.0);
        assert!(proof.bounds.min()[1] <= -3.0);
        assert!(proof.bounds.max()[1] >= 1.0);
        assert!(proof.bounds.min()[2] <= -1.0);
        assert!(proof.bounds.max()[2] >= 4.0);
    }
}
