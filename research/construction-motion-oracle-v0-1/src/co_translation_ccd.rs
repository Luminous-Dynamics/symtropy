//! Continuous collision reference for two independently translating,
//! fixed-orientation capsules over one common normalized time interval.
//!
//! The query is reduced to relative translation: delta = moving_delta -
//! obstacle_delta. It then reuses the bounded convex-quadratic active-set
//! solver in translation_ccd. Returned closest points are reconstructed in
//! world coordinates at the reported common time.

use crate::{
    Capsule, CapsuleAabb3, TranslationCcdError, Vec3, endpoints_only_translating_capsule_collision,
    translating_capsule_vs_capsule,
};

const CONTACT_EPSILON_MM: f64 = 1.0e-9;

/// Closed interval in a shared normalized trajectory time domain [0, 1].
/// Adjacent candidate windows may share an endpoint so a contact exactly at a
/// subdivision boundary cannot be dropped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedTimeWindow {
    pub start_parameter: f64,
    pub end_parameter: f64,
}

impl NormalizedTimeWindow {
    fn validate(self) -> Result<(), TranslationCcdError> {
        if !self.start_parameter.is_finite()
            || !self.end_parameter.is_finite()
            || self.start_parameter < 0.0
            || self.end_parameter > 1.0
            || self.start_parameter >= self.end_parameter
        {
            return Err(TranslationCcdError::InvalidTimeWindow);
        }
        Ok(())
    }

    pub fn width(self) -> f64 {
        self.end_parameter - self.start_parameter
    }

    pub fn contains_inclusive(self, parameter: f64) -> bool {
        parameter >= self.start_parameter && parameter <= self.end_parameter
    }
}

/// Maximum refinement depth and minimum normalized interval width.
/// A depth of zero checks the entire trajectory as one candidate interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveSubdivisionConfig {
    pub maximum_depth: u8,
    pub minimum_window_width: f64,
}

impl AdaptiveSubdivisionConfig {
    fn validate(self) -> Result<(), TranslationCcdError> {
        if self.maximum_depth > 20 {
            return Err(TranslationCcdError::InvalidAdaptiveDepth);
        }
        if !self.minimum_window_width.is_finite()
            || self.minimum_window_width <= 0.0
            || self.minimum_window_width > 1.0
        {
            return Err(TranslationCcdError::InvalidMinimumWindowWidth);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdaptiveCollisionStatus {
    Contact,
    NoContact,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AdaptiveSubdivisionStats {
    pub node_visits: u32,
    pub split_intervals: u32,
    pub pruned_intervals: u32,
    pub candidate_window_count: u32,
    pub exact_narrow_phase_calls: u32,
    pub unresolved_candidate_count: u32,
    pub maximum_depth_reached: u8,
    pub candidate_time_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveCollisionReport {
    pub status: AdaptiveCollisionStatus,
    pub witness: Option<CoTranslatingCapsuleClosestApproach>,
    pub witness_window: Option<NormalizedTimeWindow>,
    /// Sorted disjoint interiors; adjacent windows may share closed endpoints.
    pub candidate_windows: Vec<NormalizedTimeWindow>,
    pub unresolved_windows: Vec<NormalizedTimeWindow>,
    pub stats: AdaptiveSubdivisionStats,
}
const MAX_TIME_SUBDIVISIONS: u32 = 1_048_576;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeSlicedAabbCandidates {
    /// Number of equal-width time slabs evaluated.
    pub subdivision_count: u32,
    /// Number of slabs whose conservative space-time AABBs overlap.
    pub candidate_slab_count: u32,
}

impl TimeSlicedAabbCandidates {
    pub fn has_candidate(self) -> bool {
        self.candidate_slab_count > 0
    }

    /// Fraction of the normalized time domain that remains a broad-phase candidate.
    pub fn candidate_time_fraction(self) -> f64 {
        f64::from(self.candidate_slab_count) / f64::from(self.subdivision_count)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoTranslatingCapsuleClosestApproach {
    pub time_parameter: f64,
    pub moving_axis_parameter: f64,
    pub obstacle_axis_parameter: f64,
    /// Closest point on the moving capsule axis in world coordinates.
    pub moving_point: Vec3,
    /// Closest point on the obstacle capsule axis in world coordinates.
    pub obstacle_point: Vec3,
    pub distance_mm: f64,
}

impl CoTranslatingCapsuleClosestApproach {
    pub fn is_contact(self, moving_radius_mm: f64, obstacle_radius_mm: f64) -> bool {
        self.distance_mm <= moving_radius_mm + obstacle_radius_mm + CONTACT_EPSILON_MM
    }
}

/// Global closest approach between two fixed-orientation capsules that each
/// translate linearly over the same normalized time interval [0, 1].
///
/// If A(t) = A(0) + da*t and B(t) = B(0) + db*t, the distance depends on
/// relative translation (da - db)*t. The existing three-variable active-set
/// oracle therefore solves the exact bounded problem without sampling time.
///
/// Result points are translated back into world coordinates. A successful
/// result is valid only for the declared rigid-translation geometry family;
/// rotation and articulated motion are outside this API's scope.
pub fn co_translating_capsule_vs_capsule(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<CoTranslatingCapsuleClosestApproach, TranslationCcdError> {
    if !is_finite(moving_translation) || !is_finite(obstacle_translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }

    let relative_translation = moving_translation.sub(obstacle_translation);
    if !is_finite(relative_translation) {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    let relative = translating_capsule_vs_capsule(moving, relative_translation, obstacle)?;
    let common_world_shift = obstacle_translation.scale(relative.time_parameter);
    let moving_point = relative.moving_point.add(common_world_shift);
    let obstacle_point = relative.obstacle_point.add(common_world_shift);
    let distance_mm = moving_point.distance(obstacle_point);

    if !is_finite(moving_point) || !is_finite(obstacle_point) || !distance_mm.is_finite() {
        return Err(TranslationCcdError::NumericalOverflow);
    }

    Ok(CoTranslatingCapsuleClosestApproach {
        time_parameter: relative.time_parameter,
        moving_axis_parameter: relative.moving_axis_parameter,
        obstacle_axis_parameter: relative.obstacle_axis_parameter,
        moving_point,
        obstacle_point,
        distance_mm,
    })
}

/// Continuous contact truth for two fixed-orientation translating capsules.
pub fn co_translating_capsule_capsule_collision(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<bool, TranslationCcdError> {
    let closest = co_translating_capsule_vs_capsule(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
    )?;
    Ok(closest.is_contact(moving.radius_mm, obstacle.radius_mm))
}

/// Endpoint-only baseline at t=0 and t=1. It uses relative translation so
/// both bodies are evaluated at the same endpoint time.
pub fn endpoints_only_co_translating_capsule_collision(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<bool, TranslationCcdError> {
    if !is_finite(moving_translation) || !is_finite(obstacle_translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }
    endpoints_only_translating_capsule_collision(
        moving,
        moving_translation.sub(obstacle_translation),
        obstacle,
    )
}

/// Conservative swept-AABB candidate for two independently translating
/// capsules. Each capsule's initial and final AABBs are unioned over the full
/// interval. Their overlap is a broad-phase candidate only: the two swept
/// sets may intersect at different times without a synchronized contact.
pub fn co_translating_capsule_swept_aabb_candidate(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<bool, TranslationCcdError> {
    let moving_swept = swept_bounds(moving, moving_translation)?;
    let obstacle_swept = swept_bounds(obstacle, obstacle_translation)?;
    Ok(moving_swept.overlaps(obstacle_swept))
}

/// Time-coherent broad phase for two linearly translating, fixed-orientation
/// capsules. Each equal-width time slab uses the union of each body's AABBs
/// at the two slab endpoints. Since each coordinate of a fixed-orientation
/// capsule translates linearly, that union encloses the body's AABB throughout
/// the slab. Any synchronized collision must therefore appear in at least one
/// overlapping slab candidate.
///
/// The returned count is diagnostic, not a collision verdict. Increasing the
/// subdivision count along a nested 1,2,4,8,... sequence can only remove
/// candidate *time fraction*; raw candidate-slab count may rise when one
/// coarse candidate slab splits into multiple candidate children. A positive
/// result still requires the continuous narrow phase.
pub fn co_translating_capsule_time_sliced_aabb_candidates(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    subdivisions: u32,
) -> Result<TimeSlicedAabbCandidates, TranslationCcdError> {
    if subdivisions == 0 || subdivisions > MAX_TIME_SUBDIVISIONS {
        return Err(TranslationCcdError::InvalidSubdivisionCount);
    }
    validate_translating_capsule(moving, moving_translation)?;
    validate_translating_capsule(obstacle, obstacle_translation)?;

    let mut candidate_slab_count = 0_u32;
    let denominator = f64::from(subdivisions);
    for slab in 0..subdivisions {
        let t0 = f64::from(slab) / denominator;
        let t1 = f64::from(slab + 1) / denominator;
        let moving_bounds = slab_swept_bounds(moving, moving_translation, t0, t1)?;
        let obstacle_bounds = slab_swept_bounds(obstacle, obstacle_translation, t0, t1)?;
        if moving_bounds.overlaps(obstacle_bounds) {
            candidate_slab_count += 1;
        }
    }

    Ok(TimeSlicedAabbCandidates {
        subdivision_count: subdivisions,
        candidate_slab_count,
    })
}

/// Exact co-translating capsule closest approach restricted to a normalized
/// global time window. The returned time parameter is remapped to [0, 1] in
/// the original trajectory, and both closest points remain in world coordinates.
pub fn co_translating_capsule_vs_capsule_in_time_window(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    window: NormalizedTimeWindow,
) -> Result<CoTranslatingCapsuleClosestApproach, TranslationCcdError> {
    window.validate()?;
    validate_translating_capsule(moving, moving_translation)?;
    validate_translating_capsule(obstacle, obstacle_translation)?;

    let moving_at_start = capsule_at(moving, moving_translation, window.start_parameter)?;
    let obstacle_at_start = capsule_at(obstacle, obstacle_translation, window.start_parameter)?;
    let width = window.width();

    let local = co_translating_capsule_vs_capsule(
        moving_at_start,
        moving_translation.scale(width),
        obstacle_at_start,
        obstacle_translation.scale(width),
    )?;
    let global_time = window.start_parameter + local.time_parameter * width;
    if !global_time.is_finite() {
        return Err(TranslationCcdError::NumericalOverflow);
    }

    Ok(CoTranslatingCapsuleClosestApproach {
        time_parameter: global_time,
        moving_axis_parameter: local.moving_axis_parameter,
        obstacle_axis_parameter: local.obstacle_axis_parameter,
        moving_point: local.moving_point,
        obstacle_point: local.obstacle_point,
        distance_mm: local.distance_mm,
    })
}

/// Adaptive space-time broad phase followed by exact restricted continuous
/// narrow-phase queries.
///
/// Candidate intervals are generated by recursively bisecting [0, 1] only
/// while the two conservative per-window swept AABBs overlap. A disjoint pair
/// of boxes safely prunes that time interval for the supported affine,
/// fixed-orientation capsule model. Every remaining leaf is sent to the exact
/// continuous capsule oracle over that leaf's global time window.
///
/// NO_CONTACT is returned only after every candidate leaf has been checked
/// and no leaf was unresolved. A numerically ill-conditioned leaf produces
/// UNRESOLVED, never a clear result. A proven contact takes precedence over
/// unresolved leaves because contact is already established.
pub fn co_translating_capsule_adaptive_query(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    config: AdaptiveSubdivisionConfig,
) -> Result<AdaptiveCollisionReport, TranslationCcdError> {
    config.validate()?;
    validate_translating_capsule(moving, moving_translation)?;
    validate_translating_capsule(obstacle, obstacle_translation)?;

    let (candidate_windows, mut stats) = collect_adaptive_candidate_windows(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
        config,
    )?;

    let mut unresolved_windows = Vec::new();
    let mut witness = None;
    let mut witness_window = None;

    for window in &candidate_windows {
        stats.exact_narrow_phase_calls += 1;
        match co_translating_capsule_vs_capsule_in_time_window(
            moving,
            moving_translation,
            obstacle,
            obstacle_translation,
            *window,
        ) {
            Ok(closest) => {
                if closest.is_contact(moving.radius_mm, obstacle.radius_mm) {
                    witness = Some(closest);
                    witness_window = Some(*window);
                    break;
                }
            }
            Err(TranslationCcdError::NumericallyUnresolved) => {
                unresolved_windows.push(*window);
            }
            Err(error) => return Err(error),
        }
    }

    stats.unresolved_candidate_count = unresolved_windows.len() as u32;
    let status = if witness.is_some() {
        AdaptiveCollisionStatus::Contact
    } else if unresolved_windows.is_empty() {
        AdaptiveCollisionStatus::NoContact
    } else {
        AdaptiveCollisionStatus::Unresolved
    };

    Ok(AdaptiveCollisionReport {
        status,
        witness,
        witness_window,
        candidate_windows,
        unresolved_windows,
        stats,
    })
}

fn collect_adaptive_candidate_windows(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    config: AdaptiveSubdivisionConfig,
) -> Result<(Vec<NormalizedTimeWindow>, AdaptiveSubdivisionStats), TranslationCcdError> {
    let mut stack = vec![(
        NormalizedTimeWindow {
            start_parameter: 0.0,
            end_parameter: 1.0,
        },
        0_u8,
    )];
    let mut candidates = Vec::new();
    let mut stats = AdaptiveSubdivisionStats::default();

    while let Some((window, depth)) = stack.pop() {
        stats.node_visits += 1;
        stats.maximum_depth_reached = stats.maximum_depth_reached.max(depth);

        let moving_bounds = slab_swept_bounds(
            moving,
            moving_translation,
            window.start_parameter,
            window.end_parameter,
        )?;
        let obstacle_bounds = slab_swept_bounds(
            obstacle,
            obstacle_translation,
            window.start_parameter,
            window.end_parameter,
        )?;

        if !moving_bounds.overlaps(obstacle_bounds) {
            stats.pruned_intervals += 1;
            continue;
        }

        let half_width = window.width() * 0.5;
        let can_split = depth < config.maximum_depth && half_width >= config.minimum_window_width;
        if can_split {
            let midpoint = window.start_parameter + half_width;
            let left = NormalizedTimeWindow {
                start_parameter: window.start_parameter,
                end_parameter: midpoint,
            };
            let right = NormalizedTimeWindow {
                start_parameter: midpoint,
                end_parameter: window.end_parameter,
            };
            stats.split_intervals += 1;
            // Push right before left so the LIFO stack visits time in ascending order.
            stack.push((right, depth + 1));
            stack.push((left, depth + 1));
        } else {
            candidates.push(window);
        }
    }

    stats.candidate_window_count = candidates.len() as u32;
    let candidate_time_fraction = candidates.iter().map(|window| window.width()).sum::<f64>();
    // Rust's floating Sum identity can be negative zero for an empty iterator.
    // Normalize it so receipts have one canonical representation for no candidates.
    stats.candidate_time_fraction = if candidate_time_fraction == 0.0 {
        0.0
    } else {
        candidate_time_fraction
    };
    if !stats.candidate_time_fraction.is_finite() || stats.candidate_time_fraction > 1.0 + 1.0e-12 {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok((candidates, stats))
}

fn capsule_at(
    capsule: Capsule,
    translation: Vec3,
    time_parameter: f64,
) -> Result<Capsule, TranslationCcdError> {
    validate_translating_capsule(capsule, translation)?;
    if !time_parameter.is_finite() || !(0.0..=1.0).contains(&time_parameter) {
        return Err(TranslationCcdError::InvalidTimeWindow);
    }

    let shift = translation.scale(time_parameter);
    let start = capsule.start.add(shift);
    let end = capsule.end.add(shift);
    if !is_finite(start) || !is_finite(end) {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok(Capsule {
        start,
        end,
        radius_mm: capsule.radius_mm,
    })
}

fn validate_translating_capsule(
    capsule: Capsule,
    translation: Vec3,
) -> Result<(), TranslationCcdError> {
    if !is_finite(translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }
    if !capsule.radius_mm.is_finite() || capsule.radius_mm < 0.0 {
        return Err(TranslationCcdError::InvalidRadius);
    }
    if !is_finite(capsule.start) || !is_finite(capsule.end) {
        return Err(TranslationCcdError::NonFiniteCoordinate);
    }
    Ok(())
}

fn slab_swept_bounds(
    capsule: Capsule,
    translation: Vec3,
    t0: f64,
    t1: f64,
) -> Result<CapsuleAabb3, TranslationCcdError> {
    let shift0 = translation.scale(t0);
    let shift1 = translation.scale(t1);
    let start0 = capsule.start.add(shift0);
    let end0 = capsule.end.add(shift0);
    let start1 = capsule.start.add(shift1);
    let end1 = capsule.end.add(shift1);

    if !is_finite(start0) || !is_finite(end0) || !is_finite(start1) || !is_finite(end1) {
        return Err(TranslationCcdError::NumericalOverflow);
    }

    let bounds0 = Capsule {
        start: start0,
        end: end0,
        radius_mm: capsule.radius_mm,
    }
    .aabb();
    let bounds1 = Capsule {
        start: start1,
        end: end1,
        radius_mm: capsule.radius_mm,
    }
    .aabb();
    let bounds = CapsuleAabb3 {
        min: Vec3::new(
            bounds0.min.x.min(bounds1.min.x),
            bounds0.min.y.min(bounds1.min.y),
            bounds0.min.z.min(bounds1.min.z),
        ),
        max: Vec3::new(
            bounds0.max.x.max(bounds1.max.x),
            bounds0.max.y.max(bounds1.max.y),
            bounds0.max.z.max(bounds1.max.z),
        ),
    };
    if !is_finite(bounds.min) || !is_finite(bounds.max) {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok(bounds)
}

fn swept_bounds(capsule: Capsule, translation: Vec3) -> Result<CapsuleAabb3, TranslationCcdError> {
    if !is_finite(translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }

    if !capsule.radius_mm.is_finite() || capsule.radius_mm < 0.0 {
        return Err(TranslationCcdError::InvalidRadius);
    }
    if !is_finite(capsule.start) || !is_finite(capsule.end) {
        return Err(TranslationCcdError::NonFiniteCoordinate);
    }

    let final_start = capsule.start.add(translation);
    let final_end = capsule.end.add(translation);
    if !is_finite(final_start) || !is_finite(final_end) {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    let final_capsule = Capsule {
        start: final_start,
        end: final_end,
        radius_mm: capsule.radius_mm,
    };
    let initial = capsule.aabb();
    let final_bounds = final_capsule.aabb();
    let bounds = CapsuleAabb3 {
        min: Vec3::new(
            initial.min.x.min(final_bounds.min.x),
            initial.min.y.min(final_bounds.min.y),
            initial.min.z.min(final_bounds.min.z),
        ),
        max: Vec3::new(
            initial.max.x.max(final_bounds.max.x),
            initial.max.y.max(final_bounds.max.y),
            initial.max.z.max(final_bounds.max.z),
        ),
    };

    if !is_finite(bounds.min) || !is_finite(bounds.max) {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok(bounds)
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

    fn capsule(start: Vec3, end: Vec3, radius_mm: f64) -> Capsule {
        Capsule::new(start, end, radius_mm).unwrap()
    }

    #[test]
    fn adaptive_space_time_prunes_asynchronous_whole_sweep_overlap() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.25);
        let obstacle = capsule(p(6.0, 0.0, 0.0), p(6.0, 0.0, 0.0), 0.25);
        let shared_translation = p(10.0, 0.0, 0.0);

        assert!(
            co_translating_capsule_swept_aabb_candidate(
                moving,
                shared_translation,
                obstacle,
                shared_translation,
            )
            .unwrap()
        );

        let report = co_translating_capsule_adaptive_query(
            moving,
            shared_translation,
            obstacle,
            shared_translation,
            AdaptiveSubdivisionConfig {
                maximum_depth: 1,
                minimum_window_width: 0.25,
            },
        )
        .unwrap();

        assert_eq!(report.status, AdaptiveCollisionStatus::NoContact);
        assert!(report.candidate_windows.is_empty());
        assert_eq!(report.stats.node_visits, 3);
        assert_eq!(report.stats.pruned_intervals, 2);
        assert_eq!(report.stats.exact_narrow_phase_calls, 0);
        assert_eq!(report.stats.candidate_time_fraction, 0.0);
        assert!(!report.stats.candidate_time_fraction.is_sign_negative());
    }

    #[test]
    fn adaptive_space_time_finds_mid_interval_contact_at_split_boundary() {
        let moving = capsule(p(-10.0, -2.0, 0.0), p(-10.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, -2.0, 0.0), p(10.0, 2.0, 0.0), 0.5);
        let report = co_translating_capsule_adaptive_query(
            moving,
            p(20.0, 0.0, 0.0),
            obstacle,
            p(-20.0, 0.0, 0.0),
            AdaptiveSubdivisionConfig {
                maximum_depth: 4,
                minimum_window_width: 1.0 / 16.0,
            },
        )
        .unwrap();

        assert_eq!(report.status, AdaptiveCollisionStatus::Contact);
        let witness = report.witness.unwrap();
        assert!((witness.time_parameter - 0.5).abs() < 1.0e-9);
        assert!(report.witness_window.unwrap().contains_inclusive(0.5));
        assert!(report.stats.candidate_window_count >= 1);
        assert!(report.stats.exact_narrow_phase_calls >= 1);
    }

    #[test]
    fn interval_narrow_phase_remaps_local_time_to_global_time() {
        let moving = capsule(p(-10.0, -2.0, 0.0), p(-10.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, -2.0, 0.0), p(10.0, 2.0, 0.0), 0.5);
        let closest = co_translating_capsule_vs_capsule_in_time_window(
            moving,
            p(20.0, 0.0, 0.0),
            obstacle,
            p(-20.0, 0.0, 0.0),
            NormalizedTimeWindow {
                start_parameter: 0.25,
                end_parameter: 0.75,
            },
        )
        .unwrap();

        assert!((closest.time_parameter - 0.5).abs() < 1.0e-9);
        assert!(closest.distance_mm <= CONTACT_EPSILON_MM);
    }

    #[test]
    fn adaptive_no_contact_requires_every_candidate_leaf_to_be_clear() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(1_000.0, 0.0, 0.0), 0.25);
        let obstacle = capsule(p(0.0, 0.45, 0.45), p(1_000.0, 0.450_5, 0.45), 0.25);
        let report = co_translating_capsule_adaptive_query(
            moving,
            p(0.0, 0.0, 0.0),
            obstacle,
            p(0.0, 0.0, 0.0),
            AdaptiveSubdivisionConfig {
                maximum_depth: 0,
                minimum_window_width: 1.0,
            },
        )
        .unwrap();

        assert_eq!(report.status, AdaptiveCollisionStatus::Unresolved);
        assert_eq!(report.stats.candidate_window_count, 1);
        assert_eq!(report.stats.unresolved_candidate_count, 1);
        assert_eq!(report.stats.exact_narrow_phase_calls, 1);
    }

    #[test]
    fn adaptive_minimum_width_and_maximum_depth_are_both_respected() {
        let moving = capsule(p(-10.0, -2.0, 0.0), p(-10.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, -2.0, 0.0), p(10.0, 2.0, 0.0), 0.5);
        let (windows, stats) = collect_adaptive_candidate_windows(
            moving,
            p(20.0, 0.0, 0.0),
            obstacle,
            p(-20.0, 0.0, 0.0),
            AdaptiveSubdivisionConfig {
                maximum_depth: 20,
                minimum_window_width: 0.25,
            },
        )
        .unwrap();

        assert_eq!(stats.maximum_depth_reached, 2);
        assert!(
            windows
                .iter()
                .all(|window| (window.width() - 0.25).abs() < 1.0e-12)
        );
        assert_eq!(
            windows.iter().map(|window| window.width()).sum::<f64>(),
            stats.candidate_time_fraction
        );
    }

    #[test]
    fn adaptive_refinement_rejects_invalid_limits_and_time_windows() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(6.0, 0.0, 0.0), 0.5);

        assert_eq!(
            co_translating_capsule_adaptive_query(
                moving,
                p(0.0, 0.0, 0.0),
                obstacle,
                p(0.0, 0.0, 0.0),
                AdaptiveSubdivisionConfig {
                    maximum_depth: 21,
                    minimum_window_width: 0.01,
                },
            ),
            Err(TranslationCcdError::InvalidAdaptiveDepth)
        );
        assert_eq!(
            co_translating_capsule_adaptive_query(
                moving,
                p(0.0, 0.0, 0.0),
                obstacle,
                p(0.0, 0.0, 0.0),
                AdaptiveSubdivisionConfig {
                    maximum_depth: 2,
                    minimum_window_width: 0.0,
                },
            ),
            Err(TranslationCcdError::InvalidMinimumWindowWidth)
        );
        assert_eq!(
            co_translating_capsule_vs_capsule_in_time_window(
                moving,
                p(0.0, 0.0, 0.0),
                obstacle,
                p(0.0, 0.0, 0.0),
                NormalizedTimeWindow {
                    start_parameter: 0.75,
                    end_parameter: 0.25,
                },
            ),
            Err(TranslationCcdError::InvalidTimeWindow)
        );
    }

    #[test]
    fn generated_ten_thousand_contacts_are_never_pruned_by_adaptive_space_time() {
        let mut seed = 0x0005_1ace_2026_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 2_001) as f64 / 100.0 - 10.0
        };

        for case_index in 0..10_000 {
            let start = p(next(), next(), next());
            let axis = p(next(), next(), next());
            let end = start.add(axis);
            let moving_translation = p(next(), next(), next());
            let obstacle_translation = p(next(), next(), next());
            let contact_time = 0.013 + (f64::from(case_index % 9_731) / 9_731.0) * 0.974;
            let relative_translation = moving_translation.sub(obstacle_translation);
            let obstacle_start = start.add(relative_translation.scale(contact_time));
            let obstacle_end = end.add(relative_translation.scale(contact_time));
            let moving = capsule(start, end, 0.25);
            let obstacle = capsule(obstacle_start, obstacle_end, 0.25);

            let (windows, _) = collect_adaptive_candidate_windows(
                moving,
                moving_translation,
                obstacle,
                obstacle_translation,
                AdaptiveSubdivisionConfig {
                    maximum_depth: 6,
                    minimum_window_width: 1.0 / 64.0,
                },
            )
            .unwrap();

            assert!(
                windows
                    .iter()
                    .any(|window| window.contains_inclusive(contact_time)),
                "contact case {case_index} at t={contact_time} was pruned"
            );
        }
    }

    #[test]
    fn detects_head_on_mid_interval_contact_when_both_endpoints_are_clear() {
        let moving = capsule(p(-10.0, -2.0, 0.0), p(-10.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, -2.0, 0.0), p(10.0, 2.0, 0.0), 0.5);
        let moving_delta = p(20.0, 0.0, 0.0);
        let obstacle_delta = p(-20.0, 0.0, 0.0);

        assert!(
            !endpoints_only_co_translating_capsule_collision(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
            )
            .unwrap()
        );
        assert!(
            co_translating_capsule_swept_aabb_candidate(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
            )
            .unwrap()
        );
        assert!(
            co_translating_capsule_capsule_collision(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
            )
            .unwrap()
        );

        let closest =
            co_translating_capsule_vs_capsule(moving, moving_delta, obstacle, obstacle_delta)
                .unwrap();
        assert!((closest.time_parameter - 0.5).abs() < 1.0e-9);
        assert!(closest.distance_mm <= CONTACT_EPSILON_MM);
        assert!((closest.moving_point.x - closest.obstacle_point.x).abs() < 1.0e-9);
    }

    #[test]
    fn equal_translation_preserves_static_separation() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 5.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 5.0, 0.0), 0.5);
        let common_delta = p(100.0, -30.0, 4.0);
        let closest =
            co_translating_capsule_vs_capsule(moving, common_delta, obstacle, common_delta)
                .unwrap();

        assert!((closest.distance_mm - 5.0).abs() < 1.0e-9);
        assert_eq!(closest.time_parameter, 0.0);
        assert!(!closest.is_contact(moving.radius_mm, obstacle.radius_mm));
    }

    #[test]
    fn different_same_direction_speeds_use_relative_motion() {
        let moving = capsule(p(0.0, -1.0, 0.0), p(0.0, 1.0, 0.0), 0.25);
        let obstacle = capsule(p(10.0, -1.0, 0.0), p(10.0, 1.0, 0.0), 0.25);
        let closest = co_translating_capsule_vs_capsule(
            moving,
            p(20.0, 0.0, 0.0),
            obstacle,
            p(5.0, 0.0, 0.0),
        )
        .unwrap();

        assert!((closest.time_parameter - (2.0 / 3.0)).abs() < 1.0e-9);
        assert!(closest.distance_mm <= CONTACT_EPSILON_MM);
    }

    #[test]
    fn exact_tangency_at_final_time_is_contact() {
        let moving = capsule(p(-10.0, 0.0, 0.0), p(-10.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, 0.0, 0.0), p(10.0, 0.0, 0.0), 0.5);
        let closest = co_translating_capsule_vs_capsule(
            moving,
            p(9.5, 0.0, 0.0),
            obstacle,
            p(-9.5, 0.0, 0.0),
        )
        .unwrap();

        assert!((closest.time_parameter - 1.0).abs() < 1.0e-9);
        assert!((closest.distance_mm - 1.0).abs() < 1.0e-9);
        assert!(closest.is_contact(0.5, 0.5));
    }

    #[test]
    fn synchronized_near_miss_can_have_overlapping_swept_aabbs() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.25);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 0.0, 0.0), 0.25);
        let shared_delta = p(10.0, 0.0, 0.0);

        assert!(
            co_translating_capsule_swept_aabb_candidate(
                moving,
                shared_delta,
                obstacle,
                shared_delta,
            )
            .unwrap()
        );
        let closest =
            co_translating_capsule_vs_capsule(moving, shared_delta, obstacle, shared_delta)
                .unwrap();
        assert!((closest.distance_mm - 5.0).abs() < 1.0e-9);
        assert!(!closest.is_contact(0.25, 0.25));
    }

    #[test]
    fn time_sliced_broad_phase_prunes_asynchronous_swept_aabb_overlap() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.25);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 0.0, 0.0), 0.25);
        let shared_delta = p(10.0, 0.0, 0.0);

        assert!(
            co_translating_capsule_swept_aabb_candidate(
                moving,
                shared_delta,
                obstacle,
                shared_delta,
            )
            .unwrap()
        );

        let one = co_translating_capsule_time_sliced_aabb_candidates(
            moving,
            shared_delta,
            obstacle,
            shared_delta,
            1,
        )
        .unwrap();
        let two = co_translating_capsule_time_sliced_aabb_candidates(
            moving,
            shared_delta,
            obstacle,
            shared_delta,
            2,
        )
        .unwrap();
        let four = co_translating_capsule_time_sliced_aabb_candidates(
            moving,
            shared_delta,
            obstacle,
            shared_delta,
            4,
        )
        .unwrap();
        let eight = co_translating_capsule_time_sliced_aabb_candidates(
            moving,
            shared_delta,
            obstacle,
            shared_delta,
            8,
        )
        .unwrap();

        assert!(one.has_candidate());
        assert!(two.has_candidate());
        assert!(!four.has_candidate());
        assert!(!eight.has_candidate());
        assert!(!co_translating_capsule_capsule_collision(
            moving, shared_delta, obstacle, shared_delta,
        )
        .unwrap());
    }

    #[test]
    fn time_sliced_broad_phase_never_drops_mid_interval_contact() {
        let moving = capsule(p(-10.0, -2.0, 0.0), p(-10.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, -2.0, 0.0), p(10.0, 2.0, 0.0), 0.5);
        let moving_delta = p(20.0, 0.0, 0.0);
        let obstacle_delta = p(-20.0, 0.0, 0.0);

        assert!(
            co_translating_capsule_capsule_collision(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
            )
            .unwrap()
        );
        for subdivisions in [1, 2, 4, 8, 16, 32] {
            let candidates = co_translating_capsule_time_sliced_aabb_candidates(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
                subdivisions,
            )
            .unwrap();
            assert!(candidates.has_candidate(), "subdivisions={subdivisions}");
        }
    }

    #[test]
    fn time_sliced_broad_phase_is_monotone_under_dyadic_refinement() {
        let moving = capsule(p(0.0, -1.0, 0.0), p(0.0, 1.0, 0.0), 0.2);
        let obstacle = capsule(p(10.0, -1.0, 2.0), p(10.0, 1.0, 2.0), 0.2);
        let moving_delta = p(12.0, 3.0, 0.0);
        let obstacle_delta = p(-3.0, -1.0, 0.0);
        let counts: Vec<(u32, u32)> = [1, 2, 4, 8, 16]
            .into_iter()
            .map(|n| {
                let result = co_translating_capsule_time_sliced_aabb_candidates(
                    moving,
                    moving_delta,
                    obstacle,
                    obstacle_delta,
                    n,
                )
                .unwrap();
                (n, result.candidate_slab_count)
            })
            .collect();
        assert!(counts.windows(2).all(|pair| {
            u64::from(pair[1].1) * u64::from(pair[0].0)
                <= u64::from(pair[0].1) * u64::from(pair[1].0)
        }));
    }

    #[test]
    fn time_sliced_broad_phase_never_drops_deterministic_generated_contacts() {
        let mut seed = 0xa117_5eed_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 2_001) as f64 - 1_000.0
        };

        for case_index in 0..64 {
            let moving = capsule(p(next(), next(), next()), p(next(), next(), next()), 0.5);
            let obstacle = capsule(p(next(), next(), next()), p(next(), next(), next()), 0.5);
            let moving_delta = p(next(), next(), next());
            let obstacle_delta = p(next(), next(), next());
            let closest =
                co_translating_capsule_vs_capsule(moving, moving_delta, obstacle, obstacle_delta)
                    .unwrap();

            if closest.is_contact(moving.radius_mm, obstacle.radius_mm) {
                for subdivisions in [1, 2, 4, 8, 16, 32, 64] {
                    let candidates = co_translating_capsule_time_sliced_aabb_candidates(
                        moving,
                        moving_delta,
                        obstacle,
                        obstacle_delta,
                        subdivisions,
                    )
                    .unwrap();
                    assert!(
                        candidates.has_candidate(),
                        "case={case_index} subdivisions={subdivisions}"
                    );
                }
            }
        }
    }

    #[test]
    fn time_sliced_broad_phase_rejects_zero_subdivisions() {
        let valid = capsule(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), 0.5);
        for subdivisions in [0, MAX_TIME_SUBDIVISIONS + 1] {
            assert_eq!(
                co_translating_capsule_time_sliced_aabb_candidates(
                    valid,
                    p(1.0, 0.0, 0.0),
                    valid,
                    p(-1.0, 0.0, 0.0),
                    subdivisions,
                ),
                Err(TranslationCcdError::InvalidSubdivisionCount)
            );
        }
    }

    #[test]
    fn endpoint_clamping_and_skew_three_dimensional_axes_are_supported() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(4.0, 0.0, 0.0), 0.2);
        let obstacle = capsule(p(2.0, 0.0, 3.0), p(2.0, 0.0, 7.0), 0.2);
        let closest =
            co_translating_capsule_vs_capsule(moving, p(0.0, 0.0, 0.0), obstacle, p(0.0, 0.0, 0.0))
                .unwrap();
        assert!((closest.distance_mm - 3.0).abs() < 1.0e-9);
        assert!((0.0..=1.0).contains(&closest.moving_axis_parameter));
        assert!((0.0..=1.0).contains(&closest.obstacle_axis_parameter));
    }

    #[test]
    fn swapping_capsules_swaps_closest_points_and_axis_parameters() {
        let moving = capsule(p(-5.0, -1.0, 0.0), p(-5.0, 1.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, -2.0, 1.0), p(5.0, 2.0, 1.0), 0.25);
        let moving_delta = p(12.0, 0.0, 0.0);
        let obstacle_delta = p(-2.0, 0.0, 0.0);
        let ab = co_translating_capsule_vs_capsule(moving, moving_delta, obstacle, obstacle_delta)
            .unwrap();
        let ba = co_translating_capsule_vs_capsule(obstacle, obstacle_delta, moving, moving_delta)
            .unwrap();

        assert!((ab.distance_mm - ba.distance_mm).abs() < 1.0e-9);
        assert!((ab.time_parameter - ba.time_parameter).abs() < 1.0e-9);
        assert!((ab.moving_point.distance(ba.obstacle_point)).abs() < 1.0e-9);
        assert!((ab.obstacle_point.distance(ba.moving_point)).abs() < 1.0e-9);
        assert!((ab.moving_axis_parameter - ba.obstacle_axis_parameter).abs() < 1.0e-9);
        assert!((ab.obstacle_axis_parameter - ba.moving_axis_parameter).abs() < 1.0e-9);
    }

    #[test]
    fn rigid_world_transform_preserves_closest_distance() {
        let moving = capsule(p(-4.0, -2.0, 0.0), p(4.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(0.0, -3.0, 3.0), p(0.0, 3.0, 3.0), 0.25);
        let moving_delta = p(0.0, 0.0, 2.0);
        let obstacle_delta = p(0.0, 0.0, -1.0);
        let original =
            co_translating_capsule_vs_capsule(moving, moving_delta, obstacle, obstacle_delta)
                .unwrap();
        let transform = |v: Vec3| p(100.0 - v.y, -30.0 + v.x, 25.0 + v.z);
        let rotate_delta = |v: Vec3| p(-v.y, v.x, v.z);
        let transformed = co_translating_capsule_vs_capsule(
            capsule(
                transform(moving.start),
                transform(moving.end),
                moving.radius_mm,
            ),
            rotate_delta(moving_delta),
            capsule(
                transform(obstacle.start),
                transform(obstacle.end),
                obstacle.radius_mm,
            ),
            rotate_delta(obstacle_delta),
        )
        .unwrap();

        assert!((original.distance_mm - transformed.distance_mm).abs() < 1.0e-9);
        assert!((original.time_parameter - transformed.time_parameter).abs() < 1.0e-9);
    }

    #[test]
    fn uniform_scaling_preserves_contact_truth() {
        let moving = capsule(p(-5.0, 0.0, 0.0), p(-5.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 2.0, 0.0), 0.5);
        let moving_delta = p(10.0, 0.0, 0.0);
        let obstacle_delta = p(-10.0, 0.0, 0.0);
        let base =
            co_translating_capsule_vs_capsule(moving, moving_delta, obstacle, obstacle_delta)
                .unwrap();
        let scale = 8.0;
        let scaled = co_translating_capsule_vs_capsule(
            capsule(
                moving.start.scale(scale),
                moving.end.scale(scale),
                moving.radius_mm * scale,
            ),
            moving_delta.scale(scale),
            capsule(
                obstacle.start.scale(scale),
                obstacle.end.scale(scale),
                obstacle.radius_mm * scale,
            ),
            obstacle_delta.scale(scale),
        )
        .unwrap();

        assert!((scaled.distance_mm - base.distance_mm * scale).abs() < 1.0e-8);
        assert_eq!(
            base.is_contact(moving.radius_mm, obstacle.radius_mm),
            scaled.is_contact(moving.radius_mm * scale, obstacle.radius_mm * scale)
        );
    }

    #[test]
    fn rejects_nonfinite_translations_and_invalid_radii() {
        let valid = capsule(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), 0.5);
        assert_eq!(
            co_translating_capsule_vs_capsule(
                valid,
                p(0.0, f64::NAN, 0.0),
                valid,
                p(0.0, 0.0, 0.0),
            ),
            Err(TranslationCcdError::NonFiniteTranslation)
        );
        let invalid = Capsule {
            radius_mm: -1.0,
            ..valid
        };
        assert_eq!(
            co_translating_capsule_vs_capsule(invalid, p(0.0, 0.0, 0.0), valid, p(0.0, 0.0, 0.0),),
            Err(TranslationCcdError::InvalidRadius)
        );
    }

    #[test]
    fn generated_results_are_never_worse_than_synchronized_time_samples() {
        let mut seed = 0x71ab_902d_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 401) as f64 - 200.0
        };

        for _ in 0..20 {
            let moving = capsule(p(next(), next(), next()), p(next(), next(), next()), 0.5);
            let obstacle = capsule(p(next(), next(), next()), p(next(), next(), next()), 0.5);
            let moving_delta = p(next(), next(), next());
            let obstacle_delta = p(next(), next(), next());
            let exact =
                co_translating_capsule_vs_capsule(moving, moving_delta, obstacle, obstacle_delta)
                    .unwrap();

            let mut sampled_best = f64::INFINITY;
            for ti in 0..=16 {
                let t = f64::from(ti) / 16.0;
                let moving_start = moving.start.add(moving_delta.scale(t));
                let moving_end = moving.end.add(moving_delta.scale(t));
                let obstacle_start = obstacle.start.add(obstacle_delta.scale(t));
                let obstacle_end = obstacle.end.add(obstacle_delta.scale(t));
                for ui in 0..=8 {
                    let u = f64::from(ui) / 8.0;
                    let moving_point = moving_start.add(moving_end.sub(moving_start).scale(u));
                    for vi in 0..=8 {
                        let v = f64::from(vi) / 8.0;
                        let obstacle_point =
                            obstacle_start.add(obstacle_end.sub(obstacle_start).scale(v));
                        sampled_best = sampled_best.min(moving_point.distance(obstacle_point));
                    }
                }
            }
            assert!(
                exact.distance_mm <= sampled_best + 1.0e-8,
                "analytic={} sampled={sampled_best}",
                exact.distance_mm
            );
        }
    }
}


/// One terminal interval in a replayable adaptive collision certificate.
#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveProofLeaf {
    pub window: NormalizedTimeWindow,
    pub depth: u8,
    pub disposition: AdaptiveProofLeafDisposition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AdaptiveProofLeafDisposition {
    /// This interval was safely removed by disjoint conservative swept AABBs.
    Pruned {
        moving_bounds: CapsuleAabb3,
        obstacle_bounds: CapsuleAabb3,
    },
    /// The exact restricted-window analytic query established separation.
    CheckedClear {
        closest: CoTranslatingCapsuleClosestApproach,
    },
    /// The exact restricted-window analytic query established contact.
    CheckedContact {
        closest: CoTranslatingCapsuleClosestApproach,
    },
    /// The analytic query declined to classify this candidate interval.
    Unresolved {
        error: TranslationCcdError,
    },
}

/// Complete typed input snapshot and terminal-interval ledger for one adaptive query.
///
/// The certificate is replayable but not cryptographically authenticated. The
/// caller must bind its inputs to the relevant design, site epoch, robot/tool,
/// calibration and controller identities in the surrounding evidence system.
#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveCollisionCertificate {
    pub moving: Capsule,
    pub moving_translation: Vec3,
    pub obstacle: Capsule,
    pub obstacle_translation: Vec3,
    pub config: AdaptiveSubdivisionConfig,
    /// Sorted, complete, gap-free terminal partition of the normalized domain.
    pub leaves: Vec<AdaptiveProofLeaf>,
    /// The summary derived from the terminal ledger.
    pub report: AdaptiveCollisionReport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdaptiveCertificateError {
    Geometry(TranslationCcdError),
    EmptyCertificate,
    InvalidCoverage,
    InvalidLeafOrder,
    InvalidDyadicWindow,
    InvalidLeafDepth,
    CandidateLeafNotTerminal,
    PruneBoundsMismatch,
    PruneNotDisjoint,
    CandidateBoundsDisjoint,
    NarrowPhaseClaimMismatch,
    OutcomeMismatch,
    StatisticsMismatch,
    LeafLimitExceeded,
}

const MAX_ADAPTIVE_CERTIFICATE_LEAVES: usize = 65_536;

/// Run adaptive branch-and-bound and retain every terminal interval.
///
/// Unlike the existing latency-oriented API, this certificate-producing
/// variant evaluates every surviving candidate leaf, even after a contact
/// witness is found. This makes the ledger complete and independently
/// replayable. Use the existing API when early contact termination is preferred.
pub fn co_translating_capsule_adaptive_query_with_certificate(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    config: AdaptiveSubdivisionConfig,
) -> Result<AdaptiveCollisionCertificate, AdaptiveCertificateError> {
    config
        .validate()
        .map_err(AdaptiveCertificateError::Geometry)?;
    validate_translating_capsule(moving, moving_translation)
        .map_err(AdaptiveCertificateError::Geometry)?;
    validate_translating_capsule(obstacle, obstacle_translation)
        .map_err(AdaptiveCertificateError::Geometry)?;

    let root = NormalizedTimeWindow {
        start_parameter: 0.0,
        end_parameter: 1.0,
    };
    let mut stack = vec![(root, 0_u8)];
    let mut leaves = Vec::new();

    while let Some((window, depth)) = stack.pop() {
        let moving_bounds = slab_swept_bounds(
            moving,
            moving_translation,
            window.start_parameter,
            window.end_parameter,
        )
        .map_err(AdaptiveCertificateError::Geometry)?;
        let obstacle_bounds = slab_swept_bounds(
            obstacle,
            obstacle_translation,
            window.start_parameter,
            window.end_parameter,
        )
        .map_err(AdaptiveCertificateError::Geometry)?;

        if !moving_bounds.overlaps(obstacle_bounds) {
            leaves.push(AdaptiveProofLeaf {
                window,
                depth,
                disposition: AdaptiveProofLeafDisposition::Pruned {
                    moving_bounds,
                    obstacle_bounds,
                },
            });
        } else {
            let half_width = window.width() * 0.5;
            let can_split =
                depth < config.maximum_depth && half_width >= config.minimum_window_width;
            if can_split {
                if leaves.len() + stack.len() + 2 > MAX_ADAPTIVE_CERTIFICATE_LEAVES {
                    return Err(AdaptiveCertificateError::LeafLimitExceeded);
                }
                let midpoint = window.start_parameter + half_width;
                let left = NormalizedTimeWindow {
                    start_parameter: window.start_parameter,
                    end_parameter: midpoint,
                };
                let right = NormalizedTimeWindow {
                    start_parameter: midpoint,
                    end_parameter: window.end_parameter,
                };
                // Right is pushed first so the LIFO traversal visits chronological order.
                stack.push((right, depth + 1));
                stack.push((left, depth + 1));
            } else {
                let disposition = match co_translating_capsule_vs_capsule_in_time_window(
                    moving,
                    moving_translation,
                    obstacle,
                    obstacle_translation,
                    window,
                ) {
                    Ok(closest) => {
                        if closest.is_contact(moving.radius_mm, obstacle.radius_mm) {
                            AdaptiveProofLeafDisposition::CheckedContact { closest }
                        } else {
                            AdaptiveProofLeafDisposition::CheckedClear { closest }
                        }
                    }
                    Err(TranslationCcdError::NumericallyUnresolved) => {
                        AdaptiveProofLeafDisposition::Unresolved {
                            error: TranslationCcdError::NumericallyUnresolved,
                        }
                    }
                    Err(error) => return Err(AdaptiveCertificateError::Geometry(error)),
                };
                leaves.push(AdaptiveProofLeaf {
                    window,
                    depth,
                    disposition,
                });
            }
        }
    }

    leaves.sort_by(|left, right| {
        left.window
            .start_parameter
            .total_cmp(&right.window.start_parameter)
    });
    let report = summarize_certificate_leaves(&leaves);
    Ok(AdaptiveCollisionCertificate {
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
        config,
        leaves,
        report,
    })
}

/// Independently replay all bounds, interval coverage and narrow-phase claims.
///
/// A successful verification means the terminal ledger is a complete
/// dyadic partition, every pruned interval has disjoint recomputed outer
/// bounds, every checked leaf agrees with a fresh analytic query, and the
/// aggregate outcome/statistics match the leaf ledger.
pub fn verify_adaptive_collision_certificate(
    certificate: &AdaptiveCollisionCertificate,
) -> Result<(), AdaptiveCertificateError> {
    certificate
        .config
        .validate()
        .map_err(AdaptiveCertificateError::Geometry)?;
    validate_translating_capsule(certificate.moving, certificate.moving_translation)
        .map_err(AdaptiveCertificateError::Geometry)?;
    validate_translating_capsule(certificate.obstacle, certificate.obstacle_translation)
        .map_err(AdaptiveCertificateError::Geometry)?;

    if certificate.leaves.is_empty() {
        return Err(AdaptiveCertificateError::EmptyCertificate);
    }
    if certificate.leaves[0].window.start_parameter != 0.0
        || certificate.leaves.last().map(|leaf| leaf.window.end_parameter) != Some(1.0)
    {
        return Err(AdaptiveCertificateError::InvalidCoverage);
    }

    for (index, leaf) in certificate.leaves.iter().enumerate() {
        leaf.window
            .validate()
            .map_err(|_| AdaptiveCertificateError::InvalidCoverage)?;
        if leaf.depth > certificate.config.maximum_depth {
            return Err(AdaptiveCertificateError::InvalidLeafDepth);
        }
        let scale = (1_u64 << leaf.depth) as f64;
        let scaled_start = leaf.window.start_parameter * scale;
        let scaled_end = leaf.window.end_parameter * scale;
        if scaled_start.fract() != 0.0
            || scaled_end.fract() != 0.0
            || leaf.window.width() != 1.0 / scale
        {
            return Err(AdaptiveCertificateError::InvalidDyadicWindow);
        }
        if index > 0 {
            let previous = &certificate.leaves[index - 1];
            if previous.window.start_parameter >= leaf.window.start_parameter {
                return Err(AdaptiveCertificateError::InvalidLeafOrder);
            }
            if previous.window.end_parameter != leaf.window.start_parameter {
                return Err(AdaptiveCertificateError::InvalidCoverage);
            }
        }

        let moving_bounds = slab_swept_bounds(
            certificate.moving,
            certificate.moving_translation,
            leaf.window.start_parameter,
            leaf.window.end_parameter,
        )
        .map_err(AdaptiveCertificateError::Geometry)?;
        let obstacle_bounds = slab_swept_bounds(
            certificate.obstacle,
            certificate.obstacle_translation,
            leaf.window.start_parameter,
            leaf.window.end_parameter,
        )
        .map_err(AdaptiveCertificateError::Geometry)?;

        match &leaf.disposition {
            AdaptiveProofLeafDisposition::Pruned {
                moving_bounds: claimed_moving,
                obstacle_bounds: claimed_obstacle,
            } => {
                if *claimed_moving != moving_bounds || *claimed_obstacle != obstacle_bounds {
                    return Err(AdaptiveCertificateError::PruneBoundsMismatch);
                }
                if moving_bounds.overlaps(obstacle_bounds) {
                    return Err(AdaptiveCertificateError::PruneNotDisjoint);
                }
            }
            AdaptiveProofLeafDisposition::CheckedClear { closest } => {
                verify_candidate_leaf_is_terminal(leaf, certificate.config)?;
                if !moving_bounds.overlaps(obstacle_bounds) {
                    return Err(AdaptiveCertificateError::CandidateBoundsDisjoint);
                }
                let actual = co_translating_capsule_vs_capsule_in_time_window(
                    certificate.moving,
                    certificate.moving_translation,
                    certificate.obstacle,
                    certificate.obstacle_translation,
                    leaf.window,
                )
                .map_err(AdaptiveCertificateError::Geometry)?;
                if actual != *closest
                    || actual.is_contact(
                        certificate.moving.radius_mm,
                        certificate.obstacle.radius_mm,
                    )
                {
                    return Err(AdaptiveCertificateError::NarrowPhaseClaimMismatch);
                }
            }
            AdaptiveProofLeafDisposition::CheckedContact { closest } => {
                verify_candidate_leaf_is_terminal(leaf, certificate.config)?;
                if !moving_bounds.overlaps(obstacle_bounds) {
                    return Err(AdaptiveCertificateError::CandidateBoundsDisjoint);
                }
                let actual = co_translating_capsule_vs_capsule_in_time_window(
                    certificate.moving,
                    certificate.moving_translation,
                    certificate.obstacle,
                    certificate.obstacle_translation,
                    leaf.window,
                )
                .map_err(AdaptiveCertificateError::Geometry)?;
                if actual != *closest
                    || !actual.is_contact(
                        certificate.moving.radius_mm,
                        certificate.obstacle.radius_mm,
                    )
                {
                    return Err(AdaptiveCertificateError::NarrowPhaseClaimMismatch);
                }
            }
            AdaptiveProofLeafDisposition::Unresolved { error } => {
                verify_candidate_leaf_is_terminal(leaf, certificate.config)?;
                if *error != TranslationCcdError::NumericallyUnresolved
                    || !moving_bounds.overlaps(obstacle_bounds)
                {
                    return Err(AdaptiveCertificateError::CandidateBoundsDisjoint);
                }
                match co_translating_capsule_vs_capsule_in_time_window(
                    certificate.moving,
                    certificate.moving_translation,
                    certificate.obstacle,
                    certificate.obstacle_translation,
                    leaf.window,
                ) {
                    Err(TranslationCcdError::NumericallyUnresolved) => {}
                    _ => return Err(AdaptiveCertificateError::NarrowPhaseClaimMismatch),
                }
            }
        }
    }

    let expected = summarize_certificate_leaves(&certificate.leaves);
    if expected.status != certificate.report.status
        || expected.witness != certificate.report.witness
        || expected.witness_window != certificate.report.witness_window
        || expected.candidate_windows != certificate.report.candidate_windows
        || expected.unresolved_windows != certificate.report.unresolved_windows
    {
        return Err(AdaptiveCertificateError::OutcomeMismatch);
    }
    if expected.stats != certificate.report.stats {
        return Err(AdaptiveCertificateError::StatisticsMismatch);
    }
    Ok(())
}

fn verify_candidate_leaf_is_terminal(
    leaf: &AdaptiveProofLeaf,
    config: AdaptiveSubdivisionConfig,
) -> Result<(), AdaptiveCertificateError> {
    let can_split = leaf.depth < config.maximum_depth
        && leaf.window.width() * 0.5 >= config.minimum_window_width;
    if can_split {
        return Err(AdaptiveCertificateError::CandidateLeafNotTerminal);
    }
    Ok(())
}

fn summarize_certificate_leaves(leaves: &[AdaptiveProofLeaf]) -> AdaptiveCollisionReport {
    let mut candidate_windows = Vec::new();
    let mut unresolved_windows = Vec::new();
    let mut witness = None;
    let mut witness_window = None;
    let mut pruned = 0_u32;
    let mut exact_calls = 0_u32;
    let mut unresolved_count = 0_u32;
    let mut candidate_time_fraction = 0.0_f64;
    let mut maximum_depth = 0_u8;

    for leaf in leaves {
        maximum_depth = maximum_depth.max(leaf.depth);
        match leaf.disposition {
            AdaptiveProofLeafDisposition::Pruned { .. } => {
                pruned += 1;
            }
            AdaptiveProofLeafDisposition::CheckedClear { .. } => {
                candidate_windows.push(leaf.window);
                candidate_time_fraction += leaf.window.width();
                exact_calls += 1;
            }
            AdaptiveProofLeafDisposition::CheckedContact { closest } => {
                candidate_windows.push(leaf.window);
                candidate_time_fraction += leaf.window.width();
                exact_calls += 1;
                if witness.is_none() {
                    witness = Some(closest);
                    witness_window = Some(leaf.window);
                }
            }
            AdaptiveProofLeafDisposition::Unresolved { .. } => {
                candidate_windows.push(leaf.window);
                unresolved_windows.push(leaf.window);
                candidate_time_fraction += leaf.window.width();
                exact_calls += 1;
                unresolved_count += 1;
            }
        }
    }

    let status = if witness.is_some() {
        AdaptiveCollisionStatus::Contact
    } else if unresolved_windows.is_empty() {
        AdaptiveCollisionStatus::NoContact
    } else {
        AdaptiveCollisionStatus::Unresolved
    };
    if candidate_time_fraction == 0.0 {
        candidate_time_fraction = 0.0;
    }
    let leaf_count = leaves.len() as u32;
    AdaptiveCollisionReport {
        status,
        witness,
        witness_window,
        candidate_windows,
        unresolved_windows,
        stats: AdaptiveSubdivisionStats {
            node_visits: leaf_count.saturating_mul(2).saturating_sub(1),
            split_intervals: leaf_count.saturating_sub(1),
            pruned_intervals: pruned,
            candidate_window_count: exact_calls,
            exact_narrow_phase_calls: exact_calls,
            unresolved_candidate_count: unresolved_count,
            maximum_depth_reached: maximum_depth,
            candidate_time_fraction,
        },
    }
}

#[cfg(test)]
mod adaptive_certificate_tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn point_capsule(x: f64, radius_mm: f64) -> Capsule {
        Capsule::new(p(x, 0.0, 0.0), p(x, 0.0, 0.0), radius_mm).unwrap()
    }

    fn config() -> AdaptiveSubdivisionConfig {
        AdaptiveSubdivisionConfig {
            maximum_depth: 4,
            minimum_window_width: 0.0625,
        }
    }

    #[test]
    fn complete_no_contact_certificate_replays() {
        let certificate = co_translating_capsule_adaptive_query_with_certificate(
            point_capsule(0.0, 0.5),
            p(0.0, 0.0, 0.0),
            point_capsule(10.0, 0.5),
            p(0.0, 0.0, 0.0),
            config(),
        )
        .unwrap();
        assert_eq!(certificate.report.status, AdaptiveCollisionStatus::NoContact);
        assert_eq!(certificate.leaves.len(), 1);
        assert!(matches!(
            certificate.leaves[0].disposition,
            AdaptiveProofLeafDisposition::Pruned { .. }
        ));
        verify_adaptive_collision_certificate(&certificate).unwrap();
    }

    #[test]
    fn mid_interval_contact_certificate_replays_and_contains_witness() {
        let certificate = co_translating_capsule_adaptive_query_with_certificate(
            point_capsule(-10.0, 0.5),
            p(20.0, 0.0, 0.0),
            point_capsule(10.0, 0.5),
            p(-20.0, 0.0, 0.0),
            config(),
        )
        .unwrap();
        assert_eq!(certificate.report.status, AdaptiveCollisionStatus::Contact);
        assert!(certificate.report.witness.is_some());
        assert!(certificate.leaves.iter().any(|leaf| matches!(
            leaf.disposition,
            AdaptiveProofLeafDisposition::CheckedContact { .. }
        )));
        verify_adaptive_collision_certificate(&certificate).unwrap();
    }

    #[test]
    fn forged_prune_bound_is_rejected() {
        let mut certificate = co_translating_capsule_adaptive_query_with_certificate(
            point_capsule(0.0, 0.5),
            p(0.0, 0.0, 0.0),
            point_capsule(10.0, 0.5),
            p(0.0, 0.0, 0.0),
            config(),
        )
        .unwrap();
        let AdaptiveProofLeafDisposition::Pruned { moving_bounds, .. } =
            &mut certificate.leaves[0].disposition
        else {
            panic!("expected pruned root");
        };
        moving_bounds.max.x += 1.0;
        assert_eq!(
            verify_adaptive_collision_certificate(&certificate),
            Err(AdaptiveCertificateError::PruneBoundsMismatch)
        );
    }

    #[test]
    fn missing_interval_is_rejected() {
        let mut certificate = co_translating_capsule_adaptive_query_with_certificate(
            point_capsule(-10.0, 0.5),
            p(20.0, 0.0, 0.0),
            point_capsule(10.0, 0.5),
            p(-20.0, 0.0, 0.0),
            config(),
        )
        .unwrap();
        certificate.leaves[0].window.start_parameter = 0.01;
        assert!(matches!(
            verify_adaptive_collision_certificate(&certificate),
            Err(AdaptiveCertificateError::InvalidCoverage)
                | Err(AdaptiveCertificateError::InvalidDyadicWindow)
        ));
    }

    #[test]
    fn forged_narrow_phase_distance_is_rejected() {
        let mut certificate = co_translating_capsule_adaptive_query_with_certificate(
            point_capsule(0.0, 0.5),
            p(10.0, 0.0, 0.0),
            point_capsule(10.0, 0.5),
            p(10.0, 0.0, 0.0),
            AdaptiveSubdivisionConfig {
                maximum_depth: 0,
                minimum_window_width: 1.0,
            },
        )
        .unwrap();
        let leaf = certificate
            .leaves
            .iter_mut()
            .find(|leaf| {
                matches!(
                    leaf.disposition,
                    AdaptiveProofLeafDisposition::CheckedClear { .. }
                )
            })
            .expect("overlapping swept bounds require an exact check");
        let AdaptiveProofLeafDisposition::CheckedClear { closest } = &mut leaf.disposition else {
            unreachable!()
        };
        closest.distance_mm += 1.0;
        assert_eq!(
            verify_adaptive_collision_certificate(&certificate),
            Err(AdaptiveCertificateError::NarrowPhaseClaimMismatch)
        );
    }

    #[test]
    fn unresolved_leaf_blocks_no_contact_certificate() {
        let moving = Capsule::new(p(0.0, 0.0, 0.0), p(1_000.0, 0.0, 0.0), 4.2).unwrap();
        let obstacle = Capsule::new(p(0.0, 6.0, 6.0), p(1_000.0, 6.000_001, 6.0), 4.2).unwrap();
        let certificate = co_translating_capsule_adaptive_query_with_certificate(
            moving,
            p(0.0, 0.0, 0.0),
            obstacle,
            p(0.0, 0.0, 0.0),
            AdaptiveSubdivisionConfig {
                maximum_depth: 0,
                minimum_window_width: 1.0,
            },
        )
        .unwrap();
        assert_eq!(
            certificate.report.status,
            AdaptiveCollisionStatus::Unresolved
        );
        verify_adaptive_collision_certificate(&certificate).unwrap();
    }

    #[test]
    fn certificate_replay_is_deterministic() {
        let first = co_translating_capsule_adaptive_query_with_certificate(
            point_capsule(-10.0, 0.5),
            p(20.0, 0.0, 0.0),
            point_capsule(10.0, 0.5),
            p(-20.0, 0.0, 0.0),
            config(),
        )
        .unwrap();
        let second = co_translating_capsule_adaptive_query_with_certificate(
            point_capsule(-10.0, 0.5),
            p(20.0, 0.0, 0.0),
            point_capsule(10.0, 0.5),
            p(-20.0, 0.0, 0.0),
            config(),
        )
        .unwrap();
        assert_eq!(first, second);
        verify_adaptive_collision_certificate(&first).unwrap();
    }
}
