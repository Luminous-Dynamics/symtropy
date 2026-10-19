# Human Motion: Paradigm-Shift Roadmap (2026)

**Status:** proposed engineering roadmap; target values are hypotheses, not achieved results.  
**Scope:** Symtropy character animation, interactive motion generation, physical interaction, and Symthaea-to-motion contracts.  
**Ground rule:** a benchmark result is only as strong as its data rights, split, instrumentation, and reproducibility.

## Executive direction

Do not optimize for a single “realism” score. Build a **motion skill system that can explain what it intends to do, test whether the request is feasible, execute against the body's realized state, recover from disturbances, and emit synchronized evidence of success or failure**. Rendering quality is one independent dimension.

The current PR establishes the first layer: an optional rigged-avatar playback path, typed semantic motion intent, and a source-only trace comparison CLI. The next paradigm shift is from a clip/state selector to **composable, closed-loop skills with explicit contracts**. This should work first with authored clips and deterministic control. Learned generation becomes an interchangeable proposal source—not the authority for physics or safety.

## Paradigm shifts

### 1. Animation states → composable skills with contracts

A skill such as walk_to, reach, sit, hand_over, or recover_balance should declare:

- **Preconditions:** required capabilities, initial pose/contact state, available support, object reachability, and permitted environment.
- **Inputs:** typed intent, target frame, time horizon, constraints and priorities, plus the actor's mobility/embodiment profile.
- **Postconditions:** achieved task state, endpoint tolerances, contact/support condition, and completion deadline.
- **Abort/recovery:** how to stop, replan, release a constraint, or report that a task is infeasible.
- **Evidence:** skill/configuration digests, declared constraint changes, realized state, outcome and failure reason.

Compose skills through an inspectable graph with typed transitions, not by blindly cross-fading arbitrary clips. Start with deterministic authored implementations so the contract can be tested without any learned model. A skill is not “successful” merely because it emitted frames.

### 2. Requested movement → realized state as the source of truth

Keep intent, planned motion, control action, and observed/physics state as separate fields. The animation layer should follow realized velocity and contact when available. A blocked agent must not visually walk in place unless that is the explicitly selected behavior. When physics feedback is absent, mark the fallback; never silently present desired velocity as measured motion.

Use synchronized native-clock traces of reference, intent, target, action, generalized state, contact labels/forces, constraints and task outcomes. This reduces the chance that separately sampled streams or different post-processing budgets create unfair comparisons. Trace v0.3 physical signals are tracked in [issue #2093](https://github.com/Luminous-Dynamics/symtropy/issues/2093).

### 3. Static similarity → counterfactual, closed-loop competence

Test the same request under controlled changes: a clear path versus a blocked path, an available versus unreachable object, a stable versus disturbed support, a changed target mid-skill, and a dropped input channel. The desired result is not the same animation; it is an appropriate change in action, recovery, or an explicit infeasibility result.

### 4. Known clips → generalization to unseen compositions

Hold out combinations rather than only holding out individual motion clips. For example, author walk, turn, reach, and hand-over independently, then test combinations such as turn-while-reaching or approach-then-hand-over. Keep the actor, session, source clip, and task-composition splits disjoint where applicable. Report primitive performance and composition performance separately.

### 5. Smoothness → physical and perceptual fidelity together

A motion can be smooth while skating, floating, penetrating objects or breaking support. Pair geometric metrics (root/joint error and orientation) with contact timing, stance drift, constraint residuals, force/impulse continuity, collision penetration, task success and blinded preference. Do not let improvements in jerk compensate for task failure or physical invalidity.

### 6. One “standard human” → explicit embodiment and accessibility profiles

Treat target speed, stride envelope, turning radius, reach, balance constraints and mobility accommodations as inputs, not as defects to be normalized away. Evaluate whether the system honors a profile without requiring every actor to use the same gait. Report results by profile and task, and avoid inferring identity, emotion, intent or consciousness from a motion score.

### 7. One opaque score → evidence ladder and failure envelope

Publish a scorecard with separate geometry, contact/physics, constraint-following, task success, perception, latency/resources, robustness and provenance columns. Include per-task/family distributions, worst cases and confidence intervals. A weighted aggregate may be added only after public weights and sensitivity analysis; it must never conceal a failed safety or feasibility gate.

## Proposed benchmark suite

Each task should have a reference-free procedural/synthetic negative control, a deterministic baseline and—only where rights allow—a held-out human/reference condition. Preserve each scenario's seed, inputs, expected invariants, and failure oracle.

| Suite | Counterfactual / stress condition | Required measurements |
|---|---|---|
| S1 — Feasibility & conflict | reachable/unreachable target; hard constraints that are mutually incompatible; blocked route | feasibility classification, false-success rate, endpoint error, reason-coded failures |
| S2 — Composition holdout | novel combinations of individually known skills; changed order and interruption point | completion rate, transition discontinuity, contact/task success per composition |
| S3 — Closed-loop perturbation | push or external disturbance; sudden obstacle; temporary loss of velocity/contact input | time to detect, recovery rate/time, falls/collisions, realized-vs-requested motion |
| S4 — Dynamic interaction | moving object, hand-off, surface contact, support changes | constraint residual, collision/penetration, contact timing, force/impulse discontinuity when instrumented |
| S5 — Long-horizon drift | repeated turns, waypoint changes and looping for 30–120 s | root/heading drift, contact errors, failure hazard versus duration, memory growth |
| S6 — Mobility & embodiment | multiple calibrated stride/speed/reach profiles, turning and level changes | per-profile goal success and constraint residuals; deviations from requested profile |
| S7 — Rate/LOD invariance | vary rendering frame rate, physics/control tick, cognitive tick, and number of visible/logic-only agents | task/physics equivalence, stale-intent rate, desyncs, P50/P95/P99 frame and control latency |
| S8 — Input and asset fault injection | NaN/Inf, expired intent, malformed trace, missing bone/clip, asset unload, dropped input | detected/rejected fraction, deterministic error class, crashes, invalid pose count |
| S9 — Human preference | blinded randomized pairwise comparisons over diverse tasks and matched camera/render budgets | preference win rate with 95% CI, inter-rater agreement, subgroup/task breakdown |
| S10 — Provenance/replay | repeat the same commit/config/seed; compare clean and altered digests | exact replay where supported; otherwise quantified variation; fail-closed mismatch detection |

Do not use the same synthetic fixtures to claim human-fidelity. Synthetic cases test metric correctness and negative controls. Real-reference claims need a legally usable, actor/session-held-out source with traceable acquisition/consent records; public metadata alone is not independent proof of rights.

## Proposed gates (targets, not results)

Treat these as staged hypotheses to calibrate against deterministic baselines and actual sensor/physics cadence.

| Gate | Initial proposed target | Required qualifier |
|---|---|---|
| G0 — contract correctness | 100% of malformed, expired or infeasible requests are rejected or given an explicit non-success outcome; zero NaN/Inf output | property/negative-control tests; no task may silently pass |
| G1 — reachable task execution | ≥95% completion on a preregistered, reachable deterministic suite | publish task counts, success definition and failures; do not tune on held-out cases |
| G1 — infeasibility | ≥99% detection for generated impossible/conflicting requests and 0% marked successful | separately report false-positive rejections of feasible tasks |
| G2 — composition | ≥90% completion on held-out combinations, with no individual family hidden by aggregate | primitive baselines and composition split documented |
| G2 — disturbance recovery | ≥90% recovery in calibrated perturbation cases within 2 s, with no unsafe collision | physics-model and perturbation magnitude stated; target is provisional |
| G2 — contact | retain protocol target: stance-foot drift RMS ≤2 cm and P95 maximum drift ≤5 cm; independently annotated contact F1 ≥0.95 | instrument the physics/contact source; contact labels and drift stay separate |
| G2 — constraints | P95 reachable end-effector residual ≤2 cm and orientation ≤5° | only for reachable targets and a fixed skeleton/coordinate frame |
| G3 — interaction | blinded A/B preference above deterministic/authored baseline with 95% CI above chance | ≥30 independent raters for screening; preregister sample size for stronger claims |
| G3 — responsive runtime | 60 FPS at 1080p for one close-view full-rig character on a declared reference machine; report first-pose and control-update latency separately | distinguish render frame time from generation/model latency; include thermal state |
| G3 — scaling | initial target remains 32 visible full rigs + 500 logical agents with measured LOD | no desync, unbounded growth, or hidden reduction in logical update rate |
| All gates — reproducibility | exact commit, model/asset/config digests, rights manifest, seed, scenario, hardware/backend and raw report attached | nondeterministic backends report multi-run distributions, not a single score |

These gates are not universal biomechanical standards. Calibrate them on measured baselines, annotate uncertainty, and revise only through documented protocol changes—not after seeing a disappointing result. Where the motion sample interval is too coarse for an event-time target, report timing as diagnostic rather than implying sub-frame precision.

## Implementation sequence

### P0 — Make the existing baseline green and auditable

1. Apply the exact rustfmt --check diff, then require formatting, workspace invariants, clippy and relevant tests to complete on the exact head. Earlier formatter-reported hunks and a Clippy non-minimal-boolean fix have been applied; latest skill-contract format corrections are committed, with exact-head verification pending.
2. Validate the emitted v2 report against human-motion-benchmark-report-v2.schema.json; add schema validation to automated tests once a Rust-native validator/dependency policy is chosen.
3. Run synthetic positive and negative controls; clearly label those reports software-validation-only.
4. Execute an asset-backed visual smoke test only after rechecking the asset's current license and logging hash, clip names, machine and frame-time measurements.

### P1 — Motion skill contract (small Rust-first slice)

**First contract slice now exists in source** in `crates/core/symtropy-motion-contract/src/skill.rs`, exposed by `pub mod skill`: versioned contract, preconditions, hard/soft constraints with explicit precedence, runtime-supplied feasibility evidence, postconditions, and reason-coded outcomes (`ready`, `completed`, `infeasible`, `expired`, `interrupted`, `capability_missing`, `execution_failed`). The matching [motion-skill-contract-v1.schema.json](motion-skill-contract-v1.schema.json) defines the JSON shape. Keep continuous control, physics and renderer types behind adapters. This source is not yet wired into the live Bevy/physics execution loop, and its tests remain pending exact-head CI verification.

Acceptance tests should prove: (a) expired intent cannot start a skill; (b) a hard safety/physics constraint cannot be silently overridden by style; (c) incompatible hard constraints return infeasible; (d) completion requires postcondition verification; (e) interrupted skills return an explicit status and may request recovery; and (f) serialized contracts reject unknown schema versions. All tests must run on the exact PR head before claiming completion.

### P2 — Synchronized physical trace v0.3

Extend rather than mutate v0.1/v0.2. Capture input intent, planned target, action/actuator target, realized root/joint pose and velocity, per-limb contact state/force when available, constraint residual, physics step identifier/timestamp, termination reason and task outcome. Missing force data must be unavailable, not zero. Add synthetic controls for known slip, impulse discontinuity, penetration and misaligned clocks. See [issue #2093](https://github.com/Luminous-Dynamics/symtropy/issues/2093).

### P3 — Physics and preference baselines

Run authored clips and the deterministic controller first. Compare only on matched skeleton, constraints, horizon, post-processing budget and task suite. Add dynamic-object and disturbance tasks before making physics-aware claims. If a learned baseline is run, record separate model/data licenses and exact checkpoint digest; external Python research repositories are baselines, not production dependencies.

### P4 — Real-reference evaluation and ablation

Acquire or capture rights-cleared, actor/session-held-out data before any human-fidelity claim. Separate evaluation consent from commercial training rights; minimize retained identity data. Run blinded preference tests and ablate intent conditioning, skill composition, contact correction and LOD independently. Publish failures and distributions.

## Recent research informing this direction

- **HumanTracker (ECCV 2026 / arXiv:2608.13555):** approximately 153 hours from 24 performers across daily, highly dynamic, interaction and ground motion; includes task success, kinematic/contact diagnostics and a preference-aligned trajectory score. The strongest transferable engineering idea is a standardized evaluator driven by the same synchronized physical rollout. We are not claiming to reproduce its model or metric.
- **ARDY (SIGGRAPH 2026):** interactive motion generation with text prompts and kinematic constraints. Its published evaluation notes show why benchmark construction matters: some root-path errors are zero by construction when the ground-truth path is used as a constraint. Compare only matched tasks and constraint sources.
- **MotionBricks (ACM TOG 2026):** modular latent motion and smart primitives emphasize real-time scaling and fine-grained control. This motivates held-out skill composition and latency/scale measurements rather than clip count alone.
- **InterPhys (2026):** motivates going beyond hand/foot pose to human-object, human-scene and internal force/torque consistency. Symtropy must not claim this fidelity before its own force instrumentation and negative controls exist.
- **Unified human-motion generation metric review (2025):** motivates reporting metric limitations, temporal diversity and comparable protocols instead of treating one metric as a universal ranking.

## External references

- HumanTracker paper: https://arxiv.org/abs/2608.13555
- HumanTracker evaluator and implementation: https://github.com/GalaxyGeneralRobotics/HumanTracker
- ARDY project: https://research.nvidia.com/labs/sil/projects/ardy/
- ARDY code and evaluation notes: https://github.com/nv-tlabs/ardy
- MotionBricks paper: https://doi.org/10.1145/3811334
- InterPhys paper: https://arxiv.org/abs/2605.01036
- Unified evaluation framework (2025): https://doi.org/10.1016/j.cviu.2025.104337
- AMASS terms: https://amass.is.tue.mpg.de/license.html
- Current Symtropy benchmark protocol: [HUMAN_MOTION_BENCHMARK_PROTOCOL_V0_1.md](HUMAN_MOTION_BENCHMARK_PROTOCOL_V0_1.md)
