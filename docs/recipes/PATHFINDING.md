# Pathfinding with Oxidized Navigation

To enable complex NPC navigation in Symtropy levels, we recommend integrating [`oxidized_navigation`](https://github.com/TheGrimsey/oxidized_navigation). This recipe shows how to generate a navmesh from a Bevy physics world and use it alongside Symthaea's cognitive agents.

## 1. Dependencies

Add the following to your `Cargo.toml`:

```toml
[dependencies]
oxidized_navigation = "0.8" # Ensure compatibility with Bevy 0.18
```

## 2. Plugin Setup

Add the plugin to your main app, providing the required `NavMeshSettings`.

```rust
use bevy::prelude::*;
use oxidized_navigation::{
    NavMeshAffector, NavMeshSettings, OxidizedNavigationPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the navigation plugin
        .add_plugins(OxidizedNavigationPlugin::<Collider>::new(NavMeshSettings {
            cell_width: 0.25,
            cell_height: 0.1,
            tile_width: 100,
            world_half_extents: 250.0,
            world_bottom_bound: -100.0,
            max_traversable_slope_radians: (40.0_f32 - 0.1).to_radians(),
            walkable_height: 20,
            walkable_radius: 1,
            step_height: 3,
            min_region_area: 100,
            merge_region_area: 500,
            max_contour_simplification_error: 1.1,
            max_edge_length: 80,
            max_polygons_per_tile: 1024,
        }))
        .run();
}
```

## 3. Marking Colliders

You must mark static environment objects as `NavMeshAffector` so the system knows to build a mesh around them.

```rust
fn spawn_environment(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    // A simple floor
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(50.0, 50.0)),
            material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
            ..default()
        },
        Collider::cuboid(25.0, 0.1, 25.0),
        NavMeshAffector, // Crucial for generation!
    ));
}
```

## 4. Querying the NavMesh

Inside your FEP behavior loop or movement system, query the `NavMesh` resource to find a path.

```rust
use oxidized_navigation::NavMesh;

fn npc_pathfinding_system(
    nav_mesh: Res<NavMesh>,
    mut query: Query<(&mut Transform, &MoveTarget), With<CrewNpc>>,
) {
    if let Ok(nav_mesh) = nav_mesh.get().read() {
        for (transform, target) in &mut query {
            if let Some(dest) = target.target {
                let start_pos = transform.translation;
                let end_pos = dest;

                // Attempt to find a path
                if let Ok(path) = nav_mesh.find_path(start_pos, end_pos, None, None) {
                    // path is a Vec<Vec3> of waypoints
                    // Steer your NPC towards path[1] (path[0] is usually the start pos)
                }
            }
        }
    }
}
```
