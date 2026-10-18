# Human Motion Benchmark Protocol v0.1

**Status:** proposed protocol; targets below are not achieved results.  
**Date:** 2026-10-18  
**Scope:** Symtropy animation, locomotion and Symthaea motion-intent research.

## Purpose

Stop using "looks realistic" as a single subjective score. Report separate outcomes for motion fidelity, contact/physics, constraint following, behavioral utility, visual perception, latency, compute, and provenance. A system can do well on one axis and fail another.

This benchmark is designed to compare:
1. authored clip selection/blending (baseline available in the engine);
2. procedural or controller-based motion;
3. an external interactive generator such as ARDY, where its released code, weights, dataset, license, and hardware requirements permit a reproducible run;
4. a modular-skills/generative baseline inspired by MotionBricks, where a usable release and permitted assets are available;
5. the Symthaea intent + Symtropy constraint/physics stack;
6. hybrids in which a proven generator proposes motion while Symthaea plans and Symtropy checks/corrects it.

Do not claim that Symthaea itself synthesizes superior body motion unless the experiment isolates and measures its contribution.

## Dataset and asset governance

- Do not bundle third-party human body meshes, motion clips, weights, or training datasets until the applicable license and redistribution terms have been reviewed and recorded.
- AMASS is explicitly licensed for non-commercial research/education/artistic use and prohibits training methods or neural networks for commercial use under its published license. It is not an acceptable default for training a commercial product without a separate written license.
- Prefer (a) original motion capture with documented participant consent and rights, (b) assets explicitly licensed for the exact research and intended commercial use, or (c) synthetic scripted fixtures for the unit tests. A demo license is not automatically a training license. The optional Innerscene Maya locomotion GLB is currently listed as CC0 and is documented in [`assets/humans/README.md`](../../assets/humans/README.md); use it to smoke-test rendering and clip playback only, not as mocap ground truth.
- Keep source video, face identity, names, and sensitive metadata out of the public artifact by default. Publish derived traces only where consent, rights and re-identification risk are addressed.
- Every run must pin the rig/retarget map, animation clips, model/checkpoint, dataset version, split manifest, preprocessing, random seed, engine commit and configuration digest.
- Split by actor and recording session, not merely by adjacent frames. Avoid leakage of the same motion sequence into training and evaluation.

## Machine-readable trace format

The motion trace records observed/predicted trajectories; it is separate from the upstream [semantic intent schema](human-motion-intent-v0.1.schema.json), which records what an agent intends to do and with what confidence/provenance. Do not use intention correctness as a substitute for realized motion quality.

[`human-motion-trace-v0.1.schema.json`](human-motion-trace-v0.1.schema.json) defines the versioned world-space trace contract; [`fixtures/human-motion-trace-synthetic-v0.1.json`](fixtures/human-motion-trace-synthetic-v0.1.json) is a synthetic fixture only. JSON Schema cannot express all cross-array rules here: the Rust reader must reject mismatched sample lengths, non-increasing timestamps, non-finite parsed values, missing asset provenance on non-synthetic traces, incompatible rigs, and mismatched joint-name sets before computing scores. Compare prediction/reference traces only after an explicit alignment step.

## Task suite

Every scenario is executed against the same avatar, coordinate system, frame rate, physics settings and saved input trace where technically applicable.

| ID | Task | Primary measurements | Failure conditions |
|---|---|---|---|
| HM-01 | Idle, start, stop, walk/run transitions | transition latency, clip discontinuity, root speed error | NaN pose, controller state chatter, missing clip silently treated as success |
| HM-02 | Straight and curved locomotion | root ADE/FDE/max error, heading error, foot drift, cadence | major trajectory deviation or sustained stance sliding |
| HM-03 | Waypoints and path changes | endpoint error, path deviation, replanning delay | missed waypoint or uncontrolled discontinuity |
| HM-04 | Reach and object interaction | hand/end-effector position and orientation error, contact timing | interpenetration beyond configured tolerance, impossible reach |
| HM-05 | Sit/stand and level changes | support/contact sequence, balance proxy, joint limits | unsupported floating, severe collision or joint-limit violations |
| HM-06 | Conversation and gestures | gesture relevance annotation, overlap/interruption timing, gaze/pose measures where available | all agents use same canned response regardless of input |
| HM-07 | Two-person interaction | personal-space violations, path conflicts, response delay | repeated collision or non-responsive interaction |
| HM-08 | Crowd/LOD | collision rate, route/flow distributions, latency and memory by visible/logical count | unbounded resource growth, silent desynchronization |
| HM-09 | Asset robustness | load failures, missing bones/clips, material errors, scale/range warnings | broken skeleton, unreported fallback or crash |
| HM-10 | Accessibility and mobility | user-defined speed/stride/mobility adherence, transfer/turn success | system assumes a single able-bodied gait or overrides an explicit mobility profile |
| HM-11 | Blocked locomotion / intent mismatch | requested-vs-observed speed gap, realized root displacement, animation state | walk/run loop continues while the physics body remains stationary despite valid observed-velocity feedback |

## Metrics and definitions

- **Mean displacement error (ADE):** mean of the Euclidean root-position error over aligned samples.
- **Final displacement error (FDE):** Euclidean root-position error at the exact end of the shared time interval, interpolated from the two traces even when the uniform metric grid does not land exactly on that timestamp.
- **Maximum displacement error:** largest aligned root-position error; report as an outlier measure, not a substitute for ADE.
- **Named-joint trajectory error:** for every identically named joint in the canonical skeleton, report ADE, FDE at the exact shared endpoint, and maximum aligned error. Also report macro-mean per-joint ADE and the worst joint FDE. Reject differing joint-name sets; never score only the overlapping names because that can hide missing limbs or inconsistent retargeting. If neither trace contains joint trajectories, mark the comparison root/contact-only rather than implying whole-body evaluation.
- **Stance-foot drift:** RMS and maximum world-space foot displacement from the beginning of each detected contact interval. Publish the contact detector and threshold. Contact labels inferred from generated output and evaluation labels must be independently specified.
- **Contact agreement:** the Rust CLI compares reference and predicted binary contact labels on the shared time grid and reports confusion counts, accuracy, precision, recall and F1 per foot. Precision/recall are null when their denominator is absent; F1 is null when neither trace contains a positive contact label. Publish class prevalence and do not treat accuracy alone as a sufficient score under class imbalance. Contact agreement and stance-foot drift are separate: a static foot can have zero drift under badly mistimed contact labels.
- **Touchdown/liftoff event timing:** independently extract false→true (touchdown) and true→false (liftoff) transitions from each trace's native timestamps, then restrict events to the shared overlap. An event occurs at the first sample carrying the new label; timing resolution is therefore limited by contact-label sample cadence and annotation uncertainty. Match same-type events chronologically at most once within a fixed 100 ms tolerance. Report reference/prediction/matched counts, missed references, extra predictions, precision/recall/F1, signed mean timing bias, median/P95/max absolute timing error for matched pairs. Missing timing errors are not zero; unmatched counts and F1 make them visible. The matcher is deterministic O(n + m).
- **Intent/realization mismatch:** compare requested velocity against measured root velocity and displacement. If measured velocity is available, locomotion animation must follow realized velocity; a blocked character must not visually walk at the requested rate. Report when feedback is missing and the adapter falls back to desired velocity.
- **Mean-squared jerk:** discrete trajectory jerk estimate, reported with sample rate, filtering and units (m²/s⁶). It is noise-sensitive and must not be optimized in isolation; an immobile character would look artificially smooth.
- **Constraint adherence:** position/orientation error for each root, waypoint, hand, foot or joint constraint; report distributions and failure rates, not only averages.
- **Diversity/repetition:** unique motion-cluster coverage and duplicate/repetition rates, normalized by task and sequence length.
- **Perceptual quality:** blinded randomized A/B judgments from at least 30 independent raters for early screening; use a larger, pre-registered panel for strong claims. Show confidence intervals and inter-rater agreement. Do not tell raters which model made each clip.
- **Behavioral realism:** compare distributions and conditional responses to held-out, rights-cleared human reference traces; use task outcome, response timing and group flow, not just appearance.
- **Performance:** warm/cold latency, median/P95/P99 frame time, generation-to-first-pose latency, peak VRAM/RAM, CPU utilization, dropped frames, number of visible full-rig characters, number of logical agents, power if measurable. State hardware, driver, resolution, quality and LOD.
- **Robustness:** invalid numeric output, missing asset, malformed trace, extreme timestep, model unload, and loss of a sensor or input channel.

Never combine these into one overall score until component weights are public and sensitivity to those weights has been analyzed.

## Proposed targets (not achieved results)

Use the targets as initial engineering gates, not as a statement of present capabilities.

| Gate | Proposed target | Interpretation |
|---|---|---|
| Numeric / asset safety | zero NaN/Inf poses, zero unreported missing-clip fallbacks, deterministic error report for bad input | basic correctness |
| Interactive rendering | 60 FPS at 1080p for one close-view full-rig avatar on a stated reference machine; report P95 frame time and thermal state | rendering, not generative-model throughput |
| Crowd performance | first profile: 32 visible rigged people plus 500 logical residents with measured LOD; scale upward only after maintaining trace correctness | explicit LOD accounting |
| Contact quality | stance-foot drift RMS ≤ 2 cm and P95 max drift ≤ 5 cm on a fixed held-out locomotion suite | plausible first gate, not universal biomechanical validity |
| Contact-label agreement | per-foot precision, recall and F1 ≥ 0.95 on independently annotated held-out traces; publish confusion counts and class prevalence | ambitious proposed release goal; test independently of drift and avoid accuracy-only wins on imbalanced data |
| Touchdown/liftoff timing | median absolute error ≤ 30 ms and P95 ≤ 60 ms on event pairs with a declared 100 ms matching window; also require event F1 ≥ 0.95 | report cadence and annotation uncertainty; unmatched events remain failures and cannot disappear from timing summaries |
| Path following | P95 final root error ≤ 5 cm for a fixed, reachable waypoint suite; report constraint feasibility separately | controllability |
| Perceived quality | blinded preference above the authored/blended baseline, with 95% CI above chance across multiple task families | actual user evidence |
| Stability | no regression in task success or collision safety when frame rate, LOD, or cognition tick rate changes | decoupled-rate validity |
| Reproducibility | exact commit, model/hash, asset/license manifest, seed, config and raw aggregate results accompany every report | auditability |

Before promoting any target to a release gate, record baseline performance and perform a power/sensitivity analysis. If a target is too strict for the first avatar, report the gap instead of silently moving the threshold.

## Running the Rust benchmark CLI

The renderer-free Rust CLI is implemented at `crates/bridges/symtropy-bevy/src/bin/human_motion_bench.rs`. Once the branch compiles, compare the synthetic fixture with itself to validate trace loading, interpolation and the JSON report:

```sh
cargo test -p symtropy-bevy --bin human-motion-bench
cargo run -p symtropy-bevy --bin human-motion-bench -- \
  docs/research/fixtures/human-motion-trace-synthetic-v0.1.json \
  docs/research/fixtures/human-motion-trace-synthetic-v0.1.json
```

For actual traces, provide the reference and candidate JSON paths; use `--step-sec 0.033333` to explicitly set a common 30 Hz grid, and `--output report.json` to save the machine-readable report. The default common grid uses the coarser median sample interval, is bounded to one million samples, interpolates positions linearly and samples contact booleans by nearest neighbour. The JSON report includes per-foot reference/prediction contact confusion counts and precision/recall/F1, so mislabeled stance intervals cannot be hidden by a low predicted-foot-drift score. ADE and maximum error use grid samples; FDE is interpolated at the exact overlap endpoint. Both traces must overlap in time and share the same coordinate frame, canonical skeleton signature and retarget-map hash. Model/config/asset hashes are both recorded and may differ as part of the experiment.

The CLI rejects malformed JSON, incomplete required fields, wrong sample counts, non-finite coordinates, non-increasing time, incomplete asset/license provenance, and incompatible skeleton/retarget metadata. Invalid samples are rejected rather than silently dropped. The report is marked `synthetic_fixture_only` when either side is synthetic. The current implementation measures metric-computation time but does not yet provide peak-memory accounting, renderer frame-time capture, visual preference study orchestration, or a real-avatar smoke screenshot.

## Experimental protocol

1. Pin one rig and at least four motion conditions: stationary, steady walk, speed transition, and curved path. Add interaction scenes after the pipeline is reliable.
2. Compare the same reachable constraints across baselines. Separate constraints that a reference trajectory satisfies by construction from constraints that the method must infer.
3. Use the same post-processing budget, hardware and timed horizon when comparing latency or quality. Record any model setup or compilation delay separately from steady-state inference.
4. Run fixed seeds first, then multiple seeds. Use actor/session-held-out sets and report per-task means, medians, P95, failure counts and 95% confidence intervals.
5. Ablate Symthaea intent, motion selection, physics correction, contact labels, and LOD independently. A full-system win without an ablation does not identify the cause.
6. Preserve raw metrics and render pairs with stable camera/lighting. Do not hand-select only successful clips for the headline.
7. Keep visual quality, physics validity and behavioral response as separate scorecards.

## External research baselines

- **ARDY (SIGGRAPH 2026):** interactive autoregressive diffusion with text prompts and long-horizon kinematic constraints. Its released implementation uses Python/PyTorch; treat it as an external benchmark, not a production Rust dependency. Read its evaluation notes carefully: some root error tasks use ground-truth hip paths as constraints, making the ground-truth root error zero by construction. Do not compare non-identical suites as if they measure the same thing.
- **MotionBricks (ACM TOG / SIGGRAPH 2026):** modular latent generation and smart primitives aimed at real-time interactive motion skills. It is a promising baseline for skill composition and detailed control, not proof that our implementation matches the reported results.
- **Authored Bevy clips:** mandatory cheap baseline for visual transitions and locomotion before introducing a learned generator. The optional Innerscene Maya GLB provides a practical smoke scene with named clips, but its publisher describes the motion as authored periodic-pose recipes rather than motion capture.
- **Scripted deterministic controller:** mandatory task-success/robustness baseline. The Symthaea component must beat or complement it on predeclared metrics, not merely produce a more complicated trace.
- **InterPhys (2026):** recent physics-aware synthesis work motivates a dedicated dynamic-contact suite spanning body, object and scene interactions and force/torque consistency. Our current trace format measures kinematics/contact labels, not forces; do not claim physical-dynamics parity until force residuals and dynamic object interaction are instrumented.
- **AnimationBench (2026):** video-model animation evaluation operationalizes character consistency and motion rationality. It may inform blinded visual/perceptual dimensions, but its video-generation scores are not directly comparable to rigged 3D skeletal animation.
- **HumanTracker (August 2026):** its motion-tracking benchmark explicitly targets the gap between kinematic scores and human-perceived support/contact quality, using a large multi-family motion corpus and preference-aligned scoring. This motivates keeping support stability, event timing, and blinded perceptual assessment as independent benchmark axes. Do not assume its dataset license permits our use; review terms and data rights before importing traces or models.

## Next implementation slices

- P0: semantic motion enum, velocity-intent adapter, asset-backed avatar example, trace schema and standalone metric/CLI scaffolds are committed on the branch; compile/test and render evidence are still required.
- P1: download the documented optional CC0 GLB after re-checking terms, validate its rig/clips/materials in the demo, and record asset SHA-256, discovered clip names, screenshot and P50/P95 frame time.
- P2: benchmark CLI and v0.1 contact-labelled trace contract are implemented in source; it scores reference-vs-prediction contact confusion counts, precision, recall and F1 alongside drift.
- P2 follow-through: native-timestamp touchdown/liftoff event timing is now implemented in source with a fixed 100 ms matching window, signed bias, median/P95/max absolute errors, precision/recall/F1, and counts for unmatched events. Event transitions are extracted before grid resampling. Unit tests include a known 50 ms shift, one-to-one matching with duplicate detections, overlap clipping, and invalid tolerance/length controls. These tests are written but not yet run.
- Next validate both CLI metric families on the synthetic fixture and licensed/avatar traces, then add actual Bevy trace recording and reproducible run capture.
- P3: implement foot/hand contact correction as a separate opt-in system; do not conceal its compute cost.
- P4: add a Rust-facing external-model adapter contract (time-stamped root/joint constraints and pose output); any Python implementation remains an external research baseline rather than production infrastructure.
- P5: run ablations against authored, procedural and eligible generative baselines; publish a report with raw aggregates and explicit non-commercial/commercial data-rights boundaries.

## References

- Bevy 0.19 animated glTF example: https://github.com/bevyengine/bevy/blob/v0.19.0/examples/animation/animated_mesh_control.rs
- ARDY paper: https://arxiv.org/abs/2607.08741
- MotionBricks — scalable modular motion generation and smart primitives: https://doi.org/10.1145/3811334
- InterPhys — physics-aware human motion synthesis in dynamic scenes: https://arxiv.org/abs/2605.01036
- AnimationBench — character-centric animation evaluation: https://arxiv.org/abs/2604.15299
- HumanTracker — comprehensive human-aligned motion tracking benchmark: https://arxiv.org/abs/2608.13555
- HumanTracker project and paper implementation: https://dairuliu.github.io/humantracker/
- ARDY implementation and evaluation notes: https://github.com/nv-tlabs/ardy
- MotionBricks paper and project: https://nvlabs.github.io/motionbricks/
- AMASS license: https://github.com/nghorbani/amass/blob/master/LICENSE.txt
