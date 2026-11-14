// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validated, deterministic camera routes for pre-construction design review.
//!
//! This module supplies route data and pose sampling for desktop floating
//! cameras, guided walkthroughs, orbit reviews, and an XR adapter's reference
//! poses. It does not load IFC, test collisions against building geometry,
//! create an XR session, or assert that a design is safe or code-compliant.
//! Callers retain control of input, pausing, and comfort settings.

use bevy::input::mouse::{AccumulatedMouseMotion, MouseButton};
use bevy::prelude::{ButtonInput, Component, KeyCode, Quat, Res, Time, Transform, Vec3};
use serde::{Deserialize, Serialize};

const POSE_EPSILON: f32 = 1.0e-4;

/// Presentation style for a review route. This selects interaction intent; it
/// does not imply the renderer or headset runtime for a given mode exists.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ArchitecturalReviewRoute {
    pub route_id: String,
    pub design_revision_id: String,
    pub rig: ReviewRig,
    /// Explicit route-level velocity ceiling chosen for this experience.
    /// This is not a universal comfort limit; apps should offer slower options.
    pub max_translation_speed_mps: f32,
    pub waypoints: Vec<ReviewWaypoint>,
}

/// Information category displayed over a design. The category is supplied by a
/// separately reviewed rule/evidence pipeline; the viewer does not decide legal
/// applicability or the truth of engineering claims by drawing an overlay.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReviewAnnotationKind {
    /// Requirement identified as binding by the selected jurisdiction adapter.
    BindingRequirement,
    /// Competing binding assertions whose precedence has not been resolved.
    BindingConflict,
    /// Evidence needed to assess a declared binding requirement is absent.
    RequiredEvidence,
    /// Engineering calculation or test result; validity remains tool/scope-bound.
    EngineeringEvidence,
    /// Advisory information that is not itself a binding requirement.
    NonBindingGuidance,
    /// Site or survey fact not yet verified.
    SiteUnknown,
    /// A person's preference or intended experience, not legislation.
    OwnerPreference,
    /// Proposed code change awaiting legal adoption.
    CodeReformProposal,
}

/// A display disposition with semantics that prevent taste from becoming a
/// pretend code violation. The viewer must retain the kind and source when
/// filtering, sorting, or styling annotations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReviewAnnotationState {
    Satisfied,
    Failed,
    Unknown,
    NotApplicable,
    Conflict,
    UnfulfilledPreference,
    Informational,
}

/// One source attached to a conflict overlay. Multiple incompatible sources
/// must remain visible together.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewSourceRef {
    pub module_id: String,
    pub source_ref: String,
}

/// Revision-pinned annotation for a plan, room, component, or camera stop.
/// Evidence hashes are metadata until an artifact verifier checks actual bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewAnnotation {
    pub annotation_id: String,
    pub design_revision_id: String,
    pub kind: ReviewAnnotationKind,
    pub state: ReviewAnnotationState,
    pub title: String,
    pub explanation: String,
    pub source_ref: Option<String>,
    pub module_id: Option<String>,
    /// Additional sources for a binding conflict. Together with the primary
    /// source, this must contain at least two distinct module/clause pairs.
    pub related_sources: Vec<ReviewSourceRef>,
    pub evidence_artifact_id: Option<String>,
    pub evidence_sha256: Option<String>,
    pub uncertainty_note: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewAnnotationIssue {
    EmptyAnnotationId,
    DuplicateAnnotationId(String),
    EmptyDesignRevisionId,
    RevisionMismatch { annotation_id: String },
    EmptyTitle { annotation_id: String },
    EmptyExplanation { annotation_id: String },
    MissingBindingProvenance { annotation_id: String },
    MissingConflictSources { annotation_id: String },
    InvalidRelatedSource { annotation_id: String },
    MissingResultEvidenceArtifact { annotation_id: String },
    MissingUncertaintyNote { annotation_id: String },
    InvalidKindStateCombination { annotation_id: String },
    EvidenceIdWithoutDigest { annotation_id: String },
    DigestWithoutEvidenceId { annotation_id: String },
    InvalidEvidenceDigest { annotation_id: String },
}

/// Validate annotations against the exact model revision being inspected.
///
/// This is a presentation/provenance contract, not a jurisdiction evaluator or
/// cryptographic verifier. A binding annotation must identify its rule module
/// and source. Owner preferences can be unfulfilled, but cannot be labelled a
/// legal/engineering failure. Reform proposals and general guidance remain
/// informational so they do not masquerade as active rules.
pub fn validate_review_annotations_for_revision(
    annotations: &[ReviewAnnotation],
    design_revision_id: &str,
) -> Result<(), Vec<ReviewAnnotationIssue>> {
    let mut issues = Vec::new();
    if design_revision_id.trim().is_empty() {
        issues.push(ReviewAnnotationIssue::EmptyDesignRevisionId);
    }
    let mut annotation_ids = std::collections::HashSet::new();

    for annotation in annotations {
        if annotation.annotation_id.trim().is_empty() {
            issues.push(ReviewAnnotationIssue::EmptyAnnotationId);
        } else if !annotation_ids.insert(annotation.annotation_id.as_str()) {
            issues.push(ReviewAnnotationIssue::DuplicateAnnotationId(
                annotation.annotation_id.clone(),
            ));
        }
        if annotation.design_revision_id != design_revision_id {
            issues.push(ReviewAnnotationIssue::RevisionMismatch {
                annotation_id: annotation.annotation_id.clone(),
            });
        }
        if annotation.title.trim().is_empty() {
            issues.push(ReviewAnnotationIssue::EmptyTitle {
                annotation_id: annotation.annotation_id.clone(),
            });
        }
        if annotation.explanation.trim().is_empty() {
            issues.push(ReviewAnnotationIssue::EmptyExplanation {
                annotation_id: annotation.annotation_id.clone(),
            });
        }

        match annotation.kind {
            ReviewAnnotationKind::BindingRequirement | ReviewAnnotationKind::RequiredEvidence => {
                if annotation
                    .source_ref
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                    || annotation
                        .module_id
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                {
                    issues.push(ReviewAnnotationIssue::MissingBindingProvenance {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
                let invalid_state = match annotation.kind {
                    ReviewAnnotationKind::BindingRequirement => matches!(
                        annotation.state,
                        ReviewAnnotationState::Conflict
                            | ReviewAnnotationState::UnfulfilledPreference
                            | ReviewAnnotationState::Informational
                    ),
                    // Absence of required evidence is epistemically unknown,
                    // not a pass and not an implicit NotApplicable decision.
                    ReviewAnnotationKind::RequiredEvidence => {
                        annotation.state != ReviewAnnotationState::Unknown
                    }
                    _ => unreachable!("matched only the two binding kinds"),
                };
                if invalid_state {
                    issues.push(ReviewAnnotationIssue::InvalidKindStateCombination {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
            }
            ReviewAnnotationKind::BindingConflict => {
                let primary = annotation
                    .source_ref
                    .as_deref()
                    .zip(annotation.module_id.as_deref());
                let mut sources = std::collections::HashSet::new();
                let primary_valid = primary.is_some_and(|(source, module)| {
                    !source.trim().is_empty()
                        && !module.trim().is_empty()
                        && sources.insert((module, source))
                });
                let related_valid = annotation.related_sources.iter().all(|source| {
                    !source.module_id.trim().is_empty()
                        && !source.source_ref.trim().is_empty()
                        && sources.insert((source.module_id.as_str(), source.source_ref.as_str()))
                });
                if annotation.state != ReviewAnnotationState::Conflict
                    || !primary_valid
                    || !related_valid
                {
                    issues.push(ReviewAnnotationIssue::InvalidKindStateCombination {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
                if annotation.related_sources.is_empty() || !primary_valid || !related_valid {
                    issues.push(ReviewAnnotationIssue::MissingConflictSources {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
            }
            ReviewAnnotationKind::NonBindingGuidance | ReviewAnnotationKind::CodeReformProposal => {
                if annotation.state != ReviewAnnotationState::Informational {
                    issues.push(ReviewAnnotationIssue::InvalidKindStateCombination {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
            }
            ReviewAnnotationKind::SiteUnknown => {
                if annotation.state != ReviewAnnotationState::Unknown {
                    issues.push(ReviewAnnotationIssue::InvalidKindStateCombination {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
                if annotation
                    .uncertainty_note
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    issues.push(ReviewAnnotationIssue::MissingUncertaintyNote {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
            }
            ReviewAnnotationKind::OwnerPreference => {
                if !matches!(
                    annotation.state,
                    ReviewAnnotationState::Satisfied
                        | ReviewAnnotationState::Unknown
                        | ReviewAnnotationState::UnfulfilledPreference
                        | ReviewAnnotationState::Informational
                ) {
                    issues.push(ReviewAnnotationIssue::InvalidKindStateCombination {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
            }
            ReviewAnnotationKind::EngineeringEvidence => {
                if matches!(
                    annotation.state,
                    ReviewAnnotationState::Conflict
                        | ReviewAnnotationState::UnfulfilledPreference
                        | ReviewAnnotationState::Informational
                ) {
                    issues.push(ReviewAnnotationIssue::InvalidKindStateCombination {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
                if annotation.state == ReviewAnnotationState::Unknown
                    && annotation
                        .uncertainty_note
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                {
                    issues.push(ReviewAnnotationIssue::MissingUncertaintyNote {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
            }
        }

        if matches!(
            annotation.kind,
            ReviewAnnotationKind::BindingRequirement | ReviewAnnotationKind::EngineeringEvidence
        ) && matches!(
            annotation.state,
            ReviewAnnotationState::Satisfied | ReviewAnnotationState::Failed
        ) && annotation.evidence_artifact_id.is_none()
        {
            issues.push(ReviewAnnotationIssue::MissingResultEvidenceArtifact {
                annotation_id: annotation.annotation_id.clone(),
            });
        }

        match (
            annotation.evidence_artifact_id.as_deref(),
            annotation.evidence_sha256.as_deref(),
        ) {
            (Some(id), Some(digest)) => {
                if id.trim().is_empty()
                    || digest.len() != 64
                    || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    issues.push(ReviewAnnotationIssue::InvalidEvidenceDigest {
                        annotation_id: annotation.annotation_id.clone(),
                    });
                }
            }
            (Some(_), None) => issues.push(ReviewAnnotationIssue::EvidenceIdWithoutDigest {
                annotation_id: annotation.annotation_id.clone(),
            }),
            (None, Some(_)) => issues.push(ReviewAnnotationIssue::DigestWithoutEvidenceId {
                annotation_id: annotation.annotation_id.clone(),
            }),
            (None, None) => {}
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
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
    if !route.max_translation_speed_mps.is_finite() || route.max_translation_speed_mps <= 0.0 {
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

        if route.rig != ReviewRig::AccessibleTeleport {
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
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Convert one explicitly selected waypoint to a camera pose. An application
/// can use this only after the person chooses a stop; it is especially useful
/// for AccessibleTeleport, where clock-driven auto-teleporting is not provided.
pub fn review_waypoint_transform(waypoint: &ReviewWaypoint) -> Option<Transform> {
    waypoint_transform(waypoint)
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
    sample_valid_route_pose(route, elapsed_seconds)
}

fn sample_valid_route_pose(
    route: &ArchitecturalReviewRoute,
    elapsed_seconds: f32,
) -> Option<Transform> {
    // Teleport stops require an explicit user action; never advance them from
    // elapsed time. The consumer should call review_waypoint_transform only
    // after a stop is selected and confirmed.
    if route.rig == ReviewRig::AccessibleTeleport {
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
        let position =
            Vec3::from_array(from.position_m).lerp(Vec3::from_array(to.position_m), amount);
        let look_at = Vec3::from_array(from.look_at_m).lerp(Vec3::from_array(to.look_at_m), amount);
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

/// Whether the route begins moving when a review session opens. Automatic
/// motion must be explicitly opted into by the person creating the session.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReviewRouteStartPolicy {
    ManualStart,
    OptInAutoStart,
}

/// User-selected motion and control settings. These are stored in the review
/// manifest so a session can be reproduced; they are not universal comfort
/// limits or medical guidance. Static review always remains available.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReviewComfortProfile {
    pub profile_id: String,
    pub start_policy: ReviewRouteStartPolicy,
    /// Playback rate relative to the authored route; must be in (0, 1] so a
    /// comfort profile can slow a route, never make it run faster than authored.
    pub route_playback_rate: f32,
    pub free_camera_speed_mps: f32,
    /// Set to 1.0 to disable the Shift speed boost.
    pub fast_multiplier: f32,
    pub mouse_sensitivity: f32,
    /// Must remain below PI/2 to avoid camera pole flips.
    pub max_pitch_radians: f32,
    pub allow_vertical_translation: bool,
}

impl Default for ReviewComfortProfile {
    fn default() -> Self {
        Self {
            profile_id: "comfort:manual-reduced-motion".into(),
            start_policy: ReviewRouteStartPolicy::ManualStart,
            route_playback_rate: 0.5,
            free_camera_speed_mps: 0.8,
            fast_multiplier: 1.0,
            mouse_sensitivity: 0.001,
            max_pitch_radians: 1.2,
            allow_vertical_translation: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewComfortProfileIssue {
    EmptyProfileId,
    InvalidRoutePlaybackRate,
    InvalidFreeCameraSpeed,
    InvalidFastMultiplier,
    InvalidMouseSensitivity,
    InvalidMaximumPitch,
}

impl ReviewComfortProfile {
    pub fn validate(&self) -> Result<(), Vec<ReviewComfortProfileIssue>> {
        let mut issues = Vec::new();
        if self.profile_id.trim().is_empty() {
            issues.push(ReviewComfortProfileIssue::EmptyProfileId);
        }
        if !self.route_playback_rate.is_finite()
            || self.route_playback_rate <= 0.0
            || self.route_playback_rate > 1.0
        {
            issues.push(ReviewComfortProfileIssue::InvalidRoutePlaybackRate);
        }
        if !self.free_camera_speed_mps.is_finite() || self.free_camera_speed_mps <= 0.0 {
            issues.push(ReviewComfortProfileIssue::InvalidFreeCameraSpeed);
        }
        if !self.fast_multiplier.is_finite() || self.fast_multiplier < 1.0 {
            issues.push(ReviewComfortProfileIssue::InvalidFastMultiplier);
        }
        if !self.mouse_sensitivity.is_finite() || self.mouse_sensitivity <= 0.0 {
            issues.push(ReviewComfortProfileIssue::InvalidMouseSensitivity);
        }
        if !self.max_pitch_radians.is_finite()
            || self.max_pitch_radians <= 0.0
            || self.max_pitch_radians >= std::f32::consts::FRAC_PI_2
        {
            issues.push(ReviewComfortProfileIssue::InvalidMaximumPitch);
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

/// Component that plays a validated route on the Transform it is attached to.
/// For an XR scene, attach it to the rig origin and have the XR adapter apply
/// tracked head pose below that origin; do not attach it to a head-tracked camera
/// if that would overwrite headset movement. Invalid routes cannot construct a
/// player. AccessibleTeleport routes are never clock-driven.
#[derive(Component, Clone, Debug)]
pub struct ReviewRoutePlayback {
    route: ArchitecturalReviewRoute,
    comfort_profile: ReviewComfortProfile,
    elapsed_seconds: f32,
    playing: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewPlaybackSetupIssue {
    InvalidRoute(Vec<ReviewRouteIssue>),
    InvalidComfortProfile(Vec<ReviewComfortProfileIssue>),
}

impl ReviewRoutePlayback {
    /// Construct with the conservative default profile. Routes start paused;
    /// Space or an explicit app action is required to begin motion.
    pub fn new(route: ArchitecturalReviewRoute) -> Result<Self, Vec<ReviewRouteIssue>> {
        validate_review_route(&route)?;
        Ok(Self {
            route,
            comfort_profile: ReviewComfortProfile::default(),
            elapsed_seconds: 0.0,
            playing: false,
        })
    }

    /// Construct from an explicit, validated profile. Auto-start is honored
    /// only when the person has selected OptInAutoStart, and never for teleport.
    pub fn new_with_comfort_profile(
        route: ArchitecturalReviewRoute,
        comfort_profile: ReviewComfortProfile,
    ) -> Result<Self, Vec<ReviewPlaybackSetupIssue>> {
        validate_review_route(&route)
            .map_err(|issues| vec![ReviewPlaybackSetupIssue::InvalidRoute(issues)])?;
        comfort_profile
            .validate()
            .map_err(|issues| vec![ReviewPlaybackSetupIssue::InvalidComfortProfile(issues)])?;
        let playing = comfort_profile.start_policy == ReviewRouteStartPolicy::OptInAutoStart
            && route.rig != ReviewRig::AccessibleTeleport;
        Ok(Self {
            route,
            comfort_profile,
            elapsed_seconds: 0.0,
            playing,
        })
    }

    pub fn route(&self) -> &ArchitecturalReviewRoute {
        &self.route
    }

    pub fn comfort_profile(&self) -> &ReviewComfortProfile {
        &self.comfort_profile
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed_seconds
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Pause/resume only the clock-driven modes. Teleport remains user-directed.
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing && self.route.rig != ReviewRig::AccessibleTeleport;
    }

    /// Restart a clock-driven route from its first stop. This is an explicit
    /// user action; teleport routes remain stopped and manually selected.
    pub fn restart(&mut self) {
        if self.route.rig != ReviewRig::AccessibleTeleport {
            self.elapsed_seconds = 0.0;
            self.playing = true;
        }
    }
}

/// Optional user-controlled desktop floating camera. It moves the camera
/// relative to the model's declared local frame; it is not collision-aware and
/// must not be used as an accessibility or code-compliance check.
#[derive(Component, Clone, Debug)]
pub struct ReviewFreeCameraController {
    enabled: bool,
    yaw_radians: f32,
    pitch_radians: f32,
    /// Normal movement speed in model-space metres per second.
    pub speed_mps: f32,
    /// Multiplier while either Shift key is held.
    pub fast_multiplier: f32,
    /// Mouse rotation sensitivity in radians per pixel.
    pub mouse_sensitivity: f32,
    /// Maximum absolute camera pitch; keeping this below PI/2 avoids pole flips.
    pub max_pitch_radians: f32,
    /// Vertical flying camera motion is off by default and can be enabled in the profile.
    pub allow_vertical_translation: bool,
}

impl ReviewFreeCameraController {
    /// Create a controller initialized from an existing camera transform.
    /// It starts disabled so it never unexpectedly fights a guided route.
    pub fn from_transform(transform: &Transform) -> Self {
        let forward = transform.forward().as_vec3();
        let yaw_radians = (-forward.x).atan2(-forward.z);
        let max_pitch_radians = 1.2;
        let pitch_radians = forward
            .y
            .clamp(-1.0, 1.0)
            .asin()
            .clamp(-max_pitch_radians, max_pitch_radians);
        Self {
            enabled: false,
            yaw_radians,
            pitch_radians,
            speed_mps: 0.8,
            fast_multiplier: 1.0,
            mouse_sensitivity: 0.001,
            max_pitch_radians,
            allow_vertical_translation: false,
        }
    }

    /// Rebase the controller on the currently displayed camera orientation.
    /// This keeps a handoff from a scripted route or orbit view from snapping
    /// back to the controller's original spawn orientation.
    pub fn synchronize_from_transform(&mut self, transform: &Transform) {
        let forward = transform.forward().as_vec3();
        self.yaw_radians = (-forward.x).atan2(-forward.z);
        self.pitch_radians = forward
            .y
            .clamp(-1.0, 1.0)
            .asin()
            .clamp(-self.max_pitch_radians, self.max_pitch_radians);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Apply the profile recorded in the review session. Invalid profiles leave
    /// the current controller settings unchanged.
    pub fn apply_comfort_profile(
        &mut self,
        profile: &ReviewComfortProfile,
    ) -> Result<(), Vec<ReviewComfortProfileIssue>> {
        profile.validate()?;
        self.speed_mps = profile.free_camera_speed_mps;
        self.fast_multiplier = profile.fast_multiplier;
        self.mouse_sensitivity = profile.mouse_sensitivity;
        self.max_pitch_radians = profile.max_pitch_radians;
        self.pitch_radians = self
            .pitch_radians
            .clamp(-self.max_pitch_radians, self.max_pitch_radians);
        self.allow_vertical_translation = profile.allow_vertical_translation;
        Ok(())
    }

    /// Enable direct movement and atomically pause the attached scripted route.
    pub fn enable(&mut self, playback: &mut ReviewRoutePlayback, current_transform: &Transform) {
        self.synchronize_from_transform(current_transform);
        self.enabled = true;
        playback.set_playing(false);
    }

    /// Disable direct movement; the caller chooses whether to resume or restart the route.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn settings_are_valid(&self) -> bool {
        self.speed_mps.is_finite()
            && self.speed_mps > 0.0
            && self.fast_multiplier.is_finite()
            && self.fast_multiplier >= 1.0
            && self.mouse_sensitivity.is_finite()
            && self.mouse_sensitivity > 0.0
            && self.max_pitch_radians.is_finite()
            && self.max_pitch_radians > 0.0
            && self.max_pitch_radians < std::f32::consts::FRAC_PI_2
            && self.yaw_radians.is_finite()
            && self.pitch_radians.is_finite()
    }
}

/// Apply direct free-camera motion: hold right mouse to look; WASD to move
/// horizontally; E/Q to rise/descend; Shift to move faster. The separately
/// ordered `manage_architectural_review_controls` system owns F/Space/R and route
/// state. Invalid controller settings result in no camera movement. This system
/// does not lock or capture the cursor, so the host app can choose its own
/// accessible pointer behavior.
pub fn drive_free_review_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: bevy::prelude::Query<(&mut Transform, &mut ReviewFreeCameraController)>,
) {
    for (mut transform, mut controller) in &mut query {
        if !controller.enabled || !controller.settings_are_valid() {
            continue;
        }

        if mouse_buttons.pressed(MouseButton::Right) {
            controller.yaw_radians -= mouse_motion.delta.x * controller.mouse_sensitivity;
            controller.pitch_radians = (controller.pitch_radians
                - mouse_motion.delta.y * controller.mouse_sensitivity)
                .clamp(-controller.max_pitch_radians, controller.max_pitch_radians);
        }
        transform.rotation = Quat::from_rotation_y(controller.yaw_radians)
            * Quat::from_rotation_x(controller.pitch_radians);

        let forward = transform.rotation * -Vec3::Z;
        let right = transform.rotation * Vec3::X;
        let forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
        let mut movement = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            movement += forward;
        }
        if keys.pressed(KeyCode::KeyS) {
            movement -= forward;
        }
        if keys.pressed(KeyCode::KeyD) {
            movement += right;
        }
        if keys.pressed(KeyCode::KeyA) {
            movement -= right;
        }
        if controller.allow_vertical_translation {
            if keys.pressed(KeyCode::KeyE) {
                movement += Vec3::Y;
            }
            if keys.pressed(KeyCode::KeyQ) {
                movement -= Vec3::Y;
            }
        }

        let speed = controller.speed_mps
            * if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                controller.fast_multiplier
            } else {
                1.0
            };
        transform.translation += movement.normalize_or_zero() * speed * time.delta_secs();
    }
}

/// Handle discrete review controls in one deterministic system. Space pauses or
/// resumes the scripted route; R restarts it; F toggles the direct floating
/// camera. Enabling free camera pauses the route. Space/R exits free camera
/// before changing the route state, so the route player and free camera never
/// intentionally drive the same Transform in the same frame.
pub fn manage_architectural_review_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: bevy::prelude::Query<(
        &Transform,
        &mut ReviewRoutePlayback,
        &mut ReviewFreeCameraController,
    )>,
) {
    if !(keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::KeyR)
        || keys.just_pressed(KeyCode::KeyF))
    {
        return;
    }

    for (transform, mut playback, mut free_camera) in &mut query {
        if keys.just_pressed(KeyCode::KeyF) {
            if free_camera.is_enabled() {
                free_camera.disable();
            } else {
                free_camera.enable(&mut playback, transform);
            }
        } else if keys.just_pressed(KeyCode::Space) {
            free_camera.disable();
            let should_play = !playback.is_playing();
            playback.set_playing(should_play);
        } else if keys.just_pressed(KeyCode::KeyR) {
            free_camera.disable();
            playback.restart();
        }
    }
}

/// Advance camera-route players. On invalid runtime poses, playback pauses and
/// leaves the last valid Transform unchanged (a safe static fallback).
pub fn advance_architectural_review_routes(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    mut query: bevy::prelude::Query<(&mut Transform, &mut ReviewRoutePlayback)>,
) {
    for (mut transform, mut playback) in &mut query {
        if !playback.playing {
            continue;
        }
        playback.elapsed_seconds +=
            time.delta_secs() * playback.comfort_profile.route_playback_rate;
        if let Some(pose) = sample_valid_route_pose(&playback.route, playback.elapsed_seconds) {
            *transform = pose;
        } else {
            playback.playing = false;
            continue;
        }
        if let Some(last) = playback.route.waypoints.last()
            && playback.elapsed_seconds >= last.arrival_seconds + last.dwell_seconds
        {
            playback.elapsed_seconds = last.arrival_seconds + last.dwell_seconds;
            playback.playing = false;
        }
    }
}

/// A person's feedback after experiencing a design revision. This is a
/// user report—not a verified building-code finding, a measured dimension, or
/// proof of an engineering defect. Historical records should remain attached
/// to their original revision; this validator is for presenting/merging them
/// into a currently displayed review.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OccupantFeedbackCategory {
    PersonalPreference,
    SpatialComfort,
    CirculationConcern,
    AccessibilityConcern,
    SightlineAndPrivacy,
    StorageAndErgonomics,
    DaylightAndOutdoorConnection,
    AcousticsQuestion,
    MaterialsAndMaintenance,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OccupantFeedbackPriority {
    NiceToHave,
    Important,
    MustResolveBeforeBuild,
}

/// Share scope is explicit. Feedback is private by default when created by a
/// consumer; public case studies require a separate explicit consent flag.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OccupantFeedbackShareScope {
    Private,
    ProjectTeam,
    PublicCaseStudy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OccupantFeedbackLifecycle {
    Captured,
    NeedsDesignIteration,
    NeedsIndependentAssessment,
    AcceptedForDesignChange,
    Deferred,
    Resolved,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OccupantReviewFeedback {
    pub feedback_id: String,
    pub design_revision_id: String,
    /// Optional immersive route context; feedback can also come from a static plan.
    pub route_id: Option<String>,
    pub waypoint_id: Option<String>,
    /// Stable room/component ID where available, not a raw room scan.
    pub location_ref: Option<String>,
    pub category: OccupantFeedbackCategory,
    pub priority: OccupantFeedbackPriority,
    pub note: String,
    pub desired_change: Option<String>,
    pub share_scope: OccupantFeedbackShareScope,
    pub explicit_public_consent: bool,
    pub lifecycle: OccupantFeedbackLifecycle,
    pub disposition_note: Option<String>,
    /// Stable design-change request or issue ID; does not mean that the change
    /// has been applied or verified.
    pub design_change_request_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OccupantFeedbackIssue {
    EmptyFeedbackId,
    DuplicateFeedbackId(String),
    EmptyDesignRevisionId,
    RevisionMismatch {
        feedback_id: String,
    },
    EmptyNote {
        feedback_id: String,
    },
    BlankLocationReference {
        feedback_id: String,
    },
    RouteContextUnavailable {
        feedback_id: String,
    },
    RouteMismatch {
        feedback_id: String,
    },
    RouteRevisionMismatch {
        feedback_id: String,
    },
    WaypointWithoutRoute {
        feedback_id: String,
    },
    UnknownWaypoint {
        feedback_id: String,
        waypoint_id: String,
    },
    PublicShareWithoutConsent {
        feedback_id: String,
    },
    AcceptedWithoutChangeRequest {
        feedback_id: String,
    },
    DispositionWithoutExplanation {
        feedback_id: String,
    },
}

/// Validate user-authored feedback before showing it against the current
/// revision or incorporating it into a new design brief. Feedback must remain
/// distinct from verified compliance findings: recording an accessibility
/// concern prompts assessment; it does not itself declare a violation.
pub fn validate_occupant_feedback_for_review(
    feedback: &[OccupantReviewFeedback],
    design_revision_id: &str,
    route: Option<&ArchitecturalReviewRoute>,
) -> Result<(), Vec<OccupantFeedbackIssue>> {
    let mut issues = Vec::new();
    if design_revision_id.trim().is_empty() {
        issues.push(OccupantFeedbackIssue::EmptyDesignRevisionId);
    }
    if route.is_some_and(|value| value.design_revision_id != design_revision_id) {
        for item in feedback {
            issues.push(OccupantFeedbackIssue::RouteRevisionMismatch {
                feedback_id: item.feedback_id.clone(),
            });
        }
    }

    let mut ids = std::collections::HashSet::new();
    for item in feedback {
        if item.feedback_id.trim().is_empty() {
            issues.push(OccupantFeedbackIssue::EmptyFeedbackId);
        } else if !ids.insert(item.feedback_id.as_str()) {
            issues.push(OccupantFeedbackIssue::DuplicateFeedbackId(
                item.feedback_id.clone(),
            ));
        }
        if item.design_revision_id.trim().is_empty() {
            issues.push(OccupantFeedbackIssue::EmptyDesignRevisionId);
        } else if item.design_revision_id != design_revision_id {
            issues.push(OccupantFeedbackIssue::RevisionMismatch {
                feedback_id: item.feedback_id.clone(),
            });
        }
        if item.note.trim().is_empty() {
            issues.push(OccupantFeedbackIssue::EmptyNote {
                feedback_id: item.feedback_id.clone(),
            });
        }
        if item
            .location_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            issues.push(OccupantFeedbackIssue::BlankLocationReference {
                feedback_id: item.feedback_id.clone(),
            });
        }

        if let Some(route_id) = &item.route_id {
            let Some(context_route) = route else {
                issues.push(OccupantFeedbackIssue::RouteContextUnavailable {
                    feedback_id: item.feedback_id.clone(),
                });
                continue;
            };
            if route_id != &context_route.route_id {
                issues.push(OccupantFeedbackIssue::RouteMismatch {
                    feedback_id: item.feedback_id.clone(),
                });
                continue;
            }
            if let Some(waypoint_id) = &item.waypoint_id
                && !context_route
                    .waypoints
                    .iter()
                    .any(|waypoint| &waypoint.waypoint_id == waypoint_id)
            {
                issues.push(OccupantFeedbackIssue::UnknownWaypoint {
                    feedback_id: item.feedback_id.clone(),
                    waypoint_id: waypoint_id.clone(),
                });
            }
        } else if item.waypoint_id.is_some() {
            issues.push(OccupantFeedbackIssue::WaypointWithoutRoute {
                feedback_id: item.feedback_id.clone(),
            });
        }

        if item.share_scope == OccupantFeedbackShareScope::PublicCaseStudy
            && !item.explicit_public_consent
        {
            issues.push(OccupantFeedbackIssue::PublicShareWithoutConsent {
                feedback_id: item.feedback_id.clone(),
            });
        }
        if item.lifecycle == OccupantFeedbackLifecycle::AcceptedForDesignChange
            && item
                .design_change_request_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            issues.push(OccupantFeedbackIssue::AcceptedWithoutChangeRequest {
                feedback_id: item.feedback_id.clone(),
            });
        }
        if matches!(
            item.lifecycle,
            OccupantFeedbackLifecycle::Deferred | OccupantFeedbackLifecycle::Resolved
        ) && item
            .disposition_note
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            issues.push(OccupantFeedbackIssue::DispositionWithoutExplanation {
                feedback_id: item.feedback_id.clone(),
            });
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// A disposition for feedback after a design iteration. These outcomes
/// describe the product workflow, not building-code compliance or proof that a
/// physical issue was fixed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FeedbackResolutionDisposition {
    ImplementedInRevision,
    NotChangedWithRationale,
    Deferred,
    IndependentAssessmentRequested,
    WithdrawnByOwner,
}

/// Append-only resolution record tying occupant feedback to the exact revision
/// against which it was reviewed. A redesign does not erase what the person saw
/// previously; implementation evidence belongs to the new revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FeedbackResolutionRecord {
    pub resolution_id: String,
    pub feedback_id: String,
    pub source_design_revision_id: String,
    pub reviewed_against_design_revision_id: String,
    pub disposition: FeedbackResolutionDisposition,
    pub rationale: String,
    pub design_change_request_id: Option<String>,
    pub implementation_evidence_artifact_id: Option<String>,
    pub implementation_evidence_sha256: Option<String>,
    /// Required for accessibility/circulation concerns marked implemented.
    /// This points to an independent review/assessment, not a model's own claim.
    pub independent_assessment_ref: Option<String>,
    /// Required when an owner withdraws their own feedback.
    pub owner_confirmation_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeedbackResolutionIssue {
    EmptyCurrentRevision,
    EmptyResolutionId,
    DuplicateResolutionId(String),
    DuplicateFeedbackResolutionForRevision {
        feedback_id: String,
        revision_id: String,
    },
    UnknownFeedback(String),
    SourceRevisionMismatch {
        feedback_id: String,
    },
    FeedbackNotAcceptedForChange {
        feedback_id: String,
    },
    ChangeRequestMismatch {
        feedback_id: String,
    },
    StaleReviewedRevision {
        resolution_id: String,
    },
    EmptyRationale {
        resolution_id: String,
    },
    MissingChangeRequest {
        resolution_id: String,
    },
    MissingImplementationEvidence {
        resolution_id: String,
    },
    IncompleteImplementationEvidencePair {
        resolution_id: String,
    },
    InvalidImplementationEvidenceDigest {
        resolution_id: String,
    },
    MissingIndependentAssessment {
        resolution_id: String,
    },
    MissingOwnerConfirmation {
        resolution_id: String,
    },
}

/// Check the closure between occupant feedback and design decisions for one
/// current revision. This provides a traceable iteration loop without converting
/// a subjective preference into a code finding or claiming that an observed
/// accessibility concern is solved without independent assessment.
pub fn validate_feedback_resolutions_for_revision(
    feedback: &[OccupantReviewFeedback],
    resolutions: &[FeedbackResolutionRecord],
    current_design_revision_id: &str,
) -> Result<(), Vec<FeedbackResolutionIssue>> {
    let mut issues = Vec::new();
    if current_design_revision_id.trim().is_empty() {
        issues.push(FeedbackResolutionIssue::EmptyCurrentRevision);
    }
    let feedback_by_id: std::collections::HashMap<&str, &OccupantReviewFeedback> = feedback
        .iter()
        .map(|item| (item.feedback_id.as_str(), item))
        .collect();
    let mut resolution_ids = std::collections::HashSet::new();
    let mut resolved_feedback_for_revision = std::collections::HashSet::new();

    for resolution in resolutions {
        if resolution.resolution_id.trim().is_empty() {
            issues.push(FeedbackResolutionIssue::EmptyResolutionId);
        } else if !resolution_ids.insert(resolution.resolution_id.as_str()) {
            issues.push(FeedbackResolutionIssue::DuplicateResolutionId(
                resolution.resolution_id.clone(),
            ));
        }
        if resolution.reviewed_against_design_revision_id != current_design_revision_id {
            issues.push(FeedbackResolutionIssue::StaleReviewedRevision {
                resolution_id: resolution.resolution_id.clone(),
            });
        }
        if !resolved_feedback_for_revision.insert((
            resolution.feedback_id.as_str(),
            resolution.reviewed_against_design_revision_id.as_str(),
        )) {
            issues.push(
                FeedbackResolutionIssue::DuplicateFeedbackResolutionForRevision {
                    feedback_id: resolution.feedback_id.clone(),
                    revision_id: resolution.reviewed_against_design_revision_id.clone(),
                },
            );
        }

        let Some(original) = feedback_by_id.get(resolution.feedback_id.as_str()).copied() else {
            issues.push(FeedbackResolutionIssue::UnknownFeedback(
                resolution.feedback_id.clone(),
            ));
            continue;
        };
        if resolution.source_design_revision_id != original.design_revision_id {
            issues.push(FeedbackResolutionIssue::SourceRevisionMismatch {
                feedback_id: resolution.feedback_id.clone(),
            });
        }
        if resolution.rationale.trim().is_empty() {
            issues.push(FeedbackResolutionIssue::EmptyRationale {
                resolution_id: resolution.resolution_id.clone(),
            });
        }

        let has_change_request = resolution
            .design_change_request_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let evidence_id = resolution
            .implementation_evidence_artifact_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let evidence_digest = resolution.implementation_evidence_sha256.as_deref();
        let has_evidence_digest = evidence_digest.is_some_and(|value| {
            value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        });

        match resolution.disposition {
            FeedbackResolutionDisposition::ImplementedInRevision => {
                if original.lifecycle != OccupantFeedbackLifecycle::AcceptedForDesignChange {
                    issues.push(FeedbackResolutionIssue::FeedbackNotAcceptedForChange {
                        feedback_id: resolution.feedback_id.clone(),
                    });
                }
                if original.design_change_request_id.as_deref()
                    != resolution.design_change_request_id.as_deref()
                {
                    issues.push(FeedbackResolutionIssue::ChangeRequestMismatch {
                        feedback_id: resolution.feedback_id.clone(),
                    });
                }
                if resolution.reviewed_against_design_revision_id
                    == resolution.source_design_revision_id
                {
                    issues.push(FeedbackResolutionIssue::StaleReviewedRevision {
                        resolution_id: resolution.resolution_id.clone(),
                    });
                }
                if !has_change_request {
                    issues.push(FeedbackResolutionIssue::MissingChangeRequest {
                        resolution_id: resolution.resolution_id.clone(),
                    });
                }
                match (evidence_id, evidence_digest) {
                    (true, Some(_)) if has_evidence_digest => {}
                    (false, None) => {
                        issues.push(FeedbackResolutionIssue::MissingImplementationEvidence {
                            resolution_id: resolution.resolution_id.clone(),
                        });
                    }
                    (true, Some(_)) => {
                        issues.push(
                            FeedbackResolutionIssue::InvalidImplementationEvidenceDigest {
                                resolution_id: resolution.resolution_id.clone(),
                            },
                        );
                    }
                    _ => {
                        issues.push(
                            FeedbackResolutionIssue::IncompleteImplementationEvidencePair {
                                resolution_id: resolution.resolution_id.clone(),
                            },
                        );
                    }
                }
                if matches!(
                    original.category,
                    OccupantFeedbackCategory::AccessibilityConcern
                        | OccupantFeedbackCategory::CirculationConcern
                ) && !resolution
                    .independent_assessment_ref
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    issues.push(FeedbackResolutionIssue::MissingIndependentAssessment {
                        resolution_id: resolution.resolution_id.clone(),
                    });
                }
            }
            FeedbackResolutionDisposition::IndependentAssessmentRequested => {
                if !resolution
                    .independent_assessment_ref
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    issues.push(FeedbackResolutionIssue::MissingIndependentAssessment {
                        resolution_id: resolution.resolution_id.clone(),
                    });
                }
            }
            FeedbackResolutionDisposition::WithdrawnByOwner => {
                if !resolution
                    .owner_confirmation_ref
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    issues.push(FeedbackResolutionIssue::MissingOwnerConfirmation {
                        resolution_id: resolution.resolution_id.clone(),
                    });
                }
            }
            FeedbackResolutionDisposition::NotChangedWithRationale
            | FeedbackResolutionDisposition::Deferred => {}
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Presentation mode used for one design-review session. Static plans are a
/// first-class mode, not a fallback error. HeadTrackedVrReference remains an
/// adapter declaration; this value does not prove headset support exists.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DesignReviewMode {
    StaticPlan,
    FloatingCamera,
    GuidedWalkthrough,
    OrbitReview,
    AccessibleTeleport,
    HeadTrackedVrReference,
}

impl DesignReviewMode {
    fn required_route_rig(self) -> Option<ReviewRig> {
        match self {
            Self::StaticPlan => None,
            Self::FloatingCamera => Some(ReviewRig::FloatingCamera),
            Self::GuidedWalkthrough => Some(ReviewRig::GuidedWalkthrough),
            Self::OrbitReview => Some(ReviewRig::OrbitReview),
            Self::AccessibleTeleport => Some(ReviewRig::AccessibleTeleport),
            Self::HeadTrackedVrReference => Some(ReviewRig::HeadTrackedVrReference),
        }
    }
}

/// Snapshot reference returned by the jurisdiction/rule-composition pipeline.
/// It pins review overlays to one profile and one immutable composed-rule artifact.
/// The viewer checks metadata and digest syntax only; bytes and legal applicability
/// still require the Mycelix evidence/source trust adapters.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewJurisdictionContext {
    pub jurisdiction_profile_id: String,
    pub jurisdiction_label: String,
    pub rule_context_artifact_id: String,
    pub rule_context_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewJurisdictionContextIssue {
    EmptyJurisdictionProfileId,
    EmptyJurisdictionLabel,
    EmptyRuleContextArtifactId,
    InvalidRuleContextDigest,
}

impl ReviewJurisdictionContext {
    pub fn validate(&self) -> Result<(), Vec<ReviewJurisdictionContextIssue>> {
        let mut issues = Vec::new();
        if self.jurisdiction_profile_id.trim().is_empty() {
            issues.push(ReviewJurisdictionContextIssue::EmptyJurisdictionProfileId);
        }
        if self.jurisdiction_label.trim().is_empty() {
            issues.push(ReviewJurisdictionContextIssue::EmptyJurisdictionLabel);
        }
        if self.rule_context_artifact_id.trim().is_empty() {
            issues.push(ReviewJurisdictionContextIssue::EmptyRuleContextArtifactId);
        }
        if self.rule_context_sha256.len() != 64
            || !self
                .rule_context_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            issues.push(ReviewJurisdictionContextIssue::InvalidRuleContextDigest);
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

/// Declared representation of the model being reviewed. These labels describe
/// provenance, not geometric accuracy or compliance; actual source bytes must
/// still be verified by the artifact store, and fidelity needs an independent check.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReviewModelSourceKind {
    SyntheticFixture,
    DesignExchange,
    NativeScene,
    SurveyedCapture,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReviewModelFormat {
    Ifc,
    Gltf,
    Glb,
    SymtropyScene,
    BevyScene,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReviewLinearUnit {
    Metre,
    Millimetre,
    Foot,
    Inch,
}

impl ReviewLinearUnit {
    /// Exact international conversion factors for the units supported here.
    pub const fn metres_per_unit(self) -> f64 {
        match self {
            Self::Metre => 1.0,
            Self::Millimetre => 0.001,
            Self::Foot => 0.3048,
            Self::Inch => 0.0254,
        }
    }
}

/// Convert coordinates from the declared source linear unit into review-space
/// metres. This scales values only; it does not transform axis orientation,
/// geographic datum, origin, handedness, or local/world coordinate frames.
/// Those conversions must be explicit in the model-import adapter.
pub fn scale_point_to_review_metres(
    source_position: [f64; 3],
    source_unit: ReviewLinearUnit,
) -> Option<[f32; 3]> {
    let scale = source_unit.metres_per_unit();
    let mut result = [0.0_f32; 3];
    for (target, source) in result.iter_mut().zip(source_position) {
        let value = source * scale;
        if !value.is_finite() || value.abs() > f32::MAX as f64 {
            return None;
        }
        *target = value as f32;
        if !target.is_finite() {
            return None;
        }
    }
    Some(result)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReviewModelFidelity {
    SyntheticGeometry,
    VisualApproximation,
    DesignGeometry,
    SurveyedAsBuiltGeometry,
}

/// Describes what a virtual walkthrough actually represents. A route uses
/// metres in the normalized review frame even when the source model uses
/// millimetres, feet or inches; an importer must perform and test that conversion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewModelContext {
    pub source_kind: ReviewModelSourceKind,
    pub source_format: ReviewModelFormat,
    pub fidelity: ReviewModelFidelity,
    pub source_linear_unit: ReviewLinearUnit,
    pub coordinate_frame_id: String,
    /// Explicit known gaps, placeholders, unmodelled systems and assumptions.
    /// Include a non-empty "not assessed" entry when no review has been done.
    pub known_limitations: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewModelContextIssue {
    EmptyCoordinateFrameId,
    MissingLimitations,
    BlankLimitation { index: usize },
    FidelitySourceKindMismatch,
}

impl ReviewModelContext {
    pub fn validate(&self) -> Result<(), Vec<ReviewModelContextIssue>> {
        let mut issues = Vec::new();
        if self.coordinate_frame_id.trim().is_empty() {
            issues.push(ReviewModelContextIssue::EmptyCoordinateFrameId);
        }
        if self.known_limitations.is_empty() {
            issues.push(ReviewModelContextIssue::MissingLimitations);
        }
        for (index, limitation) in self.known_limitations.iter().enumerate() {
            if limitation.trim().is_empty() {
                issues.push(ReviewModelContextIssue::BlankLimitation { index });
            }
        }
        if self.fidelity == ReviewModelFidelity::SurveyedAsBuiltGeometry
            && self.source_kind != ReviewModelSourceKind::SurveyedCapture
        {
            issues.push(ReviewModelContextIssue::FidelitySourceKindMismatch);
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

/// Current serialized review-session manifest schema. Bump this on breaking changes.
pub const DESIGN_REVIEW_SESSION_SCHEMA_VERSION: u32 = 3;

/// Review manifest tied to the exact homeowner brief and model artifact.
///
/// The snapshot digest should cover a canonical manifest representation that
/// excludes the digest field itself. The validator checks digest syntax and
/// reference consistency, not actual artifact bytes or immutable storage.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DesignReviewSession {
    pub schema_version: u32,
    pub session_id: String,
    pub owner_brief_revision_id: String,
    pub design_revision_id: String,
    pub model_artifact_id: String,
    pub model_sha256: String,
    /// Explicit representation/source context so a visual approximation is not
    /// confused with surveyed as-built geometry.
    pub model_context: ReviewModelContext,
    pub viewer_build_id: String,
    pub snapshot_manifest_artifact_id: String,
    /// Omitted only in the pre-hash payload produced by review_session_manifest_bytes.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub snapshot_sha256: String,
    /// Explicit consent and motion settings used for this review session.
    pub comfort_profile: ReviewComfortProfile,
    /// Optional for purely visual/creative exploration. Required whenever the
    /// session presents binding-rule or required-evidence overlays.
    pub jurisdiction_context: Option<ReviewJurisdictionContext>,
    pub mode: DesignReviewMode,
    pub route: Option<ArchitecturalReviewRoute>,
    pub annotations: Vec<ReviewAnnotation>,
    pub occupant_feedback: Vec<OccupantReviewFeedback>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesignReviewSessionIssue {
    EmptySessionId,
    EmptyOwnerBriefRevisionId,
    EmptyDesignRevisionId,
    EmptyModelArtifactId,
    InvalidModelDigest,
    EmptyViewerBuildId,
    EmptySnapshotManifestArtifactId,
    InvalidSnapshotDigest,
    UnsupportedSchemaVersion {
        found: u32,
    },
    StaticPlanMustNotHaveRoute,
    ModeRequiresRoute {
        mode: DesignReviewMode,
    },
    RouteRigDoesNotMatchMode {
        mode: DesignReviewMode,
        rig: ReviewRig,
    },
    RouteDesignRevisionMismatch,
    InvalidRoute(Vec<ReviewRouteIssue>),
    InvalidAnnotations(Vec<ReviewAnnotationIssue>),
    InvalidOccupantFeedback(Vec<OccupantFeedbackIssue>),
    InvalidComfortProfile(Vec<ReviewComfortProfileIssue>),
    InvalidJurisdictionContext(Vec<ReviewJurisdictionContextIssue>),
    BindingAnnotationsWithoutJurisdictionContext,
    InvalidModelContext(Vec<ReviewModelContextIssue>),
}

/// Validate the provenance closure of one review session before showing it as a
/// review of a particular model revision. This ensures the route, annotations,
/// and feedback describe the same model revision. It does not prove that
/// evidence bytes match their digests or that the design is compliant.
pub fn validate_design_review_session(
    session: &DesignReviewSession,
) -> Result<(), Vec<DesignReviewSessionIssue>> {
    let mut issues = Vec::new();
    if session.schema_version != DESIGN_REVIEW_SESSION_SCHEMA_VERSION {
        issues.push(DesignReviewSessionIssue::UnsupportedSchemaVersion {
            found: session.schema_version,
        });
    }
    if session.session_id.trim().is_empty() {
        issues.push(DesignReviewSessionIssue::EmptySessionId);
    }
    if session.owner_brief_revision_id.trim().is_empty() {
        issues.push(DesignReviewSessionIssue::EmptyOwnerBriefRevisionId);
    }
    if session.design_revision_id.trim().is_empty() {
        issues.push(DesignReviewSessionIssue::EmptyDesignRevisionId);
    }
    if session.model_artifact_id.trim().is_empty() {
        issues.push(DesignReviewSessionIssue::EmptyModelArtifactId);
    }
    if session.model_sha256.len() != 64
        || !session
            .model_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        issues.push(DesignReviewSessionIssue::InvalidModelDigest);
    }
    if let Err(context_issues) = session.model_context.validate() {
        issues.push(DesignReviewSessionIssue::InvalidModelContext(
            context_issues,
        ));
    }
    if session.viewer_build_id.trim().is_empty() {
        issues.push(DesignReviewSessionIssue::EmptyViewerBuildId);
    }
    if session.snapshot_manifest_artifact_id.trim().is_empty() {
        issues.push(DesignReviewSessionIssue::EmptySnapshotManifestArtifactId);
    }
    if session.snapshot_sha256.len() != 64
        || !session
            .snapshot_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        issues.push(DesignReviewSessionIssue::InvalidSnapshotDigest);
    }
    if let Err(profile_issues) = session.comfort_profile.validate() {
        issues.push(DesignReviewSessionIssue::InvalidComfortProfile(
            profile_issues,
        ));
    }
    if let Some(context) = &session.jurisdiction_context {
        if let Err(context_issues) = context.validate() {
            issues.push(DesignReviewSessionIssue::InvalidJurisdictionContext(
                context_issues,
            ));
        }
    }
    let has_binding_annotations = session.annotations.iter().any(|annotation| {
        matches!(
            annotation.kind,
            ReviewAnnotationKind::BindingRequirement
                | ReviewAnnotationKind::BindingConflict
                | ReviewAnnotationKind::RequiredEvidence
        )
    });
    if has_binding_annotations && session.jurisdiction_context.is_none() {
        issues.push(DesignReviewSessionIssue::BindingAnnotationsWithoutJurisdictionContext);
    }

    match (session.mode.required_route_rig(), session.route.as_ref()) {
        (None, Some(_)) => issues.push(DesignReviewSessionIssue::StaticPlanMustNotHaveRoute),
        (Some(mode), None) => {
            issues.push(DesignReviewSessionIssue::ModeRequiresRoute { mode });
        }
        (Some(expected), Some(route)) => {
            if route.rig != expected {
                issues.push(DesignReviewSessionIssue::RouteRigDoesNotMatchMode {
                    mode: session.mode,
                    rig: route.rig,
                });
            }
            if route.design_revision_id != session.design_revision_id {
                issues.push(DesignReviewSessionIssue::RouteDesignRevisionMismatch);
            }
            if let Err(route_issues) = validate_review_route(route) {
                issues.push(DesignReviewSessionIssue::InvalidRoute(route_issues));
            }
        }
        (None, None) => {}
    }

    if let Err(annotation_issues) =
        validate_review_annotations_for_revision(&session.annotations, &session.design_revision_id)
    {
        issues.push(DesignReviewSessionIssue::InvalidAnnotations(
            annotation_issues,
        ));
    }
    if let Err(feedback_issues) = validate_occupant_feedback_for_review(
        &session.occupant_feedback,
        &session.design_revision_id,
        session.route.as_ref(),
    ) {
        issues.push(DesignReviewSessionIssue::InvalidOccupantFeedback(
            feedback_issues,
        ));
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Produce compact JSON bytes for a review-session manifest with its own digest
/// field blanked, avoiding a self-referential hash. Hash/store these bytes with a
/// trusted evidence adapter. This versioned field-order encoding is deterministic
/// for this schema but does not claim RFC 8785 canonical JSON conformance.
pub fn review_session_manifest_bytes(
    session: &DesignReviewSession,
) -> Result<Vec<u8>, serde_json::Error> {
    let mut payload = session.clone();
    payload.snapshot_sha256.clear();
    serde_json::to_vec(&payload)
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

    fn sample_route_fixture() -> ArchitecturalReviewRoute {
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

    fn sample_feedback(category: OccupantFeedbackCategory) -> OccupantReviewFeedback {
        OccupantReviewFeedback {
            feedback_id: "feedback:privacy-east-window".into(),
            design_revision_id: "house:r1".into(),
            route_id: None,
            waypoint_id: None,
            location_ref: Some("room:living-east".into()),
            category,
            priority: OccupantFeedbackPriority::Important,
            note: "Morning sun is pleasant but window visibility feels too exposed".into(),
            desired_change: Some("Compare a screened courtyard-facing alternative".into()),
            share_scope: OccupantFeedbackShareScope::Private,
            explicit_public_consent: false,
            lifecycle: OccupantFeedbackLifecycle::NeedsDesignIteration,
            disposition_note: None,
            design_change_request_id: None,
        }
    }

    fn sample_resolution(disposition: FeedbackResolutionDisposition) -> FeedbackResolutionRecord {
        FeedbackResolutionRecord {
            resolution_id: "resolution:privacy-r1-to-r2".into(),
            feedback_id: "feedback:privacy-east-window".into(),
            source_design_revision_id: "house:r1".into(),
            reviewed_against_design_revision_id: "house:r2".into(),
            disposition,
            rationale: "Compared alternatives and documented the design decision".into(),
            design_change_request_id: Some("change:window-screen-v2".into()),
            implementation_evidence_artifact_id: Some("capture:house-r2".into()),
            implementation_evidence_sha256: Some("a".repeat(64)),
            independent_assessment_ref: None,
            owner_confirmation_ref: None,
        }
    }

    #[test]
    fn implemented_preference_resolution_requires_revision_and_evidence_trace() {
        let mut feedback = sample_feedback(OccupantFeedbackCategory::PersonalPreference);
        feedback.lifecycle = OccupantFeedbackLifecycle::AcceptedForDesignChange;
        feedback.design_change_request_id = Some("change:window-screen-v2".into());
        let resolution = sample_resolution(FeedbackResolutionDisposition::ImplementedInRevision);
        assert_eq!(
            validate_feedback_resolutions_for_revision(&[feedback], &[resolution], "house:r2"),
            Ok(())
        );
    }

    #[test]
    fn accessibility_concern_cannot_be_marked_implemented_without_assessment() {
        let mut feedback = sample_feedback(OccupantFeedbackCategory::AccessibilityConcern);
        feedback.lifecycle = OccupantFeedbackLifecycle::AcceptedForDesignChange;
        feedback.design_change_request_id = Some("change:window-screen-v2".into());
        let resolution = sample_resolution(FeedbackResolutionDisposition::ImplementedInRevision);
        assert!(
            validate_feedback_resolutions_for_revision(&[feedback], &[resolution], "house:r2")
                .unwrap_err()
                .contains(&FeedbackResolutionIssue::MissingIndependentAssessment {
                    resolution_id: "resolution:privacy-r1-to-r2".into(),
                })
        );
    }

    #[test]
    fn implementation_cannot_skip_acceptance_or_change_request_linkage() {
        let feedback = vec![sample_feedback(
            OccupantFeedbackCategory::PersonalPreference,
        )];
        let resolution = sample_resolution(FeedbackResolutionDisposition::ImplementedInRevision);
        let issues =
            validate_feedback_resolutions_for_revision(&feedback, &[resolution], "house:r2")
                .unwrap_err();
        assert!(
            issues.contains(&FeedbackResolutionIssue::FeedbackNotAcceptedForChange {
                feedback_id: "feedback:privacy-east-window".into(),
            })
        );
        assert!(
            issues.contains(&FeedbackResolutionIssue::ChangeRequestMismatch {
                feedback_id: "feedback:privacy-east-window".into(),
            })
        );
    }

    #[test]
    fn stale_feedback_resolution_cannot_be_applied_to_the_latest_revision() {
        let feedback = vec![sample_feedback(
            OccupantFeedbackCategory::PersonalPreference,
        )];
        let mut resolution =
            sample_resolution(FeedbackResolutionDisposition::NotChangedWithRationale);
        resolution.reviewed_against_design_revision_id = "house:r1".into();
        assert!(
            validate_feedback_resolutions_for_revision(&feedback, &[resolution], "house:r2")
                .unwrap_err()
                .iter()
                .any(|issue| matches!(
                    issue,
                    FeedbackResolutionIssue::StaleReviewedRevision { resolution_id }
                        if resolution_id == "resolution:privacy-r1-to-r2"
                ))
        );
    }

    #[test]
    fn owner_withdrawal_requires_explicit_confirmation() {
        let feedback = vec![sample_feedback(
            OccupantFeedbackCategory::PersonalPreference,
        )];
        let resolution = sample_resolution(FeedbackResolutionDisposition::WithdrawnByOwner);
        assert!(
            validate_feedback_resolutions_for_revision(&feedback, &[resolution], "house:r2")
                .unwrap_err()
                .contains(&FeedbackResolutionIssue::MissingOwnerConfirmation {
                    resolution_id: "resolution:privacy-r1-to-r2".into(),
                })
        );
    }

    #[test]
    fn non_implemented_preference_can_be_explained_without_becoming_a_code_failure() {
        let feedback = vec![sample_feedback(
            OccupantFeedbackCategory::PersonalPreference,
        )];
        let mut resolution =
            sample_resolution(FeedbackResolutionDisposition::NotChangedWithRationale);
        resolution.design_change_request_id = None;
        resolution.implementation_evidence_artifact_id = None;
        resolution.implementation_evidence_sha256 = None;
        assert_eq!(
            validate_feedback_resolutions_for_revision(&feedback, &[resolution], "house:r2"),
            Ok(())
        );
    }

    fn sample_review_session(mode: DesignReviewMode) -> DesignReviewSession {
        let route = (mode != DesignReviewMode::StaticPlan).then(|| {
            let mut route = sample_route_fixture();
            route.rig = mode
                .required_route_rig()
                .expect("non-static mode has a rig");
            route
        });
        DesignReviewSession {
            schema_version: DESIGN_REVIEW_SESSION_SCHEMA_VERSION,
            session_id: "review-session:house-r1".into(),
            owner_brief_revision_id: "owner-brief:r4".into(),
            design_revision_id: "house:r1".into(),
            model_artifact_id: "model:house-r1".into(),
            model_sha256: "a".repeat(64),
            model_context: ReviewModelContext {
                source_kind: ReviewModelSourceKind::SyntheticFixture,
                source_format: ReviewModelFormat::BevyScene,
                fidelity: ReviewModelFidelity::SyntheticGeometry,
                source_linear_unit: ReviewLinearUnit::Metre,
                coordinate_frame_id: "synthetic-room:local-metres".into(),
                known_limitations: vec![
                    "Synthetic room proportions; not a measured or approved home".into(),
                    "No collision, structural, daylight, energy, acoustic, or code analysis".into(),
                ],
            },
            viewer_build_id: "symtropy-build:dev-test".into(),
            snapshot_manifest_artifact_id: "review-manifest:house-r1".into(),
            snapshot_sha256: "b".repeat(64),
            comfort_profile: ReviewComfortProfile::default(),
            jurisdiction_context: None,
            mode,
            route,
            annotations: Vec::new(),
            occupant_feedback: Vec::new(),
        }
    }

    #[test]
    fn static_plan_review_is_a_valid_first_class_session() {
        let session = sample_review_session(DesignReviewMode::StaticPlan);
        assert_eq!(validate_design_review_session(&session), Ok(()));
    }

    #[test]
    fn floating_camera_session_requires_matching_revision_and_mode() {
        let session = sample_review_session(DesignReviewMode::FloatingCamera);
        assert_eq!(validate_design_review_session(&session), Ok(()));
    }

    #[test]
    fn review_session_rejects_route_from_an_old_revision() {
        let mut session = sample_review_session(DesignReviewMode::FloatingCamera);
        session.route.as_mut().unwrap().design_revision_id = "house:r0".into();
        assert!(
            validate_design_review_session(&session)
                .unwrap_err()
                .contains(&DesignReviewSessionIssue::RouteDesignRevisionMismatch)
        );
    }

    #[test]
    fn review_session_rejects_mode_route_mismatch() {
        let mut session = sample_review_session(DesignReviewMode::GuidedWalkthrough);
        session.route.as_mut().unwrap().rig = ReviewRig::FloatingCamera;
        assert!(
            validate_design_review_session(&session)
                .unwrap_err()
                .contains(&DesignReviewSessionIssue::RouteRigDoesNotMatchMode {
                    mode: DesignReviewMode::GuidedWalkthrough,
                    rig: ReviewRig::FloatingCamera,
                })
        );
    }

    #[test]
    fn review_session_manifest_is_deterministic_and_excludes_its_digest() {
        let first = sample_review_session(DesignReviewMode::StaticPlan);
        let mut second = first.clone();
        second.snapshot_sha256 = "c".repeat(64);

        let first_bytes = review_session_manifest_bytes(&first).unwrap();
        let second_bytes = review_session_manifest_bytes(&second).unwrap();
        assert_eq!(first_bytes, second_bytes);
        let decoded: serde_json::Value = serde_json::from_slice(&first_bytes).unwrap();
        assert_eq!(
            decoded["schema_version"],
            DESIGN_REVIEW_SESSION_SCHEMA_VERSION
        );
        assert!(decoded.get("snapshot_sha256").is_none());
    }

    #[test]
    fn review_session_manifest_json_round_trips() {
        let session = sample_review_session(DesignReviewMode::FloatingCamera);
        let bytes = serde_json::to_vec(&session).unwrap();
        let restored: DesignReviewSession = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored, session);
    }

    fn sample_jurisdiction_context() -> ReviewJurisdictionContext {
        ReviewJurisdictionContext {
            jurisdiction_profile_id: "ZA/GP/Johannesburg".into(),
            jurisdiction_label: "Johannesburg, Gauteng, South Africa".into(),
            rule_context_artifact_id: "rule-context:za-jhb:r3".into(),
            rule_context_sha256: "d".repeat(64),
        }
    }

    #[test]
    fn synthetic_model_context_fixture_round_trips_and_retains_limitations() {
        let fixture = include_str!("../fixtures/synthetic-review-model-context-v1.json");
        let context: ReviewModelContext =
            serde_json::from_str(fixture).expect("model-context fixture must match serde schema");
        assert_eq!(context.source_kind, ReviewModelSourceKind::SyntheticFixture);
        assert_eq!(context.source_format, ReviewModelFormat::BevyScene);
        assert_eq!(context.fidelity, ReviewModelFidelity::SyntheticGeometry);
        assert_eq!(context.source_linear_unit, ReviewLinearUnit::Metre);
        assert_eq!(context.validate(), Ok(()));
        assert!(
            context
                .known_limitations
                .iter()
                .any(|value| value.contains("not a measured"))
        );
        assert!(serde_json::from_str::<ReviewModelContext>(fixture).is_ok());
    }

    #[test]
    fn source_linear_units_are_scaled_to_review_metres() {
        assert_eq!(
            scale_point_to_review_metres([1200.0, 2500.0, 3000.0], ReviewLinearUnit::Millimetre),
            Some([1.2, 2.5, 3.0])
        );
        assert_eq!(
            scale_point_to_review_metres([10.0, 0.0, 0.0], ReviewLinearUnit::Foot),
            Some([3.048, 0.0, 0.0])
        );
        assert_eq!(
            scale_point_to_review_metres([10.0, 0.0, 0.0], ReviewLinearUnit::Inch),
            Some([0.254, 0.0, 0.0])
        );
    }

    #[test]
    fn source_unit_conversion_rejects_nonfinite_and_overflow_values() {
        assert_eq!(
            scale_point_to_review_metres([f64::NAN, 0.0, 0.0], ReviewLinearUnit::Metre),
            None
        );
        assert_eq!(
            scale_point_to_review_metres([f64::MAX, 0.0, 0.0], ReviewLinearUnit::Foot),
            None
        );
    }

    #[test]
    fn virtual_review_requires_explicit_model_fidelity_and_limitations() {
        let mut session = sample_review_session(DesignReviewMode::StaticPlan);
        session.model_context.known_limitations.clear();
        assert!(
            validate_design_review_session(&session)
                .unwrap_err()
                .iter()
                .any(|issue| matches!(
                    issue,
                    DesignReviewSessionIssue::InvalidModelContext(context_issues)
                        if context_issues.contains(&ReviewModelContextIssue::MissingLimitations)
                ))
        );

        session.model_context.known_limitations =
            vec!["Synthetic geometry; dimensions not verified".into()];
        session.model_context.fidelity = ReviewModelFidelity::SurveyedAsBuiltGeometry;
        assert!(validate_design_review_session(&session)
            .unwrap_err()
            .iter()
            .any(|issue| matches!(
                issue,
                DesignReviewSessionIssue::InvalidModelContext(context_issues)
                    if context_issues.contains(&ReviewModelContextIssue::FidelitySourceKindMismatch)
            )));
    }

    #[test]
    fn model_context_rejects_blank_coordinate_frame_and_blank_limitations() {
        let mut session = sample_review_session(DesignReviewMode::StaticPlan);
        session.model_context.coordinate_frame_id.clear();
        session.model_context.known_limitations = vec!["  ".into()];
        let issues = validate_design_review_session(&session).unwrap_err();
        assert!(issues.iter().any(|issue| matches!(
            issue,
            DesignReviewSessionIssue::InvalidModelContext(context_issues)
                if context_issues.contains(&ReviewModelContextIssue::EmptyCoordinateFrameId)
                    && context_issues
                        .contains(&ReviewModelContextIssue::BlankLimitation { index: 0 })
        )));
    }

    #[test]
    fn binding_overlays_require_a_pinned_jurisdiction_rule_snapshot() {
        let mut session = sample_review_session(DesignReviewMode::StaticPlan);
        let mut binding = annotation(
            "rule:egress",
            "house:r1",
            ReviewAnnotationKind::BindingRequirement,
            ReviewAnnotationState::Unknown,
        );
        binding.source_ref = Some("clause:building-regulation:egress".into());
        binding.module_id = Some("ZA/GP/Johannesburg".into());
        session.annotations.push(binding);
        assert!(
            validate_design_review_session(&session)
                .unwrap_err()
                .contains(&DesignReviewSessionIssue::BindingAnnotationsWithoutJurisdictionContext)
        );

        session.jurisdiction_context = Some(sample_jurisdiction_context());
        assert_eq!(validate_design_review_session(&session), Ok(()));
    }

    #[test]
    fn invalid_jurisdiction_rule_snapshot_digest_blocks_session() {
        let mut session = sample_review_session(DesignReviewMode::StaticPlan);
        let mut context = sample_jurisdiction_context();
        context.rule_context_sha256 = "not-a-digest".into();
        session.jurisdiction_context = Some(context);
        assert!(
            validate_design_review_session(&session)
                .unwrap_err()
                .iter()
                .any(|issue| matches!(
                    issue,
                    DesignReviewSessionIssue::InvalidJurisdictionContext(context_issues)
                        if context_issues.contains(
                            &ReviewJurisdictionContextIssue::InvalidRuleContextDigest
                        )
                ))
        );
    }

    #[test]
    fn review_session_rejects_unknown_schema_version() {
        let mut session = sample_review_session(DesignReviewMode::StaticPlan);
        session.schema_version = DESIGN_REVIEW_SESSION_SCHEMA_VERSION + 1;
        assert!(
            validate_design_review_session(&session)
                .unwrap_err()
                .contains(&DesignReviewSessionIssue::UnsupportedSchemaVersion {
                    found: DESIGN_REVIEW_SESSION_SCHEMA_VERSION + 1,
                })
        );
    }

    #[test]
    fn review_session_rejects_malformed_model_digest() {
        let mut session = sample_review_session(DesignReviewMode::StaticPlan);
        session.model_sha256 = "not-a-digest".into();
        assert!(
            validate_design_review_session(&session)
                .unwrap_err()
                .contains(&DesignReviewSessionIssue::InvalidModelDigest)
        );
    }

    fn annotation(
        id: &str,
        revision: &str,
        kind: ReviewAnnotationKind,
        state: ReviewAnnotationState,
    ) -> ReviewAnnotation {
        ReviewAnnotation {
            annotation_id: id.into(),
            design_revision_id: revision.into(),
            kind,
            state,
            title: id.into(),
            explanation: "Synthetic review annotation".into(),
            source_ref: None,
            module_id: None,
            related_sources: Vec::new(),
            evidence_artifact_id: None,
            evidence_sha256: None,
            uncertainty_note: None,
        }
    }

    #[test]
    fn owner_preference_is_not_treated_as_a_code_violation() {
        let preference = annotation(
            "owner:arched-window",
            "house:r1",
            ReviewAnnotationKind::OwnerPreference,
            ReviewAnnotationState::UnfulfilledPreference,
        );
        assert_eq!(
            validate_review_annotations_for_revision(&[preference], "house:r1"),
            Ok(())
        );
    }

    #[test]
    fn owner_preference_cannot_be_mislabeled_as_a_binding_failure() {
        let preference = annotation(
            "owner:arched-window",
            "house:r1",
            ReviewAnnotationKind::OwnerPreference,
            ReviewAnnotationState::Failed,
        );
        assert!(
            validate_review_annotations_for_revision(&[preference], "house:r1")
                .unwrap_err()
                .contains(&ReviewAnnotationIssue::InvalidKindStateCombination {
                    annotation_id: "owner:arched-window".into(),
                })
        );
    }

    #[test]
    fn binding_overlay_requires_module_and_source_provenance() {
        let binding = annotation(
            "rule:egress",
            "house:r1",
            ReviewAnnotationKind::BindingRequirement,
            ReviewAnnotationState::Unknown,
        );
        assert!(
            validate_review_annotations_for_revision(&[binding], "house:r1")
                .unwrap_err()
                .contains(&ReviewAnnotationIssue::MissingBindingProvenance {
                    annotation_id: "rule:egress".into(),
                })
        );
    }

    #[test]
    fn missing_required_evidence_cannot_be_marked_not_applicable() {
        let mut missing = annotation(
            "evidence:fire-review",
            "house:r1",
            ReviewAnnotationKind::RequiredEvidence,
            ReviewAnnotationState::NotApplicable,
        );
        missing.source_ref = Some("rule:fire-review".into());
        missing.module_id = Some("jurisdiction:pilot".into());
        assert!(
            validate_review_annotations_for_revision(&[missing], "house:r1")
                .unwrap_err()
                .contains(&ReviewAnnotationIssue::InvalidKindStateCombination {
                    annotation_id: "evidence:fire-review".into(),
                })
        );
    }

    #[test]
    fn an_unresolved_binding_dispute_must_use_conflict_annotation_kind() {
        let conflict = annotation(
            "rule:ventilation",
            "house:r1",
            ReviewAnnotationKind::BindingRequirement,
            ReviewAnnotationState::Conflict,
        );
        assert!(
            validate_review_annotations_for_revision(&[conflict], "house:r1")
                .unwrap_err()
                .contains(&ReviewAnnotationIssue::InvalidKindStateCombination {
                    annotation_id: "rule:ventilation".into(),
                })
        );
    }

    #[test]
    fn binding_requirement_cannot_be_labeled_as_an_unfulfilled_preference() {
        let mut binding = annotation(
            "rule:egress",
            "house:r1",
            ReviewAnnotationKind::BindingRequirement,
            ReviewAnnotationState::UnfulfilledPreference,
        );
        binding.source_ref = Some("section:egress".into());
        binding.module_id = Some("module:ZA/JHB:current".into());
        assert!(
            validate_review_annotations_for_revision(&[binding], "house:r1")
                .unwrap_err()
                .contains(&ReviewAnnotationIssue::InvalidKindStateCombination {
                    annotation_id: "rule:egress".into(),
                })
        );
    }

    #[test]
    fn binding_conflict_must_show_two_distinct_sources() {
        let mut conflict = annotation(
            "conflict:window",
            "house:r1",
            ReviewAnnotationKind::BindingConflict,
            ReviewAnnotationState::Conflict,
        );
        conflict.source_ref = Some("section:local".into());
        conflict.module_id = Some("module:local".into());
        assert!(
            validate_review_annotations_for_revision(&[conflict.clone()], "house:r1")
                .unwrap_err()
                .contains(&ReviewAnnotationIssue::MissingConflictSources {
                    annotation_id: "conflict:window".into(),
                })
        );

        conflict.related_sources.push(ReviewSourceRef {
            module_id: "module:national".into(),
            source_ref: "section:national".into(),
        });
        assert_eq!(
            validate_review_annotations_for_revision(&[conflict], "house:r1"),
            Ok(())
        );
    }

    #[test]
    fn satisfied_engineering_annotation_requires_evidence_artifact() {
        let engineering = annotation(
            "engineering:daylight",
            "house:r1",
            ReviewAnnotationKind::EngineeringEvidence,
            ReviewAnnotationState::Satisfied,
        );
        assert!(
            validate_review_annotations_for_revision(&[engineering], "house:r1")
                .unwrap_err()
                .contains(&ReviewAnnotationIssue::MissingResultEvidenceArtifact {
                    annotation_id: "engineering:daylight".into(),
                })
        );
    }

    #[test]
    fn unknown_site_overlay_requires_a_specific_uncertainty_note() {
        let mut site = annotation(
            "site:flood-level",
            "house:r1",
            ReviewAnnotationKind::SiteUnknown,
            ReviewAnnotationState::Unknown,
        );
        assert!(
            validate_review_annotations_for_revision(&[site.clone()], "house:r1")
                .unwrap_err()
                .contains(&ReviewAnnotationIssue::MissingUncertaintyNote {
                    annotation_id: "site:flood-level".into(),
                })
        );

        site.uncertainty_note =
            Some("No dated parcel-specific flood assessment is in the evidence set".into());
        assert_eq!(
            validate_review_annotations_for_revision(&[site], "house:r1"),
            Ok(())
        );
    }

    #[test]
    fn proposals_and_guidance_remain_informational_in_the_viewer() {
        let guidance = annotation(
            "guidance:ventilation",
            "house:r1",
            ReviewAnnotationKind::NonBindingGuidance,
            ReviewAnnotationState::Informational,
        );
        let reform = annotation(
            "reform:window-alternative",
            "house:r1",
            ReviewAnnotationKind::CodeReformProposal,
            ReviewAnnotationState::Informational,
        );
        assert_eq!(
            validate_review_annotations_for_revision(&[guidance, reform], "house:r1"),
            Ok(())
        );
    }

    #[test]
    fn stale_revision_annotations_are_rejected() {
        let stale = annotation(
            "evidence:energy",
            "house:r1",
            ReviewAnnotationKind::EngineeringEvidence,
            ReviewAnnotationState::Unknown,
        );
        assert!(
            validate_review_annotations_for_revision(&[stale], "house:r2")
                .unwrap_err()
                .contains(&ReviewAnnotationIssue::RevisionMismatch {
                    annotation_id: "evidence:energy".into(),
                })
        );
    }

    fn occupant_feedback(
        id: &str,
        revision: &str,
        category: OccupantFeedbackCategory,
    ) -> OccupantReviewFeedback {
        OccupantReviewFeedback {
            feedback_id: id.into(),
            design_revision_id: revision.into(),
            route_id: None,
            waypoint_id: None,
            location_ref: Some("room:kitchen".into()),
            category,
            priority: OccupantFeedbackPriority::Important,
            note: "The room feels cramped around the preparation area.".into(),
            desired_change: Some("Explore a wider circulation path.".into()),
            share_scope: OccupantFeedbackShareScope::Private,
            explicit_public_consent: false,
            lifecycle: OccupantFeedbackLifecycle::Captured,
            disposition_note: None,
            design_change_request_id: None,
        }
    }

    #[test]
    fn owner_preference_is_valid_feedback_not_a_code_failure() {
        let feedback = occupant_feedback(
            "feedback:arched-window",
            "house:r1",
            OccupantFeedbackCategory::PersonalPreference,
        );
        assert_eq!(
            validate_occupant_feedback_for_review(&[feedback], "house:r1", None),
            Ok(())
        );
    }

    #[test]
    fn old_revision_feedback_cannot_be_silently_attached_to_a_new_revision() {
        let feedback = occupant_feedback(
            "feedback:kitchen-circulation",
            "house:r1",
            OccupantFeedbackCategory::CirculationConcern,
        );
        assert!(
            validate_occupant_feedback_for_review(&[feedback], "house:r2", None)
                .unwrap_err()
                .contains(&OccupantFeedbackIssue::RevisionMismatch {
                    feedback_id: "feedback:kitchen-circulation".into(),
                })
        );
    }

    #[test]
    fn waypoint_feedback_must_reference_the_selected_route() {
        let mut feedback = occupant_feedback(
            "feedback:entry",
            "house-model:r7",
            OccupantFeedbackCategory::AccessibilityConcern,
        );
        feedback.route_id = Some("home-review:accessible-tour".into());
        feedback.waypoint_id = Some("entry".into());
        let route = sample_route_fixture();
        assert_eq!(
            validate_occupant_feedback_for_review(&[feedback], "house-model:r7", Some(&route)),
            Ok(())
        );
    }

    #[test]
    fn unknown_waypoint_feedback_is_rejected() {
        let mut feedback = occupant_feedback(
            "feedback:unknown-stop",
            "house-model:r7",
            OccupantFeedbackCategory::SpatialComfort,
        );
        feedback.route_id = Some("home-review:accessible-tour".into());
        feedback.waypoint_id = Some("not-in-route".into());
        let route = sample_route_fixture();
        assert!(
            validate_occupant_feedback_for_review(&[feedback], "house-model:r7", Some(&route))
                .unwrap_err()
                .contains(&OccupantFeedbackIssue::UnknownWaypoint {
                    feedback_id: "feedback:unknown-stop".into(),
                    waypoint_id: "not-in-route".into(),
                })
        );
    }

    #[test]
    fn public_case_study_requires_explicit_consent() {
        let mut feedback = occupant_feedback(
            "feedback:living-area",
            "house:r1",
            OccupantFeedbackCategory::SpatialComfort,
        );
        feedback.share_scope = OccupantFeedbackShareScope::PublicCaseStudy;
        assert!(
            validate_occupant_feedback_for_review(&[feedback], "house:r1", None)
                .unwrap_err()
                .contains(&OccupantFeedbackIssue::PublicShareWithoutConsent {
                    feedback_id: "feedback:living-area".into(),
                })
        );
    }

    #[test]
    fn accepted_design_feedback_must_link_a_change_request() {
        let mut feedback = occupant_feedback(
            "feedback:storage",
            "house:r1",
            OccupantFeedbackCategory::StorageAndErgonomics,
        );
        feedback.lifecycle = OccupantFeedbackLifecycle::AcceptedForDesignChange;
        assert!(
            validate_occupant_feedback_for_review(&[feedback], "house:r1", None)
                .unwrap_err()
                .contains(&OccupantFeedbackIssue::AcceptedWithoutChangeRequest {
                    feedback_id: "feedback:storage".into(),
                })
        );
    }

    #[test]
    fn valid_route_has_a_deterministic_midpoint_pose() {
        let route = sample_route_fixture();
        assert_eq!(validate_review_route(&route), Ok(()));
        let pose = sample_review_route(&route, 3.25).expect("route midpoint must sample");
        assert!((pose.translation.z - 2.0).abs() < 1.0e-5);
    }

    #[test]
    fn route_holds_at_stops_and_clamps_after_the_final_stop() {
        let route = sample_route_fixture();
        let at_entry = sample_review_route(&route, 1.0).unwrap();
        let after_end = sample_review_route(&route, 100.0).unwrap();
        assert_eq!(at_entry.translation, Vec3::new(0.0, 1.6, 4.0));
        assert_eq!(after_end.translation, Vec3::new(3.0, 1.6, 0.0));
    }

    #[test]
    fn duplicated_waypoint_ids_are_rejected() {
        let mut route = sample_route_fixture();
        route.waypoints[1].waypoint_id = route.waypoints[0].waypoint_id.clone();
        assert!(
            validate_review_route(&route)
                .unwrap_err()
                .contains(&ReviewRouteIssue::DuplicateWaypointId("entry".into()))
        );
        assert!(sample_review_route(&route, 1.0).is_none());
    }

    #[test]
    fn route_rejects_non_finite_coordinates_and_invalid_time() {
        let mut route = sample_route_fixture();
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
        let mut route = sample_route_fixture();
        route.waypoints[1].arrival_seconds = 2.1;
        route.max_translation_speed_mps = 0.5;
        assert!(
            validate_review_route(&route)
                .unwrap_err()
                .iter()
                .any(|issue| {
                    matches!(issue, ReviewRouteIssue::TranslationSpeedExceeded { .. })
                })
        );
    }

    #[test]
    fn route_requires_dwell_and_travel_time_not_to_overlap() {
        let mut route = sample_route_fixture();
        route.waypoints[1].arrival_seconds = 1.0;
        assert!(validate_review_route(&route).unwrap_err().iter().any(|issue| {
            matches!(
                issue,
                ReviewRouteIssue::TimelineOverlap { waypoint_id } if waypoint_id == "kitchen"
            )
        }));
    }

    #[test]
    fn degenerate_camera_look_direction_is_rejected() {
        let mut route = sample_route_fixture();
        route.waypoints[1].look_at_m = route.waypoints[1].position_m;
        assert!(validate_review_route(&route).unwrap_err().iter().any(|issue| {
            matches!(
                issue,
                ReviewRouteIssue::DegenerateLookDirection { waypoint_id } if waypoint_id == "kitchen"
            )
        }));
    }

    #[test]
    fn free_camera_toggle_pauses_route_and_restart_resets_elapsed_time() {
        let camera_pose =
            Transform::from_xyz(0.0, 1.6, 3.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y);
        let route = sample_route_fixture();
        let mut playback = ReviewRoutePlayback::new(route).unwrap();
        let mut controller = ReviewFreeCameraController::from_transform(&camera_pose);

        controller.enable(&mut playback, &camera_pose);
        assert!(controller.is_enabled());
        assert!(!playback.is_playing());
        playback.elapsed_seconds = 4.0;
        controller.disable();
        playback.restart();
        assert_eq!(playback.elapsed_seconds(), 0.0);
        assert!(playback.is_playing());
    }

    #[test]
    fn free_camera_enable_rebases_to_the_current_scripted_view() {
        let transform =
            Transform::from_xyz(2.0, 1.8, -3.0).looking_at(Vec3::new(0.5, 1.2, 4.0), Vec3::Y);
        let route = sample_route_fixture();
        let mut playback = ReviewRoutePlayback::new(route).unwrap();
        let mut controller = ReviewFreeCameraController::from_transform(
            &Transform::from_xyz(0.0, 1.6, 3.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
        );

        controller.enable(&mut playback, &transform);

        let forward = transform.forward().as_vec3();
        assert!((controller.yaw_radians - (-forward.x).atan2(-forward.z)).abs() < 1.0e-5);
        assert!((controller.pitch_radians - forward.y.asin()).abs() < 1.0e-5);
        assert!(controller.is_enabled());
        assert!(!playback.is_playing());
    }

    #[test]
    fn free_camera_controller_starts_disabled_and_inherits_current_view() {
        let transform =
            Transform::from_xyz(0.0, 1.6, 3.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y);
        let controller = ReviewFreeCameraController::from_transform(&transform);
        assert!(!controller.is_enabled());
        assert!(controller.settings_are_valid());
        assert!(controller.pitch_radians < 0.0);
    }

    #[test]
    fn invalid_free_camera_settings_are_rejected_by_runtime_guard() {
        let mut controller = ReviewFreeCameraController::from_transform(&Transform::IDENTITY);
        assert!(controller.settings_are_valid());
        controller.speed_mps = f32::NAN;
        assert!(!controller.settings_are_valid());
    }

    #[test]
    fn route_starts_paused_and_only_opt_in_profile_autostarts() {
        let route = sample_route_fixture();
        let manual = ReviewRoutePlayback::new(route.clone()).unwrap();
        assert!(!manual.is_playing());

        let mut opt_in = ReviewComfortProfile::default();
        opt_in.profile_id = "comfort:explicit-autoplay".into();
        opt_in.start_policy = ReviewRouteStartPolicy::OptInAutoStart;
        let playback = ReviewRoutePlayback::new_with_comfort_profile(route, opt_in).unwrap();
        assert!(playback.is_playing());

        let mut teleport = sample_route_fixture();
        teleport.rig = ReviewRig::AccessibleTeleport;
        let mut opt_in = ReviewComfortProfile::default();
        opt_in.start_policy = ReviewRouteStartPolicy::OptInAutoStart;
        let playback = ReviewRoutePlayback::new_with_comfort_profile(teleport, opt_in).unwrap();
        assert!(!playback.is_playing());
    }

    #[test]
    fn comfort_profile_caps_playback_speed_and_validates_finite_settings() {
        let mut profile = ReviewComfortProfile::default();
        assert_eq!(profile.validate(), Ok(()));
        assert!(profile.route_playback_rate <= 1.0);
        profile.route_playback_rate = 1.5;
        assert!(
            profile
                .validate()
                .unwrap_err()
                .contains(&ReviewComfortProfileIssue::InvalidRoutePlaybackRate)
        );
        profile.route_playback_rate = f32::NAN;
        assert!(
            profile
                .validate()
                .unwrap_err()
                .contains(&ReviewComfortProfileIssue::InvalidRoutePlaybackRate)
        );
    }

    #[test]
    fn comfort_profile_applies_slower_camera_and_disables_vertical_motion() {
        let transform = Transform::from_xyz(0.0, 1.6, 3.0).looking_at(Vec3::ZERO, Vec3::Y);
        let mut controller = ReviewFreeCameraController::from_transform(&transform);
        let mut profile = ReviewComfortProfile::default();
        profile.free_camera_speed_mps = 0.35;
        profile.fast_multiplier = 1.0;
        profile.allow_vertical_translation = false;
        controller
            .apply_comfort_profile(&profile)
            .expect("profile is valid");
        assert_eq!(controller.speed_mps, 0.35);
        assert_eq!(controller.fast_multiplier, 1.0);
        assert!(!controller.allow_vertical_translation);
    }

    #[test]
    fn review_session_rejects_invalid_comfort_profile() {
        let mut session = sample_review_session(DesignReviewMode::StaticPlan);
        session.comfort_profile.route_playback_rate = f32::INFINITY;
        assert!(validate_design_review_session(&session)
            .unwrap_err()
            .iter()
            .any(|issue| matches!(
                issue,
                DesignReviewSessionIssue::InvalidComfortProfile(profile_issues)
                    if profile_issues.contains(&ReviewComfortProfileIssue::InvalidRoutePlaybackRate)
            )));
    }

    #[test]
    fn playback_requires_validated_route_and_honors_pause_controls() {
        let route = sample_route_fixture();
        let mut playback = ReviewRoutePlayback::new(route).expect("fixture route is valid");
        assert!(!playback.is_playing());
        playback.set_playing(true);
        assert!(playback.is_playing());
        playback.set_playing(false);
        assert!(!playback.is_playing());
        playback.restart();
        assert!(playback.is_playing());
        assert_eq!(playback.elapsed_seconds(), 0.0);

        let mut invalid = sample_route_fixture();
        invalid.max_translation_speed_mps = 0.0;
        assert!(ReviewRoutePlayback::new(invalid).is_err());
    }

    #[test]
    fn teleport_requires_explicit_stop_selection_and_vr_is_adapter_only() {
        let mut route = sample_route_fixture();
        route.rig = ReviewRig::AccessibleTeleport;
        route.waypoints[1].arrival_seconds = 2.0;
        assert_eq!(validate_review_route(&route), Ok(()));
        assert!(sample_review_route(&route, 3.0).is_none());
        assert_eq!(
            review_waypoint_transform(&route.waypoints[1])
                .unwrap()
                .translation,
            Vec3::new(0.0, 1.6, 0.0)
        );

        route.rig = ReviewRig::HeadTrackedVrReference;
        assert_eq!(validate_review_route(&route), Ok(()));
        // XR runtime, collision checks, and user-input handling belong to adapters.
    }
}
