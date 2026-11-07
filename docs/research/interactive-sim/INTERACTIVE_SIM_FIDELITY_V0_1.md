# Interactive simulation fidelity benchmark v0.1

Status: research specification; non-operational fictional content only.

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/4096

## Mission

Benchmark original fictional equipment, props and environment interactions as a unified interactive simulation. The objective is consistency across rendering, animation, contact, audio, input and networked replay—not real-world firearm design or calibrated weapon performance.

Weapon-like content, if present, must be clearly fictional and non-transferable to real construction or operation. No real firearm internals, dimensions, ammunition, propellants, pressure, materials, machining, ballistics or lethality optimization.

## Current platform baseline

- Bevy 0.20, released 8 October 2026: https://bevy.org/news/bevy-0-20/. The release highlights Solari real-time path tracing improvements, DLSS ray reconstruction integration, mesh shaders, WESL shader tooling, scene-system improvements and targeted ECS optimizations. Any performance number in upstream material is workload-specific, not a general engine ranking.
- Unreal Engine 5.8 Chaos Physics: https://dev.epicgames.com/documentation/unreal-engine/physics-in-unreal-engine. Epic documents rigid-body dynamics, destruction, networked physics, resimulation, vehicles, cloth, fluids and the Chaos Visual Debugger.
- Rapier determinism: https://rapier.rs/docs/user_guides/bevy_plugin/determinism/. Rapier is locally deterministic with identical initial state, insertion order, timestep, build, compiler and version; cross-platform determinism requires additional configuration and must be tested separately.
- Godot 4.6 may be added as a matched open-engine baseline where the same workload can be represented fairly.

Repository baseline correction, 7 November 2026: the Symtropy root `Cargo.toml` currently declares `bevy = "0.19"`. Therefore:
- Bevy 0.19 is the current root-manifest runtime baseline for this comparison.
- Bevy 0.20 is an upstream release and a migration candidate, not a feature already present in the current Symtropy root application.
- The benchmark must report three separate rows: current Symtropy/Bevy 0.19; a separately qualified Symtropy/Bevy 0.20 migration build; and upstream-engine references (Unreal 5.8 and Godot 4.6).
- Do not attribute Bevy 0.20 Solari, WESL, mesh-shader or ECS improvements to the current 0.19 project.
- Migration compatibility must be measured against the official [0.19-to-0.20 migration guide](https://github.com/bevyengine/bevy-website/blob/main/content/learn/migration-guides/0.19-to-0.20.md), including its breaking changes. A version bump alone is not evidence that scenes, shaders, plugins, networking, serialization or deterministic replay remain correct.

Do not infer Symtropy superiority from feature lists. The comparison must pin exact versions, assets, hardware, operating systems, drivers, scene complexity and quality settings.

## Paradigm shift

From disconnected model/animation/audio scripts to one versioned interaction state that drives:
- geometry and materials;
- animation state transitions;
- contact and collision feedback;
- sound cues and spatial audio;
- environment response;
- UI and player feedback;
- deterministic input/event replay.

A second shift is quality per unit of compute: compare perceptual and interaction quality at the same hardware budget rather than selecting the largest showcase scene.

## V0 content envelope

Use original fictional items such as:
- modular survey instrument;
- shield projector;
- energy tether;
- repair tool;
- fictional energy-effect device;
- environment-reactive tool.

Include a compact scene with static and dynamic props, varied materials and lighting, simple abstract effects, breakable non-weapon props, stateful animation/audio and a deterministic replay path. Add a small multiplayer prediction/resimulation test only after the local baseline is stable.

## Matched comparison protocol

1. Freeze a scene package, asset digests, input script, simulation cadence, hardware, OS, GPU driver, renderer and physics versions.
2. Run Symtropy/Bevy and Unreal workflows against equivalent scene complexity and target quality. Mark non-equivalent features instead of inventing a comparison.
3. Capture raw CPU/GPU/frame-time traces, memory, power/thermal behaviour, shader compilation, asset size and load times.
4. Record inputs and events and replay them. Separate same-machine exact replay from cross-platform error-bounded replay.
5. Compare image sequences and motion, not only still screenshots. Use blind human evaluation; PSNR/SSIM alone do not measure temporal or perceptual quality.
6. For networking, inject controlled RTT, jitter, loss and bandwidth conditions; report corrections, divergence, resimulation cost and player-visible interaction delay.
7. Score simulation correctness, visual quality, audio/animation coherence, responsiveness and player preference separately.

## Metrics and proposed targets

These are project targets, not measured results:
- 60 Hz operating target: p95 frame time <= 16.7 ms on the declared machine.
- 120 Hz stretch target: p95 frame time <= 8.3 ms, with p99 and worst-hitch duration reported separately.
- zero unreported dropped simulation steps and zero NaN/Inf state propagation.
- deterministic subset: 100% same-build replay hash agreement for frozen inputs and exact build profile.
- cross-machine replay: declared numeric tolerance and mismatch distribution; no blanket bit-identity requirement for floating-point physics.
- network stress profile: 0/50/100/150 ms RTT, 0/20/50 ms jitter, 0/1/3/5% packet loss; measure actual distributions.
- audio/animation state mismatch: zero on the frozen transition corpus.
- visual quality: full-resolution stills + moving sequences + at least 10 blind evaluators for pilot ratings; report inter-rater agreement and uncertainty.
- every run binds source commit, build, assets, renderer, physics, scene, input trace, device and evaluator digests.

## Industry comparison

Compare Bevy/Symtropy with Unreal Engine 5.8 Chaos on:
- rendering quality and temporal stability;
- authoring/editor maturity and asset pipeline;
- rigid-body and interaction fidelity for the declared fictional workload;
- replication, prediction, resimulation and debugging;
- build size, memory, CPU/GPU budget and load time;
- deterministic replay and provenance;
- platform portability and integration time.

A plausible advantage to test is Rust-native composition, evidence-bearing deterministic scenario replay and reuse of engineering digital-twin models. It is not assumed to beat Unreal in photorealism, editor maturity, asset ecosystem, animation tools or platform support.

## Acceptance

A claim of superiority is allowed only for a named metric, named workload, named device/build profile and independent matched result. No universal "better than AAA" statement from one demo.

## Non-goals

No real-world firearm design or manufacturing; no physical weapon performance calibration; no real-world ballistics; no military targeting or tactics; no lethal-effect optimization. This benchmark is for fictional game assets and general interactive simulation.
