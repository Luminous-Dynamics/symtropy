// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Proof-carrying disjointness checks for supplied AABB envelopes.
//!
//! A certificate proves only that two exact represented AABBs are strictly
//! disjoint. It does not prove that either AABB conservatively encloses a
//! moving shape over a timestep. That obligation belongs to the bound producer.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundInputError {
    ZeroDimension,
    NonFiniteEndpoint,
    InvertedBounds,
    NonFiniteTime,
    ReversedTimeInterval,
    EmptyProvenance,
    SelfPair,
    ContextMismatch,
}

impl fmt::Display for BoundInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => write!(f, "AABB dimension must be greater than zero"),
            Self::NonFiniteEndpoint => write!(f, "AABB endpoints must be finite"),
            Self::InvertedBounds => write!(f, "AABB minimum must not exceed maximum"),
            Self::NonFiniteTime => write!(f, "proof interval endpoints must be finite"),
            Self::ReversedTimeInterval => write!(f, "proof interval start must not exceed end"),
            Self::EmptyProvenance => write!(f, "proof provenance identifiers must not be empty"),
            Self::SelfPair => write!(f, "a collider cannot be paired with itself"),
            Self::ContextMismatch => write!(f, "both bounds must share the same proof context"),
        }
    }
}

impl Error for BoundInputError {}

/// Context prevents replay against another world revision, step, interval,
/// trajectory model, transform profile, arithmetic profile, or bound provider.
/// The identifiers are caller-supplied labels, not authenticated identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofContext {
    world_revision: u64,
    step_index: u64,
    interval_start_bits: u64,
    interval_end_bits: u64,
    integrator_id: String,
    integrator_version: String,
    bound_provider_id: String,
    bound_provider_version: String,
    transform_profile_id: String,
    arithmetic_profile_id: String,
}

impl ProofContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_revision: u64,
        step_index: u64,
        interval_start: f64,
        interval_end: f64,
        integrator_id: impl Into<String>,
        integrator_version: impl Into<String>,
        bound_provider_id: impl Into<String>,
        bound_provider_version: impl Into<String>,
        transform_profile_id: impl Into<String>,
        arithmetic_profile_id: impl Into<String>,
    ) -> Result<Self, BoundInputError> {
        if !interval_start.is_finite() || !interval_end.is_finite() {
            return Err(BoundInputError::NonFiniteTime);
        }
        if interval_start > interval_end {
            return Err(BoundInputError::ReversedTimeInterval);
        }
        let labels = [
            integrator_id.into(),
            integrator_version.into(),
            bound_provider_id.into(),
            bound_provider_version.into(),
            transform_profile_id.into(),
            arithmetic_profile_id.into(),
        ];
        if labels.iter().any(|value| value.trim().is_empty()) {
            return Err(BoundInputError::EmptyProvenance);
        }
        Ok(Self {
            world_revision,
            step_index,
            interval_start_bits: interval_start.to_bits(),
            interval_end_bits: interval_end.to_bits(),
            integrator_id: labels[0].clone(),
            integrator_version: labels[1].clone(),
            bound_provider_id: labels[2].clone(),
            bound_provider_version: labels[3].clone(),
            transform_profile_id: labels[4].clone(),
            arithmetic_profile_id: labels[5].clone(),
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
    pub fn interval_start(&self) -> f64 {
        f64::from_bits(self.interval_start_bits)
    }

    #[must_use]
    pub fn interval_end(&self) -> f64 {
        f64::from_bits(self.interval_end_bits)
    }
}

/// Stable collider identity. Generation prevents a recycled body handle from
/// being confused with the body that previously occupied that handle.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ColliderId {
    body_handle: u64,
    body_generation: u32,
    collider_id: u32,
    subshape_path: Vec<u32>,
}

impl ColliderId {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColliderPair {
    first: ColliderId,
    second: ColliderId,
}

impl ColliderPair {
    pub fn new(a: ColliderId, b: ColliderId) -> Result<Self, BoundInputError> {
        match a.cmp(&b) {
            Ordering::Less => Ok(Self {
                first: a,
                second: b,
            }),
            Ordering::Greater => Ok(Self {
                first: b,
                second: a,
            }),
            Ordering::Equal => Err(BoundInputError::SelfPair),
        }
    }

    #[must_use]
    pub fn first(&self) -> &ColliderId {
        &self.first
    }

    #[must_use]
    pub fn second(&self) -> &ColliderId {
        &self.second
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb<const D: usize> {
    min: [f64; D],
    max: [f64; D],
}

impl<const D: usize> Aabb<D> {
    pub fn new(min: [f64; D], max: [f64; D]) -> Result<Self, BoundInputError> {
        if D == 0 {
            return Err(BoundInputError::ZeroDimension);
        }
        for (lower, upper) in min.iter().zip(max.iter()) {
            if !lower.is_finite() || !upper.is_finite() {
                return Err(BoundInputError::NonFiniteEndpoint);
            }
            if lower > upper {
                return Err(BoundInputError::InvertedBounds);
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
}

/// An exact represented envelope with caller-supplied producer provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundEnvelope<const D: usize> {
    collider: ColliderId,
    context: ProofContext,
    aabb: Aabb<D>,
}

impl<const D: usize> BoundEnvelope<D> {
    pub fn new(
        collider: ColliderId,
        context: ProofContext,
        min: [f64; D],
        max: [f64; D],
    ) -> Result<Self, BoundInputError> {
        Ok(Self {
            collider,
            context,
            aabb: Aabb::new(min, max)?,
        })
    }

    #[must_use]
    pub fn collider(&self) -> &ColliderId {
        &self.collider
    }

    #[must_use]
    pub fn context(&self) -> &ProofContext {
        &self.context
    }

    #[must_use]
    pub fn aabb(&self) -> &Aabb<D> {
        &self.aabb
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedReason {
    NoConservativeBoundProvider,
    UnsupportedShape,
    UnsupportedMotionModel,
    ArithmeticProfileUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NonPruneReason {
    BoundsOverlapOrTouch,
    GapMarginNotStrictlyPositive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeparationOrdering {
    FirstBeforeSecond,
    SecondBeforeFirst,
}

/// Outward-rounded interval containing the positive coordinate gap.
/// The upper endpoint may be infinity when finite input endpoints have a
/// mathematical difference larger than f64::MAX.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GapInterval {
    lower: f64,
    upper: f64,
}

impl GapInterval {
    #[must_use]
    pub fn lower(&self) -> f64 {
        self.lower
    }

    #[must_use]
    pub fn upper(&self) -> f64 {
        self.upper
    }
}

/// Re-playable proof of disjointness for the supplied AABB snapshots only.
#[derive(Clone, Debug, PartialEq)]
pub struct AabbSeparationCertificate<const D: usize> {
    pair: ColliderPair,
    context: ProofContext,
    first: BoundEnvelope<D>,
    second: BoundEnvelope<D>,
    axis: usize,
    ordering: SeparationOrdering,
    gap: GapInterval,
}

impl<const D: usize> AabbSeparationCertificate<D> {
    #[must_use]
    pub fn pair(&self) -> &ColliderPair {
        &self.pair
    }

    #[must_use]
    pub fn axis(&self) -> usize {
        self.axis
    }

    #[must_use]
    pub fn ordering(&self) -> SeparationOrdering {
        self.ordering
    }

    #[must_use]
    pub fn gap(&self) -> GapInterval {
        self.gap
    }

    #[must_use]
    pub fn context(&self) -> &ProofContext {
        &self.context
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundAvailability<'a, const D: usize> {
    Available(&'a BoundEnvelope<D>),
    Unsupported(UnsupportedReason),
    Invalid(BoundInputError),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CullingDecision<const D: usize> {
    CertifiedDisjoint(Box<AabbSeparationCertificate<D>>),
    OverlapPossible(NonPruneReason),
    Unsupported(UnsupportedReason),
    Invalid(BoundInputError),
}

/// Only a strictly positive outward-rounded gap may produce a certificate.
/// Every other result must continue refinement or exhaustive fallback.
#[must_use]
pub fn classify_pair<const D: usize>(
    left: BoundAvailability<'_, D>,
    right: BoundAvailability<'_, D>,
) -> CullingDecision<D> {
    let (left, right) = match (left, right) {
        (BoundAvailability::Invalid(error), _) | (_, BoundAvailability::Invalid(error)) => {
            return CullingDecision::Invalid(error);
        }
        (BoundAvailability::Unsupported(reason), _) => {
            return CullingDecision::Unsupported(reason);
        }
        (_, BoundAvailability::Unsupported(reason)) => {
            return CullingDecision::Unsupported(reason);
        }
        (BoundAvailability::Available(left), BoundAvailability::Available(right)) => (left, right),
    };

    if left.context != right.context {
        return CullingDecision::Invalid(BoundInputError::ContextMismatch);
    }

    let (first, second) = match left.collider.cmp(&right.collider) {
        Ordering::Less => (left, right),
        Ordering::Greater => (right, left),
        Ordering::Equal => return CullingDecision::Invalid(BoundInputError::SelfPair),
    };
    let pair = match ColliderPair::new(first.collider.clone(), second.collider.clone()) {
        Ok(pair) => pair,
        Err(error) => return CullingDecision::Invalid(error),
    };

    let mut best: Option<(usize, SeparationOrdering, GapInterval)> = None;
    let mut strict_separation_found = false;

    for (axis, ((&first_min, &first_max), (&second_min, &second_max))) in first
        .aabb
        .min
        .iter()
        .zip(first.aabb.max.iter())
        .zip(second.aabb.min.iter().zip(second.aabb.max.iter()))
        .enumerate()
    {
        let separation = if first_max < second_min {
            Some((
                second_min - first_max,
                SeparationOrdering::FirstBeforeSecond,
            ))
        } else if second_max < first_min {
            Some((
                first_min - second_max,
                SeparationOrdering::SecondBeforeFirst,
            ))
        } else {
            None
        };

        if let Some((rounded_gap, ordering)) = separation {
            strict_separation_found = true;
            if let Some(gap) = outward_gap_interval(rounded_gap) {
                let improves = best
                    .as_ref()
                    .is_none_or(|(_, _, current)| gap.lower > current.lower);
                if gap.lower > 0.0 && improves {
                    best = Some((axis, ordering, gap));
                }
            }
        }
    }

    match best {
        Some((axis, ordering, gap)) => {
            CullingDecision::CertifiedDisjoint(Box::new(AabbSeparationCertificate {
                pair,
                context: first.context.clone(),
                first: first.clone(),
                second: second.clone(),
                axis,
                ordering,
                gap,
            }))
        }
        None if strict_separation_found => {
            CullingDecision::OverlapPossible(NonPruneReason::GapMarginNotStrictlyPositive)
        }
        None => CullingDecision::OverlapPossible(NonPruneReason::BoundsOverlapOrTouch),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateError {
    PairMismatch,
    ContextMismatch,
    BoundSnapshotMismatch,
    AxisOutOfRange,
    BoundsNotStrictlySeparated,
    GapMarginUnavailable,
    GapIntervalMismatch,
}

impl fmt::Display for CertificateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for CertificateError {}

/// Replay against current envelopes. Input order is irrelevant; identities are
/// canonicalized and all endpoint comparisons are repeated by the verifier.
pub fn verify_certificate<const D: usize>(
    certificate: &AabbSeparationCertificate<D>,
    left: &BoundEnvelope<D>,
    right: &BoundEnvelope<D>,
) -> Result<(), CertificateError> {
    if left.context != right.context || certificate.context != left.context {
        return Err(CertificateError::ContextMismatch);
    }

    let (first, second) = match left.collider.cmp(&right.collider) {
        Ordering::Less => (left, right),
        Ordering::Greater => (right, left),
        Ordering::Equal => return Err(CertificateError::PairMismatch),
    };
    let expected_pair = ColliderPair::new(first.collider.clone(), second.collider.clone())
        .map_err(|_| CertificateError::PairMismatch)?;
    if certificate.pair != expected_pair {
        return Err(CertificateError::PairMismatch);
    }
    if !same_envelope(&certificate.first, first) || !same_envelope(&certificate.second, second) {
        return Err(CertificateError::BoundSnapshotMismatch);
    }
    if certificate.axis >= D {
        return Err(CertificateError::AxisOutOfRange);
    }

    let (near, far) = match certificate.ordering {
        SeparationOrdering::FirstBeforeSecond => (
            first.aabb.max[certificate.axis],
            second.aabb.min[certificate.axis],
        ),
        SeparationOrdering::SecondBeforeFirst => (
            second.aabb.max[certificate.axis],
            first.aabb.min[certificate.axis],
        ),
    };
    if near >= far {
        return Err(CertificateError::BoundsNotStrictlySeparated);
    }

    let expected_gap =
        outward_gap_interval(far - near).ok_or(CertificateError::GapMarginUnavailable)?;
    if expected_gap.lower <= 0.0 {
        return Err(CertificateError::GapMarginUnavailable);
    }
    if !same_gap(certificate.gap, expected_gap) {
        return Err(CertificateError::GapIntervalMismatch);
    }
    Ok(())
}

fn same_envelope<const D: usize>(left: &BoundEnvelope<D>, right: &BoundEnvelope<D>) -> bool {
    left.collider == right.collider
        && left.context == right.context
        && same_aabb(left.aabb, right.aabb)
}

fn same_aabb<const D: usize>(left: Aabb<D>, right: Aabb<D>) -> bool {
    left.min
        .iter()
        .zip(right.min.iter())
        .all(|(a, b)| a.to_bits() == b.to_bits())
        && left
            .max
            .iter()
            .zip(right.max.iter())
            .all(|(a, b)| a.to_bits() == b.to_bits())
}

fn same_gap(left: GapInterval, right: GapInterval) -> bool {
    left.lower.to_bits() == right.lower.to_bits() && left.upper.to_bits() == right.upper.to_bits()
}

/// The rounded subtraction of finite represented endpoints is stepped one
/// representable value outward on either side to enclose the exact positive gap.
fn outward_gap_interval(rounded_gap: f64) -> Option<GapInterval> {
    if rounded_gap == f64::INFINITY {
        return Some(GapInterval {
            lower: f64::MAX,
            upper: f64::INFINITY,
        });
    }
    if !rounded_gap.is_finite() || rounded_gap <= 0.0 {
        return None;
    }
    Some(GapInterval {
        lower: next_down_positive(rounded_gap),
        upper: next_up_positive(rounded_gap),
    })
}

fn next_down_positive(value: f64) -> f64 {
    debug_assert!(value.is_finite() && value > 0.0);
    f64::from_bits(value.to_bits() - 1)
}

fn next_up_positive(value: f64) -> f64 {
    debug_assert!(value.is_finite() && value > 0.0);
    f64::from_bits(value.to_bits() + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> ProofContext {
        ProofContext::new(
            7,
            19,
            0.0,
            1.0 / 60.0,
            "semi-implicit-euler",
            "v3",
            "test-aabb-provider",
            "v1",
            "world-transform-v2",
            "ieee754-binary64-nearest",
        )
        .unwrap()
    }

    fn collider(body_handle: u64) -> ColliderId {
        ColliderId::new(body_handle, 4, 2, vec![1, 3])
    }

    fn bound(body_handle: u64, min: [f64; 2], max: [f64; 2]) -> BoundEnvelope<2> {
        BoundEnvelope::new(collider(body_handle), context(), min, max).unwrap()
    }

    fn certificate(
        left: &BoundEnvelope<2>,
        right: &BoundEnvelope<2>,
    ) -> AabbSeparationCertificate<2> {
        match classify_pair(
            BoundAvailability::Available(left),
            BoundAvailability::Available(right),
        ) {
            CullingDecision::CertifiedDisjoint(proof) => *proof,
            other => panic!("expected certificate, got {other:?}"),
        }
    }

    #[test]
    fn x_axis_separation_emits_a_replayable_certificate() {
        let left = bound(1, [0.0, 0.0], [1.0, 1.0]);
        let right = bound(2, [2.0, 0.0], [3.0, 1.0]);
        let proof = certificate(&left, &right);
        assert_eq!(proof.axis(), 0);
        assert_eq!(proof.ordering(), SeparationOrdering::FirstBeforeSecond);
        assert!(proof.gap().lower() > 0.0);
        verify_certificate(&proof, &left, &right).unwrap();
    }

    #[test]
    fn canonical_pair_and_certificate_are_independent_of_input_order() {
        let left = bound(1, [0.0, 0.0], [1.0, 1.0]);
        let right = bound(2, [2.0, 0.0], [3.0, 1.0]);
        let forward = certificate(&left, &right);
        let reverse = certificate(&right, &left);
        assert_eq!(forward, reverse);
        verify_certificate(&forward, &right, &left).unwrap();
    }

    #[test]
    fn later_axis_can_supply_the_separation_witness() {
        let left = bound(1, [0.0, 0.0], [2.0, 1.0]);
        let right = bound(2, [1.0, 2.0], [3.0, 3.0]);
        let proof = certificate(&left, &right);
        assert_eq!(proof.axis(), 1);
        verify_certificate(&proof, &left, &right).unwrap();
    }

    #[test]
    fn touching_and_overlapping_bounds_are_not_pruned() {
        let touching_left = bound(1, [0.0, 0.0], [1.0, 1.0]);
        let touching_right = bound(2, [1.0, 0.0], [2.0, 1.0]);
        let overlapping = bound(2, [0.5, 0.5], [2.0, 2.0]);
        assert_eq!(
            classify_pair(
                BoundAvailability::Available(&touching_left),
                BoundAvailability::Available(&touching_right)
            ),
            CullingDecision::OverlapPossible(NonPruneReason::BoundsOverlapOrTouch)
        );
        assert_eq!(
            classify_pair(
                BoundAvailability::Available(&touching_left),
                BoundAvailability::Available(&overlapping)
            ),
            CullingDecision::OverlapPossible(NonPruneReason::BoundsOverlapOrTouch)
        );
    }

    #[test]
    fn context_mismatch_and_self_pair_are_invalid() {
        let left = bound(1, [0.0, 0.0], [1.0, 1.0]);
        let other_context = ProofContext::new(
            8,
            19,
            0.0,
            1.0 / 60.0,
            "semi-implicit-euler",
            "v3",
            "test-aabb-provider",
            "v1",
            "world-transform-v2",
            "ieee754-binary64-nearest",
        )
        .unwrap();
        let right = BoundEnvelope::new(collider(2), other_context, [2.0, 0.0], [3.0, 1.0]).unwrap();
        assert_eq!(
            classify_pair(
                BoundAvailability::Available(&left),
                BoundAvailability::Available(&right)
            ),
            CullingDecision::Invalid(BoundInputError::ContextMismatch)
        );
        assert_eq!(
            classify_pair(
                BoundAvailability::Available(&left),
                BoundAvailability::Available(&left)
            ),
            CullingDecision::Invalid(BoundInputError::SelfPair)
        );
    }

    #[test]
    fn unsupported_and_invalid_inputs_force_fallback() {
        let available = bound(1, [0.0, 0.0], [1.0, 1.0]);
        assert_eq!(
            classify_pair(
                BoundAvailability::Available(&available),
                BoundAvailability::Unsupported(UnsupportedReason::UnsupportedMotionModel)
            ),
            CullingDecision::Unsupported(UnsupportedReason::UnsupportedMotionModel)
        );
        assert_eq!(
            classify_pair(
                BoundAvailability::Available(&available),
                BoundAvailability::Invalid(BoundInputError::NonFiniteEndpoint)
            ),
            CullingDecision::Invalid(BoundInputError::NonFiniteEndpoint)
        );
    }

    #[test]
    fn changed_bounds_and_signed_zero_bit_changes_fail_replay() {
        let left = bound(1, [-0.0, 0.0], [1.0, 1.0]);
        let right = bound(2, [2.0, 0.0], [3.0, 1.0]);
        let proof = certificate(&left, &right);
        let changed_left = bound(1, [0.0, 0.0], [1.0, 1.0]);
        assert_eq!(
            verify_certificate(&proof, &changed_left, &right),
            Err(CertificateError::BoundSnapshotMismatch)
        );
        let changed_right = bound(2, [2.25, 0.0], [3.0, 1.0]);
        assert_eq!(
            verify_certificate(&proof, &left, &changed_right),
            Err(CertificateError::BoundSnapshotMismatch)
        );
    }

    #[test]
    fn corrupted_gap_interval_is_rejected() {
        let left = bound(1, [0.0, 0.0], [1.0, 1.0]);
        let right = bound(2, [2.0, 0.0], [3.0, 1.0]);
        let mut proof = certificate(&left, &right);
        proof.gap.lower = 0.5;
        assert_eq!(
            verify_certificate(&proof, &left, &right),
            Err(CertificateError::GapIntervalMismatch)
        );
    }

    #[test]
    fn smallest_subnormal_gap_is_not_certified() {
        let left = bound(1, [0.0, 0.0], [0.0, 1.0]);
        let right = bound(2, [f64::from_bits(1), 0.0], [1.0, 1.0]);
        assert_eq!(
            classify_pair(
                BoundAvailability::Available(&left),
                BoundAvailability::Available(&right)
            ),
            CullingDecision::OverlapPossible(NonPruneReason::GapMarginNotStrictlyPositive)
        );
    }

    #[test]
    fn overflowing_gap_has_a_finite_positive_lower_bound() {
        let left = bound(1, [-f64::MAX, 0.0], [-f64::MAX, 1.0]);
        let right = bound(2, [f64::MAX, 0.0], [f64::MAX, 1.0]);
        let proof = certificate(&left, &right);
        assert_eq!(proof.gap().lower(), f64::MAX);
        assert_eq!(proof.gap().upper(), f64::INFINITY);
        verify_certificate(&proof, &left, &right).unwrap();
    }

    #[test]
    fn malformed_aabbs_and_time_intervals_are_rejected() {
        assert_eq!(Aabb::<0>::new([], []), Err(BoundInputError::ZeroDimension));
        assert_eq!(
            Aabb::new([f64::NAN, 0.0], [1.0, 1.0]),
            Err(BoundInputError::NonFiniteEndpoint)
        );
        assert_eq!(
            Aabb::new([2.0, 0.0], [1.0, 1.0]),
            Err(BoundInputError::InvertedBounds)
        );
        assert_eq!(
            ProofContext::new(1, 1, 0.0, f64::INFINITY, "i", "1", "b", "1", "t", "a"),
            Err(BoundInputError::NonFiniteTime)
        );
        assert_eq!(
            ProofContext::new(1, 1, 2.0, 1.0, "i", "1", "b", "1", "t", "a"),
            Err(BoundInputError::ReversedTimeInterval)
        );
    }

    #[test]
    fn three_dimensional_bounds_use_the_same_contract() {
        let ctx = context();
        let left = BoundEnvelope::new(
            ColliderId::new(1, 0, 0, vec![]),
            ctx.clone(),
            [0.0; 3],
            [1.0; 3],
        )
        .unwrap();
        let right = BoundEnvelope::new(
            ColliderId::new(2, 0, 0, vec![]),
            ctx,
            [0.0, 0.0, 2.0],
            [1.0, 1.0, 3.0],
        )
        .unwrap();
        let proof = match classify_pair(
            BoundAvailability::Available(&left),
            BoundAvailability::Available(&right),
        ) {
            CullingDecision::CertifiedDisjoint(proof) => proof,
            other => panic!("expected certificate, got {other:?}"),
        };
        assert_eq!(proof.axis(), 2);
        verify_certificate(&proof, &left, &right).unwrap();
    }
}
