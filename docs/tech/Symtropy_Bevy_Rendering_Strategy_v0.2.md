# Symtropy Bevy Rendering Strategy v0.2

**Status:** Draft v0.2  
**Date:** 2026-06-15  
**Engine:** Bevy-first, Rust-native  
**Purpose:** Define the recommended rendering direction, asset constraints, and visual pipeline for Symtropy.

---

## 1. Executive Recommendation

Symtropy should use a **stylized PBR rendering stack** in Bevy.

The goal is not generic realism. The goal is a legible, emotionally distinct, ecological sci-fi world where biospheric intelligences, civic systems, rituals, machines, habitats, and planetary atmospheres all feel connected by one visual language.

The recommended core stack is:

```text
Bevy 0.18.x stable
+ Bevy PBR / StandardMaterial
+ glTF / GLB asset pipeline
+ Forward+ as the default runtime path
+ Deferred / prepass selectively for dense-light scenes
+ Atmosphere + volumetric fog for biome identity
+ FullscreenMaterial post-processing for Symtropy perception effects
+ bevy_hanabi 0.18 for GPU particles, desktop-first
+ Optional Solari / raytraced lighting for cinematic or experimental builds only
```

**Main decision:** build the game around Bevy's stable PBR, glTF, atmosphere, VisibilityRange/HLOD, and particle ecosystem, with a controlled stylized layer on top. Treat ray tracing as an optional future/ultra/photomode path, not the baseline.

## v0.2 refinement summary

This revision applies the first review pass:

1. Upgrades `bevy_hanabi` from "candidate" to **confirmed compatible with Bevy 0.18**.
2. Adds an explicit **LOD Policy** based on pre-baked LODs and Bevy `VisibilityRange`.
3. Adds a **Not Included / Deliberate Non-Goals** section.
4. Adds **Known Hard Parts / Technical Traps** to the implementation phases.
5. Couples rendering decisions to the new **Symtropy Visual Vocabulary and Biome Palette Spec v0.1**.
6. Clarifies where Bevy's asset processor and file watching belong: useful for development and deterministic transforms, not a replacement for the Foundry registry.

---

## 2. Current Bevy Rendering Context

As of this draft, Bevy 0.18.x is the recommended stable baseline. Bevy 0.18 introduced stronger atmosphere/PBR integration, generalized atmospheric scattering media for non-Earth-like skies and foggy environments, and Solari improvements while still keeping Solari in the experimental/future-facing lane [R1].

Relevant current Bevy capabilities:

- Bevy's renderer is built on **wgpu**, which can target Vulkan, Direct3D 12, Metal, OpenGL, WebGL2, and WebGPU depending on platform.
- Bevy supports a standard PBR material model through `StandardMaterial`.
- Bevy supports glTF loading through its `bevy_gltf` crate.
- Bevy has optional Forward, Forward + Prepass, and Deferred rendering paths.
- Bevy supports `VisibilityRange`, which is explicitly described as hierarchical level of detail / HLOD and can crossfade high-poly, low-poly, and billboard/impostor entities by camera distance [R7].
- Bevy's asset processor can transform assets and can use `file_watcher` to automatically watch and reprocess assets during development [R8].
- Bevy 0.18 introduces `FullscreenMaterial`, making high-level fullscreen post-processing shaders easier to implement.
- `bevy_hanabi` 0.18 maps to Bevy 0.18 and supports a `3d` feature plus optional `serde`; note that `serde` is not WASM-compatible in Hanabi's docs due to `typetag` constraints [R6].

---

## 3. Visual Direction: Stylized PBR, Not Pure Photorealism

Symtropy should look like **ecological systems thinking made visible**.

That means:

- grounded physical materials, but not bland realism;
- strong silhouette language for species, robots, habitats, and governance spaces;
- controlled palettes per biome/faction/system;
- fog, atmosphere, bloom, spores, mist, emissive veins, and living material states;
- render layers that communicate meaning, not just decoration.

### Style Formula

```text
Physical plausibility
+ biological pattern language
+ civic-infrastructure readability
+ mythic ecological atmosphere
+ restrained UI/holographic overlays
= Symtropy visual identity
```

### Avoid

- random marketplace asset soup;
- over-glossy sci-fi metal everywhere;
- realism without governance/worldbuilding meaning;
- ultra-dark scenes where ecological detail is unreadable;
- excessive post-processing that hides gameplay information.

### Not included / deliberate non-goals

These are decision records, not temporary preferences:

| Not Doing | Reason |
|---|---|
| Generic gray space-station interiors | Symtropy is ecological/civic, not default industrial sci-fi. |
| Grimdark desaturation as a baseline | Crisis states can be dark, but the world should remain readable and alive. |
| Ultra-dark dungeon-crawler lighting | Breaks accessibility, ecological detail, and public-space legibility. |
| Lens flare and bloom by default | Bloom/emissive effects must encode meaning, not hide information. |
| Unmanaged photogrammetry realism | Real-world scans need stylization or they will clash with the visual language. |
| Random neon cyberpunk UI | Holographic UI should be tied to consent, governance, sensor state, or technology branch. |
| One global color grade | Biomes need distinct atmospheric and palette identities. |
| Solari-only production visuals | Solari remains experimental/ultra/cinematic until proven stable enough for core gameplay. |

---

## 4. Recommended Runtime Rendering Modes

| Mode | Target | Rendering Path | Purpose |
|---|---:|---|---|
| Low / Web / handheld | WebGPU/WebGL2, low-end GPUs, Steam Deck-like targets | Forward+, lower texture sizes, minimal volumetrics | Maximum reach and stable performance |
| Medium | Main desktop baseline | Forward+ with selective prepass | Default playable target |
| High | Strong desktop GPUs | Hybrid Forward+ / Deferred, better shadows, more fog | Best normal experience |
| Ultra / Experimental | RTX-class / dev builds | Optional Solari experiments | Screenshots, photomode, future GI testing |

### Recommendation

The default shipping target should be **Medium**, with deliberate downgrade and upgrade paths. This prevents Symtropy from becoming dependent on experimental rendering features while still allowing beautiful showcase shots.

---

## 5. Forward+ vs Deferred in Symtropy

Symtropy should use both strategically.

### Use Forward+ by default for:

- outdoor biomes;
- transparent or semi-transparent organic materials;
- foliage, glass, membranes, water-like materials;
- characters and robots with stylized materials;
- scenes where MSAA or transparency matters more than many dynamic lights.

### Use Deferred / Prepass selectively for:

- dense interiors;
- civic hubs with many local lights;
- labs, tribunals, bathhouses, market interiors, machine gardens;
- scenes needing screen-space effects or more predictable light accumulation;
- heavy night scenes with lanterns, bioluminescence, signage, and machinery.

### Practical rule

```text
Forward+ is the default.
Deferred is a scene-level optimization / effect choice.
Solari is experimental.
```

---

## 6. Atmosphere, Fog, and Biome Identity

Atmosphere should be a first-class game-art system in Symtropy.

Bevy 0.18's generalized atmospheric scattering is valuable because Symtropy has many non-standard environments: living wetlands, ocean-mind reefs, desert spore networks, cloud-root ecologies, ice-shell ocean biospheres, volcanic habitats, sealed arcologies, and off-world settlements [R1].

### Recommended atmosphere presets

| Preset | Use Case | Visual Markers |
|---|---|---|
| Wetland Mind | biospheric intelligence, flooded civic zones | green-blue haze, low sun scattering, reflective water planes |
| Spore Desert | fungal/desert networks | amber sky, particulate fog, heat shimmer, long shadows |
| Cloud-Root Ecology | aerial habitats, cloud forests | pale scattering, vertical fog shafts, soft silhouettes |
| Ice-Shell Ocean | Europa-like or cryogenic habitats | high contrast, cyan volumetric shafts, low warmth |
| Civic Commons | public infrastructure spaces | clean air, soft ambient light, readable faces, low bloom |
| Blackout District | ration crisis, refusal zones | lantern light, limited fog, hard silhouettes |
| Red Bloom Zone | hazardous absorption ecology | red/pink atmospheric tint, biological emissive pulses |

### Implementation principle

Atmosphere should not be only a skybox. It should influence:

- biome mood;
- navigation readability;
- ritual moments;
- threat states;
- biospheric communication;
- accessibility presets.

---

## 7. Materials: PBR Base + Symtropy Material Extensions

Use `StandardMaterial` for most assets first. Extend only when there is a clear visual reason.

### Canonical material channels

| Channel | Required? | Notes |
|---|---:|---|
| Base color / albedo | Yes | Keep clean; do not bake lighting heavily into albedo |
| Normal | Yes for hero assets | Strong organic details without extra geometry |
| ORM / metallic-roughness-AO | Yes for final assets | Prefer packed maps where pipeline supports it |
| Emissive | Strongly recommended | Needed for biospheric signals, robotics, UI, rituals |
| Height / depth | Optional | Use for parallax/decal-rich surfaces when worth it |
| Alpha / transmission | Optional | Use carefully for membranes, water, glass, fungal skin |

### Symtropy-specific material families

1. **Living Infrastructure**  
   Concrete, wood, biopolymer, moss, algae, lichen, civic repair seams.

2. **Biospheric Intelligence Tissue**  
   Wet membranes, bark-skin, coral logic, fungal mats, neural roots, microbial bloom films.

3. **Robotics and Care Machines**  
   Worn metal, ceramic shells, rubber, glass, sensor membranes, field-repaired panels.

4. **Governance and Ritual Surfaces**  
   Consent seals, public ledgers, transparent walls, oath-stones, luminous civic markings.

5. **Hazard / Boundary Materials**  
   Red Bloom, quarantined tissue, contaminated mist, warning pigments, adaptive barricades.

---

## 8. Post-Processing Layer

Symtropy should build a small set of named post-process passes:

| Pass | Purpose | Use Carefully? |
|---|---|---:|
| Consent Haze | softens civic/ritual spaces | Yes |
| Biospheric Pulse | indicates world-system communication | Yes |
| Structural Stress | shows infrastructure overload | Yes |
| Fever Bloom | hazardous ecological contamination | Yes |
| Memory Trace | shows past ecological/civic events | Yes |
| Low-Oxygen / Low-Trust State | communicates systemic danger | Yes |

These should be **game-state readable**, not just aesthetic filters.

---

## 9. Particles, Spores, Mist, and Living Air

Symtropy needs living air: spores, pollen, ash, mist, dust, breath clouds, repair nanofibers, luminous plankton, and civic signal motes.

Recommended approach:

- use Bevy's built-in volumetric fog and light shafts for broad atmosphere;
- use GPU particles for dense active effects;
- use simple billboards/mesh impostors for cheap ambient particles;
- keep particle colors tied to biome/system state.

### Confirmed particle plugin

`bevy_hanabi` is confirmed compatible with Bevy 0.18. Hanabi's docs list `bevy_hanabi 0.18` paired with `bevy 0.18` and provide this 3D-only dependency form [R6]:

```toml
bevy_hanabi = { version = "0.18", default-features = false, features = ["3d", "serde"] }
```

Use the above for desktop/dev tooling. For web/WASM builds, test without `serde` because the Hanabi docs note that `serde` is not compatible with WASM due to `typetag` dependency constraints [R6].

Policy:

- `bevy_hanabi` is approved for desktop particle experiments in Phase B.
- The v0.2 renderer sandbox should include one spore, one mist, one ember/ash, and one civic signal particle effect.
- Particle systems must expose budgets: spawn rate, lifetime, max particles, render mode, and distance culling.

---

## 10. Asset Format Policy

Symtropy should standardize asset interchange early.

### Canonical formats

| Asset Type | Source Format | Game Format | Notes |
|---|---|---|---|
| 3D models | `.blend`, `.fbx`, `.obj`, source-specific | `.glb` / `.gltf` | Use GLB for packed runtime assets |
| Textures | `.png`, `.tif`, source files | `.ktx2`, `.png`, `.webp` | KTX2 for optimized builds; PNG/WebP for dev/fallback |
| Materials | Blender materials, source metadata | Bevy `StandardMaterial` / extensions | Preserve license/provenance metadata |
| Audio | `.wav`, `.flac` | `.ogg`, `.wav` | OGG for shipped ambience/music where appropriate |
| Icons/UI | `.svg`, `.png` | `.svg`, `.png`, texture atlases | Keep vector source where possible |

### Recommended runtime model package

```text
asset_id/
  source/
    original_downloads/
    provenance.yaml
  work/
    asset.blend
    textures_source/
  export/
    asset.glb
    textures_ktx2/
    thumbnail.png
    material_preview.png
    bevy.meta
  symtropy.yaml
```

---

## 11. Public Asset Library Integration

The rendering stack should be tied to the Symtropy Asset Foundry.

Every imported asset should receive:

```yaml
id: symtropy.biome.wetland_root_bridge_001
source_url: ...
creator: ...
license: CC0
sha256: ...
asset_type: model
render_family: living_infrastructure
material_family:
  - moss
  - wet_concrete
  - fungal_thread
visual_spec_version: symtropy_visual_vocab_v0.1
palette_id: wetland_mycelial
bevy_ready: true
lods:
  - lod0
  - lod1
  - lod2
texture_budget:
  low: 1024
  medium: 2048
  high: 4096
review_status: approved
```

### License gate still matters

Rendering automation should never erase provenance. Public asset ingestion must preserve license, source URL, creator, hash, transforms, and attribution requirements.

---

## 12. Performance Budgets

Initial budgets should be simple and testable.

| Target | FPS | Notes |
|---|---:|---|
| Low | 30 FPS | Reduced volumetrics, lower shadow map, smaller textures |
| Medium | 60 FPS | Default gameplay target |
| High | 60+ FPS | Better shadows, fog, post-process, higher density |
| Ultra | Variable | Screenshots, photomode, experimental raytracing |

### Budget rules

- Every biome gets a visual budget.
- Every particle system gets a maximum count and lifetime policy.
- Every public asset gets LOD and texture-size review.
- Every post-process pass gets an on/off debug toggle.
- Every atmosphere preset gets a performance profile.

---

## 12b. LOD Policy

Bevy does not remove the need for an asset-side LOD strategy. Symtropy should use **pre-baked LODs at export time**, then switch/fade them in Bevy using `VisibilityRange` / HLOD. Bevy docs describe `VisibilityRange` as a component for rendering high-polygon meshes near the camera and lower-polygon meshes farther away, with margins for gradual dithering/crossfade [R7].

### LOD generation policy

| Asset class | LOD strategy | Tooling |
|---|---|---|
| Hero characters / major robots | Manual LODs preferred | Artist-authored Blender files; preserve silhouette and rig quality |
| Mid-size props | Blender Decimate + manual cleanup | Generate LOD1/LOD2, inspect normals/material seams |
| Foliage / reeds / spores / small organics | Mesh LOD + billboard/impostor | Use LOD2/LOD3 billboards for distance |
| Civic set dressing | HLOD clusters | Merge or replace groups with simplified distant meshes |
| Terrain chunks | Separate terrain/streaming policy | Do not mix with prop LOD policy |
| UI/icons | No mesh LOD | Texture resolution variants only |

### Naming convention

```text
asset_id/
  export/
    asset_lod0.glb        # source-quality or gameplay hero mesh
    asset_lod1.glb        # 40-60% triangle target
    asset_lod2.glb        # 10-25% triangle target
    asset_lod3_billboard.glb or .png
    asset_lod_report.json
```

Inside GLB files, use stable names:

```text
symtropy_wetland_root_bridge_lod0
symtropy_wetland_root_bridge_lod1
symtropy_wetland_root_bridge_lod2
symtropy_wetland_root_bridge_billboard
```

### Bevy switching policy

Use a parent entity with one child per LOD. Each child receives its own `VisibilityRange`. `VisibilityRange` is not automatically propagated down to children, so the component must be applied to every LOD child [R7].

Example policy:

| Level | Distance range | Notes |
|---|---|---|
| LOD0 | 0-20m | hero/full mesh |
| LOD1 | 18-55m | crossfade overlap with LOD0 |
| LOD2 | 50-120m | simplified mesh |
| LOD3 / impostor | 110-250m | billboard or HLOD cluster |
| Culled | 240m+ | hidden unless landmark asset |

### LOD acceptance tests

- LODs share consistent origin and scale.
- Material slots remain stable across LODs.
- Normal maps and tangent data are valid.
- Silhouette remains readable at intended distance.
- Crossfade range avoids visible popping.
- `asset_lod_report.json` records triangle counts, material slots, texture budgets, and screenshots.

---

## 13. Bevy Feature Flags and Build Profiles

Use Bevy's high-level cargo feature collections and keep rendering choices explicit.

Recommended build profiles:

```text
symtropy_dev
  fast iteration, hot reload, debug tools, uncompressed assets allowed

symtropy_web
  WebGPU/WebGL constraints, compressed textures, minimal volumetrics, Hanabi without serde unless confirmed

symtropy_desktop
  default 3D, PBR, atmosphere, moderate post-processing, Hanabi 3D

symtropy_cinematic
  high quality shadows, fog, capture tools, optional Solari experiments
```

---

## 14. Tooling We Should Build

### 14.1 Render Validation Scene

Create a Bevy scene that loads each approved asset and renders it under several lighting environments:

- neutral studio lighting;
- wetland haze;
- desert scattering;
- night lantern scene;
- Red Bloom hazard tint;
- low-end mode.

This produces thumbnails and catches broken materials early.

### 14.2 Material Preview Grid

Every material family should have a preview card:

```text
albedo | normal | roughness | emissive | final shaded preview
```

### 14.3 Biome Lighting Harness

A small Bevy app that switches between atmosphere presets, shadow modes, fog densities, and post-processing passes.

### 14.4 Render Regression Screenshots

Use automated screenshots for key scenes so rendering changes can be compared visually over time.

### 14.5 LOD Review Harness

Add a debug camera path that moves from 5m to 250m and captures each asset's LOD transitions. Store transition screenshots next to `asset_lod_report.json`.

---

## 15. Minimal v0.2 Implementation Plan with Known Hard Parts

### Phase A: Foundation

Deliverables:

- Pin Bevy to latest stable 0.18.x.
- Build a clean Bevy render sandbox crate.
- Add glTF / GLB test asset loading.
- Add three base material families: civic, biospheric, robotic.
- Add one lighting rig and one atmosphere preset.

Known hard parts:

- Bevy/glTF sub-assets can be easier to manage if scenes, materials, and animation clips are named predictably.
- PBR material imports are not art direction; they need style-pass metadata.
- Debug-friendly asset hot reload is useful, but final assets must still be generated by the Foundry registry.

### Phase B: Symtropy Visual Identity

Deliverables:

- Add 6 atmosphere presets.
- Add emissive material conventions.
- Add fog and bloom rules.
- Add first post-processing pass.
- Add material thumbnail generation.
- Add `bevy_hanabi` 0.18 desktop particle effects.

Known hard parts:

- Atmospheric tint can easily destroy material color identity.
- Bloom can obscure gameplay-critical UI and consent/civic signals.
- Particles need distance culling and maximum counts from day one.
- Hanabi `serde` needs separate desktop-vs-web treatment because the docs flag WASM incompatibility [R6].

### Phase C: Asset Foundry Integration

Deliverables:

- Connect asset metadata to render tags.
- Generate thumbnails per imported asset.
- Generate LOD/material review reports.
- Export Bevy-ready bundles.

Known hard parts:

- Blender conversion logs must be treated as first-class artifacts.
- Unsupported Blender node graphs need baking or quarantine.
- Material-family remaps must be bounded, otherwise the style pass becomes destructive.
- LODs must preserve origin, scale, material slots, and readable silhouette.

### Phase D: Scene-Level Renderer Choice

Deliverables:

- Forward+ default.
- Deferred for one dense civic interior test.
- Compare performance and visual quality.
- Keep Solari isolated in an experimental branch.

Known hard parts:

- Forward and deferred paths can differ in transparency behavior and material feature support.
- Dense emissive interiors can be visually beautiful but expensive and noisy.
- Solari screenshots may set an art bar the baseline renderer cannot meet; label them experimental.
- The renderer decision should be per scene/biome profile, not a global ideology.

---

## 16. Recommended Decision Record

```text
Decision: Symtropy will use Bevy stable PBR as the baseline renderer.

Baseline: Bevy 0.18.x, StandardMaterial, glTF/GLB, Forward+ default.

Selective path: Deferred/prepass for dense-light interiors and special scenes.

Atmosphere: First-class visual system using Bevy atmosphere, scattering, fog, and custom presets.

Particles: bevy_hanabi 0.18 confirmed for Bevy 0.18 desktop particle experiments.

LOD: Pre-bake LOD0/LOD1/LOD2/LOD3 or billboard variants during Foundry export; switch with Bevy VisibilityRange/HLOD.

Post-processing: Use curated Symtropy perception/effect passes.

Ray tracing: Solari is experimental only until proven stable enough for production use.

Asset policy: Every renderable asset must carry provenance, license, material family, visual palette ID, LOD status, and texture budget.
```

---

## 17. Source Notes

- [R1] Bevy 0.18 release notes. https://bevy.org/news/bevy-0-18/
- [R2] Bevy GitHub releases. https://github.com/bevyengine/bevy/releases
- [R3] Bevy deferred rendering example. https://bevy.org/examples/3d-rendering/deferred-rendering/
- [R4] Bevy atmosphere example. https://bevy.org/examples/3d-rendering/atmosphere/
- [R5] Bevy `StandardMaterial` docs. https://docs.rs/bevy/latest/bevy/prelude/struct.StandardMaterial.html
- [R6] Bevy Hanabi 0.18 docs and compatibility table. https://docs.rs/crate/bevy_hanabi/latest
- [R7] Bevy `VisibilityRange` docs. https://docs.rs/bevy/latest/bevy/camera/visibility/struct.VisibilityRange.html
- [R8] Bevy asset processor docs. https://docs.rs/bevy/latest/bevy/asset/processor/index.html
- [R9] Bevy `Gltf` docs. https://docs.rs/bevy/latest/bevy/gltf/struct.Gltf.html
