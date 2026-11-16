//! Conservative collision classification under bounded clock-offset uncertainty.
//!
//! Each span declares a hard absolute clock-offset bound relative to the shared
//! site-monotonic domain. A clock offset epsilon moves an affine capsule by at
//! most |translation| * epsilon / span_duration. We widen each interval by
//! epsilon and inflate its radius by that positional bound. The resulting
//! space-time geometry is an outer approximation of all allowed alignments.
//!
//! A disjoint outer query proves NO POSSIBLE CONTACT under the declared bounds.
//! An overlap is POSSIBLE CONTACT / UNRESOLVED: the inflated geometry can include
//! alignments that are not physically realizable. Non-zero clock uncertainty is
//! never converted into an exact collision or clear verdict.

use crate::{
    Capsule, CapsuleMotionSpan, CoTranslatingCapsuleClosestApproach, TimedCcdError,
    TranslationCcdError, Vec3, co_translating_capsule_vs_capsule,
    timed_co_translating_capsule_vs_capsule,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockUncertaintyError {
    EmptyField(&'static str),
    InvalidTimeInterval,
    TimestampUnderflow,
    TimestampOverflow,
    SiteEpochMismatch,
    FrameMismatch,
    ClockDomainMismatch,
    Geometry(TranslationCcdError),
}

impl From<TranslationCcdError> for ClockUncertaintyError {
    fn from(error: TranslationCcdError) -> Self {
        Self::Geometry(error)
    }
}

impl From<TimedCcdError> for ClockUncertaintyError {
    fn from(error: TimedCcdError) -> Self {
        match error {
            TimedCcdError::EmptyField(field) => Self::EmptyField(field),
            TimedCcdError::InvalidTimeInterval => Self::InvalidTimeInterval,
            TimedCcdError::NoTemporalOverlap => Self::InvalidTimeInterval,
            TimedCcdError::SiteEpochMismatch => Self::SiteEpochMismatch,
            TimedCcdError::FrameMismatch => Self::FrameMismatch,
            TimedCcdError::ClockDomainMismatch => Self::ClockDomainMismatch,
            TimedCcdError::ClockUncertaintyUnmodeled => {
                Self::Geometry(TranslationCcdError::NumericallyUnresolved)
            }
            TimedCcdError::Geometry(error) => Self::Geometry(error),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockUncertaintyDisposition {
    ExactContact,
    ExactNoContact,
    BoundaryCandidateUnresolved,
    NoPossibleTemporalOverlap,
    NoPossibleContact,
    PotentialContactUnresolved,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockUncertaintyCcdResult {
    pub disposition: ClockUncertaintyDisposition,
    pub possible_overlap_start_ms: Option<u64>,
    pub possible_overlap_end_ms: Option<u64>,
    pub closest_time_offset_ms: Option<f64>,
    /// Closest approach of the conservative inflated geometry. With non-zero
    /// clock uncertainty this need not be a physically realizable witness.
    pub conservative_closest: Option<CoTranslatingCapsuleClosestApproach>,
    pub moving_clock_margin_mm: f64,
    pub obstacle_clock_margin_mm: f64,
    pub effective_moving_radius_mm: f64,
    pub effective_obstacle_radius_mm: f64,
}

impl ClockUncertaintyCcdResult {
    pub fn exact_contact_established(self) -> bool {
        self.disposition == ClockUncertaintyDisposition::ExactContact
    }

    pub fn no_contact_proven_under_bounds(self) -> bool {
        matches!(
            self.disposition,
            ClockUncertaintyDisposition::ExactNoContact
                | ClockUncertaintyDisposition::NoPossibleContact
                | ClockUncertaintyDisposition::NoPossibleTemporalOverlap
        )
    }
}

/// Compare two absolute-time capsule spans under independent bounded clock offsets.
///
/// A clock offset is assumed constant over its span and bounded in milliseconds
/// relative to the common monotonic domain. The capsule orientation is fixed
/// and translation is affine over the declared half-open interval.
///
/// For each span, this query widens its time interval to [start-epsilon,end+epsilon)
/// and expands its radius by speed*epsilon. Any pose permitted by the declared
/// clock offset is inside that expanded capsule. If the expanded space-time
/// query is disjoint, no collision can occur under the declared clock bounds.
/// If it overlaps, the correct result is unresolved, not a physical collision.
pub fn clock_uncertainty_aware_capsule_query(
    moving: &CapsuleMotionSpan,
    obstacle: &CapsuleMotionSpan,
) -> Result<ClockUncertaintyCcdResult, ClockUncertaintyError> {
    validate_span(moving)?;
    validate_span(obstacle)?;
    validate_identity(moving, obstacle)?;

    let moving_margin = clock_position_margin(moving)?;
    let obstacle_margin = clock_position_margin(obstacle)?;
    let moving_radius = checked_radius(moving.capsule_at_start.radius_mm, moving_margin)?;
    let obstacle_radius = checked_radius(obstacle.capsule_at_start.radius_mm, obstacle_margin)?;

    // Preserve the exact zero-uncertainty path and its half-open terminal handling.
    if moving.clock_uncertainty_ms == 0 && obstacle.clock_uncertainty_ms == 0 {
        let exact = match timed_co_translating_capsule_vs_capsule(moving, obstacle) {
            Ok(result) => result,
            Err(TimedCcdError::NoTemporalOverlap) => {
                return Ok(empty_result(
                    ClockUncertaintyDisposition::NoPossibleTemporalOverlap,
                    moving_margin,
                    obstacle_margin,
                    moving_radius,
                    obstacle_radius,
                ));
            }
            Err(error) => return Err(error.into()),
        };
        let contact = exact.is_contact(
            moving.capsule_at_start.radius_mm,
            obstacle.capsule_at_start.radius_mm,
        );
        let disposition = if contact && exact.closest_at_terminal_limit {
            ClockUncertaintyDisposition::BoundaryCandidateUnresolved
        } else if contact {
            ClockUncertaintyDisposition::ExactContact
        } else {
            ClockUncertaintyDisposition::ExactNoContact
        };
        return Ok(ClockUncertaintyCcdResult {
            disposition,
            possible_overlap_start_ms: Some(exact.overlap_start_ms),
            possible_overlap_end_ms: Some(exact.overlap_end_ms),
            closest_time_offset_ms: Some(exact.closest_time_offset_ms),
            conservative_closest: Some(exact.closest),
            moving_clock_margin_mm: 0.0,
            obstacle_clock_margin_mm: 0.0,
            effective_moving_radius_mm: moving_radius,
            effective_obstacle_radius_mm: obstacle_radius,
        });
    }

    let moving_window = widened_interval(moving)?;
    let obstacle_window = widened_interval(obstacle)?;
    let overlap_start_ms = moving_window.0.max(obstacle_window.0);
    let overlap_end_ms = moving_window.1.min(obstacle_window.1);
    if overlap_start_ms >= overlap_end_ms {
        return Ok(empty_result(
            ClockUncertaintyDisposition::NoPossibleTemporalOverlap,
            moving_margin,
            obstacle_margin,
            moving_radius,
            obstacle_radius,
        ));
    }

    let moving_rebased =
        rebase_and_inflate(moving, overlap_start_ms, overlap_end_ms, moving_margin)?;
    let obstacle_rebased =
        rebase_and_inflate(obstacle, overlap_start_ms, overlap_end_ms, obstacle_margin)?;
    let closest = co_translating_capsule_vs_capsule(
        moving_rebased.0,
        moving_rebased.1,
        obstacle_rebased.0,
        obstacle_rebased.1,
    )?;
    let closest_time_offset_ms =
        ((overlap_end_ms - overlap_start_ms) as f64) * closest.time_parameter;
    if !closest_time_offset_ms.is_finite() {
        return Err(ClockUncertaintyError::Geometry(
            TranslationCcdError::NumericalOverflow,
        ));
    }

    let disposition = if closest.is_contact(moving_radius, obstacle_radius) {
        ClockUncertaintyDisposition::PotentialContactUnresolved
    } else {
        ClockUncertaintyDisposition::NoPossibleContact
    };

    Ok(ClockUncertaintyCcdResult {
        disposition,
        possible_overlap_start_ms: Some(overlap_start_ms),
        possible_overlap_end_ms: Some(overlap_end_ms),
        closest_time_offset_ms: Some(closest_time_offset_ms),
        conservative_closest: Some(closest),
        moving_clock_margin_mm: moving_margin,
        obstacle_clock_margin_mm: obstacle_margin,
        effective_moving_radius_mm: moving_rebased.0.radius_mm,
        effective_obstacle_radius_mm: obstacle_rebased.0.radius_mm,
    })
}

fn empty_result(
    disposition: ClockUncertaintyDisposition,
    moving_margin: f64,
    obstacle_margin: f64,
    moving_radius: f64,
    obstacle_radius: f64,
) -> ClockUncertaintyCcdResult {
    ClockUncertaintyCcdResult {
        disposition,
        possible_overlap_start_ms: None,
        possible_overlap_end_ms: None,
        closest_time_offset_ms: None,
        conservative_closest: None,
        moving_clock_margin_mm: moving_margin,
        obstacle_clock_margin_mm: obstacle_margin,
        effective_moving_radius_mm: moving_radius,
        effective_obstacle_radius_mm: obstacle_radius,
    }
}

fn validate_span(span: &CapsuleMotionSpan) -> Result<(), ClockUncertaintyError> {
    if span.start_ms >= span.end_ms {
        return Err(ClockUncertaintyError::InvalidTimeInterval);
    }
    for (field, value) in [
        ("site_epoch", span.site_epoch.as_str()),
        ("frame_id", span.frame_id.as_str()),
        ("clock_domain_id", span.clock_domain_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ClockUncertaintyError::EmptyField(field));
        }
    }
    if !span.capsule_at_start.radius_mm.is_finite() || span.capsule_at_start.radius_mm < 0.0 {
        return Err(ClockUncertaintyError::Geometry(
            TranslationCcdError::InvalidRadius,
        ));
    }
    if !is_finite(span.capsule_at_start.start) || !is_finite(span.capsule_at_start.end) {
        return Err(ClockUncertaintyError::Geometry(
            TranslationCcdError::NonFiniteCoordinate,
        ));
    }
    if !is_finite(span.translation) {
        return Err(ClockUncertaintyError::Geometry(
            TranslationCcdError::NonFiniteTranslation,
        ));
    }
    Ok(())
}

fn validate_identity(
    moving: &CapsuleMotionSpan,
    obstacle: &CapsuleMotionSpan,
) -> Result<(), ClockUncertaintyError> {
    if moving.site_epoch != obstacle.site_epoch {
        return Err(ClockUncertaintyError::SiteEpochMismatch);
    }
    if moving.frame_id != obstacle.frame_id {
        return Err(ClockUncertaintyError::FrameMismatch);
    }
    if moving.clock_domain_id != obstacle.clock_domain_id {
        return Err(ClockUncertaintyError::ClockDomainMismatch);
    }
    Ok(())
}

fn widened_interval(span: &CapsuleMotionSpan) -> Result<(u64, u64), ClockUncertaintyError> {
    let start = span
        .start_ms
        .checked_sub(span.clock_uncertainty_ms)
        .ok_or(ClockUncertaintyError::TimestampUnderflow)?;
    let end = span
        .end_ms
        .checked_add(span.clock_uncertainty_ms)
        .ok_or(ClockUncertaintyError::TimestampOverflow)?;
    Ok((start, end))
}

fn clock_position_margin(span: &CapsuleMotionSpan) -> Result<f64, ClockUncertaintyError> {
    if span.clock_uncertainty_ms == 0 {
        return Ok(0.0);
    }
    let duration_ms = (span.end_ms - span.start_ms) as f64;
    let uncertainty_fraction = (span.clock_uncertainty_ms as f64) / duration_ms;
    let speed_numerator = span
        .translation
        .x
        .hypot(span.translation.y)
        .hypot(span.translation.z);
    let margin = speed_numerator * uncertainty_fraction;
    if !margin.is_finite() {
        return Err(ClockUncertaintyError::Geometry(
            TranslationCcdError::NumericalOverflow,
        ));
    }
    Ok(margin)
}

fn checked_radius(radius_mm: f64, margin_mm: f64) -> Result<f64, ClockUncertaintyError> {
    let effective = radius_mm + margin_mm;
    if !effective.is_finite() || effective < 0.0 {
        return Err(ClockUncertaintyError::Geometry(
            TranslationCcdError::NumericalOverflow,
        ));
    }
    Ok(effective)
}

fn rebase_and_inflate(
    span: &CapsuleMotionSpan,
    overlap_start_ms: u64,
    overlap_end_ms: u64,
    clock_margin_mm: f64,
) -> Result<(Capsule, Vec3), ClockUncertaintyError> {
    let duration = i128::from(span.end_ms - span.start_ms);
    let start_delta = i128::from(overlap_start_ms) - i128::from(span.start_ms);
    let end_delta = i128::from(overlap_end_ms) - i128::from(span.start_ms);
    let start_fraction = (start_delta as f64) / (duration as f64);
    let end_fraction = (end_delta as f64) / (duration as f64);
    let start_shift = span.translation.scale(start_fraction);
    let overlap_translation = span.translation.scale(end_fraction - start_fraction);
    let radius_mm = checked_radius(span.capsule_at_start.radius_mm, clock_margin_mm)?;
    let capsule = Capsule {
        start: span.capsule_at_start.start.add(start_shift),
        end: span.capsule_at_start.end.add(start_shift),
        radius_mm,
    };
    if !is_finite(capsule.start) || !is_finite(capsule.end) || !is_finite(overlap_translation) {
        return Err(ClockUncertaintyError::Geometry(
            TranslationCcdError::NumericalOverflow,
        ));
    }
    Ok((capsule, overlap_translation))
}

fn is_finite(point: Vec3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn capsule(x: f64) -> Capsule {
        Capsule::new(p(x, -1.0, 0.0), p(x, 1.0, 0.0), 0.5).unwrap()
    }

    fn span(x: f64, dx: f64, start_ms: u64, end_ms: u64, uncertainty_ms: u64) -> CapsuleMotionSpan {
        CapsuleMotionSpan {
            capsule_at_start: capsule(x),
            translation: p(dx, 0.0, 0.0),
            start_ms,
            end_ms,
            site_epoch: "epoch-7".into(),
            frame_id: "site-frame".into(),
            clock_domain_id: "site-monotonic".into(),
            clock_uncertainty_ms: uncertainty_ms,
        }
    }

    #[test]
    fn zero_uncertainty_matches_exact_time_query() {
        let a = span(-10.0, 20.0, 0, 1_000, 0);
        let b = span(10.0, -20.0, 250, 750, 0);
        let result = clock_uncertainty_aware_capsule_query(&a, &b).unwrap();
        let exact = timed_co_translating_capsule_vs_capsule(&a, &b).unwrap();
        assert_eq!(
            result.disposition,
            ClockUncertaintyDisposition::ExactContact
        );
        assert_eq!(
            result.possible_overlap_start_ms,
            Some(exact.overlap_start_ms)
        );
        assert_eq!(result.possible_overlap_end_ms, Some(exact.overlap_end_ms));
        assert_eq!(
            result.closest_time_offset_ms,
            Some(exact.closest_time_offset_ms)
        );
        assert_eq!(result.conservative_closest, Some(exact.closest));
    }

    #[test]
    fn exact_terminal_limit_remains_unresolved() {
        let a = span(-10.0, 19.0, 0, 1_000, 0);
        let b = span(10.0, 0.0, 0, 1_000, 0);
        let result = clock_uncertainty_aware_capsule_query(&a, &b).unwrap();
        assert_eq!(
            result.disposition,
            ClockUncertaintyDisposition::BoundaryCandidateUnresolved
        );
    }

    #[test]
    fn clock_margin_is_speed_times_absolute_offset_bound() {
        let a = span(0.0, 100.0, 0, 1_000, 50);
        let b = span(500.0, 0.0, 0, 1_000, 0);
        let result = clock_uncertainty_aware_capsule_query(&a, &b).unwrap();
        assert!((result.moving_clock_margin_mm - 5.0).abs() < 1.0e-9);
        assert_eq!(result.obstacle_clock_margin_mm, 0.0);
        assert!((result.effective_moving_radius_mm - 5.5).abs() < 1.0e-9);
    }

    #[test]
    fn nonzero_uncertainty_never_establishes_exact_contact() {
        let a = span(-10.0, 20.0, 0, 1_000, 5);
        let b = span(10.0, -20.0, 0, 1_000, 5);
        let result = clock_uncertainty_aware_capsule_query(&a, &b).unwrap();
        assert_eq!(
            result.disposition,
            ClockUncertaintyDisposition::PotentialContactUnresolved
        );
        assert!(!result.exact_contact_established());
        assert!(!result.no_contact_proven_under_bounds());
    }

    #[test]
    fn clock_offset_that_could_hide_contact_returns_unresolved() {
        let a = span(0.0, 100.0, 0, 1_000, 50);
        let b = span(5.0, 0.0, 0, 1_000, 0);
        let result = clock_uncertainty_aware_capsule_query(&a, &b).unwrap();
        assert_eq!(
            result.disposition,
            ClockUncertaintyDisposition::PotentialContactUnresolved
        );
    }

    #[test]
    fn separated_bodies_are_proven_clear_under_declared_bounds() {
        let a = span(0.0, 0.0, 0, 1_000, 50);
        let b = span(10.0, 0.0, 0, 1_000, 25);
        let result = clock_uncertainty_aware_capsule_query(&a, &b).unwrap();
        assert_eq!(
            result.disposition,
            ClockUncertaintyDisposition::NoPossibleContact
        );
        assert!(result.no_contact_proven_under_bounds());
    }

    #[test]
    fn widening_can_create_a_possible_temporal_overlap() {
        let a = span(0.0, 0.0, 100, 200, 60);
        let b = span(10.0, 0.0, 250, 350, 0);
        let result = clock_uncertainty_aware_capsule_query(&a, &b).unwrap();
        assert_eq!(
            result.disposition,
            ClockUncertaintyDisposition::NoPossibleContact
        );
        assert_eq!(result.possible_overlap_start_ms, Some(250));
        assert_eq!(result.possible_overlap_end_ms, Some(260));
    }

    #[test]
    fn disjoint_widened_windows_prove_no_possible_temporal_overlap() {
        let a = span(0.0, 0.0, 0, 100, 10);
        let b = span(10.0, 0.0, 200, 300, 20);
        let result = clock_uncertainty_aware_capsule_query(&a, &b).unwrap();
        assert_eq!(
            result.disposition,
            ClockUncertaintyDisposition::NoPossibleTemporalOverlap
        );
        assert_eq!(result.conservative_closest, None);
    }

    #[test]
    fn larger_clock_bounds_do_not_reduce_the_outer_geometry() {
        let small_span = span(0.0, 100.0, 0, 1_000, 10);
        let large_span = span(0.0, 100.0, 0, 1_000, 25);
        let obstacle = span(5.0, 0.0, 0, 1_000, 0);
        let small = clock_uncertainty_aware_capsule_query(&small_span, &obstacle).unwrap();
        let large = clock_uncertainty_aware_capsule_query(&large_span, &obstacle).unwrap();
        assert_eq!(
            small.disposition,
            ClockUncertaintyDisposition::PotentialContactUnresolved
        );
        assert_eq!(
            large.disposition,
            ClockUncertaintyDisposition::PotentialContactUnresolved
        );
        assert!(large.moving_clock_margin_mm >= small.moving_clock_margin_mm);
        assert!(
            large.possible_overlap_start_ms.unwrap() <= small.possible_overlap_start_ms.unwrap()
        );
        assert!(large.possible_overlap_end_ms.unwrap() >= small.possible_overlap_end_ms.unwrap());
    }

    #[test]
    fn world_translation_preserves_disposition_and_distance() {
        let a = span(0.0, 100.0, 0, 1_000, 50);
        let b = span(5.0, 0.0, 0, 1_000, 0);
        let mut moved_a = a.clone();
        let mut moved_b = b.clone();
        let shift = p(100.0, -50.0, 25.0);
        moved_a.capsule_at_start.start = moved_a.capsule_at_start.start.add(shift);
        moved_a.capsule_at_start.end = moved_a.capsule_at_start.end.add(shift);
        moved_b.capsule_at_start.start = moved_b.capsule_at_start.start.add(shift);
        moved_b.capsule_at_start.end = moved_b.capsule_at_start.end.add(shift);
        let original = clock_uncertainty_aware_capsule_query(&a, &b).unwrap();
        let shifted = clock_uncertainty_aware_capsule_query(&moved_a, &moved_b).unwrap();
        assert_eq!(original.disposition, shifted.disposition);
        assert!(
            (original.conservative_closest.unwrap().distance_mm
                - shifted.conservative_closest.unwrap().distance_mm)
                .abs()
                < 1.0e-9
        );
    }

    #[test]
    fn generated_time_shifted_contacts_are_never_pruned() {
        // The reported span starts differ from physical start times by independent
        // constant offsets bounded by epsilon. Every exact physical contact must
        // remain an outer-envelope candidate after widening and radius inflation.
        let mut seed = 0x5ea1_2026_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 2_001) as i64 - 1_000
        };
        let mut contacts = 0_u32;
        for index in 0..128_u64 {
            let uncertainty_a = 1 + (index % 25);
            let uncertainty_b = 1 + ((index * 7) % 25);
            let offset_a = (index as i64 % (2 * uncertainty_a as i64 + 1)) - uncertainty_a as i64;
            let offset_b =
                ((index * 3) as i64 % (2 * uncertainty_b as i64 + 1)) - uncertainty_b as i64;
            let base_time = 100_000_u64;
            let (reported_a_x, reported_a_dx, reported_b_x, reported_b_dx) = if index < 32 {
                (-10.0, 20.0, 10.0, -20.0)
            } else {
                (next() as f64, next() as f64, next() as f64, next() as f64)
            };
            let mut reported_a = span(
                reported_a_x,
                reported_a_dx,
                base_time,
                base_time + 1_000,
                uncertainty_a,
            );
            let mut reported_b = span(
                reported_b_x,
                reported_b_dx,
                base_time + 100,
                base_time + 900,
                uncertainty_b,
            );
            // Use point capsules so every generated case has a unique centreline parameter.
            reported_a.capsule_at_start = Capsule::new(
                reported_a.capsule_at_start.start,
                reported_a.capsule_at_start.start,
                0.5,
            )
            .unwrap();
            reported_b.capsule_at_start = Capsule::new(
                reported_b.capsule_at_start.start,
                reported_b.capsule_at_start.start,
                0.5,
            )
            .unwrap();

            let outer = clock_uncertainty_aware_capsule_query(&reported_a, &reported_b).unwrap();
            let physical_a = CapsuleMotionSpan {
                start_ms: (base_time as i64 - offset_a) as u64,
                end_ms: (base_time as i64 + 1_000 - offset_a) as u64,
                clock_uncertainty_ms: 0,
                ..reported_a.clone()
            };
            let physical_b = CapsuleMotionSpan {
                start_ms: (base_time as i64 + 100 - offset_b) as u64,
                end_ms: (base_time as i64 + 900 - offset_b) as u64,
                clock_uncertainty_ms: 0,
                ..reported_b.clone()
            };
            let actual = timed_co_translating_capsule_vs_capsule(&physical_a, &physical_b);
            if actual.is_ok_and(|result| result.is_contact(0.5, 0.5)) {
                contacts += 1;
                assert_eq!(
                    outer.disposition,
                    ClockUncertaintyDisposition::PotentialContactUnresolved,
                    "actual contact was pruned at generated case {index}"
                );
            }
        }
        assert!(contacts > 0, "generated suite must contain contact cases");
    }

    #[test]
    fn rejects_timestamp_arithmetic_overflow_and_identity_mismatch() {
        let a = span(0.0, 1.0, 5, 100, 10);
        let b = span(10.0, 0.0, 5, 100, 0);
        assert_eq!(
            clock_uncertainty_aware_capsule_query(&a, &b),
            Err(ClockUncertaintyError::TimestampUnderflow)
        );

        let a = span(0.0, 1.0, 0, u64::MAX, 1);
        let b = span(10.0, 0.0, 0, u64::MAX, 0);
        assert_eq!(
            clock_uncertainty_aware_capsule_query(&a, &b),
            Err(ClockUncertaintyError::TimestampOverflow)
        );

        let a = span(0.0, 1.0, 0, 100, 5);
        let mut b = span(10.0, 0.0, 0, 100, 0);
        b.frame_id = "wrong-frame".into();
        assert_eq!(
            clock_uncertainty_aware_capsule_query(&a, &b),
            Err(ClockUncertaintyError::FrameMismatch)
        );
    }
}
