//! Deterministic time-swept multi-body broad phase for translating capsules.
//!
//! The broad phase partitions a common absolute-time horizon into dyadic slabs.
//! Within each slab it:
//! 1. rebases every active capsule to its own active portion of the slab;
//! 2. builds an outward-rounded swept AABB;
//! 3. selects the axis with the greatest total spread;
//! 4. performs stable sweep-and-prune on that axis;
//! 5. intersects the two bodies' active time intervals and rebuilds both boxes
//!    on that *same* interval before emitting a pair candidate.
//!
//! The final candidate is still broad phase only. It never means that the
//! capsules collide. The narrow-phase owner is timed_co_translating_ccd.
//!
//! Fixed-orientation linear translation and a shared exact monotonic clock are
//! the complete supported geometry/time model. Non-zero clock uncertainty
//! abstains through the existing timed-motion validator.

use std::collections::{BTreeMap, BTreeSet};

use crate::timed_co_translation_ccd::{
    CapsuleMotionSpan, TimedCcdError, rebase_span_to_time, validate_capsule_motion_span,
};
use crate::{Capsule, CapsuleAabb3, CapsuleError, Vec3};

pub const MAX_TIME_SLABS: u32 = 4_096;
pub const MAX_BODY_SLAB_CELLS: u64 = 4_194_304;
const OUTWARD_ULP_STEPS: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct TimedCapsuleBody {
    pub body_id: String,
    pub motion: CapsuleMotionSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SweepAxis {
    X,
    Y,
    Z,
}

impl SweepAxis {
    fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateSlab {
    pub slab_index: u32,
    /// Positive-duration common active interval for this pair inside the slab.
    pub overlap_start_ms: u64,
    pub overlap_end_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimedCapsulePairCandidate {
    /// IDs are lexicographically ordered for deterministic pair identity.
    pub first_body_id: String,
    pub second_body_id: String,
    pub active_overlap_start_ms: u64,
    pub active_overlap_end_ms: u64,
    pub active_duration_ms: u64,
    pub candidate_duration_ms: u64,
    pub candidate_time_fraction: f64,
    pub candidate_slabs: Vec<CandidateSlab>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlabMetrics {
    pub slab_index: u32,
    pub slab_start_ms: u64,
    pub slab_end_ms: u64,
    pub active_body_count: u32,
    pub sweep_axis: SweepAxis,
    /// Pairs whose per-body slab AABBs overlap on all three axes.
    pub coarse_spatial_pairs: u64,
    /// Coarse spatial pairs with a positive common active-time interval.
    pub time_coherent_pairs: u64,
    /// Pairs still overlapping after both boxes are rebuilt on common time.
    pub emitted_pair_candidates: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MultiBodyBroadphaseReport {
    pub horizon_start_ms: u64,
    pub horizon_end_ms: u64,
    pub subdivision_count: u32,
    pub body_count: u32,
    pub active_body_slab_cells: u64,
    pub total_coarse_spatial_pairs: u64,
    pub total_time_coherent_pairs: u64,
    pub total_emitted_pair_candidates: u64,
    pub pairs: Vec<TimedCapsulePairCandidate>,
    pub slabs: Vec<SlabMetrics>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiBodyBroadphaseError {
    NoBodies,
    EmptyBodyId,
    DuplicateBodyId,
    BodyIdLookupFailure,
    InvalidSubdivisionCount,
    TooManyBodySlabCells,
    SiteEpochMismatch,
    FrameMismatch,
    ClockDomainMismatch,
    TimeArithmeticOverflow,
    GeometryOverflow,
    Timed(TimedCcdError),
}

impl From<TimedCcdError> for MultiBodyBroadphaseError {
    fn from(error: TimedCcdError) -> Self {
        Self::Timed(error)
    }
}

/// Generate sorted conservative candidate pairs for a collection of
/// time-indexed capsule translations.
///
/// `subdivision_count` must be a power of two in [1, 4096]. More slabs
/// tighten time-coherent AABB pruning but increase per-body slab work. All
/// records must share site epoch, frame and exact clock domain. Clock
/// uncertainty is not silently dropped: the existing span validator rejects
/// non-zero uncertainty.
pub fn time_swept_capsule_broadphase(
    bodies: &[TimedCapsuleBody],
    subdivision_count: u32,
) -> Result<MultiBodyBroadphaseReport, MultiBodyBroadphaseError> {
    if bodies.is_empty() {
        return Err(MultiBodyBroadphaseError::NoBodies);
    }
    if subdivision_count == 0
        || !subdivision_count.is_power_of_two()
        || subdivision_count > MAX_TIME_SLABS
    {
        return Err(MultiBodyBroadphaseError::InvalidSubdivisionCount);
    }

    let mut ids = BTreeSet::new();
    for body in bodies {
        if body.body_id.trim().is_empty() {
            return Err(MultiBodyBroadphaseError::EmptyBodyId);
        }
        if !ids.insert(body.body_id.as_str()) {
            return Err(MultiBodyBroadphaseError::DuplicateBodyId);
        }
        validate_capsule_motion_span(&body.motion)?;
    }

    let first = &bodies[0].motion;
    for body in &bodies[1..] {
        if body.motion.site_epoch != first.site_epoch {
            return Err(MultiBodyBroadphaseError::SiteEpochMismatch);
        }
        if body.motion.frame_id != first.frame_id {
            return Err(MultiBodyBroadphaseError::FrameMismatch);
        }
        if body.motion.clock_domain_id != first.clock_domain_id {
            return Err(MultiBodyBroadphaseError::ClockDomainMismatch);
        }
    }

    let body_count =
        u32::try_from(bodies.len()).map_err(|_| MultiBodyBroadphaseError::TooManyBodySlabCells)?;
    let requested_cells = (bodies.len() as u128) * (subdivision_count as u128);
    if requested_cells > u128::from(MAX_BODY_SLAB_CELLS) {
        return Err(MultiBodyBroadphaseError::TooManyBodySlabCells);
    }

    let horizon_start_ms = bodies
        .iter()
        .map(|body| body.motion.start_ms)
        .min()
        .ok_or(MultiBodyBroadphaseError::NoBodies)?;
    let horizon_end_ms = bodies
        .iter()
        .map(|body| body.motion.end_ms)
        .max()
        .ok_or(MultiBodyBroadphaseError::NoBodies)?;
    let horizon_duration = horizon_end_ms
        .checked_sub(horizon_start_ms)
        .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;

    let mut body_order: Vec<usize> = (0..bodies.len()).collect();
    body_order.sort_by(|left, right| bodies[*left].body_id.cmp(&bodies[*right].body_id));

    let mut pair_slabs: BTreeMap<(String, String), Vec<CandidateSlab>> = BTreeMap::new();
    let mut slab_metrics = Vec::with_capacity(subdivision_count as usize);
    let mut active_body_slab_cells = 0_u64;
    let mut total_coarse_spatial_pairs = 0_u64;
    let mut total_time_coherent_pairs = 0_u64;
    let mut total_emitted_pair_candidates = 0_u64;

    for slab_index in 0..subdivision_count {
        let slab_start_ms = slab_boundary(
            horizon_start_ms,
            horizon_duration,
            slab_index,
            subdivision_count,
        )?;
        let slab_end_ms = slab_boundary(
            horizon_start_ms,
            horizon_duration,
            slab_index + 1,
            subdivision_count,
        )?;
        if slab_start_ms >= slab_end_ms {
            continue;
        }

        let mut entries = Vec::new();
        for body_index in &body_order {
            let span = &bodies[*body_index].motion;
            let active_start_ms = slab_start_ms.max(span.start_ms);
            let active_end_ms = slab_end_ms.min(span.end_ms);
            if active_start_ms >= active_end_ms {
                continue;
            }
            let bounds = swept_bounds(span, active_start_ms, active_end_ms)?;
            entries.push(SlabEntry {
                body_index: *body_index,
                active_start_ms,
                active_end_ms,
                bounds,
            });
        }

        active_body_slab_cells = active_body_slab_cells
            .checked_add(entries.len() as u64)
            .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;

        if entries.len() < 2 {
            slab_metrics.push(SlabMetrics {
                slab_index,
                slab_start_ms,
                slab_end_ms,
                active_body_count: entries.len() as u32,
                sweep_axis: SweepAxis::X,
                coarse_spatial_pairs: 0,
                time_coherent_pairs: 0,
                emitted_pair_candidates: 0,
            });
            continue;
        }

        let sweep_axis = select_sweep_axis(&entries);
        let axis_index = sweep_axis.index();
        entries.sort_by(|left, right| {
            component(left.bounds.min, axis_index)
                .total_cmp(&component(right.bounds.min, axis_index))
                .then_with(|| {
                    component(left.bounds.max, axis_index)
                        .total_cmp(&component(right.bounds.max, axis_index))
                })
                .then_with(|| {
                    bodies[left.body_index]
                        .body_id
                        .cmp(&bodies[right.body_index].body_id)
                })
        });

        let mut active: Vec<usize> = Vec::new();
        let mut slab_coarse = 0_u64;
        let mut slab_time_coherent = 0_u64;
        let mut slab_emitted = 0_u64;

        for current_index in 0..entries.len() {
            let current = &entries[current_index];
            let current_min = component(current.bounds.min, axis_index);
            active.retain(|previous_index| {
                component(entries[*previous_index].bounds.max, axis_index) >= current_min
            });

            for previous_index in active.iter().copied() {
                let previous = &entries[previous_index];
                if !overlaps_on_other_axes(previous.bounds, current.bounds, sweep_axis) {
                    continue;
                }
                slab_coarse = slab_coarse
                    .checked_add(1)
                    .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;

                let overlap_start_ms = previous.active_start_ms.max(current.active_start_ms);
                let overlap_end_ms = previous.active_end_ms.min(current.active_end_ms);
                if overlap_start_ms >= overlap_end_ms {
                    continue;
                }
                slab_time_coherent = slab_time_coherent
                    .checked_add(1)
                    .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;

                // Rebuild both bounds on their actual shared active interval.
                // This removes false candidates caused by disjoint portions of
                // a single coarse time slab.
                let previous_common = swept_bounds(
                    &bodies[previous.body_index].motion,
                    overlap_start_ms,
                    overlap_end_ms,
                )?;
                let current_common = swept_bounds(
                    &bodies[current.body_index].motion,
                    overlap_start_ms,
                    overlap_end_ms,
                )?;
                if !previous_common.overlaps(current_common) {
                    continue;
                }

                slab_emitted = slab_emitted
                    .checked_add(1)
                    .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;
                let previous_id = &bodies[previous.body_index].body_id;
                let current_id = &bodies[current.body_index].body_id;
                let (first_id, second_id) = if previous_id < current_id {
                    (previous_id.clone(), current_id.clone())
                } else {
                    (current_id.clone(), previous_id.clone())
                };
                pair_slabs
                    .entry((first_id, second_id))
                    .or_default()
                    .push(CandidateSlab {
                        slab_index,
                        overlap_start_ms,
                        overlap_end_ms,
                    });
            }

            active.push(current_index);
        }

        total_coarse_spatial_pairs = total_coarse_spatial_pairs
            .checked_add(slab_coarse)
            .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;
        total_time_coherent_pairs = total_time_coherent_pairs
            .checked_add(slab_time_coherent)
            .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;
        total_emitted_pair_candidates = total_emitted_pair_candidates
            .checked_add(slab_emitted)
            .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;

        slab_metrics.push(SlabMetrics {
            slab_index,
            slab_start_ms,
            slab_end_ms,
            active_body_count: entries.len() as u32,
            sweep_axis,
            coarse_spatial_pairs: slab_coarse,
            time_coherent_pairs: slab_time_coherent,
            emitted_pair_candidates: slab_emitted,
        });
    }

    let span_by_id: BTreeMap<&str, &CapsuleMotionSpan> = bodies
        .iter()
        .map(|body| (body.body_id.as_str(), &body.motion))
        .collect();
    let mut pairs = Vec::with_capacity(pair_slabs.len());
    for ((first_body_id, second_body_id), mut candidate_slabs) in pair_slabs {
        candidate_slabs.sort_by_key(|candidate| candidate.slab_index);
        let first_span = span_by_id
            .get(first_body_id.as_str())
            .ok_or(MultiBodyBroadphaseError::BodyIdLookupFailure)?;
        let second_span = span_by_id
            .get(second_body_id.as_str())
            .ok_or(MultiBodyBroadphaseError::BodyIdLookupFailure)?;
        let active_overlap_start_ms = first_span.start_ms.max(second_span.start_ms);
        let active_overlap_end_ms = first_span.end_ms.min(second_span.end_ms);
        if active_overlap_start_ms >= active_overlap_end_ms {
            return Err(MultiBodyBroadphaseError::TimeArithmeticOverflow);
        }
        let active_duration_ms = active_overlap_end_ms - active_overlap_start_ms;
        let candidate_duration_u128 = candidate_slabs
            .iter()
            .try_fold(0_u128, |total, candidate| {
                total.checked_add(u128::from(
                    candidate.overlap_end_ms - candidate.overlap_start_ms,
                ))
            })
            .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;
        let candidate_duration_ms = u64::try_from(candidate_duration_u128)
            .map_err(|_| MultiBodyBroadphaseError::TimeArithmeticOverflow)?;
        if candidate_duration_ms > active_duration_ms {
            return Err(MultiBodyBroadphaseError::TimeArithmeticOverflow);
        }
        pairs.push(TimedCapsulePairCandidate {
            first_body_id,
            second_body_id,
            active_overlap_start_ms,
            active_overlap_end_ms,
            active_duration_ms,
            candidate_duration_ms,
            candidate_time_fraction: candidate_duration_ms as f64 / active_duration_ms as f64,
            candidate_slabs,
        });
    }

    Ok(MultiBodyBroadphaseReport {
        horizon_start_ms,
        horizon_end_ms,
        subdivision_count,
        body_count,
        active_body_slab_cells,
        total_coarse_spatial_pairs,
        total_time_coherent_pairs,
        total_emitted_pair_candidates,
        pairs,
        slabs: slab_metrics,
    })
}


/// Result of refining a conservative coarse candidate set without rebuilding
/// the full-body SAP for every temporal slab.
#[derive(Clone, Debug, PartialEq)]
pub struct CandidateRefinementReport {
    pub coarse_candidate_pairs: u64,
    pub refinement_subdivision_count: u32,
    pub tested_pair_slabs: u64,
    pub pairs: Vec<TimedCapsulePairCandidate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HierarchicalMultiBodyBroadphaseReport {
    pub coarse: MultiBodyBroadphaseReport,
    pub refinement: CandidateRefinementReport,
}

/// Build a one-slab conservative coarse SAP, then refine only its candidate
/// pairs into the requested dyadic temporal subdivision.
pub fn time_swept_capsule_broadphase_hierarchical(
    bodies: &[TimedCapsuleBody],
    refinement_subdivisions: u32,
) -> Result<HierarchicalMultiBodyBroadphaseReport, MultiBodyBroadphaseError> {
    let coarse = time_swept_capsule_broadphase(bodies, 1)?;
    let refinement =
        refine_time_swept_capsule_candidates(bodies, &coarse, refinement_subdivisions)?;
    Ok(HierarchicalMultiBodyBroadphaseReport { coarse, refinement })
}

/// Refine a one-slab conservative candidate report into smaller common-time
/// intervals. Every pair omitted by the coarse report remains safely omitted
/// only under the same numeric swept-AABB assumptions as the coarse SAP.
///
/// This computes time-slab bounds only for coarse candidate pairs. It does not
/// rebuild per-body sweep-and-prune for every slab, making the refinement cost
/// proportional to coarse candidates × slab count rather than bodies × slabs
/// plus a full SAP pass for each slab.
///
/// The caller must retain the coarse report and its claim ceiling. This method
/// is a candidate optimization, not a collision verdict or formal certificate.
fn refine_time_swept_capsule_candidates(
    bodies: &[TimedCapsuleBody],
    coarse: &MultiBodyBroadphaseReport,
    subdivision_count: u32,
) -> Result<CandidateRefinementReport, MultiBodyBroadphaseError> {
    if subdivision_count == 0
        || !subdivision_count.is_power_of_two()
        || subdivision_count > MAX_TIME_SLABS
    {
        return Err(MultiBodyBroadphaseError::InvalidSubdivisionCount);
    }
    if bodies.is_empty() {
        return Err(MultiBodyBroadphaseError::NoBodies);
    }
    if coarse.subdivision_count != 1
        || coarse.body_count as usize != bodies.len()
        || coarse.horizon_start_ms >= coarse.horizon_end_ms
    {
        return Err(MultiBodyBroadphaseError::BodyIdLookupFailure);
    }

    let mut span_by_id = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let first = &bodies[0].motion;
    for body in bodies {
        if body.body_id.trim().is_empty() {
            return Err(MultiBodyBroadphaseError::EmptyBodyId);
        }
        if !ids.insert(body.body_id.as_str()) {
            return Err(MultiBodyBroadphaseError::DuplicateBodyId);
        }
        validate_capsule_motion_span(&body.motion)?;
        if body.motion.site_epoch != first.site_epoch {
            return Err(MultiBodyBroadphaseError::SiteEpochMismatch);
        }
        if body.motion.frame_id != first.frame_id {
            return Err(MultiBodyBroadphaseError::FrameMismatch);
        }
        if body.motion.clock_domain_id != first.clock_domain_id {
            return Err(MultiBodyBroadphaseError::ClockDomainMismatch);
        }
        span_by_id.insert(body.body_id.as_str(), &body.motion);
    }

    let horizon_duration = coarse
        .horizon_end_ms
        .checked_sub(coarse.horizon_start_ms)
        .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;
    let mut output_pairs = Vec::with_capacity(coarse.pairs.len());
    let mut tested_pair_slabs = 0_u64;

    for pair in &coarse.pairs {
        let first_span = span_by_id
            .get(pair.first_body_id.as_str())
            .ok_or(MultiBodyBroadphaseError::BodyIdLookupFailure)?;
        let second_span = span_by_id
            .get(pair.second_body_id.as_str())
            .ok_or(MultiBodyBroadphaseError::BodyIdLookupFailure)?;
        let active_overlap_start_ms = first_span.start_ms.max(second_span.start_ms);
        let active_overlap_end_ms = first_span.end_ms.min(second_span.end_ms);
        if active_overlap_start_ms >= active_overlap_end_ms {
            return Err(MultiBodyBroadphaseError::TimeArithmeticOverflow);
        }
        let active_duration_ms = active_overlap_end_ms - active_overlap_start_ms;
        let mut candidate_slabs = Vec::new();

        for slab_index in 0..subdivision_count {
            let slab_start_ms = slab_boundary(
                coarse.horizon_start_ms,
                horizon_duration,
                slab_index,
                subdivision_count,
            )?;
            let slab_end_ms = slab_boundary(
                coarse.horizon_start_ms,
                horizon_duration,
                slab_index + 1,
                subdivision_count,
            )?;
            let overlap_start_ms = active_overlap_start_ms.max(slab_start_ms);
            let overlap_end_ms = active_overlap_end_ms.min(slab_end_ms);
            if overlap_start_ms >= overlap_end_ms {
                continue;
            }
            tested_pair_slabs = tested_pair_slabs
                .checked_add(1)
                .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;

            let first_bounds = swept_bounds(first_span, overlap_start_ms, overlap_end_ms)?;
            let second_bounds = swept_bounds(second_span, overlap_start_ms, overlap_end_ms)?;
            if first_bounds.overlaps(second_bounds) {
                candidate_slabs.push(CandidateSlab {
                    slab_index,
                    overlap_start_ms,
                    overlap_end_ms,
                });
            }
        }

        if candidate_slabs.is_empty() {
            continue;
        }
        let candidate_duration_u128 = candidate_slabs
            .iter()
            .try_fold(0_u128, |total, candidate| {
                total.checked_add(u128::from(
                    candidate.overlap_end_ms - candidate.overlap_start_ms,
                ))
            })
            .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)?;
        let candidate_duration_ms = u64::try_from(candidate_duration_u128)
            .map_err(|_| MultiBodyBroadphaseError::TimeArithmeticOverflow)?;
        if candidate_duration_ms > active_duration_ms {
            return Err(MultiBodyBroadphaseError::TimeArithmeticOverflow);
        }
        output_pairs.push(TimedCapsulePairCandidate {
            first_body_id: pair.first_body_id.clone(),
            second_body_id: pair.second_body_id.clone(),
            active_overlap_start_ms,
            active_overlap_end_ms,
            active_duration_ms,
            candidate_duration_ms,
            candidate_time_fraction: candidate_duration_ms as f64 / active_duration_ms as f64,
            candidate_slabs,
        });
    }

    Ok(CandidateRefinementReport {
        coarse_candidate_pairs: coarse.pairs.len() as u64,
        refinement_subdivision_count: subdivision_count,
        tested_pair_slabs,
        pairs: output_pairs,
    })
}

#[derive(Clone, Copy, Debug)]
struct SlabEntry {
    body_index: usize,
    active_start_ms: u64,
    active_end_ms: u64,
    bounds: CapsuleAabb3,
}

fn slab_boundary(
    horizon_start_ms: u64,
    horizon_duration_ms: u64,
    boundary_index: u32,
    subdivision_count: u32,
) -> Result<u64, MultiBodyBroadphaseError> {
    let offset = (u128::from(horizon_duration_ms) * u128::from(boundary_index))
        / u128::from(subdivision_count);
    horizon_start_ms
        .checked_add(
            u64::try_from(offset).map_err(|_| MultiBodyBroadphaseError::TimeArithmeticOverflow)?,
        )
        .ok_or(MultiBodyBroadphaseError::TimeArithmeticOverflow)
}

fn swept_bounds(
    span: &CapsuleMotionSpan,
    start_ms: u64,
    end_ms: u64,
) -> Result<CapsuleAabb3, MultiBodyBroadphaseError> {
    let (start_capsule, translation) =
        rebase_span_to_time(span, start_ms, end_ms).map_err(MultiBodyBroadphaseError::Timed)?;
    let end_capsule = Capsule::new(
        start_capsule.start.add(translation),
        start_capsule.end.add(translation),
        start_capsule.radius_mm,
    )
    .map_err(map_capsule_error)?;
    let start_bounds = start_capsule.aabb();
    let end_bounds = end_capsule.aabb();
    let minimum = Vec3::new(
        start_bounds.min.x.min(end_bounds.min.x),
        start_bounds.min.y.min(end_bounds.min.y),
        start_bounds.min.z.min(end_bounds.min.z),
    );
    let maximum = Vec3::new(
        start_bounds.max.x.max(end_bounds.max.x),
        start_bounds.max.y.max(end_bounds.max.y),
        start_bounds.max.z.max(end_bounds.max.z),
    );
    let bounds = CapsuleAabb3 {
        min: Vec3::new(
            outward_down(minimum.x),
            outward_down(minimum.y),
            outward_down(minimum.z),
        ),
        max: Vec3::new(
            outward_up(maximum.x),
            outward_up(maximum.y),
            outward_up(maximum.z),
        ),
    };
    if !is_finite(bounds.min) || !is_finite(bounds.max) {
        return Err(MultiBodyBroadphaseError::GeometryOverflow);
    }
    Ok(bounds)
}

fn map_capsule_error(error: CapsuleError) -> MultiBodyBroadphaseError {
    match error {
        CapsuleError::InvalidRadius => MultiBodyBroadphaseError::Timed(TimedCcdError::Geometry(
            crate::TranslationCcdError::InvalidRadius,
        )),
        CapsuleError::NonFiniteCoordinate => MultiBodyBroadphaseError::GeometryOverflow,
    }
}

fn outward_down(mut value: f64) -> f64 {
    for _ in 0..OUTWARD_ULP_STEPS {
        value = value.next_down();
    }
    value
}

fn outward_up(mut value: f64) -> f64 {
    for _ in 0..OUTWARD_ULP_STEPS {
        value = value.next_up();
    }
    value
}

fn is_finite(point: Vec3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

fn component(point: Vec3, axis: usize) -> f64 {
    match axis {
        0 => point.x,
        1 => point.y,
        2 => point.z,
        _ => unreachable!("axis index is in [0, 2]"),
    }
}

fn select_sweep_axis(entries: &[SlabEntry]) -> SweepAxis {
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    for entry in entries {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(component(entry.bounds.min, axis));
            maximum[axis] = maximum[axis].max(component(entry.bounds.max, axis));
        }
    }
    let spreads = [
        maximum[0] - minimum[0],
        maximum[1] - minimum[1],
        maximum[2] - minimum[2],
    ];
    let mut best = 0;
    for axis in 1..3 {
        if spreads[axis] > spreads[best] {
            best = axis;
        }
    }
    match best {
        0 => SweepAxis::X,
        1 => SweepAxis::Y,
        _ => SweepAxis::Z,
    }
}

fn overlaps_on_other_axes(left: CapsuleAabb3, right: CapsuleAabb3, sweep_axis: SweepAxis) -> bool {
    match sweep_axis {
        SweepAxis::X => overlap_axis(left, right, 1) && overlap_axis(left, right, 2),
        SweepAxis::Y => overlap_axis(left, right, 0) && overlap_axis(left, right, 2),
        SweepAxis::Z => overlap_axis(left, right, 0) && overlap_axis(left, right, 1),
    }
}

fn overlap_axis(left: CapsuleAabb3, right: CapsuleAabb3, axis: usize) -> bool {
    component(left.min, axis) <= component(right.max, axis)
        && component(left.max, axis) >= component(right.min, axis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TimedCcdError, timed_co_translating_capsule_vs_capsule};

    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn capsule(x: f64, y: f64, radius_mm: f64) -> Capsule {
        Capsule::new(p(x, y - 1.0, 0.0), p(x, y + 1.0, 0.0), radius_mm).unwrap()
    }

    fn span(x: f64, y: f64, delta_x: f64, start_ms: u64, end_ms: u64) -> CapsuleMotionSpan {
        CapsuleMotionSpan {
            capsule_at_start: capsule(x, y, 0.5),
            translation: p(delta_x, 0.0, 0.0),
            start_ms,
            end_ms,
            site_epoch: "site-epoch-7".into(),
            frame_id: "site-frame".into(),
            clock_domain_id: "site-clock".into(),
            clock_uncertainty_ms: 0,
        }
    }

    fn body(id: &str, motion: CapsuleMotionSpan) -> TimedCapsuleBody {
        TimedCapsuleBody {
            body_id: id.into(),
            motion,
        }
    }

    fn pair_ids(report: &MultiBodyBroadphaseReport) -> Vec<(String, String)> {
        report
            .pairs
            .iter()
            .map(|pair| (pair.first_body_id.clone(), pair.second_body_id.clone()))
            .collect()
    }

    #[test]
    fn emits_head_on_continuous_contact_candidate_with_clear_endpoints() {
        let report = time_swept_capsule_broadphase(
            &[
                body("robot-a", span(-10.0, 0.0, 20.0, 0, 100)),
                body("robot-b", span(10.0, 0.0, -20.0, 0, 100)),
                body("static-remote", span(500.0, 500.0, 0.0, 0, 100)),
            ],
            8,
        )
        .unwrap();

        assert!(pair_ids(&report).contains(&("robot-a".into(), "robot-b".into())));
        let narrow = timed_co_translating_capsule_vs_capsule(
            &span(-10.0, 0.0, 20.0, 0, 100),
            &span(10.0, 0.0, -20.0, 0, 100),
        )
        .unwrap();
        assert!(narrow.is_contact(0.5, 0.5));
    }

    #[test]
    fn prunes_spatial_overlap_from_nonconcurrent_motion_windows() {
        let report = time_swept_capsule_broadphase(
            &[
                body("early", span(0.0, 0.0, 10.0, 0, 40)),
                body("late", span(10.0, 0.0, -10.0, 60, 100)),
            ],
            1,
        )
        .unwrap();
        assert!(report.pairs.is_empty());
        assert_eq!(report.total_coarse_spatial_pairs, 1);
        assert_eq!(report.total_time_coherent_pairs, 0);
    }

    #[test]
    fn same_slab_partial_windows_are_rechecked_on_the_common_interval() {
        let report = time_swept_capsule_broadphase(
            &[
                body("a", span(0.0, 0.0, 40.0, 0, 60)),
                body("b", span(30.0, 0.0, -40.0, 40, 100)),
            ],
            1,
        )
        .unwrap();

        // Their coarse per-span swept boxes overlap, and they are co-active
        // on [40,60). Rebased common-interval boxes also overlap.
        assert_eq!(report.pairs.len(), 1);
        assert_eq!(report.pairs[0].candidate_slabs[0].overlap_start_ms, 40);
        assert_eq!(report.pairs[0].candidate_slabs[0].overlap_end_ms, 60);
        assert_eq!(report.pairs[0].active_duration_ms, 20);
    }

    #[test]
    fn time_slicing_reduces_candidate_duration_for_head_on_crossing() {
        let bodies = [
            body("a", span(-10.0, 0.0, 20.0, 0, 100)),
            body("b", span(10.0, 0.0, -20.0, 0, 100)),
        ];
        let coarse = time_swept_capsule_broadphase(&bodies, 1).unwrap();
        let fine = time_swept_capsule_broadphase(&bodies, 16).unwrap();
        assert_eq!(coarse.pairs.len(), 1);
        assert_eq!(fine.pairs.len(), 1);
        assert!(fine.pairs[0].candidate_duration_ms <= coarse.pairs[0].candidate_duration_ms);
        assert!(fine.pairs[0].candidate_time_fraction <= coarse.pairs[0].candidate_time_fraction);
    }

    #[test]
    fn output_is_deterministic_and_independent_of_input_order() {
        let first = vec![
            body("c", span(0.0, 5.0, 5.0, 0, 100)),
            body("a", span(-10.0, 0.0, 20.0, 0, 100)),
            body("b", span(10.0, 0.0, -20.0, 0, 100)),
        ];
        let second = vec![first[1].clone(), first[2].clone(), first[0].clone()];
        assert_eq!(
            time_swept_capsule_broadphase(&first, 8).unwrap(),
            time_swept_capsule_broadphase(&second, 8).unwrap()
        );
    }

    #[test]
    fn axis_choice_is_deterministic_and_non_x_axis_is_supported() {
        let report = time_swept_capsule_broadphase(
            &[
                body(
                    "a",
                    CapsuleMotionSpan {
                        capsule_at_start: Capsule::new(p(0.0, -1.0, 0.0), p(0.0, 1.0, 0.0), 0.5)
                            .unwrap(),
                        translation: p(0.0, 100.0, 0.0),
                        start_ms: 0,
                        end_ms: 100,
                        site_epoch: "site-epoch-7".into(),
                        frame_id: "site-frame".into(),
                        clock_domain_id: "site-clock".into(),
                        clock_uncertainty_ms: 0,
                    },
                ),
                body(
                    "b",
                    CapsuleMotionSpan {
                        capsule_at_start: Capsule::new(p(0.0, 0.0, -1.0), p(0.0, 0.0, 1.0), 0.5)
                            .unwrap(),
                        translation: p(0.0, -100.0, 0.0),
                        start_ms: 0,
                        end_ms: 100,
                        site_epoch: "site-epoch-7".into(),
                        frame_id: "site-frame".into(),
                        clock_domain_id: "site-clock".into(),
                        clock_uncertainty_ms: 0,
                    },
                ),
            ],
            4,
        )
        .unwrap();
        assert!(
            report
                .slabs
                .iter()
                .all(|slab| slab.sweep_axis != SweepAxis::X)
        );
    }

    #[test]
    fn candidate_pairs_match_independent_brute_force_reference() {
        let mut seed = 0x5eed_901d_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 401) as f64 - 200.0
        };
        let bodies: Vec<_> = (0..80)
            .map(|index| {
                let id = format!("body-{index:03}");
                body(
                    &id,
                    CapsuleMotionSpan {
                        capsule_at_start: Capsule::new(
                            p(next(), next(), next()),
                            p(next(), next(), next()),
                            0.5,
                        )
                        .unwrap(),
                        translation: p(next(), next(), next()),
                        start_ms: 0,
                        end_ms: 1_000,
                        site_epoch: "site-epoch-7".into(),
                        frame_id: "site-frame".into(),
                        clock_domain_id: "site-clock".into(),
                        clock_uncertainty_ms: 0,
                    },
                )
            })
            .collect();
        let sap = time_swept_capsule_broadphase(&bodies, 8).unwrap();
        let reference = brute_force_candidates(&bodies, 8);
        assert_eq!(pair_ids(&sap), reference);
    }

    fn brute_force_candidates(
        bodies: &[TimedCapsuleBody],
        subdivision_count: u32,
    ) -> Vec<(String, String)> {
        let horizon_start = bodies
            .iter()
            .map(|body| body.motion.start_ms)
            .min()
            .unwrap();
        let horizon_end = bodies.iter().map(|body| body.motion.end_ms).max().unwrap();
        let duration = horizon_end - horizon_start;
        let mut pairs = BTreeSet::new();
        for left in 0..bodies.len() {
            for right in (left + 1)..bodies.len() {
                for slab in 0..subdivision_count {
                    let slab_start =
                        slab_boundary(horizon_start, duration, slab, subdivision_count).unwrap();
                    let slab_end =
                        slab_boundary(horizon_start, duration, slab + 1, subdivision_count)
                            .unwrap();
                    if slab_start >= slab_end {
                        continue;
                    }
                    let overlap_start = slab_start
                        .max(bodies[left].motion.start_ms)
                        .max(bodies[right].motion.start_ms);
                    let overlap_end = slab_end
                        .min(bodies[left].motion.end_ms)
                        .min(bodies[right].motion.end_ms);
                    if overlap_start >= overlap_end {
                        continue;
                    }
                    let left_bounds =
                        swept_bounds(&bodies[left].motion, overlap_start, overlap_end).unwrap();
                    let right_bounds =
                        swept_bounds(&bodies[right].motion, overlap_start, overlap_end).unwrap();
                    if left_bounds.overlaps(right_bounds) {
                        let a = &bodies[left].body_id;
                        let b = &bodies[right].body_id;
                        pairs.insert(if a < b {
                            (a.clone(), b.clone())
                        } else {
                            (b.clone(), a.clone())
                        });
                    }
                }
            }
        }
        pairs.into_iter().collect()
    }

    #[test]
    fn rejects_duplicate_ids_incompatible_context_and_uncertain_clock() {
        let motion = span(0.0, 0.0, 0.0, 0, 100);
        assert_eq!(
            time_swept_capsule_broadphase(
                &[body("same", motion.clone()), body("same", motion.clone())],
                1
            ),
            Err(MultiBodyBroadphaseError::DuplicateBodyId)
        );

        let mut wrong_frame = motion.clone();
        wrong_frame.frame_id = "other-frame".into();
        assert_eq!(
            time_swept_capsule_broadphase(&[body("a", motion.clone()), body("b", wrong_frame)], 1),
            Err(MultiBodyBroadphaseError::FrameMismatch)
        );

        let mut uncertain = motion;
        uncertain.clock_uncertainty_ms = 1;
        assert_eq!(
            time_swept_capsule_broadphase(&[body("a", uncertain)], 1),
            Err(MultiBodyBroadphaseError::Timed(
                TimedCcdError::ClockUncertaintyUnmodeled
            ))
        );
    }

    #[test]
    fn rejects_invalid_subdivision_and_excessive_cells() {
        let motion = span(0.0, 0.0, 0.0, 0, 100);
        assert_eq!(
            time_swept_capsule_broadphase(&[body("a", motion.clone())], 3),
            Err(MultiBodyBroadphaseError::InvalidSubdivisionCount)
        );
        let many: Vec<_> = (0..1_025)
            .map(|index| body(&format!("body-{index}"), motion.clone()))
            .collect();
        assert_eq!(
            time_swept_capsule_broadphase(&many, MAX_TIME_SLABS),
            Err(MultiBodyBroadphaseError::TooManyBodySlabCells)
        );
    }

    #[test]
    fn exact_contact_is_never_pruned_by_sap_or_time_slabs() {
        let mut seed = 0xadc0_ffee_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 401) as f64 - 200.0
        };
        for _ in 0..40 {
            let moving = body(
                "moving",
                CapsuleMotionSpan {
                    capsule_at_start: Capsule::new(
                        p(next(), next(), next()),
                        p(next(), next(), next()),
                        0.0,
                    )
                    .unwrap(),
                    translation: p(next(), next(), next()),
                    start_ms: 0,
                    end_ms: 1_000,
                    site_epoch: "site-epoch-7".into(),
                    frame_id: "site-frame".into(),
                    clock_domain_id: "site-clock".into(),
                    clock_uncertainty_ms: 0,
                },
            );
            let mut obstacle = moving.clone();
            obstacle.body_id = "obstacle".into();
            obstacle.motion.translation = moving.motion.translation;
            // Exact same world trajectory guarantees contact at every common time.
            let report = time_swept_capsule_broadphase(&[moving, obstacle], 16).unwrap();
            assert!(pair_ids(&report).contains(&("moving".into(), "obstacle".into())));
        }
    }

    #[test]
    fn hierarchical_refinement_matches_full_multislab_sap_candidates() {
        for bodies in [
            vec![
                body("a", span(-10.0, 0.0, 20.0, 0, 100)),
                body("b", span(10.0, 0.0, -20.0, 0, 100)),
                body("c", span(500.0, 0.0, 0.0, 0, 100)),
            ],
            vec![
                body("a", span(0.0, 0.0, 10.0, 0, 100)),
                body("b", span(2.0, 0.0, -10.0, 0, 100)),
                body("c", span(4.0, 0.0, 10.0, 0, 100)),
                body("d", span(6.0, 0.0, -10.0, 0, 100)),
            ],
        ] {
            for subdivisions in [1, 2, 4, 8, 16] {
                let hierarchical =
                    time_swept_capsule_broadphase_hierarchical(&bodies, subdivisions).unwrap();
                let direct =
                    time_swept_capsule_broadphase(&bodies, subdivisions).unwrap();
                assert_eq!(
                    hierarchical.refinement.pairs,
                    direct.pairs,
                    "hierarchical refinement diverged at {subdivisions} slabs"
                );
            }
        }
    }


    #[test]
    fn generated_hierarchical_refinement_matches_direct_sap_for_all_slab_counts() {
        let mut bodies = Vec::new();
        for index in 0..64_usize {
            let x = ((index * 37) % 113) as f64 * 2.5;
            let y = ((index * 19) % 71) as f64 * 1.5;
            let delta_x = ((index * 23) % 41) as f64 - 20.0;
            let start_ms = (index % 4) as u64 * 25;
            let end_ms = 1_000 + (index % 4) as u64 * 25;
            let id = format!("body-{index:03}");
            bodies.push(body(
                &id,
                span(x, y, delta_x, start_ms, end_ms),
            ));
        }

        for subdivisions in [1, 2, 4, 8, 16] {
            let hierarchical =
                time_swept_capsule_broadphase_hierarchical(&bodies, subdivisions).unwrap();
            let direct = time_swept_capsule_broadphase(&bodies, subdivisions).unwrap();
            assert_eq!(
                hierarchical.refinement.pairs,
                direct.pairs,
                "generated pair/slab records diverged at {subdivisions} subdivisions"
            );
            assert_eq!(
                hierarchical.refinement.coarse_candidate_pairs,
                hierarchical.coarse.pairs.len() as u64
            );
            assert!(
                hierarchical.refinement.tested_pair_slabs
                    >= hierarchical.refinement.pairs.len() as u64
            );
        }
    }

}
