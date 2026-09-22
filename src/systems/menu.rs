// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Symtropy Nexus — extensible engine launcher with living background.
//!
//! Reads from ExperienceRegistry to build the menu dynamically.
//! Background is a breathing mycelial network rendered via gizmos.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::experience::ExperienceRegistry;
use crate::resources::{
    BiometricsCtx, DungeonSeed, GamePhase, LeviathanState, PhysicsWorldRes, PlayerInput,
};

/// Marker for Nexus UI entities (despawned on transition).
#[derive(Component)]
pub struct MenuUi;

/// Marker for loading screen entities.
#[derive(Component)]
pub struct LoadingUi;

/// Persistent ECS entity set captured immediately before a gameplay session is
/// spawned.
///
/// Every entity created after this snapshot belongs to that gameplay session
/// unless a future ownership API explicitly promotes it to persistent state.
/// This gives teardown one auditable boundary that also catches entities spawned
/// later during play (drones, effects, auras, projectiles, etc.) without asking
/// every subsystem to remember a cleanup marker.
#[derive(Resource, Default)]
pub struct GameplaySessionBaseline {
    persistent_entities: HashSet<Entity>,
}

impl GameplaySessionBaseline {
    fn owns(&self, entity: Entity) -> bool {
        !self.persistent_entities.contains(&entity)
    }
}

/// Marker for the selection indicator text.
#[derive(Component)]
pub struct SelectionIndicator(pub usize);

/// Nexus transition state — smooth launch animation.
#[derive(Resource, Default)]
pub struct NexusTransition {
    pub active: bool,
    pub target_phase: Option<GamePhase>,
    pub progress: f32, // 0.0 → 1.0 over ~1.5 seconds
}

/// Fade overlay for Nexus transitions.
#[derive(Component)]
pub struct NexusFade;

/// Spawn the Symtropy Nexus launcher.
///
/// Returning to the Nexus is also the whole-session teardown boundary. The
/// previous gameplay session is removed before new menu entities are queued,
/// and the coupled physics/consciousness/input state is replaced atomically at
/// the resource level. Fine-grained in-session body removal remains a separate
/// `PhysicsWorld` API requirement tracked by #19.
#[allow(clippy::too_many_arguments)]
pub fn setup_menu(
    mut commands: Commands,
    registry: Res<ExperienceRegistry>,
    baseline: Option<Res<GameplaySessionBaseline>>,
    all_entities: Query<Entity>,
    mut physics_world: ResMut<PhysicsWorldRes>,
    mut player_input: ResMut<PlayerInput>,
    mut leviathan: ResMut<LeviathanState>,
    mut biometrics: ResMut<BiometricsCtx>,
    #[cfg(feature = "fep-ai")] mut ai_player: Option<ResMut<super::ai_player::AiPlayer>>,
) {
    if let Some(baseline) = baseline.as_deref() {
        let mut queued = 0usize;
        for entity in &all_entities {
            if baseline.owns(entity) {
                // `try_despawn` is deliberate: despawning a parent recursively
                // removes descendants, so a later queued command for one of
                // those descendants must be allowed to become a no-op.
                commands.entity(entity).try_despawn();
                queued += 1;
            }
        }
        eprintln!("[symtropy] queued teardown for {queued} gameplay-session entities");
    }

    // Whole-session replacement keeps ECS PhysicsBody handles, exact physics
    // state, and consciousness registrations from crossing episode boundaries.
    *physics_world = PhysicsWorldRes::default();
    *player_input = PlayerInput::default();
    *leviathan = LeviathanState::default();
    biometrics.encoder.reset();
    biometrics.model.reset();

    #[cfg(feature = "fep-ai")]
    if let Some(mut ai_player) = ai_player {
        // Learning persistence must be an explicit experiment mode. The default
        // runtime treats beliefs/model/RNG/advisory counters as episode state so
        // same-seed replay can start from the same controller state.
        let enabled = ai_player.enabled;
        *ai_player = super::ai_player::AiPlayer::new();
        ai_player.enabled = enabled;
    }

    // Dark background
    commands.insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.04)));

    // Camera for UI rendering (Bevy UI requires a camera entity)
    commands.spawn((Camera2d, MenuUi));

    // Root container — full screen, centered
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 1.0)),
            MenuUi,
        ))
        .with_children(|parent| {
            // Inner block — left-aligned text within centered block
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart, // left-align the text
                    width: Val::Px(420.0),
                    ..default()
                })
                .with_children(|inner| {
                    // ═══ TITLE (centered within block) ═══════════════
                    inner.spawn((
                        Text::new("SYMTROPY"),
                        TextFont {
                            font_size: FontSize::Px(56.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.3, 0.95, 0.85)),
                        Node {
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                    ));

                    inner.spawn((
                        Text::new("consciousness-first technology"),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.75, 0.7, 0.7)),
                        Node {
                            margin: UiRect::bottom(Val::Px(40.0)),
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                    ));

                    // ═══ EXPERIENCE ENTRIES ══════════════════════════
                    for (i, exp) in registry.experiences.iter().enumerate() {
                        let is_selected = i == registry.selected;
                        let prefix = if is_selected { "> " } else { "  " };

                        // Experience name — selected: full color, unselected: muted slate
                        let (r, g, b, a) = if is_selected {
                            (exp.icon_color[0], exp.icon_color[1], exp.icon_color[2], 1.0)
                        } else {
                            (0.35, 0.45, 0.45, 0.7) // muted slate — legible without straining
                        };

                        inner.spawn((
                            Text::new(format!("{}[{}]  {}", prefix, i + 1, exp.name)),
                            TextFont {
                                font_size: FontSize::Px(22.0),
                                ..default()
                            },
                            TextColor(Color::srgba(r, g, b, a)),
                            Node {
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                            SelectionIndicator(i),
                        ));

                        // Subtitle
                        let sub_a = if is_selected { 0.6 } else { 0.4 };
                        inner.spawn((
                            Text::new(format!("      {}", exp.subtitle)),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgba(0.45, 0.55, 0.5, sub_a)),
                            Node {
                                margin: UiRect::bottom(Val::Px(16.0)),
                                ..default()
                            },
                            SelectionIndicator(i),
                        ));
                    }

                    // Settings option
                    inner.spawn((
                        Text::new("  [S]  Settings"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::srgba(0.4, 0.55, 0.5, 0.5)),
                        Node {
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        },
                    ));

                    inner.spawn((
                        Text::new("  [Esc]  Quit"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::srgba(0.4, 0.5, 0.45, 0.5)),
                        Node {
                            margin: UiRect::bottom(Val::Px(40.0)),
                            ..default()
                        },
                    ));

                    // ═══ FOOTER ═════════════════════════════════════
                    inner.spawn((
                        Text::new("Powered by Symthaea | Mycelix | Eight Harmonies"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgba(0.35, 0.5, 0.45, 0.5)),
                    ));

                    // Version
                    inner.spawn((
                        Text::new("v0.1.0"),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(Color::srgba(0.3, 0.4, 0.35, 0.4)),
                        Node {
                            margin: UiRect::top(Val::Px(4.0)),
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                    ));
                }); // close inner block
        }); // close root

    // Transition fade overlay (initially transparent)
    commands.spawn((
        Node {
            width: Val::Vw(100.0),
            height: Val::Vh(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ZIndex(i32::MAX),
        NexusFade,
        MenuUi,
    ));

    commands.insert_resource(NexusTransition::default());

    eprintln!(
        "[symtropy] Nexus displayed — {} experiences available",
        registry.experiences.len()
    );
}

/// Nexus input — navigate and launch experiences.
pub fn menu_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut seed: ResMut<DungeonSeed>,
    mut registry: ResMut<ExperienceRegistry>,
    mut indicators: Query<(&SelectionIndicator, &mut TextColor)>,
    mut app_exit: MessageWriter<bevy::app::AppExit>,
) {
    let count = registry.experiences.len();

    // Navigate: Up/Down arrows
    if keyboard.just_pressed(KeyCode::ArrowUp) && registry.selected > 0 {
        registry.selected -= 1;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) && registry.selected < count - 1 {
        registry.selected += 1;
    }

    // Quick-select: number keys
    if keyboard.just_pressed(KeyCode::Digit1) && count > 0 {
        registry.selected = 0;
    }
    if keyboard.just_pressed(KeyCode::Digit2) && count > 1 {
        registry.selected = 1;
    }
    if keyboard.just_pressed(KeyCode::Digit3) && count > 2 {
        registry.selected = 2;
    }

    // Update visual selection — selected: experience color, unselected: muted slate
    for (indicator, mut color) in indicators.iter_mut() {
        let exp = &registry.experiences[indicator.0];
        let is_selected = indicator.0 == registry.selected;
        let (r, g, b, a) = if is_selected {
            (exp.icon_color[0], exp.icon_color[1], exp.icon_color[2], 1.0)
        } else {
            (0.35, 0.45, 0.45, 0.7)
        };
        *color = TextColor(Color::srgba(r, g, b, a));
    }

    // Launch: Enter or N — starts transition animation
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::KeyN) {
        let exp = &registry.experiences[registry.selected];
        eprintln!("[nexus] Launching: {}", exp.name);

        if exp.phase == GamePhase::Loading {
            seed.0 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(42);
        }

        // Start transition (immediate for now — cinematic coalescence in future)
        next_state.set(exp.phase);
    }

    // Replay (for The Room)
    if keyboard.just_pressed(KeyCode::KeyR) {
        eprintln!("[nexus] Replay — seed: {}", seed.0);
        next_state.set(GamePhase::Loading);
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        app_exit.write(bevy::app::AppExit::Success);
    }
}

/// Living mycelial background — breathing network of nodes and connections.
pub fn nexus_background_system(mut gizmos: Gizmos, time: Res<Time>) {
    let t = time.elapsed_secs();
    let node_count = 60;

    // Generate deterministic node positions with slow drift
    for i in 0..node_count {
        let seed = i as f32 * 1.618;
        let base_x = (seed * 7.3).sin() * 400.0;
        let base_y = (seed * 3.7).cos() * 280.0;
        // Slow Brownian drift
        let drift_x = (t * 0.05 + seed * 2.1).sin() * 30.0;
        let drift_y = (t * 0.07 + seed * 1.3).cos() * 20.0;
        let x = base_x + drift_x;
        let y = base_y + drift_y;

        // Node glow
        let flicker = (t * 2.0 + seed).sin().abs() * 0.3 + 0.2;
        let node_color = Color::linear_rgba(0.0, 0.4 * flicker, 0.5 * flicker, flicker * 0.4);
        gizmos.circle_2d(Vec2::new(x, y), 2.0, node_color);

        // Connect to nearby nodes (deterministic pairs)
        for j in (i + 1)..node_count {
            let seed_j = j as f32 * 1.618;
            let jx = (seed_j * 7.3).sin() * 400.0 + (t * 0.05 + seed_j * 2.1).sin() * 30.0;
            let jy = (seed_j * 3.7).cos() * 280.0 + (t * 0.07 + seed_j * 1.3).cos() * 20.0;
            let dist = ((x - jx).powi(2) + (y - jy).powi(2)).sqrt();

            if dist < 120.0 && (i + j) % 3 == 0 {
                // Pulse along connection
                let pulse = (t * 0.3 + (i + j) as f32 * 0.5).sin().abs() * 0.15 + 0.05;
                let edge_color = Color::linear_rgba(0.0, 0.3 * pulse, 0.4 * pulse, pulse);
                gizmos.line_2d(Vec2::new(x, y), Vec2::new(jx, jy), edge_color);
            }
        }
    }
}

/// Despawn Nexus UI when leaving MainMenu state.
pub fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuUi>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Spawn loading screen and capture the persistent ECS baseline for this run.
///
/// `setup_loading` is the first system in the chained Loading setup. The menu UI
/// has already been removed by `OnExit(MainMenu)`, so the snapshot contains only
/// entities that are intended to outlive a gameplay session. Everything spawned
/// after this point is session-owned by default.
pub fn setup_loading(
    mut commands: Commands,
    seed: Res<DungeonSeed>,
    all_entities: Query<Entity>,
) {
    commands.insert_resource(GameplaySessionBaseline {
        persistent_entities: all_entities.iter().collect(),
    });

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            LoadingUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Generating dungeon..."),
                TextFont {
                    font_size: FontSize::Px(28.0),
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.8, 0.7)),
            ));
            parent.spawn((
                Text::new(format!("Seed: {}", seed.0)),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.6, 0.5)),
                Node {
                    margin: UiRect::top(Val::Px(12.0)),
                    ..default()
                },
            ));
        });
}

/// Cleanup loading screen.
pub fn cleanup_loading(mut commands: Commands, query: Query<Entity, With<LoadingUi>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_baseline_owns_only_entities_created_after_snapshot() {
        let mut world = World::new();
        let persistent = world.spawn_empty().id();
        let baseline = GameplaySessionBaseline {
            persistent_entities: HashSet::from([persistent]),
        };
        let session_entity = world.spawn_empty().id();

        assert!(!baseline.owns(persistent));
        assert!(baseline.owns(session_entity));
    }
}
