// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sol Atlas globe view integration.
//!
//! Press M during gameplay to open the planetary map.
//! Press Escape to return to the dungeon.

use bevy::prelude::*;
use sol_atlas_bevy::camera::OrbitalCamera;
use sol_atlas_bevy::globe::{Atmosphere, Globe};
use sol_atlas_bevy::markers::DataMarker;
use sol_atlas_bevy::timeline::{TimelineLayer, TimelineState};
use sol_atlas_core::geo;
use sol_atlas_core::lod::LodLevel;
use sol_atlas_core::types::{CountryProfile, Layer, OrbitalObjectType};

/// Tag for markers that are only visible at Surface LOD (close zoom).
#[derive(Component)]
pub struct SurfaceLod;

/// Tag for heat blob markers visible at Orbit LOD (far zoom).
#[derive(Component)]
pub struct OrbitLod;

/// 4D temporal position for a marker — W coordinate = year.
/// When 4D mode is active, markers fade based on distance from the timeline slice.
#[derive(Component)]
pub struct TemporalW {
    pub year: f64,
}

/// Per-fossil-deposit data needed to compute timeline-driven opacity/EROI
/// color without re-deriving those formulas locally (added 2026-07-27,
/// extending the sol-atlas-leptos timeline-unification fix to this
/// renderer — `sol-atlas-core::timeline`'s own doc comment names
/// sol-atlas-bevy as a mandatory consumer, but until now
/// `timeline_visibility_system`'s Fossil arm used a generic fade
/// ("per-deposit opacity needs stored data") and nothing here ever called
/// `fossil_eroi_at_year`, so scrubbing the Bevy timeline never changed a
/// deposit's color, only a uniform fade. Attached to both the main marker
/// and its CO2 halo, since both need status-based opacity; only the main
/// marker also gets `FossilMainMarker` for the EROI color update.
#[derive(Component, Clone)]
pub struct FossilTimelineData {
    pub status: String,
    pub fuel_type: sol_atlas_core::types::FuelType,
    pub base_eroi: f64,
}

/// Tags the primary fossil-deposit marker (not its CO2 halo) so
/// `fossil_eroi_color_system` only recolors the one entity whose color is
/// actually EROI-derived — the halo's color is a fixed translucent red
/// regardless of EROI.
#[derive(Component)]
pub struct FossilMainMarker;

use crate::resources::GamePhase;

/// Marker for all atlas-spawned entities so we can despawn them on exit.
#[derive(Component)]
pub struct AtlasEntity;

/// Cached `solar_system_bodies()` output. That data never changes after
/// startup, but (until 2026-07-26) three separate per-frame systems
/// (`celestial_body_update_system`, `celestial_orbit_system`,
/// `planet_focus_system`) each rebuilt it from scratch every single
/// frame -- a fresh `Vec` of 6 structs, each with several heap-allocated
/// `String` fields, discarded 60 times a second for no reason. Computed
/// once via `Default`/`init_resource`.
#[derive(Resource)]
pub struct SolarSystemBodiesRes(pub Vec<sol_atlas_core::solar_system::CelestialBody>);

impl Default for SolarSystemBodiesRes {
    fn default() -> Self {
        Self(sol_atlas_core::solar_system::solar_system_bodies())
    }
}

/// Marker for a solar system body mesh — enables per-frame orbital position updates.
#[derive(Component)]
pub struct CelestialBodyMesh {
    /// Index into solar_system_bodies() for position lookup.
    pub body_index: usize,
    /// Whether this is a corona (outer glow) vs the body itself.
    pub is_corona: bool,
}

/// Marks a `CelestialBodyMesh` entity as currently wearing the holographic
/// material (added 2026-07-26 for "the currently selected planet should be
/// holographic so features can be clearly displayed") rather than its plain
/// `StandardMaterial`. Tracked explicitly because `MeshMaterial3d<T>` is a
/// distinct component per `T` — this marker is how
/// `planet_holographic_focus_system` knows which state an entity is
/// currently in, so it knows whether to swap forward or back.
#[derive(Component)]
pub struct HolographicFocused;

/// Cloud layer marker — rotates independently for parallax depth.
#[derive(Component)]
pub struct CloudLayer;

/// Sacred-geometry wireframe sphere marker — gated by `OverlayManager::show_grid`.
#[derive(Component)]
pub struct GridLayer;

/// City indicator — always visible at orbit, shows grid stress.
#[derive(Component)]
pub struct CityIndicator {
    pub name: String,
    pub load: f32,
}

/// HUD element showing timeline year + Turchin cycle phase.
#[derive(Component)]
pub struct TimelineHud;

/// Active data view preset — filters which layers are visible.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub enum DataView {
    /// No data overlays — just the globe. Default, so first impressions
    /// aren't a wall of arcs/markers obscuring the landmass.
    Off,
    All,
    Energy,
    Climate,
    Civilization,
    /// Population, literacy, life expectancy, urbanization, food security —
    /// people-centric real per-country stats. One of 3 thematic sub-groups
    /// the 15 country-profile layers were split into 2026-07-26 (were
    /// briefly all dumped into `Civilization` at once, which read as
    /// visually dense/undifferentiated).
    HumanDevelopment,
    /// GDP per capita, unemployment, sovereignty (UN membership/
    /// independence), internet access — economic + governance +
    /// connectivity real per-country stats.
    EconomyGovernance,
    /// Electricity/clean-cooking access, CO2 emissions, renewable energy,
    /// forest cover, water stress — energy + environment + resources real
    /// per-country stats.
    EnergyEnvironment,
    Infrastructure,
    Interplanetary,
    /// Speculative planning fiction (Maglev, Resontia Vaults, Terra Lumina,
    /// Geothermal, Robotics, DeSci Evidence Mesh) — none of these exist.
    /// Kept out of every other preset (including `All`) so a first-time
    /// viewer's "real data" impression is never silently mixed with
    /// scenario content; reachable only by explicitly cycling here.
    Scenarios,
}

impl Default for DataView {
    fn default() -> Self {
        Self::Off
    }
}

impl DataView {
    /// Every variant, in the same order `next()` cycles through them --
    /// used to spawn one clickable button per view (2026-07-27, Bevy-side
    /// parity with sol-atlas-leptos's mode/lens picker: this enum already
    /// plays the same role as Leptos's `Lens`, it just had no visible
    /// picker UI before this, only blind Tab-cycling).
    pub const ALL: [DataView; 11] = [
        Self::Off,
        Self::All,
        Self::Energy,
        Self::Climate,
        Self::Civilization,
        Self::HumanDevelopment,
        Self::EconomyGovernance,
        Self::EnergyEnvironment,
        Self::Infrastructure,
        Self::Interplanetary,
        Self::Scenarios,
    ];

    /// HUD/button tint for this view. Extracted from `data_view_switch_system`
    /// so the Tab-key path and the new click-button path can't drift apart.
    pub fn color(&self) -> [f32; 3] {
        match self {
            Self::Off => [0.4, 0.4, 0.45],
            Self::All => [0.5, 0.7, 0.8],
            Self::Energy => [1.0, 0.8, 0.2],
            Self::Climate => [0.9, 0.3, 0.2],
            Self::Civilization => [0.3, 0.8, 0.5],
            Self::HumanDevelopment => [0.9, 0.4, 0.6],
            Self::EconomyGovernance => [0.6, 0.4, 0.9],
            Self::EnergyEnvironment => [0.2, 0.75, 0.6],
            Self::Infrastructure => [0.8, 0.6, 0.3],
            Self::Interplanetary => [0.4, 0.5, 0.9],
            // Warm amber warning tone — visually distinct from every
            // real-data view, so the "this isn't real" label reads even at
            // a glance.
            Self::Scenarios => [0.95, 0.55, 0.15],
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "OVERLAYS OFF",
            Self::All => "ALL DATA",
            Self::Energy => "ENERGY",
            Self::Climate => "CLIMATE",
            Self::Civilization => "CIVILIZATION",
            Self::HumanDevelopment => "HUMAN DEVELOPMENT",
            Self::EconomyGovernance => "ECONOMY & GOVERNANCE",
            Self::EnergyEnvironment => "ENERGY & ENVIRONMENT",
            Self::Infrastructure => "INFRASTRUCTURE",
            Self::Interplanetary => "INTERPLANETARY",
            Self::Scenarios => "PLANNING SCENARIOS (fictional)",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::Energy,
            Self::Energy => Self::Climate,
            Self::Climate => Self::Civilization,
            Self::Civilization => Self::HumanDevelopment,
            Self::HumanDevelopment => Self::EconomyGovernance,
            Self::EconomyGovernance => Self::EnergyEnvironment,
            Self::EnergyEnvironment => Self::Infrastructure,
            Self::Infrastructure => Self::Interplanetary,
            Self::Interplanetary => Self::Scenarios,
            Self::Scenarios => Self::Off,
        }
    }

    /// Which layers are visible in this view. `All` deliberately means "all
    /// real/curated data", not literally `Layer::all()` — scenario-kind
    /// layers (Maglev, Resontia Vaults, Terra Lumina, Geothermal, Robotics,
    /// DeSci Evidence Mesh) only ever appear under `Scenarios`, so nothing
    /// fictional silently shows up in a "show me everything" first
    /// impression. See `Layer::provenance()` for the real/curated/scenario
    /// classification this split is drawn from.
    pub fn visible_layers(&self) -> Vec<Layer> {
        match self {
            Self::Off => Vec::new(),
            Self::All => Layer::all()
                .into_iter()
                .filter(|l| l.provenance().kind != sol_atlas_core::types::DataKind::Scenario)
                .collect(),
            Self::Energy => vec![Layer::Energy, Layer::FossilDeposits, Layer::Nuclear],
            Self::Climate => vec![
                Layer::Earthquakes,
                Layer::Fires,
                Layer::Storms,
                Layer::Volcanoes,
                Layer::Climate,
            ],
            Self::Civilization => vec![Layer::Regions, Layer::Health, Layer::Emergency],
            // 15 real per-country World Bank/governance layers (added
            // 2026-07-26, previously only in sol-atlas-leptos), split into
            // 3 thematic sub-groups the same day after the first version
            // dumped all 15 into Civilization at once and looked visually
            // dense/undifferentiated in a live screenshot.
            Self::HumanDevelopment => vec![
                Layer::Population,
                Layer::Literacy,
                Layer::LifeExpectancy,
                Layer::Urbanization,
                Layer::FoodSecurity,
            ],
            Self::EconomyGovernance => vec![
                Layer::GdpPerCapita,
                Layer::Unemployment,
                Layer::Sovereignty,
                Layer::InternetAccess,
            ],
            Self::EnergyEnvironment => vec![
                Layer::ElectricityAccess,
                Layer::CarbonEmissions,
                Layer::RenewableEnergy,
                Layer::CleanCooking,
                Layer::ForestCover,
                Layer::WaterStress,
            ],
            Self::Infrastructure => vec![
                Layer::SupplyChain,
                Layer::Infrastructure,
                Layer::Chokepoints,
                Layer::RailNetwork,
            ],
            Self::Interplanetary => vec![Layer::Orbital], // planets + colonies + Earth-orbit objects
            Self::Scenarios => vec![
                Layer::Maglev,
                Layer::ResontiaVaults,
                Layer::TerraLumina,
                Layer::Geothermal,
                Layer::Robotics,
                Layer::DeSciEvidence,
            ],
        }
    }
}

/// HUD element showing current data view.
#[derive(Component)]
pub struct ViewHud;

/// Overlay manager — toggles HUD elements on/off.
#[derive(Resource)]
pub struct OverlayManager {
    pub show_controls: bool,
    pub show_metrics: bool,
    pub show_labels: bool, // SOL ATLAS + year + view labels
    pub show_timeline_bar: bool,
    pub show_grid: bool,   // gravity well grid
    pub show_orbits: bool, // orbital rings around planets
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self {
            show_controls: true,
            show_metrics: true,
            show_labels: true,
            show_timeline_bar: true,
            // Off by default, matching DataView — the gravity-well grid
            // funnel and orbit rings are exactly the "circles at the
            // bottom" clutter, now correctly wired (draw_gravity_grid_system)
            // but still needed an actual default change to stop showing.
            show_grid: false,
            show_orbits: false,
        }
    }
}

/// Toggle overlays with H key (cycle: all → labels only → minimal → all).
pub fn overlay_toggle_system(
    kb: Res<ButtonInput<KeyCode>>,
    mut overlays: ResMut<OverlayManager>,
    mut panels: Query<&mut Visibility, With<SidePanel>>,
    mut scrubbers: Query<&mut Visibility, (With<TimelineScrubber>, Without<SidePanel>)>,
    mut grids: Query<
        &mut Visibility,
        (
            With<GridLayer>,
            Without<SidePanel>,
            Without<TimelineScrubber>,
        ),
    >,
) {
    if kb.just_pressed(KeyCode::KeyH) {
        if overlays.show_controls && overlays.show_metrics {
            // All visible → labels only (hide panel + scrubber)
            overlays.show_controls = false;
            overlays.show_metrics = false;
            overlays.show_timeline_bar = false;
            info!("[overlay] Minimal HUD");
        } else if !overlays.show_controls {
            // Labels only → everything hidden
            overlays.show_labels = false;
            overlays.show_grid = false;
            overlays.show_orbits = false;
            info!("[overlay] HUD off");
        } else {
            // Hidden → restore all
            overlays.show_controls = true;
            overlays.show_metrics = true;
            overlays.show_labels = true;
            overlays.show_timeline_bar = true;
            overlays.show_grid = true;
            overlays.show_orbits = true;
            info!("[overlay] Full HUD");
        }

        // Apply visibility
        let panel_vis = if overlays.show_controls || overlays.show_metrics {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        for mut vis in panels.iter_mut() {
            *vis = panel_vis;
        }

        let scrub_vis = if overlays.show_timeline_bar {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        for mut vis in scrubbers.iter_mut() {
            *vis = scrub_vis;
        }

        let grid_vis = if overlays.show_grid {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        for mut vis in grids.iter_mut() {
            *vis = grid_vis;
        }
    }
}

/// Side panel UI marker.
#[derive(Component)]
pub struct SidePanel;

/// Side panel metrics text.
#[derive(Component)]
pub struct PanelMetrics;

/// Marker pulse — makes data markers breathe with sinusoidal scale modulation.
#[derive(Component)]
pub struct MarkerPulse {
    pub speed: f32,
    pub amplitude: f32,
    pub phase: f32,
    pub base_scale: f32,
}

/// Current aesthetic preset — cycle with number keys 1-5.
#[derive(Resource)]
pub struct CurrentAesthetic {
    pub aesthetic: sol_atlas_core::aesthetics::Aesthetic,
    pub changed: bool,
}

impl Default for CurrentAesthetic {
    fn default() -> Self {
        Self {
            aesthetic: sol_atlas_core::aesthetics::Aesthetic::Holographic,
            changed: false,
        }
    }
}

/// Holds loaded data for arc rendering each frame (gizmos are immediate-mode).
#[derive(Resource)]
pub struct AtlasData {
    pub data: sol_atlas_core::types::LoadedData,
}

/// Toggle to globe view when M is pressed during gameplay.
pub fn atlas_toggle_system(kb: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<GamePhase>>) {
    if kb.just_pressed(KeyCode::KeyM) {
        next.set(GamePhase::GlobeView);
    }
}

/// Watches for `CellZoomTransition::arrived_at_floor` (set by
/// `sol_atlas_bevy::cell_entry` — a separate crate that can't reference
/// `GamePhase` itself) and transitions into the walkable view. Only fires
/// when you've drilled all the way in, never at intermediate zoom steps —
/// "only walkable if you're already in the hex, otherwise just an overhead
/// view."
pub fn watch_for_cell_arrival_system(
    mut transition: ResMut<sol_atlas_bevy::cell_entry::CellZoomTransition>,
    mut next: ResMut<NextState<GamePhase>>,
) {
    if transition.arrived_at_floor.take().is_some() {
        next.set(GamePhase::CellWalk);
    }
}

/// Escape returns from the walkable cell view to the orbital globe view.
pub fn cell_walk_escape_system(
    kb: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<GamePhase>>,
) {
    if kb.just_pressed(KeyCode::Escape) {
        next.set(GamePhase::GlobeView);
    }
}

/// Set up the globe view — spawn globe, camera, lights, stars, and data markers.
pub fn setup_globe_view(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut holo_materials: ResMut<Assets<sol_atlas_bevy::holographic_material::HolographicMaterial>>,
    mut cloud_materials: ResMut<Assets<sol_atlas_bevy::clouds::CloudMaterial>>,
    asset_server: Res<AssetServer>,
    dungeon_cameras: Query<Entity, (With<Camera2d>, Without<OrbitalCamera>)>,
    hud_texts: Query<Entity, With<crate::systems::rendering::HudText>>,
) {
    // Hide the 2D dungeon camera
    // Keep Camera2d visible — Bevy UI needs it for text/panel rendering.
    // Only hide the dungeon HUD text (not the globe HUD).
    for entity in hud_texts.iter() {
        commands.entity(entity).insert(Visibility::Hidden);
    }

    // Pure black space background
    // Deep space — subtle blue-purple glow (scattered starlight)
    commands.insert_resource(ClearColor(Color::linear_rgb(0.012, 0.008, 0.025)));

    // Ambient light so the dark side of the globe isn't pitch black
    commands.spawn((
        AmbientLight {
            color: Color::linear_rgb(0.15, 0.18, 0.25),
            brightness: 300.0,
            ..default()
        },
        AtlasEntity,
    ));

    // ═══ HOLOGRAPHIC GLOBE ═══════════════════════════════════════

    // [1] Globe with custom holographic shader — Fresnel + scanlines
    // Night lights as emissive — cities glow through any transparency level
    let earth_mesh = meshes.add(Sphere::new(1.0).mesh().uv(64, 64));
    let earth_texture: Handle<Image> = asset_server.load(sol_atlas_bevy::globe::EARTH_TEXTURE_PATH);
    let night_texture: Handle<Image> = asset_server.load("textures/earth-night-8k.jpg");
    let holo_globe =
        holo_materials.add(sol_atlas_bevy::holographic_material::HolographicMaterial {
            base: StandardMaterial {
                // Brightened from (0.15, 0.22, 0.28, 0.5) — combined with
                // the old fresnel-gated shader alpha, that near-black tint
                // crushed the actual continent/ocean texture to
                // near-invisible everywhere except the grazing silhouette
                // edge. Still teal-tinted, but the real texture now reads.
                base_color: Color::linear_rgba(0.55, 0.68, 0.72, 0.9),
                base_color_texture: Some(earth_texture.clone()),
                emissive: LinearRgba::new(4.0, 3.0, 1.5, 1.0), // strong glow — city lights visible through hologram
                emissive_texture: Some(night_texture), // city lights glow through holographic transparency
                alpha_mode: AlphaMode::Blend,
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            extension: sol_atlas_bevy::holographic_material::HolographicExtension {
                fresnel_color: LinearRgba::new(0.0, 0.87, 1.0, 1.0),
                fresnel_power: 3.0,
                scanline_speed: 0.5,
                scanline_density: 20.0,
                hologram_alpha: 0.9,
                // Bright glowing coastline outline — legible landmass
                // boundaries independent of the tinted base texture color.
                outline_color: LinearRgba::new(0.3, 1.0, 0.85, 1.0),
                outline_intensity: 1.4,
                outline_threshold: 0.12,
                surface_texture: earth_texture,
                ..default()
            },
        });
    // Captured so every lat/lon-anchored surface marker below can be spawned
    // as a child of the globe (2026-07-26 fix -- see the ChildOf comment
    // further down for why this matters).
    let globe_entity = commands
        .spawn((
            Mesh3d(earth_mesh),
            MeshMaterial3d(holo_globe),
            Transform::IDENTITY,
            Globe,
            AtlasEntity,
        ))
        .id();

    // [1.5] Cloud layer — dual-layer parallax scroll, contrast-enhanced
    // coverage, sun-facing silver-lining rim glow (see clouds.wgsl).
    let clouds_mesh = meshes.add(Sphere::new(1.0).mesh().uv(64, 64));
    let clouds_texture: Handle<Image> = asset_server.load("textures/earth-clouds.jpg");
    let clouds_material = cloud_materials.add(sol_atlas_bevy::clouds::CloudMaterial {
        base: StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            unlit: false,
            // Fully matte, zero specular reflectance — real clouds are a
            // diffuse scattering medium, not shiny. Likely cause of the
            // rapid "flashing": a sharp moving specular highlight sweeping
            // across the cloud alpha-cutout pattern as the orbital camera
            // slowly auto-rotates/drifts, only visible now that real GPU
            // PBR lighting is actually running.
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        },
        extension: sol_atlas_bevy::clouds::CloudExtension {
            settings: sol_atlas_bevy::clouds::CloudSettings::default(),
            cloud_texture: clouds_texture,
        },
    });
    commands.spawn((
        Mesh3d(clouds_mesh),
        MeshMaterial3d(clouds_material),
        Transform::from_scale(Vec3::splat(1.012)), // 1.2% larger — sits above surface
        CloudLayer,
        AtlasEntity,
    ));

    // [2] Sacred geometry wireframe grid — the hologram's skeleton
    // Inner sphere at 0.97 radius — far enough inside to avoid z-fighting
    let grid_mesh = meshes.add(Sphere::new(0.97).mesh().uv(24, 24)); // low-poly = visible edges
    let grid_material = materials.add(StandardMaterial {
        base_color: Color::linear_rgba(0.0, 0.6, 0.8, 0.02), // barely visible — data is the hero
        emissive: LinearRgba::new(0.0, 0.08, 0.1, 1.0),      // very dim grid
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(grid_mesh),
        MeshMaterial3d(grid_material),
        Transform::IDENTITY,
        GridLayer,
        AtlasEntity,
    ));

    // [5] Fresnel edge glow — outer atmosphere, brighter at grazing angles
    let fresnel_mesh = meshes.add(Sphere::new(1.03).mesh().uv(48, 48));
    let fresnel_material = materials.add(StandardMaterial {
        base_color: Color::linear_rgba(0.0, 0.6, 0.8, 0.04),
        emissive: LinearRgba::new(0.0, 0.5, 0.7, 1.0), // 2x brighter — visible glow ring
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(fresnel_mesh.clone()),
        MeshMaterial3d(fresnel_material),
        Transform::IDENTITY,
        Atmosphere,
        AtlasEntity,
    ));

    // Second Fresnel layer — wider, softer
    let fresnel2_material = materials.add(StandardMaterial {
        base_color: Color::linear_rgba(0.0, 0.87, 1.0, 0.03),
        emissive: LinearRgba::new(0.0, 0.8, 1.1, 1.0), // 2x brighter
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.05).mesh().uv(32, 32))),
        MeshMaterial3d(fresnel2_material),
        Transform::IDENTITY,
        Atmosphere,
        AtlasEntity,
    ));

    // [7] Holographic projection base — subtle dark glass disc
    let base_mesh = meshes.add(Sphere::new(1.2).mesh().uv(32, 4));
    let base_material = materials.add(StandardMaterial {
        base_color: Color::linear_rgba(0.01, 0.02, 0.03, 0.3),
        emissive: LinearRgba::new(0.0, 0.01, 0.015, 1.0),
        perceptual_roughness: 0.95, // matte — won't reflect sun bloom
        metallic: 0.0,              // non-metallic — no mirror reflections
        unlit: true,                // bypass PBR entirely — just a dark platform
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(base_mesh),
        MeshMaterial3d(base_material),
        Transform::from_scale(Vec3::new(1.0, 0.005, 1.0)) // thinner disc
            .with_translation(Vec3::new(0.0, -1.15, 0.0)),
        AtlasEntity,
    ));

    // Sun light — moderate to avoid bloom oversaturation
    commands.spawn((
        DirectionalLight {
            illuminance: 5_000.0,
            color: Color::linear_rgb(1.0, 0.98, 0.95),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.4, 0.6, 0.0)),
        AtlasEntity,
    ));

    // 3D orbital camera — holographic post-processing
    commands.spawn((
        Camera3d::default(),
        bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
        // [6] Bloom — makes emissive markers glow through the hologram
        bevy::post_process::bloom::Bloom {
            intensity: 0.05, // minimal — only extreme HDR (Sun) blooms
            ..default()
        },
        // [3] Chromatic aberration — holographic projection artifact
        bevy::post_process::effect_stack::ChromaticAberration {
            intensity: 0.002, // minimal — preserves text clarity
            max_samples: 8,
            ..default()
        },
        // [7] Depth of field — cinematic lens, background blurs
        bevy::post_process::dof::DepthOfField {
            mode: bevy::post_process::dof::DepthOfFieldMode::Bokeh,
            focal_distance: 4.2,    // focus on globe surface
            sensor_height: 0.01866, // Super 35 cinema format
            max_circle_of_confusion_diameter: 40.0,
            max_depth: 50.0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 4.2).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitalCamera,
        AtlasEntity,
        // 2026-07-26 fix: without an explicit UI-camera marker, Bevy falls
        // back to "highest order camera" for resolving which camera hosts
        // Text/Node UI -- and a stray misconfigured `Camera`-only entity
        // elsewhere (logged: "Entity ... has a Camera component, but it
        // doesn't have a render graph configured", pre-existing, unrelated
        // to this session's other work) was apparently winning that
        // resolution. Confirmed via live screenshots: every UI text
        // element in the globe view (title, view label, selection info)
        // rendered nothing at all until this camera was marked explicitly.
        IsDefaultUiCamera,
    ));

    // ─── Globe Label HUD ─────────────────────────────────────────
    commands.spawn((
        Text::new("SOL ATLAS"),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgba(0.4, 0.7, 0.8, 0.65)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(24.0),
            left: Val::Px(28.0),
            ..default()
        },
        AtlasEntity,
    ));

    // Data view indicator HUD (top-right)
    commands.spawn((
        Text::new("ALL DATA"),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgba(0.5, 0.7, 0.8, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(24.0),
            right: Val::Px(28.0),
            ..default()
        },
        ViewHud,
        AtlasEntity,
    ));

    // ─── Data View Picker (top-right, below the label) ───────────────
    // 2026-07-27, Bevy-side parity with sol-atlas-leptos's lens picker:
    // DataView already plays that exact role (curated named layer-set
    // shortcuts), it just had no click UI before this, only Tab-cycling.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(50.0),
                right: Val::Px(28.0),
                width: Val::Px(190.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            AtlasEntity,
        ))
        .with_children(|parent| {
            for view in DataView::ALL {
                parent
                    .spawn((
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                        DataViewButton(view),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(view.label()),
                            TextFont {
                                font_size: FontSize::Px(9.0),
                                ..default()
                            },
                            TextColor(Color::srgba(0.5, 0.55, 0.6, 0.55)),
                        ));
                    });
            }
        });

    // Timeline year + Turchin cycle phase HUD
    commands.spawn((
        Text::new("Year 0 | Growth"),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgba(0.4, 0.7, 0.5, 0.5)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(46.0),
            left: Val::Px(28.0),
            ..default()
        },
        TimelineHud,
        AtlasEntity,
    ));

    // ─── Starfield ───────────────────────────────────────────────
    let star_data = sol_atlas_core::geometry::generate_starfield(300, 40.0);
    let star_mesh = meshes.add(Sphere::new(1.0).mesh().uv(4, 4));
    // 7 floats per star: pos.xyz, color.rgb, brightness
    for chunk in star_data.chunks_exact(7) {
        let brightness = chunk[6];
        // Only spawn the brighter stars as entities (top ~30%)
        if brightness < 0.55 {
            // only brightest stars — prevents edge strays
            continue;
        }
        let size = 0.06 + brightness * 0.12; // smaller — stars shouldn't be diamonds
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(
                chunk[3] * brightness * 1.5,
                chunk[4] * brightness * 1.5,
                chunk[5] * brightness * 1.5,
            ),
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(chunk[0], chunk[1], chunk[2]).with_scale(Vec3::splat(size)),
            AtlasEntity,
        ));
    }

    // ─── Data markers ────────────────────────────────────────────
    let data = sol_atlas_bevy::data::load_all();
    let marker_mesh = meshes.add(Sphere::new(1.0).mesh().uv(6, 6));
    let mut marker_count = 0usize;

    // [6] Energy sites — deep cybernetic palette, emissive glow
    for site in &data.sites {
        let pos = geo::lat_lon_to_xyz(site.lat, site.lon, 1.04);
        let size = geo::marker_size_from_capacity(site.capacity_mw);
        let c = site.energy_type.rgb();
        // Desaturate and deepen colors to match holographic aesthetic
        let depth = 0.6; // pull colors toward deeper tones
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(c[0] * depth, c[1] * depth, c[2] * depth),
            emissive: LinearRgba::new(c[0] * 0.25, c[1] * 0.25, c[2] * 0.25, 1.0),
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(marker_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            DataMarker {
                layer: Layer::Energy,
                name: site.name.clone(),
            },
            TemporalW { year: 2010.0 },
            MarkerPulse {
                speed: 1.0,
                amplitude: 0.15,
                phase: site.lat as f32 * 0.1,
                base_scale: size,
            },
            SurfaceLod,
            TimelineLayer::Renewable,
            AtlasEntity,
            // Anchors this marker to Earth's real lat/lon so it rotates and
            // tilts together with the globe mesh itself (2026-07-26 fix --
            // before this, every marker layer sat at a fixed world position
            // computed once at spawn time, so once cloud_rotation_system
            // started genuinely spinning/tilting the globe, every marker
            // stayed frozen in place while the surface rotated underneath
            // it -- the actual reason the tilt/rotation looked "wrong",
            // since markers are the most visually prominent content).
            ChildOf(globe_entity),
        ));
        marker_count += 1;
    }

    // Heat blob clustering for ALL marker types (visible when zoomed out)
    {
        let mut all_markers: Vec<(f64, f64, f64, [f32; 3])> = Vec::new();
        // Energy
        for s in &data.sites {
            let c = s.energy_type.rgb();
            all_markers.push((
                s.lat,
                s.lon,
                s.capacity_mw,
                [c[0] * 0.5, c[1] * 0.5, c[2] * 0.5],
            ));
        }
        // Geothermal
        let gc = Layer::Geothermal.rgb();
        for n in &data.geothermal_nodes {
            all_markers.push((
                n.lat,
                n.lon,
                n.capacity_mw,
                [gc[0] * 0.5, gc[1] * 0.5, gc[2] * 0.5],
            ));
        }
        // Fossil
        for d in &data.fossil_deposits {
            let eroi = sol_atlas_core::economics::compute_eroi(d).unwrap_or(5.0);
            let c = sol_atlas_core::economics::eroi_color(eroi);
            all_markers.push((
                d.lat,
                d.lon,
                d.proven_reserves_mboe * 0.01,
                [c[0] * 0.5, c[1] * 0.5, c[2] * 0.5],
            ));
        }
        // Nuclear
        let nc = Layer::Nuclear.rgb();
        for s in &data.nuclear_sites {
            all_markers.push((
                s.lat,
                s.lon,
                s.capacity_mw,
                [nc[0] * 0.5, nc[1] * 0.5, nc[2] * 0.5],
            ));
        }

        let clusters = sol_atlas_core::lod::cluster_markers(&all_markers, 4, 8); // coarser = fewer blobs
        // Heat blobs disabled — city indicators + event markers provide better data.
        let blob_mesh = meshes.add(Sphere::new(1.0).mesh().uv(10, 10));
        let _spawn_blobs = false;
        for cell in &clusters {
            if !_spawn_blobs {
                continue;
            }
            let pos = geo::lat_lon_to_xyz(cell.center_lat, cell.center_lon, 1.04);
            let size = sol_atlas_core::lod::heat_blob_size(cell.count);
            let c = cell.avg_color;
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgba(c[0] * 0.1, c[1] * 0.1, c[2] * 0.1, 0.02),
                emissive: LinearRgba::new(0.0, 0.0, 0.0, 0.0), // no emissive — prevents bloom
                // Ghost blobs — barely visible hint of data density
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            });
            commands.spawn((
                Mesh3d(blob_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
                OrbitLod,
                AtlasEntity,
                ChildOf(globe_entity),
            ));
        }
    }

    // Geothermal nodes — red
    let gc = Layer::Geothermal.rgb();
    for node in &data.geothermal_nodes {
        let pos = geo::lat_lon_to_xyz(node.lat, node.lon, 1.04);
        let size = geo::marker_size_from_capacity(node.capacity_mw);
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(gc[0], gc[1], gc[2]),
            emissive: LinearRgba::new(gc[0] * 0.4, gc[1] * 0.4, gc[2] * 0.4, 1.0),
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(marker_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            DataMarker {
                layer: Layer::Geothermal,
                name: node.name.clone(),
            },
            SurfaceLod,
            TimelineLayer::Renewable,
            AtlasEntity,
            ChildOf(globe_entity),
        ));
        marker_count += 1;
    }

    // Terra Lumina sites — purple, larger (flagship projects)
    let tc = Layer::TerraLumina.rgb();
    for site in &data.terra_lumina_sites {
        let pos = geo::lat_lon_to_xyz(site.lat, site.lon, 1.04);
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(tc[0] * 1.3, tc[1] * 1.3, tc[2] * 1.3),
            emissive: LinearRgba::new(tc[0] * 0.5, tc[1] * 0.5, tc[2] * 0.5, 1.0),
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(marker_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(0.014)),
            DataMarker {
                layer: Layer::TerraLumina,
                name: site.name.clone(),
            },
            SurfaceLod,
            TimelineLayer::Renewable,
            AtlasEntity,
            ChildOf(globe_entity),
        ));
        marker_count += 1;
    }

    // Resontia vaults — emerald
    let vc = Layer::ResontiaVaults.rgb();
    for (i, vault) in data.resontia_vaults.iter().enumerate() {
        let pos = geo::lat_lon_to_xyz(vault.lat, vault.lon, 1.04);
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(vc[0], vc[1], vc[2]),
            emissive: LinearRgba::new(vc[0] * 0.3, vc[1] * 0.3, vc[2] * 0.3, 1.0),
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(marker_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(0.010)),
            DataMarker {
                layer: Layer::ResontiaVaults,
                name: vault.name.clone(),
            },
            SurfaceLod,
            TimelineLayer::Vault(i),
            AtlasEntity,
            ChildOf(globe_entity),
        ));
        marker_count += 1;
    }

    // Fossil deposits — EROI-colored (green→amber→red), with carbon emission halos
    let halo_mesh = meshes.add(Sphere::new(1.0).mesh().uv(12, 12));
    for deposit in &data.fossil_deposits {
        let pos = geo::lat_lon_to_xyz(deposit.lat, deposit.lon, 1.04);
        let eroi = sol_atlas_core::economics::compute_eroi(deposit).unwrap_or(5.0);
        let c = sol_atlas_core::economics::eroi_color(eroi);
        let emissive = geo::fossil_emissive_factor(&deposit.status) * 0.5;
        let scale = geo::fossil_scale_factor(&deposit.status);
        let size = geo::marker_size_from_reserves(deposit.proven_reserves_mboe) * scale;
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(c[0] * emissive, c[1] * emissive, c[2] * emissive),
            unlit: true,
            ..default()
        });
        let timeline_data = FossilTimelineData {
            status: deposit.status.clone(),
            fuel_type: deposit.fuel_type,
            base_eroi: eroi,
        };
        commands.spawn((
            Mesh3d(marker_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            DataMarker {
                layer: Layer::FossilDeposits,
                name: deposit.name.clone(),
            },
            TemporalW {
                year: deposit.discovery_year as f64,
            },
            SurfaceLod,
            TimelineLayer::Fossil,
            AtlasEntity,
            ChildOf(globe_entity),
            timeline_data.clone(),
            FossilMainMarker,
        ));

        // Carbon emission halo — translucent red, sized by CO2 output
        if deposit.annual_production_mboe > 0.0 {
            let halo_radius =
                geo::emission_halo_radius(deposit.annual_production_mboe, &deposit.fuel_type);
            let halo_mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgba(1.0, 0.15, 0.05, 0.10),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            });
            commands.spawn((
                Mesh3d(halo_mesh.clone()),
                MeshMaterial3d(halo_mat),
                Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(halo_radius)),
                DataMarker {
                    layer: Layer::FossilDeposits,
                    name: format!("{} CO2 halo", deposit.name),
                },
                SurfaceLod,
                TimelineLayer::Fossil,
                AtlasEntity,
                ChildOf(globe_entity),
                timeline_data,
            ));
        }

        marker_count += 1;
    }

    // Nuclear sites — violet, SMR planned sites brighter
    let nc = Layer::Nuclear.rgb();
    for site in &data.nuclear_sites {
        let pos = geo::lat_lon_to_xyz(site.lat, site.lon, 1.04);
        let size = geo::marker_size_from_capacity(site.capacity_mw);
        let brightness = if site.reactor_type.is_smr() { 1.4 } else { 1.0 };
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(
                nc[0] * brightness,
                nc[1] * brightness,
                nc[2] * brightness,
            ),
            emissive: LinearRgba::new(nc[0] * 0.4, nc[1] * 0.4, nc[2] * 0.4, 1.0),
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(marker_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            DataMarker {
                layer: Layer::Nuclear,
                name: site.name.clone(),
            },
            TemporalW {
                year: site.commission_year as f64,
            },
            SurfaceLod,
            TimelineLayer::Nuclear,
            AtlasEntity,
            ChildOf(globe_entity),
        ));
        marker_count += 1;
    }

    // Grid stress markers (FEP allostatic load visualization)
    // ─── City Grid Stress Indicators (ALWAYS VISIBLE) ─────────────
    // 16 major cities showing civilization's energy heartbeat at orbit zoom.
    let city_mesh = meshes.add(Sphere::new(1.0).mesh().uv(12, 12));
    let stress_data = sol_atlas_core::energy_trading::simulate_grid_stress(0);
    for stress in &stress_data {
        let pos = geo::lat_lon_to_xyz(stress.lat, stress.lon, 1.04);
        // City stress: cyan→yellow (cool palette, never red/orange)
        let load = stress.allostatic_load;
        let c = if load < 0.3 {
            [0.1, 0.6, 0.7] // teal — stable
        } else if load < 0.6 {
            [0.5, 0.7, 0.2] // yellow-green — transitioning
        } else {
            [0.8, 0.8, 0.1] // yellow — stressed (NOT red/orange)
        };
        let size = 0.005 + load * 0.004; // small — 0.005 to 0.009

        // Emissive city indicator — visible at ALL zoom levels (no LOD tag)
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgba(c[0], c[1], c[2], 0.8),
            emissive: LinearRgba::new(c[0] * 0.2, c[1] * 0.2, c[2] * 0.2, 1.0), // subtle glow
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        commands.spawn((
            Mesh3d(city_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            CityIndicator {
                name: stress.name.clone(),
                load: stress.allostatic_load,
            },
            MarkerPulse {
                speed: 1.5 + stress.allostatic_load * 2.0, // stressed cities pulse faster
                amplitude: 0.1 + stress.allostatic_load * 0.15,
                phase: stress.lat as f32 * 0.2,
                base_scale: size,
            },
            DataMarker {
                layer: Layer::Energy,
                name: format!(
                    "{} (load={:.0}%)",
                    stress.name,
                    stress.allostatic_load * 100.0
                ),
            },
            // NO SurfaceLod — always visible at orbit!
            AtlasEntity,
            ChildOf(globe_entity),
        ));
    }

    // ─── Earth Region Indicators (ALWAYS VISIBLE) ────────────────
    // 12 UN-calibrated regions showing vulnerability at a glance.
    let region_mesh = meshes.add(Sphere::new(1.0).mesh().uv(8, 8));
    for region in &data.earth_regions {
        let pos = geo::lat_lon_to_xyz(region.lat, region.lon, 1.04);
        // Color by vulnerability: blue (resilient) → purple (vulnerable)
        // NOT red — red is reserved for emergencies/earthquakes
        let v = region.climate_vulnerability as f32;
        let c = [
            0.2 + v * 0.4,   // R: slight increase
            0.3 * (1.0 - v), // G: decreases
            0.6 + v * 0.3,   // B: stays high (blue→purple)
        ];
        let size = 0.006 + (region.population_m as f32 / 3000.0).min(1.0) * 0.006;
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgba(c[0], c[1], c[2], 0.4),
            emissive: LinearRgba::new(c[0] * 0.1, c[1] * 0.1, c[2] * 0.1, 1.0), // minimal emissive
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(region_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            DataMarker {
                layer: Layer::Regions,
                name: format!(
                    "{} (pop={}M, vuln={:.0}%)",
                    region.name,
                    region.population_m as u32,
                    v * 100.0
                ),
            },
            // NO LOD tag — always visible at orbit
            AtlasEntity,
            ChildOf(globe_entity),
        ));
    }

    // ─── Major World Cities (232 cities, pop >= 1M) ──────────────
    // Only show top 30 megacities to avoid visual noise.
    let city_dot_mesh = meshes.add(Sphere::new(1.0).mesh().uv(4, 4));
    let mut sorted_cities = data.major_cities.clone();
    sorted_cities.sort_by(|a, b| b.population.cmp(&a.population));
    for city in sorted_cities.iter().take(30) {
        let pos = geo::lat_lon_to_xyz(city.lat, city.lon, 1.04);
        let pop_scale = (city.population as f32 / 20_000_000.0).clamp(0.0, 1.0);
        let size = 0.002 + pop_scale * 0.003; // pinpoint dots
        let alpha = 0.3 + pop_scale * 0.4;
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.8, 0.85, 0.6), // opaque warm-white dot
            emissive: LinearRgba::new(0.15, 0.15, 0.08, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        commands.spawn((
            Mesh3d(city_dot_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            DataMarker {
                layer: Layer::Regions,
                name: format!("{} ({}M)", city.name, city.population / 1_000_000),
            },
            AtlasEntity,
            ChildOf(globe_entity),
        ));
    }
    info!(
        "[atlas] {} major cities loaded (pop >= 1M)",
        data.major_cities.len()
    );

    // ─── Country Profiles (15 real per-country layers, added 2026-07-26) ──
    // Ports sol-atlas-leptos's globe_canvas.rs sizing/color rules exactly.
    // Loaded once here rather than per-frame (unlike the gizmo-arc layers'
    // load_shipping_lanes()/load_rail_corridors() pattern) since these are
    // real spawned+tagged entities, filtered by the same DataMarker/
    // Visibility system every other marker layer uses.
    let country_profiles = sol_atlas_bevy::data::load_country_profiles();
    let country_marker_mesh = meshes.add(Sphere::new(1.0).mesh().uv(4, 4));

    /// Spawn one flat-colored point marker per country with a real value,
    /// sized by `size_fn`. Shared by every non-brightness country layer.
    macro_rules! spawn_country_layer {
        ($layer:expr, $alt:expr, $field:ident, $size_fn:expr) => {
            let c = $layer.rgb();
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(c[0], c[1], c[2]),
                emissive: LinearRgba::new(c[0] * 0.3, c[1] * 0.3, c[2] * 0.3, 1.0),
                unlit: true,
                ..default()
            });
            for p in country_profiles.iter() {
                let Some(v) = p.$field else { continue };
                let pos = geo::lat_lon_to_xyz(p.lat, p.lon, $alt);
                let size: f32 = $size_fn(v as f32);
                commands.spawn((
                    Mesh3d(country_marker_mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
                    DataMarker {
                        layer: $layer,
                        name: format!("{} ({:.1})", p.name, v),
                    },
                    AtlasEntity,
                    ChildOf(globe_entity),
                ));
            }
        };
    }

    /// Spawn one point marker per country whose brightness (not size)
    /// encodes the value -- for layers where most countries cluster near
    /// the high end (electricity/clean-cooking access) or a boolean signal
    /// (sovereignty), matching Leptos's brightness-dimming treatment.
    macro_rules! spawn_country_layer_brightness {
        ($layer:expr, $alt:expr, $size:expr, $brightness_fn:expr) => {
            let c = $layer.rgb();
            for p in country_profiles.iter() {
                let Some(brightness) = $brightness_fn(p) else {
                    continue;
                };
                let pos = geo::lat_lon_to_xyz(p.lat, p.lon, $alt);
                let mat = materials.add(StandardMaterial {
                    base_color: Color::linear_rgb(
                        c[0] * brightness,
                        c[1] * brightness,
                        c[2] * brightness,
                    ),
                    emissive: LinearRgba::new(
                        c[0] * brightness * 0.3,
                        c[1] * brightness * 0.3,
                        c[2] * brightness * 0.3,
                        1.0,
                    ),
                    unlit: true,
                    ..default()
                });
                commands.spawn((
                    Mesh3d(country_marker_mesh.clone()),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat($size)),
                    DataMarker {
                        layer: $layer,
                        name: p.name.clone(),
                    },
                    AtlasEntity,
                    ChildOf(globe_entity),
                ));
            }
        };
    }

    // Population is a plain u64 (always present, unlike every other field
    // below which is Option<f64>) -- doesn't fit spawn_country_layer!'s
    // `Some(v) = p.$field` pattern, so it gets its own loop.
    {
        let c = Layer::Population.rgb();
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(c[0], c[1], c[2]),
            emissive: LinearRgba::new(c[0] * 0.3, c[1] * 0.3, c[2] * 0.3, 1.0),
            unlit: true,
            ..default()
        });
        for p in country_profiles.iter() {
            let pos = geo::lat_lon_to_xyz(p.lat, p.lon, 1.050);
            let pop = p.population as f32;
            let size = (0.006 + (pop + 1.0).ln() * 0.0016).clamp(0.008, 0.024);
            commands.spawn((
                Mesh3d(country_marker_mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
                DataMarker {
                    layer: Layer::Population,
                    name: format!("{} ({})", p.name, p.population),
                },
                AtlasEntity,
                ChildOf(globe_entity),
            ));
        }
    }
    spawn_country_layer!(
        Layer::FoodSecurity,
        1.051,
        agricultural_land_pct,
        |pct: f32| 0.006 + (pct / 100.0).clamp(0.0, 1.0) * 0.016
    );
    spawn_country_layer!(
        Layer::WaterStress,
        1.052,
        freshwater_withdrawal_pct,
        |pct: f32| 0.006 + (pct / 100.0).clamp(0.0, 1.0) * 0.018
    );
    spawn_country_layer_brightness!(Layer::Sovereignty, 1.053, 0.01, |p: &CountryProfile| Some(
        if p.un_member { 1.0 } else { 0.4 }
    ));
    spawn_country_layer_brightness!(
        Layer::ElectricityAccess,
        1.054,
        0.01,
        |p: &CountryProfile| p
            .electricity_access_pct
            .map(|pct| 0.25 + (pct as f32 / 100.0).clamp(0.0, 1.0) * 0.75)
    );
    spawn_country_layer!(Layer::CarbonEmissions, 1.055, co2_per_capita, |t: f32| {
        0.006 + (t / 20.0).clamp(0.0, 1.0) * 0.018
    });
    spawn_country_layer!(
        Layer::InternetAccess,
        1.056,
        internet_access_pct,
        |pct: f32| 0.006 + (pct / 100.0).clamp(0.0, 1.0) * 0.016
    );
    spawn_country_layer!(Layer::Literacy, 1.057, literacy_pct, |pct: f32| {
        0.006 + (pct / 100.0).clamp(0.0, 1.0) * 0.016
    });
    spawn_country_layer!(
        Layer::LifeExpectancy,
        1.058,
        life_expectancy,
        |years: f32| { 0.006 + ((years - 50.0) / 35.0).clamp(0.0, 1.0) * 0.016 }
    );
    spawn_country_layer!(Layer::GdpPerCapita, 1.059, gdp_per_capita, |gdp: f32| {
        (0.006 + (gdp + 1.0).ln() * 0.0016).clamp(0.008, 0.022)
    });
    spawn_country_layer!(
        Layer::RenewableEnergy,
        1.060,
        renewable_energy_pct,
        |pct: f32| 0.006 + (pct / 100.0).clamp(0.0, 1.0) * 0.018
    );
    spawn_country_layer!(Layer::ForestCover, 1.061, forest_pct, |pct: f32| {
        0.006 + (pct / 100.0).clamp(0.0, 1.0) * 0.018
    });
    spawn_country_layer!(Layer::Urbanization, 1.062, urban_pop_pct, |pct: f32| {
        0.006 + (pct / 100.0).clamp(0.0, 1.0) * 0.016
    });
    spawn_country_layer!(Layer::Unemployment, 1.063, unemployment_pct, |pct: f32| {
        0.006 + (pct / 30.0).clamp(0.0, 1.0) * 0.018
    });
    spawn_country_layer_brightness!(Layer::CleanCooking, 1.064, 0.01, |p: &CountryProfile| p
        .clean_cooking_access_pct
        .map(|pct| 0.25 + (pct as f32 / 100.0).clamp(0.0, 1.0) * 0.75));
    info!(
        "[atlas] {} country profiles loaded, backing 15 real per-country layers",
        country_profiles.len()
    );

    // ─── DeSci Evidence (illustrative Scenario data, added 2026-07-26) ───
    // The static demo dataset sol-atlas-leptos uses -- distinct from the
    // dormant, unregistered desci_evidence.rs::DeSciEvidencePlugin, which
    // targets a live EvidencePacket stream that doesn't exist here. Colors
    // match Leptos's globe_canvas.rs exactly (sapphire/crimson/cyan).
    let desci_evidence = sol_atlas_bevy::data::load_desci_evidence();
    let desci_mesh = meshes.add(Sphere::new(1.0).mesh().uv(6, 6));
    for marker in desci_evidence.iter() {
        let c = match marker.tier {
            sol_atlas_core::types::EvidenceTier::Empirical => [0.0, 0.4, 0.8],
            sol_atlas_core::types::EvidenceTier::Somatic => [0.8, 0.08, 0.24],
            sol_atlas_core::types::EvidenceTier::Network => [0.0, 0.84, 0.78],
        };
        let pos = geo::lat_lon_to_xyz(marker.lat, marker.lon, 1.065);
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(c[0], c[1], c[2]),
            emissive: LinearRgba::new(c[0] * 0.5, c[1] * 0.5, c[2] * 0.5, 1.0),
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(desci_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(0.012)),
            DataMarker {
                layer: Layer::DeSciEvidence,
                name: format!("{} ({:?})", marker.feature_name, marker.tier),
            },
            AtlasEntity,
            ChildOf(globe_entity),
        ));
    }
    info!(
        "[atlas] {} DeSci evidence markers loaded (illustrative demo scenario)",
        desci_evidence.len()
    );

    // ─── Natural Events (ALWAYS VISIBLE) ──────────────────────────
    // Earthquakes, fires, storms, volcanoes from USGS/NASA/NOAA.
    let event_mesh = meshes.add(Sphere::new(1.0).mesh().uv(6, 6));
    for event in &data.natural_events {
        // Filter: only show significant events (M4+ quakes, high-confidence fires)
        // Strict filter — only major events to avoid visual noise
        match event.event_type {
            sol_atlas_core::types::NaturalEventType::Earthquake if event.magnitude < 5.5 => {
                continue;
            }
            sol_atlas_core::types::NaturalEventType::Fire => continue, // fires disabled — too many, too noisy
            sol_atlas_core::types::NaturalEventType::Storm if event.magnitude < 30.0 => continue, // only major storms
            _ => {}
        }
        let pos = geo::lat_lon_to_xyz(event.lat, event.lon, 1.04);
        let (c, size, layer) = match event.event_type {
            sol_atlas_core::types::NaturalEventType::Earthquake => {
                let s = 0.003 + ((event.magnitude as f32 - 4.0) / 5.0).clamp(0.0, 1.0) * 0.004;
                ([0.8, 0.15, 0.1], s, Layer::Earthquakes)
            }
            sol_atlas_core::types::NaturalEventType::Fire => {
                ([0.85, 0.4, 0.1], 0.002, Layer::Fires)
            }
            sol_atlas_core::types::NaturalEventType::Storm => {
                ([0.1, 0.6, 0.8], 0.004, Layer::Storms)
            }
            sol_atlas_core::types::NaturalEventType::Volcano => {
                ([0.8, 0.25, 0.05], 0.004, Layer::Volcanoes)
            }
        };
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgba(c[0], c[1], c[2], 0.5),
            emissive: LinearRgba::new(c[0] * 0.15, c[1] * 0.15, c[2] * 0.15, 1.0), // low emissive — won't bloom
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(event_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            MarkerPulse {
                speed: 1.5,
                amplitude: 0.08, // subtle — events shouldn't dominate visually
                phase: event.lat as f32 * 0.3,
                base_scale: size,
            },
            DataMarker {
                layer,
                name: event.name.clone(),
            },
            // NO LOD tag — always visible
            AtlasEntity,
            ChildOf(globe_entity),
        ));
    }
    info!(
        "[atlas] {} natural events loaded (earthquakes + fires + storms + volcanoes)",
        data.natural_events.len()
    );

    // ─── Maritime Chokepoints (ALWAYS VISIBLE) ──────────────────
    // 8 critical bottlenecks in global trade — diamond-shaped, warning color.
    let choke_mesh = meshes.add(Sphere::new(1.0).mesh().uv(6, 6));
    for choke in &data.chokepoints {
        let pos = geo::lat_lon_to_xyz(choke.lat, choke.lon, 1.04);
        let size = 0.008 + (choke.daily_barrels_m as f32 / 25.0).min(1.0) * 0.008;
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgba(0.8, 0.6, 0.2, 0.5),
            emissive: LinearRgba::new(0.15, 0.1, 0.03, 1.0), // low emissive
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        commands.spawn((
            Mesh3d(choke_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            MarkerPulse {
                speed: 0.8,
                amplitude: 0.2,
                phase: choke.lat as f32,
                base_scale: size,
            },
            DataMarker {
                layer: Layer::Chokepoints,
                name: format!("{} ({}M bbl/day)", choke.name, choke.daily_barrels_m),
            },
            AtlasEntity,
            ChildOf(globe_entity),
        ));
    }

    // ─── Critical Infrastructure (ALWAYS VISIBLE) ───────────────
    // 10 single points of failure — bright warning markers.
    let crit_mesh = meshes.add(Sphere::new(1.0).mesh().uv(8, 8));
    for infra in &data.critical_infrastructure {
        let pos = geo::lat_lon_to_xyz(infra.lat, infra.lon, 1.04);
        let size = 0.006 + (infra.global_share as f32).min(1.0) * 0.010;
        // Color by type: semiconductor=purple, port=blue, oil=amber, other=cyan
        let c = match infra.infra_type.as_str() {
            "semiconductor" => [0.7, 0.3, 0.9],
            "port" => [0.2, 0.5, 0.9],
            "oil_hub" | "gas_pipeline" => [0.9, 0.6, 0.1],
            "submarine_cable" => [0.1, 0.8, 0.7],
            "seed_bank" => [0.2, 0.8, 0.3],
            _ => [0.5, 0.7, 0.8],
        };
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgba(c[0], c[1], c[2], 0.75),
            emissive: LinearRgba::new(c[0] * 0.5, c[1] * 0.5, c[2] * 0.5, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        commands.spawn((
            Mesh3d(crit_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            MarkerPulse {
                speed: 1.2,
                amplitude: 0.15,
                phase: infra.lon as f32 * 0.1,
                base_scale: size,
            },
            DataMarker {
                layer: Layer::Infrastructure,
                name: format!(
                    "{} ({}, risk: {})",
                    infra.name, infra.infra_type, infra.risk
                ),
            },
            AtlasEntity,
            ChildOf(globe_entity),
        ));
    }

    // ─── Orbital Objects (real satellites + debris, SGP4-propagated) ──
    // Point-in-time snapshot from CelesTrak; see
    // orbital-mechanics/examples/sol_atlas_export.rs. Rendered above the
    // surface at a schematic (not true-to-scale) altitude: real LEO-to-MEO
    // altitudes span ~400-20,600km, which at true scale would place GPS
    // markers ~4x the globe's radius away.
    let orbital_mesh = meshes.add(Sphere::new(1.0).mesh().uv(6, 6));
    for obj in &data.orbital_objects {
        let radius = 1.04 + (obj.alt_km / 20_000.0).clamp(0.0, 1.0) * 0.3;
        let pos = geo::lat_lon_to_xyz(obj.lat, obj.lon, radius);
        let (c, size) = match obj.object_type {
            OrbitalObjectType::Payload => ([0.49, 0.83, 0.99], 0.010),
            OrbitalObjectType::RocketBody => ([0.7, 0.7, 0.75], 0.008),
            OrbitalObjectType::Debris => ([0.9, 0.5, 0.3], 0.005),
        };
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgba(c[0], c[1], c[2], 0.8),
            emissive: LinearRgba::new(c[0] * 0.6, c[1] * 0.6, c[2] * 0.6, 1.0),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        commands.spawn((
            Mesh3d(orbital_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            MarkerPulse {
                speed: 1.5,
                amplitude: 0.2,
                phase: obj.lon as f32 * 0.1,
                base_scale: size,
            },
            DataMarker {
                layer: Layer::Orbital,
                name: format!("{} ({:?}, {:.0}km)", obj.name, obj.object_type, obj.alt_km),
            },
            AtlasEntity,
            // Deliberately NOT ChildOf(globe_entity) unlike every other
            // surface marker above: these are real satellites/debris in
            // independent orbits, not features fixed to Earth's rotating
            // surface -- most don't co-rotate with Earth at all (only
            // geostationary ones would). This is already a frozen point-
            // in-time snapshot rather than a live propagation, so leaving
            // it in world space is the more physically honest choice.
        ));
    }
    marker_count += data.orbital_objects.len();

    // ─── Solar System Bodies ────────────────────────────────────
    let bodies = sol_atlas_core::solar_system::solar_system_bodies();
    let body_mesh = meshes.add(Sphere::new(1.0).mesh().uv(32, 32));
    for (body_idx, body) in bodies.iter().enumerate() {
        let pos = sol_atlas_core::solar_system::body_position(body, 0.0);
        let texture: Handle<Image> = asset_server.load(format!("textures/{}", body.texture));
        let mat = if body.is_sun {
            // Spawn corona glow sphere (2x size, additive blend)
            let corona = materials.add(StandardMaterial {
                base_color: Color::linear_rgba(1.0, 0.8, 0.3, 0.12),
                emissive: LinearRgba::new(5.0, 3.0, 0.8, 1.0),
                alpha_mode: AlphaMode::Add,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            });
            commands.spawn((
                Mesh3d(body_mesh.clone()),
                MeshMaterial3d(corona),
                Transform::from_xyz(pos[0], pos[1], pos[2])
                    .with_scale(Vec3::splat(body.visual_radius * 1.4)), // tight corona — no overlap
                CelestialBodyMesh {
                    body_index: body_idx,
                    is_corona: true,
                },
                AtlasEntity,
            ));
            // The sun itself — white-hot, extreme HDR emissive
            materials.add(StandardMaterial {
                base_color: Color::linear_rgba(1.0, 0.95, 0.85, 1.0),
                base_color_texture: Some(texture),
                emissive: LinearRgba::new(20.0, 14.0, 4.0, 1.0),
                unlit: true,
                ..default()
            })
        } else {
            // Real planet textures with subtle emissive for visibility in space
            materials.add(StandardMaterial {
                base_color: Color::linear_rgba(0.9, 0.9, 0.9, 0.85),
                base_color_texture: Some(texture),
                emissive: LinearRgba::new(0.08, 0.08, 0.08, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                ..default()
            })
        };
        commands.spawn((
            Mesh3d(body_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(body.visual_radius)),
            CelestialBodyMesh {
                body_index: body_idx,
                is_corona: false,
            },
            AtlasEntity,
        ));
    }

    // ─── Governance participation markers ──────────────────────
    let gov_pulses = sol_atlas_core::mycelix_flows::simulate_governance_pulses();
    let gov_mesh = meshes.add(Sphere::new(1.0).mesh().uv(8, 8));
    for pulse in &gov_pulses {
        let pos = geo::lat_lon_to_xyz(pulse.lat, pulse.lon, 1.04);
        let c = sol_atlas_core::mycelix_flows::governance_color(pulse.participation);
        let size = 0.012 + pulse.participation * 0.018;
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(c[0], c[1], c[2]),
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(gov_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos[0], pos[1], pos[2]).with_scale(Vec3::splat(size)),
            DataMarker {
                layer: Layer::Regions,
                name: format!(
                    "{} ({}% participation)",
                    pulse.name,
                    (pulse.participation * 100.0) as u32
                ),
            },
            SurfaceLod,
            AtlasEntity,
            ChildOf(globe_entity),
        ));
    }

    // ─── Side Panel UI ───────────────────────────────────────────
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(80.0),
                width: Val::Px(200.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.6)),
            SidePanel,
            AtlasEntity,
        ))
        .with_children(|parent| {
            // Section: Controls
            parent.spawn((
                Text::new("-- Controls --"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.7, 0.8, 0.7)),
                Node {
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
            ));
            for line in [
                "Tab    Data View",
                "1-5    Aesthetic",
                "←/→    Timeline",
                "Scroll Zoom",
                "Drag   Rotate",
                "F1-F6  Planets",
                "F7     Earth",
                "F8     Whole System",
                "F10-12 Mercury/Uranus/Neptune",
                "H      Toggle HUD",
                "Space  Play/Pause",
                "F9     Record",
            ] {
                parent.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgba(0.4, 0.55, 0.5, 0.5)),
                    Node {
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                ));
            }

            // Section: Metrics
            parent.spawn((
                Text::new("-- Metrics --"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.7, 0.8, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(10.0)).with_bottom(Val::Px(6.0)),
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("Loading..."),
                TextFont {
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::srgba(0.4, 0.6, 0.5, 0.6)),
                PanelMetrics,
            ));
        });

    // ─── Timeline Scrubber Bar (bottom of screen) ─────────────
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(6.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.1, 0.6)),
            TimelineScrubber,
            AtlasEntity,
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    width: Val::Percent(0.0), // updated each frame
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.0, 0.7, 0.6)), // teal fill
                TimelineFill,
            ));
        });

    // Store data for arc rendering (gizmos are immediate-mode)
    commands.insert_resource(AtlasData { data });

    info!(
        "[atlas] Globe view: {marker_count} markers + {} stress + {} bodies + {} governance — Esc to return",
        stress_data.len(),
        bodies.len(),
        gov_pulses.len()
    );
}

/// Draw maglev corridor arcs using gizmos (immediate-mode, redrawn each frame).
pub fn draw_arcs_system(
    atlas_data: Option<Res<AtlasData>>,
    mut gizmos: Gizmos,
    time: Res<Time>,
    view: Res<DataView>,
    globe: Query<&Transform, With<Globe>>,
) {
    // Data-flow arcs (trades/TEND/maglev/supply/shipping) were previously
    // drawn via immediate-mode gizmos unconditionally, regardless of
    // DataView — they don't have a Visibility component like marker
    // entities do, so the DataView::Off default didn't hide them. This is
    // the actual fix for "I can still see the overlays".
    if *view == DataView::Off {
        return;
    }

    let Some(atlas_data) = atlas_data else { return };
    let t = time.elapsed_secs();

    // Arcs are recomputed from scratch every frame (immediate-mode gizmos,
    // no persistent entity to attach ChildOf to like the point markers in
    // setup_globe_view got -- 2026-07-26 fix, same bug), so each endpoint
    // needs the globe's current rotation applied by hand here instead.
    // `generate_arc`'s slerp is rotation-equivariant (depends only on the
    // angle between its two inputs), so rotating the raw lat/lon endpoints
    // before generating the arc is equivalent to rotating the whole arc
    // afterward, and much cheaper.
    let globe_rotation = globe.single().map(|t| t.rotation).unwrap_or(Quat::IDENTITY);
    let rotate = |p: [f32; 3]| -> [f32; 3] { (globe_rotation * Vec3::from_array(p)).into() };

    // P2P energy trades — green animated arcs between renewable sites.
    // Gated on Layer::Energy (2026-07-26 fix, same bug class as Maglev/
    // SupplyChain above): trade_sites is literally drawn from
    // atlas_data.data.sites, i.e. Layer::Energy's own real entities, so
    // that's the correct layer to gate on -- this block previously
    // rendered in every non-Off view regardless of whether Energy was
    // even part of the current preset.
    if view.visible_layers().contains(&Layer::Energy) {
        let trade_sites: Vec<(f64, f64, f64)> = atlas_data
            .data
            .sites
            .iter()
            .take(8) // top 8 sites — fewer arcs = less pole convergence
            .map(|s| (s.lat, s.lon, s.capacity_mw))
            .collect();
        let trades = sol_atlas_core::energy_trading::simulate_trades(&trade_sites, t as f64);
        let trade_color = Color::linear_rgba(0.2, 1.0, 0.3, 0.8); // bright green, more opaque
        for trade in &trades {
            let from = rotate(geo::lat_lon_to_xyz(
                trade.seller_lat,
                trade.seller_lon,
                1.04,
            ));
            let to = rotate(geo::lat_lon_to_xyz(trade.buyer_lat, trade.buyer_lon, 1.04));
            let dist = sol_atlas_core::geo::haversine_km(
                trade.seller_lat,
                trade.seller_lon,
                trade.buyer_lat,
                trade.buyer_lon,
            );
            let peak = geo::arc_peak_height(dist);
            let segments = 12u32;
            let arc = sol_atlas_core::geometry::generate_arc(from, to, peak, segments);
            for i in 0..segments as usize {
                let a = Vec3::new(arc[i * 3], arc[i * 3 + 1], arc[i * 3 + 2]);
                let b = Vec3::new(arc[(i + 1) * 3], arc[(i + 1) * 3 + 1], arc[(i + 1) * 3 + 2]);
                gizmos.line(a, b, trade_color);
            }
        }
    }

    // TEND time-banking flows — Mycelix lime arcs with animated packets.
    // Gated on Layer::Regions: TEND is a Mycelix mutual-aid/civic
    // mechanism with no dedicated real dataset of its own (fully
    // simulated via simulate_tend_flows()), so Regions -- the broadest
    // real "civilization" layer, part of the Civilization view -- is the
    // closest thematic fit. Same bug class fix as above.
    if view.visible_layers().contains(&Layer::Regions) {
        let tend_flows = sol_atlas_core::mycelix_flows::simulate_tend_flows();
        for (fi, flow) in tend_flows.iter().enumerate() {
            let from = rotate(geo::lat_lon_to_xyz(flow.from_lat, flow.from_lon, 1.04));
            let to = rotate(geo::lat_lon_to_xyz(flow.to_lat, flow.to_lon, 1.04));
            let dist = sol_atlas_core::geo::haversine_km(
                flow.from_lat,
                flow.from_lon,
                flow.to_lat,
                flow.to_lon,
            );
            let peak = geo::arc_peak_height(dist) * 1.5;
            let segments = 16u32;
            let arc = sol_atlas_core::geometry::generate_arc(from, to, peak, segments);
            let packet_pos = ((t * 0.25 + fi as f32 * 0.5) % 1.0).abs();
            let packet_seg = (packet_pos * segments as f32) as usize;
            for i in 0..segments as usize {
                let a = Vec3::new(arc[i * 3], arc[i * 3 + 1], arc[i * 3 + 2]);
                let b = Vec3::new(arc[(i + 1) * 3], arc[(i + 1) * 3 + 1], arc[(i + 1) * 3 + 2]);
                let dist_to_packet = (i as f32 - packet_seg as f32).abs() / segments as f32;
                let brightness = 0.3 + 0.7 * (-dist_to_packet * 6.0).exp();
                gizmos.line(
                    a,
                    b,
                    Color::linear_rgba(
                        0.486 * brightness,
                        0.988 * brightness,
                        0.0,
                        brightness * 0.7,
                    ),
                );
            }
        }
    }

    // Maglev corridors — amber arcs with animated data packets. Gated on
    // Layer::Maglev's actual visibility (2026-07-26 fix): this whole
    // function previously only checked `view != Off`, so fictional Maglev
    // corridors rendered in EVERY view including "ALL DATA" and
    // "INFRASTRUCTURE" regardless of the `visible_layers()` split above —
    // that split only ever gated the marker/Visibility-component system,
    // never this separate gizmo-based arc system. Maglev now only draws
    // under the explicit Scenarios view, matching its fictional provenance.
    if view.visible_layers().contains(&Layer::Maglev) {
        for (ci, corridor) in atlas_data.data.maglev_corridors.iter().enumerate() {
            let from = rotate(geo::lat_lon_to_xyz(
                corridor.from_lat,
                corridor.from_lon,
                1.04,
            ));
            let to = rotate(geo::lat_lon_to_xyz(corridor.to_lat, corridor.to_lon, 1.04));
            let peak = geo::arc_peak_height(corridor.distance_km);
            let segments = 24u32;
            let arc = sol_atlas_core::geometry::generate_arc(from, to, peak, segments);

            // Data packet position (0.0-1.0) travels along the arc
            let packet_pos = ((t * 0.3 + ci as f32 * 0.4) % 1.0).abs();
            let packet_seg = (packet_pos * segments as f32) as usize;

            for i in 0..segments as usize {
                let a = Vec3::new(arc[i * 3], arc[i * 3 + 1], arc[i * 3 + 2]);
                let b = Vec3::new(arc[(i + 1) * 3], arc[(i + 1) * 3 + 1], arc[(i + 1) * 3 + 2]);
                let dist_to_packet = (i as f32 - packet_seg as f32).abs() / segments as f32;
                let brightness = 0.4 + 0.6 * (-dist_to_packet * 8.0).exp();
                let color = Color::linear_rgba(
                    1.0 * brightness,
                    0.8 * brightness,
                    0.1 * brightness,
                    brightness,
                );
                let glow = Color::linear_rgba(
                    1.0 * brightness * 0.3,
                    0.8 * brightness * 0.3,
                    0.1 * brightness * 0.3,
                    brightness * 0.3,
                );
                // Center line + 2 glow flanks
                gizmos.line(a, b, color);
                let normal = (b - a).cross(Vec3::Y).normalize_or_zero() * 0.004;
                gizmos.line(a + normal, b + normal, glow);
                gizmos.line(a - normal, b - normal, glow);

                // Data pulse sphere at packet position
                if dist_to_packet < 0.05 {
                    let pulse_pos = a.lerp(b, 0.5);
                    gizmos.sphere(
                        Isometry3d::from_translation(pulse_pos),
                        0.008,
                        Color::linear_rgba(1.0, 0.95, 0.7, 0.9),
                    );
                }
            }
        }
    }

    // Real high-speed rail corridors — sky-blue arcs, no animated packet
    // (that's Maglev's "futuristic tech" visual signature; rail is just
    // real infrastructure). Gated on Layer::RailNetwork like every other
    // per-layer marker, so it respects the current view.
    if view.visible_layers().contains(&Layer::RailNetwork) {
        let rail_color = Color::linear_rgba(0.220, 0.741, 0.973, 0.85);
        for corridor in sol_atlas_bevy::data::load_rail_corridors().iter() {
            let from = rotate(geo::lat_lon_to_xyz(
                corridor.from_lat,
                corridor.from_lon,
                1.04,
            ));
            let to = rotate(geo::lat_lon_to_xyz(corridor.to_lat, corridor.to_lon, 1.04));
            let peak = geo::arc_peak_height(corridor.distance_km);
            let segments = 24u32;
            let arc = sol_atlas_core::geometry::generate_arc(from, to, peak, segments);
            for i in 0..segments as usize {
                let a = Vec3::new(arc[i * 3], arc[i * 3 + 1], arc[i * 3 + 2]);
                let b = Vec3::new(arc[(i + 1) * 3], arc[(i + 1) * 3 + 1], arc[(i + 1) * 3 + 2]);
                gizmos.line(a, b, rail_color);
            }
        }
    }

    // Supply routes — cyan arcs, dimmer. Gated on Layer::SupplyChain's
    // actual visibility (2026-07-26 fix, same bug class as Maglev above):
    // this block previously ignored `visible_layers()` entirely and drew
    // in every non-Off view, including Climate/Interplanetary/Scenarios
    // where SupplyChain isn't part of the preset.
    if view.visible_layers().contains(&Layer::SupplyChain) {
        let supply_color = Color::linear_rgba(0.0, 0.5, 0.8, 0.3); // cyan, dimmer — background layer
        for route in atlas_data.data.supply_routes.iter().take(5) {
            // limit to 5 routes
            let from = rotate(geo::lat_lon_to_xyz(route.from_lat, route.from_lon, 1.04));
            let to = rotate(geo::lat_lon_to_xyz(route.to_lat, route.to_lon, 1.04));
            let dist = sol_atlas_core::geo::haversine_km(
                route.from_lat,
                route.from_lon,
                route.to_lat,
                route.to_lon,
            );
            let peak = geo::arc_peak_height(dist);

            let segments = 16u32;
            let arc = sol_atlas_core::geometry::generate_arc(from, to, peak, segments);
            for i in 0..segments as usize {
                let a = Vec3::new(arc[i * 3], arc[i * 3 + 1], arc[i * 3 + 2]);
                let b = Vec3::new(arc[(i + 1) * 3], arc[(i + 1) * 3 + 1], arc[(i + 1) * 3 + 2]);
                gizmos.line(a, b, supply_color);
            }
        }
    }

    // ═══ SHIPPING LANES (Infrastructure view only) ════════════════
    // Was unconditionally `break`-ing out of the loop before it ever drew
    // anything, regardless of view -- the comment's stated intent ("enable
    // in Infrastructure view") was never actually wired to the real
    // DataView check the rest of this function uses. Fixed: gate on view
    // instead of a hardcoded break.
    if *view == DataView::Infrastructure || *view == DataView::All {
        let lanes = sol_atlas_bevy::data::load_shipping_lanes();
        let lane_color = Color::linear_rgba(0.15, 0.3, 0.55, 0.10);
        for route in &lanes {
            for w in route.windows(2) {
                let a = rotate(geo::lat_lon_to_xyz(w[0][1], w[0][0], 1.04));
                let b = rotate(geo::lat_lon_to_xyz(w[1][1], w[1][0], 1.04));
                gizmos.line(
                    Vec3::new(a[0], a[1], a[2]),
                    Vec3::new(b[0], b[1], b[2]),
                    lane_color,
                );
            }
        }
    }
}

/// Space-time gravity-well grid — radial + concentric funnel grid beneath
/// the globe (y = base - k / (r² + ε), a curvature visualization). Was
/// drawn unconditionally inside `draw_arcs_system` despite
/// `OverlayManager::show_grid` existing specifically to gate it — the flag
/// was set by `overlay_toggle_system` but nothing ever read it. Split into
/// its own system so that field actually does something.
pub fn draw_gravity_grid_system(
    mut gizmos: Gizmos,
    time: Res<Time>,
    overlays: Res<OverlayManager>,
) {
    if !overlays.show_grid {
        return;
    }
    let t = time.elapsed_secs();

    let base_y = -0.7;
    let gravity_k = 0.15;
    let epsilon = 0.3;
    let flicker = (t * 5.0).sin().abs() * 0.4 + 0.6; // holographic flicker

    // Concentric circles (8 rings)
    for ring in 1..=8 {
        let r = ring as f32 * 0.5;
        let segments = 48;
        let ring_alpha = 0.04 * flicker * (1.0 - ring as f32 / 10.0); // subtle — doesn't compete with data
        let ring_color = Color::linear_rgba(0.0, 0.5, 0.7, ring_alpha);
        for i in 0..segments {
            let a0 = i as f32 / segments as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            let x0 = r * a0.cos();
            let z0 = r * a0.sin();
            let x1 = r * a1.cos();
            let z1 = r * a1.sin();
            let y0 = base_y - gravity_k / (x0 * x0 + z0 * z0 + epsilon);
            let y1 = base_y - gravity_k / (x1 * x1 + z1 * z1 + epsilon);
            gizmos.line(Vec3::new(x0, y0, z0), Vec3::new(x1, y1, z1), ring_color);
        }
    }

    // Radial spokes (12 lines from center outward)
    for spoke in 0..12 {
        let angle = spoke as f32 / 12.0 * std::f32::consts::TAU;
        let spoke_alpha = 0.05 * flicker;
        let spoke_color = Color::linear_rgba(0.0, 0.4, 0.6, spoke_alpha);
        let segments = 16;
        for i in 0..segments {
            let r0 = i as f32 / segments as f32 * 4.0 + 0.3;
            let r1 = (i + 1) as f32 / segments as f32 * 4.0 + 0.3;
            let x0 = r0 * angle.cos();
            let z0 = r0 * angle.sin();
            let x1 = r1 * angle.cos();
            let z1 = r1 * angle.sin();
            let y0 = base_y - gravity_k / (x0 * x0 + z0 * z0 + epsilon);
            let y1 = base_y - gravity_k / (x1 * x1 + z1 * z1 + epsilon);
            gizmos.line(Vec3::new(x0, y0, z0), Vec3::new(x1, y1, z1), spoke_color);
        }
    }
}

/// Update marker visibility based on timeline year.
pub fn timeline_visibility_system(
    state: Res<TimelineState>,
    mut markers: Query<
        (&TimelineLayer, Option<&FossilTimelineData>, &mut Visibility),
        With<AtlasEntity>,
    >,
) {
    let year = state.year;
    for (layer, fossil_data, mut vis) in markers.iter_mut() {
        let opacity = match layer {
            TimelineLayer::Fossil => match fossil_data {
                // Real per-deposit status now that FossilTimelineData is
                // attached at spawn (2026-07-27 fix -- this used to be a
                // generic fade because "per-deposit opacity needs stored
                // data" that didn't exist yet).
                Some(data) => {
                    sol_atlas_core::timeline::fossil_opacity_for_status(&data.status, year)
                }
                None => 1.0 - (year as f32 / 300.0).min(1.0),
            },
            TimelineLayer::Renewable => sol_atlas_core::timeline::renewable_opacity(year),
            TimelineLayer::Nuclear => sol_atlas_core::timeline::nuclear_opacity(year),
            TimelineLayer::Vault(i) => {
                if sol_atlas_core::timeline::vault_visible(*i, year) {
                    1.0
                } else {
                    0.0
                }
            }
            TimelineLayer::Corridor(i) => {
                if sol_atlas_core::timeline::corridor_visible(*i, year) {
                    1.0
                } else {
                    0.0
                }
            }
            TimelineLayer::Star => 1.0,
        };

        *vis = if opacity < 0.05 {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

/// Recolors each fossil deposit's main marker as the timeline scrubs,
/// calling the same `fossil_eroi_at_year` decay curve
/// sol-atlas-leptos uses instead of leaving color fixed at spawn-time EROI
/// (added 2026-07-27, closing the other half of the timeline-unification
/// gap `timeline_visibility_system` fixed above — color previously never
/// changed at all as the Bevy timeline moved, only visibility did).
/// `FossilMainMarker` restricts this to the primary dot, not its CO2 halo
/// (fixed translucent red, not EROI-derived).
pub fn fossil_eroi_color_system(
    state: Res<TimelineState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    markers: Query<
        (&FossilTimelineData, &MeshMaterial3d<StandardMaterial>),
        With<FossilMainMarker>,
    >,
) {
    let year = state.year;
    for (data, mat_handle) in &markers {
        let Some(mut mat) = materials.get_mut(&mat_handle.0) else {
            continue;
        };
        let eroi =
            sol_atlas_core::timeline::fossil_eroi_at_year(data.base_eroi, &data.fuel_type, year);
        let c = sol_atlas_core::economics::eroi_color(eroi);
        let emissive = geo::fossil_emissive_factor(&data.status) * 0.5;
        mat.base_color = Color::linear_rgb(c[0] * emissive, c[1] * emissive, c[2] * emissive);
    }
}

/// 4D temporal visibility — markers emerge from the 4th dimension based on timeline year.
/// Uses Projector4D hyperplane slicing: markers with TemporalW close to the current
/// timeline year are visible; those far from the slice fade out.
pub fn temporal_4d_system(
    timeline: Res<TimelineState>,
    mut markers: Query<(&TemporalW, &mut Visibility), With<AtlasEntity>>,
) {
    // Map timeline year (0-500) to absolute year (1900-2400)
    let current_year = 1900.0 + timeline.year as f64 * 1.0;

    // Create a Projector4D with the timeline as W-slice
    let projector = symtropy_render_bridge::Projector4D::new(
        current_year,
        100.0, // slice_thickness: markers within ±100 years are visible
        1.0,
    );

    for (temporal, mut vis) in markers.iter_mut() {
        let point = symtropy_math::Point::new([0.0, 0.0, 0.0, temporal.year]);
        let alpha = projector.alpha(&point);

        *vis = if alpha > 0.05 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update side panel metrics with live data.
pub fn panel_metrics_system(
    timeline: Res<TimelineState>,
    view: Res<DataView>,
    aesthetic: Res<CurrentAesthetic>,
    mut texts: Query<&mut Text, With<PanelMetrics>>,
) {
    let year = timeline.year;
    let phase = sol_atlas_core::simulation::secular_phase_at_year(year);
    let colonies = sol_atlas_core::simulation::colonies_at_year(year);
    let total_colony_pop: u32 = colonies.iter().map(|c| c.population).sum();

    let metrics = format!(
        "Year: {}\nPhase: {}\nView: {}\nStyle: {:?}\nColonies: {}\nOff-world: {}",
        year,
        phase.label(),
        view.label(),
        aesthetic.aesthetic,
        colonies.len(),
        total_colony_pop,
    );

    for mut text in texts.iter_mut() {
        *text = Text::new(metrics.clone());
    }
}

/// Timeline scrubber bar — visual progress indicator at bottom of screen.
#[derive(Component)]
pub struct TimelineScrubber;

#[derive(Component)]
pub struct TimelineFill;

/// Update timeline scrubber fill width based on current year.
pub fn timeline_scrubber_system(
    timeline: Res<TimelineState>,
    mut fills: Query<&mut Node, With<TimelineFill>>,
) {
    let progress = (timeline.year as f32 / 500.0).clamp(0.0, 1.0) * 100.0;
    for mut node in fills.iter_mut() {
        node.width = Val::Percent(progress);
    }
}

/// Bevy's default `PerspectiveProjection` vertical FOV -- confirmed no
/// override anywhere in `sol-atlas-bevy`'s camera spawn code
/// (`camera::spawn_camera` uses `Camera3d::default()`). Used below to
/// derive a guaranteed-in-frame F8 distance rather than a hand-picked one.
const DEFAULT_VERTICAL_FOV_DEG: f32 = 45.0;

/// Planet focus — F1-F6 fly camera to a planet, F7 returns to Earth,
/// F8 pulls back to frame the whole solar system at once, F10-F12 fly to
/// Mercury/Uranus/Neptune (added later than F1-F6, hence the non-contiguous
/// key range -- F9 was already claimed by the unrelated frame-capture system).
pub fn planet_focus_system(
    kb: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera: ResMut<sol_atlas_bevy::camera::OrbitalCameraConfig>,
    bodies_res: Res<SolarSystemBodiesRes>,
) {
    let t = time.elapsed_secs();
    let bodies = &bodies_res.0;

    let target_body = if kb.just_pressed(KeyCode::F1) {
        Some("Moon")
    } else if kb.just_pressed(KeyCode::F2) {
        Some("Venus")
    } else if kb.just_pressed(KeyCode::F3) {
        Some("Mars")
    } else if kb.just_pressed(KeyCode::F4) {
        Some("Jupiter")
    } else if kb.just_pressed(KeyCode::F5) {
        Some("Saturn")
    } else if kb.just_pressed(KeyCode::F6) {
        Some("Sun")
    } else if kb.just_pressed(KeyCode::F10) {
        Some("Mercury")
    } else if kb.just_pressed(KeyCode::F11) {
        Some("Uranus")
    } else if kb.just_pressed(KeyCode::F12) {
        Some("Neptune")
    } else if kb.just_pressed(KeyCode::F7) {
        // Return to Earth. Distance references the shared default constant
        // (2026-07-26 fix) instead of duplicating a magic number that would
        // silently drift from it -- e.g. when Earth-tilt-visibility Finding
        // 3 raised `CAMERA_INITIAL_DISTANCE` for better whole-silhouette
        // framing, this preset used to still reset to the old, too-tight
        // 4.2. `phi` is likewise raised to stay consistent with the new
        // `AUTO_ORIENT_TARGET_PHI` default rather than reintroducing the
        // same too-flat framing the default-camera fix just corrected.
        camera.look_offset = Vec3::ZERO;
        camera.focused_body = None;
        camera.distance = sol_atlas_core::constants::CAMERA_INITIAL_DISTANCE;
        camera.theta = 0.0;
        camera.phi = 0.3;
        camera.auto_rotate = true;
        info!("[atlas] Camera → Earth");
        None
    } else if kb.just_pressed(KeyCode::F8) {
        // Whole solar system overview -- added 2026-07-26, distance
        // formula fixed same day: the original hand-picked distance=38
        // did NOT reliably frame Jupiter/Saturn (computed against Bevy's
        // real default 45° vertical FOV: out of frame for ~32%/~51% of
        // their respective orbits, depending on simulated time). A body
        // orbiting at radius R never subtends more than arcsin(R/distance)
        // from the camera's boresight, regardless of orbital phase or
        // viewing angle (the same bound as a sphere of radius R viewed
        // from an external point -- our bodies orbiting in a plane is a
        // conservative subset of that), so solving for distance against
        // the tighter of the two FOV axes (vertical -- horizontal is wider
        // at any normal landscape aspect ratio) guarantees every body
        // stays in frame at all times, not just on average. Stays centered
        // on Earth's origin (look_offset unchanged) since this is meant to
        // frame everything at once, not one body. Saturn's orbit_radius
        // (28.4) sits well beyond CAMERA_ZOOM_MAX (8.0), but that clamp
        // only applies to scroll-wheel zoom input, not a direct
        // `camera.distance` assignment (same reason F1-F6 below can
        // already fly out to Jupiter/Saturn/Sun despite that limit).
        // auto_rotate off, matching every other F-key preset, for a stable
        // frame to screenshot rather than one that immediately drifts.
        let max_orbit_radius = bodies
            .iter()
            .map(|b| b.orbit_radius)
            .fold(0.0_f32, f32::max);
        let half_vertical_fov = (DEFAULT_VERTICAL_FOV_DEG / 2.0).to_radians();
        // A modest buffer over the exact worst-case boundary -- not a
        // hand-picked guess, since the formula above is exact geometry --
        // so a body doesn't sit precisely on the frame edge.
        const WHOLE_SYSTEM_FRAMING_MARGIN: f32 = 1.15;
        camera.look_offset = Vec3::ZERO;
        camera.focused_body = None;
        camera.distance =
            (max_orbit_radius / half_vertical_fov.sin()) * WHOLE_SYSTEM_FRAMING_MARGIN;
        camera.theta = 0.6;
        camera.phi = 0.4;
        camera.auto_rotate = false;
        info!(
            "[atlas] Camera → Whole Solar System (distance {:.1})",
            camera.distance
        );
        None
    } else {
        None
    };

    if let Some(name) = target_body {
        if let Some(body) = bodies.iter().find(|b| b.name == name) {
            let pos = sol_atlas_core::solar_system::body_position(body, t);
            // 2026-07-26 fix: this used to point theta/phi at the body's
            // direction from Earth while still looking_at(ZERO) (Earth) --
            // so the camera ended up positioned near the body but facing
            // the WRONG way (back toward Earth), putting the "focused"
            // body behind/beside the frame instead of centered in it.
            // Now the camera genuinely orbits the body itself, and
            // celestial_body_update_system re-syncs look_offset to this
            // body's current position every frame (focused_body), so the
            // frame doesn't drift as the body continues to orbit.
            camera.look_offset = Vec3::new(pos[0], pos[1], pos[2]);
            camera.focused_body = Some(name.to_string());
            camera.theta = 0.4; // a pleasant fixed viewing angle, same for every body
            camera.phi = 0.3;
            // `* 4.0` keeps R/distance = 0.25 for every body regardless of
            // size, so each one subtends the same arcsin(0.25) ≈ 14.5°
            // from boresight -- comfortably inside both the ~22.5° default
            // vertical and ~36.4° horizontal half-FOV (see
            // DEFAULT_VERTICAL_FOV_DEG above), and well clear of the near
            // clip plane even for the smallest body (Moon). Verified this
            // holds for every body from Moon (0.06) to Sun (1.8); if
            // `visual_radius` scaling or the camera's default FOV ever
            // change independently, re-check this margin.
            camera.distance = body.visual_radius * 4.0;
            camera.auto_rotate = false;
            info!(
                "[atlas] Camera → {} (distance {:.2})",
                name, camera.distance
            );
        }
    }
}

/// Data view switcher — Tab key cycles through categorical presets.
pub fn data_view_switch_system(
    kb: Res<ButtonInput<KeyCode>>,
    mut view: ResMut<DataView>,
    mut view_hud: Query<(&mut Text, &mut TextColor), With<ViewHud>>,
) {
    if kb.just_pressed(KeyCode::Tab) {
        *view = view.next();
        info!("[view] Switched to: {}", view.label());
        let c = view.color();
        for (mut text, mut color) in view_hud.iter_mut() {
            *text = Text::new(view.label());
            *color = TextColor(Color::srgba(c[0], c[1], c[2], 0.65));
        }
    }
}

/// Marker for a clickable DataView picker button (2026-07-27, Bevy-side
/// parity with sol-atlas-leptos's lens picker).
#[derive(Component)]
pub struct DataViewButton(pub DataView);

/// Click a DataView picker button to jump straight to that view -- the
/// click-driven counterpart to Tab-cycling. Sets the same `DataView`
/// resource `data_view_filter_system` already reads every frame, so no
/// other wiring is needed for the layer filter itself to pick this up.
pub fn data_view_button_click_system(
    mut view: ResMut<DataView>,
    mut view_hud: Query<(&mut Text, &mut TextColor), With<ViewHud>>,
    buttons: Query<(&Interaction, &DataViewButton), Changed<Interaction>>,
) {
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed && *view != button.0 {
            *view = button.0;
            info!("[view] Clicked: {}", view.label());
            let c = view.color();
            for (mut text, mut color) in view_hud.iter_mut() {
                *text = Text::new(view.label());
                *color = TextColor(Color::srgba(c[0], c[1], c[2], 0.65));
            }
        }
    }
}

/// Re-tints each picker button so the currently-active view is visibly
/// distinct from the other 10 -- otherwise a click-driven picker with no
/// feedback is worse than blind Tab-cycling, not better.
pub fn data_view_button_highlight_system(
    view: Res<DataView>,
    mut buttons: Query<(&DataViewButton, &mut BackgroundColor, &Children)>,
    mut texts: Query<&mut TextColor>,
) {
    if !view.is_changed() {
        return;
    }
    for (button, mut bg, children) in &mut buttons {
        let active = button.0 == *view;
        let c = button.0.color();
        *bg = if active {
            BackgroundColor(Color::srgba(c[0], c[1], c[2], 0.22))
        } else {
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0))
        };
        for child in children.iter() {
            if let Ok(mut text_color) = texts.get_mut(child) {
                *text_color = if active {
                    TextColor(Color::srgba(c[0], c[1], c[2], 0.95))
                } else {
                    TextColor(Color::srgba(0.5, 0.55, 0.6, 0.55))
                };
            }
        }
    }
}

/// Data view filter — hides markers not in the active view's layer set.
/// Data view filter — hides markers not in the active view's layer set.
/// Runs every frame (not just on change) because LOD also sets Visibility.
pub fn data_view_filter_system(
    view: Res<DataView>,
    mut markers: Query<
        (&DataMarker, &mut Visibility),
        (
            Without<CityIndicator>,
            Without<SurfaceLod>,
            Without<OrbitLod>,
        ),
    >,
    mut surface_markers: Query<
        (&DataMarker, &mut Visibility),
        (With<SurfaceLod>, Without<OrbitLod>, Without<CityIndicator>),
    >,
    mut orbit_markers: Query<
        (&DataMarker, &mut Visibility),
        (With<OrbitLod>, Without<SurfaceLod>, Without<CityIndicator>),
    >,
    camera: Query<&Transform, With<OrbitalCamera>>,
) {
    let visible_layers = view.visible_layers();
    let show_all = *view == DataView::All;

    // Get LOD state
    let distance = camera
        .single()
        .map(|t| t.translation.length())
        .unwrap_or(4.2);
    let lod = LodLevel::from_camera_distance(distance);
    let show_surface = matches!(lod, LodLevel::Surface);
    let show_orbit = matches!(lod, LodLevel::Orbit);

    // Always-visible markers (no LOD tag): filter by view only
    for (marker, mut vis) in markers.iter_mut() {
        *vis = if show_all || visible_layers.contains(&marker.layer) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // SurfaceLod markers: filter by view AND distance
    for (marker, mut vis) in surface_markers.iter_mut() {
        let view_ok = show_all || visible_layers.contains(&marker.layer);
        *vis = if view_ok && show_surface {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // OrbitLod markers: filter by view AND distance
    for (marker, mut vis) in orbit_markers.iter_mut() {
        let view_ok = show_all || visible_layers.contains(&marker.layer);
        *vis = if view_ok && show_orbit {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update the timeline HUD with current year + Turchin cycle phase.
pub fn timeline_hud_system(
    timeline: Res<TimelineState>,
    mut texts: Query<(&mut Text, &mut TextColor), With<TimelineHud>>,
) {
    let year = timeline.year;
    let phase = sol_atlas_core::simulation::secular_phase_at_year(year);
    let c = phase.color();

    for (mut text, mut color) in texts.iter_mut() {
        *text = Text::new(format!("Year {} | {}", year, phase.label()));
        *color = TextColor(Color::linear_rgba(c[0], c[1], c[2], 0.6));
    }
}

/// Evolve city grid stress based on timeline year.
/// As the timeline scrubs, city indicators change color to reflect
/// the civilization simulator's predictions of grid stress evolution.
pub fn city_stress_evolution_system(
    timeline: Res<TimelineState>,
    mut city_markers: Query<(&CityIndicator, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let stress_data = sol_atlas_core::simulation::evolve_grid_stress(timeline.year);

    for (city, mat_handle) in city_markers.iter_mut() {
        // Find matching stress data by city name
        if let Some(stress) = stress_data.iter().find(|s| s.name == city.name) {
            if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                let c = sol_atlas_core::energy_trading::stress_color(stress.allostatic_load);
                mat.base_color = Color::linear_rgba(c[0], c[1], c[2], 0.8);
                mat.emissive = LinearRgba::new(c[0] * 0.5, c[1] * 0.5, c[2] * 0.5, 1.0);
            }
        }
    }
}

/// Aesthetic switcher — number keys 1-5 cycle visual presets.
pub fn aesthetic_switch_system(
    kb: Res<ButtonInput<KeyCode>>,
    mut current: ResMut<CurrentAesthetic>,
) {
    use sol_atlas_core::aesthetics::Aesthetic;
    let new = if kb.just_pressed(KeyCode::Digit1) {
        Some(Aesthetic::Holographic)
    } else if kb.just_pressed(KeyCode::Digit2) {
        Some(Aesthetic::Satellite)
    } else if kb.just_pressed(KeyCode::Digit3) {
        Some(Aesthetic::Procedural)
    } else if kb.just_pressed(KeyCode::Digit4) {
        Some(Aesthetic::Minimal)
    } else if kb.just_pressed(KeyCode::Digit5) {
        Some(Aesthetic::Night)
    } else {
        None
    };

    if let Some(aesthetic) = new {
        if aesthetic != current.aesthetic {
            current.aesthetic = aesthetic;
            current.changed = true;
            info!(
                "[atlas] Aesthetic: {} (press 1-5 to switch)",
                aesthetic.label()
            );
        }
    }
}

/// Apply aesthetic changes to globe materials when preset changes.
pub fn aesthetic_apply_system(
    mut current: ResMut<CurrentAesthetic>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut holo_materials: ResMut<Assets<sol_atlas_bevy::holographic_material::HolographicMaterial>>,
    atmo_q: Query<&MeshMaterial3d<StandardMaterial>, With<Atmosphere>>,
    mut cloud_vis: Query<&mut Visibility, With<CloudLayer>>,
) {
    if !current.changed {
        return;
    }
    current.changed = false;

    let config = sol_atlas_core::aesthetics::config_for(current.aesthetic);

    // Update holographic globe material + shader params per aesthetic
    for (_, mat) in holo_materials.iter_mut() {
        let c = config.globe.base_color;
        mat.base.base_color = Color::linear_rgba(c[0], c[1], c[2], c[3]);
        let e = config.globe.emissive;
        mat.base.emissive = LinearRgba::new(e[0], e[1], e[2], e[3]);
        mat.base.unlit = config.globe.unlit;

        // Override holographic shader params per aesthetic
        match current.aesthetic {
            sol_atlas_core::aesthetics::Aesthetic::Satellite => {
                // PBR mode — disable holographic, enable real lighting
                mat.extension.enable_holographic = 0.0; // shader skips scanlines/noise
                mat.extension.hologram_alpha = 1.0;
                mat.extension.fresnel_power = 2.0;
                mat.extension.fresnel_color = LinearRgba::new(0.3, 0.5, 0.8, 1.0);
                mat.extension.outline_intensity = 0.0; // real photo texture — no need for a vector outline
                mat.base.unlit = false; // enable PBR lighting
            }
            sol_atlas_core::aesthetics::Aesthetic::Night => {
                mat.extension.enable_holographic = 1.0;
                mat.extension.hologram_alpha = 0.7;
                mat.extension.scanline_density = 10.0;
                mat.extension.fresnel_power = 2.0;
                mat.extension.fresnel_color = LinearRgba::new(0.1, 0.15, 0.3, 1.0);
                mat.extension.outline_intensity = 0.0;
                mat.base.unlit = true;
            }
            sol_atlas_core::aesthetics::Aesthetic::Minimal => {
                mat.extension.enable_holographic = 0.0; // clean PBR
                mat.extension.hologram_alpha = 0.3;
                mat.extension.fresnel_power = 4.0;
                mat.extension.fresnel_color = LinearRgba::new(0.2, 0.3, 0.4, 1.0);
                // "Minimal" is billed as coastline-outlines-only — a good
                // future candidate for this effect too, but out of scope
                // for the current ask (fixing Holographic specifically).
                mat.extension.outline_intensity = 0.0;
                mat.base.unlit = true;
            }
            sol_atlas_core::aesthetics::Aesthetic::Procedural => {
                mat.extension.enable_holographic = 1.0;
                mat.extension.hologram_alpha = 0.65;
                mat.extension.scanline_density = 12.0;
                mat.extension.fresnel_power = 3.0;
                mat.extension.fresnel_color = LinearRgba::new(0.0, 0.87, 1.0, 1.0);
                mat.extension.outline_intensity = 0.0;
                mat.base.unlit = true;
            }
            sol_atlas_core::aesthetics::Aesthetic::Holographic => {
                // Full holographic effects. hologram_alpha was 0.35 — with
                // the old fresnel-gated shader alpha that crushed the
                // visible landmass to near-nothing face-on; now that the
                // shader alpha isn't fresnel-gated, keep it high enough
                // that continents actually read.
                mat.extension.enable_holographic = 1.0;
                mat.extension.hologram_alpha = 0.9;
                mat.extension.scanline_density = 20.0;
                mat.extension.scanline_speed = 0.5;
                mat.extension.fresnel_power = 3.0;
                mat.extension.fresnel_color = LinearRgba::new(0.0, 0.87, 1.0, 1.0);
                mat.extension.outline_color = LinearRgba::new(0.3, 1.0, 0.85, 1.0);
                mat.extension.outline_intensity = 1.4;
                mat.extension.outline_threshold = 0.12;
                mat.base.unlit = true;
            }
        }
    }

    // Update atmosphere/fresnel materials
    for mat_handle in atmo_q.iter() {
        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            let f = &config.fresnel;
            mat.base_color = Color::linear_rgba(f.color[0], f.color[1], f.color[2], f.color[3]);
            mat.emissive =
                LinearRgba::new(f.emissive[0], f.emissive[1], f.emissive[2], f.emissive[3]);
        }
    }

    // Toggle cloud layer visibility per aesthetic
    let show_clouds = matches!(
        current.aesthetic,
        sol_atlas_core::aesthetics::Aesthetic::Satellite
            | sol_atlas_core::aesthetics::Aesthetic::Procedural
    );
    for mut vis in cloud_vis.iter_mut() {
        *vis = if show_clouds {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    info!("[aesthetic] Switched to {:?}", current.aesthetic);
}

/// LOD visibility — toggle markers based on camera zoom distance.
/// Mutually exclusive: Orbit = heat blobs, Surface = markers, Atmosphere = neither.
pub fn lod_visibility_system(
    camera: Query<&Transform, With<OrbitalCamera>>,
    mut surface_markers: Query<&mut Visibility, (With<SurfaceLod>, Without<OrbitLod>)>,
    mut orbit_blobs: Query<&mut Visibility, (With<OrbitLod>, Without<SurfaceLod>)>,
) {
    let Ok(cam_tf) = camera.single() else { return };
    let distance = cam_tf.translation.length();
    let lod = LodLevel::from_camera_distance(distance);

    let show_surface = matches!(lod, LodLevel::Surface);
    let show_orbit = matches!(lod, LodLevel::Orbit);
    // Atmosphere = clean gap — only arcs + globe visible

    for mut vis in surface_markers.iter_mut() {
        *vis = if show_surface {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in orbit_blobs.iter_mut() {
        *vis = if show_orbit {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// [8] Sacred Stillness breathing — atmosphere shells pulse on 8-second cycle.
/// Only pulses the atmosphere (not markers, to avoid scale drift).
pub fn holographic_pulse_system(
    time: Res<Time>,
    mut atmospheres: Query<&mut Transform, (With<Atmosphere>, With<AtlasEntity>)>,
) {
    let t = time.elapsed_secs();
    // 8-second Sacred Stillness breathing cycle
    let breath = 1.0 + 0.03 * (t * std::f32::consts::TAU / 8.0).sin();

    for mut tf in atmospheres.iter_mut() {
        let base = tf.scale.x.max(0.5); // avoid zero scale
        // Apply breathing to atmosphere shells (they started at ~1.03-1.05 scale)
        tf.scale = Vec3::splat(base.signum() * breath * 1.04);
    }
}

/// Earth's real axial tilt, degrees (its own value -- Earth isn't part of
/// `solar_system_bodies()`, which only holds the 6 bodies orbiting it).
const EARTH_AXIAL_TILT_DEG: f32 = 23.44;
/// Spin rate, radians/second -- tuned for a ~55s/rotation pace, close to
/// Jupiter/Mars's simulated speed and slow enough to clearly watch
/// continents sweep across, matching the reasoning in `solar_system.rs`'s
/// `spin_speed` doc comment.
const EARTH_SPIN_SPEED: f32 = 0.1142;
/// A cloud layer's own rotation rate relative to the globe underneath it
/// -- real clouds drift independently of the solid surface they cover
/// (different atmospheric layer, different apparent speed), not in lockstep.
const CLOUD_DRIFT_SPEED: f32 = 0.02;

/// Fixes a real bug found 2026-07-26 (user report: "the earth is rotating
/// around the equator rather than the poles", confirmed to affect every
/// other body too): every rotation formula in this file assumed Bevy's
/// `Sphere::new(r).mesh().uv(sectors, stacks)` puts the mesh's poles along
/// +/-Y, matching the engine's Y-up scene convention, and so spun bodies
/// with `Quat::from_rotation_y(spin_angle)`. That assumption is wrong.
/// Reading `bevy_mesh`'s actual generator (`primitives/dim3/sphere.rs`,
/// ported from a Z-up OpenGL tutorial convention) confirms each vertex is
/// built as `[x, y, z]` where `z = radius * sin(stack_angle)` is the
/// latitude/pole coordinate and `x`/`y` trace the equatorial ring -- i.e.
/// the mesh's own geometric poles sit on its **local Z axis**, with the
/// equator in its local XY plane. Spinning around local Y (which lies IN
/// that equatorial plane) therefore swings the real poles around in a big
/// circle instead of holding them still -- exactly "rotating around the
/// equator instead of the poles". This function is the one-time fixed
/// correction that re-orients the mesh's native Z-pole to point along
/// world +Y (this scene's up/orbital-normal axis) before any spin/tilt is
/// applied: `Quat::from_rotation_x(-FRAC_PI_2)` maps local (0,0,1) -> world
/// (0,1,0), verified by direct calculation. Multiplying it in on the right
/// (applied first, closest to the raw mesh vertices) leaves every existing
/// "tilt * spin-around-Y" formula in this file conceptually unchanged --
/// it was already correct FOR A Y-UP MESH, it was just never given one.
fn mesh_pole_to_world_up() -> Quat {
    Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
}

/// Earth's own axial rotation (2026-07-26 fix): previously nothing ever
/// rotated the `Globe` entity at all -- it sat at `Transform::IDENTITY`
/// forever, and the only apparent "spin" was the orbital camera moving
/// around a completely static globe. Clouds got a tiny, wrong-axis drift
/// of their own (`rotate_x` is not a real polar axis), independent of the
/// solid surface beneath them. Now both spin together around Earth's real
/// 23.44° tilted axis, with clouds drifting a bit faster on top -- same
/// "spin first (local Y), then tilt the whole frame" composition used for
/// the 6 orbiting bodies in `celestial_body_update_system`, now with the
/// `mesh_pole_to_world_up()` correction (see its doc comment) so "local Y"
/// is actually the mesh's real polar axis instead of its equator.
pub fn cloud_rotation_system(
    time: Res<Time>,
    mut globe: Query<&mut Transform, (With<Globe>, Without<CloudLayer>)>,
    mut clouds: Query<&mut Transform, (With<CloudLayer>, Without<Globe>)>,
) {
    let t = time.elapsed_secs();
    let tilt = Quat::from_rotation_z(EARTH_AXIAL_TILT_DEG.to_radians());
    let pole_fix = mesh_pole_to_world_up();

    for mut tf in globe.iter_mut() {
        tf.rotation = tilt * Quat::from_rotation_y(t * EARTH_SPIN_SPEED) * pole_fix;
    }
    for mut tf in clouds.iter_mut() {
        tf.rotation =
            tilt * Quat::from_rotation_y(t * (EARTH_SPIN_SPEED + CLOUD_DRIFT_SPEED)) * pole_fix;
    }
}

/// Marker pulse — data markers breathe with sinusoidal scale modulation.
pub fn marker_pulse_system(time: Res<Time>, mut markers: Query<(&MarkerPulse, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (pulse, mut tf) in markers.iter_mut() {
        let scale =
            pulse.base_scale * (1.0 + pulse.amplitude * (t * pulse.speed + pulse.phase).sin());
        tf.scale = Vec3::splat(scale);
    }
}

/// Consciousness-coupled holographic shader — Phi modulates visual quality.
pub fn consciousness_shader_system(
    player_c: Option<Res<crate::systems::consciousness::PlayerConsciousness>>,
    mut materials: ResMut<Assets<sol_atlas_bevy::holographic_material::HolographicMaterial>>,
) {
    // Get consciousness level (fallback to 0.5 if not in dungeon)
    let phi = player_c.map(|c| c.level as f32).unwrap_or(0.5);

    for (_, mat) in materials.iter_mut() {
        // Higher phi = tighter fresnel (more integrated, coherent light)
        mat.extension.fresnel_power = 2.0 + phi * 3.0; // 2.0 → 5.0
        // Higher phi = slower scanlines (serene vs anxious)
        mat.extension.scanline_speed = 1.3 - phi * 1.0; // 1.3 → 0.3
        // Higher phi = more opaque (clearer perception)
        mat.extension.hologram_alpha = 0.40 + phi * 0.40; // 0.40 → 0.80
    }
}

/// Animate celestial bodies along their orbits (drawn as gizmo orbit rings).
pub fn celestial_orbit_system(
    mut gizmos: Gizmos,
    time: Res<Time>,
    timeline: Res<TimelineState>,
    bodies_res: Res<SolarSystemBodiesRes>,
) {
    let t = time.elapsed_secs();
    let bodies = &bodies_res.0;

    for body in bodies {
        // Draw the body's real orbital ellipse shape, reprojected through
        // Earth's current position every frame (2026-07-26: replaces the
        // old fixed-circle-at-`orbit_radius` ring, which became a stale,
        // visually-mismatched leftover once real Keplerian positions
        // landed). Alpha raised from 0.02/0.03 so the ring is actually
        // usable as a "level" reference for judging axial tilt (Earth-
        // tilt-visibility Finding 3), not just barely-visible ghost lines.
        let segments = 64;
        let orbit_color = if body.is_sun {
            Color::linear_rgba(1.0, 0.8, 0.3, 0.12)
        } else {
            Color::linear_rgba(0.3, 0.4, 0.5, 0.15)
        };

        let ring = sol_atlas_core::solar_system::body_orbit_ring(body, t, segments);
        for i in 0..segments {
            let p0 = ring[i];
            let p1 = ring[(i + 1) % segments];
            gizmos.line(
                Vec3::new(p0[0], p0[1], p0[2]),
                Vec3::new(p1[0], p1[1], p1[2]),
                orbit_color,
            );
        }

        // Glow ring around body at current position — holographic aura
        let pos = sol_atlas_core::solar_system::body_position(body, t);
        let flicker = (t * 4.0 + body.orbit_offset).sin().abs() * 0.4 + 0.6;
        let ring_r = body.visual_radius * 1.8;
        let ring_alpha = if body.is_sun { 0.25 } else { 0.12 } * flicker;
        let ring_color = if body.is_sun {
            Color::linear_rgba(1.0, 0.8, 0.3, ring_alpha) // bright solar corona
        } else {
            Color::linear_rgba(0.3, 0.5, 0.8, ring_alpha)
        };
        let ring_segs = 24;
        for i in 0..ring_segs {
            let a0 = i as f32 / ring_segs as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / ring_segs as f32 * std::f32::consts::TAU;
            gizmos.line(
                Vec3::new(
                    pos[0] + ring_r * a0.cos(),
                    pos[1],
                    pos[2] + ring_r * a0.sin(),
                ),
                Vec3::new(
                    pos[0] + ring_r * a1.cos(),
                    pos[1],
                    pos[2] + ring_r * a1.sin(),
                ),
                ring_color,
            );
        }
    }

    // ═══ COLONY INDICATORS ═════════════════════════════════════════
    // Show civilization presence on planets based on timeline year.
    let colonies = sol_atlas_core::simulation::colonies_at_year(timeline.year);
    for colony in &colonies {
        // Find the planet's current position
        if let Some(body) = bodies.iter().find(|b| b.name == colony.body) {
            let pos = sol_atlas_core::solar_system::body_position(body, t);
            let flicker = (t * 3.0 + colony.light_delay_s * 0.01).sin().abs() * 0.3 + 0.7;

            // Colony indicator: small bright sphere near the planet
            let colony_color = Color::linear_rgba(0.2, 0.9, 0.4, 0.6 * flicker);
            gizmos.sphere(
                Isometry3d::from_translation(Vec3::new(
                    pos[0] + body.visual_radius * 1.2,
                    pos[1] + body.visual_radius * 0.5,
                    pos[2],
                )),
                0.03 + (colony.population as f32 / 5000.0).min(1.0) * 0.03,
                colony_color,
            );

            // Light-delay connection arc back to Earth (pulsing)
            let earth_pos = Vec3::ZERO;
            let colony_pos = Vec3::new(pos[0], pos[1], pos[2]);
            let mid = (earth_pos + colony_pos) / 2.0 + Vec3::Y * 0.5;
            let sync_alpha = 0.05 * flicker;
            let sync_color = Color::linear_rgba(0.3, 0.7, 1.0, sync_alpha);
            gizmos.line(earth_pos, mid, sync_color);
            gizmos.line(mid, colony_pos, sync_color);
        }
    }
}

/// Update celestial body mesh positions to match their calculated orbital positions.
/// This keeps meshes in sync with the gizmo orbit rings and camera focus targets.
pub fn celestial_body_update_system(
    time: Res<Time>,
    mut query: Query<(&CelestialBodyMesh, &mut Transform)>,
    mut camera: ResMut<sol_atlas_bevy::camera::OrbitalCameraConfig>,
    bodies_res: Res<SolarSystemBodiesRes>,
) {
    let t = time.elapsed_secs();
    let bodies = &bodies_res.0;

    for (body_marker, mut transform) in &mut query {
        if body_marker.body_index >= bodies.len() {
            continue;
        }
        let body = &bodies[body_marker.body_index];
        let pos = sol_atlas_core::solar_system::body_position(body, t);

        // Update position, preserve existing scale
        transform.translation = Vec3::new(pos[0], pos[1], pos[2]);

        // Spin around the body's own (tilted) axis (2026-07-26 fix: no
        // body ever rotated on its own axis before this; only its orbital
        // position updated). Spin first (local Y), then tilt that whole
        // frame -- see body_spin_angle's doc comment for why. Also applies
        // mesh_pole_to_world_up() (2026-07-26, same-day follow-up fix):
        // Bevy's UV-sphere mesh puts its real geometric poles on local Z,
        // not Y (see that function's doc comment), so every body here was
        // actually spinning around its own equator, not its poles, until
        // this correction was added.
        let spin_angle = sol_atlas_core::solar_system::body_spin_angle(body, t);
        let tilt = Quat::from_rotation_z(body.axial_tilt_deg.to_radians());
        transform.rotation = tilt * Quat::from_rotation_y(spin_angle) * mesh_pole_to_world_up();

        // Keep the camera locked onto a focused body as it orbits (this
        // function's own doc comment already claimed to do this --
        // "camera focus targets" -- but nothing here actually did until
        // 2026-07-26; look_offset was only ever set once, at the moment
        // the F-key was pressed, so a focused planet visibly drifted out
        // of frame as it continued to orbit).
        if camera.focused_body.as_deref() == Some(body.name.as_str()) {
            camera.look_offset = Vec3::new(pos[0], pos[1], pos[2]);
        }
    }
}

/// Builds the same plain `StandardMaterial` `setup_globe_view` gives each
/// planet body at spawn time (real texture, subtle emissive, unlit). Kept
/// here so `planet_holographic_focus_system` can rebuild an exact match
/// when reverting a body out of holographic focus, without needing to have
/// cached the original material handle from spawn time.
fn standard_planet_material(
    body: &sol_atlas_core::solar_system::CelestialBody,
    texture: Handle<Image>,
) -> StandardMaterial {
    if body.is_sun {
        StandardMaterial {
            base_color: Color::linear_rgba(1.0, 0.95, 0.85, 1.0),
            base_color_texture: Some(texture),
            emissive: LinearRgba::new(20.0, 14.0, 4.0, 1.0),
            unlit: true,
            ..default()
        }
    } else {
        StandardMaterial {
            base_color: Color::linear_rgba(0.9, 0.9, 0.9, 0.85),
            base_color_texture: Some(texture),
            emissive: LinearRgba::new(0.08, 0.08, 0.08, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            ..default()
        }
    }
}

/// Builds a holographic treatment of a planet body, matching the same
/// Fresnel/scanline/coastline-outline aesthetic Earth's own globe uses in
/// `setup_globe_view` (2026-07-26, "the currently selected planet should be
/// holographic so features can be clearly displayed"). Unlike Earth's globe
/// (which has a dedicated night-lights emissive texture), planet bodies use
/// their own day texture for both the base color and the outline-detection
/// `surface_texture` -- the same texture the plain `StandardMaterial` path
/// already uses, so this is a material *treatment* change, not a new asset.
fn holographic_planet_material(
    texture: Handle<Image>,
) -> sol_atlas_bevy::holographic_material::HolographicMaterial {
    sol_atlas_bevy::holographic_material::HolographicMaterial {
        base: StandardMaterial {
            base_color: Color::linear_rgba(0.55, 0.68, 0.72, 0.9),
            base_color_texture: Some(texture.clone()),
            emissive: LinearRgba::new(1.5, 1.2, 0.6, 1.0),
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension: sol_atlas_bevy::holographic_material::HolographicExtension {
            fresnel_color: LinearRgba::new(0.0, 0.87, 1.0, 1.0),
            fresnel_power: 3.0,
            scanline_speed: 0.5,
            scanline_density: 20.0,
            hologram_alpha: 0.9,
            outline_color: LinearRgba::new(0.3, 1.0, 0.85, 1.0),
            outline_intensity: 1.4,
            outline_threshold: 0.12,
            surface_texture: texture,
            ..default()
        },
    }
}

/// Swaps the currently F-key-focused planet's material to the same
/// holographic treatment Earth's own globe uses, so its surface features
/// read clearly, and reverts it to plain PBR the moment focus moves
/// elsewhere (2026-07-26). Corona spheres (`is_corona: true`) are skipped —
/// they're already an additive glow effect, not a body surface.
pub fn planet_holographic_focus_system(
    mut commands: Commands,
    camera: Res<sol_atlas_bevy::camera::OrbitalCameraConfig>,
    bodies_res: Res<SolarSystemBodiesRes>,
    asset_server: Res<AssetServer>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut holo_materials: ResMut<Assets<sol_atlas_bevy::holographic_material::HolographicMaterial>>,
    not_focused: Query<
        (Entity, &CelestialBodyMesh),
        (Without<HolographicFocused>, With<CelestialBodyMesh>),
    >,
    focused: Query<(Entity, &CelestialBodyMesh), With<HolographicFocused>>,
) {
    let bodies = &bodies_res.0;

    // Bring the newly-focused body into holographic dress.
    for (entity, body_marker) in &not_focused {
        if body_marker.is_corona || body_marker.body_index >= bodies.len() {
            continue;
        }
        let body = &bodies[body_marker.body_index];
        if camera.focused_body.as_deref() != Some(body.name.as_str()) {
            continue;
        }
        let texture: Handle<Image> = asset_server.load(format!("textures/{}", body.texture));
        let holo = holo_materials.add(holographic_planet_material(texture));
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(holo))
            .insert(HolographicFocused);
    }

    // Revert any body that lost focus back to its plain material.
    for (entity, body_marker) in &focused {
        if body_marker.body_index >= bodies.len() {
            continue;
        }
        let body = &bodies[body_marker.body_index];
        if camera.focused_body.as_deref() == Some(body.name.as_str()) {
            continue;
        }
        let texture: Handle<Image> = asset_server.load(format!("textures/{}", body.texture));
        let standard = standard_materials.add(standard_planet_material(body, texture));
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<sol_atlas_bevy::holographic_material::HolographicMaterial>>()
            .insert(MeshMaterial3d(standard))
            .remove::<HolographicFocused>();
    }
}

/// Return to gameplay when Escape is pressed in globe view.
pub fn globe_input_system(kb: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<GamePhase>>) {
    if kb.just_pressed(KeyCode::Escape) {
        next.set(GamePhase::Playing);
    }
}

/// Tear down globe view — despawn all atlas entities, restore 2D camera.
pub fn cleanup_globe_view(
    mut commands: Commands,
    atlas_entities: Query<Entity, With<AtlasEntity>>,
    hidden_cameras: Query<Entity, With<Camera2d>>,
    hidden_huds: Query<Entity, With<crate::systems::rendering::HudText>>,
) {
    for entity in atlas_entities.iter() {
        commands.entity(entity).despawn();
    }

    for entity in hidden_cameras.iter() {
        commands.entity(entity).insert(Visibility::Visible);
    }
    // Restore dungeon HUD
    for entity in hidden_huds.iter() {
        commands.entity(entity).insert(Visibility::Visible);
    }

    // Restore dungeon background color and remove atlas data
    commands.insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.04)));
    commands.remove_resource::<AtlasData>();

    info!("[atlas] Returned to dungeon");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for the 2026-07-26 "rotating around the equator
    /// instead of the poles" bug: composing `tilt * spin` without
    /// `mesh_pole_to_world_up()` sweeps the mesh's real Z-pole through a
    /// great circle instead of holding it near the intended world-space
    /// axis. This test would have caught the bug even though the
    /// quaternion *composition* looked correct in the abstract -- the real
    /// defect was in the mesh generator's coordinate convention, which pure
    /// code review missed.
    #[test]
    fn mesh_pole_stays_near_world_axis_across_full_spin() {
        let tilt = Quat::from_rotation_z(23.44_f32.to_radians());
        let intended_axis = tilt * Vec3::Y;
        let mesh_local_pole = Vec3::new(0.0, 0.0, 1.0); // Bevy UV-sphere's real geometric pole

        for i in 0..36 {
            let spin_angle = i as f32 / 36.0 * std::f32::consts::TAU;
            let rotation = tilt * Quat::from_rotation_y(spin_angle) * mesh_pole_to_world_up();
            let world_pole = rotation * mesh_local_pole;
            let angle_off = world_pole.angle_between(intended_axis);
            assert!(
                angle_off < 0.01,
                "spin_angle={spin_angle}: mesh pole drifted {angle_off} rad from intended axis"
            );
        }
    }
}
