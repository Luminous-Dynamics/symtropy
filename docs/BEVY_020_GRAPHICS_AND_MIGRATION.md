# Bevy 0.20 Graphics Program and Migration Gate

**Status:** active planning + first scene-quality change; the workspace has **not** been upgraded to Bevy 0.20 yet.
**Reviewed:** 2026-10-18

## The paradigm shift

Treat rendering as a set of explicit, replaceable views over one authoritative simulation—not as the simulation itself.

1. **One world, multiple views.** The same immutable simulation observations can drive a cinematic view, an engineering/debug view, an accessibility view, or a low-power view. Changing cameras, materials, overlays, or rendering quality must not change physics, control, or governance outcomes.
2. **Evidence-aware rendering.** Distinguish observed, derived, inferred, uncertain, stale, and unavailable data through labels and shapes as well as colour. A polished visualization is not validation evidence. Source/time/frame/provenance stay attached to data.
3. **Physics-to-pixel observability.** Force vectors, contact normals, constraints, trajectories, energy ledgers, and N-dimensional slices become first-class tools for understanding the world. Make them optional render adapters over read-only observation contracts.
4. **Budgeted fidelity.** Render quality is a profile with explicit time and memory budgets. The system gracefully reduces shadow quality, LOD, animation update rate, particle density, or resolution before allowing unbounded resource use.
5. **Reproducible graphics.** Every visual test binds scene data, assets, simulation tick/seed, camera, render configuration, adapter, dimensions, and commit. Compare like with like; driver-dependent image changes are not automatically regressions.

## Current foundations

- `symtropy-bevy-scene` already owns camera/light/clear-colour setup. This branch adds explicit sun-shadow configuration and a shadow-aware secondary-light helper.
- `symtropy-render-bridge` already projects 2D/3D/4D physics and exposes slicing/telemetry-material scaffolding. Improve this bridge before creating another renderer.
- `symtropy-demo-capture` already schedules screenshots and checks for nonempty files. Extend its outputs into a versioned capture receipt rather than replacing it with another capture system.
- The existing roadmap calls for a replayable 4D/state-coupled hero scene with Phi/metric controls, contact and energy overlays, and capture support. Use that as the flagship benchmark.

## Bevy 0.20: verified facts and blockers

Bevy 0.20 was released on **2026-10-08**. The official release highlights improved Solari path tracing, WESL shader support, custom sprite materials, extended 2D materials, and a built-in pan-orbit camera. Read the official [release notes](https://bevy.org/news/bevy-0-20/) and [0.19 → 0.20 migration guide](https://bevy.org/learn/migration-guides/0-19-to-0-20/) while migrating.

This workspace remains on Bevy 0.19. Do not mass-replace versions without updating the lockfile, adapting source APIs, and preserving the repository's single-Bevy-version invariant.

**Toolchain gate:** Bevy 0.20.0's own root manifest declares `rust-version = "1.97.1"`, while the production Symtropy CI workflow pins Rust 1.96.0. The full migration must explicitly update and review `rust-toolchain.toml`, CI toolchains, cache keys, and any downstream MSRV promises before a 0.20 lockfile can be qualified. A separate probe under `research/bevy-020-graphics-probe` tests 0.20 on Rust 1.97.1 without changing the production workspace version set.

### Ecosystem compatibility update — 2026-11-10

The original Lightyear blocker is now removed by upstream releases. The published compatibility table now advertises **Lightyear 0.31 for Bevy 0.20**, while 0.28–0.30 remain on Bevy 0.19. Compatible companion releases are also published:

| Component | Bevy 0.20-compatible release | Evidence / migration implication |
|---|---:|---|
| Bevy | 0.20.1 | Current 0.20 patch release; use one exact lockfile resolution across the workspace |
| Lightyear | 0.31.0 | Upstream compatibility table explicitly maps 0.31 → Bevy 0.20 |
| bevy_egui | 0.43.1 | Release 0.43.0 updated to Bevy 0.20; 0.43.1 contains a follow-up fix |
| bevy-inspector-egui | 0.38.0 | Published dependencies target Bevy 0.20 |
| bevy-tokio-tasks | 0.20.0 | Version compatibility table maps 0.20.0 → Bevy 0.20 |

Sources checked 2026-11-10:
- [Bevy 0.20.1 on docs.rs](https://docs.rs/crate/bevy/latest)
- [Lightyear supported Bevy versions](https://docs.rs/crate/lightyear/latest)
- [bevy_egui 0.43.1 changelog and manifest](https://docs.rs/crate/bevy_egui/latest)
- [bevy-inspector-egui 0.38.0 dependency metadata](https://docs.rs/crate/bevy-inspector-egui/latest)
- [bevy-tokio-tasks 0.20.0 compatibility table](https://docs.rs/crate/bevy-tokio-tasks/latest)

This resolves **published-version availability**, not Symtropy adapter compatibility. The actual Lightyear integration still needs to be migrated and compiled against 0.31.0 with the multiplayer demo included. Do not update just the Lightyear manifest while the rest of the workspace remains on 0.19; that would introduce the exact duplicate-Bevy type graph the invariant is designed to prevent.

### Remaining production migration gates

1. Update the root and every workspace member/bridge that directly depends on Bevy or a Bevy sub-crate as one coordinated change. Pin all Bevy crates to the same 0.20 patch line.
2. Update `symtropy-lightyear` to Lightyear 0.31 and Bevy 0.20, then compile and test the multiplayer demo in the same workspace.
3. Update `bevy_egui`, `bevy-inspector-egui`, and `bevy-tokio-tasks` only where those packages are actually consumed; verify feature sets and no duplicate type graph.
4. Upgrade `rust-toolchain.toml` and CI from Rust 1.96.0 to at least Bevy 0.20's declared 1.97.1 MSRV. Reconcile cache keys and downstream MSRV promises.
5. Audit the direct `naga` dependency against Bevy 0.20's shader stack instead of retaining an accidental older Naga line.
6. Generate `Cargo.lock` with Cargo; never hand-edit it. Run the workspace invariant script and preserve the single-Bevy assertion.
7. Migrate WESL shaders, typed shader-buffer layouts, 2D sprite/material code, screen-space transmission settings, and the remaining official 0.19 → 0.20 API changes.
8. Run format, strict Clippy, focused crate tests, the complete workspace check, multiplayer tests, shader smoke tests, and receipt-bound visual captures at one exact migration head.

### Probe evidence — exact historical head

The isolated compatibility probe in [PR #2012](https://github.com/Luminous-Dynamics/symtropy/pull/2012) compiled Bevy 0.20.0 under Rust 1.97.1 without adding Bevy 0.20 to the production workspace. Its dedicated workflow run [39153632186](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39153632186) completed successfully on 2026-10-18. This proves that isolated probe only; it does **not** prove the production workspace, Lightyear adapter, or all custom shaders have migrated.

### Migration hotspots to inventory and qualify

- **WESL shaders:** inspect every custom render-bridge shader and its imports/bind groups; migrate syntax and shader entry points according to Bevy's guide. Keep a smoke test that loads each shader.
- **`ShaderBuffer`:** Bevy 0.20 changes typed buffer storage to use an aligned data container and explicit buffer size. Audit `TelemetryExtension` and all buffer construction/update paths.
- **2D rendering:** Bevy 0.20 migrates `Sprite` to `Mesh2d` + `SpriteMaterial`; check sprite sync, alpha/blend behavior, and custom materials.
- **Transmission:** screen-space transmission is opt-in for cameras; compare translucent scenes before/after.
- **API moves/renames:** audit `Tonemapping`/`DebandDither`, `MeshAabb::get_aabb`, `bevy_shape` and `bevy_curve` splits, flat pointer events, and the other entries in the official guide.
- **Companion crates:** update only to releases that explicitly target Bevy 0.20. The project currently references `bevy_egui 0.40`, `bevy-inspector-egui 0.37`, and `bevy-tokio-tasks 0.19`; discover/verify the compatible releases before changing the manifest.
- **Lockfile/toolchain:** generate and commit the lockfile with a 0.20-capable Rust toolchain, then run the full format/Clippy/test/check suite. Do not claim a migration pass from manifest edits alone.

## Benchmark corpus

Use deterministic procedural geometry and bundled, license-reviewed assets. Freeze scene generator versions and raw fixture digests.

| Fixture | Workload | What it proves |
|---|---|---|
| `SCENE-LIGHT-01` | Room, varied PBR materials, direct + ambient/environment lighting, shadow casters | Lighting quality, shadow stability, material/tonemapping regressions |
| `HUMAN-LOCO-01` | 1/10/25/100 rigged actors, fixed locomotion clips, fixed camera path | Animation cost, foot sliding/contact quality, LOD transition stability |
| `CITY-BLOCK-01` | Procedural urban block; scale visible instances from 1k to 100k | Batching, culling, LOD, memory and frame-time scaling |
| `PHYSICS-OBS-01` | 100/1k/10k bodies with contacts, constraints, trajectories, and selected entity | Overlay overhead and whether presentation stays separate from simulation |
| `ND-SLICE-01` | Fixed 4D fixture, moving hyperplane, cross-section edges + telemetry | Slice correctness, temporal stability, shader and buffer integrity |

Each run records: commit SHA; rustc and Cargo versions; OS; GPU adapter/driver/backend; display dimensions and scale; feature set; quality profile; asset manifest digest; seed/tick; warm/cold cache state; per-frame CPU/GPU times where supported; memory; dropped frames; shader/asset errors; capture paths and hashes.

### Candidate acceptance targets (not yet measured)

Use a reference machine with an RTX 2070 Laptop-class GPU and 8 GB VRAM as one mandatory profile, plus software/low-end and WebGPU-compatible profiles. Report results per adapter—never average away a failing target.

- **Balanced 1080p:** target p95 frame time ≤16.7 ms for the fixed room-lighting and 1–10 actor scenarios; p99 ≤25 ms. Treat this as a goal until the first baseline is measured, not a claim about today's build.
- **Low-power 720p:** target p95 frame time ≤33.3 ms on the declared low-power profile, with shadows/LOD/animation detail reductions visible and deterministic.
- **Memory headroom:** keep the balanced benchmark's total GPU allocation ≤6 GB on an 8 GB adapter, leaving headroom for desktop/compositor and asset variance.
- **Debug-overlay cost:** overlay-off vs overlay-on must be measured at 100/1k/10k entities. Initial target for 1k entities: added p95 CPU cost ≤1 ms or ≤5% of the frame budget, whichever is smaller. Revise only after measuring a baseline and record the rationale.
- **No simulation mutation:** for the same replay input, turning overlays on/off or changing camera/quality profile produces byte-identical simulation receipts/state digests under the declared determinism profile.
- **Capture integrity:** 100% of requested images exist, decode successfully, match expected dimensions, and have recorded digests. An image hash alone is not a perceptual quality metric.
- **Regression thresholds:** same adapter/backend should use stable image diffs or exact hashes where reproducible; cross-adapter comparisons use recorded tolerances and focused assertions (silhouette, bounding boxes, luminance bands, missing geometry), not an unconditional byte match.
- **Accessibility:** every critical state is understandable without hue alone; overlays remain readable at UI scale; motion/flash checks run on any animated visual theme.

## Reusable Bevy contributions: priority order

1. **Improve existing `symtropy-bevy-scene`**: quality-first shadow controls and later named low-power/balanced/quality presets. Do not add a separate generic scene framework yet.
2. **Extract `bevy-sim-observability` only after the contracts stabilize**: generic read-only observations + optional gizmo/render adapters. It must have no Symtropy or Mycelix dependency in its core and must never write simulation state.
3. **Upgrade `symtropy-demo-capture` into a visual-test receipt tool**: frozen fixture/config hashes and per-adapter baseline policy.
4. **Keep N-dimensional projection in `symtropy-render-bridge`** and add a demonstrator that makes 4D cross-section slicing explainable rather than merely decorative.

## Exit gates

- [ ] Lightyear 0.20 compatibility is resolved without weakening single-Bevy invariants.
- [ ] Cargo manifests and lockfile resolve only the approved Bevy version set.
- [ ] Every custom shader loads; no unreviewed fallback shader silently replaces it.
- [ ] Key workspace crates pass fmt, Clippy, focused tests, and workspace check at the exact migration head.
- [ ] Fixed reference scenes have an initial receipt-backed baseline on the declared hardware matrix.
- [ ] Overlay invariance and negative-control tests prove visualization is read-only.
- [ ] Benchmark thresholds are calibrated from results and do not masquerade as measured results before evidence exists.
