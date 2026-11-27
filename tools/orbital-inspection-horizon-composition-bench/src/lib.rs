#![deny(unsafe_code)]

use orbital_inspection_chord_tube_bench::{CaseInput, Disposition as SegmentDisposition, verify};
use std::collections::BTreeSet;

pub const RECEIPT_SCHEMA: &str = "luminous.orbital-inspection-horizon-composition.receipt.v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryKind {
    HorizonStart,
    SmoothJoin,
    ImpulsiveVelocityChange,
    UnsupportedDiscontinuity,
}

impl BoundaryKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "HORIZON_START" => Some(Self::HorizonStart),
            "SMOOTH_JOIN" => Some(Self::SmoothJoin),
            "IMPULSIVE_VELOCITY_CHANGE" => Some(Self::ImpulsiveVelocityChange),
            "UNSUPPORTED_DISCONTINUITY" => Some(Self::UnsupportedDiscontinuity),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HorizonStart => "HORIZON_START",
            Self::SmoothJoin => "SMOOTH_JOIN",
            Self::ImpulsiveVelocityChange => "IMPULSIVE_VELOCITY_CHANGE",
            Self::UnsupportedDiscontinuity => "UNSUPPORTED_DISCONTINUITY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizonDisposition {
    CompleteConditionalClear,
    NotProven,
    InvalidInput,
    IncompleteCoverage,
    ScopeMismatch,
    BoundaryMismatch,
    SeamIncoherent,
    DuplicateSegment,
    Overflow,
}

impl HorizonDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CompleteConditionalClear => "COMPLETE_CONDITIONAL_CLEAR",
            Self::NotProven => "NOT_PROVEN",
            Self::InvalidInput => "INVALID_INPUT",
            Self::IncompleteCoverage => "INCOMPLETE_COVERAGE",
            Self::ScopeMismatch => "SCOPE_MISMATCH",
            Self::BoundaryMismatch => "BOUNDARY_MISMATCH",
            Self::SeamIncoherent => "SEAM_INCOHERENT",
            Self::DuplicateSegment => "DUPLICATE_SEGMENT",
            Self::Overflow => "OVERFLOW",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorizonScope {
    pub action_id: String,
    pub target_id: String,
    pub keepout_profile_id: String,
    pub keepout_geometry_revision: String,
    pub transform_digest: String,
    pub frame_id: String,
    pub frame_revision: String,
    pub time_basis_id: String,
    pub dynamics_profile_revision: String,
}

impl HorizonScope {
    fn is_valid(&self) -> bool {
        [
            self.action_id.as_str(),
            self.target_id.as_str(),
            self.keepout_profile_id.as_str(),
            self.keepout_geometry_revision.as_str(),
            self.transform_digest.as_str(),
            self.frame_id.as_str(),
            self.frame_revision.as_str(),
            self.time_basis_id.as_str(),
            self.dynamics_profile_revision.as_str(),
        ]
        .iter()
        .all(|value| !value.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub segment_id: String,
    pub scope: HorizonScope,
    pub start_tai_ns: u64,
    pub end_tai_ns: u64,
    pub start_event_id: String,
    pub end_event_id: String,
    pub start_boundary: BoundaryKind,
    pub motion: CaseInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Horizon {
    pub horizon_id: String,
    pub scope: HorizonScope,
    pub start_tai_ns: u64,
    pub end_tai_ns: u64,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentResult {
    pub segment_id: String,
    pub disposition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorizonResult {
    pub horizon_id: String,
    pub disposition: HorizonDisposition,
    pub complete_time_coverage: bool,
    pub segment_count: usize,
    pub seam_count: usize,
    pub segment_results: Vec<SegmentResult>,
}

fn seam_balls_overlap(left: &Segment, right: &Segment) -> Option<bool> {
    let dx = i128::from(left.motion.end[0]).checked_sub(i128::from(right.motion.start[0]))?;
    let dy = i128::from(left.motion.end[1]).checked_sub(i128::from(right.motion.start[1]))?;
    let dz = i128::from(left.motion.end[2]).checked_sub(i128::from(right.motion.start[2]))?;

    let dx2 = dx.checked_mul(dx)?;
    let dy2 = dy.checked_mul(dy)?;
    let dz2 = dz.checked_mul(dz)?;
    let distance_squared = dx2.checked_add(dy2)?.checked_add(dz2)?;

    let combined_error = i128::from(left.motion.endpoint_error_bound_scaled)
        .checked_add(i128::from(right.motion.endpoint_error_bound_scaled))?;
    let combined_error_squared = combined_error.checked_mul(combined_error)?;
    Some(distance_squared <= combined_error_squared)
}

fn segment_results(segments: &[Segment]) -> Vec<SegmentResult> {
    segments
        .iter()
        .map(|segment| SegmentResult {
            segment_id: segment.segment_id.clone(),
            disposition: verify(&segment.motion).disposition.as_str().to_owned(),
        })
        .collect()
}

fn result(
    horizon: &Horizon,
    disposition: HorizonDisposition,
    complete_time_coverage: bool,
    seam_count: usize,
) -> HorizonResult {
    HorizonResult {
        horizon_id: horizon.horizon_id.clone(),
        disposition,
        complete_time_coverage,
        segment_count: horizon.segments.len(),
        seam_count,
        segment_results: segment_results(&horizon.segments),
    }
}

/// Composes per-segment conditional chord-tube results into a complete
/// piecewise-smooth horizon disposition.
///
/// Smooth joins and impulsive velocity changes are both treated as
/// position-continuous boundaries. Each segment is checked independently;
/// no derivative/curvature bound is carried across a seam. Endpoint-ball
/// overlap is necessary for continuity but is not proof that the physical
/// seam state is correct. Source authenticity, bound validity and dynamics
/// correctness remain upstream obligations.
pub fn evaluate(horizon: &Horizon) -> HorizonResult {
    if horizon.horizon_id.trim().is_empty()
        || !horizon.scope.is_valid()
        || horizon.start_tai_ns >= horizon.end_tai_ns
        || horizon.segments.is_empty()
    {
        return result(
            horizon,
            if horizon.segments.is_empty() {
                HorizonDisposition::IncompleteCoverage
            } else {
                HorizonDisposition::InvalidInput
            },
            false,
            0,
        );
    }

    let mut ids = BTreeSet::new();
    let mut seam_events = BTreeSet::new();

    for (index, segment) in horizon.segments.iter().enumerate() {
        if segment.segment_id.trim().is_empty()
            || !ids.insert(segment.segment_id.as_str())
            || segment.start_event_id.trim().is_empty()
            || segment.end_event_id.trim().is_empty()
            || segment.motion.case_id != segment.segment_id
            || segment.start_tai_ns >= segment.end_tai_ns
            || segment.motion.duration_ns != segment.end_tai_ns - segment.start_tai_ns
            || segment.motion.acceleration_bound_scaled_per_s2 < 0
            || segment.motion.endpoint_error_bound_scaled < 0
        {
            return result(horizon, HorizonDisposition::InvalidInput, false, index);
        }

        if !segment.scope.is_valid() {
            return result(horizon, HorizonDisposition::InvalidInput, false, index);
        }
        if segment.scope != horizon.scope {
            return result(horizon, HorizonDisposition::ScopeMismatch, false, index);
        }

        if index == 0 {
            if segment.start_boundary != BoundaryKind::HorizonStart
                || segment.start_tai_ns != horizon.start_tai_ns
            {
                return result(horizon, HorizonDisposition::IncompleteCoverage, false, 0);
            }
        } else if !matches!(
            segment.start_boundary,
            BoundaryKind::SmoothJoin | BoundaryKind::ImpulsiveVelocityChange
        ) {
            return result(horizon, HorizonDisposition::BoundaryMismatch, false, index);
        }
    }

    if horizon.segments.last().map(|segment| segment.end_tai_ns) != Some(horizon.end_tai_ns) {
        return result(
            horizon,
            HorizonDisposition::IncompleteCoverage,
            false,
            horizon.segments.len().saturating_sub(1),
        );
    }

    for index in 1..horizon.segments.len() {
        let left = &horizon.segments[index - 1];
        let right = &horizon.segments[index];

        if left.end_tai_ns != right.start_tai_ns {
            return result(
                horizon,
                HorizonDisposition::IncompleteCoverage,
                false,
                index - 1,
            );
        }

        if left.end_event_id != right.start_event_id
            || !seam_events.insert(left.end_event_id.as_str())
        {
            return result(horizon, HorizonDisposition::BoundaryMismatch, true, index);
        }

        match seam_balls_overlap(left, right) {
            Some(true) => {}
            Some(false) => {
                return result(horizon, HorizonDisposition::SeamIncoherent, true, index);
            }
            None => {
                return result(horizon, HorizonDisposition::Overflow, true, index);
            }
        }
    }

    for segment in &horizon.segments {
        match verify(&segment.motion).disposition {
            SegmentDisposition::ConditionalClear => {}
            SegmentDisposition::Overflow => {
                return result(
                    horizon,
                    HorizonDisposition::Overflow,
                    true,
                    horizon.segments.len().saturating_sub(1),
                );
            }
            SegmentDisposition::InvalidInput => {
                return result(
                    horizon,
                    HorizonDisposition::InvalidInput,
                    true,
                    horizon.segments.len().saturating_sub(1),
                );
            }
            SegmentDisposition::NotProven => {
                return result(
                    horizon,
                    HorizonDisposition::NotProven,
                    true,
                    horizon.segments.len().saturating_sub(1),
                );
            }
        }
    }

    result(
        horizon,
        HorizonDisposition::CompleteConditionalClear,
        true,
        horizon.segments.len().saturating_sub(1),
    )
}

#[cfg(test)]
mod tests {
    use super::{BoundaryKind, Horizon, HorizonDisposition, HorizonScope, Segment, evaluate};
    use orbital_inspection_chord_tube_bench::CaseInput;

    fn scope() -> HorizonScope {
        HorizonScope {
            action_id: "action-1".to_owned(),
            target_id: "target-1".to_owned(),
            keepout_profile_id: "keepout-1".to_owned(),
            keepout_geometry_revision: "geometry-r1".to_owned(),
            transform_digest: "transform-r1".to_owned(),
            frame_id: "LVLH".to_owned(),
            frame_revision: "frame-r1".to_owned(),
            time_basis_id: "TAI".to_owned(),
            dynamics_profile_revision: "eccentric-model-r1".to_owned(),
        }
    }

    struct SegmentSpec<'a> {
        id: &'a str,
        start: u64,
        end: u64,
        start_event: &'a str,
        end_event: &'a str,
        boundary: BoundaryKind,
        q_start: [i64; 3],
        q_end: [i64; 3],
        endpoint_error: i64,
    }

    fn segment(spec: SegmentSpec<'_>) -> Segment {
        Segment {
            segment_id: spec.id.to_owned(),
            scope: scope(),
            start_tai_ns: spec.start,
            end_tai_ns: spec.end,
            start_event_id: spec.start_event.to_owned(),
            end_event_id: spec.end_event.to_owned(),
            start_boundary: spec.boundary,
            motion: CaseInput {
                case_id: spec.id.to_owned(),
                start: spec.q_start,
                end: spec.q_end,
                acceleration_bound_scaled_per_s2: 0,
                duration_ns: spec.end - spec.start,
                endpoint_error_bound_scaled: spec.endpoint_error,
            },
        }
    }
    fn horizon(segments: Vec<Segment>) -> Horizon {
        Horizon {
            horizon_id: "horizon-1".to_owned(),
            scope: scope(),
            start_tai_ns: 0,
            end_tai_ns: 10,
            segments,
        }
    }

    #[test]
    fn one_segment_covers_the_complete_horizon() {
        let h = horizon(vec![segment(SegmentSpec { id: "s1", start: 0, end: 10, start_event: "begin", end_event: "end", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 })]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::CompleteConditionalClear
        );
    }

    #[test]
    fn two_segments_compose_across_a_smooth_join() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "join-1", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "join-1", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::CompleteConditionalClear
        );
    }

    #[test]
    fn impulsive_velocity_change_resets_the_segment_bound() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "burn-1", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "burn-1", end_event: "end", boundary: BoundaryKind::ImpulsiveVelocityChange, q_start: [2_000_000, 0, 0], q_end: [3_000_000, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::CompleteConditionalClear
        );
    }

    #[test]
    fn one_nanosecond_gap_is_incomplete() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 4, start_event: "begin", end_event: "gap-left", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "gap-right", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::IncompleteCoverage
        );
    }

    #[test]
    fn one_nanosecond_overlap_is_incomplete() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 6, start_event: "begin", end_event: "join", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "join", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::IncompleteCoverage
        );
    }

    #[test]
    fn first_and_last_horizon_coverage_are_required() {
        let missing_start = horizon(vec![segment(SegmentSpec { id: "s1", start: 1, end: 10, start_event: "begin", end_event: "end", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 })]);
        let missing_end = horizon(vec![segment(SegmentSpec { id: "s1", start: 0, end: 9, start_event: "begin", end_event: "end", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 })]);
        assert_eq!(
            evaluate(&missing_start).disposition,
            HorizonDisposition::IncompleteCoverage
        );
        assert_eq!(
            evaluate(&missing_end).disposition,
            HorizonDisposition::IncompleteCoverage
        );
    }

    #[test]
    fn seam_event_identity_must_match() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "left-event", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "right-event", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::BoundaryMismatch
        );
    }

    #[test]
    fn disjoint_endpoint_uncertainty_balls_reject_the_seam() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "join", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 10 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "join", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_021, 0, 0], q_end: [2_000_021, 0, 0], endpoint_error: 10 }),
        ]);
        assert_eq!(evaluate(&h).disposition, HorizonDisposition::SeamIncoherent);
    }

    #[test]
    fn touching_closed_uncertainty_balls_are_consistent() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "join", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 5 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "join", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_010, 0, 0], q_end: [2_000_010, 0, 0], endpoint_error: 5 }),
        ]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::CompleteConditionalClear
        );
    }

    #[test]
    fn scope_cannot_change_between_segments() {
        let mut second = segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "join", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 });
        second.scope.frame_revision = "frame-r2".to_owned();
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "join", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            second,
        ]);
        assert_eq!(evaluate(&h).disposition, HorizonDisposition::ScopeMismatch);
    }

    #[test]
    fn unsupported_discontinuity_is_not_a_supported_join() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "jump", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "jump", end_event: "end", boundary: BoundaryKind::UnsupportedDiscontinuity, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::BoundaryMismatch
        );
    }

    #[test]
    fn a_single_not_proven_segment_blocks_the_whole_horizon() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "join", boundary: BoundaryKind::HorizonStart, q_start: [-2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "join", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(evaluate(&h).disposition, HorizonDisposition::NotProven);
    }

    #[test]
    fn duplicate_segment_ids_are_rejected() {
        let first = segment(SegmentSpec { id: "same", start: 0, end: 5, start_event: "begin", end_event: "join", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 });
        let second = segment(SegmentSpec { id: "same", start: 5, end: 10, start_event: "join", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 });
        assert_eq!(
            evaluate(&horizon(vec![first, second])).disposition,
            HorizonDisposition::InvalidInput
        );
    }

    #[test]
    fn repeated_interior_event_identity_is_rejected() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 3, start_event: "begin", end_event: "same-event", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s2", start: 3, end: 7, start_event: "same-event", end_event: "same-event", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s3", start: 7, end: 10, start_event: "same-event", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::BoundaryMismatch
        );
    }

    #[test]
    fn duration_must_match_the_segment_time_interval() {
        let mut s = segment(SegmentSpec { id: "s1", start: 0, end: 10, start_event: "begin", end_event: "end", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 });
        s.motion.duration_ns = 9;
        assert_eq!(
            evaluate(&horizon(vec![s])).disposition,
            HorizonDisposition::InvalidInput
        );
    }

    #[test]
    fn empty_horizon_is_incomplete() {
        assert_eq!(
            evaluate(&horizon(Vec::new())).disposition,
            HorizonDisposition::IncompleteCoverage
        );
    }

    #[test]
    fn nonchronological_segments_are_not_reordered_silently() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "join", end_event: "end", boundary: BoundaryKind::HorizonStart, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "join", boundary: BoundaryKind::SmoothJoin, q_start: [2_000_000, 0, 0], q_end: [2_000_000, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(
            evaluate(&h).disposition,
            HorizonDisposition::IncompleteCoverage
        );
    }

    #[test]
    fn overflowing_seam_distance_abstains() {
        let h = horizon(vec![
            segment(SegmentSpec { id: "s1", start: 0, end: 5, start_event: "begin", end_event: "join", boundary: BoundaryKind::HorizonStart, q_start: [i64::MIN, 0, 0], q_end: [i64::MIN, 0, 0], endpoint_error: 0 }),
            segment(SegmentSpec { id: "s2", start: 5, end: 10, start_event: "join", end_event: "end", boundary: BoundaryKind::SmoothJoin, q_start: [i64::MAX, 0, 0], q_end: [i64::MAX, 0, 0], endpoint_error: 0 }),
        ]);
        assert_eq!(evaluate(&h).disposition, HorizonDisposition::Overflow);
    }
}
