// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Opt-in skeletal animation playback for human avatars loaded from rigged glTF/GLB assets.
//!
//! This module handles animation state and blending, not mesh creation, cognition,
//! locomotion, or root-motion extraction. Attach HumanAnimationController to the entity
//! that owns Bevy's AnimationPlayer. A scene loader can build an AnimationGraph from
//! named clips in a licensed character asset and assign node indices to HumanAnimationClips.
//!
//! The visual asset is supplied by the application; no model or motion-capture asset is
//! bundled or silently downloaded by this crate.

use std::{error::Error, fmt, time::Duration};

use crate::human_motion_intent::{IntentValidationError, MotionIntentKind, SemanticMotionIntent};
use bevy::animation::{
    AnimationGraph, AnimationGraphHandle, AnimationNodeIndex, AnimationPlayer, AnimationTransitions,
};
use bevy::prelude::*;

/// Semantic motion state emitted by a locomotion, interaction, or cognition system.
///
/// This is intentionally independent from Symtropy's Phi/safety tier. A character's
/// appearance must not be treated as a direct display of a consciousness score.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HumanMotion {
    #[default]
    Idle,
    Walk,
    Run,
    /// Optional speaking/gesturing clip. Falls back to idle when not provided.
    Talk,
}

/// Animation-graph nodes mapped to semantic human motion states.
///
/// Build these indices from actual clips in the avatar's glTF/GLB asset. The runtime
/// never assumes clip indices or names are identical across character assets.
#[derive(Debug, Clone)]
pub struct HumanAnimationClips {
    pub idle: AnimationNodeIndex,
    pub walk: AnimationNodeIndex,
    pub run: AnimationNodeIndex,
    pub talk: Option<AnimationNodeIndex>,
}

impl HumanAnimationClips {
    pub fn index_for(&self, motion: HumanMotion) -> AnimationNodeIndex {
        match motion {
            HumanMotion::Idle => self.idle,
            HumanMotion::Walk => self.walk,
            HumanMotion::Run => self.run,
            HumanMotion::Talk => self.talk.unwrap_or(self.idle),
        }
    }
}

/// Attach this component to an entity carrying Bevy's AnimationPlayer.
///
/// Another system changes motion in response to movement intent, interaction state,
/// or a Symthaea decision. The plugin cross-fades to the corresponding animation node.
/// The graph is bound when this component is initialized; use a new player entity to
/// switch to a different graph. transition_seconds is clamped to 0–2 seconds.
#[derive(Component, Debug, Clone)]
pub struct HumanAnimationController {
    pub graph: Handle<AnimationGraph>,
    pub clips: HumanAnimationClips,
    pub motion: HumanMotion,
    pub transition_seconds: f32,
}

impl HumanAnimationController {
    pub fn new(
        graph: Handle<AnimationGraph>,
        clips: HumanAnimationClips,
        motion: HumanMotion,
    ) -> Self {
        Self {
            graph,
            clips,
            motion,
            transition_seconds: 0.2,
        }
    }
}

#[derive(Component, Default)]
struct HumanAnimationRuntime {
    motion: Option<HumanMotion>,
}

/// Physics/cognition adapter input: desired planar velocity in world-space X/Z, in m/s.
///
/// This is an *intent*, not a transform or applied force. A locomotion/physics system remains
/// responsible for moving the body; the animation system only chooses a matching clip.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HumanLocomotionIntent {
    /// Desired world-space planar velocity (x, z), in metres per second.
    pub desired_planar_velocity_mps: Vec2,
    /// Optional physics feedback. When present, animation follows realized rather than
    /// requested movement, preventing walk/run cycles while a body is blocked.
    pub observed_planar_velocity_mps: Option<Vec2>,
    /// Explicit semantic state such as Idle or Talk. This takes precedence over velocity.
    /// Set to None to let locomotion hysteresis select Idle/Walk/Run.
    pub motion_override: Option<HumanMotion>,
    /// Simulation-clock expiry copied from the semantic intent. The runtime enforces it.
    pub valid_until_sec: Option<f64>,
}

/// Error from translating a high-level semantic intent to this controller's
/// currently supported low-level locomotion input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticIntentAdapterError {
    InvalidIntent(IntentValidationError),
    UnsupportedIntentKind(MotionIntentKind),
}

impl fmt::Display for SemanticIntentAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIntent(error) => write!(f, "invalid semantic motion intent: {error}"),
            Self::UnsupportedIntentKind(kind) => {
                write!(
                    f,
                    "semantic intent {kind:?} is not implemented by the idle/walk/run adapter"
                )
            }
        }
    }
}

impl Error for SemanticIntentAdapterError {}

/// Adapt a versioned cognition intent to the controller's minimal planar-velocity input.
///
/// Only idle and locomote are implemented in this first adapter. Richer actions such as
/// reach/grasp/sit/yield fail explicitly instead of being silently mapped to an unrelated clip.
pub fn adapt_semantic_intent_to_locomotion_at(
    intent: &SemanticMotionIntent,
    now_sec: f64,
) -> Result<HumanLocomotionIntent, SemanticIntentAdapterError> {
    intent
        .validate_at(now_sec)
        .map_err(SemanticIntentAdapterError::InvalidIntent)?;

    let (desired_planar_velocity_mps, motion_override) = match intent.intent_kind {
        MotionIntentKind::Idle => (bevy::math::Vec2::ZERO, Some(HumanMotion::Idle)),
        MotionIntentKind::Locomote => {
            let [x, z] = intent.desired_planar_velocity_mps.ok_or(
                SemanticIntentAdapterError::InvalidIntent(
                    IntentValidationError::MissingLocomotionVelocity,
                ),
            )?;
            (bevy::math::Vec2::new(x, z), None)
        }
        unsupported => {
            return Err(SemanticIntentAdapterError::UnsupportedIntentKind(
                unsupported,
            ));
        }
    };

    Ok(HumanLocomotionIntent {
        desired_planar_velocity_mps,
        observed_planar_velocity_mps: None,
        motion_override,
        valid_until_sec: intent.valid_until_sec,
    })
}

/// Hysteresis thresholds for choosing idle, walk, and run clips.
#[derive(Resource, Debug, Clone, Copy)]
pub struct HumanLocomotionConfig {
    /// A walking/running character returns to idle at or below this speed.
    pub stop_exit_speed_mps: f32,
    /// An idle character enters walk at or above this speed.
    pub walk_enter_speed_mps: f32,
    /// A walking character enters run at or above this speed.
    pub run_enter_speed_mps: f32,
    /// A running character returns to walk below this speed.
    pub run_exit_speed_mps: f32,
}

impl Default for HumanLocomotionConfig {
    fn default() -> Self {
        Self {
            stop_exit_speed_mps: 0.12,
            walk_enter_speed_mps: 0.18,
            run_enter_speed_mps: 2.0,
            run_exit_speed_mps: 1.65,
        }
    }
}

/// Installs intent-to-clip selection, graph handles, and cross-faded playback.
///
/// Enable with the human-animation feature. It is opt-in so existing robotics consumers
/// do not automatically pull in glTF/PBR/animation rendering support.
pub struct HumanAnimationPlugin;

impl Plugin for HumanAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HumanLocomotionConfig>().add_systems(
            Update,
            (
                derive_human_motion_from_intent,
                initialize_human_animation_players,
                drive_human_animation_players,
            )
                .chain(),
        );
    }
}

fn derive_human_motion_from_intent(
    time: Res<Time>,
    config: Res<HumanLocomotionConfig>,
    mut query: Query<(&mut HumanLocomotionIntent, &mut HumanAnimationController)>,
) {
    let now_sec = time.elapsed_secs_f64();
    for (mut intent, mut controller) in &mut query {
        if has_invalid_locomotion_velocity(&intent) {
            intent.desired_planar_velocity_mps = Vec2::ZERO;
            intent.observed_planar_velocity_mps = None;
            intent.motion_override = Some(HumanMotion::Idle);
            intent.valid_until_sec = None;
            controller.motion = HumanMotion::Idle;
            continue;
        }

        if expire_locomotion_intent(&mut intent, now_sec, config.stop_exit_speed_mps) {
            if intent.motion_override == Some(HumanMotion::Idle) {
                controller.motion = HumanMotion::Idle;
                continue;
            }
        }
        controller.motion = motion_for_locomotion_intent(&intent, controller.motion, *config);
    }
}

fn has_invalid_locomotion_velocity(intent: &HumanLocomotionIntent) -> bool {
    !intent.desired_planar_velocity_mps.is_finite()
        || !intent.desired_planar_velocity_mps.length().is_finite()
        || intent
            .observed_planar_velocity_mps
            .is_some_and(|velocity| !velocity.is_finite() || !velocity.length().is_finite())
}

/// Consume an expired intent exactly once, zeroing requested movement. If measured velocity
/// says the body is still coasting, defer clip choice to that measurement instead of forcing Idle.
fn expire_locomotion_intent(
    intent: &mut HumanLocomotionIntent,
    now_sec: f64,
    stop_exit_speed_mps: f32,
) -> bool {
    let expired = intent
        .valid_until_sec
        .is_some_and(|end| !end.is_finite() || !now_sec.is_finite() || now_sec >= end);
    if !expired {
        return false;
    }

    intent.desired_planar_velocity_mps = Vec2::ZERO;
    intent.valid_until_sec = None;
    let observed_speed = intent
        .observed_planar_velocity_mps
        .unwrap_or(Vec2::ZERO)
        .length();
    if !observed_speed.is_finite() || observed_speed <= stop_exit_speed_mps {
        intent.motion_override = Some(HumanMotion::Idle);
    } else {
        intent.motion_override = None;
    }
    true
}

/// Prefer an explicit semantic-state override, then realized velocity feedback, then requested
/// velocity. The intent-only fallback keeps the adapter usable before physics feedback is wired.
fn motion_for_locomotion_intent(
    intent: &HumanLocomotionIntent,
    current: HumanMotion,
    config: HumanLocomotionConfig,
) -> HumanMotion {
    if let Some(override_motion) = intent.motion_override {
        if let Some(observed_velocity) = intent.observed_planar_velocity_mps {
            let observed_speed = observed_velocity.length();
            if observed_speed.is_finite() && observed_speed > config.stop_exit_speed_mps {
                return select_locomotion_motion(observed_speed, current, config);
            }
        }
        return override_motion;
    }

    let velocity = intent
        .observed_planar_velocity_mps
        .unwrap_or(intent.desired_planar_velocity_mps);
    select_locomotion_motion(velocity.length(), current, config)
}

/// Select a clip state using explicit thresholds and hysteresis to prevent state chatter.
fn select_locomotion_motion(
    speed_mps: f32,
    current: HumanMotion,
    config: HumanLocomotionConfig,
) -> HumanMotion {
    let thresholds = [
        config.stop_exit_speed_mps,
        config.walk_enter_speed_mps,
        config.run_enter_speed_mps,
        config.run_exit_speed_mps,
    ];
    if !speed_mps.is_finite()
        || speed_mps < 0.0
        || thresholds
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        || config.stop_exit_speed_mps >= config.walk_enter_speed_mps
        || config.walk_enter_speed_mps >= config.run_enter_speed_mps
        || config.run_exit_speed_mps < config.walk_enter_speed_mps
        || config.run_exit_speed_mps >= config.run_enter_speed_mps
    {
        return HumanMotion::Idle;
    }

    match current {
        HumanMotion::Idle => {
            if speed_mps >= config.run_enter_speed_mps {
                HumanMotion::Run
            } else if speed_mps >= config.walk_enter_speed_mps {
                HumanMotion::Walk
            } else {
                HumanMotion::Idle
            }
        }
        HumanMotion::Walk => {
            if speed_mps <= config.stop_exit_speed_mps {
                HumanMotion::Idle
            } else if speed_mps >= config.run_enter_speed_mps {
                HumanMotion::Run
            } else {
                HumanMotion::Walk
            }
        }
        HumanMotion::Run => {
            if speed_mps <= config.stop_exit_speed_mps {
                HumanMotion::Idle
            } else if speed_mps < config.run_exit_speed_mps {
                HumanMotion::Walk
            } else {
                HumanMotion::Run
            }
        }
        HumanMotion::Talk => {
            if speed_mps >= config.run_enter_speed_mps {
                HumanMotion::Run
            } else if speed_mps >= config.walk_enter_speed_mps {
                HumanMotion::Walk
            } else {
                HumanMotion::Talk
            }
        }
    }
}

fn initialize_human_animation_players(
    mut commands: Commands,
    query: Query<(Entity, &HumanAnimationController), Without<HumanAnimationRuntime>>,
) {
    for (entity, controller) in &query {
        commands.entity(entity).insert((
            AnimationGraphHandle(controller.graph.clone()),
            AnimationTransitions::new(),
            HumanAnimationRuntime::default(),
        ));
    }
}

fn drive_human_animation_players(
    mut query: Query<(
        &HumanAnimationController,
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut HumanAnimationRuntime,
    )>,
) {
    for (controller, mut player, mut transitions, mut runtime) in &mut query {
        if runtime.motion == Some(controller.motion) {
            continue;
        }

        transitions
            .play(
                &mut player,
                controller.clips.index_for(controller.motion),
                bounded_transition_duration(controller.transition_seconds),
            )
            .repeat();

        runtime.motion = Some(controller.motion);
    }
}

fn bounded_transition_duration(seconds: f32) -> Duration {
    let seconds = if seconds.is_finite() {
        seconds.clamp(0.0, 2.0)
    } else {
        0.0
    };
    Duration::from_secs_f32(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_duration_clamps_negative_and_excessive_values() {
        assert_eq!(bounded_transition_duration(-1.0), Duration::ZERO);
        assert_eq!(
            bounded_transition_duration(10.0),
            Duration::from_secs_f32(2.0)
        );
    }

    #[test]
    fn transition_duration_rejects_non_finite_values() {
        assert_eq!(bounded_transition_duration(f32::NAN), Duration::ZERO);
        assert_eq!(bounded_transition_duration(f32::INFINITY), Duration::ZERO);
    }

    #[test]
    fn default_motion_is_idle() {
        assert_eq!(HumanMotion::default(), HumanMotion::Idle);
    }

    fn semantic_intent_for_test(
        intent_kind: MotionIntentKind,
        velocity: Option<[f32; 2]>,
        target: Option<crate::human_motion_intent::MotionIntentTarget>,
    ) -> SemanticMotionIntent {
        use crate::human_motion_intent::{
            ConfidenceSemantics, HUMAN_MOTION_INTENT_SCHEMA_VERSION, IntentProvenance,
            IntentSource, InterruptionPolicy,
        };

        SemanticMotionIntent {
            schema_version: HUMAN_MOTION_INTENT_SCHEMA_VERSION,
            intent_id: "test-intent-1".to_owned(),
            actor_ref: "test-actor".to_owned(),
            intent_kind,
            coordinate_frame: crate::human_motion_intent::MotionIntentCoordinateFrame {
                length_unit: "metre".to_owned(),
                up_axis: "+Y".to_owned(),
                forward_axis: "-Z".to_owned(),
                position_space: "world".to_owned(),
            },
            issued_at_sec: 0.0,
            valid_until_sec: Some(2.0),
            urgency: 0.25,
            confidence: 0.8,
            confidence_semantics: ConfidenceSemantics::UncalibratedScore,
            uncertainty: 0.2,
            desired_planar_velocity_mps: velocity,
            target,
            interruption_policy: InterruptionPolicy::Preemptible,
            calibration_profile_id: None,
            provenance: IntentProvenance {
                source: IntentSource::RuleBased,
                policy_id: "adapter-test".to_owned(),
                policy_version: "1".to_owned(),
                observation_digest_sha256: None,
                model_sha256: None,
            },
        }
    }

    #[test]
    fn semantic_locomotion_intent_maps_velocity_without_applying_motion() {
        let intent = semantic_intent_for_test(MotionIntentKind::Locomote, Some([0.8, -0.2]), None);
        let adapted = adapt_semantic_intent_to_locomotion_at(&intent, 0.5).unwrap();
        assert!(adapted.motion_override.is_none());
        assert!(adapted.observed_planar_velocity_mps.is_none());
        assert!((adapted.desired_planar_velocity_mps.x - 0.8).abs() < 1e-6);
        assert!((adapted.desired_planar_velocity_mps.y + 0.2).abs() < 1e-6);
    }

    #[test]
    fn semantic_idle_maps_to_zero_velocity() {
        let intent = semantic_intent_for_test(MotionIntentKind::Idle, None, None);
        let adapted = adapt_semantic_intent_to_locomotion_at(&intent, 0.5).unwrap();
        assert_eq!(adapted.desired_planar_velocity_mps, Vec2::ZERO);
        assert_eq!(adapted.motion_override, Some(HumanMotion::Idle));
        assert_eq!(adapted.valid_until_sec, Some(2.0));
        assert_eq!(
            motion_for_locomotion_intent(
                &adapted,
                HumanMotion::Talk,
                HumanLocomotionConfig::default()
            ),
            HumanMotion::Idle
        );
    }

    #[test]
    fn non_finite_or_overflowing_velocity_is_rejected() {
        let mut intent = HumanLocomotionIntent::default();
        intent.desired_planar_velocity_mps = Vec2::new(f32::MAX, f32::MAX);
        assert!(has_invalid_locomotion_velocity(&intent));
        intent.desired_planar_velocity_mps = Vec2::ZERO;
        intent.observed_planar_velocity_mps = Some(Vec2::new(f32::NAN, 0.0));
        assert!(has_invalid_locomotion_velocity(&intent));
        intent.observed_planar_velocity_mps = Some(Vec2::new(f32::MAX, f32::MAX));
        assert!(has_invalid_locomotion_velocity(&intent));
    }

    #[test]
    fn expired_intent_zeros_requested_velocity_and_returns_to_idle_without_feedback() {
        let mut intent = adapt_semantic_intent_to_locomotion_at(
            &semantic_intent_for_test(MotionIntentKind::Locomote, Some([1.0, 0.0]), None),
            0.1,
        )
        .unwrap();
        intent.valid_until_sec = Some(0.5);
        assert!(expire_locomotion_intent(
            &mut intent,
            0.5,
            HumanLocomotionConfig::default().stop_exit_speed_mps
        ));
        assert_eq!(intent.desired_planar_velocity_mps, Vec2::ZERO);
        assert_eq!(intent.valid_until_sec, None);
        assert_eq!(intent.motion_override, Some(HumanMotion::Idle));
    }

    #[test]
    fn expired_intent_allows_measured_coasting_motion_to_finish() {
        let mut intent = adapt_semantic_intent_to_locomotion_at(
            &semantic_intent_for_test(MotionIntentKind::Locomote, Some([1.0, 0.0]), None),
            0.1,
        )
        .unwrap();
        intent.valid_until_sec = Some(0.5);
        intent.observed_planar_velocity_mps = Some(Vec2::new(0.6, 0.0));
        assert!(expire_locomotion_intent(
            &mut intent,
            0.5,
            HumanLocomotionConfig::default().stop_exit_speed_mps
        ));
        assert_eq!(intent.desired_planar_velocity_mps, Vec2::ZERO);
        assert_eq!(intent.motion_override, None);
        assert_eq!(
            intent.observed_planar_velocity_mps,
            Some(Vec2::new(0.6, 0.0))
        );
    }

    #[test]
    fn observed_motion_overrides_stationary_semantic_state_while_body_is_moving() {
        let mut intent = adapt_semantic_intent_to_locomotion_at(
            &semantic_intent_for_test(MotionIntentKind::Idle, None, None),
            0.5,
        )
        .unwrap();
        intent.observed_planar_velocity_mps = Some(Vec2::new(0.8, 0.0));
        assert_eq!(
            motion_for_locomotion_intent(
                &intent,
                HumanMotion::Idle,
                HumanLocomotionConfig::default()
            ),
            HumanMotion::Walk
        );
    }

    #[test]
    fn observed_velocity_prevents_walking_animation_when_body_is_blocked() {
        let mut intent = adapt_semantic_intent_to_locomotion_at(
            &semantic_intent_for_test(MotionIntentKind::Locomote, Some([2.0, 0.0]), None),
            0.5,
        )
        .unwrap();
        intent.observed_planar_velocity_mps = Some(Vec2::ZERO);
        assert_eq!(
            motion_for_locomotion_intent(
                &intent,
                HumanMotion::Walk,
                HumanLocomotionConfig::default()
            ),
            HumanMotion::Idle
        );
    }

    #[test]
    fn observed_velocity_can_trigger_motion_when_requested_speed_is_zero() {
        let mut intent = adapt_semantic_intent_to_locomotion_at(
            &semantic_intent_for_test(MotionIntentKind::Locomote, Some([0.0, 0.0]), None),
            0.5,
        )
        .unwrap();
        intent.observed_planar_velocity_mps = Some(Vec2::new(0.4, 0.0));
        assert_eq!(
            motion_for_locomotion_intent(
                &intent,
                HumanMotion::Idle,
                HumanLocomotionConfig::default()
            ),
            HumanMotion::Walk
        );
    }

    #[test]
    fn expired_semantic_intents_are_rejected_before_adaptation() {
        let mut intent =
            semantic_intent_for_test(MotionIntentKind::Locomote, Some([1.0, 0.0]), None);
        intent.valid_until_sec = Some(0.5);
        assert_eq!(
            adapt_semantic_intent_to_locomotion_at(&intent, 0.5).unwrap_err(),
            SemanticIntentAdapterError::InvalidIntent(IntentValidationError::Expired)
        );
    }

    #[test]
    fn unsupported_semantic_actions_fail_closed() {
        use crate::human_motion_intent::MotionIntentTarget;
        let target = Some(MotionIntentTarget {
            entity_ref: Some("object:cup".to_owned()),
            position_world_m: None,
            orientation_xyzw: None,
        });
        let intent = semantic_intent_for_test(MotionIntentKind::Reach, None, target);
        assert_eq!(
            adapt_semantic_intent_to_locomotion_at(&intent, 0.5).unwrap_err(),
            SemanticIntentAdapterError::UnsupportedIntentKind(MotionIntentKind::Reach)
        );
    }

    #[test]
    fn locomotion_transitions_use_hysteresis() {
        let config = HumanLocomotionConfig::default();
        assert_eq!(
            select_locomotion_motion(0.0, HumanMotion::Walk, config),
            HumanMotion::Idle
        );
        assert_eq!(
            select_locomotion_motion(0.15, HumanMotion::Walk, config),
            HumanMotion::Walk
        );
        assert_eq!(
            select_locomotion_motion(0.19, HumanMotion::Idle, config),
            HumanMotion::Walk
        );
        assert_eq!(
            select_locomotion_motion(2.1, HumanMotion::Walk, config),
            HumanMotion::Run
        );
        assert_eq!(
            select_locomotion_motion(1.8, HumanMotion::Run, config),
            HumanMotion::Run
        );
        assert_eq!(
            select_locomotion_motion(1.6, HumanMotion::Run, config),
            HumanMotion::Walk
        );
    }

    #[test]
    fn invalid_velocity_or_thresholds_fail_closed_to_idle() {
        let config = HumanLocomotionConfig::default();
        assert_eq!(
            select_locomotion_motion(f32::NAN, HumanMotion::Run, config),
            HumanMotion::Idle
        );
        assert_eq!(
            select_locomotion_motion(f32::INFINITY, HumanMotion::Walk, config),
            HumanMotion::Idle
        );
        let invalid = HumanLocomotionConfig {
            run_exit_speed_mps: 2.5,
            ..config
        };
        assert_eq!(
            select_locomotion_motion(3.0, HumanMotion::Run, invalid),
            HumanMotion::Idle
        );
    }

    #[test]
    fn stationary_talk_state_is_preserved_until_movement_is_requested() {
        let config = HumanLocomotionConfig::default();
        assert_eq!(
            select_locomotion_motion(0.0, HumanMotion::Talk, config),
            HumanMotion::Talk
        );
        assert_eq!(
            select_locomotion_motion(0.8, HumanMotion::Talk, config),
            HumanMotion::Walk
        );
    }
}
