// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validated, deterministic camera routes for pre-construction design review.
//!
//! This module supplies route data and pose sampling for desktop floating
//! cameras, guided walkthroughs, orbit reviews, and an XR adapter's reference
//! poses. It does not load IFC, test collisions against building geometry,
//! create an XR session, or assert that a design is safe or code-compliant.
//! Callers retain control of input, pausing, and comfort settings.

use bevy::prelude::{Transform, Vec3};

const POSE_EPSILON: f32 = 1.0e-4;

/// Presentation style for a review route. This selects interaction intent; it
/// does not imply the renderer or headset runtime for a given mode exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewRig {
    /// Human-height camera route; the consumer must supply collision handling.
    GuidedWalkthrough,
    /// Unconstrained inspection camera path, explicitly not a geometry check.
    FloatingCamera,
    /// Camera review around a selected object or room feature.
    OrbitReview,
    /// Reference route for a head-tracked XR adapter. XR is not implemented here.
    HeadTrackedVrReference,
    /// Route whose stops are meant to be reached by user-confirmed teleports.
    AccessibleTeleport,
}

/// A named stop on a camera route. Positions are in metres in the model's
/// declared coordinate frame; the route does not establish georeferencing.
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewWaypoint {
    pub waypoint_id: String,
    pub label: String,
    pub position_m: [f32; 3],
    pub look_at_m: [f32; 3],
    /// Seconds on the route timeline at which this stop is reached.
    pub arrival_seconds: f32,
    /// Minimum authored pause before travel toward the next stop begins.
    pub dwell_seconds: f32,
}

/// A user-visible camera route associated with a design revision.
#[derive(Clone, Debug, PartialEq)]
pub struct ArchitecturalReviewRoute {
    pub route_id: String,
    pub design_revision_id: String,
    pub rig: ReviewRig,
    /// Explicit route-level velocity ceiling chosen for this experience.
    /// This is not a universal comfort limit; apps should offer slower options.
    pub max_translation_speed_mps: f32,
    pub waypoints: Vec<ReviewWaypoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewRouteIssue {
    EmptyRouteId,
    EmptyDesignRevisionId,
    TooFewWaypoints,
    InvalidSpeedLimit,
    EmptyWaypointId { index: usize },
    EmptyWaypointLabel { index: usize },
    DuplicateWaypointId(String),
    NonFinitePose { waypoint_id: String },
    DegenerateLookDirection { waypoint_id: String },
    InvalidArrivalTime { waypoint_id: String },
    InvalidDwellTime { waypoint_id: String },
    TimelineOverlap { waypoint_id: String },
    InsufficientTravelTime { from_id: String, to_id: String },
    TranslationSpeedExceeded { from_id: String, to_id: String },
}

/// Validate stable identifiers, finite poses, route chronology and declared
/// translation speed. This is not collision, vestibular-safety, accessibility,
/// building-code, or geometry validation.
pub fn validate_review_route(
    route: &ArchitecturalReviewRoute,
) -> Result<(), Vec<ReviewRouteIssue>> {
    let mut issues = Vec::new();
    if route.route_id.trim().is_empty() {
        issues.push(ReviewRouteIssue::EmptyRouteId);
    }
    if route.design_revision_id.trim().is_empty() {
        issues.push(ReviewRouteIssue::EmptyDesignRevisionId);
    }
    if route.waypoints.len() < 2 {
        issues.push(ReviewRouteIssue::TooFewWaypoints);
    }
    if !route.max_translation_speed_mps.is_finite()
        || route.max_translation_speed_mps <= 0.0
    {
        issues.push(ReviewRouteIssue::InvalidSpeedLimit);
    }

    let mut ids = std::collections::HashSet::new();
    for (index, waypoint) in route.waypoints.iter().enumerate() {
        if waypoint.waypoint_id.trim().is_empty() {
            issues.push(ReviewRouteIssue::EmptyWaypointId { index });
        } else if !ids.insert(waypoint.waypoint_id.as_str()) {
            issues.push(ReviewRouteIssue::DuplicateWaypointId(
                waypoint.waypoint_id.clone(),
            ));
        }
        if waypoint.label.trim().is_empty() {
            issues.push(ReviewRouteIssue::EmptyWaypointLabel { index });
        }
        let finite = waypoint
            .position_m
            .iter()
            .chain(waypoint.look_at_m.iter())
            .all(|value| value.is_finite());
        if !finite {
            issues.push(ReviewRouteIssue::NonFinitePose {
                waypoint_id: waypoint.waypoint_id.clone(),
            });
            continue;
        }
        let position = Vec3::from_array(waypoint.position_m);
        let look_at = Vec3::from_array(waypoint.look_at_m);
        if position.distance_squared(look_at) <= POSE_EPSILON * POSE_EPSILON {
            issues.push(ReviewRouteIssue::DegenerateLookDirection {
                waypoint_id: waypoint.waypoint_id.clone(),
            });
        }
        if !waypoint.arrival_seconds.is_finite() || waypoint.arrival_seconds < 0.0 {
            issues.push(ReviewRouteIssue::InvalidArrivalTime {
                waypoint_id: waypoint.waypoint_id.clone(),
            });
        }
        if !waypoint.dwell_seconds.is_finite() || waypoint.dwell_seconds < 0.0 {
            issues.push(ReviewRouteIssue::InvalidDwellTime {
                waypoint_id: waypoint.waypoint_id.clone(),
            });
        }
    }

    for pair in route.waypoints.windows(2) {
        let from = &pair[0];
        let to = &pair[1];
        if !from.arrival_seconds.is_finite()
            || !from.dwell_seconds.is_finite()
            || !to.arrival_seconds.is_finite()
        {
            continue;
        }

        let movement_start = from.arrival_seconds + from.dwell_seconds;
        let travel_seconds = to.arrival_seconds - movement_start;
        if travel_seconds < 0.0 {
            issues.push(ReviewRouteIssue::TimelineOverlap {
                waypoint_id: to.waypoint_id.clone(),
            });
            continue;
        }

        let from_position = Vec3::from_array(from.position_m);
        let to_position = Vec3::from_array(to.position_m);
        let from_look_at = Vec3::from_array(from.look_at_m);
        let to_look_at = Vec3::from_array(to.look_at_m);
        let travel_distance = from_position.distance(to_position);
        let view_target_delta = from_look_at.distance(to_look_at);

        if travel_seconds <= 0.0 {
            if travel_distance > POSE_EPSILON || view_target_delta > POSE_EPSILON {
                issues.push(ReviewRouteIssue::InsufficientTravelTime {
                    from_id: from.waypoint_id.clone(),
                    to_id: to.waypoint_id.clone(),
                });
            }
        } else if travel_distance / travel_seconds > route.max_translation_speed_mps {
            issues.push(ReviewRouteIssue::TranslationSpeedExceeded {
                from_id: from.waypoint_id.clone(),
                to_id: to.waypoint_id.clone(),
            });
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn waypoint_transform(waypoint: &ReviewWaypoint) -> Option<Transform> {
    let position = Vec3::from_array(waypoint.position_m);
    let look_at = Vec3::from_array(waypoint.look_at_m);
    if !position.is_finite()
        || !look_at.is_finite()
        || position.distance_squared(look_at) <= POSE_EPSILON * POSE_EPSILON
    {
        return None;
    }
    Some(Transform::from_translation(position).looking_at(look_at, Vec3::Y))
}

/// Sample a deterministic reference pose at elapsed route time.
///
/// Before the first arrival and after the last stop, the pose is clamped to the
/// endpoint. A pause holds the current pose. Invalid routes or degenerate
/// intermediate look vectors return `None`, allowing the caller to retain a
/// safe static camera instead of applying an invalid transform.
pub fn sample_review_route(
    route: &ArchitecturalReviewRoute,
    elapsed_seconds: f32,
) -> Option<Transform> {
    if !elapsed_seconds.is_finite() || validate_review_route(route).is_err() {
        return None;
    }
    let first = route.waypoints.first()?;
    if elapsed_seconds <= first.arrival_seconds + first.dwell_seconds {
        return waypoint_transform(first);
    }

    for pair in route.waypoints.windows(2) {
        let from = &pair[0];
        let to = &pair[1];
        if elapsed_seconds > to.arrival_seconds {
            continue;
        }

        let movement_start = from.arrival_seconds + from.dwell_seconds;
        let duration = to.arrival_seconds - movement_start;
        if duration <= 0.0 {
            return waypoint_transform(to);
        }
        let amount = ((elapsed_seconds - movement_start) / duration).clamp(0.0, 1.0);
        let position = Vec3::from_array(from.position_m)
            .lerp(Vec3::from_array(to.position_m), amount);
        let look_at = Vec3::from_array(from.look_at_m)
            .lerp(Vec3::from_array(to.look_at_m), amount);
        if !position.is_finite()
            || !look_at.is_finite()
            || position.distance_squared(look_at) <= POSE_EPSILON * POSE_EPSILON
        {
            return None;
        }
        return Some(Transform::from_translation(position).looking_at(look_at, Vec3::Y));
    }

    waypoint_transform(route.waypoints.last()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waypoint(
        id: &str,
        position_m: [f32; 3],
        look_at_m: [f32; 3],
        arrival_seconds: f32,
        dwell_seconds: f32,
    ) -> ReviewWaypoint {
        ReviewWaypoint {
            waypoint_id: id.into(),
            label: id.into(),
            position_m,
            look_at_m,
            arrival_seconds,
            dwell_seconds,
        }
    }

    fn route() -> ArchitecturalReviewRoute {
        ArchitecturalReviewRoute {
            route_id: "home-review:accessible-tour".into(),
            design_revision_id: "house-model:r7".into(),
            rig: ReviewRig::FloatingCamera,
            max_translation_speed_mps: 2.0,
            waypoints: vec![
                waypoint("entry", [0.0, 1.6, 4.0], [0.0, 1.4, 0.0], 0.0, 2.0),
                waypoint("kitchen", [0.0, 1.6, 0.0], [2.0, 1.4, 0.0], 4.5, 1.0),
                waypoint("garden", [3.0, 1.6, 0.0], [3.0, 1.4, -2.0], 7.0, 1.5),
            ],
        }
    }

    #[test]
    fn valid_route_has_a_deterministic_midpoint_pose() {
        let route = route();
        assert_eq!(validate_review_route(&route), Ok(()));
        let pose = sample_review_route(&route, 3.25).expect("route midpoint must sample");
        assert!((pose.translation.z - 2.0).abs() < 1.0e-5);
    }

    #[test]
    fn route_holds_at_stops_and_clamps_after_the_final_stop() {
        let route = route();
        let at_entry = sample_review_route(&route, 1.0).unwrap();
        let after_end = sample_review_route(&route, 100.0).unwrap();
        assert_eq!(at_entry.translation, Vec3::new(0.0, 1.6, 4.0));
        assert_eq!(after_end.translation, Vec3::new(3.0, 1.6, 0.0));
    }

    #[test]
    fn duplicated_waypoint_ids_are_rejected() {
        let mut route = route();
        route.waypoints[1].waypoint_id = route.waypoints[0].waypoint_id.clone();
        assert!(validate_review_route(&route).unwrap_err().contains(
            &ReviewRouteIssue::DuplicateWaypointId("entry".into())
        ));
        assert!(sample_review_route(&route, 1.0).is_none());
    }

    #[test]
    fn route_rejects_non_finite_coordinates_and_invalid_time() {
        let mut route = route();
        route.waypoints[1].position_m[0] = f32::NAN;
        route.waypoints[2].arrival_seconds = f32::INFINITY;
        let issues = validate_review_route(&route).unwrap_err();
        assert!(issues.iter().any(|issue| matches!(
            issue,
            ReviewRouteIssue::NonFinitePose { waypoint_id } if waypoint_id == "kitchen"
        )));
        assert!(issues.iter().any(|issue| matches!(
            issue,
            ReviewRouteIssue::InvalidArrivalTime { waypoint_id } if waypoint_id == "garden"
        )));
    }

    #[test]
    fn route_rejects_speed_above_the_declared_limit() {
        let mut route = route();
        route.waypoints[1].arrival_seconds = 2.1;
        route.max_translation_speed_mps = 0.5;
        assert!(validate_review_route(&route).unwrap_err().iter().any(|issue| {
            matches!(issue, ReviewRouteIssue::TranslationSpeedExceeded { .. })
        }));
    }

    #[test]
    fn route_requires_dwell_and_travel_time_not_to_overlap() {
        let mut route = route();
        route.waypoints[1].arrival_seconds = 1.0;
        assert!(validate_review_route(&route).unwrap_err().iter().any(|issue| {
            matches!(issue, ReviewRouteIssue::TimelineOverlap { waypoint_id } if waypoint_id == "kitchen")
        }));
    }

    #[test]
    fn degenerate_camera_look_direction_is_rejected() {
        let mut route = route();
        route.waypoints[1].look_at_m = route.waypoints[1].position_m;
        assert!(validate_review_route(&route).unwrap_err().iter().any(|issue| {
            matches!(issue, ReviewRouteIssue::DegenerateLookDirection { waypoint_id } if waypoint_id == "kitchen")
        }));
    }

    #[test]
    fn teleport_or_headset_route_modes_are_data_only() {
        let mut route = route();
        route.rig = ReviewRig::AccessibleTeleport;
        assert_eq!(validate_review_route(&route), Ok(()));
        route.rig = ReviewRig::HeadTrackedVrReference;
        assert_eq!(validate_review_route(&route), Ok(()));
        // XR runtime, collision checks, and user-input handling belong to adapters.
    }
}
