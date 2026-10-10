// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Player movement, sprinting, flashlight, and extraction systems.

use bevy::prelude::*;

use crate::components::{Flashlight, FusionCore, NoiseEmitter, Player};
use crate::resources::{BiometricsCtx, PlayerInput};

/// Walk noise level.
const WALK_NOISE: f32 = 0.15;
/// Sprint noise level.
const SPRINT_NOISE: f32 = 0.6;
/// Standing noise decay factor.
const NOISE_DECAY: f32 = 0.85;

/// Capture human player input and write to the shared `PlayerInput` resource.
///
/// Runs in Update (where `pressed()` works reliably). The physics system
/// in FixedUpdate reads PlayerInput to set velocity on the physics body.
///
/// When the optional FEP AI player is enabled, this system relinquishes
/// control entirely. That makes `PlayerInput` single-writer by mode instead of
/// relying on unspecified ordering between the human and AI controller systems.
pub fn player_movement_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<PlayerInput>,
    mut noise_query: Query<&mut NoiseEmitter, With<Player>>,
    #[cfg(feature = "fep-ai")] ai_player: Option<Res<super::ai_player::AiPlayer>>,
) {
    #[cfg(feature = "fep-ai")]
    if ai_player.as_ref().is_some_and(|ai| ai.enabled) {
        return;
    }

    let mut direction = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    let sprinting = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    input.direction = direction;
    input.sprinting = sprinting;

    if let Ok(mut noise) = noise_query.single_mut() {
        update_player_noise(&mut noise, direction, sprinting);
    }
}

/// Apply the shared locomotion-noise contract for both human and AI control.
///
/// Keeping this in one place prevents AI movement from accidentally becoming
/// silent (and therefore bypassing the Leviathan mechanic) while also keeping
/// the human and AI controllers free to own `PlayerInput` exclusively by mode.
pub(crate) fn update_player_noise(noise: &mut NoiseEmitter, direction: Vec2, sprinting: bool) {
    noise.level = movement_noise_level(noise.level, direction, sprinting);
}

#[inline]
fn movement_noise_level(current: f32, direction: Vec2, sprinting: bool) -> f32 {
    if direction.length_squared() > f32::EPSILON {
        if sprinting { SPRINT_NOISE } else { WALK_NOISE }
    } else {
        current * NOISE_DECAY
    }
}

/// Update flashlight flicker based on player stress.
pub fn flashlight_system(
    biometrics: Res<BiometricsCtx>,
    mut query: Query<
        (
            &mut Flashlight,
            Option<&mut Sprite>,
            Option<&MeshMaterial3d<StandardMaterial>>,
        ),
        With<Player>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((mut flashlight, opt_sprite, opt_mat)) = query.single_mut() else {
        return;
    };
    flashlight.flicker =
        biometrics.encoder.velocity_surprise() * 0.4 + biometrics.model.allostatic_load * 0.3;

    // Player sprite dims/brightens with stress (visual feedback)
    let stress_dim = 1.0 - biometrics.model.allostatic_load * 0.3;
    let color = Color::srgba(0.2 * stress_dim, 0.9 * stress_dim, 1.0 * stress_dim, 1.0);

    if let Some(mut sprite) = opt_sprite {
        sprite.color = color;
    }
    if let Some(mat_handle) = opt_mat
        && let Some(mut mat) = materials.get_mut(&mat_handle.0)
    {
        mat.base_color = color;
    }
}

/// Fusion core extraction: hold E near the core. Mouse steadiness matters.
/// Collected fragments reduce extraction time (up to 50% faster with all 48).
pub fn extraction_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    mut core_query: Query<
        (
            &Transform,
            &mut FusionCore,
            Option<&mut Sprite>,
            Option<&MeshMaterial3d<StandardMaterial>>,
        ),
        Without<Player>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    biometrics: Res<BiometricsCtx>,
    mut noise_query: Query<&mut NoiseEmitter, With<Player>>,
    collected: Res<super::scavenge::CollectedPrimitives>,
    time: Res<Time>,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    for (core_tf, mut core, opt_sprite, opt_mat) in &mut core_query {
        let dist = player_tf
            .translation
            .truncate()
            .distance(core_tf.translation.truncate());

        if dist < 50.0 && keyboard.pressed(KeyCode::KeyE) {
            core.being_extracted = true;

            // Extraction speed penalized by mouse jitter (the biometric mechanic!)
            // Calm hands = fast extraction. Shaky hands = slow and noisy.
            let surprise = biometrics.encoder.velocity_surprise() as f64;
            let steadiness = (1.0f64 - surprise).max(0.1f64);
            let stress_penalty = 1.0f64 - biometrics.model.allostatic_load as f64 * 0.5f64;
            // Fragments boost: up to 50% faster with all 48 collected
            let fragment_bonus = 1.0f64 + (collected.total() as f64 / 48.0f64) * 0.5f64;
            let rate = 0.12f64 * steadiness * stress_penalty * fragment_bonus;

            core.extraction_progress += (rate * time.delta_secs() as f64) as f32;

            // Extraction generates noise — more with jittery hands
            if let Ok(mut noise) = noise_query.single_mut() {
                noise.level = (0.2 + surprise * 0.8) as f32;
            }

            // Core pulses faster as extraction progresses
            let pulse = (time.elapsed_secs() * (4.0 + core.extraction_progress * 12.0)).sin();
            let brightness = 0.7 + pulse * 0.3;
            let color = Color::srgb(brightness, brightness * 0.9, 0.1);
            if let Some(mut sprite) = opt_sprite {
                sprite.color = color;
            }
            if let Some(mat_handle) = opt_mat
                && let Some(mut mat) = materials.get_mut(&mat_handle.0)
            {
                mat.base_color = color;
            }
        } else {
            core.being_extracted = false;
            // Core gentle pulse when not being extracted
            if dist < 120.0 {
                let pulse = (time.elapsed_secs() * 1.5).sin();
                let color = Color::srgb(0.9 + pulse * 0.1, 0.8 + pulse * 0.1, 0.1);
                if let Some(mut sprite) = opt_sprite {
                    sprite.color = color;
                }
                if let Some(mat_handle) = opt_mat
                    && let Some(mut mat) = materials.get_mut(&mat_handle.0)
                {
                    mat.base_color = color;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locomotion_noise_contract_is_shared_and_monotonic() {
        assert!((movement_noise_level(0.0, Vec2::X, false) - WALK_NOISE).abs() < f32::EPSILON);
        assert!((movement_noise_level(0.0, Vec2::X, true) - SPRINT_NOISE).abs() < f32::EPSILON);
        assert!(SPRINT_NOISE > WALK_NOISE);

        let decayed = movement_noise_level(0.8, Vec2::ZERO, false);
        assert!((decayed - 0.8 * NOISE_DECAY).abs() < f32::EPSILON);
        assert!(decayed < 0.8);
    }
}
