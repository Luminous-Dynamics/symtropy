//! Opt-in skeletal animation playback for human avatars loaded from rigged glTF/GLB assets.
//!
//! This module handles animation state and blending, not mesh creation, cognition,
//! locomotion, or root-motion extraction. Attach HumanAnimationController to the entity
//! that owns Bevy's AnimationPlayer. A scene loader can build an AnimationGraph from
//! named clips in a licensed character asset and assign node indices to HumanAnimationClips.
//!
//! The visual asset is supplied by the application; no model or motion-capture asset is
//! bundled or silently downloaded by this crate.

use std::time::Duration;

use bevy::animation::{
    AnimationGraph, AnimationGraphHandle, AnimationNodeIndex, AnimationPlayer,
    AnimationTransitions,
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

impl FromWorld for HumanLocomotionConfig {
    fn from_world(_world: &mut World) -> Self {
        Self::default()
    }
}

fn derive_human_motion_from_intent(
    config: Res<HumanLocomotionConfig>,
    mut query: Query<(&HumanLocomotionIntent, &mut HumanAnimationController)>,
) {
    for (intent, mut controller) in &mut query {
        let speed = intent.desired_planar_velocity_mps.length();
        controller.motion = select_locomotion_motion(speed, controller.motion, *config);
    }
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
        || thresholds.iter().any(|value| !value.is_finite() || *value < 0.0)
        || config.stop_exit_speed_mps >= config.walk_enter_speed_mps
        || config.walk_enter_speed_mps >= config.run_enter_speed_mps
        || config.run_exit_speed_mps < config.stop_exit_speed_mps
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

    #[test]
    fn locomotion_transitions_use_hysteresis() {
        let config = HumanLocomotionConfig::default();
        assert_eq!(select_locomotion_motion(0.0, HumanMotion::Walk, config), HumanMotion::Idle);
        assert_eq!(select_locomotion_motion(0.15, HumanMotion::Walk, config), HumanMotion::Walk);
        assert_eq!(select_locomotion_motion(0.19, HumanMotion::Idle, config), HumanMotion::Walk);
        assert_eq!(select_locomotion_motion(2.1, HumanMotion::Walk, config), HumanMotion::Run);
        assert_eq!(select_locomotion_motion(1.8, HumanMotion::Run, config), HumanMotion::Run);
        assert_eq!(select_locomotion_motion(1.6, HumanMotion::Run, config), HumanMotion::Walk);
    }

    #[test]
    fn invalid_velocity_or_thresholds_fail_closed_to_idle() {
        let config = HumanLocomotionConfig::default();
        assert_eq!(select_locomotion_motion(f32::NAN, HumanMotion::Run, config), HumanMotion::Idle);
        assert_eq!(select_locomotion_motion(f32::INFINITY, HumanMotion::Walk, config), HumanMotion::Idle);
        let invalid = HumanLocomotionConfig { run_exit_speed_mps: 2.5, ..config };
        assert_eq!(select_locomotion_motion(3.0, HumanMotion::Run, invalid), HumanMotion::Idle);
    }

    #[test]
    fn stationary_talk_state_is_preserved_until_movement_is_requested() {
        let config = HumanLocomotionConfig::default();
        assert_eq!(select_locomotion_motion(0.0, HumanMotion::Talk, config), HumanMotion::Talk);
        assert_eq!(select_locomotion_motion(0.8, HumanMotion::Talk, config), HumanMotion::Walk);
    }
}
