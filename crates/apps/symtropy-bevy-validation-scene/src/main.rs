// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Minimal executable smoke scene for the permissive Symtropy Bevy stack.

use bevy::prelude::*;
use symtropy_bevy_scene::{SymtropyScenePlugin, fixed_camera};
use symtropy_core::prelude::BevyPhysicsPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SymtropyScenePlugin::default())
        .add_plugins(BevyPhysicsPlugin::<3>::with_gravity([0.0, -9.81, 0.0]))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(fixed_camera(Vec3::new(5.0, 5.0, 5.0), Vec3::ZERO));
}
