// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Illustrative solar-scale Bevy viewer for precision/representation experiments.
//!
//! Orbital phases are deliberately illustrative, not ephemeris data. The viewer
//! retains canonical positions in LargeWorldPosition, rebases all scene transforms
//! to the moving camera each frame, and exaggerates far-away body markers only in
//! presentation space when physical apparent size falls below the configured band.
//! That exaggeration is reported in the HUD and never changes body coordinates.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::f64::consts::PI;
use symtropy_render_bridge::{
    ApparentSizeBand, ApparentSizeProfile, ApparentSizeProjection, CoordinateFrameId,
    LargeWorldPosition, WORLD_CELL_SIZE_METERS, projected_sphere_diameter_px,
};

const FRAME: CoordinateFrameId = CoordinateFrameId(1);
const AU_M: f64 = 149_597_870_700.0;
const EARTH_RADIUS_M: f64 = 6_371_008.4;
const MOON_MEAN_DISTANCE_M: f64 = 384_400_000.0;
const FOV_RADIANS: f64 = PI / 3.0;
const VIEWPORT_HEIGHT_PX: u32 = 720;
const POSITION_ERROR_BUDGET_FRACTION_OF_PIXEL: f64 = 0.05;
const MIN_POSITION_ERROR_BUDGET_M: f64 = 0.001;
const TRANSITION_SECONDS: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewStage {
    LocalInspection,
    PlanetSurface,
    LowOrbit,
    EarthMoon,
    InnerSystem,
    OuterSystem,
    ReturnToLocal,
}

impl ViewStage {
    const ALL: [Self; 7] = [
        Self::LocalInspection,
        Self::PlanetSurface,
        Self::LowOrbit,
        Self::EarthMoon,
        Self::InnerSystem,
        Self::OuterSystem,
        Self::ReturnToLocal,
    ];

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0)
    }

    fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    fn label(self) -> &'static str {
        match self {
            Self::LocalInspection => "1 / LOCAL INSPECTION",
            Self::PlanetSurface => "2 / PLANETARY SURFACE",
            Self::LowOrbit => "3 / LOW ORBIT",
            Self::EarthMoon => "4 / EARTH-MOON SYSTEM",
            Self::InnerSystem => "5 / INNER SOLAR SYSTEM",
            Self::OuterSystem => "6 / OUTER SOLAR SYSTEM",
            Self::ReturnToLocal => "7 / RETURN TO ORIGINAL FEATURE",
        }
    }

    fn is_surface_view(self) -> bool {
        matches!(
            self,
            Self::LocalInspection | Self::PlanetSurface | Self::ReturnToLocal
        )
    }

    fn stage_clip(self) -> (f64, f64) {
        match self {
            Self::LocalInspection | Self::ReturnToLocal => (0.05, 1_000.0),
            Self::PlanetSurface => (0.5, 200_000.0),
            Self::LowOrbit => (10_000.0, 1_000_000_000.0),
            Self::EarthMoon => (100_000.0, 1_200_000_000.0),
            Self::InnerSystem => (0.001 * AU_M, 8.0 * AU_M),
            Self::OuterSystem => (1.0 * AU_M, 80.0 * AU_M),
        }
    }

    fn camera_offset(self) -> [f64; 3] {
        match self {
            Self::LocalInspection | Self::ReturnToLocal => [0.0, 2.0, 8.0],
            Self::PlanetSurface => [0.0, 1_500.0, 3_000.0],
            Self::LowOrbit => [0.0, 0.0, 3.0 * EARTH_RADIUS_M],
            Self::EarthMoon => [0.0, 0.0, 500_000_000.0],
            Self::InnerSystem => [0.0, 0.0, 2.0 * AU_M],
            Self::OuterSystem => [0.0, 0.0, 40.0 * AU_M],
        }
    }

    fn tile_size_m(self) -> f32 {
        match self {
            Self::LocalInspection | Self::ReturnToLocal => 100.0,
            Self::PlanetSurface => 50_000.0,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SolarBody {
    id: &'static str,
    name: &'static str,
    position: LargeWorldPosition,
    physical_radius_m: f64,
    color: Color,
}

#[derive(Debug, Clone, Copy)]
struct CameraTransition {
    start_camera: LargeWorldPosition,
    goal_camera: LargeWorldPosition,
    start_focus: LargeWorldPosition,
    goal_focus: LargeWorldPosition,
    elapsed_seconds: f32,
    duration_seconds: f32,
    start_near: f64,
    goal_near: f64,
    start_far: f64,
    goal_far: f64,
}

#[derive(Resource)]
struct ViewerState {
    bodies: Vec<SolarBody>,
    surface_feature: LargeWorldPosition,
    stage: ViewStage,
    camera_position: LargeWorldPosition,
    focus_position: LargeWorldPosition,
    transition: Option<CameraTransition>,
    near_m: f64,
    far_m: f64,
    viewport_height_px: u32,
    bands: [usize; 6],
    visible_bodies: usize,
    visible_body_names: Vec<String>,
    visible_orbit_segments: usize,
    precision_rejections: usize,
    max_radius_exaggeration: f64,
}

#[derive(Component)]
struct BodyVisual {
    body_index: usize,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct SkyStar {
    direction: Vec3,
    size_factor: f32,
}

#[derive(Component)]
struct OrbitSegment {
    start: LargeWorldPosition,
    end: LargeWorldPosition,
    thickness_m: f64,
}

#[derive(Component)]
struct SurfaceTile;

#[derive(Component)]
struct LocalFeature;

#[derive(Component)]
struct SolarCamera;

#[derive(Component)]
struct SolarHud;

// Named ECS query contracts keep system signatures readable and make each
// disjoint render target explicit for Bevy's mutable-query alias analysis.
type BodyRenderQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static BodyVisual,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<BodyVisual>, Without<SolarCamera>),
>;
type TileRenderQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (
        With<SurfaceTile>,
        Without<BodyVisual>,
        Without<LocalFeature>,
        Without<SolarCamera>,
    ),
>;
type FeatureRenderQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (
        With<LocalFeature>,
        Without<BodyVisual>,
        Without<SurfaceTile>,
        Without<SolarCamera>,
    ),
>;
type OrbitRenderQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static OrbitSegment,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (
        With<OrbitSegment>,
        Without<BodyVisual>,
        Without<SurfaceTile>,
        Without<LocalFeature>,
        Without<SolarCamera>,
        Without<SkyStar>,
    ),
>;
type StarRenderQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SkyStar,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (
        With<SkyStar>,
        Without<BodyVisual>,
        Without<SurfaceTile>,
        Without<LocalFeature>,
        Without<OrbitSegment>,
        Without<SolarCamera>,
    ),
>;
type CameraRenderQuery<'w, 's> =
    Query<'w, 's, (&'static mut Transform, &'static mut Projection), With<SolarCamera>>;
type PrimaryWindowQuery<'w, 's> = Query<'w, 's, &'static Window, With<PrimaryWindow>>;

#[derive(SystemParam)]
struct CameraViewportParams<'w, 's> {
    camera_query: CameraRenderQuery<'w, 's>,
    windows: PrimaryWindowQuery<'w, 's>,
}

fn world_position(x_m: f64, y_m: f64, z_m: f64) -> LargeWorldPosition {
    LargeWorldPosition::from_absolute_meters(FRAME, [x_m, y_m, z_m])
        .expect("illustrative solar-system coordinate must fit large-world representation")
}

fn make_catalog() -> Vec<SolarBody> {
    let sun = world_position(0.0, 0.0, 0.0);
    let mercury = world_position(0.387 * AU_M, 0.0, 0.0);
    let venus = world_position(0.0, 0.723 * AU_M, 0.0);
    let earth = world_position(AU_M, 0.0, 0.0);
    let moon = earth
        .offset_by_meters([MOON_MEAN_DISTANCE_M, 0.0, 0.0])
        .expect("Moon offset fits world cell range");
    let mars = world_position(1.524 * AU_M, -0.08 * AU_M, 0.0);
    let jupiter = world_position(-5.204 * AU_M, 0.1 * AU_M, 0.0);
    let saturn = world_position(0.0, 9.58 * AU_M, 0.0);
    let uranus = world_position(19.2 * AU_M, -0.3 * AU_M, 0.0);
    let neptune = world_position(-30.07 * AU_M, -0.15 * AU_M, 0.0);

    vec![
        SolarBody {
            id: "sun",
            name: "Sun",
            position: sun,
            physical_radius_m: 695_700_000.0,
            color: Color::srgb(1.0, 0.72, 0.22),
        },
        SolarBody {
            id: "mercury",
            name: "Mercury",
            position: mercury,
            physical_radius_m: 2_439_700.0,
            color: Color::srgb(0.52, 0.50, 0.48),
        },
        SolarBody {
            id: "venus",
            name: "Venus",
            position: venus,
            physical_radius_m: 6_051_800.0,
            color: Color::srgb(0.88, 0.68, 0.36),
        },
        SolarBody {
            id: "earth",
            name: "Earth",
            position: earth,
            physical_radius_m: EARTH_RADIUS_M,
            color: Color::srgb(0.18, 0.40, 0.92),
        },
        SolarBody {
            id: "moon",
            name: "Moon",
            position: moon,
            physical_radius_m: 1_737_400.0,
            color: Color::srgb(0.66, 0.68, 0.70),
        },
        SolarBody {
            id: "mars",
            name: "Mars",
            position: mars,
            physical_radius_m: 3_389_500.0,
            color: Color::srgb(0.82, 0.28, 0.16),
        },
        SolarBody {
            id: "jupiter",
            name: "Jupiter",
            position: jupiter,
            physical_radius_m: 69_911_000.0,
            color: Color::srgb(0.76, 0.56, 0.38),
        },
        SolarBody {
            id: "saturn",
            name: "Saturn",
            position: saturn,
            physical_radius_m: 58_232_000.0,
            color: Color::srgb(0.82, 0.72, 0.48),
        },
        SolarBody {
            id: "uranus",
            name: "Uranus",
            position: uranus,
            physical_radius_m: 25_362_000.0,
            color: Color::srgb(0.40, 0.78, 0.82),
        },
        SolarBody {
            id: "neptune",
            name: "Neptune",
            position: neptune,
            physical_radius_m: 24_622_000.0,
            color: Color::srgb(0.18, 0.30, 0.92),
        },
    ]
}

fn make_sky_stars() -> Vec<SkyStar> {
    const STAR_COUNT: usize = 512;
    let golden_angle = PI * (3.0 - 5.0_f64.sqrt());
    (0..STAR_COUNT)
        .map(|index| {
            let z = 1.0 - 2.0 * (index as f64 + 0.5) / STAR_COUNT as f64;
            let radial = (1.0 - z * z).sqrt();
            let angle = golden_angle * index as f64;
            let direction = Vec3::new(
                (radial * angle.cos()) as f32,
                (radial * angle.sin()) as f32,
                z as f32,
            );
            let size_factor = 0.65 + ((index * 37 % 101) as f32 / 100.0) * 0.7;
            SkyStar {
                direction,
                size_factor,
            }
        })
        .collect()
}

fn make_orbit_segments(bodies: &[SolarBody]) -> Vec<OrbitSegment> {
    const SEGMENT_COUNT: u32 = 96;
    let sun = bodies
        .iter()
        .find(|body| body.id == "sun")
        .expect("Sun is catalogued");
    let earth = bodies
        .iter()
        .find(|body| body.id == "earth")
        .expect("Earth is catalogued");
    let mut segments = Vec::new();

    for body in bodies {
        let origin = if body.id == "moon" {
            earth.position
        } else {
            sun.position
        };
        if body.id == "sun" {
            continue;
        }
        let relative = body
            .position
            .relative_to_meters(&origin)
            .expect("illustrative bodies share a coordinate frame");
        let radius = relative[0].hypot(relative[1]).hypot(relative[2]);
        if !radius.is_finite() || radius <= 0.0 {
            continue;
        }

        // A thin presentation guide keeps the orbit readable at overview scale;
        // this thickness is not a physical orbital structure.
        let thickness_m = (radius * 0.002).max(10.0);
        for index in 0..SEGMENT_COUNT {
            let a0 = 2.0 * PI * f64::from(index) / f64::from(SEGMENT_COUNT);
            let a1 = 2.0 * PI * f64::from(index + 1) / f64::from(SEGMENT_COUNT);
            let start = origin
                .offset_by_meters([radius * a0.cos(), radius * a0.sin(), 0.0])
                .expect("orbit waypoint is representable");
            let end = origin
                .offset_by_meters([radius * a1.cos(), radius * a1.sin(), 0.0])
                .expect("orbit waypoint is representable");
            segments.push(OrbitSegment {
                start,
                end,
                thickness_m,
            });
        }
    }
    segments
}

fn focus_for_stage(
    stage: ViewStage,
    bodies: &[SolarBody],
    surface_feature: LargeWorldPosition,
) -> LargeWorldPosition {
    match stage {
        ViewStage::LocalInspection | ViewStage::PlanetSurface | ViewStage::ReturnToLocal => {
            surface_feature
        }
        ViewStage::LowOrbit => {
            bodies
                .iter()
                .find(|body| body.id == "earth")
                .expect("Earth is catalogued")
                .position
        }
        ViewStage::EarthMoon => bodies
            .iter()
            .find(|body| body.id == "earth")
            .expect("Earth is catalogued")
            .position
            .offset_by_meters([MOON_MEAN_DISTANCE_M * 0.5, 0.0, 0.0])
            .expect("Earth-Moon midpoint is representable"),
        ViewStage::InnerSystem | ViewStage::OuterSystem => {
            bodies
                .iter()
                .find(|body| body.id == "sun")
                .expect("Sun is catalogued")
                .position
        }
    }
}

fn camera_goal(
    stage: ViewStage,
    bodies: &[SolarBody],
    surface_feature: LargeWorldPosition,
) -> (LargeWorldPosition, LargeWorldPosition) {
    let focus = focus_for_stage(stage, bodies, surface_feature);
    let camera = focus
        .offset_by_meters(stage.camera_offset())
        .expect("camera waypoint fits large-world coordinate range");
    (camera, focus)
}

fn interpolate_position(
    start: LargeWorldPosition,
    goal: LargeWorldPosition,
    fraction: f64,
) -> Option<LargeWorldPosition> {
    let delta = goal.relative_to_meters(&start).ok()?;
    start
        .offset_by_meters([
            delta[0] * fraction,
            delta[1] * fraction,
            delta[2] * fraction,
        ])
        .ok()
}

fn start_transition(state: &mut ViewerState, goal_stage: ViewStage) {
    let (goal_camera, goal_focus) = camera_goal(goal_stage, &state.bodies, state.surface_feature);
    let (goal_near, goal_far) = goal_stage.stage_clip();
    state.stage = goal_stage;
    state.transition = Some(CameraTransition {
        start_camera: state.camera_position,
        goal_camera,
        start_focus: state.focus_position,
        goal_focus,
        elapsed_seconds: 0.0,
        duration_seconds: TRANSITION_SECONDS,
        start_near: state.near_m,
        goal_near,
        start_far: state.far_m,
        goal_far,
    });
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clear_color: ResMut<ClearColor>,
    mut state: ResMut<ViewerState>,
) {
    clear_color.0 = Color::srgb(0.008, 0.012, 0.028);
    let body_mesh = meshes.add(Sphere::new(1.0).mesh().uv(32, 20));

    for (body_index, body) in state.bodies.iter().enumerate() {
        let material = materials.add(StandardMaterial {
            base_color: body.color,
            perceptual_roughness: 0.8,
            metallic: 0.0,
            ..default()
        });
        commands.spawn((
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Hidden,
            BodyVisual { body_index },
        ));
    }

    // Deterministic circular guides are separate render representations; the
    // catalog positions are not advanced along them and remain illustrative.
    let orbit_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let orbit_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.34, 0.58),
        unlit: true,
        ..default()
    });
    for segment in make_orbit_segments(&state.bodies) {
        commands.spawn((
            Mesh3d(orbit_mesh.clone()),
            MeshMaterial3d(orbit_material.clone()),
            Transform::default(),
            Visibility::Hidden,
            segment,
        ));
    }

    let tile_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    let tile_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.18, 0.15),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(tile_mesh),
        MeshMaterial3d(tile_material),
        Transform::default(),
        Visibility::Hidden,
        SurfaceTile,
    ));

    let feature_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let feature_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.24, 0.07),
        perceptual_roughness: 0.65,
        metallic: 0.15,
        ..default()
    });
    commands.spawn((
        Mesh3d(feature_mesh),
        MeshMaterial3d(feature_material),
        Transform::default(),
        Visibility::Hidden,
        LocalFeature,
    ));

    // Camera-relative synthetic starfield. These are angular background markers,
    // not catalog stars with physical distance or ephemeris coordinates.
    let star_mesh = meshes.add(Sphere::new(1.0).mesh().uv(8, 6));
    let star_palette = [
        Color::srgb(0.50, 0.64, 0.88),
        Color::srgb(0.78, 0.84, 0.96),
        Color::srgb(1.0, 0.90, 0.70),
    ];
    let star_materials: Vec<Handle<StandardMaterial>> = star_palette
        .into_iter()
        .map(|color| {
            materials.add(StandardMaterial {
                base_color: color,
                unlit: true,
                ..default()
            })
        })
        .collect();
    for (index, star) in make_sky_stars().into_iter().enumerate() {
        commands.spawn((
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_materials[index % star_materials.len()].clone()),
            Transform::default(),
            Visibility::Visible,
            star,
        ));
    }

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(5.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let (initial_near, initial_far) = state.stage.stage_clip();
    state.near_m = initial_near;
    state.far_m = initial_far;
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV_RADIANS as f32,
            near: initial_near as f32,
            far: initial_far as f32,
            ..default()
        }),
        Transform::IDENTITY,
        SolarCamera,
    ));

    commands.spawn((
        Text::new("Symtropy solar-scale explorer"),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(12.0),
            ..default()
        },
        SolarHud,
    ));
}

fn handle_controls(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<ViewerState>) {
    let requested = if keys.just_pressed(KeyCode::Digit1) {
        Some(ViewStage::LocalInspection)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(ViewStage::PlanetSurface)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(ViewStage::LowOrbit)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(ViewStage::EarthMoon)
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(ViewStage::InnerSystem)
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(ViewStage::OuterSystem)
    } else if keys.just_pressed(KeyCode::Digit7) {
        Some(ViewStage::ReturnToLocal)
    } else if keys.just_pressed(KeyCode::Space) {
        Some(ViewStage::from_index(state.stage.index() + 1))
    } else {
        None
    };

    if let Some(stage) = requested.filter(|stage| *stage != state.stage) {
        start_transition(&mut state, stage);
    }
}

fn advance_transition(time: Res<Time>, mut state: ResMut<ViewerState>) {
    let Some(mut movement) = state.transition else {
        return;
    };

    movement.elapsed_seconds += time.delta_secs();
    let raw = (movement.elapsed_seconds / movement.duration_seconds).clamp(0.0, 1.0);
    let smooth = raw * raw * (3.0 - 2.0 * raw);
    let fraction = f64::from(smooth);

    if let Some(camera) =
        interpolate_position(movement.start_camera, movement.goal_camera, fraction)
    {
        state.camera_position = camera;
    } else {
        state.precision_rejections = state.precision_rejections.saturating_add(1);
    }

    if let Some(focus) = interpolate_position(movement.start_focus, movement.goal_focus, fraction) {
        state.focus_position = focus;
    } else {
        state.precision_rejections = state.precision_rejections.saturating_add(1);
    }

    state.near_m =
        (movement.start_near.ln() * (1.0 - fraction) + movement.goal_near.ln() * fraction).exp();
    state.far_m =
        (movement.start_far.ln() * (1.0 - fraction) + movement.goal_far.ln() * fraction).exp();

    if raw >= 1.0 {
        state.camera_position = movement.goal_camera;
        state.focus_position = movement.goal_focus;
        state.near_m = movement.goal_near;
        state.far_m = movement.goal_far;
        state.transition = None;
    } else {
        state.transition = Some(movement);
    }
}

fn band_index(band: ApparentSizeBand) -> usize {
    match band {
        ApparentSizeBand::Subpixel => 0,
        ApparentSizeBand::PointSource => 1,
        ApparentSizeBand::ResolvedDisc => 2,
        ApparentSizeBand::BodyProxy => 3,
        ApparentSizeBand::RegionalDetail => 4,
        ApparentSizeBand::SurfaceDetail => 5,
    }
}

fn position_error_budget_m(distance_m: f64, viewport_height_px: u32) -> Option<f64> {
    if !distance_m.is_finite() || distance_m <= 0.0 || viewport_height_px == 0 {
        return None;
    }
    let one_pixel_world_m =
        2.0 * (FOV_RADIANS / 2.0).tan() * distance_m / f64::from(viewport_height_px);
    let budget_m = (one_pixel_world_m * POSITION_ERROR_BUDGET_FRACTION_OF_PIXEL)
        .max(MIN_POSITION_ERROR_BUDGET_M);
    budget_m.is_finite().then_some(budget_m)
}

fn render_radius_for_band(
    physical_radius_m: f64,
    distance_m: f64,
    band: ApparentSizeBand,
    viewport_height_px: u32,
) -> f64 {
    let target_diameter_px = match band {
        ApparentSizeBand::Subpixel => Some(0.6),
        ApparentSizeBand::PointSource => Some(0.8),
        _ => None,
    };
    target_diameter_px.map_or(physical_radius_m, |target_px| {
        let marker_radius_m =
            target_px * distance_m * (FOV_RADIANS / 2.0).tan() / f64::from(viewport_height_px);
        physical_radius_m.max(marker_radius_m)
    })
}

fn update_scene(
    mut state: ResMut<ViewerState>,
    mut body_query: BodyRenderQuery<'_, '_>,
    mut tile_query: TileRenderQuery<'_, '_>,
    mut feature_query: FeatureRenderQuery<'_, '_>,
    mut orbit_query: OrbitRenderQuery<'_, '_>,
    mut star_query: StarRenderQuery<'_, '_>,
    mut camera_viewport: CameraViewportParams<'_, '_>,
) {
    let viewport_height_px = camera_viewport
        .windows
        .iter()
        .next()
        .map(|window| window.physical_height().max(1))
        .unwrap_or(VIEWPORT_HEIGHT_PX);
    state.viewport_height_px = viewport_height_px;
    state.bands = [0; 6];
    state.visible_bodies = 0;
    state.visible_body_names.clear();
    state.visible_orbit_segments = 0;
    state.precision_rejections = 0;
    state.max_radius_exaggeration = 1.0;

    let profile = ApparentSizeProfile::proposed_default();
    let surface_view = state.stage.is_surface_view();

    for (visual, mut transform, mut visibility) in &mut body_query {
        let body = state.bodies[visual.body_index];

        if surface_view {
            *visibility = Visibility::Hidden;
            continue;
        }

        let relative = match body.position.relative_to_meters(&state.camera_position) {
            Ok(value) => value,
            Err(_) => {
                state.precision_rejections = state.precision_rejections.saturating_add(1);
                *visibility = Visibility::Hidden;
                continue;
            }
        };
        let distance_m = relative[0].hypot(relative[1]).hypot(relative[2]);
        if !distance_m.is_finite()
            || distance_m + body.physical_radius_m < state.near_m
            || distance_m - body.physical_radius_m > state.far_m
        {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some(error_budget_m) = position_error_budget_m(distance_m, viewport_height_px) else {
            state.precision_rejections = state.precision_rejections.saturating_add(1);
            *visibility = Visibility::Hidden;
            continue;
        };
        let relative_f32 = match body
            .position
            .relative_to_f32_with_error_budget(&state.camera_position, error_budget_m)
        {
            Ok(value) => value,
            Err(_) => {
                state.precision_rejections = state.precision_rejections.saturating_add(1);
                *visibility = Visibility::Hidden;
                continue;
            }
        };

        let apparent_diameter = match projected_sphere_diameter_px(
            body.physical_radius_m,
            ApparentSizeProjection::Perspective {
                viewport_height_px,
                vertical_fov_radians: FOV_RADIANS,
                centre_distance: distance_m,
            },
        ) {
            Ok(value) => value,
            Err(_) => {
                *visibility = Visibility::Hidden;
                continue;
            }
        };
        let band = match profile.classify(apparent_diameter) {
            Ok(value) => value,
            Err(_) => {
                *visibility = Visibility::Hidden;
                continue;
            }
        };
        state.bands[band_index(band)] += 1;
        state.visible_bodies += 1;
        state.visible_body_names.push(body.name.to_owned());

        let visual_radius =
            render_radius_for_band(body.physical_radius_m, distance_m, band, viewport_height_px);
        state.max_radius_exaggeration = state
            .max_radius_exaggeration
            .max(visual_radius / body.physical_radius_m);

        transform.translation = Vec3::new(relative_f32[0], relative_f32[1], relative_f32[2]);
        transform.scale = Vec3::splat(visual_radius as f32);
        *visibility = Visibility::Visible;
    }

    let orbit_paths_visible = matches!(
        state.stage,
        ViewStage::LowOrbit
            | ViewStage::EarthMoon
            | ViewStage::InnerSystem
            | ViewStage::OuterSystem
    );
    for (segment, mut transform, mut visibility) in &mut orbit_query {
        if !orbit_paths_visible {
            *visibility = Visibility::Hidden;
            continue;
        }

        let start_relative = match segment.start.relative_to_meters(&state.camera_position) {
            Ok(value) => value,
            Err(_) => {
                state.precision_rejections = state.precision_rejections.saturating_add(1);
                *visibility = Visibility::Hidden;
                continue;
            }
        };
        let end_relative = match segment.end.relative_to_meters(&state.camera_position) {
            Ok(value) => value,
            Err(_) => {
                state.precision_rejections = state.precision_rejections.saturating_add(1);
                *visibility = Visibility::Hidden;
                continue;
            }
        };
        let start_distance = start_relative[0]
            .hypot(start_relative[1])
            .hypot(start_relative[2]);
        let end_distance = end_relative[0]
            .hypot(end_relative[1])
            .hypot(end_relative[2]);
        let (Some(start_error_budget), Some(end_error_budget)) = (
            position_error_budget_m(start_distance, viewport_height_px),
            position_error_budget_m(end_distance, viewport_height_px),
        ) else {
            state.precision_rejections = state.precision_rejections.saturating_add(1);
            *visibility = Visibility::Hidden;
            continue;
        };
        let start_f32 = segment
            .start
            .relative_to_f32_with_error_budget(&state.camera_position, start_error_budget);
        let end_f32 = segment
            .end
            .relative_to_f32_with_error_budget(&state.camera_position, end_error_budget);
        let (Ok(start_f32), Ok(end_f32)) = (start_f32, end_f32) else {
            state.precision_rejections = state.precision_rejections.saturating_add(1);
            *visibility = Visibility::Hidden;
            continue;
        };

        let start_position = Vec3::new(start_f32[0], start_f32[1], start_f32[2]);
        let end_position = Vec3::new(end_f32[0], end_f32[1], end_f32[2]);
        let delta = end_position - start_position;
        let length = delta.length();
        if !length.is_finite() || length <= f32::EPSILON {
            *visibility = Visibility::Hidden;
            continue;
        }

        transform.translation = (start_position + end_position) * 0.5;
        transform.rotation = Quat::from_rotation_arc(Vec3::X, delta / length);
        let width = segment.thickness_m as f32;
        transform.scale = Vec3::new(length, width, width);
        *visibility = Visibility::Visible;
        state.visible_orbit_segments += 1;
    }

    let surface_relative = match state
        .surface_feature
        .relative_to_meters(&state.camera_position)
    {
        Ok(relative) => Some(relative),
        Err(_) => {
            state.precision_rejections = state.precision_rejections.saturating_add(1);
            None
        }
    };
    let tile_size = state.stage.tile_size_m();
    for (mut transform, mut visibility) in &mut tile_query {
        if tile_size > 0.0 {
            if let Some(relative) = surface_relative {
                transform.translation =
                    Vec3::new(relative[0] as f32, relative[1] as f32, relative[2] as f32);
                transform.scale = Vec3::new(tile_size, 1.0, tile_size);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    for (mut transform, mut visibility) in &mut feature_query {
        if matches!(
            state.stage,
            ViewStage::LocalInspection | ViewStage::ReturnToLocal
        ) {
            let feature_position = state
                .surface_feature
                .offset_by_meters([0.0, 1.0, 0.0])
                .and_then(|position| position.relative_to_meters(&state.camera_position));
            if let Ok(relative) = feature_position {
                transform.translation =
                    Vec3::new(relative[0] as f32, relative[1] as f32, relative[2] as f32);
                transform.scale = Vec3::splat(2.0);
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
                state.precision_rejections += 1;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    let star_depth = state.far_m * 0.94;
    let star_radius = star_depth * (FOV_RADIANS / 2.0).tan() / f64::from(viewport_height_px) * 0.5;
    for (star, mut transform, mut visibility) in &mut star_query {
        transform.translation = star.direction * star_depth as f32;
        transform.scale = Vec3::splat((star_radius as f32) * star.size_factor);
        *visibility = Visibility::Visible;
    }

    let focus_relative = match state
        .focus_position
        .relative_to_meters(&state.camera_position)
    {
        Ok(relative) => Some(relative),
        Err(_) => {
            state.precision_rejections = state.precision_rejections.saturating_add(1);
            None
        }
    };
    for (mut transform, mut projection) in &mut camera_viewport.camera_query {
        transform.translation = Vec3::ZERO;
        if let Some(focus_relative) = focus_relative {
            transform.rotation = Transform::from_translation(Vec3::ZERO)
                .looking_at(
                    Vec3::new(
                        focus_relative[0] as f32,
                        focus_relative[1] as f32,
                        focus_relative[2] as f32,
                    ),
                    Vec3::Y,
                )
                .rotation;
        } else {
            // Fail closed: point the camera along a neutral axis instead of
            // fabricating a plausible coordinate from an unavailable transform.
            transform.rotation = Quat::IDENTITY;
        }

        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.fov = FOV_RADIANS as f32;
            perspective.near = state.near_m as f32;
            perspective.far = state.far_m as f32;
        }
    }
}

fn update_hud(state: Res<ViewerState>, mut query: Query<&mut Text, With<SolarHud>>) {
    let movement = if state.transition.is_some() {
        "camera rebasing in flight"
    } else {
        "camera settled"
    };
    let visible_body_names = if state.visible_body_names.is_empty() {
        "none".to_owned()
    } else {
        state.visible_body_names.join(", ")
    };
    let message = format!(
        "SYMTRY / SOLAR-SCALE EXPLORER  [ILLUSTRATIVE ORBITS + SYNTHETIC STARS — NOT EPHEMERIS]\n{}\n{}\nVisible bodies: {} | orbit segments: {} | coordinate conversions rejected: {}\nBodies: {}\nSynthetic sky markers are angular background only; orbit-line widths are exaggerated.\nApparent bands  subpixel {}  point {}  disc {}  proxy {}  regional {}  surface {}\nMaximum visual radius exaggeration: {:.1}x\nViewport: {} physical pixels high | cell size: {:.0} m | camera-relative origin is presentation-only\n\n1 Local  2 Surface  3 Low orbit  4 Earth-Moon  5 Inner system  6 Outer system  7 Return  | Space: next",
        state.stage.label(),
        movement,
        state.visible_bodies,
        state.visible_orbit_segments,
        state.precision_rejections,
        visible_body_names,
        state.bands[0],
        state.bands[1],
        state.bands[2],
        state.bands[3],
        state.bands[4],
        state.bands[5],
        state.max_radius_exaggeration,
        state.viewport_height_px,
        WORLD_CELL_SIZE_METERS
    );
    for mut text in &mut query {
        text.0.clone_from(&message);
    }
}

fn build_initial_state() -> ViewerState {
    let bodies = make_catalog();
    let earth = bodies
        .iter()
        .find(|body| body.id == "earth")
        .expect("Earth is catalogued");
    let surface_feature = earth
        .position
        .offset_by_meters([0.0, EARTH_RADIUS_M, 0.0])
        .expect("surface feature remains in supported coordinate range");
    let stage = ViewStage::LocalInspection;
    let (camera_position, focus_position) = camera_goal(stage, &bodies, surface_feature);
    let (near_m, far_m) = stage.stage_clip();

    ViewerState {
        bodies,
        surface_feature,
        stage,
        camera_position,
        focus_position,
        transition: None,
        near_m,
        far_m,
        viewport_height_px: VIEWPORT_HEIGHT_PX,
        bands: [0; 6],
        visible_bodies: 0,
        visible_body_names: Vec::new(),
        visible_orbit_segments: 0,
        precision_rejections: 0,
        max_radius_exaggeration: 1.0,
    }
}

fn main() {
    App::new()
        .insert_resource(build_initial_state())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Symtropy Solar-Scale Explorer (illustrative)".to_owned(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_controls,
                advance_transition,
                update_scene,
                update_hud,
            )
                .chain(),
        )
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn illustrative_catalog_reaches_neptune_scale() {
        let bodies = make_catalog();
        let sun = bodies.iter().find(|body| body.id == "sun").expect("Sun");
        let neptune = bodies
            .iter()
            .find(|body| body.id == "neptune")
            .expect("Neptune");
        let delta = neptune
            .position
            .relative_to_meters(&sun.position)
            .expect("common illustrative frame");
        let radius = delta[0].hypot(delta[1]).hypot(delta[2]);
        assert!(radius > 4.4e12);
        assert!(radius < 4.7e12);
    }

    #[test]
    fn synthetic_skyfield_is_deterministic_and_directions_are_normalized() {
        let first = make_sky_stars();
        let second = make_sky_stars();
        assert_eq!(first.len(), 512);
        assert_eq!(first, second);
        assert!(first.iter().all(|star| {
            let norm = star.direction.length();
            (norm - 1.0).abs() < 1e-5 && star.size_factor.is_finite() && star.size_factor > 0.0
        }));
    }

    #[test]
    fn orbital_guide_segments_span_the_catalog_without_mutating_body_positions() {
        let bodies = make_catalog();
        let before = bodies.clone();
        let segments = make_orbit_segments(&bodies);
        assert_eq!(segments.len(), 9 * 96);
        assert_eq!(bodies, before);
        assert!(
            segments
                .iter()
                .all(|segment| segment.thickness_m.is_finite() && segment.thickness_m > 0.0)
        );
        for segment in segments {
            let delta = segment
                .end
                .relative_to_meters(&segment.start)
                .expect("segment frame");
            let length = delta[0].hypot(delta[1]).hypot(delta[2]);
            assert!(length.is_finite() && length > 0.0);
        }
    }

    #[test]
    fn focus_and_camera_waypoints_remain_in_the_same_canonical_frame() {
        let state = build_initial_state();
        for stage in ViewStage::ALL {
            let (camera, focus) = camera_goal(stage, &state.bodies, state.surface_feature);
            assert_eq!(camera.frame(), focus.frame());
            assert!(camera.relative_to_meters(&focus).is_ok());
        }
    }

    #[test]
    fn position_error_budget_scales_with_distance_and_render_resolution() {
        let distance_m = AU_M;
        let base = position_error_budget_m(distance_m, 720).expect("valid baseline budget");
        let doubled_distance =
            position_error_budget_m(distance_m * 2.0, 720).expect("valid doubled distance");
        let doubled_resolution =
            position_error_budget_m(distance_m, 1_440).expect("valid doubled resolution");

        assert!(((doubled_distance / base) - 2.0).abs() < 1e-12);
        assert!(((doubled_resolution / base) - 0.5).abs() < 1e-12);
        assert_eq!(
            position_error_budget_m(1e-6, 720),
            Some(MIN_POSITION_ERROR_BUDGET_M)
        );
    }

    #[test]
    fn position_error_budget_rejects_non_finite_or_undefined_inputs() {
        assert_eq!(position_error_budget_m(0.0, 720), None);
        assert_eq!(position_error_budget_m(-1.0, 720), None);
        assert_eq!(position_error_budget_m(f64::NAN, 720), None);
        assert_eq!(position_error_budget_m(f64::INFINITY, 720), None);
        assert_eq!(position_error_budget_m(AU_M, 0), None);
    }

    #[test]
    fn render_marker_exaggeration_does_not_mutate_physical_radius_or_position() {
        let radius = EARTH_RADIUS_M;
        let visual_radius = render_radius_for_band(
            radius,
            40.0 * AU_M,
            ApparentSizeBand::Subpixel,
            VIEWPORT_HEIGHT_PX,
        );
        assert!(visual_radius > radius);
        let bodies = make_catalog();
        let earth = bodies
            .iter()
            .find(|body| body.id == "earth")
            .expect("Earth");
        assert_eq!(earth.physical_radius_m, radius);
        assert_eq!(earth.position, world_position(AU_M, 0.0, 0.0));
    }
}
