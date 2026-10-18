# Human Simulation and Animation Foundation v0.1

**Status:** implementation baseline and benchmark contract; not a claim of human-level fidelity.  
**Date:** 2026-10-18  
**Repositories:** Symtropy (embodied rendering/physics) + Symthaea (cognition/behavior)

## Decision

Build a measurable human-simulation stack by separating appearance, motion, motor control, cognition, and population/social behavior. These layers interact through explicit data contracts; they must not be collapsed into one “human-like” score.

The first increment adds an opt-in Bevy skeletal-animation driver for licensed glTF/GLB avatars. It maps semantic motion intent (idle/walk/run/talk) to animation-graph clips and cross-fades between them. It does **not** provide a human mesh, facial animation, inverse kinematics, root-motion extraction, or evidence of realistic human behavior. Those remain explicit follow-on work.

## Source audit

- The current Symtropy Bevy animation module defines robotic animation states and maps Phi/safety tiers to state labels. The visible system does not itself update a skinned human skeleton or play an animation clip.
- The existing Symtropy Bevy dependency intentionally enables a small subset of Bevy features. Skinned glTF playback needs animation, glTF, and PBR features enabled. Bevy 0.19 has been released; its official animated-mesh examples demonstrate skinned glTF playback, animation graphs, and cross-faded state transitions.
- Symthaea has a humanoid-control implementation and gait-oriented metrics such as cadence, stride length, foot clearance, asymmetry, and cost of transport. These evaluate control and locomotion, but do not substitute for visual character animation or population-level human validation.
- The psychology benchmark documentation explicitly warns that most existing tests validate synthetic architecture mechanics rather than human-normed task performance. Human simulation claims therefore require independent reference data.

## Paradigm shifts

1. **Intent-to-motion, not a clip menu.** Symthaea should emit semantic intent plus constraints and confidence; Symtropy turns that into feasible motor goals and visual motion. The animation state is an output of intent, not the source of cognition.
2. **Visual avatar and physical body are distinct but synchronized.** The render rig supplies skin, face, hair, clothing, hands, and animation. Physics/controller state supplies root motion, balance, contacts, collisions, and forces. Define one authoritative root transform to prevent visible foot sliding and physics drift.
3. **Physics-corrected motion.** Begin with author-provided mocap/animation clips and blend them, then add contact-aware correction and trajectory tracking. Do not invent a new motion generator until the baseline can replay and score existing motion.
4. **Individual differences are first class.** Height, proportions, mobility, speed, strength, attention, preferred distance, goals, habits, and relationships should be explicit model parameters—not random cosmetic skins.
5. **Evidence-grounded population behavior.** Validate individual trajectories and group-level distributions separately. A crowd can look plausible while producing incorrect flows, unsafe conflicts, or unrealistic social responses.
6. **Multi-rate simulation.** Run high-frequency animation and physics only where needed; update cognition and distant agents less frequently with explicit interpolation/LOD. Measure artifacts as well as throughput.
7. **Open benchmark before grand claims.** Use held-out real motion/trajectory data, fixed seeds, negative controls, ablations, and baselines. Publish raw aggregate metrics and exact asset/data versions.

## Layered architecture

- **Appearance/asset layer:** legally sourced, rigged glTF/GLB body mesh; PBR materials; consistent scale; skin weights; optional facial blend shapes, eye/teeth geometry, hair, clothing, hands, footwear, and LODs.
- **Animation layer (this PR):** named GLTF clips -> animation graph -> semantic motion states -> cross-fades. Missing optional clips fall back to a documented safe state. No asset is silently downloaded.
- **Motor/physics layer:** movement controller and Symtropy physics remain authoritative for root position, velocity, contacts, balance, and collision response. Root-motion extraction and contact correction are a separate adapter to be added and tested.
- **Cognition layer:** Symthaea supplies goals, plans, observed beliefs, uncertainty, interaction choice, and high-level motion intent. Keep its tick rate independent of render frame rate.
- **Social/population layer:** Mycelix identity/relationship data can supply stable, privacy-minimized identity and relationship references where needed; do not treat reputation tokens or Phi as direct measures of human character.
- **Evidence layer:** each experiment records commit SHA, configuration digest, seed, asset/license identifier, dataset version, run duration, failures, and metrics.

## Avatar asset requirements

Accept externally supplied, licensed glTF/GLB assets; do not bundle unreviewed models or motion-capture files. A production-quality “realistic person” needs more than a skeleton: believable proportions, skin and clothing materials, facial details, correct lighting, deformation, and suitable LODs. Choose assets only after checking redistribution and commercial-use terms.

**First demo candidate (not bundled):** [Maya 3D Person, Animated — locomotion collection](https://www.innerscene.com/tools/library/3d-parts/locomotion-collection-maya-3d-person-animated-1499b2fd) is listed by its publisher as a 20.3 MB CC0 GLB with named idle/walk/jog/run/carry/sit-stand clips. The publisher describes these as original authored periodic-pose animations, not motion-capture data. Use it as a loader/render/clip-switching smoke fixture only; do not treat its motion as human-reference ground truth. Re-check the asset page and library terms before adopting it, and record the downloaded file's SHA-256.

SMPL-X is a useful research body representation with articulated body, hands, and face, but its body-asset terms and its full model/software terms are not identical; review the exact license before shipping or redistributing any asset. AMASS provides more than 40 hours of motion-capture data from over 300 subjects and 11,000 motions, but access/use is subject to its dataset terms. Neither should be repackaged into this repository without a separate provenance/license decision.

## Benchmark contract

All scores must report dataset split, sample count, seed policy, confidence intervals, hardware, runtime, asset identifiers, and baseline commit. No benchmark passes by construction; include negative controls and independent scoring.

| Area | Metrics | Initial gate |
|---|---|---|
| Asset/runtime integrity | load success; missing meshes/materials/joints/clips; invalid node index | 100% of declared required assets load; missing required clips fail visibly rather than silently selecting a wrong clip |
| Animation transitions | transition latency; blend discontinuity; abrupt state changes; repeated/restarted clips | state changes take effect on the next scheduled animation update; valid blend duration remains within configured bounds |
| Motion fidelity | joint-angle error, trajectory error, pose jerk, foot sliding, self/interpenetration, ground-contact consistency | baseline first; then set tolerances from mocap noise and application needs |
| Gait/physics | fall rate, stance-foot drift, contact violations, stride length, cadence, symmetry, cost of transport | report per task and per subject; no aggregate score may hide falls or contact failures |
| Behavior | next-action prediction, goal success, calibration/Brier score, recovery after perturbation | beat scripted and rule-based baselines on held-out scenarios with uncertainty reported |
| Navigation/social | ADE/FDE plus collision/yielding/personal-space measures; queue/flow distributions; pairwise interactions | evaluate both individual trajectories and aggregate distributions against real observations |
| Visual quality | blinded pairwise preference; pose plausibility; facial/hand artifacts; silhouette/body-shape diversity | compare against baseline renders under identical camera, lighting, and resolution |
| Runtime | P50/P95 frame time, CPU/GPU time, memory, draw calls, active rigs, physics and cognition tick costs | establish a measured reference machine baseline before setting release gates |

### Candidate engineering targets (not yet achieved)

- Reference desktop target: 1080p and stable 60 FPS (16.67 ms/frame budget) with one close-up avatar and 32 mid-distance animated people on the reference GPU; record P95 and GPU memory, not only average FPS.
- Crowd stress target: 500–1,000 visible or near-visible residents using skeletal/animation LOD, batched material variants and reduced-rate distant updates; report frame time and simulation step time separately.
- Stance quality target: aim for less than 2 cm RMS root-relative stance-foot drift on a controlled walk test after correction; separately count ground penetration and missed contacts. Revisit the bound after measuring the selected motion-capture reference.
- Behavioral target: report held-out next-action accuracy/calibration and collision/yielding differences versus baselines; do not select an arbitrary global “human realism percentage.”
- Portability target: deterministic scenario seed and event replay for physics/decision traces where supported; do not claim cross-platform bitwise determinism for GPU or floating-point animation.

These are proposed gates, not measured results. The first report must include the actual baseline even if it misses every target.

## Implementation phases

### P0 — Foundation (this PR)
- Add the opt-in human-animation feature and a state-to-clip transition driver.
- Leave the default robotics feature set unchanged.
- Document the avatar/physics contract, license boundary, and benchmark protocol.

### P1 — First visible person (scene scaffold committed; render still unverified)
- `crates/bridges/symtropy-bevy/examples/human_avatar.rs` loads a user-supplied GLB from the Bevy `assets/` root, enumerates named clips, maps common idle/walk/run/talk aliases, binds the controller to the `AnimationPlayer` in the loaded scene, and supports keyboard state changes.
- No binary avatar is bundled. Re-check asset provenance/terms and record the asset SHA-256 before use.
- Run the documented cargo command and visually validate scale, skinning, materials, lighting, shadows, clip names, transitions, and failure behavior. Source presence is not proof that the model rendered.

### P2 — Physically credible movement
- Add root-motion policy (in-place clips versus extracted root motion), pelvis/root alignment, foot-contact detection, ground alignment, obstacle response, and physics-corrected motion.
- Evaluate against held-out AMASS or other properly licensed mocap sequences and motion-tracking baselines.
- Add reach/grasp/sit/stand/turn/crouch actions using explicit contact and joint limits.

### P3 — Expressive person
- Add facial expression and gaze, hand gestures, eye/head coordination, cloth/hair quality appropriate to hardware, and identity-preserving animation transitions.
- Evaluate blinded human preference and artifacts; visual fidelity does not prove cognitive or social fidelity.

### P4 — Symthaea integration
- Map cognition output to semantic action/intent with confidence and fallback behavior.
- Run ablations: scripted, rule-based, Symthaea cognition, and Symthaea cognition without memory/social models.
- Test false-belief tasks, conflicting goals, uncertainty, interruption, and learning from observation.

### P5 — Population scale
- Add multi-agent navigation, attention/visibility, crowd LOD, and calibrated interaction policies.
- Match observed distributions of trajectories, flows, queues, yielding, and interpersonal distance before increasing simulated population size.

## Local validation commands

Run locally from a fresh clone after obtaining the branch:

    cargo fmt --check
    cargo check -p symtropy-bevy --features human-animation
    cargo test -p symtropy-bevy --features human-animation
    cargo check -p symtropy-bevy --features human-animation --example human_avatar
    cargo run -p symtropy-bevy --features human-animation --example human_avatar

Place a licensed rigged GLB at `assets/humans/person.glb`, or set `SYMTROPY_HUMAN_AVATAR` to a path relative to the Bevy `assets/` directory. The example logs discovered clip names and mappings. The exact command output and a screenshot from the rendered scene must be retained as evidence.

Then run the visual scene with a legally sourced rigged GLB. Source presence or a queued workflow is not a passing result. Attach the exact commit SHA, command output, asset hashes, dataset provenance, and benchmark JSON to the validation record.

## References

- Bevy 0.19 release: https://bevy.org/news/bevy-0-19/
- Official Bevy skinned glTF example: https://github.com/bevyengine/bevy/blob/v0.19.0/examples/animation/animated_mesh.rs
- Official Bevy animation state/blending example: https://github.com/bevyengine/bevy/blob/v0.19.0/examples/animation/animated_mesh_control.rs
- AMASS motion-capture archive: https://amass.is.tue.mpg.de/
- SMPL-X body and license information: https://smpl-x.is.tue.mpg.de/
- SIGGRAPH 2025, PARC physics-based motion augmentation: https://www.siggraph.org/wp-content/uploads/2025/08/Conference-Papers.html
- ZMP-guided sampling for physically plausible character animation (August 2026): https://www.sciencedirect.com/science/article/abs/pii/S0097849326001263
- Symthaea humanoid metrics: https://github.com/Luminous-Dynamics/symthaea/blob/main/crates/domains/symthaea-humanoid/src/benchmarks.rs
- Symthaea psychology benchmark scope note: https://github.com/Luminous-Dynamics/symthaea/blob/main/crates/domains/symthaea-psych-bench/README.md
