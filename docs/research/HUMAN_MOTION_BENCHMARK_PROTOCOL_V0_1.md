# Human Motion Benchmark Protocol v0.1

**Status:** proposed protocol; targets below are not achieved results.  
**Date:** 2026-10-19  
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
- **Evidence tiers are claims backed by metadata, not self-authenticating facts.** Trace hashes prove byte identity only if the hash is anchored to a trusted source; a `test` label or a non-empty license field does not independently prove a participant gave consent, that the signer had authority, or that the recording was held out. Preserve a rights/consent audit record outside public traces where needed, and have a human reviewer approve its release and evaluation scope. The CLI's strict gate checks declared fields; it does not verify legal rights, signatures, or consent documents.
- Keep identity data private: actor pseudonyms and recording IDs should be random, scope-limited identifiers, with any re-identification key stored separately and access-controlled.
- Split by actor and recording session, not merely by adjacent frames. Avoid leakage of the same motion sequence into training and evaluation.

## Machine-readable trace format

The motion trace records observed/predicted trajectories; it is separate from the upstream [semantic intent schema](human-motion-intent-v0.1.schema.json), which records what an agent intends to do and with what confidence/provenance. Do not use intention correctness as a substitute for realized motion quality.

[`human-motion-trace-v0.1.schema.json`](human-motion-trace-v0.1.schema.json) preserves the original position/contact contract, with [its synthetic fixture](fixtures/human-motion-trace-synthetic-v0.1.json). The additive [v0.2 schema](human-motion-trace-v0.2.schema.json) adds world-space root quaternions in XYZW order, one sample per timestamp; the [v0.2 synthetic orientation fixture](fixtures/human-motion-trace-orientation-synthetic-v0.2.json) tests the extension without representing a real person. The Rust reader supports v0.1 and v0.2, requires equal trace versions when comparing, and refuses to infer missing rotations as zero error. It rejects mismatched sample lengths, non-increasing timestamps, non-finite parsed values, missing asset provenance on non-synthetic traces, incompatible rigs, mismatched joint-name sets and invalid/non-unit quaternions before scoring. Accepted quaternion norms must differ from 1 by at most 1e-3; accepted quaternions are normalized for interpolation. Compare prediction/reference traces only after explicit time alignment.

The emitted report has an independent version, currently v2, described by [human-motion-benchmark-report-v2.schema.json](human-motion-benchmark-report-v2.schema.json). Downstream consumers should validate report JSON against that schema and branch on the report schema_version field rather than assuming every metric is present and numeric. In particular, root_orientation_error may be null for v0.1 comparisons and yaw statistics may be null when the heading is undefined.

The next layer is separate from trace serialization: [motion-skill-contract-v1.schema.json](motion-skill-contract-v1.schema.json) defines the version-1 declarative skill request, including typed preconditions, hard/soft constraints and required postconditions. The [synthetic contract fixture](fixtures/motion-skill-contract-synthetic-v1.json) is decoded and semantically validated by a Rust unit test. The Rust `symtropy-motion-contract::skill` module evaluates start feasibility and explicit outcomes from runtime-supplied evidence. This metadata schema does not replace semantic checks: expiry, duplicate identifiers, quaternion normalization, constraint-kind/target compatibility and physics conflict evidence must also pass the Rust validator. A JSON Schema validator has not yet validated the fixture against the schema; that remains CI hardening work.

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

- **Evidence scope:** each report records an explicit `evidence_scope` separate from geometric accuracy. Synthetic comparisons are `synthetic_software_validation_only`; a non-synthetic trace with ordinary provenance checks but no qualifying human reference is `source_provenance_present_reference_not_held_out_human_verified`; distinct traces with a declared original consent capture reference, `test` split, dataset ID/version, pseudonymous actor ID and recording/session ID are `declared_held_out_human_reference`. The last label is still a metadata-backed declaration, not an independent rights/consent audit. Use `--require-held-out-human-reference` to reject comparisons that do not meet the declared reference gate. A zero-error self-comparison is a plumbing test, not motion-fidelity evidence.
- **Mean displacement error (ADE):** mean of the Euclidean root-position error over aligned samples.
- **Final displacement error (FDE):** Euclidean root-position error at the exact end of the shared time interval, interpolated from the two traces even when the uniform metric grid does not land exactly on that timestamp.
- **Maximum displacement error:** largest aligned root-position error; report as an outlier measure, not a substitute for ADE.
- **Named-joint trajectory error:** for every identically named joint in the canonical skeleton, report ADE, FDE at the exact shared endpoint, and maximum aligned error. Also report macro-mean per-joint ADE and the worst joint FDE. Reject differing joint-name sets; never score only the overlapping names because that can hide missing limbs or inconsistent retargeting. If neither trace contains joint trajectories, mark the comparison root/contact-only rather than implying whole-body evaluation.
- **Stance-foot drift:** RMS and maximum world-space foot displacement from the beginning of each detected contact interval. Publish the contact detector and threshold. Contact labels inferred from generated output and evaluation labels must be independently specified.
- **Contact agreement:** the Rust CLI compares reference and predicted binary contact labels on the shared time grid and reports confusion counts, accuracy, precision, recall and F1 per foot. Precision/recall are null when their denominator is absent; F1 is null when neither trace contains a positive contact label. Publish class prevalence and do not treat accuracy alone as a sufficient score under class imbalance. Contact agreement and stance-foot drift are separate: a static foot can have zero drift under badly mistimed contact labels.
- **Root orientation (v0.2):** accept world-space unit quaternions in XYZW component order. Validate finite components and norm within 1e-3 of 1, normalize for numerical stability, interpolate with shortest-arc SLERP, and compare rotation using geodesic error (2\arccos(|q_p \cdot q_r|)). Report mean, exact-overlap-end final, nearest-rank P95 and maximum in both radians and degrees. Because the declared up-axis is +Y, also report signed mean yaw bias and mean-absolute/final/P95/maximum absolute yaw error; full rotation error remains primary because yaw alone misses pitch and roll. Yaw is undefined when the rotated forward direction has a near-zero horizontal projection; report the number of yaw samples scored/undefined and serialize yaw metrics as null when undefined, rather than inventing a heading. A v0.1 report must say orientation is unavailable (JSON null), never imply the missing rotation error is zero.
- **Touchdown/liftoff event timing:** independently extract false→true (touchdown) and true→false (liftoff) transitions from each trace's native timestamps before uniform-grid resampling. The alignment section of each JSON report records native median and maximum input sample intervals for reference and prediction; report event errors with that cadence so sub-frame precision is not implied. Use the half-open/closed shared interval `(start, end]`: the left boundary is excluded because preceding contact state may be unavailable in one trace; the right boundary is included. An event occurs at the first sample carrying the new label, so timing resolution is limited by contact-label cadence and annotation uncertainty. The current Rust CLI uses deterministic `chronological_first_eligible` one-to-one matching within a fixed 100 ms tolerance, O(n + m). Report reference/prediction/matched counts, missed references, extra predictions, precision/recall/F1, signed mean timing bias, median/P95/max absolute timing error for matched pairs. Matching policy and tolerance must remain fixed across compared systems. Missing timing errors are not zero; unmatched counts and F1 make them visible.
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
| Touchdown/liftoff timing | median absolute error ≤ 30 ms and P95 ≤ 60 ms on event pairs with a declared 100 ms matching window; also require event F1 ≥ 0.95 | target is eligible only when native max sample interval is ≤ 16.7 ms (≥ 60 Hz without gaps) or event times come from a higher-resolution independent annotation; otherwise timing scores are diagnostic only. Unmatched events remain failures. |
| Root orientation | P95 full-rotation geodesic error ≤ 5° on held-out reachable path-following tasks | proposed target; separately inspect yaw bias and do not treat v0.1 traces without rotations as zero error |
| Path following | P95 final root error ≤ 5 cm for a fixed, reachable waypoint suite; report constraint feasibility separately | controllability |
| Perceived quality | blinded preference above the authored/blended baseline, with 95% CI above chance across multiple task families | actual user evidence |
| Stability | no regression in task success or collision safety when frame rate, LOD, or cognition tick rate changes | decoupled-rate validity |
| Reproducibility | exact commit, model/hash, asset/license manifest, seed, config and raw aggregate results accompany every report | auditability |

Before promoting any target to a release gate, record baseline performance and perform a power/sensitivity analysis. If a target is too strict for the first avatar, report the gap instead of silently moving the threshold.

## Running the Rust benchmark CLI

The renderer-free Rust CLI is implemented at `crates/bridges/symtropy-bevy/src/bin/human_motion_bench.rs`. Compare either versioned synthetic fixture with itself to validate trace loading and report serialization after compilation; v0.2 additionally exercises quaternion validation and orientation metrics:

```sh
cargo test -p symtropy-bevy --bin human-motion-bench
cargo run -p symtropy-bevy --bin human-motion-bench -- \
  docs/research/fixtures/human-motion-trace-synthetic-v0.1.json \
  docs/research/fixtures/human-motion-trace-synthetic-v0.1.json

# Version 0.2 additionally validates and scores root orientation.
cargo run -p symtropy-bevy --bin human-motion-bench -- \
  docs/research/fixtures/human-motion-trace-orientation-synthetic-v0.2.json \
  docs/research/fixtures/human-motion-trace-orientation-synthetic-v0.2.json

# Add this flag to an evaluation command when a held-out human reference is required:
# --require-held-out-human-reference
```

For actual traces, provide the reference and candidate JSON paths; use `--step-sec 0.033333` to explicitly set a common 30 Hz grid, and `--output report.json` to save the machine-readable report. The default common grid uses the coarser median sample interval, is bounded to one million samples, interpolates positions linearly and samples contact booleans by nearest neighbour. The JSON report includes per-foot reference/prediction contact confusion counts and precision/recall/F1, native median/max source sampling intervals, and native-time touchdown/liftoff event metrics, so mislabeled stance intervals cannot be hidden by a low predicted-foot-drift score and coarse sampling is explicit. ADE and maximum error use grid samples; FDE is interpolated at the exact overlap endpoint. Both traces must overlap in time and share the same coordinate frame, canonical skeleton signature and retarget-map hash. Model/config/asset hashes are both recorded and may differ as part of the experiment.

The CLI rejects malformed JSON, incomplete required fields, wrong sample counts, non-finite coordinates, non-increasing time, incomplete asset/license provenance, and incompatible skeleton/retarget metadata. Invalid samples are rejected rather than silently dropped. Every report (report schema version 2) contains an explicit evidence scope; use `--require-held-out-human-reference` to fail unless the reference declares `original_consent_capture`, a `test` split, dataset ID/version, pseudonymous actor ID and recording/session ID. The gate also rejects same-ID reference/prediction pairs and synthetic predictions. This is a metadata contract only: it cannot independently verify consent, permissions, dataset integrity or signed attestations. The report is marked `synthetic_fixture_only` when either side is synthetic. The current implementation measures metric-computation time but does not yet provide peak-memory accounting, renderer frame-time capture, visual preference study orchestration, or a real-avatar smoke screenshot.

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
- **HumanTracker (ECCV 2026; repository reviewed 2026-10-19):** the authors describe roughly 153 hours of motion from 24 performers across Daily, Highly Dynamic, Interaction and Ground families. They report task success, joint-angle MPJPE, preference-aligned HumanScore and jerk by family. Their evaluator standardizes the humanoid model, reference indexing, rollout accounting and metric implementation while retaining native tracker observation/action decoding; it records one synchronized rollout history with generalized positions/velocities, actions, motor targets, foot contacts/forces, foot/pelvis velocities and keypoint poses. This is a useful pattern for our next v0.3 trace: derive diagnostics from the same recorded run so separate capture and post-processing pipelines cannot create unfair comparisons. We should not copy their HumanScore, code, weights, or dataset into Symtropy without a separate use/license review.
- **Reproducibility caution from HumanTracker:** its README reports bit-identical CPU runs but notes CUDA results can vary even for repeated single-trajectory runs. Therefore record backend/device, seed, solver/controller config and hardware. On nondeterministic backends, publish multiple-run distributions and confidence intervals rather than a single supposedly deterministic score.
- **Dataset rights:** HumanTracker-authored code is Apache-2.0, but the project notes that bundled runtime components, policy weights and third-party policies retain their own license status. Its code license does not automatically grant rights to its motion data, weights or external policies; evaluate those separately before reuse.

## Next implementation slices

- P0: semantic motion enum, velocity-intent adapter, asset-backed avatar example, trace schema and standalone metric/CLI scaffolds are committed on the branch; compile/test and render evidence are still required.
- P1: download the documented optional CC0 GLB after re-checking terms, validate its rig/clips/materials in the demo, and record asset SHA-256, discovered clip names, screenshot and P50/P95 frame time.
- P2: benchmark CLI and v0.1 contact-labelled trace contract are implemented in source; it scores reference-vs-prediction contact confusion counts, precision, recall and F1 alongside drift.
- P2 follow-through: native-timestamp touchdown/liftoff event timing is implemented in source with a fixed 100 ms matching window, signed bias, median/P95/max absolute errors, precision/recall/F1, and counts for unmatched events. Event transitions are extracted before grid resampling; the overlap uses `(start, end]` to avoid left-censored events. The deterministic matcher is named in each report. Unit tests include a known 50 ms shift, one-to-one matching with duplicate detections, overlap clipping, and invalid tolerance/length controls. These tests are written but not yet run.
- P3 foundation: schema v0.2 and an orientation fixture now extend the trace with root XYZW quaternions. The CLI validates unit norms (1e-3 tolerance), uses shortest-arc SLERP, and reports full geodesic angular errors plus Y-up yaw error in radians/degrees; v0.1 traces serialize orientation as unavailable. Tests cover a 90° yaw error, quaternion sign equivalence, invalid/missing rotations and an exact non-grid-aligned endpoint. These tests are in source but are not yet verified by a successful build/test run.
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
- HumanTracker project, evaluation harness and metric definitions: https://github.com/GalaxyGeneralRobotics/HumanTracker
- HumanTracker project and paper implementation: https://dairuliu.github.io/humantracker/
- AMASS official dataset license and commercial-use restriction: https://amass.is.tue.mpg.de/license.html
- ARDY implementation and evaluation notes: https://github.com/nv-tlabs/ardy
- MotionBricks paper and project: https://nvlabs.github.io/motionbricks/
- AMASS license: https://github.com/nghorbani/amass/blob/master/LICENSE.txt
