# Evidence-Bounded Action Horizon Qualification

Status: deterministic reference implementation authored; not yet compiled or test-executed. This document defines selector semantics and the benchmark programme. It does not claim controller, safety, locomotion, manipulation or hardware qualification.

## Research question

How long can a robot commit to executing an action prefix before it must reconsider fresh observations, physical validity and recovery options—and can it overlap slow deliberation without using an unsafe open-loop queue?

The design principle is **viability-bounded commitment, not latency-sized commitment**.

A horizon selector is a composition layer. It consumes lower-bound time-to-invalidity estimates already qualified by their owners. It must not recreate their sensor, thermal, actuator, viability, projection, scheduler or controller logic.

## Paradigm shift

Three separate clocks are easy to conflate:

- how quickly a policy can think;
- how long a planned action remains valid against the observations and model that produced it;
- how long the physical system can continue while preserving a safe stop or recovery option.

The longest safe action chunk is bounded by the smallest qualified physical/evidence horizon. Inference latency determines whether a queued open-loop architecture can cover the time needed to produce the next update. It does not extend the physical validity horizon.

### Qualified horizon inputs

The selector consumes eight identity-bound lower bounds, all already mapped to one application-time basis:

1. Observation freshness.
2. Action-chunk expiry.
3. Action/task deadline.
4. State/certificate validity.
5. Contact and hybrid-model validity.
6. Actuator response/envelope validity.
7. Per-node thermal derating.
8. Mission reserve and terminal recovery/safe-stop validity.

For conservative lower bounds H_i:

    H_safe = min(H_observation, H_chunk, H_deadline, H_certificate,
                 H_contact, H_actuator, H_thermal, H_recovery)

For controller period Δt:

    K_max = floor(H_safe / Δt)

For measured p99 total observation-cutoff-to-controller-admission latency L_p99:

    K_min = ceil(L_p99 / Δt)

The queued open-loop mode is feasible only when K_min <= K_max. The selected prefix is the longest whole-cycle prefix K_max within that interval.

A split-timescale architecture is different: a continuously running local action expert keeps control responsive to fresh observations while the slower world model updates context. The slow deliberation latency does not become an open-loop minimum queue requirement; the local expert remains bounded by K_max and its independent control/safety contract.

Do not combine these two modes in one undifferentiated action-chunk score.

## Fail-closed contract

The selector must reject composition when:
- any required horizon is Unknown;
- any evidence item declares the wrong constraint identity for its canonical slot;
- qualified bounds use different application-time basis identifiers;
- the common horizon is shorter than one controller cycle;
- a queued architecture's measured p99 queue minimum exceeds K_max;
- the controller period, evidence identity or p99 latency-profile identity is invalid.

Unknown is not infinity. A missing thermal horizon cannot be dropped because the other seven inputs are known. A raw source timestamp is not a qualified duration; the upstream owner must map it, account for uncertainty, and provide the conservative lower bound in the common application-time basis.

Every decision carries a fixed-size receipt containing all eight bounds, source identities, timebase IDs, unknown reasons, the active minimum constraint(s), p99 latency profile, K_min and K_max. The selector is fixed-size and allocation-free; it does not authorize or dispatch an action.

## Implementation status

The Rust module is `crates/core/symtropy-physics/src/action_horizon.rs` and is exported from the crate.

It includes 16 authored deterministic tests for:
- exact minimum and floor arithmetic;
- tied active constraints;
- Unknown rejection for every required constraint;
- incompatible timebases;
- misordered constraint identity;
- K_min > K_max rejection without horizon extension;
- split-timescale operation despite remote deliberation latency;
- exact and fractional p99 cycle rounding;
- zero safe horizon and zero controller period;
- empty source/timebase/reason/latency-profile identities, including direct enum-construction bypass attempts;
- canonical receipt order.

Criterion cases are authored for feasible queued mode, split-timescale mode, and Unknown-thermal rejection. They are latency measurements only; they do not qualify physical validity. These tests and benchmarks have not been compiled or executed in the current environment.

## Industry and SOTA comparison

### Asynchronous manipulation models

Current 2026 manipulation research demonstrates that execution horizon and inference architecture matter:

| Reference | Reported result | What it establishes | What it does not establish |
|---|---|---|---|
| Streaming-WAM, arXiv:2609.28927 | 98.35% average LIBERO success; mean Stamp Paper episode time 90 s to 38 s versus synchronous Joint-WAM | Action-conditioned future prediction can account for an already-committed prefix during asynchronous execution | A committed prefix is physically safe under contact, thermal, actuator or recovery constraints |
| AHA-WAM, arXiv:2606.09811 | 92.80% RoboTwin average success; 78.3% across four real-world tasks; 24.17 Hz closed-loop control; reported 4.59x speedup over Fast-WAM | Low-frequency world context can be separated from a high-frequency action expert | A universal horizon for legged/humanoid motion or a certified recovery guarantee |
| SplineWAM, arXiv:2609.39873 | +8.2 and +4.4 success points on LIBERO-Plus/RoboCasa; 22% and 26% fewer policy calls per episode; 1.2–1.6x more executed motion per call in three bimanual tasks | Motion-dependent chunk duration can allocate temporal resolution non-uniformly | The physical validity of each variable-duration chunk |
| Rolling-WAM, arXiv:2609.30247 | Reported 4.5x steady-state replanning speedup; evaluated on LIBERO, RoboTwin and a real Unitree G1 | Prediction/denoising work can be spread across replanning cycles | Thermal/actuator/mission-reserve bounded action commitment |

These are preprint results on different manipulation benchmarks and hardware; do not rank the percentages across papers as if they were matched experiments.

**Luminous differentiation:** make action commitment conditional on the physical evidence horizon and on preservation of recoverable future options. Compare the same action model under fixed-K, latency-only K, freshness-only K, individual physical-bound K, joint K_safe and split-timescale execution. Count safe completion, terminal safe-stop, stale rejection and recovery outcomes—not just task success or throughput.

### Passivity and physical human-robot interaction

The August 2026 paper *NMPC-based variable admittance control with passivity constraints* includes impedance-parameter dynamics in nonlinear MPC and jointly constrains passivity, robot state and control signals. This is a strong comparator for any later adaptive interaction controller. Luminous should consume a qualified controller/passivity envelope as an upstream physical constraint; the horizon selector is not a passivity controller and does not replace passivity proofs or contact-force limits.

### Humanoid system qualification

Use NIST's Humanoid Robot Baseline Performance Benchmark as the developing low-footprint task foundation: mobility, manipulation/dexterity, coordinated loco-manipulation, whole-body awareness in confined spaces, and basic reasoning/scene understanding. NIST describes it as under development with industry and research participation sought; it is not yet a finalized public scoring leaderboard.

Use Fraunhofer IPA's independent six-module humanoid assessment as the industrial-readiness reference:
1. Basic technologies and capabilities.
2. Complex tasks and precision.
3. Cleanroom suitability.
4. Functional safety.
5. Cybersecurity.
6. Energy efficiency.

Fraunhofer's reported Unitree G1 EDU-4 / Dex3-1 assessment (firmware 1.04, specific tested configuration) reported collision forces above 500 N under tested conditions, battery duration of 2 h 49 min while standing, and 1 h 49 min in a mixed standing/walking scenario. It also identified a Bluetooth vulnerability in the tested software version that was subsequently resolved. These are configuration-specific observations, not universal properties of all G1 systems.

For hands, retain POMDAR as the recognized structured physical-dexterity throughput benchmark and HAND ERC's application/system/hand/component attribution levels. Use SHAP for approved user-led ADL assessment. For a full humanoid, do not let a hand dexterity score stand in for whole-body safety, human compatibility, maintainability or industrial readiness.

## Required matched benchmark matrix

Compare nine policies/architectures under matched task, robot, sensor, compute, latency stratum, payload, terrain, controller period and safety authority:

1. Fixed K selected from manipulation defaults.
2. Latency-only K sized to measured p99.
3. Observation-freshness-only K.
4. Contact/model-validity-only K.
5. Actuator/thermal-only K.
6. Recovery-reserve-only K.
7. Joint evidence/viability-bounded K_safe.
8. Split-timescale local action expert with a slower world-context update.
9. Evaluator-only clairvoyant K upper bound (never deployable).

Include thermal-aware MPC/HQP, robust MPC and thermal-aware RL as physical-controller comparators. Manipulation WAMs should only be included in a legged/humanoid comparison after a matched embodiment/observation/action interface exists.

### Adversarial scenario families

- Warm motor with little thermal headroom and high inference latency (K_min > K_max).
- Fast inference but rapidly changing contact mode.
- Unknown source-clock mapping or stale calibration.
- One actuator nearing derating while whole-body mean temperature remains benign.
- Backlash/tracking delay invalidating a long prefix.
- Recovery path lost after a disturbance, despite current one-step feasibility.
- Edge contention raising p99 after the action queue was sized.
- Network partition where the local expert remains available but remote context is stale.
- Fixed long K wins nominal success but loses terminal recovery/safe-stop.
- Unknown physical-model envelope blocks horizon extension without inventing a substitute value.

### Scorecard

Report separately:
- selected K, K_min, K_max, H_safe and every input bound/source/uncertainty;
- active minimum constraint(s) and rejection reason;
- p50/p95/p99 observation-cutoff-to-admission latency by hardware/network/contention stratum;
- queue underflow, deadline miss, stale action rejection, chunk discontinuity, jerk and tracking error;
- per-node peak temperature and time-to-derating;
- actuator delay, torque/slew tracking and energy;
- task success, safe-success and failure taxonomy;
- observation-constrained future continuations and terminal safe-stop success;
- fallback/abstention rate, recovery time and adverse-event severity.

Never hide a single actuator thermal violation in a whole-body mean. Never treat task success as safe-success. Never compare a clairvoyant evaluator to a deployable controller as an equal competitor.

### Initial qualification targets

These are targets, not measured results:
- 100% correct decisions on the deterministic known-answer corpus;
- zero Unknown/misordered/timebase-mismatched inputs accepted as positive horizons;
- zero queued execution selected when K_min > K_max;
- zero execution beyond the minimum qualified horizon in finite reference tests;
- zero hard per-node thermal-limit violation hidden by an aggregate metric;
- zero missed terminal safe-stop caused by horizon extension;
- selector p99 latency below 100 microseconds on named target x86-64 and ARM64 CPUs, with allocation count zero;
- at matched service and compute budgets, joint-horizon or split-timescale control improves safe-success/recoverability Pareto performance over fixed-K and latency-only baselines.

Freeze performance gates after repeatability and hardware variance are measured. The 100 microsecond target is a microbenchmark goal for the selector, not an end-to-end robot control-loop claim.

## Ownership and integration

This selector composes with:
- Symtropy #4117: state-dependent evidence/certificate validity;
- #4083: contact/hybrid model coverage;
- #4610: per-node thermal bounds;
- #4206: actuator response envelope;
- #4179/#4044: actuator loss and measured energy;
- #4382: policy-to-dynamics projection;
- #4549/#4555: persistent mission reserve and terminal recovery;
- #4588: observation-constrained future mobility;
- #4689: asynchronous execution under persistent thermal/actuator state;
- Symthaea #11561/#11573: cognition timing and compute placement.

It does not create another sensor clock validator, certificate validator, thermal model, actuator model, viability solver, policy projector, scheduler, or motor controller.

## References

- Streaming-WAM, Huang et al., arXiv:2609.28927: https://arxiv.org/abs/2609.28927
- AHA-WAM, Cai et al., arXiv:2606.09811: https://arxiv.org/abs/2606.09811
- SplineWAM, Guo et al., arXiv:2609.39873: https://arxiv.org/abs/2609.39873
- Rolling-WAM, Zhou et al., arXiv:2609.30247: https://arxiv.org/abs/2609.30247
- StreamingWAM, Ren et al., arXiv:2610.15585: https://arxiv.org/abs/2610.15585
- Poignonec et al. (2026), NMPC-based variable admittance control with passivity constraints: https://doi.org/10.1016/j.robot.2026.105483
- NIST Humanoid Robot Baseline Performance Benchmark: https://www.nist.gov/el/intelligent-systems-division-73500/humanoid-robot-baseline-performance-benchmark
- Fraunhofer IPA humanoid benchmark: https://www.ipa.fraunhofer.de/en/press-media/press_releases/benchmark-for-humanoid-robots.html
- POMDAR, IEEE RA-L (November 2026): https://doi.org/10.1109/LRA.2026.3735899
- HAND ERC Multi-Level Benchmarks: https://hand-erc.github.io/benchmarking/benchmarks.html
