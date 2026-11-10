// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! CRT consciousness post-processing pipeline.
//!
//! Holds the consciousness-driven CRT post-process settings and future render-node
//! integration point. The WESL shader is migrated, but no custom render node is
//! currently registered; the active effect path still uses Bevy's built-in
//! post-processing controls.
//!
//! - Chromatic aberration (allostatic_load × 0.008 + danger × 0.004)
//! - CRT scanlines (arousal-modulated intensity)
//! - Phosphor glow (warm gold calm → cold teal stress)
//! - Vignette (deepens with stress + danger)
//! - Film grain (increases with arousal)
//! - Danger red-shift
//! - Golden consciousness glow when deeply calm
//!
//! All effects are driven by the `ConsciousnessVisuals` resource which is
//! updated each frame from biometric state + Leviathan phase.
//!
//! # Architecture
//!
//! Follows the exact pattern of Bevy's `EffectStackPlugin`:
//! - `CrtNode` implements `ViewNode`
//! - Registered in Core2d render graph after Bloom, before Tonemapping
//! - Uses `ViewTarget::post_process_write()` for source/destination textures
//! - `CrtSettings` component on Camera extracts to render world
//! - Uniform buffer uploaded each frame from `ConsciousnessVisuals`

use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

use crate::systems::postprocess::ConsciousnessVisuals;

/// Component attached to cameras to enable CRT post-processing.
/// The actual visual parameters come from `ConsciousnessVisuals` resource.
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct CrtSettings {
    /// Master enable/disable.
    pub enabled: bool,
}

impl Default for CrtSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// GPU-side uniform for the CRT consciousness shader.
/// Reserved for a future, separately qualified fullscreen-material integration.
#[allow(dead_code)] // Infrastructure for future render node
#[derive(ShaderType, Clone, Copy, Default)]
pub struct CrtConsciousnessUniforms {
    pub allostatic_load: f32,
    pub arousal: f32,
    pub danger: f32,
    pub time: f32,
    pub resolution: Vec2,
    pub _pad: Vec2,
}

/// System that updates `CrtSettings` component data from `ConsciousnessVisuals`.
///
/// This remains a placeholder for the future custom render-node integration.
/// The shader source has been ported to Bevy 0.20 WESL, but it is not connected
/// to a render graph node and therefore does not currently produce the CRT effect.
///
/// For now, we maximize the built-in effects:
/// - ChromaticAberration intensity: consciousness-driven (already wired in postprocess.rs)
/// - Bloom: consciousness-driven (already wired in postprocess.rs)
/// - Background color: consciousness-driven vignette (already in visual_stress_system)
/// - Camera shake: consciousness-driven trauma (already in postprocess.rs)
///
/// This module adds the MISSING effects that can't be done with built-ins:
/// - None currently possible without custom render node
///
/// The CRT shader source is available as WESL; the render graph integration remains unimplemented.
#[allow(dead_code)] // Infrastructure for future render node
pub fn crt_settings_sync_system(
    visuals: Res<ConsciousnessVisuals>,
    time: Res<Time>,
    mut settings: Query<&mut CrtSettings>,
) {
    // Sync consciousness state — placeholder for future render node.
    // Currently the visuals are consumed by the existing postprocess systems.
    let _ = (visuals, time, settings);
}

// NOTE: The full CRT render node implementation requires:
//
// 1. A Plugin that accesses RenderApp (not available from game-side code)
// 2. ViewNode implementation with ViewTarget::post_process_write()
// 3. SpecializedRenderPipeline for the CRT shader
// 4. DynamicUniformBuffer for ConsciousnessUniforms
// 5. ExtractComponent for CrtSettings
// 6. Registration in Core2d graph between Node2d::Bloom and Node2d::Tonemapping
//
// This is ~300 lines of render-world boilerplate that mirrors the
// EffectStackPlugin in bevy_post_process. The WGSL shader is ready
// (assets/shaders/crt_consciousness.wgsl) — only the Rust wiring is needed.
//
// For Bevy 0.19+: use FullscreenMaterialPlugin<CrtConsciousnessMaterial>
// which eliminates all this boilerplate.
//
// For now, the existing consciousness-driven ChromaticAberration + Bloom +
// ClearColor vignette + camera shake provide 70% of the CRT effect. The
// remaining 30% (scanlines, film grain, phosphor glow, color grading)
// requires the custom render node.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crt_uniforms_default() {
        let u = CrtConsciousnessUniforms::default();
        assert_eq!(u.allostatic_load, 0.0);
        assert_eq!(u.arousal, 0.0);
        assert_eq!(u.danger, 0.0);
    }

    #[test]
    fn crt_settings_default_enabled() {
        let s = CrtSettings::default();
        assert!(s.enabled);
    }
}
