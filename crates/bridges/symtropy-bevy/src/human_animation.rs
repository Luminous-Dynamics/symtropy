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
/// transition_seconds is sanitized to a finite value in the range 0–2 seconds.
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
    graph: Option<Handle<AnimationGraph>>,
    motion: Option<HumanMotion>,
}

/// Installs graph handles, transition management, and state-driven clip playback.
///
/// Enable with the human-animation feature. It is opt-in so existing robotics consumers
/// do not automatically pull in glTF/PBR/animation rendering support.
pub struct HumanAnimationPlugin;

impl Plugin for HumanAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (initialize_human_animation_players, drive_human_animation_players).chain(),
        );
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
        &mut AnimationGraphHandle,
        &mut AnimationTransitions,
        &mut HumanAnimationRuntime,
    )>,
) {
    for (controller, mut player, mut graph_handle, mut transitions, mut runtime) in &mut query {
        if runtime.graph.as_ref() != Some(&controller.graph) {
            graph_handle.0 = controller.graph.clone();
            runtime.graph = Some(controller.graph.clone());
            runtime.motion = None;
        }

        if runtime.motion == Some(controller.motion) {
            continue;
        }

        let seconds = if controller.transition_seconds.is_finite() {
            controller.transition_seconds.clamp(0.0, 2.0)
        } else {
            0.0
        };

        transitions
            .play(
                &mut player,
                controller.clips.index_for(controller.motion),
                Duration::from_secs_f32(seconds),
            )
            .repeat();

        runtime.motion = Some(controller.motion);
    }
}
