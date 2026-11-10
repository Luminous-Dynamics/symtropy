# Bevy 0.20 workspace migration

**Status:** in progress on `migration/bevy-0.20-workspace-v1`.  
**Target:** Bevy 0.20.1, one Bevy version across the entire standalone workspace.  
**Migration PR:** [#4472](https://github.com/Luminous-Dynamics/symtropy/pull/4472) (draft; exact-head qualification in progress).

## Why this is a coordinated change

Bevy 0.20.1 is published (28 October 2026) and declares the 0.20 API line. The previous production workspace was on Bevy 0.19 with Rust 1.96.0. The Bevy 0.20.0 manifest declares Rust 1.97.1 as its minimum, so manifests, lockfile, local toolchain and CI must move together.

The isolated Bevy 0.20.0 renderer probe in [PR #2012](https://github.com/Luminous-Dynamics/symtropy/pull/2012) and its October 18 workflow run establish that the new renderer API can compile in isolation. They do **not** establish workspace compatibility.

## Current compatible release set

| Dependency | Target | Compatibility evidence |
|---|---:|---|
| `bevy` and Bevy subcrates | 0.20.1 / 0.20 line | [Bevy 0.20.1 package metadata](https://docs.rs/crate/bevy/0.20.1) |
| `lightyear` | 0.31 | [Lightyear Bevy compatibility table](https://github.com/cBournhonesque/lightyear#supported-bevy-version) |
| `bevy_egui` | 0.43.1 | [0.43.0 Bevy 0.20 change and 0.43.1 fix](https://docs.rs/crate/bevy_egui/latest/source/CHANGELOG.md) |
| `bevy-inspector-egui` | 0.38 | [0.38 dependency metadata](https://docs.rs/crate/bevy-inspector-egui/latest) |
| `bevy-tokio-tasks` | 0.20 | [compatibility table](https://docs.rs/crate/bevy-tokio-tasks/latest) |

This release inventory proves that compatible published versions exist. The Symtropy adapter source still needs to compile and pass tests against those versions.

## Changes in this migration branch

- Updated direct Bevy dependencies across root, launcher, app demos, bridges, integrations, domain crates and distribution crates.
- Updated Lightyear integration from 0.28 / Bevy 0.19 to 0.31 / Bevy 0.20.
- Updated `bevy_egui`, `bevy-inspector-egui` and `bevy-tokio-tasks` constraints to their Bevy 0.20-compatible releases.
- Updated `rust-toolchain.toml` and main CI from Rust 1.96.0 to 1.97.1.
- Added a migration-only workflow that lets Cargo resolve and commit the lockfile, then runs the one-Bevy invariant, formatting, strict Clippy, core tests and full workspace check.

The lockfile remains a Cargo-generated artifact. It must not be hand-edited.

## First source migration slice — typed ShaderBuffer API

The root telemetry bridge was updated for Bevy 0.20's `ShaderBuffer` API in `src/systems/telemetry.rs`:
- initialization now passes an owned, typed `Vec<NodeTelemetryGpu>` to `ShaderBuffer::new`;
- per-frame updates use `clear` + `extend_from_slice` instead of the removed `set_data` call;
- an empty telemetry set retains a zero-filled minimum-size binding, avoiding a zero-byte GPU storage binding without inventing a node or observation.

This matches Bevy 0.20's documented typed `AlignedVec` data and explicit buffer-size contract. It remains subject to exact-head compilation and runtime validation.

## First source migration slice — WESL 4D slicing

The active 4D slicing material was migrated in this branch:
- `assets/shaders/nd_slicing.wgsl` → `assets/shaders/nd_slicing.wesl`;
- `NdSlicingExtension::fragment_shader()` now loads `shaders/nd_slicing.wesl`;
- naga_oil `#import` / `#ifdef` / `#{MATERIAL_BIND_GROUP}` syntax was translated to WESL imports, `@if` conditionals and `constants::MATERIAL_BIND_GROUP`;
- zero-thickness slicing now avoids an edge-fade 0/0 division.

This change is source migration only until the Bevy 0.20 shader pipeline compiles and loads the asset on an exact-head runner. It does not prove shader rendering quality or physical correctness.

The wider asset inventory is now tracked in [RENDER-SHADER-ASSET-QUAL-001 #4475](https://github.com/Luminous-Dynamics/symtropy/issues/4475). That audit found:
- `holographic.wgsl` also uses legacy naga_oil preprocessor syntax; active callsites still need confirmation;
- `crt_consciousness.wgsl` uses a legacy `#import` directive and needs a usage audit;
- `TelemetryExtension` refers to `shaders/telemetry_deform.wgsl`, but no such asset exists in the repository tree;
- `spatial_hash_broadphase.wgsl` is plain WGSL without preprocessor directives and should not be renamed just because it ends in `.wgsl`.

Do not create a replacement telemetry shader until its vertex/fragment contract, bind group and storage-buffer semantics are recovered.

## Required source/API migration

Bevy's [0.19 → 0.20 migration guide](https://bevy.org/learn/migration-guides/0-19-to-0-20/) and [0.20 release notes](https://bevy.org/news/bevy-0-20/) call out several relevant changes:

1. WESL is now the primary shader language. Migrate Bevy's former extended-WGSL files and imports deliberately; keep plain WGSL where appropriate.
2. Audit typed shader-buffer storage, layout/alignment and explicit buffer sizing.
3. Migrate legacy 2D sprite/material paths to the 0.20 material APIs.
4. Check translucent/screen-space transmission because it is opt-in.
5. Audit lifecycle observers and bundle generic changes.
6. Audit pointer-event type changes and the remaining API moves in the migration guide.
7. Smoke-test custom render-bridge shaders; a silent fallback is a failure.

### Direct Naga dependency

The root manifest also declares `naga = 27`. Bevy 0.20's renderer has moved to the newer wgpu generation, so the direct Naga pin must be audited against actual source usage and the resolved graph. Do not blindly add a second Naga version or change shader APIs without inspecting consumers. The lockfile and source audit must report whether one or multiple Naga versions remain and why.

## Exit gates

- [ ] Every workspace and excluded demo Bevy consumer uses the approved 0.20 line where it is intended to build.
- [ ] Lightyear 0.31 integration and multiplayer demo compile in the same workspace.
- [ ] Lockfile resolves exactly one `bevy` package version, matching root manifest.
- [ ] All Bevy subcrates resolve to 0.20 and no duplicate Bevy ECS/render type graph is present.
- [ ] Companion plugins resolve the expected Bevy 0.20 versions.
- [ ] Rust 1.97.1 is pinned consistently across toolchain and workflows that build Bevy.
- [ ] WESL/custom shaders, telemetry buffers, 2D/3D/4D scene paths and capture scenes smoke-test.
- [ ] Format, strict Clippy, focused physics tests and full workspace check pass at one exact head.
- [ ] Receipt-bound PBR/render captures are available and simulation receipts are invariant across view/quality changes.

## Claim ceiling

Updating Cargo constraints is not a completed engine migration. A successful isolated probe is not a successful workspace build. A successful workspace build is not a rendering-quality result or physics validation. The PR remains migration-in-progress until exact-head evidence covers all gates above.
