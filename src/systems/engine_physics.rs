// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authoritative launcher bridge into `symtropy-physics`.
//!
//! The launcher historically spawned real physics bodies but did not advance
//! `PhysicsWorld` in its FixedUpdate path. This module restores an intentionally
//! narrow 2D dogfood loop while keeping the current Old Waterworks 3D mode on
//! its explicitly documented 2D horizontal proxy until a true `PhysicsWorld<3>`
//! migration is ready.

use crate::components::Player;
use crate::resources::{GamePhase, PhysicsWorldRes, PlayerInput, SiteLayout, TileGrid};
use crate::systems::procgen::tile_code_is_walkable;
use bevy::prelude::*;
use symtropy_render_bridge::PhysicsBody;

/// Match the current 3D controller's authored movement scale so switching
/// presentation modes does not silently imply a radically different player.
const PLAYER_WALK_SPEED: f64 = 85.0;
const PLAYER_SPRINT_MULTIPLIER: f64 = 1.5;
const INTENT_WAKE_EPSILON_SQ: f64 = 1.0e-12;

pub fn update_physics_consciousness(
    mut physics: ResMut<PhysicsWorldRes>,
    query: Query<(&PhysicsBody, &crate::components::HarmonyComponent)>,
) {
    for (body_comp, harmony) in query.iter() {
        // Sync harmony activations
        if let Some(entity) = physics.consciousness.entities.get_mut(&body_comp.handle) {
            entity.harmony_activations = [
                harmony.activations[0] as f64,
                harmony.activations[1] as f64,
                harmony.activations[2] as f64,
                harmony.activations[3] as f64,
                harmony.activations[4] as f64,
                harmony.activations[5] as f64,
                harmony.activations[6] as f64,
                harmony.activations[7] as f64,
                0.0, // Index 8
            ];
        }
    }
}

/// FixedUpdate authority bridge.
///
/// The public name is retained for plugin compatibility, but the system now
/// performs the complete authoritative 2D sequence when `GamePhase::Playing`:
///
/// 1. repair the derived `TileGrid` to the generator's canonical tile semantics,
/// 2. consume buffered player intent into the Symtropy body,
/// 3. wake dynamic bodies that received velocity intent (including FEP NPCs),
/// 4. advance `PhysicsWorld<2>` exactly once at Bevy's fixed timestep,
/// 5. copy authoritative body positions back to Bevy transforms.
///
/// During `Playing3D` we deliberately do **not** step the 2D proxy. The current
/// Old Waterworks controller still owns horizontal motion directly; only the
/// tile-grid repair and transform synchronization remain active there. A later
/// migration must introduce an actual `PhysicsWorld<3>` rather than quietly
/// pretending this proxy is full 3D physics.
pub fn physics_sync_transforms(
    mut physics: ResMut<PhysicsWorldRes>,
    state: Res<State<GamePhase>>,
    input: Res<PlayerInput>,
    layout: Res<SiteLayout>,
    mut tile_grid: ResMut<TileGrid>,
    player_query: Query<&PhysicsBody, With<Player>>,
    mut transform_query: Query<(&PhysicsBody, &mut Transform)>,
    fixed_time: Res<Time<bevy::time::Fixed>>,
) {
    ensure_canonical_tile_grid(&layout, &mut tile_grid);

    if matches!(state.get(), GamePhase::Playing) {
        let dt = fixed_time.delta().as_secs_f64();
        if dt.is_finite() && dt > 0.0 {
            if let Ok(player_body) = player_query.single() {
                apply_player_intent(
                    &mut physics,
                    player_body.handle,
                    &input,
                    &tile_grid,
                    dt,
                );
            }

            // NPC intent is written by the FEP movement system in Update.
            // Direct velocity assignment does not wake a sleeping RigidBody, so
            // honor any non-zero controller intent before the integrator skips
            // sleeping bodies. Restrict this to bodies that are already asleep:
            // calling wake() on every moving body would reset sleep counters each
            // fixed tick and prevent slow bodies from ever settling to sleep.
            for body in &mut physics.world.bodies {
                if body.is_dynamic()
                    && body.sleeping
                    && body.linear_velocity.norm_squared() > INTENT_WAKE_EPSILON_SQ
                {
                    body.wake();
                }
            }

            step_authoritative_world(&mut physics, dt);
        }
    }

    for (body_comp, mut transform) in &mut transform_query {
        if let Some(body) = physics.world.body(body_comp.handle) {
            let pos: nalgebra::SVector<f64, 2> = body.position();
            transform.translation.x = pos[0] as f32;
            transform.translation.y = pos[1] as f32;
        }
    }
}

/// `TileGrid` is derived state, so repair it from `SiteLayout` whenever the
/// stored booleans disagree with the authoritative tile codes.
///
/// The old generator path populated `walkable = cell != 1`, which inverted the
/// wall/floor meaning (`0` wall became walkable, `1` floor became blocked).
/// Rebuilding here makes both 2D FixedUpdate and the current 3D proxy consume a
/// canonical collision grid without mutating the authored layout.
fn ensure_canonical_tile_grid(layout: &SiteLayout, grid: &mut TileGrid) {
    if layout.width == 0 || layout.height == 0 {
        return;
    }

    let dimensions_match = grid.cols == layout.width as i32
        && grid.rows == layout.height as i32
        && grid.origin_col == layout.width as i32 / 2
        && grid.origin_row == layout.height as i32 / 2;

    let semantics_match = dimensions_match
        && (0..layout.height).all(|row| {
            (0..layout.width).all(|col| {
                let cell = layout
                    .tiles
                    .get(row)
                    .and_then(|values| values.get(col))
                    .copied()
                    .unwrap_or(0);
                grid.cells.get(&(col as i32, row as i32)).copied()
                    == Some(tile_code_is_walkable(cell))
            })
        });

    if semantics_match {
        return;
    }

    grid.cols = layout.width as i32;
    grid.rows = layout.height as i32;
    grid.origin_col = grid.cols / 2;
    grid.origin_row = grid.rows / 2;
    grid.cells.clear();

    for row in 0..layout.height {
        for col in 0..layout.width {
            let cell = layout
                .tiles
                .get(row)
                .and_then(|values| values.get(col))
                .copied()
                .unwrap_or(0);
            grid.cells.insert(
                (col as i32, row as i32),
                tile_code_is_walkable(cell),
            );
        }
    }
}

fn apply_player_intent(
    physics: &mut PhysicsWorldRes,
    handle: symtropy_physics::BodyHandle,
    input: &PlayerInput,
    tile_grid: &TileGrid,
    dt: f64,
) {
    let intent = bounded_intent(input.direction);
    let gain = player_motor_gain(physics, handle);
    let speed = PLAYER_WALK_SPEED
        * if input.sprinting {
            PLAYER_SPRINT_MULTIPLIER
        } else {
            1.0
        }
        * gain;

    let mut desired_velocity = nalgebra::SVector::from([
        intent.x as f64 * speed,
        intent.y as f64 * speed,
    ]);

    let (current_position, radius) = match physics.world.body(handle) {
        Some(body) => {
            let (_, radius) = body.collider.bounding_sphere();
            (body.position(), radius.max(0.0))
        }
        None => return,
    };

    if tile_grid.tile_size > 0.0 && !tile_grid.cells.is_empty() {
        desired_velocity = constrain_velocity_to_tile_grid(
            current_position,
            radius,
            desired_velocity,
            tile_grid,
            dt,
        );
    }

    if let Some(body) = physics.world.body_mut(handle) {
        body.linear_velocity = desired_velocity;
        if desired_velocity.norm_squared() > INTENT_WAKE_EPSILON_SQ {
            body.wake();
        }
    }
}

/// Preserve sub-unit intent magnitude (used by the AI player) while preventing
/// diagonal keyboard input from exceeding the authored maximum speed.
fn bounded_intent(direction: Vec2) -> Vec2 {
    let length = direction.length();
    if !length.is_finite() || length <= f32::EPSILON {
        Vec2::ZERO
    } else if length > 1.0 {
        direction / length
    } else {
        direction
    }
}

#[cfg(feature = "consciousness-runtime")]
fn player_motor_gain(physics: &PhysicsWorldRes, handle: symtropy_physics::BodyHandle) -> f64 {
    physics
        .consciousness
        .entities
        .get(&handle)
        .map(|entity| entity.effective_motor_gain())
        .unwrap_or(1.0)
        .clamp(0.0, 1.0)
}

#[cfg(not(feature = "consciousness-runtime"))]
fn player_motor_gain(physics: &PhysicsWorldRes, handle: symtropy_physics::BodyHandle) -> f64 {
    use crate::resources::SafetyTier;

    physics
        .consciousness
        .entities
        .get(&handle)
        .map(|entity| {
            let tier_gain = match entity.safety_tier {
                SafetyTier::Green => 1.0,
                SafetyTier::Yellow => 0.6,
                SafetyTier::Red => 0.0,
            };
            tier_gain * entity.motor_precision.clamp(0.0, 1.0)
        })
        .unwrap_or(1.0)
        .clamp(0.0, 1.0)
}

fn constrain_velocity_to_tile_grid(
    position: nalgebra::SVector<f64, 2>,
    radius: f64,
    velocity: nalgebra::SVector<f64, 2>,
    grid: &TileGrid,
    dt: f64,
) -> nalgebra::SVector<f64, 2> {
    if !dt.is_finite() || dt <= 0.0 {
        return nalgebra::SVector::zeros();
    }

    let mut result = velocity;
    let candidate_x = Vec2::new(
        (position[0] + result[0] * dt) as f32,
        position[1] as f32,
    );
    if !can_occupy_circle(grid, candidate_x, radius as f32) {
        result[0] = 0.0;
    }

    let candidate_y = Vec2::new(
        (position[0] + result[0] * dt) as f32,
        (position[1] + result[1] * dt) as f32,
    );
    if !can_occupy_circle(grid, candidate_y, radius as f32) {
        result[1] = 0.0;
    }

    result
}

/// Temporary R1 boundary adapter. R2 should replace this with static physics
/// colliders so contact provenance and response live entirely inside the solver.
fn can_occupy_circle(grid: &TileGrid, center: Vec2, radius: f32) -> bool {
    let radius = radius.max(0.0);
    let diagonal = radius * std::f32::consts::FRAC_1_SQRT_2;
    let samples = [
        center,
        center + Vec2::new(radius, 0.0),
        center + Vec2::new(-radius, 0.0),
        center + Vec2::new(0.0, radius),
        center + Vec2::new(0.0, -radius),
        center + Vec2::new(diagonal, diagonal),
        center + Vec2::new(diagonal, -diagonal),
        center + Vec2::new(-diagonal, diagonal),
        center + Vec2::new(-diagonal, -diagonal),
    ];

    samples
        .into_iter()
        .all(|sample| grid.is_walkable(sample.x, sample.y))
}

#[cfg(feature = "consciousness-runtime")]
fn step_authoritative_world(physics: &mut PhysicsWorldRes, dt: f64) {
    let positions: Vec<_> = physics
        .world
        .bodies
        .iter()
        .map(|body| (body.handle, symtropy_math::Point(body.position())))
        .collect();
    physics.consciousness.rebuild_harmony_field(&positions);

    let PhysicsWorldRes {
        world,
        consciousness,
    } = physics;
    world.step_with_callback(dt, consciousness);
}

#[cfg(not(feature = "consciousness-runtime"))]
fn step_authoritative_world(physics: &mut PhysicsWorldRes, dt: f64) {
    physics.world.step(dt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_grid() -> TileGrid {
        let mut cells = HashMap::new();
        // 3x3 world with a walkable center and east neighbor, walls elsewhere.
        for row in 0..3 {
            for col in 0..3 {
                cells.insert((col, row), false);
            }
        }
        cells.insert((1, 1), true);
        cells.insert((2, 1), true);
        TileGrid {
            cells,
            tile_size: 32.0,
            origin_col: 1,
            origin_row: 1,
            cols: 3,
            rows: 3,
        }
    }

    #[test]
    fn canonical_tile_codes_match_generator_contract() {
        assert!(!tile_code_is_walkable(0));
        assert!(tile_code_is_walkable(1));
        assert!(tile_code_is_walkable(2));
        assert!(tile_code_is_walkable(3));
    }

    #[test]
    fn derived_grid_repairs_inverted_wall_floor_semantics() {
        let layout = SiteLayout {
            site_id: "test".to_string(),
            width: 2,
            height: 2,
            tiles: vec![vec![0, 1], vec![2, 3]],
            room_centers: Vec::new(),
            player_start: Vec2::ZERO,
            core_pos: Vec2::ZERO,
        };
        let mut grid = TileGrid {
            cells: HashMap::from([
                ((0, 0), true),
                ((1, 0), false),
                ((0, 1), true),
                ((1, 1), true),
            ]),
            tile_size: 32.0,
            origin_col: 1,
            origin_row: 1,
            cols: 2,
            rows: 2,
        };

        ensure_canonical_tile_grid(&layout, &mut grid);

        assert_eq!(grid.cells.get(&(0, 0)), Some(&false));
        assert_eq!(grid.cells.get(&(1, 0)), Some(&true));
        assert_eq!(grid.cells.get(&(0, 1)), Some(&true));
        assert_eq!(grid.cells.get(&(1, 1)), Some(&true));
    }

    #[test]
    fn bounded_intent_preserves_analog_magnitude_and_caps_diagonal() {
        let analog = bounded_intent(Vec2::new(0.3, 0.4));
        assert!((analog.length() - 0.5).abs() < 1.0e-6);

        let diagonal = bounded_intent(Vec2::new(1.0, 1.0));
        assert!((diagonal.length() - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn tile_boundary_adapter_allows_open_axis_and_blocks_wall_axis() {
        let grid = test_grid();
        let position = nalgebra::SVector::from([0.0, 0.0]);
        let velocity = nalgebra::SVector::from([80.0, 80.0]);

        let constrained = constrain_velocity_to_tile_grid(position, 2.0, velocity, &grid, 0.2);

        // +x lands in the open east cell; +y lands in the north wall.
        assert!(constrained[0] > 0.0);
        assert_eq!(constrained[1], 0.0);
    }

    #[test]
    fn circle_occupancy_respects_body_radius() {
        let grid = test_grid();
        assert!(can_occupy_circle(&grid, Vec2::ZERO, 2.0));
        assert!(!can_occupy_circle(&grid, Vec2::ZERO, 20.0));
    }
}
