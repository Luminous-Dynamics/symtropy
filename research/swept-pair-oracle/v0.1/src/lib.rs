//! Fail-closed swept broadphase pair-set reference.
//!
//! This crate composes with directional-motion-envelopes. It is deliberately
//! an O(n²) qualification oracle, not a production broadphase. It only omits a
//! pair when both endpoint motion receipts replay and their conservative
//! envelopes are strictly disjoint. Unknown or unsupported motion always
//! falls back to retaining the pair.
//!
//! The independent temporal oracle below checks whether two linearly
//! translating AABBs overlap at a common time in [0, 1]. It solves the
//! per-axis linear inequalities directly; it does not reuse the swept-envelope
//! construction.

use directional_motion_envelopes::{
    Aabb, ColliderIdentity, EnvelopeReceipt, IntegratorEnvelopeReceipt, verify_envelope,
    verify_semi_implicit_euler_envelope,
};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedReason {
    Rotation,
    UnknownMotion,
    SkippedIntegratorPath,
    MissingMotionBound,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MotionEvidence<const D: usize> {
    Envelope(EnvelopeReceipt<D>),
    SemiImplicitEuler(IntegratorEnvelopeReceipt<D>),
    Unsupported(UnsupportedReason),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Proxy<const D: usize> {
    pub identity: ColliderIdentity,
    pub motion: MotionEvidence<D>,
    pub collision_group: u32,
    pub collision_mask: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairKey {
    pub left: ColliderIdentity,
    pub right: ColliderIdentity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OmissionWitness<const D: usize> {
    pub pair: PairKey,
    pub left_envelope: Aabb<D>,
    pub right_envelope: Aabb<D>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PairSetStats {
    pub input_proxies: usize,
    pub possible_pairs: usize,
    pub same_body_pairs: usize,
    pub filter_rejected_pairs: usize,
    pub eligible_pairs: usize,
    pub candidate_pairs: usize,
    pub omitted_pairs: usize,
    /// Subset of candidate_pairs retained because at least one endpoint has
    /// unsupported or unknown motion evidence.
    pub fallback_pairs: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairSetReceipt<const D: usize> {
    /// Canonically sorted, complete input population. This is an auditable
    /// replay snapshot, not a cryptographic signature.
    pub proxies: Vec<Proxy<D>>,
    pub candidate_pairs: Vec<PairKey>,
    pub fallback_pairs: Vec<PairKey>,
    pub omissions: Vec<OmissionWitness<D>>,
    pub stats: PairSetStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PairSetError {
    InvalidIdentity,
    DuplicateIdentity,
    ProxyIdentityMismatch,
    InvalidMotionEvidence,
    InvalidTemporalMotion,
    CountOverflow,
    ReplayMismatch,
}

fn identity_cmp(left: &ColliderIdentity, right: &ColliderIdentity) -> Ordering {
    left.body_slot
        .cmp(&right.body_slot)
        .then_with(|| left.body_generation.cmp(&right.body_generation))
        .then_with(|| left.collider_id.cmp(&right.collider_id))
        .then_with(|| left.subshape_path.cmp(&right.subshape_path))
}

fn same_body(left: &ColliderIdentity, right: &ColliderIdentity) -> bool {
    left.body_slot == right.body_slot && left.body_generation == right.body_generation
}

fn pair_key(left: &ColliderIdentity, right: &ColliderIdentity) -> PairKey {
    if identity_cmp(left, right) == Ordering::Greater {
        PairKey {
            left: right.clone(),
            right: left.clone(),
        }
    } else {
        PairKey {
            left: left.clone(),
            right: right.clone(),
        }
    }
}

fn pair_cmp(left: &PairKey, right: &PairKey) -> Ordering {
    identity_cmp(&left.left, &right.left).then_with(|| identity_cmp(&left.right, &right.right))
}

fn validate_proxy<const D: usize>(proxy: &Proxy<D>) -> Result<Option<Aabb<D>>, PairSetError> {
    proxy
        .identity
        .validate()
        .map_err(|_| PairSetError::InvalidIdentity)?;

    match &proxy.motion {
        MotionEvidence::Envelope(receipt) => {
            if receipt.identity != proxy.identity {
                return Err(PairSetError::ProxyIdentityMismatch);
            }
            verify_envelope(
                receipt,
                &proxy.identity,
                receipt.context,
                receipt.tight_bounds,
                receipt.displacement,
                receipt.guard,
            )
            .map_err(|_| PairSetError::InvalidMotionEvidence)?;
            Ok(Some(receipt.envelope))
        }
        MotionEvidence::SemiImplicitEuler(receipt) => {
            if receipt.base.identity != proxy.identity {
                return Err(PairSetError::ProxyIdentityMismatch);
            }
            verify_semi_implicit_euler_envelope(
                receipt,
                &proxy.identity,
                receipt.base.context,
                receipt.base.tight_bounds,
                receipt.motion,
                receipt.caller_guard,
            )
            .map_err(|_| PairSetError::InvalidMotionEvidence)?;
            Ok(Some(receipt.base.envelope))
        }
        MotionEvidence::Unsupported(_) => Ok(None),
    }
}

fn strictly_disjoint<const D: usize>(left: &Aabb<D>, right: &Aabb<D>) -> bool {
    (0..D).any(|axis| left.max()[axis] < right.min()[axis] || right.max()[axis] < left.min()[axis])
}

/// Build a deterministic candidate set from verified motion evidence.
///
/// Collision filtering follows Symtropy's symmetric group/mask rule. Pairs
/// within the same body generation are skipped. Touching envelope boundaries
/// are retained as candidates; only strict separation permits omission.
pub fn build_swept_pair_set<const D: usize>(
    proxies: &[Proxy<D>],
) -> Result<PairSetReceipt<D>, PairSetError> {
    let mut ordered = proxies.to_vec();
    ordered.sort_by(|left, right| identity_cmp(&left.identity, &right.identity));

    let mut envelopes = Vec::with_capacity(ordered.len());
    for proxy in &ordered {
        envelopes.push(validate_proxy(proxy)?);
    }

    for pair in ordered.windows(2) {
        if pair[0].identity == pair[1].identity {
            return Err(PairSetError::DuplicateIdentity);
        }
    }

    let n = ordered.len();
    let possible_pairs = n
        .checked_mul(n.saturating_sub(1))
        .map(|count| count / 2)
        .ok_or(PairSetError::CountOverflow)?;

    let mut receipt = PairSetReceipt {
        proxies: ordered,
        candidate_pairs: Vec::new(),
        fallback_pairs: Vec::new(),
        omissions: Vec::new(),
        stats: PairSetStats {
            input_proxies: n,
            possible_pairs,
            ..PairSetStats::default()
        },
    };

    for left_index in 0..n {
        for right_index in (left_index + 1)..n {
            let left = &receipt.proxies[left_index];
            let right = &receipt.proxies[right_index];

            if same_body(&left.identity, &right.identity) {
                receipt.stats.same_body_pairs += 1;
                continue;
            }

            let filters_accept = (left.collision_group & right.collision_mask != 0)
                && (right.collision_group & left.collision_mask != 0);
            if !filters_accept {
                receipt.stats.filter_rejected_pairs += 1;
                continue;
            }

            receipt.stats.eligible_pairs += 1;
            let key = pair_key(&left.identity, &right.identity);
            match (envelopes[left_index], envelopes[right_index]) {
                (Some(left_envelope), Some(right_envelope))
                    if strictly_disjoint(&left_envelope, &right_envelope) =>
                {
                    receipt.omissions.push(OmissionWitness {
                        pair: key,
                        left_envelope,
                        right_envelope,
                    });
                }
                (Some(_), Some(_)) => receipt.candidate_pairs.push(key),
                _ => {
                    receipt.stats.fallback_pairs += 1;
                    receipt.fallback_pairs.push(key.clone());
                    receipt.candidate_pairs.push(key);
                }
            }
        }
    }

    receipt.candidate_pairs.sort_by(pair_cmp);
    receipt.fallback_pairs.sort_by(pair_cmp);
    receipt
        .omissions
        .sort_by(|left, right| pair_cmp(&left.pair, &right.pair));

    receipt.stats.candidate_pairs = receipt.candidate_pairs.len();
    receipt.stats.omitted_pairs = receipt.omissions.len();

    if receipt.stats.eligible_pairs != receipt.stats.candidate_pairs + receipt.stats.omitted_pairs
        || receipt.stats.possible_pairs
            != receipt.stats.same_body_pairs
                + receipt.stats.filter_rejected_pairs
                + receipt.stats.eligible_pairs
    {
        return Err(PairSetError::ReplayMismatch);
    }

    Ok(receipt)
}

/// Verify a previously produced pair-set receipt against the current inputs.
/// This recomputes all dispositions and compares the complete canonical
/// snapshot. It provides replayability, not cryptographic authenticity.
pub fn verify_swept_pair_set<const D: usize>(
    proxies: &[Proxy<D>],
    receipt: &PairSetReceipt<D>,
) -> Result<(), PairSetError> {
    let replay = build_swept_pair_set(proxies)?;
    if &replay == receipt {
        Ok(())
    } else {
        Err(PairSetError::ReplayMismatch)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearMotionAabb<const D: usize> {
    pub identity: ColliderIdentity,
    pub start_bounds: Aabb<D>,
    /// Total translation from t=0 to t=1.
    pub displacement: [f64; D],
    pub collision_group: u32,
    pub collision_mask: u32,
}

fn intersect_linear_leq(
    constant: f64,
    slope: f64,
    interval: &mut (f64, f64),
) -> Result<bool, PairSetError> {
    if !constant.is_finite() || !slope.is_finite() {
        return Err(PairSetError::InvalidTemporalMotion);
    }
    if slope == 0.0 {
        return Ok(constant <= 0.0);
    }
    let crossing = -constant / slope;
    if !crossing.is_finite() {
        return Err(PairSetError::InvalidTemporalMotion);
    }
    if slope > 0.0 {
        interval.1 = interval.1.min(crossing);
    } else {
        interval.0 = interval.0.max(crossing);
    }
    Ok(interval.0 <= interval.1)
}

/// Independent common-time overlap oracle for two linearly translating AABBs.
///
/// For every axis it intersects [0, 1] with the two linear inequalities:
/// a.min(t) <= b.max(t) and b.min(t) <= a.max(t). A pair is positive only
/// when all axes admit one common time. This is exact for the stated
/// constant-translation AABB model (up to ordinary f64 arithmetic).
pub fn exact_linear_aabb_overlap<const D: usize>(
    a: &LinearMotionAabb<D>,
    b: &LinearMotionAabb<D>,
) -> Result<bool, PairSetError> {
    if !a.start_bounds.is_valid()
        || !b.start_bounds.is_valid()
        || a.displacement.iter().any(|value| !value.is_finite())
        || b.displacement.iter().any(|value| !value.is_finite())
    {
        return Err(PairSetError::InvalidTemporalMotion);
    }

    let mut interval = (0.0_f64, 1.0_f64);
    for axis in 0..D {
        let relative = a.displacement[axis] - b.displacement[axis];
        let a_before_b = a.start_bounds.min()[axis] - b.start_bounds.max()[axis];
        if !intersect_linear_leq(a_before_b, relative, &mut interval)? {
            return Ok(false);
        }

        let b_before_a = b.start_bounds.min()[axis] - a.start_bounds.max()[axis];
        if !intersect_linear_leq(b_before_a, -relative, &mut interval)? {
            return Ok(false);
        }
    }
    Ok(interval.0 <= interval.1)
}

/// Return every filter-eligible pair that the independent common-time oracle
/// finds overlapping. Same-body collider siblings are excluded.
pub fn temporal_oracle_pairs<const D: usize>(
    motions: &[LinearMotionAabb<D>],
) -> Result<Vec<PairKey>, PairSetError> {
    let mut ordered = motions.to_vec();
    for motion in &ordered {
        motion
            .identity
            .validate()
            .map_err(|_| PairSetError::InvalidIdentity)?;
        if !motion.start_bounds.is_valid()
            || motion.displacement.iter().any(|value| !value.is_finite())
        {
            return Err(PairSetError::InvalidTemporalMotion);
        }
    }
    ordered.sort_by(|left, right| identity_cmp(&left.identity, &right.identity));
    for pair in ordered.windows(2) {
        if pair[0].identity == pair[1].identity {
            return Err(PairSetError::DuplicateIdentity);
        }
    }

    let mut pairs = Vec::new();
    for i in 0..ordered.len() {
        for j in (i + 1)..ordered.len() {
            let left = &ordered[i];
            let right = &ordered[j];
            if same_body(&left.identity, &right.identity) {
                continue;
            }
            if (left.collision_group & right.collision_mask == 0)
                || (right.collision_group & left.collision_mask == 0)
            {
                continue;
            }
            if exact_linear_aabb_overlap(left, right)? {
                pairs.push(pair_key(&left.identity, &right.identity));
            }
        }
    }
    pairs.sort_by(pair_cmp);
    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use directional_motion_envelopes::{
        Digest, DisplacementInterval, EnvelopeContext, IntegrationParticipation, MotionKind,
        SemiImplicitEulerMotion, build_envelope, build_semi_implicit_euler_envelope,
    };

    fn identity(slot: u32) -> ColliderIdentity {
        ColliderIdentity {
            body_slot: slot,
            body_generation: 1,
            collider_id: u64::from(slot) + 10,
            subshape_path: Vec::new(),
        }
    }

    fn context() -> EnvelopeContext {
        EnvelopeContext {
            world_revision: 1,
            frame_index: 5,
            horizon_steps: 1,
            integrator: Digest::from_bytes([1; 32]),
            motion_bound_producer: Digest::from_bytes([2; 32]),
            arithmetic_profile: Digest::from_bytes([3; 32]),
        }
    }

    fn proxy_for<const D: usize>(motion: &LinearMotionAabb<D>) -> Proxy<D> {
        let min = std::array::from_fn(|axis| motion.displacement[axis].min(0.0));
        let max = std::array::from_fn(|axis| motion.displacement[axis].max(0.0));
        let displacement = DisplacementInterval { min, max };
        let result = build_envelope(
            motion.identity.clone(),
            context(),
            MotionKind::FixedOrientationTranslation,
            motion.start_bounds,
            displacement,
            0.0,
        )
        .unwrap();
        Proxy {
            identity: motion.identity.clone(),
            motion: MotionEvidence::Envelope(result.receipt.unwrap()),
            collision_group: motion.collision_group,
            collision_mask: motion.collision_mask,
        }
    }

    fn linear<const D: usize>(
        slot: u32,
        min: [f64; D],
        max: [f64; D],
        displacement: [f64; D],
    ) -> LinearMotionAabb<D> {
        LinearMotionAabb {
            identity: identity(slot),
            start_bounds: Aabb::new(min, max).unwrap(),
            displacement,
            collision_group: 1,
            collision_mask: u32::MAX,
        }
    }

    #[test]
    fn crossing_trajectories_are_candidates_and_temporal_oracle_positives() {
        let motions = [
            linear(1, [0.0, 0.0], [1.0, 1.0], [4.0, 0.0]),
            linear(2, [3.0, 0.0], [4.0, 1.0], [-4.0, 0.0]),
        ];
        let oracle = temporal_oracle_pairs(&motions).unwrap();
        let proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        let candidates = build_swept_pair_set(&proxies).unwrap();
        assert_eq!(oracle.len(), 1);
        assert_eq!(candidates.candidate_pairs, oracle);
        assert_eq!(candidates.stats.omitted_pairs, 0);
    }

    #[test]
    fn overlapping_swept_envelopes_can_be_a_temporal_false_positive() {
        let motions = [
            linear(1, [0.0], [1.0], [10.0]),
            linear(2, [4.0], [5.0], [10.0]),
        ];
        let oracle = temporal_oracle_pairs(&motions).unwrap();
        let proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        let candidates = build_swept_pair_set(&proxies).unwrap();
        assert!(oracle.is_empty());
        assert_eq!(candidates.stats.candidate_pairs, 1);
        assert_eq!(candidates.stats.omitted_pairs, 0);
    }

    #[test]
    fn semi_implicit_integrator_receipt_is_accepted_for_pair_qualification() {
        let static_motion = linear(1, [0.0], [1.0], [0.0]);
        let bounds = Aabb::new([10.0], [11.0]).unwrap();
        let motion = SemiImplicitEulerMotion {
            participation: IntegrationParticipation::AwakeDynamicBodyIntegrated,
            post_linear_velocity: [0.0],
            post_angular_velocity_norm_squared: 0.0,
            dt: 0.1,
            deterministic_net_q16_16: false,
        };
        let envelope =
            build_semi_implicit_euler_envelope(identity(2), context(), bounds, motion, 0.0)
                .unwrap();
        let moving_proxy = Proxy {
            identity: identity(2),
            motion: MotionEvidence::SemiImplicitEuler(envelope.receipt.unwrap()),
            collision_group: 1,
            collision_mask: u32::MAX,
        };
        let static_proxy = proxy_for(&static_motion);
        let receipt = build_swept_pair_set(&[static_proxy, moving_proxy]).unwrap();
        assert_eq!(receipt.stats.omitted_pairs, 1);
        assert!(receipt.candidate_pairs.is_empty());
        assert!(verify_swept_pair_set(&receipt.proxies, &receipt).is_ok());
    }

    #[test]
    fn unsupported_motion_is_always_retained_as_fallback() {
        let motions = [
            linear(1, [0.0, 0.0], [1.0, 1.0], [0.0, 0.0]),
            linear(2, [100.0, 100.0], [101.0, 101.0], [0.0, 0.0]),
        ];
        let mut proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        proxies[1].motion = MotionEvidence::Unsupported(UnsupportedReason::Rotation);
        let receipt = build_swept_pair_set(&proxies).unwrap();
        assert_eq!(receipt.stats.candidate_pairs, 1);
        assert_eq!(receipt.stats.fallback_pairs, 1);
        assert!(receipt.omissions.is_empty());
    }

    #[test]
    fn filters_and_same_body_siblings_are_accounted_separately() {
        let mut motions = [
            linear(1, [0.0], [1.0], [0.0]),
            linear(2, [0.0], [1.0], [0.0]),
            linear(3, [0.0], [1.0], [0.0]),
        ];
        motions[1].identity.body_slot = 1;
        motions[1].identity.collider_id += 100;
        motions[1].identity.subshape_path = vec![7];
        motions[1].collision_group = 2;
        motions[1].collision_mask = 2;
        motions[2].collision_group = 1;
        motions[2].collision_mask = 1;
        let proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        let receipt = build_swept_pair_set(&proxies).unwrap();
        assert_eq!(receipt.stats.possible_pairs, 3);
        assert_eq!(receipt.stats.same_body_pairs, 1);
        assert_eq!(receipt.stats.filter_rejected_pairs, 1);
        assert_eq!(receipt.stats.eligible_pairs, 1);
        assert_eq!(receipt.stats.candidate_pairs, 1);
    }

    #[test]
    fn input_order_does_not_change_pair_set_and_receipt_replays() {
        let motions = [
            linear(1, [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]),
            linear(2, [3.0, 0.0], [4.0, 1.0], [-1.0, 0.0]),
            linear(3, [10.0, 10.0], [11.0, 11.0], [0.0, 0.0]),
        ];
        let mut proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        let first = build_swept_pair_set(&proxies).unwrap();
        proxies.reverse();
        let second = build_swept_pair_set(&proxies).unwrap();
        assert_eq!(first, second);
        assert!(verify_swept_pair_set(&proxies, &first).is_ok());

        let mut changed = proxies.clone();
        changed[0].collision_mask = 0;
        assert_eq!(
            verify_swept_pair_set(&changed, &first),
            Err(PairSetError::ReplayMismatch)
        );
    }

    #[test]
    fn changed_generation_is_a_different_identity_and_invalidates_replay() {
        let motions = [
            linear(1, [0.0], [1.0], [1.0]),
            linear(2, [3.0], [4.0], [-1.0]),
        ];
        let proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        let receipt = build_swept_pair_set(&proxies).unwrap();
        let mut changed = proxies.clone();
        changed[1].identity.body_generation += 1;
        assert_eq!(
            verify_swept_pair_set(&changed, &receipt),
            Err(PairSetError::ProxyIdentityMismatch)
        );
    }

    #[test]
    fn tampered_motion_receipt_is_rejected_before_pair_omission() {
        let motions = [
            linear(1, [0.0], [1.0], [0.0]),
            linear(2, [10.0], [11.0], [0.0]),
        ];
        let mut proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        if let MotionEvidence::Envelope(receipt) = &mut proxies[1].motion {
            receipt.envelope = Aabb::new([0.0], [1.0]).unwrap();
        }
        assert_eq!(
            build_swept_pair_set(&proxies),
            Err(PairSetError::InvalidMotionEvidence)
        );
    }

    #[test]
    fn touching_at_the_final_time_remains_a_candidate() {
        let motions = [
            linear(1, [0.0], [1.0], [1.0]),
            linear(2, [2.0], [3.0], [0.0]),
        ];
        let oracle = temporal_oracle_pairs(&motions).unwrap();
        let proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        let receipt = build_swept_pair_set(&proxies).unwrap();
        assert_eq!(oracle.len(), 1);
        assert_eq!(receipt.candidate_pairs, oracle);
        assert!(receipt.omissions.is_empty());
    }

    #[test]
    fn proven_separation_emits_a_strict_omission_witness() {
        let motions = [
            linear(1, [0.0], [1.0], [0.0]),
            linear(2, [3.0], [4.0], [-0.5]),
        ];
        assert!(temporal_oracle_pairs(&motions).unwrap().is_empty());
        let proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        let receipt = build_swept_pair_set(&proxies).unwrap();
        assert!(receipt.candidate_pairs.is_empty());
        assert_eq!(receipt.stats.omitted_pairs, 1);
        let witness = &receipt.omissions[0];
        assert!(
            witness.left_envelope.max()[0] < witness.right_envelope.min()[0]
                || witness.right_envelope.max()[0] < witness.left_envelope.min()[0]
        );
    }

    #[test]
    fn non_finite_temporal_motion_is_rejected() {
        let mut motion = linear(1, [0.0], [1.0], [0.0]);
        motion.displacement[0] = f64::NAN;
        assert_eq!(
            temporal_oracle_pairs(&[motion]),
            Err(PairSetError::InvalidTemporalMotion)
        );
    }

    #[test]
    fn duplicate_proxy_identities_are_rejected() {
        let motion = linear(1, [0.0], [1.0], [0.0]);
        let proxy = proxy_for(&motion);
        assert_eq!(
            build_swept_pair_set(&[proxy.clone(), proxy]),
            Err(PairSetError::DuplicateIdentity)
        );
    }

    fn next_value(state: &mut u64, min: f64, max: f64) -> f64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let unit = ((*state >> 11) as f64) / ((1_u64 << 53) as f64);
        min + (max - min) * unit
    }

    fn generated_scene<const D: usize>(count: u32) -> Vec<LinearMotionAabb<D>> {
        let mut state = 0x8a5c_1f2d_9937_4b61_u64;
        (0..count)
            .map(|slot| {
                let min = std::array::from_fn(|_| next_value(&mut state, -20.0, 20.0));
                let extent: [f64; D] = std::array::from_fn(|_| next_value(&mut state, 0.1, 2.0));
                let max = std::array::from_fn(|axis| min[axis] + extent[axis]);
                let displacement = std::array::from_fn(|_| next_value(&mut state, -15.0, 15.0));
                linear(slot + 1, min, max, displacement)
            })
            .collect()
    }

    fn assert_generated_recall<const D: usize>() {
        let motions = generated_scene::<D>(72);
        let oracle = temporal_oracle_pairs(&motions).unwrap();
        let proxies: Vec<_> = motions.iter().map(proxy_for).collect();
        let receipt = build_swept_pair_set(&proxies).unwrap();
        for pair in &oracle {
            assert!(
                receipt.candidate_pairs.contains(pair),
                "temporal oracle pair omitted: {:?}",
                pair
            );
        }
        assert_eq!(
            receipt.stats.eligible_pairs,
            receipt.stats.candidate_pairs + receipt.stats.omitted_pairs
        );
        assert!(receipt.stats.candidate_pairs >= oracle.len());
    }

    #[test]
    fn generated_2d_temporal_oracle_has_zero_false_negatives() {
        assert_generated_recall::<2>();
    }

    #[test]
    fn generated_3d_temporal_oracle_has_zero_false_negatives() {
        assert_generated_recall::<3>();
    }

    #[test]
    fn generated_4d_temporal_oracle_has_zero_false_negatives() {
        assert_generated_recall::<4>();
    }
}
