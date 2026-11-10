# WORLD-STATE-TO-PIXEL-001 — one world, multiple evidence-preserving views

**Status:** proposed benchmark contract; not an implementation or performance result.  
**Owner:** Symtropy rendering / simulation boundary.  
**Parent issue:** [#4468](https://github.com/Luminous-Dynamics/symtropy/issues/4468)

## Thesis

Symtropy should not maintain one hidden mutable world for simulation and a second loosely synchronized world for presentation. It should execute one authoritative scenario and generate multiple read-only views from immutable, provenance-carrying observations.

```text
scenario + exact model/profile + deterministic execution
                         |
                         v
             immutable observation frame
                         |
       +-----------------+------------------+
       |                 |                  |
       v                 v                  v
 cinematic / PBR   engineering / debug   accessibility /
 Solari / raster   units / fields /       low-power
                   contacts / uncertainty
       |                 |                  |
       +-----------------+------------------+
                         |
                         v
                receipt-bound pixels

A view may change what a person sees.
A view may not silently change what the simulation did.
```

This gives us a falsifiable bridge between AAA presentation and scientific/engineering simulation. A photorealistic frame is not proof of physical accuracy; a physics trace is not proof of visual quality.

## Current foundations and ownership

Do not start parallel versions of systems already under active development:

| Existing work | Reuse it for |
|---|---|
| [#1989 / #1990](https://github.com/Luminous-Dynamics/symtropy/pull/1990) | Bevy 0.20 migration and scene/light controls |
| [#2009](https://github.com/Luminous-Dynamics/symtropy/pull/2009) | Frozen PBR scene / frame-time receipts |
| [#1999](https://github.com/Luminous-Dynamics/symtropy/pull/1999) | Provenance-aware visual capture |
| [#2008](https://github.com/Luminous-Dynamics/symtropy/pull/2008) | Receipt-bound visual comparisons |
| [#2035](https://github.com/Luminous-Dynamics/symtropy/pull/2035) | Typed multiphysics and spectral-field visualization |
| [#2398](https://github.com/Luminous-Dynamics/symtropy/pull/2398) | Semantic screen-space LOD and error budgets |
| [#4096](https://github.com/Luminous-Dynamics/symtropy/issues/4096) | Interactive simulation fidelity / industry comparisons |
| [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460) | Sole canonical physics qualification corpus |
| [#1472](https://github.com/Luminous-Dynamics/symtropy/issues/1472) | Exact capability / approximation / unsupported-path census |
| [#4446](https://github.com/Luminous-Dynamics/symtropy/issues/4446) | Counterexample-guided differential physics testing |
| [#1172 / #1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467) | Engineering scenario/replay and read-only twin projection |

This work owns only the cross-layer mapping and invariance contract. It does not become a second physics corpus or model-credibility authority.

## Paradigm shifts

### 1. Rendering becomes a view, not a second world

A cinematic camera, debug overlay, LOD transition, path-traced frame, accessibility palette or low-power profile is a rendering decision over an immutable observation frame. It must not write transforms, forces, solver settings, clocks, controller inputs or event ordering back into simulation.

### 2. The material is a shared engineering object

A material should be more than a base-colour/roughness pair. Where the context of use requires it, a material record can reference separately versioned optical, thermal, acoustic, elastic, plastic, fracture and fluid-interaction models. The renderer uses the optical view; the solver uses the relevant mechanical/thermal views. Shared identity does **not** mean all properties are inferred from a texture or that an optical shader validates the constitutive model.

### 3. Fidelity is multidimensional and task-relative

Do not use one global LOD/fidelity scalar. A component can be visually coarse but remain solver-critical; a camera can be cinematic while an inspection overlay must preserve precise geometry and measurement uncertainty. Allocation uses projected error, task criticality and declared resource cost. Protected engineering detail cannot be silently demoted by camera distance or server load.

### 4. Physics-to-pixel becomes an engineering instrument

Contacts, impulse vectors, constraint residuals, stresses, thermal fields, spectral bands, trajectories and uncertainty should be explorable from the same typed observation frame. Overlays preserve units, reference frames, model-time, source class and uncertainty. Unknown is not zero; synthetic is not measured.

### 5. Visual regression becomes a scientific experiment

Freeze scene generator, assets, simulation tick/seed, camera, lighting, renderer profile, GPU adapter, driver, backend and resolution. Use receipt-bound captures, perceptual metrics and blind review. Same-adapter byte diffs are useful for regressions; cross-adapter byte equality is not a default requirement.

### 6. Physics superiority becomes an evidence frontier

There is no single simulator that is best for every domain. Symtropy should select and orchestrate a solver portfolio, expose exact model paths and couple domains with explicit time/work/energy contracts. Compare cost at matched error and matched physical semantics, not raw steps/second or feature checklists.

## V0.1 contracts

### Immutable observation frame

Every claim-bearing frame should bind:
- scenario/run, source commit, model/profile, feature set and execution backend;
- initial-state commitment, simulation tick and model-time convention;
- stable entity/component identity and coordinate frame;
- typed quantity, unit and value representation;
- source class: measured, calibrated measurement, derived, inferred, synthetic or unavailable;
- uncertainty/bounds and source/currentness references when available.

Projection may preserve an incomplete frame, but it must expose missing or mismatched semantics. It must not insert gameplay-friendly defaults into an engineering claim.

### Render view receipt

Each rendered output should bind:
- observation-frame identity and simulation state digest;
- scene/asset manifest and content digests;
- view identity, camera/projection, resolution and colour/tonemapping configuration;
- renderer/shader versions and quality profile;
- GPU adapter, driver, API/backend, OS and toolchain;
- capture timestamp, image byte digest and dimensions;
- any approximations, unsupported layers or dropped presentation detail.

### Read-only invariant

For the same deterministic scenario and simulation profile, compare:
1. overlays off vs on;
2. cinematic vs engineering camera;
3. high vs low visual quality;
4. full-detail vs semantic LOD;
5. raster vs path-traced renderer where both consume the same frame.

The canonical simulation receipt, event order and final-state digest must remain identical. A purposeful intervention must be represented as a separately declared simulation input and start a new run identity.

## Benchmark design

Reuse the frozen render fixtures and physics corpus. The cross-layer matrix consists of:

| Fixture | Scale / focus | Required evidence |
|---|---|---|
| `SCENE-LIGHT-01` | room, varied PBR materials, shadows and lighting | image quality, shadow stability, frame pacing, capture receipt |
| `HUMAN-LOCO-01` | 1 / 10 / 25 / 100 articulated actors | animation cost, contact quality, foot-slide and LOD review |
| `CITY-BLOCK-01` | 1k / 10k / 100k visible instances | streaming, batching, culling, semantic LOD and memory |
| `PHYSICS-OBS-01` | 100 / 1k / 10k bodies and contact overlays | physics digest invariance and overlay overhead |
| `ND-SLICE-01` | known-answer 4D geometry and moving hyperplane | slice topology, measurement labels, buffer/shader correctness |
| #460 oracle cases | analytically checkable and semantically admitted physics cases | solver error, energy/momentum, constraints, contact and event timing |

### Visual scorecard

Report, independently:
- linear/HDR radiometric error where a ground-truth reference exists;
- FLIP or another perceptual metric, with metric version and known limitations;
- temporal shimmer/flicker and motion stability;
- blinded preference / artifact review, including inter-rater agreement;
- frame-time p50/p95/p99, worst hitches, shader compilation and asset-streaming stalls;
- CPU/GPU time, memory/VRAM, power and thermal state.

Do not collapse these into one “realism score”. A perceptual metric is not a substitute for human review, and human preference does not validate engineering physics.

### Physics scorecard

Use analytical/manufactured solutions first. Then admit only matched semantic comparisons:
- native Symtropy vs Rapier for supported rigid-body cases;
- MuJoCo for articulated dynamics/contact where model semantics match;
- Project Chrono for suitable multibody, FSI and peridynamics work;
- specialist FEM/CFD/DEM references only on domain-specific, matched models.

Report global position/velocity/phase error, energy and momentum drift, contact penetration, constraint residual, event-localization error, conservation/work transfer, uncertainty, runtime and memory. Preserve unresolved disagreement when no independent oracle selects a correct result.

## Proposed target profiles (not measurements)

The machine-readable proposal is in [`benchmarks/world-state-to-pixel/v0.1/manifest.json`](../../benchmarks/world-state-to-pixel/v0.1/manifest.json).

Initial pilot targets:
- Balanced 1080p: p95 <=16.7 ms; p99 <=25 ms on the declared reference machine.
- Low-power 720p: p95 <=33.3 ms on the declared low-power profile.
- 8 GB GPU profile: <=6 GB measured GPU allocation for the balanced fixed scene.
- 1k-entity overlay: <=1 ms p95 CPU overhead or <=5% of frame budget, whichever is smaller; recalibrate only with a measured baseline.
- Zero presentation-induced simulation receipt/event-order changes.
- Zero silent loss of quantity units, reference frames, source class, time or uncertainty.
- No hidden demotion of critical engineering geometry or protected entities.

These are proposed gates, not measured results or claims that Symtropy currently meets them.

## Industry reference lanes

### Visual

- **Unreal Engine 5.8** is a high-end visual/authoring reference: it offers a mature real-time rendering and content workflow, with continued work on world partition, terrain, animation, streaming and high-fidelity production tools. Compare matched output and cost; Symtropy should not claim parity in editor or content ecosystem from a renderer benchmark.
- **Bevy 0.20.1** is the Rust-native base. Bevy 0.20 adds Solari improvements (real-time path tracing), WESL shader tooling, mesh-shader APIs, sprite materials and a pan-orbit camera. Solari's light-type and platform support must be recorded per exact profile.
- **Godot** is an open-source engine comparator where a feature-equivalent 3D workflow can be defined. Do not count unmatched features as a losing benchmark result.

### Physics / engineering

- **MuJoCo** is a serious articulated/contact reference. Its integrator portfolio includes semi-implicit Euler, implicitfast, implicit, discrete and RK4. Compare solver/profile and model semantics, not just engine defaults.
- **Project Chrono 10.0** offers multibody and vehicle simulation plus FSI alternatives (SPH and TDPF), peridynamics, sensor and checkpointing improvements. Use the matching Chrono modules where Symtropy does not implement an equivalent solver.
- **Rapier** remains a useful Rust-native rigid-body comparator, but it is not an oracle for physical truth.
- **NASA-STD-7009B** and the ASME V&V standards family are useful references for model credibility, verification, validation and uncertainty discipline.

References:
- [Bevy 0.20 release notes](https://bevy.org/news/bevy-0-20/)
- [Bevy 0.19 → 0.20 migration guide](https://bevy.org/learn/migration-guides/0-19-to-0-20/)
- [Unreal Engine 5.8 release notes](https://dev.epicgames.com/documentation/en-us/unreal-engine/unreal-engine-5-8-release-notes)
- [Unreal Engine 5.8 available](https://www.unrealengine.com/news/unreal-engine-5-8-is-now-available)
- [MuJoCo integrators and computation](https://mujoco.readthedocs.io/en/stable/computation/)
- [Project Chrono 10.0 release](https://projectchrono.org/news/)
- [NASA-STD-7009B](https://standards.nasa.gov/standard/NASA/B/1/NASA-STD-7009B)
- [ASME Verification, Validation and Uncertainty Quantification standards](https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty)

## Implementation sequence

1. Freeze the V1 observation-frame and render-view receipt schema; add adversarial schema fixtures for unknown, stale, synthetic and unit/frame-mismatched values.
2. Extend the existing capture and visual-diff path; do not fork it.
3. Add a first same-snapshot capture set (cinematic, engineering, accessibility and low-power).
4. Add view-invariance tests that compare exact simulation receipts/event logs.
5. Compose field visualization and 4D slice fixtures with their analytical known answers.
6. Integrate with #460 only by reference; rejected semantic pairs remain unscored.
7. Publish a pilot report on declared machines before claiming any SOTA advantage.

## Claim ceiling

A passing fixture proves only the exact invariant or metric it checks. It does not establish physical truth, model credibility outside the declared context of use, certification, general visual superiority, or performance on untested hardware.
