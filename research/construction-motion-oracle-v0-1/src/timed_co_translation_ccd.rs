//! Absolute-time collision queries for linearly translating fixed-orientation capsules.
//!
//! A motion span supplies its capsule at its own start time and one linear
//! translation over a half-open interval. The query intersects two spans,
//! rebases each motion to the shared overlap, then delegates geometry to the
//! co-translating capsule oracle. Clock/frame/site identity is checked before
//! geometry is compared.

use crate::{
    Capsule, CoTranslatingCapsuleClosestApproach, TranslationCcdError, Vec3,
    co_translating_capsule_vs_capsule,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CapsuleMotionSpan {
    /// Capsule geometry at the motion span's start timestamp.
    pub capsule_at_start: Capsule,
    /// Translation from start_ms to end_ms; orientation is fixed.
    pub translation: Vec3,
    /// Common site-monotonic timestamp domain, half-open interval start.
    pub start_ms: u64,
    /// Common site-monotonic timestamp domain, half-open interval end.
    pub end_ms: u64,
    pub site_epoch: String,
    pub frame_id: String,
    pub clock_domain_id: String,
    /// This v0.1 implementation accepts only an exactly qualified shared clock.
    /// Non-zero uncertainty abstains rather than being silently ignored.
    pub clock_uncertainty_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimedCcdError {
    EmptyField(&'static str),
    InvalidTimeInterval,
    NoTemporalOverlap,
    SiteEpochMismatch,
    FrameMismatch,
    ClockDomainMismatch,
    ClockUncertaintyUnmodeled,
    Geometry(TranslationCcdError),
}

impl From<TranslationCcdError> for TimedCcdError {
    fn from(error: TranslationCcdError) -> Self {
        Self::Geometry(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedCoTranslatingClosestApproach {
    /// Positive-duration common interval. The source execution spans are half-open.
    pub overlap_start_ms: u64,
    pub overlap_end_ms: u64,
    /// Closest time measured from overlap_start_ms; avoids converting a large
    /// absolute monotonic timestamp to f64.
    pub closest_time_offset_ms: f64,
    pub closest_time_parameter: f64,
    /// True when the closest point is at the excluded end limit of the common
    /// half-open interval. The CCD query includes this limit conservatively;
    /// it can therefore be a boundary candidate rather than an attained
    /// collision at an active timestamp.
    pub closest_at_terminal_limit: bool,
    pub closest: CoTranslatingCapsuleClosestApproach,
}

impl TimedCoTranslatingClosestApproach {
    pub fn is_contact(self, moving_radius_mm: f64, obstacle_radius_mm: f64) -> bool {
        self.closest.is_contact(moving_radius_mm, obstacle_radius_mm)
    }
}

/// Compare two capsule motions whose absolute start/end times may differ.
///
/// Both spans must use the same non-empty site epoch, target frame and clock
/// domain. Non-zero clock uncertainty is explicitly unsupported in v0.1 and
/// returns ClockUncertaintyUnmodeled; it is never treated as zero.
///
/// The shared interval is the positive-duration intersection of the two
/// half-open intervals. A pair that only touches at one interval boundary
/// returns NoTemporalOverlap. For conservative continuous collision
/// checking, the geometric solver includes the terminal limit of a positive
/// overlap interval. If closest approach occurs only at that excluded limit,
/// closest_at_terminal_limit flags it for caller-side conservative handling.
pub fn timed_co_translating_capsule_vs_capsule(
    moving: &CapsuleMotionSpan,
    obstacle: &CapsuleMotionSpan,
) -> Result<TimedCoTranslatingClosestApproach, TimedCcdError> {
    validate_span(moving)?;
    validate_span(obstacle)?;

    if moving.site_epoch != obstacle.site_epoch {
        return Err(TimedCcdError::SiteEpochMismatch);
    }
    if moving.frame_id != obstacle.frame_id {
        return Err(TimedCcdError::FrameMismatch);
    }
    if moving.clock_domain_id != obstacle.clock_domain_id {
        return Err(TimedCcdError::ClockDomainMismatch);
    }

    let overlap_start_ms = moving.start_ms.max(obstacle.start_ms);
    let overlap_end_ms = moving.end_ms.min(obstacle.end_ms);
    if overlap_start_ms >= overlap_end_ms {
        return Err(TimedCcdError::NoTemporalOverlap);
    }

    let moving_at_overlap_start =
        rebase_span_to_time(moving, overlap_start_ms, overlap_end_ms)?;
    let obstacle_at_overlap_start =
        rebase_span_to_time(obstacle, overlap_start_ms, overlap_end_ms)?;

    let closest = co_translating_capsule_vs_capsule(
        moving_at_overlap_start.0,
        moving_at_overlap_start.1,
        obstacle_at_overlap_start.0,
        obstacle_at_overlap_start.1,
    )?;

    let overlap_duration_ms = overlap_end_ms - overlap_start_ms;
    let closest_time_offset_ms =
        (overlap_duration_ms as f64) * closest.time_parameter;
    if !closest_time_offset_ms.is_finite() {
        return Err(TimedCcdError::Geometry(
            TranslationCcdError::NumericalOverflow,
        ));
    }

    Ok(TimedCoTranslatingClosestApproach {
        overlap_start_ms,
        overlap_end_ms,
        closest_time_offset_ms,
        closest_time_parameter: closest.time_parameter,
        closest_at_terminal_limit: closest.time_parameter == 1.0,
        closest,
    })
}

/// Continuous contact predicate over the common execution window.
pub fn timed_co_translating_capsules_collision(
    moving: &CapsuleMotionSpan,
    obstacle: &CapsuleMotionSpan,
) -> Result<bool, TimedCcdError> {
    let closest = timed_co_translating_capsule_vs_capsule(moving, obstacle)?;
    Ok(closest.is_contact(
        moving.capsule_at_start.radius_mm,
        obstacle.capsule_at_start.radius_mm,
    ))
}

fn validate_span(span: &CapsuleMotionSpan) -> Result<(), TimedCcdError> {
    if span.start_ms >= span.end_ms {
        return Err(TimedCcdError::InvalidTimeInterval);
    }
    for (field, value) in [
        ("site_epoch", span.site_epoch.as_str()),
        ("frame_id", span.frame_id.as_str()),
        ("clock_domain_id", span.clock_domain_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(TimedCcdError::EmptyField(field));
        }
    }
    if span.clock_uncertainty_ms != 0 {
        return Err(TimedCcdError::ClockUncertaintyUnmodeled);
    }
    if !span.capsule_at_start.radius_mm.is_finite()
        || span.capsule_at_start.radius_mm < 0.0
    {
        return Err(TimedCcdError::Geometry(
            TranslationCcdError::InvalidRadius,
        ));
    }
    if !is_finite(span.capsule_at_start.start)
        || !is_finite(span.capsule_at_start.end)
    {
        return Err(TimedCcdError::Geometry(
            TranslationCcdError::NonFiniteCoordinate,
        ));
    }
    if !is_finite(span.translation) {
        return Err(TimedCcdError::Geometry(
            TranslationCcdError::NonFiniteTranslation,
        ));
    }
    Ok(())
}

/// Return the capsule pose at overlap_start and its translation over the
/// complete common interval. The result is still a rigid translation.
fn rebase_span_to_time(
    span: &CapsuleMotionSpan,
    overlap_start_ms: u64,
    overlap_end_ms: u64,
) -> Result<(Capsule, Vec3), TimedCcdError> {
    let span_duration = (span.end_ms - span.start_ms) as f64;
    let overlap_start_fraction = ((overlap_start_ms - span.start_ms) as f64) / span_duration;
    let overlap_end_fraction = ((overlap_end_ms - span.start_ms) as f64) / span_duration;
    let shift = span.translation.scale(overlap_start_fraction);
    let overlap_translation =
        span.translation.scale(overlap_end_fraction - overlap_start_fraction);

    let capsule = Capsule {
        start: span.capsule_at_start.start.add(shift),
        end: span.capsule_at_start.end.add(shift),
        radius_mm: span.capsule_at_start.radius_mm,
    };
    if !is_finite(capsule.start)
        || !is_finite(capsule.end)
        || !is_finite(overlap_translation)
    {
        return Err(TimedCcdError::Geometry(
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

    fn capsule(x: f64, radius_mm: f64) -> Capsule {
        Capsule::new(p(x, -1.0, 0.0), p(x, 1.0, 0.0), radius_mm).unwrap()
    }

    fn span(x: f64, translation_x: f64, start_ms: u64, end_ms: u64) -> CapsuleMotionSpan {
        CapsuleMotionSpan {
            capsule_at_start: capsule(x, 0.5),
            translation: p(translation_x, 0.0, 0.0),
            start_ms,
            end_ms,
            site_epoch: "epoch-7".into(),
            frame_id: "site-frame".into(),
            clock_domain_id: "site-monotonic".into(),
            clock_uncertainty_ms: 0,
        }
    }

    #[test]
    fn different_start_end_times_detect_collision_in_shared_window() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 250, 750);
        let closest = timed_co_translating_capsule_vs_capsule(&moving, &obstacle).unwrap();

        assert_eq!(closest.overlap_start_ms, 250);
        assert_eq!(closest.overlap_end_ms, 750);
        assert!((closest.closest_time_parameter - 0.5).abs() < 1.0e-9);
        assert!((closest.closest_time_offset_ms - 250.0).abs() < 1.0e-9);
        assert!(!closest.closest_at_terminal_limit);
        assert!(closest.is_contact(0.5, 0.5));
        assert!(timed_co_translating_capsules_collision(&moving, &obstacle).unwrap());
    }

    #[test]
    fn different_durations_rebase_each_velocity_to_the_common_interval() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 200, 800);
        let closest = timed_co_translating_capsule_vs_capsule(&moving, &obstacle).unwrap();

        assert_eq!(closest.overlap_start_ms, 200);
        assert_eq!(closest.overlap_end_ms, 800);
        assert!((closest.closest_time_parameter - 0.5).abs() < 1.0e-9);
        assert!((closest.closest_time_offset_ms - 300.0).abs() < 1.0e-9);
        assert!(closest.is_contact(0.5, 0.5));
    }

    #[test]
    fn touching_only_half_open_intervals_have_no_positive_overlap() {
        let moving = span(-10.0, 20.0, 0, 100);
        let obstacle = span(10.0, -20.0, 100, 200);
        assert_eq!(
            timed_co_translating_capsule_vs_capsule(&moving, &obstacle),
            Err(TimedCcdError::NoTemporalOverlap)
        );
    }

    #[test]
    fn disjoint_execution_windows_do_not_call_geometry_a_clear_verdict() {
        let moving = span(-10.0, 20.0, 0, 100);
        let obstacle = span(10.0, -20.0, 200, 300);
        assert_eq!(
            timed_co_translating_capsule_vs_capsule(&moving, &obstacle),
            Err(TimedCcdError::NoTemporalOverlap)
        );
    }

    #[test]
    fn rejects_site_epoch_frame_and_clock_domain_mismatches() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let mut obstacle = span(10.0, -20.0, 0, 1_000);

        obstacle.site_epoch = "epoch-8".into();
        assert_eq!(
            timed_co_translating_capsule_vs_capsule(&moving, &obstacle),
            Err(TimedCcdError::SiteEpochMismatch)
        );
        obstacle.site_epoch = moving.site_epoch.clone();
        obstacle.frame_id = "other-frame".into();
        assert_eq!(
            timed_co_translating_capsule_vs_capsule(&moving, &obstacle),
            Err(TimedCcdError::FrameMismatch)
        );
        obstacle.frame_id = moving.frame_id.clone();
        obstacle.clock_domain_id = "other-clock".into();
        assert_eq!(
            timed_co_translating_capsule_vs_capsule(&moving, &obstacle),
            Err(TimedCcdError::ClockDomainMismatch)
        );
    }

    #[test]
    fn nonzero_clock_uncertainty_abstains() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let mut obstacle = span(10.0, -20.0, 0, 1_000);
        obstacle.clock_uncertainty_ms = 1;
        assert_eq!(
            timed_co_translating_capsule_vs_capsule(&moving, &obstacle),
            Err(TimedCcdError::ClockUncertaintyUnmodeled)
        );
    }

    #[test]
    fn common_time_origin_shift_preserves_geometry_and_relative_offset() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 250, 750);
        let shifted_moving = span(-10.0, 20.0, 50_000, 51_000);
        let shifted_obstacle = span(10.0, -20.0, 50_250, 50_750);

        let original = timed_co_translating_capsule_vs_capsule(&moving, &obstacle).unwrap();
        let shifted =
            timed_co_translating_capsule_vs_capsule(&shifted_moving, &shifted_obstacle).unwrap();

        assert_eq!(shifted.overlap_start_ms, original.overlap_start_ms + 50_000);
        assert_eq!(shifted.overlap_end_ms, original.overlap_end_ms + 50_000);
        assert!((shifted.closest_time_offset_ms - original.closest_time_offset_ms).abs() < 1.0e-9);
        assert!((shifted.closest.distance_mm - original.closest.distance_mm).abs() < 1.0e-9);
    }

    #[test]
    fn world_translation_preserves_distance_and_closest_offset() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 250, 750);
        let mut moved_a = moving.clone();
        let mut moved_b = obstacle.clone();
        moved_a.capsule_at_start.start = moved_a.capsule_at_start.start.add(p(100.0, -50.0, 25.0));
        moved_a.capsule_at_start.end = moved_a.capsule_at_start.end.add(p(100.0, -50.0, 25.0));
        moved_b.capsule_at_start.start = moved_b.capsule_at_start.start.add(p(100.0, -50.0, 25.0));
        moved_b.capsule_at_start.end = moved_b.capsule_at_start.end.add(p(100.0, -50.0, 25.0));
        let original = timed_co_translating_capsule_vs_capsule(&moving, &obstacle).unwrap();
        let transformed = timed_co_translating_capsule_vs_capsule(&moved_a, &moved_b).unwrap();
        assert!((original.closest.distance_mm - transformed.closest.distance_mm).abs() < 1.0e-9);
        assert!(
            (original.closest_time_offset_ms - transformed.closest_time_offset_ms).abs() < 1.0e-9
        );
    }

    #[test]
    fn reports_conservative_contact_at_the_terminal_limit() {
        let moving = span(-10.0, 19.0, 0, 1_000);
        let obstacle = span(10.0, 0.0, 0, 1_000);
        let closest = timed_co_translating_capsule_vs_capsule(&moving, &obstacle).unwrap();

        assert_eq!(closest.closest_time_parameter, 1.0);
        assert!(closest.closest_at_terminal_limit);
        assert!(closest.is_contact(0.5, 0.5));
    }

    #[test]
    fn rejects_empty_identity_and_invalid_intervals() {
        let mut moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 0, 1_000);
        moving.frame_id.clear();
        assert_eq!(
            timed_co_translating_capsule_vs_capsule(&moving, &obstacle),
            Err(TimedCcdError::EmptyField("frame_id"))
        );
        moving.frame_id = "site-frame".into();
        moving.end_ms = moving.start_ms;
        assert_eq!(
            timed_co_translating_capsule_vs_capsule(&moving, &obstacle),
            Err(TimedCcdError::InvalidTimeInterval)
        );
    }
}
